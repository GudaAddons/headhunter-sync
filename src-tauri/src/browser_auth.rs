//! "Sign in with browser" (APP-007, website WEB-093): for accounts made with Battle.net,
//! Discord or Google, which have no password. The website does the login in the
//! player's own browser and sends a single-use code back to a one-time listener on
//! 127.0.0.1; the app trades it, with a PKCE verifier only it knows, for a device token.

use std::time::Duration;

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

use crate::config;

/// How long the app waits for the player to finish in the browser.
pub const WAIT: Duration = Duration::from_secs(300);

pub struct Pkce {
    pub verifier: String,
    pub challenge: String,
}

impl Pkce {
    pub fn new() -> Self {
        let verifier = random_token();
        let challenge = challenge_for(&verifier);
        Self { verifier, challenge }
    }
}

impl Default for Pkce {
    fn default() -> Self {
        Self::new()
    }
}

pub fn random_token() -> String {
    URL_SAFE_NO_PAD.encode(rand::random::<[u8; 32]>())
}

/// PKCE S256: base64url(sha256(verifier)) without padding.
pub fn challenge_for(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

/// A one-time listener on a free local port; the website sends the browser back here.
pub struct Callback {
    listener: TcpListener,
    pub redirect_uri: String,
}

impl Callback {
    pub async fn bind() -> Result<Self, String> {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .map_err(|e| format!("Could not open a local port for the sign in: {e}"))?;
        let port = listener.local_addr().map_err(|e| e.to_string())?.port();
        Ok(Self { listener, redirect_uri: format!("http://127.0.0.1:{port}/callback") })
    }

    /// Waits for the browser; returns the code when the state matches.
    pub async fn code(&self, state: &str) -> Result<String, String> {
        loop {
            let (mut socket, _) = self.listener.accept().await.map_err(|e| e.to_string())?;
            let Some(target) = read_target(&mut socket).await else {
                continue;
            };
            let Some(query) = target.strip_prefix("/callback?") else {
                respond(&mut socket, "404 Not Found", "Nothing here.").await;
                continue;
            };
            match parse_callback(query, state) {
                Ok(code) => {
                    respond(&mut socket, "200 OK", "You are signed in. You can close this tab and go back to HeadHunter Sync.").await;
                    return Ok(code);
                }
                Err(message) => {
                    respond(&mut socket, "400 Bad Request", &message).await;
                    return Err(message);
                }
            }
        }
    }
}

/// The website page that asks the player to connect this PC.
pub fn connect_url(pkce: &Pkce, state: &str, redirect_uri: &str, device: &str) -> String {
    let mut url = reqwest::Url::parse(&config::web("app/connect")).expect("website address");
    url.query_pairs_mut()
        .append_pair("redirect_uri", redirect_uri)
        .append_pair("state", state)
        .append_pair("code_challenge", &pkce.challenge)
        .append_pair("code_challenge_method", "S256")
        .append_pair("device_name", device);
    url.into()
}

/// The code from "?code=...&state=...", or why there is none.
fn parse_callback(query: &str, expected_state: &str) -> Result<String, String> {
    let url = reqwest::Url::parse(&format!("http://127.0.0.1/?{query}")).map_err(|e| e.to_string())?;
    let value = |key: &str| url.query_pairs().find(|(k, _)| k == key).map(|(_, v)| v.into_owned());
    if value("state").as_deref() != Some(expected_state) {
        return Err("This sign in did not come from this app. Try again from HeadHunter Sync.".into());
    }
    if value("error").is_some() {
        return Err("Sign in was cancelled.".into());
    }
    value("code").filter(|code| !code.is_empty()).ok_or_else(|| "The website sent no sign-in code.".into())
}

/// The request target of "GET /callback?... HTTP/1.1".
async fn read_target(socket: &mut TcpStream) -> Option<String> {
    let mut buffer = vec![0u8; 8192];
    let mut read = 0;
    while read < buffer.len() {
        let n = tokio::time::timeout(Duration::from_secs(5), socket.read(&mut buffer[read..])).await.ok()?.ok()?;
        if n == 0 {
            break;
        }
        read += n;
        if buffer[..read].windows(4).any(|w| w == b"\r\n\r\n") {
            break;
        }
    }
    let head = String::from_utf8_lossy(&buffer[..read]);
    let mut parts = head.lines().next()?.split_whitespace();
    (parts.next()? == "GET").then(|| parts.next().map(String::from))?
}

async fn respond(socket: &mut TcpStream, status: &str, message: &str) {
    let body = format!(
        "<!doctype html><meta charset=utf-8><title>HeadHunter Sync</title>\
         <body style=\"font-family:sans-serif;background:#15100b;color:#eab13c;display:grid;place-items:center;height:100vh;margin:0\">\
         <p style=\"font-size:1.2rem;max-width:32rem;text-align:center\">{}</p>",
        message.replace('<', "&lt;")
    );
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = socket.write_all(response.as_bytes()).await;
    let _ = socket.shutdown().await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn makes_the_rfc_7636_challenge() {
        // RFC 7636 appendix B
        assert_eq!(
            challenge_for("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
        let pkce = Pkce::new();
        assert_eq!(pkce.verifier.len(), 43);
        assert_ne!(pkce.verifier, Pkce::new().verifier);
    }

    #[test]
    fn reads_the_code_only_with_the_right_state() {
        assert_eq!(parse_callback("code=abc&state=s1", "s1"), Ok("abc".into()));
        assert!(parse_callback("code=abc&state=other", "s1").is_err());
        assert_eq!(parse_callback("error=access_denied&state=s1", "s1"), Err("Sign in was cancelled.".into()));
        assert!(parse_callback("state=s1", "s1").is_err());
    }

    #[test]
    fn builds_the_connect_link() {
        let pkce = Pkce { verifier: "v".into(), challenge: "c".into() };
        let url = connect_url(&pkce, "s1", "http://127.0.0.1:5000/callback", "TESS PC");
        assert!(url.starts_with(&format!("{}/app/connect?", config::API_URL.trim_end_matches('/'))));
        assert!(url.contains("redirect_uri=http%3A%2F%2F127.0.0.1%3A5000%2Fcallback"));
        assert!(url.contains("device_name=TESS+PC"));
        assert!(url.contains("code_challenge_method=S256"));
    }

    #[tokio::test]
    async fn answers_the_browser_and_returns_the_code() {
        let callback = Callback::bind().await.unwrap();
        let address = callback.redirect_uri.trim_start_matches("http://").trim_end_matches("/callback").to_string();
        let browser = tokio::spawn(async move {
            let mut stream = TcpStream::connect(address).await.unwrap();
            stream.write_all(b"GET /callback?code=abc&state=s1 HTTP/1.1\r\nHost: x\r\n\r\n").await.unwrap();
            let mut answer = String::new();
            stream.read_to_string(&mut answer).await.unwrap();
            answer
        });
        assert_eq!(callback.code("s1").await, Ok("abc".into()));
        assert!(browser.await.unwrap().starts_with("HTTP/1.1 200 OK"));
    }
}
