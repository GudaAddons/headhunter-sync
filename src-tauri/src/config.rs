//! Build settings, fixed when the app is compiled: the local build talks to the Sail
//! site on http://localhost, the production build to HEADHUNTER_API_URL (see build.rs
//! and scripts/build-prod.mjs).

pub const API_URL: &str = match option_env!("HEADHUNTER_API_URL") {
    Some(url) => url,
    None => "http://localhost",
};

pub const BUILD: &str = match option_env!("HEADHUNTER_BUILD") {
    Some(build) => build,
    None => "local",
};

pub fn is_production() -> bool {
    BUILD == "prod"
}

/// Keeps the local and production apps' tokens apart in the credential store.
pub fn keyring_service() -> &'static str {
    if is_production() {
        "HeadHunter Sync"
    } else {
        "HeadHunter Sync (Local)"
    }
}

pub fn api(path: &str) -> String {
    format!("{}/api/v1/{}", API_URL.trim_end_matches('/'), path.trim_start_matches('/'))
}
