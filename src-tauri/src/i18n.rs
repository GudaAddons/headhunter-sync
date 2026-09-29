//! The app's languages, like the website (WEB-100): the English text is the key, and
//! `src/locales/zh_CN.json` holds the Chinese for every key, for the window and for the
//! texts made here (tray, notifications, messages). Values go in as `:name`.
//! The language is the Language setting; "auto" follows the system, which the window
//! reports (`set_system_language`) and the settings keep for the next start.

use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

pub const AUTO: &str = "auto";
pub const ENGLISH: &str = "en";
pub const CHINESE: &str = "zh_CN";

static CURRENT: RwLock<&'static str> = RwLock::new(ENGLISH);

fn chinese() -> &'static HashMap<String, String> {
    static TEXTS: OnceLock<HashMap<String, String>> = OnceLock::new();
    TEXTS.get_or_init(|| serde_json::from_str(include_str!("../../src/locales/zh_CN.json")).expect("src/locales/zh_CN.json"))
}

/// One of our languages for a setting or a system language ("zh-CN", "zh_TW", "en-US").
pub fn supported(language: &str) -> &'static str {
    if language.to_ascii_lowercase().starts_with("zh") {
        CHINESE
    } else {
        ENGLISH
    }
}

/// The language to use: the setting, or the system's when it is "auto".
pub fn resolve(setting: &str, system: Option<&str>) -> &'static str {
    if setting == AUTO {
        supported(system.unwrap_or(ENGLISH))
    } else {
        supported(setting)
    }
}

pub fn set(language: &str) {
    *CURRENT.write().unwrap() = supported(language);
}

pub fn current() -> &'static str {
    *CURRENT.read().unwrap()
}

/// For the website, so its messages come in the same language.
pub fn accept_language() -> &'static str {
    if current() == CHINESE { "zh-CN" } else { "en" }
}

pub fn t(text: &str) -> String {
    tr(text, &[])
}

pub fn tr(text: &str, values: &[(&str, &str)]) -> String {
    translate(current(), text, values)
}

fn translate(language: &str, text: &str, values: &[(&str, &str)]) -> String {
    let mut out = match language {
        CHINESE => chinese().get(text).cloned().unwrap_or_else(|| text.to_string()),
        _ => text.to_string(),
    };
    // Longest name first, so ":names" is not cut by ":name"
    let mut values = values.to_vec();
    values.sort_by_key(|(name, _)| std::cmp::Reverse(name.len()));
    for (name, value) in values {
        out = out.replace(&format!(":{name}"), value);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    use std::path::Path;

    #[test]
    fn picks_the_language() {
        assert_eq!(supported("zh-CN"), CHINESE);
        assert_eq!(supported("zh_TW"), CHINESE);
        assert_eq!(supported("en-US"), ENGLISH);
        assert_eq!(supported("de-DE"), ENGLISH);
        assert_eq!(resolve(AUTO, Some("zh-CN")), CHINESE);
        assert_eq!(resolve(AUTO, None), ENGLISH);
        assert_eq!(resolve(ENGLISH, Some("zh-CN")), ENGLISH);
        assert_eq!(resolve(CHINESE, Some("en-US")), CHINESE);
    }

    #[test]
    fn translates_with_values() {
        assert_eq!(translate(ENGLISH, "Cannot reach :url. Is it online?", &[("url", "x.com")]), "Cannot reach x.com. Is it online?");
        assert_eq!(translate(CHINESE, "Sync now", &[]), chinese()["Sync now"]);
        assert_eq!(translate(CHINESE, "Not a key", &[]), "Not a key");
    }

    /// Every text in the window (`$t('...')`, `trans('...')`) and here (`t("...")`,
    /// `tr("...", ...)`) has its Chinese with the same values, and no Chinese is unused.
    #[test]
    fn every_text_has_chinese() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut keys = BTreeSet::new();
        for file in files(&root.join("../src"), &["vue", "ts"]) {
            if file.to_string_lossy().replace('\\', "/").contains("/components/ui/") {
                continue;
            }
            let text = std::fs::read_to_string(&file).unwrap();
            keys.extend(literals(&text, &["$t(", "trans("], '\''));
        }
        for file in files(&root.join("src"), &["rs"]).into_iter().filter(|f| !f.ends_with("i18n.rs")) {
            let text = std::fs::read_to_string(&file).unwrap();
            keys.extend(literals(&text, &["t(", "tr("], '"'));
        }

        let chinese = chinese();
        let missing: Vec<_> = keys.iter().filter(|key| !chinese.contains_key(*key)).collect();
        assert!(missing.is_empty(), "no Chinese in src/locales/zh_CN.json for: {missing:#?}");
        let unused: Vec<_> = chinese.keys().filter(|key| !keys.contains(*key)).collect();
        assert!(unused.is_empty(), "unused Chinese in src/locales/zh_CN.json: {unused:#?}");
        for (key, value) in chinese {
            assert_eq!(placeholders(key), placeholders(value), "the values differ in the Chinese of {key:?}");
        }
    }

    fn files(dir: &Path, extensions: &[&str]) -> Vec<std::path::PathBuf> {
        let mut out = Vec::new();
        for entry in std::fs::read_dir(dir).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                out.extend(files(&path, extensions));
            } else if path.extension().and_then(|e| e.to_str()).is_some_and(|e| extensions.contains(&e)) {
                out.push(path);
            }
        }
        out
    }

    /// The first string argument after each call name; `t(` must not end a longer name.
    fn literals(text: &str, calls: &[&str], quote: char) -> Vec<String> {
        let mut out = Vec::new();
        for call in calls {
            for (at, _) in text.match_indices(call) {
                let before = text[..at].chars().next_back();
                if call.starts_with(|c: char| c.is_alphabetic()) && before.is_some_and(|c| c.is_alphanumeric() || c == '_') {
                    continue;
                }
                let rest = text[at + call.len()..].trim_start();
                let Some(body) = rest.strip_prefix(quote) else { continue };
                let mut key = String::new();
                let mut chars = body.chars();
                while let Some(c) = chars.next() {
                    match c {
                        '\\' => key.extend(chars.next()),
                        c if c == quote => break,
                        c => key.push(c),
                    }
                }
                out.push(key);
            }
        }
        out
    }

    fn placeholders(text: &str) -> BTreeSet<String> {
        let mut out = BTreeSet::new();
        let mut rest = text;
        while let Some(at) = rest.find(':') {
            let name: String = rest[at + 1..].chars().take_while(|c| c.is_ascii_alphabetic() || *c == '_').collect();
            if !name.is_empty() {
                out.insert(name);
            }
            rest = &rest[at + 1..];
        }
        out
    }
}
