//! The screenshots the addon takes at a death or a catch (HH-132): it saves each one in
//! `screenshots.shots` of its home, and the game writes the picture to the
//! `Screenshots` folder of the install. Here they are found, made into small WebP files
//! for the website and, when the player wants it, deleted after the upload.

use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::payload::{self, Character, Client, Context, PayloadError, Sent};

/// The game names a file by the PC's clock, which may tick on between the addon's call
/// and the write.
pub const MATCH_SECONDS: i64 = 3;
pub const MAX_WIDTH: u32 = 1280;
/// Only the middle of the screen goes up (author, 2026-10-05): the fight is there, and
/// the chat, the minimap and the action bars stay on the player's PC.
pub const CROP_WIDTH_SHARE: f64 = 0.6;
pub const CROP_HEIGHT_SHARE: f64 = 0.7;
pub const WEBP_QUALITY: f32 = 75.0;
pub const MAX_BYTES: usize = 1_000_000;
/// A picture still too big is tried again this much lower, down to `MIN_QUALITY`.
const QUALITY_STEP: f32 = 15.0;
const MIN_QUALITY: f32 = 30.0;
/// A shot whose file is not there after this long is given up.
pub const MAX_AGE_SECONDS: i64 = 7 * 24 * 60 * 60;
/// The uploaded files the app remembers, so it never uploads or deletes a file twice.
pub const KEEP_UPLOADED: usize = 200;
/// Next to `WTF` and `Interface` in the install folder.
const FOLDER: &str = "Screenshots";
const NAME_PREFIX: &str = "WoWScrnShot_";
/// The game's screenshot formats (its `screenshotFormat` setting).
const EXTENSIONS: [&str; 3] = ["jpg", "tga", "png"];
const SHOTS_FIELD: &str = "screenshots";
const SECONDS_PER_DAY: i64 = 24 * 60 * 60;
/// The game writes two-digit years.
const CENTURY: i64 = 2000;

/// One screenshot to upload, with the character that took it.
#[derive(Debug, Clone, PartialEq)]
pub struct Shot {
    pub t: i64,
    /// The game's file name without extension, e.g. "WoWScrnShot_100526_141305".
    pub file: String,
    pub kinds: Vec<String>,
    pub refs: Vec<String>,
    /// The killer or the outlaw: name and Era realm.
    pub target: Option<(String, Option<String>)>,
    /// Our character's player key.
    pub hunter: String,
    pub map_id: Option<i64>,
    pub character: Character,
}

pub fn folder(install: &Path) -> PathBuf {
    install.join(FOLDER)
}

/// The shots of one home not handled yet, oldest first. A shot of a character the file
/// does not know is left out: the website could not file it.
pub fn pending(db: &Value, ctx: &Context, sent: impl Fn(&str) -> Sent) -> Result<Vec<Shot>, PayloadError> {
    let (client, characters) = payload::characters(db, ctx)?;
    let mut shots: Vec<Shot> = payload::values(&db[SHOTS_FIELD]["shots"])
        .filter_map(|shot| {
            let hunter = payload::text(&shot["hunter"])?;
            let character = characters.get(&hunter)?.clone();
            let t = payload::int(&shot["t"])?;
            let already = sent(&hunter);
            if t <= already.screenshots || already.screenshots_done.contains(&t) {
                return None;
            }
            Some(Shot {
                t,
                file: payload::text(&shot["file"])?,
                kinds: texts(&shot["kinds"]),
                refs: texts(&shot["refs"]),
                target: payload::text(&shot["target"])
                    .and_then(|key| payload::player_ref(&key, client))
                    .map(|player| (player.name, player.realm)),
                hunter,
                map_id: payload::int(&shot["mapID"]).filter(|m| *m >= 1),
                character,
            })
        })
        .collect();
    shots.sort_by_key(|shot| shot.t);
    Ok(shots)
}

fn texts(list: &Value) -> Vec<String> {
    payload::values(list).filter_map(payload::text).collect()
}

