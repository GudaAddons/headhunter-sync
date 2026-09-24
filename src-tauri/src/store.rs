//! What the app keeps on disk: settings (settings.json), what was sent and the last
//! result per character (state.json), both in the app's data folder, and the sign-in
//! token in the system credential store (Windows Credential Manager).

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::config;
use crate::payload::Sent;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Settings {
    /// WoW folders the player picked, next to the ones found automatically.
    pub wow_folders: Vec<PathBuf>,
    /// Per install folder (e.g. `...\_classic_beta_`): what the saved data cannot say.
    pub installs: BTreeMap<String, InstallSettings>,
    /// Upload a few seconds after the game writes HeadHunter.lua.
    pub sync_on_change: bool,
    pub sync_on_start: bool,
    /// 0 = off.
    pub interval_minutes: u32,
    pub start_with_system: bool,
    /// Shown on the status screen; the token itself is in the credential store.
    pub user_name: Option<String>,
    pub email: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            wow_folders: Vec::new(),
            installs: BTreeMap::new(),
            sync_on_change: true,
            sync_on_start: true,
            interval_minutes: 60,
            start_with_system: false,
            user_name: None,
            email: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct InstallSettings {
    pub enabled: bool,
    /// Used until the addon saves its region (addon 0.1.3+ does).
    pub region: Option<String>,
    /// Forever only: pvp, pve, roleplay or hardcore.
    pub realm_type: String,
}

impl Default for InstallSettings {
    fn default() -> Self {
        Self { enabled: true, region: None, realm_type: "pvp".into() }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct State {
    /// "install|account|character" -> what was sent.
    pub sent: BTreeMap<String, Sent>,
    /// Same key -> the last result.
    pub results: BTreeMap<String, SyncResult>,
    pub last_run: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SyncResult {
    pub at: i64,
    /// sent, already_there, nothing_new, claimed, rejected, retry, error
    pub outcome: String,
    pub message: Option<String>,
    pub records: usize,
}

pub fn state_key(install: &str, account: &str, character: &str) -> String {
    format!("{install}|{account}|{character}")
}

pub fn load<T: DeserializeOwned + Default>(path: &Path) -> T {
    fs::read_to_string(path).ok().and_then(|text| serde_json::from_str(&text).ok()).unwrap_or_default()
}

pub fn save<T: Serialize>(path: &Path, value: &T) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, serde_json::to_vec_pretty(value)?)?;
    fs::rename(tmp, path)
}

fn token_entry() -> keyring::Result<keyring::Entry> {
    keyring::Entry::new(config::keyring_service(), "sync-token")
}

pub fn token() -> Option<String> {
    token_entry().ok()?.get_password().ok()
}

pub fn set_token(token: &str) -> Result<(), String> {
    token_entry().and_then(|e| e.set_password(token)).map_err(|e| e.to_string())
}

pub fn clear_token() {
    if let Ok(entry) = token_entry() {
        let _ = entry.delete_credential();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_default_and_survive_a_round_trip() {
        let dir = std::env::temp_dir().join(format!("hh-sync-test-{}", std::process::id()));
        let path = dir.join("settings.json");
        let missing: Settings = load(&path);
        assert_eq!(missing, Settings::default());
        assert!(missing.sync_on_change && missing.sync_on_start && missing.interval_minutes == 60);

        let mut settings = Settings::default();
        settings.interval_minutes = 15;
        settings.installs.insert("G:/WoW/_classic_beta_".into(), InstallSettings { realm_type: "hardcore".into(), ..Default::default() });
        save(&path, &settings).unwrap();
        assert_eq!(load::<Settings>(&path), settings);

        std::fs::write(&path, "{ \"interval_minutes\": 30 }").unwrap();
        let partial: Settings = load(&path);
        assert_eq!(partial.interval_minutes, 30);
        assert!(partial.sync_on_change, "missing fields take their defaults");
        let _ = std::fs::remove_dir_all(dir);
    }
}
