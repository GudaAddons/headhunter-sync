//! Finds the WoW clients HeadHunter runs on and their saved data:
//! `<WoW>\_classic_era_` (Classic Era) and `<WoW>\_classic_beta_` (WoW Forever beta),
//! each with `WTF\Account\<account>\SavedVariables\HeadHunter.lua`.
//! The WoW folder comes from the Blizzard registry keys, common install paths and
//! folders the player picked (the WoW folder itself or one of its game folders).

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::payload::Client;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Install {
    /// The game folder, e.g. `G:\Games\World of Warcraft\_classic_era_`.
    pub path: PathBuf,
    pub client: Client,
    /// `## Version` of the installed addon, when it is installed.
    pub addon_version: Option<String>,
    /// The region the game client uses (`SET portal` in `WTF\Config.wtf`), when it says.
    pub portal_region: Option<String>,
    pub accounts: Vec<Account>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Account {
    pub name: String,
    pub saved_file: PathBuf,
}

pub fn client_of(folder_name: &str) -> Option<Client> {
    match folder_name.to_ascii_lowercase().as_str() {
        "_classic_era_" => Some(Client::Era),
        "_classic_beta_" => Some(Client::Forever),
        _ => None,
    }
}

/// WoW folders to look in: registry, common paths, then the player's own picks.
pub fn wow_folders(picked: &[PathBuf]) -> Vec<PathBuf> {
    let mut folders: Vec<PathBuf> = registry_folders();
    for common in [
        r"C:\Program Files (x86)\World of Warcraft",
        r"C:\Program Files\World of Warcraft",
        "/Applications/World of Warcraft",
    ] {
        folders.push(PathBuf::from(common));
    }
    for folder in picked {
        let is_game_folder = folder.file_name().and_then(|n| n.to_str()).and_then(client_of).is_some();
        folders.push(if is_game_folder { folder.parent().map(Path::to_path_buf).unwrap_or_default() } else { folder.clone() });
    }
    let mut seen = BTreeSet::new();
    folders.into_iter().filter(|f| f.is_dir() && seen.insert(normalize(f))).collect()
}

pub fn find(picked: &[PathBuf]) -> Vec<Install> {
    let mut installs = Vec::new();
    for root in wow_folders(picked) {
        let Ok(entries) = fs::read_dir(&root) else { continue };
        let mut games: Vec<(PathBuf, Client)> = entries
            .flatten()
            .filter_map(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                client_of(&name).map(|client| (e.path(), client))
            })
            .collect();
        games.sort();
        for (path, client) in games {
            installs.push(Install {
                addon_version: addon_version(&path),
                portal_region: fs::read_to_string(path.join("WTF").join("Config.wtf"))
                    .ok()
                    .and_then(|config| portal_region(&config)),
                accounts: accounts(&path),
                path,
                client,
            });
        }
    }
    installs
}

fn accounts(game: &Path) -> Vec<Account> {
    let Ok(entries) = fs::read_dir(game.join("WTF").join("Account")) else { return Vec::new() };
    let mut accounts: Vec<Account> = entries
        .flatten()
        .filter(|e| e.path().is_dir())
        .map(|e| Account {
            name: e.file_name().to_string_lossy().to_string(),
            saved_file: e.path().join("SavedVariables").join("HeadHunter.lua"),
        })
        .filter(|a| a.saved_file.is_file())
        .collect();
    accounts.sort_by(|a, b| a.name.cmp(&b.name));
    accounts
}

/// The game's AddOns folder that holds HeadHunter (the folder is spelled both ways).
pub fn addons_dir(game: &Path) -> Option<PathBuf> {
    ["AddOns", "Addons"]
        .iter()
        .map(|dir| game.join("Interface").join(dir))
        .find(|dir| dir.join("HeadHunter").join("HeadHunter.toc").is_file())
}

pub fn addon_toc(game: &Path) -> Option<String> {
    fs::read_to_string(addons_dir(game)?.join("HeadHunter").join("HeadHunter.toc")).ok()
}

fn addon_version(game: &Path) -> Option<String> {
    version_from_toc(&addon_toc(game)?)
}