/// Moves the watermark: `done` shots are handled; `waiting` ones are tried next time, so
/// the watermark stays below the oldest of them.
pub fn advance(sent: &mut Sent, done: impl IntoIterator<Item = i64>, waiting: &[i64]) {
    sent.screenshots_done.extend(done);
    let floor = waiting.iter().min().copied();
    if let Some(newest) = sent.screenshots_done.iter().copied().filter(|t| floor.is_none_or(|floor| *t < floor)).max() {
        sent.screenshots = sent.screenshots.max(newest);
    }
    let watermark = sent.screenshots;
    sent.screenshots_done.retain(|t| *t > watermark);
}

/// Seconds of the name's own clock (the PC's local time) for "WoWScrnShot_MMDDYY_HHMMSS";
/// only differences between two names mean something.
pub fn name_time(stem: &str) -> Option<i64> {
    let rest = stem.strip_prefix(NAME_PREFIX)?;
    let (date, time) = rest.split_once('_')?;
    let field = |part: &str, at: usize| -> Option<i64> {
        let digits = part.get(at..at + 2)?;
        digits.bytes().all(|b| b.is_ascii_digit()).then(|| digits.parse().ok())?
    };
    if date.len() != 6 || time.len() != 6 {
        return None;
    }
    let (month, day, year) = (field(date, 0)?, field(date, 2)?, field(date, 4)?);
    let (hour, minute, second) = (field(time, 0)?, field(time, 2)?, field(time, 4)?);
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) || hour > 23 || minute > 59 || second > 60 {
        return None;
    }
    Some(days_from_civil(CENTURY + year, month, day) * SECONDS_PER_DAY + hour * 3600 + minute * 60 + second)
}

/// Days since 1970-01-01 (Howard Hinnant's algorithm), so names across midnight or a
/// month end still compare.
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let day_of_year = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// The file name (with extension) among `names` closest in time to the shot's `file`,
/// within `MATCH_SECONDS`. Names that are not the game's screenshots never match.
pub fn closest<'a>(names: impl IntoIterator<Item = &'a str>, file: &str) -> Option<&'a str> {
    let wanted = name_time(file)?;
    names
        .into_iter()
        .filter_map(|name| {
            let (stem, extension) = name.rsplit_once('.')?;
            if !EXTENSIONS.iter().any(|e| e.eq_ignore_ascii_case(extension)) {
                return None;
            }
            let gap = (name_time(stem)? - wanted).abs();
            (gap <= MATCH_SECONDS).then_some((gap, name))
        })
        .min_by_key(|(gap, _)| *gap)
        .map(|(_, name)| name)
}

/// The shot's picture in `dir`, leaving out files already uploaded.
pub fn find(dir: &Path, file: &str, uploaded: &[String]) -> Option<PathBuf> {
    let names: Vec<String> = std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| !uploaded.contains(&dir.join(name).to_string_lossy().to_string()))
        .collect();
    closest(names.iter().map(String::as_str), file).map(|name| dir.join(name))
}

/// The middle `CROP_WIDTH_SHARE` x `CROP_HEIGHT_SHARE` of the picture.
pub fn center(picture: &image::DynamicImage) -> image::DynamicImage {
    let width = ((f64::from(picture.width()) * CROP_WIDTH_SHARE).round() as u32).max(1);
    let height = ((f64::from(picture.height()) * CROP_HEIGHT_SHARE).round() as u32).max(1);
    let x = (picture.width() - width) / 2;
    let y = (picture.height() - height) / 2;
    picture.crop_imm(x, y, width, height)
}

/// The middle of the picture as a lossy WebP at most `MAX_WIDTH` wide and `MAX_BYTES` big.
pub fn to_webp(path: &Path) -> Result<Vec<u8>, String> {
    let picture = center(&image::open(path).map_err(|e| e.to_string())?);
    let picture = if picture.width() > MAX_WIDTH {
        let height = (u64::from(picture.height()) * u64::from(MAX_WIDTH) / u64::from(picture.width())).max(1) as u32;
        picture.resize_exact(MAX_WIDTH, height, image::imageops::FilterType::Lanczos3)
    } else {
        picture
    };
    // Screenshots have nothing to see through; RGB keeps the file smaller
    let rgb = picture.to_rgb8();
    let mut quality = WEBP_QUALITY;
    loop {
        let bytes = webp::Encoder::from_rgb(&rgb, rgb.width(), rgb.height()).encode(quality).to_vec();
        if bytes.len() <= MAX_BYTES {
            return Ok(bytes);
        }
        if quality - QUALITY_STEP < MIN_QUALITY {
            return Err(format!("{} is {} bytes as WebP", path.display(), bytes.len()));
        }
        quality -= QUALITY_STEP;
    }
}

