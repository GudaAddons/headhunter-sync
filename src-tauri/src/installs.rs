//! Finds the WoW games HeadHunter runs on and their saved data. A game folder is any folder
//! in a WoW folder that has the addon installed or `WTF\Account\<account>\SavedVariables\HeadHunter.lua`;
//! its name does not matter, so new Blizzard folders need no app update. The client comes from
//! the addon's saved `meta.client`. The WoW folder comes from the Blizzard registry keys, common
//! install paths and folders the player picked (the WoW folder itself or one of its game folders).

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::payload::Client;

const REGIONS: [&str; 5] = ["us", "eu", "kr", "tw", "cn"];

/// WoW Forever is released on 2026-11-04 (00:00 UTC); from then on the beta games never sync.
pub const BETA_END: i64 = 1_793_750_400;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Install {
    /// The game folder, e.g. `G:\Games\World of Warcraft\_classic_era_`.
    pub path: PathBuf,
    /// From the addon's saved `meta.client`; `None` until the addon saved it.
    pub client: Option<Client>,
    /// `## Version` of the installed addon, when it is installed.
    pub addon_version: Option<String>,
    /// The game's region: `SET portal` in `WTF\Config.wtf`, else the Battle.net branch of the
    /// game in the WoW folder's `.build.info` (the beta says it only there).
    pub game_region: Option<String>,
    /// A beta game: its product (`.flavor.info`), else its folder name, says "beta".
    pub beta: bool,
    pub accounts: Vec<Account>,
}