pub fn version_from_toc(toc: &str) -> Option<String> {
    toc_field(toc, "Version").map(|v| v.chars().take(16).collect())
}

/// `SET portal "EU"` in Config.wtf as the website's region; test and PTR portals say nothing.
pub fn portal_region(config: &str) -> Option<String> {
    let portal = config.lines().find_map(|line| line.trim().strip_prefix("SET portal "))?;
    let region = portal.trim().trim_matches('"').to_ascii_lowercase();
    matches!(region.as_str(), "us" | "eu" | "kr" | "tw" | "cn").then_some(region)
}

pub fn toc_field(toc: &str, field: &str) -> Option<String> {
    let prefix = format!("## {field}:");
    toc.lines().find_map(|line| line.trim().strip_prefix(prefix.as_str())).map(|v| v.trim().to_string())
}

fn normalize(path: &Path) -> String {
    path.to_string_lossy().trim_end_matches(['\\', '/']).to_lowercase()
}

#[cfg(windows)]
fn registry_folders() -> Vec<PathBuf> {
    use winreg::enums::HKEY_LOCAL_MACHINE;
    use winreg::RegKey;

    let mut folders = Vec::new();
    let Ok(wow) = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey(r"SOFTWARE\WOW6432Node\Blizzard Entertainment\World of Warcraft") else {
        return folders;
    };
    let mut install_paths: Vec<String> = wow.get_value("InstallPath").into_iter().collect();
    for sub in wow.enum_keys().flatten() {
        if let Ok(key) = wow.open_subkey(&sub) {
            install_paths.extend(key.get_value::<String, _>("InstallPath"));
        }
    }
    for path in install_paths {
        // InstallPath names a game folder (`...\_classic_era_\`); the WoW folder is its parent.
        if let Some(parent) = PathBuf::from(path.trim_end_matches(['\\', '/'])).parent() {
            folders.push(parent.to_path_buf());
        }
    }
    folders
}

#[cfg(not(windows))]
fn registry_folders() -> Vec<PathBuf> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_era_and_forever_with_their_saved_data() {
        let root = std::env::temp_dir().join(format!("hh-wow-{}", std::process::id()));
        let era = root.join("_classic_era_");
        let forever = root.join("_classic_beta_");
        fs::create_dir_all(era.join(r"WTF\Account\11111#1\SavedVariables")).unwrap();
        fs::write(era.join(r"WTF\Account\11111#1\SavedVariables\HeadHunter.lua"), "HeadHunter_DB = {}").unwrap();
        fs::create_dir_all(era.join(r"WTF\Account\22222#1\SavedVariables")).unwrap();
        fs::create_dir_all(era.join(r"Interface\AddOns\HeadHunter")).unwrap();
        fs::write(era.join(r"Interface\AddOns\HeadHunter\HeadHunter.toc"), "## Title: HeadHunter\n## Version: 0.1.4\n").unwrap();
        fs::create_dir_all(forever.join("WTF")).unwrap();
        fs::create_dir_all(root.join("_retail_")).unwrap();

        let installs: Vec<Install> = find(&[era.clone()]).into_iter().filter(|i| i.path.starts_with(&root)).collect();
        assert_eq!(installs.len(), 2, "picking a game folder finds its siblings; retail is not ours");
        let era_install = installs.iter().find(|i| i.client == Client::Era).unwrap();
        assert_eq!(era_install.addon_version.as_deref(), Some("0.1.4"));
        assert_eq!(era_install.accounts.len(), 1, "only accounts that have HeadHunter data");
        assert_eq!(era_install.accounts[0].name, "11111#1");
        assert!(installs.iter().any(|i| i.client == Client::Forever && i.accounts.is_empty()));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn reads_the_region_from_the_game_config() {
        assert_eq!(portal_region("SET locale \"enGB\"\r\nSET portal \"EU\"\r\n").as_deref(), Some("eu"));
        assert_eq!(portal_region("SET portal \"US\"").as_deref(), Some("us"));
        assert_eq!(portal_region("SET portal \"test\""), None, "the beta and PTR portals name no region");
        assert_eq!(portal_region("SET locale \"enUS\""), None);
    }
}