/// The text fields of the upload, in order; the picture goes as `image`.
pub fn fields(shot: &Shot) -> Vec<(String, String)> {
    let character = &shot.character;
    let mut out: Vec<(String, String)> = vec![
        ("character[client]".into(), payload::client_name(character.client).into()),
        ("character[region]".into(), character.region.clone()),
    ];
    let optional = [
        ("character[realm]", character.realm.clone()),
        ("character[realm_type]", character.realm_type.clone()),
    ];
    out.extend(optional.into_iter().filter_map(|(name, value)| Some((name.to_string(), value?))));
    out.push(("character[name]".into(), character.name.clone()));
    if let (Client::Forever, Some(server)) = (character.client, character.server) {
        out.push(("character[server]".into(), server.to_string()));
    }
    out.push(("t".into(), shot.t.to_string()));
    out.extend(shot.kinds.iter().map(|kind| ("kinds[]".to_string(), kind.clone())));
    out.extend(shot.refs.iter().map(|id| ("refs[]".to_string(), id.clone())));
    if let Some((name, realm)) = &shot.target {
        out.push(("target[name]".into(), name.clone()));
        if let Some(realm) = realm {
            out.push(("target[realm]".into(), realm.clone()));
        }
    }
    if let Some(map_id) = shot.map_id {
        out.push(("map_id".into(), map_id.to_string()));
    }
    out
}

/// The file name the website gets.
pub fn upload_name(shot: &Shot) -> String {
    format!("{}.webp", shot.file)
}

/// Remembers an uploaded file, keeping the newest `KEEP_UPLOADED`.
pub fn remember(uploaded: &mut Vec<String>, path: &Path) {
    uploaded.push(path.to_string_lossy().to_string());
    let extra = uploaded.len().saturating_sub(KEEP_UPLOADED);
    uploaded.drain(..extra);
}

