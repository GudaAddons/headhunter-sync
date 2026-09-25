//! The way back into the game: the website's WANTED and Duels lists and the player's
//! own records (api.rs `download`) written as a small addon next to HeadHunter,
//! `Interface\AddOns\HeadHunter_Data\` (a .toc and Data.lua). The game loads it at
//! login as addon code, so it works on WoW Forever too, where saved variables are not
//! loaded back. HeadHunter lists it as an optional dependency and reads
//! `HeadHunter_SiteData`; the data keeps the website's names, the addon maps them.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::installs;
use crate::lua;

pub const FOLDER: &str = "HeadHunter_Data";
pub const VARIABLE: &str = "HeadHunter_SiteData";
/// Used when HeadHunter's own .toc cannot be read.
const FALLBACK_INTERFACE: &str = "11509, 16001";

/// `Interface\AddOns\HeadHunter_Data` of a game folder with HeadHunter installed.
pub fn data_dir(game: &Path) -> Option<PathBuf> {
    installs::addons_dir(game).map(|dir| dir.join(FOLDER))
}

/// Whether the data file is still there (the player may have deleted the folder).
pub fn written(game: &Path) -> bool {
    data_dir(game).is_some_and(|dir| dir.join("Data.lua").is_file())
}

pub fn toc(interface: &str, version: &str) -> String {
    format!(
        "## Interface: {interface}\n\
         ## Title: HeadHunter Data\n\
         ## Notes: The HeadHunter website's WANTED and Duels lists and your own records, written by HeadHunter Sync.\n\
         ## Author: Vati\n\
         ## Version: {version}\n\
         Data.lua\n"
    )
}

pub fn data_file(data: &Value) -> String {
    format!(
        "-- Written by HeadHunter Sync; the next sync replaces it.\n{}",
        lua::write_variable(VARIABLE, data)
    )
}

/// Writes the addon; returns false when the files already held this data.
pub fn write(game: &Path, data: &Value) -> Result<bool, String> {
    let dir = data_dir(game).ok_or("HeadHunter is not installed in this game folder.")?;
    let interface = installs::addon_toc(game)
        .and_then(|toc| installs::toc_field(&toc, "Interface"))
        .unwrap_or_else(|| FALLBACK_INTERFACE.to_string());
    let files = [
        (dir.join(format!("{FOLDER}.toc")), toc(&interface, env!("CARGO_PKG_VERSION"))),
        (dir.join("Data.lua"), data_file(data)),
    ];
    if files.iter().all(|(path, text)| fs::read_to_string(path).is_ok_and(|old| &old == text)) {
        return Ok(false);
    }
    fs::create_dir_all(&dir).map_err(|e| format!("Cannot create {}: {e}", dir.display()))?;
    for (path, text) in files {
        let tmp = path.with_extension("tmp");
        fs::write(&tmp, text)
            .and_then(|_| fs::rename(&tmp, &path))
            .map_err(|e| format!("Cannot write {}: {e}", path.display()))?;
    }
    Ok(true)
}

/// WANTED entries across the worlds, for the status screen.
pub fn wanted_count(data: &Value) -> usize {
    data["worlds"]
        .as_object()
        .map(|worlds| worlds.values().map(|w| w["wanted"].as_array().map_or(0, Vec::len)).sum())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn game_with_addon(name: &str) -> PathBuf {
        let game = std::env::temp_dir().join(format!("hh-download-{name}-{}", std::process::id())).join("_classic_era_");
        let addon = game.join(r"Interface\AddOns\HeadHunter");
        fs::create_dir_all(&addon).unwrap();
        fs::write(addon.join("HeadHunter.toc"), "## Interface: 11508, 16001\n## Version: 0.1.6\n").unwrap();
        game
    }

    fn site_data() -> Value {
        json!({
            "format_version": 1,
            "generated_at": 1790300000,
            "worlds": { "era|eu|Firemaw": { "generated_at": 1790300000, "wanted": [{ "name": "Dusk Blade" }, { "name": "Grim Tusk" }], "duels": { "alliance": [], "horde": [] } } },
            "characters": []
        })
    }

    #[test]
    fn writes_the_addon_next_to_headhunter() {
        let game = game_with_addon("write");
        assert!(!written(&game));

        assert!(write(&game, &site_data()).unwrap(), "first write");
        let dir = game.join(r"Interface\AddOns\HeadHunter_Data");
        let toc = fs::read_to_string(dir.join("HeadHunter_Data.toc")).unwrap();
        assert!(toc.contains("## Interface: 11508, 16001"), "the interface follows HeadHunter's .toc");
        assert!(toc.ends_with("Data.lua\n"));
        let data = fs::read_to_string(dir.join("Data.lua")).unwrap();
        assert_eq!(lua::read_variable(&data, VARIABLE).unwrap(), site_data(), "Data.lua holds the website's answer");
        assert!(written(&game));

        assert!(!write(&game, &site_data()).unwrap(), "the same data is not written again");
        let mut changed = site_data();
        changed["generated_at"] = json!(1790300600);
        assert!(write(&game, &changed).unwrap(), "new data is written");

        let _ = fs::remove_dir_all(game.parent().unwrap());
    }

    #[test]
    fn needs_headhunter_installed() {
        let game = std::env::temp_dir().join(format!("hh-download-none-{}", std::process::id()));
        assert!(write(&game, &site_data()).is_err());
    }

    #[test]
    fn counts_wanted_across_worlds() {
        assert_eq!(wanted_count(&site_data()), 2);
        assert_eq!(wanted_count(&json!({})), 0);
    }
}