impl Install {
    /// A beta game after the release: its data is never sent again.
    pub fn beta_ended(&self, now: i64) -> bool {
        self.beta && now >= BETA_END
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Account {
    pub name: String,
    pub saved_file: PathBuf,
}

/// Retail and the test realms (PTR) are never ours.
fn is_skipped(folder_name: &str) -> bool {
    let name = folder_name.to_ascii_lowercase();
    name == "_retail_" || name.ends_with("ptr_")
}

/// A folder the game made (e.g. `_classic_era_`), not the WoW folder.
fn is_game_folder(folder: &Path) -> bool {
    folder.join(".flavor.info").is_file() || folder.join("WTF").is_dir()
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
        folders.push(if is_game_folder(folder) { folder.parent().map(Path::to_path_buf).unwrap_or_default() } else { folder.clone() });
    }
    let mut seen = BTreeSet::new();
    folders.into_iter().filter(|f| f.is_dir() && seen.insert(normalize(f))).collect()
}

pub fn find(picked: &[PathBuf]) -> Vec<Install> {
    let mut installs = Vec::new();
    for root in wow_folders(picked) {
        let Ok(entries) = fs::read_dir(&root) else { continue };
        let build_info = fs::read_to_string(root.join(".build.info")).unwrap_or_default();
        let mut games: Vec<PathBuf> = entries
            .flatten()
            .filter(|e| e.path().is_dir() && !is_skipped(&e.file_name().to_string_lossy()))
            .map(|e| e.path())
            .collect();
        games.sort();
        for path in games {
            let accounts = accounts(&path);
            if accounts.is_empty() && addons_dir(&path).is_none() {
                continue;
            }
            let portal = fs::read_to_string(path.join("WTF").join("Config.wtf")).ok().and_then(|config| portal_region(&config));
            let product = fs::read_to_string(path.join(".flavor.info")).ok().and_then(|flavor| product(&flavor));
            let branch = product.as_deref().and_then(|product| branch_region(&build_info, product));
            let beta = product.unwrap_or_else(|| path.file_name().unwrap_or_default().to_string_lossy().to_string()).to_ascii_lowercase().contains("beta");
            installs.push(Install {
                client: accounts.iter().find_map(|a| fs::read_to_string(&a.saved_file).ok().and_then(|saved| saved_client(&saved))),
                addon_version: addon_version(&path),
                game_region: portal.or(branch),
                beta,
                accounts,
                path,
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

/// `["client"] = "era",` in the saved file; only `meta.client` has that key.
pub fn saved_client(saved: &str) -> Option<Client> {
    saved.lines().find_map(|line| match line.trim().strip_prefix(r#"["client"] = "#)?.trim_end_matches(',') {
        r#""era""# => Some(Client::Era),
        r#""forever""# => Some(Client::Forever),
        _ => None,
    })
}

/// The product in a game folder's `.flavor.info`, e.g. `wow_classic_beta`.
pub fn product(flavor: &str) -> Option<String> {
    flavor.lines().skip(1).map(str::trim).find(|line| !line.is_empty()).map(String::from)
}

/// The Battle.net branch of a product in the WoW folder's `.build.info` as a region.
pub fn branch_region(build_info: &str, product: &str) -> Option<String> {
    let mut lines = build_info.lines();
    let columns: Vec<&str> = lines.next()?.split('|').map(|c| c.split('!').next().unwrap_or(c)).collect();
    let branch = columns.iter().position(|c| *c == "Branch")?;
    let product_column = columns.iter().position(|c| *c == "Product")?;
    lines
        .map(|line| line.split('|').collect::<Vec<_>>())
        .find(|row| row.get(product_column).is_some_and(|p| p.trim() == product))
        .and_then(|row| row.get(branch).map(|b| b.trim().to_ascii_lowercase()))
        .filter(|region| REGIONS.contains(&region.as_str()))
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
    REGIONS.contains(&region.as_str()).then_some(region)
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

    const BUILD_INFO: &str = "Branch!STRING:0|Active!DEC:1|Version!STRING:0|Product!STRING:0\n\
        eu|1|1.15.9.70003|wow_classic_era\n\
        us|1|1.60.1.70205|wow_classic_beta\n\
        cn|1|1.60.1.70300|wow_classic_beta_cn\n";

    fn game(root: &Path, name: &str, product: &str) -> PathBuf {
        let game = root.join(name);
        fs::create_dir_all(game.join("WTF")).unwrap();
        fs::write(game.join(".flavor.info"), format!("Product Flavor!STRING:0\n{product}\n")).unwrap();
        game
    }

    fn saved(game: &Path, account: &str, text: &str) {
        let dir = game.join("WTF").join("Account").join(account).join("SavedVariables");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("HeadHunter.lua"), text).unwrap();
    }

    #[test]
    fn finds_games_by_their_content_not_their_name() {
        let root = std::env::temp_dir().join(format!("hh-wow-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join(".build.info"), BUILD_INFO).unwrap();
        let era = game(&root, "_classic_era_", "wow_classic_era");
        saved(&era, "11111#1", "HeadHunter_DB = {\n[\"meta\"] = {\n[\"client\"] = \"era\",\n},\n}\n");
        fs::create_dir_all(era.join(r"WTF\Account\22222#1\SavedVariables")).unwrap();
        fs::create_dir_all(era.join(r"Interface\AddOns\HeadHunter")).unwrap();
        fs::write(era.join(r"Interface\AddOns\HeadHunter\HeadHunter.toc"), "## Title: HeadHunter\n## Version: 0.1.4\n").unwrap();
        let cn = game(&root, "_cn_beta_", "wow_classic_beta_cn");
        saved(&cn, "33333#1", "HeadHunter_DB = {\n[\"meta\"] = {\n[\"client\"] = \"forever\",\n},\n}\n");
        let release = game(&root, "_classic_forever_", "wow_classic_forever");
        fs::create_dir_all(release.join(r"Interface\Addons\HeadHunter")).unwrap();
        fs::write(release.join(r"Interface\Addons\HeadHunter\HeadHunter.toc"), "## Version: 0.3.8\n").unwrap();
        let beta = game(&root, "_classic_beta_", "wow_classic_beta");
        fs::write(beta.join(r"WTF\Config.wtf"), "SET portal \"test\"\n").unwrap();
        saved(&beta, "44444#1", "HeadHunter_DB = {}\n");
        saved(&game(&root, "_retail_", "wow"), "55555#1", "HeadHunter_DB = {}\n");
        saved(&game(&root, "_classic_era_ptr_", "wow_classic_era_ptr"), "66666#1", "HeadHunter_DB = {}\n");
        game(&root, "_anniversary_", "wow_anniversary");

        let installs: Vec<Install> = find(&[cn.clone()]).into_iter().filter(|i| i.path.starts_with(&root)).collect();
        let names: Vec<String> = installs.iter().map(|i| i.path.file_name().unwrap().to_string_lossy().to_string()).collect();
        assert_eq!(names, ["_classic_beta_", "_classic_era_", "_classic_forever_", "_cn_beta_"], "picking a game folder finds its siblings");

        let era_install = &installs[1];
        assert_eq!(era_install.client, Some(Client::Era));
        assert_eq!(era_install.addon_version.as_deref(), Some("0.1.4"));
        assert_eq!(era_install.accounts.len(), 1, "only accounts that have HeadHunter data");
        assert_eq!(era_install.accounts[0].name, "11111#1");
        assert_eq!(era_install.game_region.as_deref(), Some("eu"));

        assert_eq!(installs[3].client, Some(Client::Forever));
        assert_eq!(installs[3].game_region.as_deref(), Some("cn"), "the CN beta's branch");
        assert_eq!(installs[0].client, None, "not saved by the addon yet");
        assert_eq!(installs[0].game_region.as_deref(), Some("us"), "the test portal says nothing, the branch does");
        assert_eq!(installs[2].client, None);
        assert!(installs[2].accounts.is_empty(), "addon installed, no data yet");

        let betas: Vec<bool> = installs.iter().map(|i| i.beta).collect();
        assert_eq!(betas, [true, false, false, true], "the beta and CN beta products say beta");
        assert!(!installs[0].beta_ended(BETA_END - 1), "the beta syncs until the release");
        assert!(installs[0].beta_ended(BETA_END) && installs[3].beta_ended(BETA_END));
        assert!(!installs[1].beta_ended(BETA_END) && !installs[2].beta_ended(BETA_END));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn reads_the_client_the_addon_saved() {
        assert_eq!(saved_client("HeadHunter_DB = {\n[\"meta\"] = {\n\t[\"client\"] = \"forever\",\n"), Some(Client::Forever));
        assert_eq!(saved_client("[\"client\"] = \"era\""), Some(Client::Era));
        assert_eq!(saved_client("[\"client\"] = \"unsupported\","), None);
        assert_eq!(saved_client("HeadHunter_DB = {}"), None);
    }

    #[test]
    fn reads_the_region_from_the_battle_net_branch() {
        assert_eq!(product("Product Flavor!STRING:0\r\nwow_classic_beta\r\n").as_deref(), Some("wow_classic_beta"));
        assert_eq!(branch_region(BUILD_INFO, "wow_classic_beta").as_deref(), Some("us"));
        assert_eq!(branch_region(BUILD_INFO, "wow_classic_beta_cn").as_deref(), Some("cn"));
        assert_eq!(branch_region(BUILD_INFO, "wow_unknown"), None);
        assert_eq!(branch_region("", "wow_classic_beta"), None);
    }

    #[test]
    fn reads_the_region_from_the_game_config() {
        assert_eq!(portal_region("SET locale \"enGB\"\r\nSET portal \"EU\"\r\n").as_deref(), Some("eu"));
        assert_eq!(portal_region("SET portal \"US\"").as_deref(), Some("us"));
        assert_eq!(portal_region("SET portal \"test\""), None, "the beta and PTR portals name no region");
        assert_eq!(portal_region("SET locale \"enUS\""), None);
    }
}
