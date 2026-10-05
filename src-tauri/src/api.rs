//! The website's sync API (routes/api.php in the website repo): device tokens and
//! uploads. Every answer is turned into something the status screen can say.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{config, i18n};
use crate::payload::Payload;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct User {
    pub name: String,
    /// None for an account made with Battle.net, which shares no email.
    pub email: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UploadOutcome {
    /// 202: stored, the site imports it in the background.
    Sent,
    /// 200: this exact data was sent before.
    AlreadyThere,
    /// 409: the character belongs to another account.
    Claimed(String),
    /// 422: the site refused the data.
    Rejected(String),
    /// 401: the token was signed out on the website.
    SignedOut,
    /// Offline, rate limited or a server error: try again later.
    Retry(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ScreenshotOutcome {
    /// 201 saved, or 200: the website had it already.
    Saved,
    /// 409, 422 or another 4xx: the website will never take this one.
    Refused(String),
    /// 401: the token was signed out on the website.
    SignedOut,
    /// Offline, rate limited or a server error: try again later.
    Retry(String),
}

const WEBP_TYPE: &str = "image/webp";

#[derive(Debug, Clone, PartialEq)]
pub enum DownloadOutcome {
    /// 200: the data and its ETag.
    Fresh(Value, Option<String>),
    /// 304: nothing changed since the ETag sent.
    Unchanged,
    /// 401: the token was signed out on the website.
    SignedOut,
    /// Offline, rate limited or a server error: try again later.
    Retry(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UploadInfo {
    pub id: i64,
    pub status: String,
    pub character: Option<String>,
    pub character_id: Option<i64>,
    pub realm: Option<String>,
    pub created_at: Option<String>,
    pub error: Option<String>,
}

pub struct Api {
    http: reqwest::Client,
}

impl Default for Api {
    fn default() -> Self {
        Self::new()
    }
}

impl Api {
    pub fn new() -> Self {
        let http = reqwest::Client::builder()
            .user_agent(concat!("HeadHunterSync/", env!("CARGO_PKG_VERSION")))
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("HTTP client");
        Self { http }
    }

    /// Signs this PC in; one token per device name (a new sign-in replaces the old one).
    pub async fn sign_in(&self, email: &str, password: &str, device: &str) -> Result<(String, User), String> {
        let response = self
            .http
            .post(config::api("auth/token"))
            .header("Accept", "application/json")
            .header("Accept-Language", i18n::accept_language())
            .json(&serde_json::json!({ "email": email, "password": password, "device_name": device }))
            .send()
            .await
            .map_err(|e| offline_message(&e))?;
        token_answer(response, &i18n::t("Check your email and password.")).await
    }

    /// "Sign in with browser": trades the website's single-use code for a device token.
    pub async fn exchange(&self, code: &str, verifier: &str, redirect_uri: &str, device: &str) -> Result<(String, User), String> {
        let response = self
            .http
            .post(config::api("auth/exchange"))
            .header("Accept", "application/json")
            .header("Accept-Language", i18n::accept_language())
            .json(&serde_json::json!({ "code": code, "code_verifier": verifier, "redirect_uri": redirect_uri, "device_name": device }))
            .send()
            .await
            .map_err(|e| offline_message(&e))?;
        token_answer(response, &i18n::t("The sign in expired. Try again.")).await
    }

    pub async fn sign_out(&self, token: &str) {
        let _ = self
            .http
            .delete(config::api("auth/token"))
            .bearer_auth(token)
            .header("Accept", "application/json")
            .header("Accept-Language", i18n::accept_language())
            .send()
            .await;
    }

    pub async fn upload(&self, token: &str, payload: &Payload) -> UploadOutcome {
        let response = match self
            .http
            .post(config::api("sync/uploads"))
            .bearer_auth(token)
            .header("Accept", "application/json")
            .header("Accept-Language", i18n::accept_language())
            .json(payload)
            .send()
            .await
        {
            Ok(response) => response,
            Err(e) => return UploadOutcome::Retry(offline_message(&e)),
        };
        let status = response.status().as_u16();
        let body: Value = response.json().await.unwrap_or(Value::Null);
        match status {
            202 => UploadOutcome::Sent,
            200 => UploadOutcome::AlreadyThere,
            401 | 403 => UploadOutcome::SignedOut,
            409 => UploadOutcome::Claimed(message(&body).unwrap_or_else(|| i18n::t("This character is linked to another account."))),
            422 => UploadOutcome::Rejected(first_error(&body).unwrap_or_else(|| i18n::t("The website refused the data."))),
            429 => UploadOutcome::Retry(i18n::t("The website asks to slow down; trying again soon.")),
            _ => UploadOutcome::Retry(i18n::tr("The website answered :status; trying again soon.", &[("status", &status.to_string())])),
        }
    }

    /// One screenshot as `multipart/form-data`: the text `fields` and the WebP picture.
    pub async fn upload_screenshot(&self, token: &str, fields: Vec<(String, String)>, file_name: String, picture: Vec<u8>) -> ScreenshotOutcome {
        let image = match reqwest::multipart::Part::bytes(picture).file_name(file_name).mime_str(WEBP_TYPE) {
            Ok(image) => image,
            Err(e) => return ScreenshotOutcome::Refused(e.to_string()),
        };
        let form = fields.into_iter().fold(reqwest::multipart::Form::new(), |form, (name, value)| form.text(name, value)).part("image", image);
        let response = match self
            .http
            .post(config::api("sync/screenshots"))
            .bearer_auth(token)
            .header("Accept", "application/json")
            .header("Accept-Language", i18n::accept_language())
            .multipart(form)
            .send()
            .await
        {
            Ok(response) => response,
            Err(e) => return ScreenshotOutcome::Retry(offline_message(&e)),
        };
        let status = response.status().as_u16();
        let body: Value = response.json().await.unwrap_or(Value::Null);
        screenshot_outcome(status, &body)
    }

    /// The WANTED and Duels lists of these worlds and the account's own records there,
    /// for the HeadHunter_Data addon. `etag` from the last answer gives `Unchanged`.
    pub async fn download(&self, token: &str, worlds: &[String], etag: Option<&str>) -> DownloadOutcome {
        let query: Vec<(&str, &str)> = worlds.iter().map(|w| ("worlds[]", w.as_str())).collect();
        let mut request = self
            .http
            .get(config::api("sync/download"))
            .query(&query)
            .bearer_auth(token)
            .header("Accept", "application/json")
            .header("Accept-Language", i18n::accept_language());
        if let Some(etag) = etag {
            request = request.header("If-None-Match", etag);
        }
        let response = match request.send().await {
            Ok(response) => response,
            Err(e) => return DownloadOutcome::Retry(offline_message(&e)),
        };
        let status = response.status().as_u16();
        let etag = response.headers().get("ETag").and_then(|v| v.to_str().ok()).map(String::from);
        match status {
            200 => match response.json::<Value>().await {
                Ok(body) => DownloadOutcome::Fresh(body, etag),
                Err(_) => DownloadOutcome::Retry(i18n::t("The website sent game data that could not be read.")),
            },
            304 => DownloadOutcome::Unchanged,
            401 | 403 => DownloadOutcome::SignedOut,
            429 => DownloadOutcome::Retry(i18n::t("The website asks to slow down; trying again soon.")),
            _ => DownloadOutcome::Retry(i18n::tr("The website answered :status for the game data; trying again soon.", &[("status", &status.to_string())])),
        }
    }

    pub async fn recent_uploads(&self, token: &str) -> Result<Vec<UploadInfo>, String> {
        let response = self
            .http
            .get(config::api("sync/uploads"))
            .bearer_auth(token)
            .header("Accept", "application/json")
            .header("Accept-Language", i18n::accept_language())
            .send()
            .await
            .map_err(|e| offline_message(&e))?;
        if !response.status().is_success() {
            return Err(i18n::tr("The website answered :status.", &[("status", &response.status().as_u16().to_string())]));
        }
        let body: Value = response.json().await.map_err(|e| e.to_string())?;
        Ok(body["data"]
            .as_array()
            .map(|list| list.iter().map(upload_info).collect())
            .unwrap_or_default())
    }
}

async fn token_answer(response: reqwest::Response, refused: &str) -> Result<(String, User), String> {
    let status = response.status().as_u16();
    let body: Value = response.json().await.unwrap_or(Value::Null);
    match status {
        200 | 201 => {
            let token = body["token"].as_str().ok_or_else(|| i18n::t("The website sent no token."))?.to_string();
            Ok((token, user_from(&body["user"])))
        }
        422 => Err(first_error(&body).unwrap_or_else(|| refused.into())),
        429 => Err(i18n::t("Too many tries. Wait a minute and try again.")),
        _ => Err(i18n::tr("The website answered :status. Try again later.", &[("status", &status.to_string())])),
    }
}

fn screenshot_outcome(status: u16, body: &Value) -> ScreenshotOutcome {
    match status {
        200 | 201 => ScreenshotOutcome::Saved,
        401 | 403 => ScreenshotOutcome::SignedOut,
        429 => ScreenshotOutcome::Retry(i18n::t("The website asks to slow down; trying again soon.")),
        400..=499 => ScreenshotOutcome::Refused(first_error(body).unwrap_or_else(|| i18n::t("The website refused the data."))),
        _ => ScreenshotOutcome::Retry(i18n::tr("The website answered :status; trying again soon.", &[("status", &status.to_string())])),
    }
}

fn user_from(u: &Value) -> User {
    User {
        name: u["name"].as_str().unwrap_or_default().to_string(),
        email: u["email"].as_str().map(String::from),
    }
}

fn upload_info(u: &Value) -> UploadInfo {
    UploadInfo {
        id: u["id"].as_i64().unwrap_or_default(),
        status: u["status"].as_str().unwrap_or_default().to_string(),
        character: u["character"]["name"].as_str().map(String::from),
        character_id: u["character"]["id"].as_i64(),
        realm: u["character"]["realm"].as_str().map(String::from),
        created_at: u["created_at"].as_str().map(String::from),
        error: u["error"].as_str().map(String::from),
    }
}

fn message(body: &Value) -> Option<String> {
    body["message"].as_str().map(String::from)
}

/// The first validation message, e.g. "These credentials do not match our records."
fn first_error(body: &Value) -> Option<String> {
    body["errors"]
        .as_object()
        .and_then(|errors| errors.values().find_map(|list| list[0].as_str()))
        .map(String::from)
        .or_else(|| message(body))
}

fn offline_message(e: &reqwest::Error) -> String {
    if e.is_timeout() {
        i18n::t("The website did not answer in time.")
    } else if e.is_connect() {
        i18n::tr("Cannot reach :url. Is it online?", &[("url", config::API_URL)])
    } else {
        e.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn reads_the_first_validation_message() {
        let body = json!({ "message": "The given data was invalid.", "errors": { "email": ["These credentials do not match our records."] } });
        assert_eq!(first_error(&body).as_deref(), Some("These credentials do not match our records."));
        assert_eq!(first_error(&json!({ "message": "Nope" })).as_deref(), Some("Nope"));
    }

    #[test]
    fn reads_a_screenshot_answer() {
        assert_eq!(screenshot_outcome(201, &Value::Null), ScreenshotOutcome::Saved);
        assert_eq!(screenshot_outcome(200, &Value::Null), ScreenshotOutcome::Saved);
        assert_eq!(screenshot_outcome(422, &json!({ "errors": { "t": ["Too old."] } })), ScreenshotOutcome::Refused("Too old.".into()));
        assert!(matches!(screenshot_outcome(409, &Value::Null), ScreenshotOutcome::Refused(_)));
        assert_eq!(screenshot_outcome(401, &Value::Null), ScreenshotOutcome::SignedOut);
        assert!(matches!(screenshot_outcome(429, &Value::Null), ScreenshotOutcome::Retry(_)));
        assert!(matches!(screenshot_outcome(503, &Value::Null), ScreenshotOutcome::Retry(_)));
    }

    #[test]
    fn reads_a_user_with_or_without_email() {
        assert_eq!(user_from(&json!({ "name": "Tess", "email": "tess@example.com" })).email.as_deref(), Some("tess@example.com"));
        assert_eq!(user_from(&json!({ "name": "Tess", "email": null })), User { name: "Tess".into(), email: None });
    }

    #[test]
    fn reads_an_upload_from_the_list() {
        let info = upload_info(&json!({ "id": 7, "status": "parsed", "character": { "id": 3, "name": "Tess Rider", "realm": "Firemaw" }, "created_at": "2026-09-25T10:00:00Z", "error": null }));
        assert_eq!(info, UploadInfo { id: 7, status: "parsed".into(), character: Some("Tess Rider".into()), character_id: Some(3), realm: Some("Firemaw".into()), created_at: Some("2026-09-25T10:00:00Z".into()), error: None });
    }
}
