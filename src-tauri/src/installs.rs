//! Finds the WoW games HeadHunter runs on and their saved data. A game folder is any folder
//! in a WoW folder that has the addon installed or `WTF\Account\<account>\SavedVariables\HeadHunter.lua`,
//! or whose product is WoW Forever (shown before the addon is installed, and not synced until
//! it is); its name does not matter, so new Blizzard folders need no app update. The client
//! comes from the addon's saved `meta.client`. The WoW folder comes from the Blizzard registry
//! keys, common install paths and folders the player picked: any folder near the WoW folder
//! (`wow_folder_near`) is turned into the WoW folder before it is kept.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::payload::Client;

const REGIONS: [&str; 5] = ["us", "eu", "kr", "tw", "cn"];

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Install {
    /// The game folder, e.g. `G:\Games\World of Warcraft\_classic_era_`.
    pub path: PathBuf,
    /// From the addon's saved `meta.client`, else WoW Forever for a WoW Forever product
    /// (`is_forever`); `None` until the addon saved it in any other game.
    pub client: Option<Client>,
    /// `## Version` of the installed addon, when it is installed.
    pub addon_version: Option<String>,
    /// HeadHunter is installed in this game (`Interface\AddOns\HeadHunter`); a game without it
    /// is shown but neither uploaded nor downloaded.
    pub has_addon: bool,
    /// The game's region: `SET portal` in `WTF\Config.wtf`, else the Battle.net branch of the
    /// game in the WoW folder's `.build.info`.
    pub game_region: Option<String>,
    pub accounts: Vec<Account>,
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

/// A local build started with `HEADHUNTER_NO_AUTODETECT=1` looks only in the picked folders,
/// as on a PC where the game is found nowhere (never in the production build).
fn autodetect() -> bool {
    crate::config::is_production() || std::env::var_os("HEADHUNTER_NO_AUTODETECT").is_none()
}

/// WoW folders to look in: registry, common paths, then the player's own picks.
pub fn wow_folders(picked: &[PathBuf]) -> Vec<PathBuf> {
    let mut folders: Vec<PathBuf> = Vec::new();
    if autodetect() {
        folders.extend(registry_folders());
        for common in [
            r"C:\Program Files (x86)\World of Warcraft",
            r"C:\Program Files\World of Warcraft",
            "/Applications/World of Warcraft",
        ] {
            folders.push(PathBuf::from(common));
        }
    }
    for folder in picked {
        folders.push(if is_game_folder(folder) { folder.parent().map(Path::to_path_buf).unwrap_or_default() } else { folder.clone() });
    }
    let mut seen = BTreeSet::new();
    folders.into_iter().filter(|f| f.is_dir() && seen.insert(normalize(f))).collect()
}

pub fn find(picked: &[PathBuf]) -> Vec<Install> {
    wow_folders(picked).iter().flat_map(|root| find_in(root)).collect()
}

/// The games of one WoW folder.
fn find_in(root: &Path) -> Vec<Install> {
    let mut installs = Vec::new();
    let Ok(entries) = fs::read_dir(root) else { return installs };
    let build_info = fs::read_to_string(root.join(".build.info")).unwrap_or_default();
    let mut games: Vec<PathBuf> = entries
        .flatten()
        .filter(|e| e.path().is_dir() && !is_skipped(&e.file_name().to_string_lossy()))
        .map(|e| e.path())
        .collect();
    games.sort();
    for path in games {
        let accounts = accounts(&path);
        let has_addon = addons_dir(&path).is_some();
        let product = fs::read_to_string(path.join(".flavor.info")).ok().and_then(|flavor| product(&flavor));
        if accounts.is_empty() && !has_addon && !is_forever(product.as_deref()) {
            continue;
        }
        let portal = fs::read_to_string(path.join("WTF").join("Config.wtf")).ok().and_then(|config| portal_region(&config));
        let client = accounts
            .iter()
            .find_map(|a| fs::read_to_string(&a.saved_file).ok().and_then(|saved| saved_client(&saved)))
            .or_else(|| is_forever(product.as_deref()).then_some(Client::Forever));
        if is_classic_era(product.as_deref(), client) || is_beta(product.as_deref(), &path) {
            continue;
        }
        let branch = product.as_deref().and_then(|product| branch_region(&build_info, product));
        installs.push(Install {
            client,
            addon_version: addon_version(&path),
            has_addon,
            game_region: portal.or(branch),
            accounts,
            path,
        });
    }
    installs
}

/// How far from a picked folder the WoW folder is looked for: up (a game folder, or a folder
/// inside one such as `_forever_\Interface\AddOns`) and down (`D:\Games\Battle.net` or `D:\Games`).
const LEVELS_UP: usize = 3;
const LEVELS_DOWN: usize = 2;

/// A WoW folder: not a game folder itself, with a game folder in it.
fn is_wow_folder(folder: &Path) -> bool {
    if is_game_folder(folder) {
        return false;
    }
    let Ok(entries) = fs::read_dir(folder) else { return false };
    entries.flatten().any(|e| e.path().is_dir() && is_game_folder(&e.path()))
}

/// True when the WoW folder holds a WoW Forever game.
fn has_forever_game(wow_folder: &Path) -> bool {
    find_in(wow_folder).iter().any(|install| install.client == Some(Client::Forever))
}

fn subfolders(folder: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(folder) else { return Vec::new() };
    let mut folders: Vec<PathBuf> = entries.flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect();
    folders.sort();
    folders
}

/// The WoW folder with a WoW Forever game nearest to a picked folder: the folder itself, a
/// folder above it (LEVELS_UP) or below it (LEVELS_DOWN). `None` when there is none.
pub fn wow_folder_near(picked: &Path) -> Option<PathBuf> {
    let found = |folder: &Path| is_wow_folder(folder) && has_forever_game(folder);
    for folder in picked.ancestors().take(LEVELS_UP + 1) {
        if found(folder) {
            return Some(folder.to_path_buf());
        }
    }
    let mut level = vec![picked.to_path_buf()];
    for _ in 0..LEVELS_DOWN {
        level = level.iter().flat_map(|folder| subfolders(folder)).collect();
        if let Some(folder) = level.iter().find(|folder| found(folder)) {
            return Some(folder.clone());
        }
    }
    None
}

/// HeadHunter is for WoW Forever only (2026-10-10): a Classic Era game is not shown and
/// not synced, by its product or by the client the addon saved there.
fn is_classic_era(product: Option<&str>, client: Option<Client>) -> bool {
    client == Some(Client::Era) || product.is_some_and(|product| product.to_ascii_lowercase().contains("classic_era"))
}

/// WoW Forever's own products: `wowf`, and since the release (2026-11-04) any product that
/// says "forever" and is neither a beta nor Classic Era (e.g. `wow_forever`, `wow_classic_forever`).
fn is_forever(product: Option<&str>) -> bool {
    product.is_some_and(|product| {
        let product = product.to_ascii_lowercase();
        product == "wowf" || (product.contains("forever") && !product.contains("beta") && !product.contains("classic_era"))
    })
}

/// The WoW Forever beta closed for good (2026-10-23): a beta or CN beta game is not shown and
/// not synced, by its product (`.flavor.info`), else by its folder name.
fn is_beta(product: Option<&str>, game: &Path) -> bool {
    product
        .map(String::from)
        .unwrap_or_else(|| game.file_name().unwrap_or_default().to_string_lossy().to_string())
        .to_ascii_lowercase()
        .contains("beta")
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
        cn|1|1.60.1.70300|wow_classic_beta_cn\n\
        us|1|1.60.2.71000|wow_classic_forever\n\
        eu|1|1.60.1.70400|wowf\n";

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
        fs::create_dir_all(era.join("Interface").join("AddOns").join("HeadHunter")).unwrap();
        fs::write(era.join("Interface").join("AddOns").join("HeadHunter").join("HeadHunter.toc"), "## Title: HeadHunter\n## Version: 0.1.4\n").unwrap();
        let cn = game(&root, "_cn_beta_", "wow_classic_beta_cn");
        saved(&cn, "33333#1", "HeadHunter_DB = {\n[\"meta\"] = {\n[\"client\"] = \"forever\",\n},\n}\n");
        let beta = game(&root, "_classic_beta_", "wow_classic_beta");
        saved(&beta, "44444#1", "HeadHunter_DB = {}\n");
        let unnamed_beta = root.join("_beta_");
        fs::create_dir_all(unnamed_beta.join("WTF")).unwrap();
        saved(&unnamed_beta, "45454#1", "HeadHunter_DB = {}\n");
        let release = game(&root, "_classic_forever_", "wow_classic_forever");
        fs::write(release.join("WTF").join("Config.wtf"), "SET portal \"test\"\n").unwrap();
        fs::create_dir_all(release.join("Interface").join("Addons").join("HeadHunter")).unwrap();
        fs::write(release.join("Interface").join("Addons").join("HeadHunter").join("HeadHunter.toc"), "## Version: 0.3.8\n").unwrap();
        let forever = game(&root, "_forever_", "wowf");
        fs::create_dir_all(forever.join("Interface").join("AddOns").join("HeadHunter")).unwrap();
        saved(&forever, "77777#1", "HeadHunter_DB = {\n[\"meta\"] = {\n[\"client\"] = \"forever\",\n},\n}\n");
        fs::create_dir_all(forever.join("WTF").join("Account").join("88888#1").join("SavedVariables")).unwrap();
        saved(&game(&root, "_retail_", "wow"), "55555#1", "HeadHunter_DB = {}\n");
        saved(&game(&root, "_classic_era_ptr_", "wow_classic_era_ptr"), "66666#1", "HeadHunter_DB = {}\n");
        let anniversary = game(&root, "_anniversary_", "wow_anniversary");
        saved(&anniversary, "99999#1", "HeadHunter_DB = {}\n");

        let installs: Vec<Install> = find(&[cn.clone()]).into_iter().filter(|i| i.path.starts_with(&root)).collect();
        let names: Vec<String> = installs.iter().map(|i| i.path.file_name().unwrap().to_string_lossy().to_string()).collect();
        assert_eq!(names, ["_anniversary_", "_classic_forever_", "_forever_"], "picking a game folder finds its siblings; Classic Era and the beta games are left out");

        assert_eq!(installs[0].client, None, "not saved by the addon yet, and not a WoW Forever product");
        assert_eq!(installs[1].client, Some(Client::Forever), "the release product syncs as WoW Forever before the addon saved it");
        assert_eq!(installs[1].game_region.as_deref(), Some("us"), "the test portal says nothing, the branch does");
        assert_eq!(installs[1].addon_version.as_deref(), Some("0.3.8"));
        assert!(installs[1].accounts.is_empty(), "addon installed, no data yet");

        assert_eq!(installs[2].client, Some(Client::Forever));
        assert_eq!(installs[2].accounts.len(), 1, "only accounts that have HeadHunter data");
        assert_eq!(installs[2].accounts[0].name, "77777#1");
        assert_eq!(installs[2].game_region.as_deref(), Some("eu"), "the wowf branch");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn shows_a_wow_forever_game_before_the_addon_is_installed() {
        let root = std::env::temp_dir().join(format!("hh-wow-new-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let forever = game(&root, "_forever_", "wowf");
        game(&root, "_retail_", "wow");

        let installs = find_in(&root);
        assert_eq!(installs.len(), 1, "the WoW Forever game, though HeadHunter is not there yet");
        assert_eq!(installs[0].client, Some(Client::Forever));
        assert!(!installs[0].has_addon, "not synced until the addon is installed");

        fs::create_dir_all(forever.join("Interface").join("AddOns").join("HeadHunter")).unwrap();
        fs::write(forever.join("Interface").join("AddOns").join("HeadHunter").join("HeadHunter.toc"), "## Version: 1.0.2\n").unwrap();
        assert!(find_in(&root)[0].has_addon, "synced once it is");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn finds_the_wow_folder_near_any_folder_the_player_picks() {
        let drive = std::env::temp_dir().join(format!("hh-wow-pick-{}", std::process::id()));
        let wow = drive.join("Games").join("Battle.net").join("World of Warcraft");
        let forever = game(&wow, "_forever_", "wowf");
        fs::create_dir_all(forever.join("Interface").join("AddOns")).unwrap();

        assert_eq!(wow_folder_near(&wow).as_deref(), Some(wow.as_path()), "the WoW folder itself");
        assert_eq!(wow_folder_near(&forever).as_deref(), Some(wow.as_path()), "its game folder");
        assert_eq!(wow_folder_near(&forever.join("Interface").join("AddOns")).as_deref(), Some(wow.as_path()), "a folder inside the game");
        assert_eq!(wow_folder_near(&drive.join("Games").join("Battle.net")).as_deref(), Some(wow.as_path()), "the Battle.net folder above it");
        assert_eq!(wow_folder_near(&drive.join("Games")).as_deref(), Some(wow.as_path()), "two levels above it");
        let near_the_root = PathBuf::from(format!("hh-wow-root-{}", std::process::id()));
        let shallow = near_the_root.join("Battle.net").join("World of Warcraft");
        game(&shallow, "_forever_", "wowf");
        assert_eq!(
            wow_folder_near(&near_the_root.join("Battle.net")).as_deref(),
            Some(shallow.as_path()),
            "a folder near the drive's root, where the folders above run out before LEVELS_UP (G:\\Games\\Battle.net)"
        );
        let _ = fs::remove_dir_all(&near_the_root);

        let era_only = drive.join("Era").join("World of Warcraft");
        game(&era_only, "_classic_era_", "wow_classic_era");
        assert_eq!(wow_folder_near(&era_only), None, "a WoW folder without WoW Forever is not kept");
        assert_eq!(wow_folder_near(&drive.join("Nothing")), None, "nor a folder with no game near it");
        let _ = fs::remove_dir_all(drive);
    }

    #[test]
    fn knows_the_wow_forever_products() {
        assert!(is_forever(Some("wowf")));
        assert!(is_forever(Some("wow_forever")));
        assert!(is_forever(Some("wow_classic_forever")));
        assert!(!is_forever(Some("wow_forever_beta")), "a beta is never synced");
        assert!(!is_forever(Some("wow_classic_era")));
        assert!(!is_forever(Some("wow_anniversary")));
        assert!(!is_forever(None));
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