/// Deletes the picture only when the setting is on and it is one the app uploaded.
pub fn delete_uploaded(path: &Path, uploaded: &[String], enabled: bool) -> bool {
    enabled && uploaded.contains(&path.to_string_lossy().to_string()) && std::fs::remove_file(path).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    use serde_json::json;

    fn ctx() -> Context {
        Context { client: Some(Client::Era), region: Some("eu".into()), realm_type: "pvp".into(), known_servers: BTreeSet::new(), addon_version: "0.4.3".into() }
    }

    /// Made-up players only.
    fn db() -> Value {
        json!({
            "meta": { "client": "era", "region": "eu", "player": { "key": "Tessa-Firemaw", "realm": "Firemaw", "level": 30 } },
            "screenshots": { "shots": [
                { "t": 300, "file": "WoWScrnShot_100526_141305", "kinds": ["death", "wanted"], "refs": ["Tessa-Firemaw:300"],
                  "target": "Brute-Firemaw", "hunter": "Tessa-Firemaw", "mapID": 1426 },
                { "t": 100, "file": "WoWScrnShot_100526_120000", "kinds": ["bully"], "target": "Brute-Firemaw", "hunter": "Tessa-Firemaw" },
                { "t": 200, "file": "WoWScrnShot_100526_130000", "kinds": ["death"], "hunter": "Rowena-Firemaw" }
            ] }
        })
    }

    #[test]
    fn reads_the_shots_not_sent_yet_with_their_character() {
        let shots = pending(&db(), &ctx(), |_| Sent::default()).unwrap();
        assert_eq!(shots.iter().map(|s| s.t).collect::<Vec<_>>(), vec![100, 300], "unknown characters are left out, oldest first");
        let shot = &shots[1];
        assert_eq!(shot.kinds, vec!["death", "wanted"]);
        assert_eq!(shot.refs, vec!["Tessa-Firemaw:300"]);
        assert_eq!(shot.target, Some(("Brute".into(), Some("Firemaw".into()))));
        assert_eq!(shot.character.name, "Tessa");
        assert_eq!(shot.character.realm.as_deref(), Some("Firemaw"));
        assert_eq!(shot.map_id, Some(1426));
        assert!(shots[0].refs.is_empty());

        let sent = Sent { screenshots: 100, screenshots_done: [300].into(), ..Sent::default() };
        assert!(pending(&db(), &ctx(), |_| sent.clone()).unwrap().is_empty());
    }

    #[test]
    fn the_watermark_stays_below_a_shot_that_waits() {
        let mut sent = Sent::default();
        advance(&mut sent, [100, 300], &[200]);
        assert_eq!(sent.screenshots, 100);
        assert_eq!(sent.screenshots_done, [300].into());
        advance(&mut sent, [200], &[]);
        assert_eq!(sent.screenshots, 300);
        assert!(sent.screenshots_done.is_empty());
    }

    #[test]
    fn reads_the_time_in_a_name() {
        let base = name_time("WoWScrnShot_100526_141305").unwrap();
        assert_eq!(name_time("WoWScrnShot_100526_141307").unwrap() - base, 2);
        assert_eq!(name_time("WoWScrnShot_100626_000000").unwrap() - name_time("WoWScrnShot_100526_235959").unwrap(), 1, "across midnight");
        assert_eq!(name_time("WoWScrnShot_100126_000000").unwrap() - name_time("WoWScrnShot_093026_235959").unwrap(), 1, "across a month");
        for bad in ["WoWScrnShot_1005_141305", "WoWScrnShot_130526_141305", "WoWScrnShot_10x526_141305", "Holiday_100526_141305", "WoWScrnShot_100526_141305_2"] {
            assert_eq!(name_time(bad), None, "{bad}");
        }
    }

    #[test]
    fn matches_the_closest_file_within_three_seconds() {
        let names = [
            "WoWScrnShot_100526_141309.jpg",
            "WoWScrnShot_100526_141307.jpg",
            "WoWScrnShot_100526_141306.png",
            "WoWScrnShot_100526_141305.txt",
            "WoWScrnShot_100526_141305 copy.jpg",
            "my picture.jpg",
        ];
        assert_eq!(closest(names, "WoWScrnShot_100526_141305"), Some("WoWScrnShot_100526_141306.png"));
        assert_eq!(closest(["WoWScrnShot_100526_141309.jpg"], "WoWScrnShot_100526_141305"), None, "too far");
        assert_eq!(closest(["WoWScrnShot_100526_141305.TGA"], "WoWScrnShot_100526_141305"), Some("WoWScrnShot_100526_141305.TGA"));
        assert_eq!(closest(names, "not a name"), None);
    }

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("hh-sync-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn finds_a_file_but_not_one_already_uploaded() {
        let dir = temp_dir("find");
        let old = dir.join("WoWScrnShot_100526_141304.jpg");
        let new = dir.join("WoWScrnShot_100526_141306.jpg");
        std::fs::write(&old, b"x").unwrap();
        std::fs::write(&new, b"x").unwrap();
        assert_eq!(find(&dir, "WoWScrnShot_100526_141305", &[]), Some(old.clone()));
        let uploaded = vec![old.to_string_lossy().to_string()];
        assert_eq!(find(&dir, "WoWScrnShot_100526_141305", &uploaded), Some(new));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn makes_a_small_webp() {
        let dir = temp_dir("webp");
        let path = dir.join("WoWScrnShot_100526_141305.jpg");
        let picture = image::RgbImage::from_fn(2560, 1440, |x, y| image::Rgb([(x % 256) as u8, (y % 256) as u8, ((x ^ y) % 256) as u8]));
        picture.save(&path).unwrap();
        let bytes = to_webp(&path).unwrap();
        assert!(bytes.len() <= MAX_BYTES, "{} bytes", bytes.len());
        assert_eq!(&bytes[..4], b"RIFF");
        assert_eq!(&bytes[8..12], b"WEBP");
        let decoded = webp::Decoder::new(&bytes).decode().unwrap();
        // The middle 1536 x 1008 of 2560 x 1440, then 1280 wide
        assert_eq!((decoded.width(), decoded.height()), (1280, 840));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn keeps_only_the_middle_of_the_screen() {
        // Chat in the bottom left corner (red), the fight in the middle (green)
        let picture = image::RgbImage::from_fn(1000, 1000, |x, y| {
            if x < 150 && y > 850 { image::Rgb([255, 0, 0]) } else { image::Rgb([0, 255, 0]) }
        });
        let middle = center(&image::DynamicImage::ImageRgb8(picture)).to_rgb8();
        assert_eq!((middle.width(), middle.height()), (600, 700));
        assert!(middle.pixels().all(|p| p[0] == 0), "no chat left in the picture");
    }

    #[test]
    fn never_makes_a_small_picture_bigger() {
        let dir = temp_dir("small");
        let path = dir.join("WoWScrnShot_100526_141305.png");
        image::RgbImage::from_pixel(800, 600, image::Rgb([20, 40, 60])).save(&path).unwrap();
        let decoded_size = {
            let bytes = to_webp(&path).unwrap();
            let decoded = webp::Decoder::new(&bytes).decode().unwrap();
            (decoded.width(), decoded.height())
        };
        assert_eq!(decoded_size, (480, 420), "the middle of 800 x 600, not made bigger");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn sends_the_shot_and_the_same_character_as_the_upload() {
        let shot = pending(&db(), &ctx(), |_| Sent::default()).unwrap().pop().unwrap();
        let fields = fields(&shot);
        let pairs: Vec<(&str, &str)> = fields.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
        assert_eq!(
            pairs,
            vec![
                ("character[client]", "era"),
                ("character[region]", "eu"),
                ("character[realm]", "Firemaw"),
                ("character[name]", "Tessa"),
                ("t", "300"),
                ("kinds[]", "death"),
                ("kinds[]", "wanted"),
                ("refs[]", "Tessa-Firemaw:300"),
                ("target[name]", "Brute"),
                ("target[realm]", "Firemaw"),
                ("map_id", "1426"),
            ]
        );
        assert_eq!(upload_name(&shot), "WoWScrnShot_100526_141305.webp");
    }

    #[test]
    fn forever_sends_the_realm_type_and_server() {
        let db = json!({
            "meta": { "client": "forever", "region": "us", "player": { "key": "Corin Vale", "level": 12, "server": 4619 } },
            "screenshots": { "shots": [ { "t": 50, "file": "WoWScrnShot_100526_141305", "kinds": ["deadbeat"], "target": "Brute Stone", "hunter": "Corin Vale" } ] }
        });
        let context = Context { client: Some(Client::Forever), ..ctx() };
        let shot = pending(&db, &context, |_| Sent::default()).unwrap().pop().unwrap();
        let names: Vec<String> = fields(&shot).into_iter().map(|(k, v)| format!("{k}={v}")).collect();
        assert_eq!(
            names,
            vec!["character[client]=forever", "character[region]=us", "character[realm_type]=pvp", "character[name]=Corin Vale", "character[server]=4619", "t=50", "kinds[]=deadbeat", "target[name]=Brute Stone"]
        );
    }

    #[test]
    fn deletes_only_the_uploaded_file_and_only_when_asked() {
        let dir = temp_dir("delete");
        let uploaded_file = dir.join("WoWScrnShot_100526_141305.jpg");
        let other = dir.join("WoWScrnShot_100526_141306.jpg");
        std::fs::write(&uploaded_file, b"x").unwrap();
        std::fs::write(&other, b"x").unwrap();
        let mut uploaded = Vec::new();
        remember(&mut uploaded, &uploaded_file);

        assert!(!delete_uploaded(&uploaded_file, &uploaded, false));
        assert!(uploaded_file.exists(), "the setting is off");
        assert!(!delete_uploaded(&other, &uploaded, true));
        assert!(other.exists(), "never uploaded");
        assert!(delete_uploaded(&uploaded_file, &uploaded, true));
        assert!(!uploaded_file.exists());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn remembers_only_the_newest_uploads() {
        let mut uploaded = Vec::new();
        for n in 0..KEEP_UPLOADED + 5 {
            remember(&mut uploaded, Path::new(&format!("shot{n}.jpg")));
        }
        assert_eq!(uploaded.len(), KEEP_UPLOADED);
        assert_eq!(uploaded[0], "shot5.jpg");
    }
}
