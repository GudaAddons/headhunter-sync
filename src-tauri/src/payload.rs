//! Turns the addon's `HeadHunter_DB` into the website's sync uploads
//! (`StoreUploadRequest` in the website repo), one upload per character.
//!
//! The saved data is account-wide: own deaths name their victim, catches their hunter,
//! bounty events their hunter (addon 0.1.4+). `meta.player` is the character played
//! last; witnessed duels, learned zones, bounty events without a hunter, the deaths other
//! HeadHunters shared and the players' bounty posters and payments (addon HH-118,
//! 0.2.2+) go with it, and so do the enemies the addon saw up close (their sex, race and
//! class fill in what the website does not know about them yet).
//! Each character's own honorable kills (addon `honorKills`, by map and time) and finished
//! wars (addon `wars`, HH-136) go with that character, so a character with kills or wars
//! but no deaths uploads too.
//! Demo and simulated records are never sent.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const FORMAT_VERSION: u8 = 1;
pub const MAX_DEATHS: usize = 500;
pub const MAX_CATCHES: usize = 500;
pub const MAX_DUELS: usize = 2000;
pub const MAX_BOUNTY_EVENTS: usize = 500;
pub const MAX_ZONES: usize = 200;
pub const MAX_POSTERS: usize = 200;
pub const MAX_PAYMENTS: usize = 500;
pub const MAX_SHARED_DEATHS: usize = 500;
pub const MAX_EVENT_RESULTS: usize = 200;
/// The addon keeps at most 2000 enemies (its Core/Database.lua).
pub const MAX_KNOWN_PLAYERS: usize = 2000;
pub const MAX_GLASSES: usize = 500;
/// The website takes at most 2000 a upload (`StoreUploadRequest::MAX_HONOR_KILLS`).
pub const MAX_HONOR_KILLS: usize = 2000;
/// The website takes at most 100 wars an upload (`StoreUploadRequest::MAX_WARS`).
pub const MAX_WARS: usize = 100;
/// The website takes at most 500 witness records an upload (`StoreUploadRequest::MAX_WITNESSES`).
pub const MAX_WITNESSES: usize = 500;
/// The addon keeps at most 300 fighters a war (its Alerts/Wars.lua).
pub const MAX_WAR_FIGHTERS: usize = 300;
/// The addon's field for the Forever server number (`meta.player`, `deaths[].victim`).
const SERVER_FIELD: &str = "server";
/// The screenshot statuses the addon writes on a death (Sync/Screenshots.lua STATUS).
const SHOT_STATUSES: [&str; 5] = ["taken", "off", "limit", "no_app", "failed"];
pub const MIN_SERVER: i64 = 1;
/// Addon schema 2 keeps each home's tables under `homes["forever|4619"]`.
const HOMES_FIELD: &str = "homes";
/// The region, in `meta` for the account and, from addon 0.5.1, in each home.
const REGION_FIELD: &str = "region";
const META_FIELD: &str = "meta";
const PLAYER_FIELD: &str = "player";
/// `meta.home`: the home played last.
const HOME_FIELD: &str = "home";
/// Map names stay at the root in every schema.
const ZONES_FIELD: &str = "zones";
/// The game's faction on a character, an enemy entry or a duel ("Alliance" or "Horde").
const FACTION_FIELD: &str = "faction";
const ALLIANCE: &str = "alliance";
const HORDE: &str = "horde";
const FACTIONS: [&str; 2] = [ALLIANCE, HORDE];
/// The addon's race tokens, the website's names and the faction the race tells
/// (none when both factions can play it).
const RACES: &[(&str, &str, Option<&str>)] = &[
    ("Human", "human", Some(ALLIANCE)),
    ("Dwarf", "dwarf", Some(ALLIANCE)),
    ("NightElf", "night_elf", Some(ALLIANCE)),
    ("Gnome", "gnome", Some(ALLIANCE)),
    ("Draenei", "draenei", Some(ALLIANCE)),
    ("Orc", "orc", Some(HORDE)),
    ("Scourge", "undead", Some(HORDE)),
    ("Undead", "undead", Some(HORDE)),
    ("Tauren", "tauren", Some(HORDE)),
    ("Troll", "troll", Some(HORDE)),
    ("BloodElf", "blood_elf", Some(HORDE)),
    ("Skyborne", "skyborne", None),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Client {
    Era,
    Forever,
}

/// What the file does not say but the upload needs.
#[derive(Debug, Clone)]
pub struct Context {
    /// The install's client (another saved file's `meta.client`); the file's own wins.
    pub client: Option<Client>,
    /// Used when the addon has not saved `meta.region` yet: the setting, else the game's region.
    pub region: Option<String>,
    /// Forever worlds (pvp, pve, roleplay, hardcore): the addon cannot tell.
    pub realm_type: String,
    /// Forever server numbers the website knows (`forever_servers` of its last download).
    pub known_servers: BTreeSet<i64>,
    pub addon_version: String,
}

/// What was already sent for one character: the newest time per list, zones by id.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Sent {
    pub deaths: i64,
    pub catches: i64,
    pub duels: i64,
    pub bounty: i64,
    #[serde(default)]
    pub zones: BTreeSet<i64>,
    #[serde(default)]
    pub posters: i64,
    #[serde(default)]
    pub payments: i64,
    #[serde(default)]
    pub shared_deaths: i64,
    #[serde(default)]
    pub event_results: i64,
    /// The newest enemy sighting sent (the addon's `enemies[].lastSeen`).
    #[serde(default)]
    pub known_players: i64,
    /// The newest glass sent (the addon's `glasses[].t`).
    #[serde(default)]
    pub glasses: i64,
    /// The newest honorable kill sent (the addon's `honorKills[].t`).
    #[serde(default)]
    pub honor_kills: i64,
    /// The newest war end sent (the addon's `wars[].ended`).
    #[serde(default)]
    pub wars: i64,
    /// The newest witness record sent (the addon's `witness[].t`).
    #[serde(default)]
    pub witnesses: i64,
    /// The Forever server sent last: a new one is sent even with no new records, so the
    /// website files the character under its server's realm type.
    #[serde(default)]
    pub server: Option<i64>,
    /// The newest screenshot handled (the addon's `screenshots.shots[].t`): it and every
    /// older one was uploaded, refused by the website or too old.
    #[serde(default)]
    pub screenshots: i64,
    /// Screenshots handled after an older one that still waits for its file.
    #[serde(default)]
    pub screenshots_done: BTreeSet<i64>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Payload {
    pub format_version: u8,
    pub addon_version: String,
    pub character: Character,
    pub deaths: Vec<Death>,
    pub catches: Vec<Catch>,
    pub duels: Vec<Duel>,
    pub bounty_events: Vec<BountyEvent>,
    pub zones: Vec<Zone>,
    pub bounty_posters: Vec<BountyPoster>,
    pub bounty_payments: Vec<BountyPayment>,
    pub shared_deaths: Vec<SharedDeath>,
    pub event_results: Vec<EventResult>,
    /// The enemies the addon saw up close, newest sighting first: the website fills in
    /// their sex, race and class on players it already has.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub known_players: Vec<PlayerRef>,
    /// Glasses raised to catches in game, ours and other HeadHunters' (the addon's
    /// Sync/Glasses.lua): the website counts them with its own.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub glasses: Vec<Glass>,
    /// The character's own honorable kills (the addon's Detection/HonorKills.lua): the
    /// website adds them up as shootouts.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub honor_kills: Vec<HonorKill>,
    /// The character's finished wars (the addon's Alerts/Wars.lua): the website merges
    /// every HeadHunter's report of one war.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub wars: Vec<War>,
    /// The hunted players the character saw die (the addon's Sync/Witness.lua): the
    /// website takes another account's witness as proof of a catch or a bounty claim.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub witnesses: Vec<WitnessRecord>,
}

/// A hunted player the character saw die: when, where, and who landed the blow when the
/// game told (Classic Era).
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct WitnessRecord {
    pub t: i64,
    pub outlaw: PlayerRef,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub map_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub y: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub killer: Option<PlayerRef>,
}

/// One HeadHunter's record of a war: a hotzone's fight from its start to its end, the
/// players seen fighting on both sides, and our own kills, honor and deaths in it.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct War {
    pub uid: String,
    pub map_id: i64,
    pub started: i64,
    pub ended: i64,
    pub peak_fire: i64,
    pub layers: Vec<i64>,
    pub kills: i64,
    pub honor: i64,
    pub deaths: i64,
    /// Whether the character was in the war's zone during it (addon 0.5.2, HH-140); none
    /// from older addons, which also recorded wars they only heard of.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub present: Option<bool>,
    pub fighters: Vec<WarFighter>,
}

/// A player seen fighting in a war: "ally" fought for our side, "enemy" for the other.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct WarFighter {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub realm: Option<String>,
    pub side: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub faction: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub race: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guild: Option<String>,
    pub first: i64,
    pub last: i64,
}

/// An honorable kill the game gave the character: `seq` tells apart kills in the same
/// second; the victim only when the game's line names them (Classic Era).
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct HonorKill {
    pub t: i64,
    pub seq: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub map_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub y: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub layer: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub victim: Option<PlayerRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub honor: Option<i64>,
}

/// A glass raised to a catch: the catch is its outlaw and time, `by` who raised it.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Glass {
    pub outlaw: PlayerRef,
    pub caught_at: i64,
    pub by: PlayerRef,
    pub t: i64,
    /// Raised from the popup at the moment of the bust: the only glasses the Barflies
    /// ranking counts.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub popup: bool,
}

/// A tournament match result the character confirmed in game as host or co-organizer
/// (addon Tournament/Matches.lua); the website checks it is theirs to set.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct EventResult {
    pub tournament: String,
    pub round: i64,
    pub r#match: i64,
    pub entrant_a: String,
    pub entrant_b: String,
    pub wins_a: i64,
    pub wins_b: i64,
    pub forfeit: Option<String>,
    pub t: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Character {
    pub client: Client,
    pub region: String,
    pub realm: Option<String>,
    pub realm_type: Option<String>,
    /// Forever server number (the middle part of the character GUID).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server: Option<i64>,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub faction: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub race: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sex: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<i64>,
    /// `Some(None)` is sent as null: the character is in no guild (it left one).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guild: Option<Option<String>>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Default)]
pub struct PlayerRef {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub realm: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub faction: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub race: Option<String>,
    /// 2 male, 3 female (the game's UnitSex): the website shows the race icon of that gender.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sex: Option<i64>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Death {
    pub uid: String,
    pub t: i64,
    pub victim_level: i64,
    pub victim_class: Option<String>,
    pub victim_race: Option<String>,
    pub map_id: Option<i64>,
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub layer: Option<i64>,
    pub confidence: String,
    pub classification: String,
    pub attackers: Vec<Attacker>,
    /// What happened with the screenshot of this death (addon HH-132): taken, off, limit,
    /// no_app or failed; none from older addons and for deaths others shared
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shot: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Default)]
pub struct Attacker {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub realm: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub given_name: Option<String>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub skull: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub race: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sex: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub faction: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Catch {
    pub t: i64,
    pub outlaw: PlayerRef,
    pub map_id: Option<i64>,
    pub killer_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Duel {
    pub t: i64,
    pub winner: PlayerRef,
    pub loser: PlayerRef,
    pub winner_level: i64,
    pub loser_level: i64,
    pub map_id: Option<i64>,
    pub retreat: bool,
    pub faction: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct BountyEvent {
    pub t: i64,
    #[serde(rename = "type")]
    pub kind: String,
    pub bounty: i64,
    pub total_after: Option<i64>,
    pub outlaw_name: Option<String>,
    pub outlaw_rank: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outlaw: Option<PlayerRef>,
}

/// Another HeadHunter's death the addon took in (addon `db.reports`, origin peer or
/// relay): the website needs them for the same WANTED list as in the game.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SharedDeath {
    pub victim: PlayerRef,
    #[serde(flatten)]
    pub death: Death,
    /// "peer" (live from the victim) or "relay" (passed on at login catch-up)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub via: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relayed_by: Option<String>,
    /// Our character that got it, when and where (addon 0.4.2 on)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub received_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub received_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub received_map_id: Option<i64>,
}

/// A player's bounty on their killer (addon `db.posters`).
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct BountyPoster {
    pub t: i64,
    pub owner: PlayerRef,
    pub target: PlayerRef,
    pub reason: i64,
    /// Copper
    pub gold: i64,
    pub until: i64,
}

/// A bounty claimed, paid or unpaid (addon `db.bountyPay`); `t` is when it got this status.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct BountyPayment {
    pub t: i64,
    pub poster_owner: PlayerRef,
    pub poster_t: i64,
    pub hunter: PlayerRef,
    pub status: String,
    pub claimed_at: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Zone {
    pub map_id: i64,
    pub name: String,
    pub continent_map_id: Option<i64>,
    pub locale: Option<String>,
}

/// One character's upload and what it will have sent once accepted.
#[derive(Debug, Clone, PartialEq)]
pub struct CharacterUpload {
    /// The addon's player key ("Name-Realm" on Era, "Given Family" on Forever).
    pub key: String,
    pub payload: Payload,
    pub sent_after: Sent,
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum PayloadError {
    #[error("{}", crate::i18n::t("Region unknown: log in to the game once with the addon, or pick the region in Settings"))]
    UnknownRegion,
    #[error("{}", crate::i18n::t("Game unknown: log in to the game once with the addon"))]
    UnknownClient,
}

/// One home of the saved file as a flat file: its tables, the root zones and the root
/// meta with the home's own player and, from addon 0.5.1 (HH-139), the home's own region:
/// the root region is the account's last one, so a US server's characters were filed in
/// China after playing there.
#[derive(Debug, Clone, PartialEq)]
pub struct Home {
    /// "forever|4619"; `None` for an old flat file.
    pub key: Option<String>,
    /// The home played last.
    pub last: bool,
    pub db: Value,
}

/// Every home in the file (addon schema 2); an old flat file is one home without a key.
pub fn homes(db: &Value) -> Vec<Home> {
    let Some(homes) = db[HOMES_FIELD].as_object() else {
        return vec![Home { key: None, last: true, db: db.clone() }];
    };
    let last_home = text(&db[META_FIELD][HOME_FIELD]);
    homes
        .iter()
        .filter_map(|(key, home)| {
            let mut view = home.as_object()?.clone();
            let mut meta = db[META_FIELD].as_object().cloned().unwrap_or_default();
            meta.remove(PLAYER_FIELD);
            if let Some(player) = view.remove(PLAYER_FIELD) {
                meta.insert(PLAYER_FIELD.into(), player);
            }
            if let Some(region) = view.remove(REGION_FIELD).filter(Value::is_string) {
                meta.insert(REGION_FIELD.into(), region);
            }
            view.insert(META_FIELD.into(), Value::Object(meta));
            view.insert(ZONES_FIELD.into(), db[ZONES_FIELD].clone());
            Some(Home { key: Some(key.clone()), last: last_home.as_deref() == Some(key.as_str()), db: Value::Object(view) })
        })
        .collect()
}

/// Every character's upload with records newer than `sent(key)`; characters with
/// nothing new are left out.
pub fn build(db: &Value, ctx: &Context, sent: impl Fn(&str) -> Sent) -> Result<Vec<CharacterUpload>, PayloadError> {
    let (client, region) = client_and_region(db, ctx)?;
    let deaths = own_deaths(db);
    let (keys, last_player) = character_keys(db, &deaths);

    let own = keys.clone();
    let mut uploads = Vec::new();
    for key in keys {
        let is_last = last_player.as_deref() == Some(key.as_str());
        let already = sent(&key);
        let mut after = already.clone();

        let character = character(db, &key, is_last, client, &region, ctx, &deaths);

        let my_deaths = newest_first_limited(
            deaths.iter().copied().filter(|d| text(&d["victim"]["key"]).as_deref() == Some(&key)),
            already.deaths,
            MAX_DEATHS,
        );
        let deaths_out: Vec<Death> = my_deaths.iter().filter_map(|d| death(d, client)).collect();
        after.deaths = my_deaths.iter().map(|d| int(&d["t"]).unwrap_or(0)).max().unwrap_or(already.deaths).max(already.deaths);

        let catches_src = newest_first_limited(
            values(&db["justice"]).filter(|j| {
                text(&j["origin"]).as_deref() == Some("local")
                    && !flag(&j["demo"])
                    && text(&j["hunter"]).as_deref() == Some(&key)
            }),
            already.catches,
            MAX_CATCHES,
        );
        let catches: Vec<Catch> = catches_src.iter().filter_map(|j| catch(j, client)).collect();
        after.catches = max_t(&catches_src, already.catches);

        let (duels, duels_src) = if is_last {
            let src = newest_first_limited(
                // Every duel in the addon's Duels (author, 2026-09-26): our own, shared by other
                // HeadHunters and relayed at login; the website merges copies of one duel
                values(&db["duels"]).filter(|d| !from_website(d) && !flag(&d["demo"])),
                already.duels,
                MAX_DUELS,
            );
            (src.iter().filter_map(|d| duel(d, client)).collect(), src)
        } else {
            (Vec::new(), Vec::new())
        };
        after.duels = max_t(&duels_src, already.duels);

        let events_src = newest_first_limited(
            values(&db["marks"]["events"]).filter(|e| {
                !from_website(e)
                    && match text(&e["hunter"]) {
                        Some(hunter) => hunter == key,
                        None => is_last,
                    }
            }),
            already.bounty,
            MAX_BOUNTY_EVENTS,
        );
        let bounty_events: Vec<BountyEvent> = events_src.iter().filter_map(|e| bounty_event(e, client)).collect();
        after.bounty = max_t(&events_src, already.bounty);

        let zones: Vec<Zone> = if is_last {
            zone_list(db).into_iter().filter(|z| !already.zones.contains(&z.map_id)).take(MAX_ZONES).collect()
        } else {
            Vec::new()
        };
        after.zones.extend(zones.iter().map(|z| z.map_id));

        // Players' bounties (HH-118): every poster and payment the addon knows, like duels
        let (posters_src, payments_src) = if is_last {
            (
                newest_first_limited(values(&db["posters"]), already.posters, MAX_POSTERS),
                newest_first_limited(values(&db["bountyPay"]), already.payments, MAX_PAYMENTS),
            )
        } else {
            (Vec::new(), Vec::new())
        };
        let bounty_posters: Vec<BountyPoster> =
            posters_src.iter().filter(|p| !from_website(p)).filter_map(|p| bounty_poster(p, client)).collect();
        let bounty_payments: Vec<BountyPayment> = payments_src.iter().filter_map(|p| bounty_payment(p, client)).collect();
        after.posters = max_t(&posters_src, already.posters);
        after.payments = max_t(&payments_src, already.payments);

        let shared_src = if is_last {
            newest_first_limited(shared_deaths(db, &own), already.shared_deaths, MAX_SHARED_DEATHS)
        } else {
            Vec::new()
        };
        let shared_out: Vec<SharedDeath> = shared_src.iter().filter_map(|d| shared_death(d, client)).collect();
        after.shared_deaths = max_t(&shared_src, already.shared_deaths);

        // Tournament results this character confirmed as an organizer (theirs only)
        let results_src = newest_first_limited(
            values(&db["eventResults"]).filter(|r| {
                text(&r["origin"]).as_deref() == Some("local") && text(&r["organizer"]).as_deref() == Some(&key)
            }),
            already.event_results,
            MAX_EVENT_RESULTS,
        );
        let event_results: Vec<EventResult> = results_src.iter().filter_map(|r| event_result(r)).collect();
        after.event_results = max_t(&results_src, already.event_results);

        // The enemies seen since last time; and what a killer's record lacks, from them
        let enemies = &db["enemies"];
        let (known_players, known_src) = if is_last {
            let src = newest_seen_limited(enemies, already.known_players, MAX_KNOWN_PLAYERS);
            (src.iter().filter_map(|(key, e)| known_player(key, e, client)).collect(), src)
        } else {
            (Vec::new(), Vec::new())
        };
        after.known_players = known_src.iter().map(|(_, e)| int(&e["lastSeen"]).unwrap_or(0)).max().unwrap_or(0).max(already.known_players);

        // Glasses raised to catches, every HeadHunter's the addon knows, with the last character
        let glasses_src = if is_last {
            newest_first_limited(
                values(&db["glasses"]).filter(|g| text(&g["origin"]).as_deref() != Some("website")),
                already.glasses,
                MAX_GLASSES,
            )
        } else {
            Vec::new()
        };
        let glasses: Vec<Glass> = glasses_src.iter().filter_map(|g| glass(g, client)).collect();
        after.glasses = max_t(&glasses_src, already.glasses);

        let honor_src = whole_seconds_limited(
            values(&db["honorKills"]).filter(|k| text(&k["by"]).as_deref() == Some(&key)),
            already.honor_kills,
            MAX_HONOR_KILLS,
        );
        let honor_kills: Vec<HonorKill> = honor_src.iter().filter_map(|k| honor_kill(k, client)).collect();
        after.honor_kills = max_t(&honor_src, already.honor_kills);

        let witness_src = newest_first_limited(
            values(&db["witness"]).filter(|w| {
                text(&w["by"]).as_deref() == Some(&key) && text(&w["origin"]).as_deref() == Some("local")
            }),
            already.witnesses,
            MAX_WITNESSES,
        );
        let witnesses: Vec<WitnessRecord> = witness_src.iter().filter_map(|w| witness_record(w, client)).collect();
        after.witnesses = max_t(&witness_src, already.witnesses);

        let wars_src = ended_wars(&db["wars"], &key, already.wars, MAX_WARS);
        let wars: Vec<War> = wars_src.iter().filter_map(|w| war(w, client)).collect();
        after.wars = wars_src.iter().filter_map(|w| int(&w["ended"])).max().unwrap_or(already.wars).max(already.wars);
        let mut deaths_out = deaths_out;
        let mut shared_out = shared_out;
        for death in deaths_out.iter_mut().chain(shared_out.iter_mut().map(|s| &mut s.death)) {
            for attacker in death.attackers.iter_mut() {
                fill_from_enemy(attacker, enemies, client);
            }
        }

        let new_server = character.server.is_some() && character.server != already.server;
        after.server = character.server.or(already.server);

        if !new_server
            && deaths_out.is_empty()
            && shared_out.is_empty()
            && catches.is_empty()
            && duels.is_empty()
            && bounty_events.is_empty()
            && zones.is_empty()
            && bounty_posters.is_empty()
            && bounty_payments.is_empty()
            && event_results.is_empty()
            && known_players.is_empty()
            && glasses.is_empty()
            && honor_kills.is_empty()
            && wars.is_empty()
            && witnesses.is_empty()
        {
            continue;
        }

        uploads.push(CharacterUpload {
            key: key.clone(),
            payload: Payload {
                format_version: FORMAT_VERSION,
                addon_version: ctx.addon_version.clone(),
                character,
                deaths: deaths_out,
                catches,
                duels,
                bounty_events,
                zones,
                bounty_posters,
                bounty_payments,
                shared_deaths: shared_out,
                event_results,
                known_players,
                glasses,
                honor_kills,
                wars,
                witnesses,
            },
            sent_after: after,
        });
    }
    Ok(uploads)
}

/// The world key of every character in the file, as the website's download takes it:
/// "era|eu|Firemaw", "forever|us|4620" for a server the website knows, else "forever|us|pvp".
pub fn world_keys(db: &Value, ctx: &Context) -> Result<BTreeSet<String>, PayloadError> {
    let (client, characters) = characters(db, ctx)?;
    Ok(characters
        .into_values()
        .filter_map(|character| {
            let known = character.server.filter(|s| ctx.known_servers.contains(s)).map(|s| s.to_string());
            let place = character.realm.or(known).or(character.realm_type)?;
            Some(format!("{}|{}|{}", client_name(client), character.region, place))
        })
        .collect())
}

/// Every character of the file by key, named as the upload names it.
pub fn characters(db: &Value, ctx: &Context) -> Result<(Client, BTreeMap<String, Character>), PayloadError> {
    let (client, region) = client_and_region(db, ctx)?;
    let deaths = own_deaths(db);
    let (keys, last_player) = character_keys(db, &deaths);
    let characters = keys
        .into_iter()
        .map(|key| {
            let is_last = last_player.as_deref() == Some(key.as_str());
            let character = character(db, &key, is_last, client, &region, ctx, &deaths);
            (key, character)
        })
        .collect();
    Ok((client, characters))
}

pub fn client_name(client: Client) -> &'static str {
    match client {
        Client::Era => "era",
        Client::Forever => "forever",
    }
}

fn client_and_region(db: &Value, ctx: &Context) -> Result<(Client, String), PayloadError> {
    let client = match text(&db[META_FIELD]["client"]).as_deref() {
        Some("era") => Client::Era,
        Some("forever") => Client::Forever,
        _ => ctx.client.ok_or(PayloadError::UnknownClient)?,
    };
    let region = text(&db[META_FIELD]["region"]).or_else(|| ctx.region.clone()).ok_or(PayloadError::UnknownRegion)?;
    Ok((client, region))
}

/// Every character with own deaths, honorable kills or wars, plus the one played last
/// (and which one that is).
fn character_keys(db: &Value, deaths: &[&Value]) -> (BTreeSet<String>, Option<String>) {
    let last_player = text(&db[META_FIELD][PLAYER_FIELD]["key"])
        .or_else(|| deaths.iter().max_by_key(|d| int(&d["t"])).and_then(|d| text(&d["victim"]["key"])));
    let mut keys: BTreeSet<String> = deaths.iter().filter_map(|d| text(&d["victim"]["key"])).collect();
    keys.extend(values(&db["honorKills"]).filter_map(|k| text(&k["by"])));
    keys.extend(values(&db["wars"]).filter_map(|w| text(&w["by"])));
    keys.extend(last_player.clone());
    (keys, last_player)
}

fn character(db: &Value, key: &str, is_last: bool, client: Client, region: &str, ctx: &Context, deaths: &[&Value]) -> Character {
    let (name, realm) = split_key(key, client);
    let snapshot = if is_last {
        db[META_FIELD][PLAYER_FIELD].clone()
    } else {
        deaths
            .iter()
            .filter(|d| text(&d["victim"]["key"]).as_deref() == Some(key))
            .max_by_key(|d| int(&d["t"]))
            .map(|d| d["victim"].clone())
            .unwrap_or(Value::Null)
    };
    let race = text(&snapshot["race"]);
    Character {
        client,
        region: region.to_string(),
        realm: match client {
            Client::Era => text(&snapshot["realm"]).or(realm),
            Client::Forever => None,
        },
        realm_type: match client {
            Client::Forever => Some(ctx.realm_type.clone()),
            Client::Era => None,
        },
        server: match client {
            Client::Forever => int(&snapshot[SERVER_FIELD]).filter(|s| *s >= MIN_SERVER),
            Client::Era => None,
        },
        name,
        faction: player_faction(&snapshot[FACTION_FIELD], race.as_deref()),
        class: text(&snapshot["class"]).map(|c| c.to_lowercase()),
        race: race.as_deref().and_then(map_race),
        sex: sex(&snapshot["sex"]),
        level: int(&snapshot["level"]).filter(|l| (1..=100).contains(l)),
        // The last character's own record (saved at logout) also tells "no guild" (null);
        // a death's victim record only tells a guild it saw
        guild: {
            let guild = text(&snapshot["guild"]).map(|g| g.chars().take(64).collect());
            if is_last { Some(guild) } else { guild.map(Some) }
        },
    }
}

/// The account's own deaths (the `deaths` list), without demo or simulated ones and
/// without those the website sent back.
fn own_deaths(db: &Value) -> Vec<&Value> {
    values(&db["deaths"])
        .filter(|d| {
            !flag(&d["demo"])
                && !from_website(d)
                && matches!(text(&d["confidence"]).as_deref(), Some("exact") | Some("inferred"))
        })
        .collect()
}

/// Deaths other HeadHunters shared with us or passed on at login, not our own characters'.
fn shared_deaths<'a>(db: &'a Value, own: &'a BTreeSet<String>) -> impl Iterator<Item = &'a Value> + 'a {
    values(&db["reports"]).filter(move |d| {
        matches!(text(&d["origin"]).as_deref(), Some("peer") | Some("relay"))
            && !flag(&d["demo"])
            && matches!(text(&d["confidence"]).as_deref(), Some("exact") | Some("inferred"))
            && text(&d["victim"]["key"]).is_some_and(|key| !own.contains(&key))
    })
}

fn shared_death(d: &Value, client: Client) -> Option<SharedDeath> {
    let key = text(&d["victim"]["key"])?;
    let (name, realm) = split_key(&key, client);
    let race = text(&d["victim"]["race"]);
    Some(SharedDeath {
        victim: PlayerRef {
            name,
            realm,
            class: text(&d["victim"]["class"]).map(|c| c.to_lowercase()),
            faction: player_faction(&d["victim"][FACTION_FIELD], race.as_deref()),
            race: race.as_deref().and_then(map_race),
            sex: sex(&d["victim"]["sex"]),
        },
        death: death(d, client)?,
        via: text(&d["origin"]),
        relayed_by: text(&d["relayedBy"]).map(|k| split_key(&k, client).0),
        received_by: text(&d["receivedBy"]).map(|k| split_key(&k, client).0),
        received_at: int(&d["receivedAt"]),
        received_map_id: int(&d["receivedMap"]),
    })
}

/// Records the HeadHunter_Data addon brought back; the website has them already.
fn from_website(record: &Value) -> bool {
    text(&record["origin"]).as_deref() == Some("website")
}

fn death(d: &Value, client: Client) -> Option<Death> {
    let victim_level = int(&d["victim"]["level"]).filter(|l| (1..=100).contains(l))?;
    let mut attackers = Vec::new();
    if let Some(killer) = attacker(&d["killer"], "killer", client) {
        attackers.push(killer);
    }
    attackers.extend(values(&d["assists"]).filter_map(|a| attacker(a, "assist", client)));
    if attackers.is_empty() {
        return None;
    }
    attackers.truncate(10);
    let classification = text(&d["classification"])
        .filter(|c| matches!(c.as_str(), "coward" | "fair" | "giant" | "normal" | "unknown"))
        .unwrap_or_else(|| "unknown".into());
    Some(Death {
        uid: text(&d["id"]).map(|id| id.chars().take(160).collect()).unwrap_or_default(),
        t: int(&d["t"])?,
        victim_level,
        victim_class: text(&d["victim"]["class"]).map(|c| c.to_lowercase()),
        victim_race: text(&d["victim"]["race"]).as_deref().and_then(map_race),
        map_id: int(&d["mapID"]).filter(|m| *m >= 1),
        x: float(&d["x"]).filter(|v| (0.0..=1.0).contains(v)),
        y: float(&d["y"]).filter(|v| (0.0..=1.0).contains(v)),
        layer: int(&d["layer"]).filter(|l| *l >= 1),
        confidence: text(&d["confidence"])?,
        classification,
        attackers,
        shot: text(&d["shot"]).filter(|s| SHOT_STATUSES.contains(&s.as_str())),
    })
}

fn attacker(enemy: &Value, role: &str, client: Client) -> Option<Attacker> {
    if !enemy.is_object() {
        return None;
    }
    let level = int(&enemy["level"]);
    let race = text(&enemy["race"]);
    let mut out = Attacker {
        role: role.into(),
        skull: level == Some(-1),
        level: level.filter(|l| (1..=100).contains(l)),
        class: text(&enemy["class"]).map(|c| c.to_lowercase()),
        race: race.as_deref().and_then(map_race),
        sex: sex(&enemy["sex"]),
        faction: player_faction(&enemy[FACTION_FIELD], race.as_deref()),
        ..Attacker::default()
    };
    match text(&enemy["key"]) {
        Some(key) => {
            let (name, realm) = split_key(&key, client);
            out.name = Some(name);
            out.realm = realm;
        }
        None => {
            out.given_name = text(&enemy["name"]).map(|n| n.chars().take(40).collect());
            out.guid = text(&enemy["guid"]);
            out.guid.as_ref()?;
        }
    }
    Some(out)
}

fn catch(j: &Value, client: Client) -> Option<Catch> {
    let outlaw = text(&j["outlaw"]).filter(|o| !o.starts_with("guid:"))?;
    let (name, realm) = split_key(&outlaw, client);
    Some(Catch {
        t: int(&j["t"])?,
        outlaw: PlayerRef { name, realm, ..PlayerRef::default() },
        map_id: int(&j["mapID"]).filter(|m| *m >= 1),
        killer_name: text(&j["killer"]).map(|k| split_key(&k, client).0),
    })
}

fn duel(d: &Value, client: Client) -> Option<Duel> {
    let player = |side: &str| -> Option<PlayerRef> {
        let (name, realm) = split_key(&text(&d[side])?, client);
        let race = text(&d[format!("{side}Race")]);
        Some(PlayerRef {
            name,
            realm,
            class: text(&d[format!("{side}Class")]).map(|c| c.to_lowercase()),
            faction: duelist_faction(&d[FACTION_FIELD], race.as_deref()),
            race: race.as_deref().and_then(map_race),
            sex: sex(&d[format!("{side}Sex")]),
        })
    };
    Some(Duel {
        t: int(&d["t"])?,
        winner: player("winner")?,
        loser: player("loser")?,
        winner_level: int(&d["winnerLevel"]).filter(|l| (1..=100).contains(l))?,
        loser_level: int(&d["loserLevel"]).filter(|l| (1..=100).contains(l))?,
        map_id: int(&d["mapID"]).filter(|m| *m >= 1),
        retreat: flag(&d["retreat"]),
        faction: faction(&d[FACTION_FIELD]),
    })
}

fn bounty_event(e: &Value, client: Client) -> Option<BountyEvent> {
    let kind = text(&e["reason"]).filter(|r| matches!(r.as_str(), "join" | "catch" | "decline"))?;
    let outlaw_name = text(&e["outlaw"]).filter(|o| o != "?");
    Some(BountyEvent {
        t: int(&e["t"])?,
        bounty: int(&e["delta"])?.clamp(-1, 20),
        total_after: int(&e["total"]).filter(|t| *t >= 0),
        outlaw_rank: text(&e["rank"]).as_deref().and_then(map_outlaw_rank),
        outlaw: outlaw_name.as_ref().map(|o| {
            let (name, realm) = split_key(o, client);
            PlayerRef { name, realm, ..PlayerRef::default() }
        }),
        outlaw_name: outlaw_name.map(|o| o.chars().take(80).collect()),
        kind,
    })
}

/// A plain player key; a Forever killer known only by the given name ("guid:...") is left out.
pub(crate) fn player_ref(key: &str, client: Client) -> Option<PlayerRef> {
    if key.starts_with("guid:") {
        return None;
    }
    let (name, realm) = split_key(key, client);
    Some(PlayerRef { name, realm, ..PlayerRef::default() })
}

fn bounty_poster(p: &Value, client: Client) -> Option<BountyPoster> {
    Some(BountyPoster {
        t: int(&p["t"])?,
        owner: player_ref(&text(&p["owner"])?, client)?,
        target: player_ref(&text(&p["target"])?, client)?,
        reason: int(&p["reason"]).filter(|r| (1..=4).contains(r))?,
        gold: int(&p["gold"]).filter(|g| *g >= 1)?,
        until: int(&p["until"])?,
    })
}

fn bounty_payment(p: &Value, client: Client) -> Option<BountyPayment> {
    // The poster id is "<owner>:<time>"; player keys never hold a colon
    let (owner, poster_t) = text(&p["posterId"])?.rsplit_once(':').map(|(o, t)| (o.to_string(), t.to_string()))?;
    Some(BountyPayment {
        t: int(&p["t"])?,
        poster_owner: player_ref(&owner, client)?,
        poster_t: poster_t.parse().ok()?,
        hunter: player_ref(&text(&p["hunter"])?, client)?,
        status: text(&p["status"]).filter(|s| matches!(s.as_str(), "claimed" | "paid" | "unpaid"))?,
        claimed_at: int(&p["claimedAt"])?,
    })
}

fn event_result(r: &Value) -> Option<EventResult> {
    Some(EventResult {
        tournament: text(&r["tournament"])?,
        round: int(&r["round"]).filter(|n| *n >= 1)?,
        r#match: int(&r["match"]).filter(|n| *n >= 1)?,
        entrant_a: text(&r["a"])?,
        entrant_b: text(&r["b"])?,
        wins_a: int(&r["winsA"]).unwrap_or(0),
        wins_b: int(&r["winsB"]).unwrap_or(0),
        forfeit: text(&r["forfeit"]).filter(|f| f == "a" || f == "b"),
        t: int(&r["t"])?,
    })
}

fn zone_list(db: &Value) -> Vec<Zone> {
    let Some(map) = db[ZONES_FIELD].as_object() else { return Vec::new() };
    let mut zones: Vec<Zone> = map
        .iter()
        .filter_map(|(id, z)| {
            Some(Zone {
                map_id: id.parse().ok().filter(|m: &i64| *m >= 1)?,
                name: text(&z["name"])?.chars().take(80).collect(),
                continent_map_id: int(&z["continent"]).filter(|c| *c >= 1),
                locale: text(&z["locale"]).map(|l| l.chars().take(8).collect()),
            })
        })
        .collect();
    zones.sort_by_key(|z| z.map_id);
    zones
}

/// Records newer than `after`, oldest first, at most `limit` (the rest go next time).
fn newest_first_limited<'a>(records: impl Iterator<Item = &'a Value>, after: i64, limit: usize) -> Vec<&'a Value> {
    let mut list: Vec<&Value> = records.filter(|r| int(&r["t"]).is_some_and(|t| t > after)).collect();
    list.sort_by_key(|r| int(&r["t"]));
    list.truncate(limit);
    list
}

fn glass(g: &Value, client: Client) -> Option<Glass> {
    Some(Glass {
        outlaw: player_ref(&text(&g["outlaw"])?, client)?,
        caught_at: int(&g["caughtAt"])?,
        by: player_ref(&text(&g["by"])?, client)?,
        t: int(&g["t"])?,
        popup: g["popup"].as_bool().unwrap_or(false),
    })
}

/// Like `newest_first_limited`, but a cut never splits one second: the kills after the
/// cut that share its last second would never be sent (`sent` keeps only the time), so
/// that second waits for the next upload, unless it alone is over the limit.
fn whole_seconds_limited<'a>(records: impl Iterator<Item = &'a Value>, after: i64, limit: usize) -> Vec<&'a Value> {
    let mut list: Vec<&Value> = records.filter(|r| int(&r["t"]).is_some_and(|t| t > after)).collect();
    list.sort_by_key(|r| (int(&r["t"]), int(&r["seq"])));
    if list.len() > limit {
        let cut = int(&list[limit]["t"]);
        let whole = list[..limit].iter().filter(|r| int(&r["t"]) != cut).count();
        list.truncate(if whole > 0 { whole } else { limit });
    }
    list
}

fn honor_kill(k: &Value, client: Client) -> Option<HonorKill> {
    Some(HonorKill {
        t: int(&k["t"])?,
        seq: int(&k["seq"]).filter(|s| (1..=1000).contains(s)).unwrap_or(1),
        map_id: int(&k["mapID"]).filter(|m| *m >= 1),
        x: float(&k["x"]).filter(|v| (0.0..=1.0).contains(v)),
        y: float(&k["y"]).filter(|v| (0.0..=1.0).contains(v)),
        layer: int(&k["layer"]).filter(|l| *l >= 1),
        victim: text(&k["victim"]).and_then(|v| player_ref(&v, client)),
        honor: int(&k["honor"]).filter(|h| (0..=10000).contains(h)),
    })
}

fn witness_record(w: &Value, client: Client) -> Option<WitnessRecord> {
    Some(WitnessRecord {
        t: int(&w["t"])?,
        outlaw: player_ref(&text(&w["outlaw"])?, client)?,
        map_id: int(&w["mapID"]).filter(|m| *m >= 1),
        x: float(&w["x"]).filter(|v| (0.0..=1.0).contains(v)),
        y: float(&w["y"]).filter(|v| (0.0..=1.0).contains(v)),
        killer: text(&w["killer"]).and_then(|k| player_ref(&k, client)),
    })
}

/// The character's finished wars ended after `after`, the oldest first, at most `limit`.
/// A war still going has no end and waits for it.
fn ended_wars<'a>(wars: &'a Value, key: &str, after: i64, limit: usize) -> Vec<&'a Value> {
    let mut list: Vec<&Value> = values(wars)
        .filter(|w| text(&w["by"]).as_deref() == Some(key) && int(&w["ended"]).is_some_and(|e| e > after))
        .collect();
    list.sort_by_key(|w| int(&w["ended"]));
    list.truncate(limit);
    list
}

fn war(w: &Value, client: Client) -> Option<War> {
    let mut layers: Vec<i64> = values(&w["layers"]).filter_map(int).filter(|l| *l >= 1).collect();
    layers.sort_unstable();
    layers.dedup();
    layers.truncate(20);
    let mut fighters: Vec<WarFighter> = w["fighters"]
        .as_object()
        .map(|fighters| fighters.iter().filter_map(|(key, f)| war_fighter(key, f, client)).collect())
        .unwrap_or_default();
    fighters.sort_by(|a, b| (a.first, &a.name).cmp(&(b.first, &b.name)));
    fighters.truncate(MAX_WAR_FIGHTERS);
    Some(War {
        uid: text(&w["id"])?.chars().take(160).collect(),
        map_id: int(&w["zone"]).filter(|m| *m >= 1)?,
        started: int(&w["started"])?,
        ended: int(&w["ended"])?,
        peak_fire: int(&w["peak"]).map(|p| p.clamp(1, 3)).unwrap_or(1),
        layers,
        kills: int(&w["kills"]).filter(|k| *k >= 0).unwrap_or(0),
        honor: int(&w["honor"]).filter(|h| *h >= 0).unwrap_or(0),
        deaths: int(&w["deaths"]).filter(|d| *d >= 0).unwrap_or(0),
        present: w["present"].as_bool(),
        fighters,
    })
}

fn war_fighter(key: &str, f: &Value, client: Client) -> Option<WarFighter> {
    let player = player_ref(key, client)?;
    let side = text(&f["side"]).filter(|s| s == "ally" || s == "enemy")?;
    let race = text(&f["race"]);
    Some(WarFighter {
        name: player.name,
        realm: player.realm,
        side,
        faction: faction(&f[FACTION_FIELD]),
        class: text(&f["class"]).map(|c| c.to_lowercase()),
        race: race.as_deref().and_then(map_race),
        level: int(&f["level"]).filter(|l| (1..=100).contains(l)),
        guild: text(&f["guild"]).map(|g| g.chars().take(64).collect()),
        first: int(&f["first"])?,
        last: int(&f["last"])?,
    })
}

fn max_t(records: &[&Value], already: i64) -> i64 {
    records.iter().filter_map(|r| int(&r["t"])).max().unwrap_or(already).max(already)
}

/// "Name-Realm" on Era; Forever keys ("Given Family") have no realm.
pub fn split_key(key: &str, client: Client) -> (String, Option<String>) {
    match (client, key.split_once('-')) {
        (Client::Era, Some((name, realm))) if !realm.is_empty() => (name.to_string(), Some(realm.to_string())),
        _ => (key.to_string(), None),
    }
}

fn race_entry(race: &str) -> Option<&'static (&'static str, &'static str, Option<&'static str>)> {
    RACES.iter().find(|(token, _, _)| *token == race)
}

/// The addon's enemies (key -> record) seen after `after`, newest first, at most `limit`.
fn newest_seen_limited(enemies: &Value, after: i64, limit: usize) -> Vec<(String, Value)> {
    let Some(map) = enemies.as_object() else { return Vec::new() };
    let mut seen: Vec<(String, Value)> = map
        .iter()
        .filter(|(_, e)| int(&e["lastSeen"]).unwrap_or(0) > after)
        .map(|(key, e)| (key.clone(), e.clone()))
        .collect();
    seen.sort_by_key(|(_, e)| std::cmp::Reverse(int(&e["lastSeen"]).unwrap_or(0)));
    seen.truncate(limit);
    seen
}

/// An enemy the addon saw, for the website to fill in what it lacks; only one with
/// something to tell (sex, race or class).
fn known_player(key: &str, enemy: &Value, client: Client) -> Option<PlayerRef> {
    let (name, realm) = split_key(key, client);
    let race = text(&enemy["race"]);
    let player = PlayerRef {
        name,
        realm,
        class: text(&enemy["class"]).map(|c| c.to_lowercase()),
        faction: player_faction(&enemy[FACTION_FIELD], race.as_deref()),
        race: race.as_deref().and_then(map_race),
        sex: sex(&enemy["sex"]),
    };
    (player.sex.is_some() || player.race.is_some() || player.class.is_some()).then_some(player)
}

/// What a killer's or assist's record lacks (sex, race, class), from the addon's enemy
/// record of that player: it may have seen them up close after the death.
fn fill_from_enemy(attacker: &mut Attacker, enemies: &Value, client: Client) {
    let Some(name) = attacker.name.as_ref() else { return };
    let key = match (client, attacker.realm.as_ref()) {
        (Client::Era, Some(realm)) => format!("{name}-{realm}"),
        _ => name.clone(),
    };
    let enemy = &enemies[key.as_str()];
    if !enemy.is_object() {
        return;
    }
    attacker.sex = attacker.sex.or_else(|| sex(&enemy["sex"]));
    if attacker.race.is_none() {
        attacker.race = text(&enemy["race"]).as_deref().and_then(map_race);
    }
    if attacker.class.is_none() {
        attacker.class = text(&enemy["class"]).map(|c| c.to_lowercase());
    }
}

/// The game's sex as the website takes it: 2 male, 3 female, else nothing.
fn sex(value: &Value) -> Option<i64> {
    int(value).filter(|s| *s == 2 || *s == 3)
}

/// The addon's race tokens to the website's names.
pub fn map_race(race: &str) -> Option<String> {
    race_entry(race).map(|(_, name, _)| name.to_string())
}

pub fn faction_of_race(race: &str) -> Option<String> {
    race_entry(race).and_then(|(_, _, faction)| faction.map(String::from))
}

/// The game's faction as the website names it; anything else is dropped.
fn faction(v: &Value) -> Option<String> {
    text(v).map(|f| f.to_lowercase()).filter(|f| FACTIONS.contains(&f.as_str()))
}

/// The faction the game gave, else the one the race tells.
fn player_faction(given: &Value, race: Option<&str>) -> Option<String> {
    faction(given).or_else(|| race.and_then(faction_of_race))
}

/// A duel names one faction for both sides and duels can cross factions, so the race
/// wins; the duel's faction only when the race does not tell (Skyborne).
fn duelist_faction(duel_faction: &Value, race: Option<&str>) -> Option<String> {
    race.and_then(faction_of_race).or_else(|| faction(duel_faction))
}

fn map_outlaw_rank(rank: &str) -> Option<String> {
    let mapped = match rank {
        "ganker" => "ganker",
        "outlaw" => "outlaw",
        "desperado" => "desperado",
        "mostwanted" => "most_wanted",
        "deadoralive" => "dead_or_alive",
        _ => return None,
    };
    Some(mapped.into())
}

pub(crate) fn values(v: &Value) -> Box<dyn Iterator<Item = &Value> + '_> {
    match v {
        Value::Array(list) => Box::new(list.iter()),
        Value::Object(map) => Box::new(map.values()),
        _ => Box::new(std::iter::empty()),
    }
}

pub(crate) fn text(v: &Value) -> Option<String> {
    v.as_str().map(str::trim).filter(|s| !s.is_empty()).map(String::from)
}

pub(crate) fn int(v: &Value) -> Option<i64> {
    v.as_i64().or_else(|| v.as_f64().map(|f| f as i64))
}

fn float(v: &Value) -> Option<f64> {
    v.as_f64()
}

fn flag(v: &Value) -> bool {
    v.as_bool().unwrap_or(false)
}

/// Grouping helper for the status screen: characters found in a file.
pub fn characters_in(db: &Value) -> BTreeMap<String, bool> {
    let mut found: BTreeMap<String, bool> = own_deaths(db).iter().filter_map(|d| text(&d["victim"]["key"])).map(|k| (k, false)).collect();
    if let Some(last) = text(&db[META_FIELD][PLAYER_FIELD]["key"]) {
        found.insert(last, true);
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn ctx(client: Client) -> Context {
        Context { client: Some(client), region: Some("us".into()), realm_type: "pvp".into(), known_servers: BTreeSet::new(), addon_version: "0.1.4".into() }
    }

    /// Made-up players only (never the author's characters).
    fn era_db() -> Value {
        json!({
            "meta": {
                "region": "eu", "client": "era",
                "player": { "key": "Tessa-Firemaw", "realm": "Firemaw", "class": "PRIEST", "race": "Scourge", "sex": 3, "level": 42, "faction": "Horde", "guild": "Night Watch" }
            },
            "deaths": [
                { "id": "Tessa-Firemaw:100", "t": 100, "victim": { "key": "Tessa-Firemaw", "level": 41, "class": "PRIEST", "race": "Scourge" },
                  "killer": { "key": "Brute-Firemaw", "level": -1, "class": "WARRIOR", "race": "Human", "sex": 3 },
                  "assists": [ { "name": "Sly", "guid": "Player-1-ABC", "level": 58, "class": "ROGUE", "race": "NightElf" } ],
                  "mapID": 1434, "x": 0.5, "y": 0.25, "confidence": "exact", "classification": "coward", "shot": "taken" },
                { "id": "Alt-Firemaw:200", "t": 200, "victim": { "key": "Alt-Firemaw", "level": 20, "class": "MAGE", "race": "Troll" },
                  "killer": { "key": "Brute-Firemaw", "level": 30, "race": "Human" },
                  "mapID": 1413, "confidence": "inferred", "classification": "normal", "shot": "bogus" },
                { "id": "Tessa-Firemaw:300", "t": 300, "victim": { "key": "Tessa-Firemaw", "level": 42 },
                  "killer": { "key": "Fake-Firemaw", "level": 60 }, "confidence": "sim", "classification": "fair" },
                { "id": "Tessa-Firemaw:400:demo", "t": 400, "demo": true, "victim": { "key": "Tessa-Firemaw", "level": 42 },
                  "killer": { "key": "Demo-Firemaw", "level": 60 }, "confidence": "exact", "classification": "fair" }
            ],
            "justice": {
                "Brute-Firemaw:150": { "outlaw": "Brute-Firemaw", "t": 150, "mapID": 1434, "killer": "Tessa-Firemaw", "hunter": "Tessa-Firemaw", "origin": "local" },
                "guid:X:160": { "outlaw": "guid:Player-1-X", "t": 160, "hunter": "Tessa-Firemaw", "origin": "local" },
                "Other-Firemaw:170": { "outlaw": "Other-Firemaw", "t": 170, "hunter": "Someone-Firemaw", "origin": "peer" }
            },
            "duels": {
                "A-Firemaw>B-Firemaw:120": { "winner": "A-Firemaw", "loser": "B-Firemaw", "t": 120, "faction": "Horde",
                  "winnerClass": "SHAMAN", "winnerRace": "Orc", "winnerSex": 2, "winnerLevel": 40, "loserClass": "MAGE", "loserRace": "Troll", "loserLevel": 38, "mapID": 1413, "origin": "local", "retreat": true },
                "C-Firemaw>D-Firemaw:130": { "winner": "C-Firemaw", "loser": "D-Firemaw", "t": 130, "winnerLevel": 40, "loserLevel": 40, "origin": "peer" },
                "E-Firemaw>F-Firemaw:140": { "winner": "E-Firemaw", "loser": "F-Firemaw", "t": 140, "winnerLevel": 40, "loserLevel": 40, "origin": "local", "demo": true }
            },
            "marks": { "total": 6, "events": [
                { "t": 150, "delta": 5, "reason": "catch", "outlaw": "Brute", "rank": "outlaw", "total": 5, "hunter": "Tessa-Firemaw" },
                { "t": 155, "delta": 1, "reason": "join", "outlaw": "Brute", "rank": "mostwanted", "total": 6 },
                { "t": 156, "delta": 0, "reason": "skip", "outlaw": "Low", "total": 6 },
                { "t": 157, "delta": 1, "reason": "join", "outlaw": "X", "total": 7, "hunter": "Alt-Firemaw" }
            ] },
            "demoMarks": { "total": 99, "events": [] },
            "zones": { "1434": { "name": "Stranglethorn Vale", "continent": 1415, "locale": "enUS" }, "1413": { "name": "The Barrens", "continent": 1414 } }
        })
    }

    fn by_key<'a>(uploads: &'a [CharacterUpload], key: &str) -> &'a CharacterUpload {
        uploads.iter().find(|u| u.key == key).expect(key)
    }

    #[test]
    fn one_upload_per_character_with_the_right_records() {
        let uploads = build(&era_db(), &ctx(Client::Era), |_| Sent::default()).unwrap();
        assert_eq!(uploads.len(), 2);

        let main = &by_key(&uploads, "Tessa-Firemaw").payload;
        assert_eq!(main.character.name, "Tessa");
        assert_eq!(main.character.realm.as_deref(), Some("Firemaw"));
        assert_eq!(main.character.region, "eu");
        assert_eq!(main.character.faction.as_deref(), Some("horde"));
        assert_eq!(main.character.race.as_deref(), Some("undead"));
        assert_eq!(main.character.class.as_deref(), Some("priest"));
        assert_eq!(main.character.realm_type, None);
        assert_eq!(main.deaths.len(), 1, "sim and demo deaths stay home");
        assert_eq!(main.catches.len(), 1, "guid outlaws and other hunters' catches stay home");
        assert_eq!(main.duels.len(), 2, "our own and shared duels, no demo");
        assert_eq!(main.bounty_events.len(), 2, "own and hunter-less events, no skip");
        assert_eq!(main.zones.len(), 2);

        let alt = &by_key(&uploads, "Alt-Firemaw").payload;
        assert_eq!(alt.character.name, "Alt");
        assert_eq!(alt.character.faction.as_deref(), Some("horde"));
        assert_eq!(alt.deaths.len(), 1);
        assert_eq!(alt.bounty_events.len(), 1);
        assert!(alt.duels.is_empty() && alt.zones.is_empty(), "duels and zones go with the last played character");
    }

    #[test]
    fn maps_deaths_attackers_and_events_to_the_website_names() {
        let uploads = build(&era_db(), &ctx(Client::Era), |_| Sent::default()).unwrap();
        let main = &by_key(&uploads, "Tessa-Firemaw").payload;

        let death = &main.deaths[0];
        assert_eq!(death.uid, "Tessa-Firemaw:100");
        assert_eq!(death.victim_race.as_deref(), Some("undead"));
        assert_eq!(death.map_id, Some(1434));
        let killer = &death.attackers[0];
        assert_eq!((killer.role.as_str(), killer.name.as_deref(), killer.realm.as_deref()), ("killer", Some("Brute"), Some("Firemaw")));
        assert!(killer.skull && killer.level.is_none());
        assert_eq!(killer.faction.as_deref(), Some("alliance"));
        assert_eq!(killer.sex, Some(3), "the killer's sex, for the race icon by gender");
        let assist = &death.attackers[1];
        assert_eq!((assist.name.as_deref(), assist.given_name.as_deref(), assist.guid.as_deref()), (None, Some("Sly"), Some("Player-1-ABC")));
        assert_eq!(assist.race.as_deref(), Some("night_elf"));

        let duel = &main.duels[0];
        assert_eq!((duel.winner.name.as_str(), duel.winner.race.as_deref(), duel.faction.as_deref()), ("A", Some("orc"), Some("horde")));
        assert!(duel.retreat);
        assert_eq!((duel.winner.sex, duel.loser.sex), (Some(2), None), "a sex the addon did not see stays out");

        let join = main.bounty_events.iter().find(|e| e.kind == "join").unwrap();
        assert_eq!(join.outlaw_rank.as_deref(), Some("most_wanted"));
        assert_eq!(join.outlaw.as_ref().unwrap().name, "Brute");

        let json = serde_json::to_value(main).unwrap();
        assert_eq!(json["bounty_events"][0]["type"], "catch");
        assert_eq!(json["character"]["realm_type"], serde_json::Value::Null);
        assert!(json["deaths"][0]["attackers"][1].get("skull").is_none(), "skull only when true");
        assert_eq!(death.shot.as_deref(), Some("taken"), "the screenshot status of the death");
        let alt = &by_key(&uploads, "Alt-Firemaw").payload;
        assert_eq!(alt.deaths[0].shot, None, "an unknown status stays out");
        assert!(serde_json::to_value(alt).unwrap()["deaths"][0].get("shot").is_none(), "left out, not null");
    }

    #[test]
    fn sends_only_what_is_newer_than_last_time() {
        let first = build(&era_db(), &ctx(Client::Era), |_| Sent::default()).unwrap();
        let sent: BTreeMap<String, Sent> = first.iter().map(|u| (u.key.clone(), u.sent_after.clone())).collect();

        let again = build(&era_db(), &ctx(Client::Era), |key| sent.get(key).cloned().unwrap_or_default()).unwrap();
        assert!(again.is_empty(), "nothing new, nothing sent");

        let mut db = era_db();
        db["deaths"].as_array_mut().unwrap().push(json!({ "id": "Tessa-Firemaw:900", "t": 900, "victim": { "key": "Tessa-Firemaw", "level": 42 },
            "killer": { "key": "New-Firemaw", "level": 44 }, "confidence": "exact", "classification": "fair" }));
        let later = build(&db, &ctx(Client::Era), |key| sent.get(key).cloned().unwrap_or_default()).unwrap();
        assert_eq!(later.len(), 1);
        assert_eq!(later[0].payload.deaths.len(), 1);
        assert_eq!(later[0].payload.deaths[0].t, 900);
        assert!(later[0].payload.zones.is_empty(), "zones already sent");
    }

    #[test]
    fn sends_the_players_bounties_with_the_last_character() {
        let mut db = era_db();
        db["posters"] = json!({
            "Tessa-Firemaw:500": { "id": "Tessa-Firemaw:500", "owner": "Tessa-Firemaw", "target": "Brute-Firemaw", "reason": 1,
              "gold": 20000, "until": 259700, "t": 500, "origin": "local" },
            "Tessa-Firemaw:600": { "owner": "Tessa-Firemaw", "target": "guid:Player-1-X", "reason": 2, "gold": 20000, "until": 9000, "t": 600 },
            "Mira-Firemaw:550": { "id": "Mira-Firemaw:550", "owner": "Mira-Firemaw", "target": "Brute-Firemaw", "reason": 3,
              "gold": 30000, "until": 90000, "t": 550, "origin": "website" }
        });
        db["bountyPay"] = json!({
            "Tessa-Firemaw:500": { "posterId": "Tessa-Firemaw:500", "hunter": "Kestrel-Firemaw", "status": "unpaid",
              "claimedAt": 700, "t": 800, "origin": "peer" }
        });
        let uploads = build(&db, &ctx(Client::Era), |_| Sent::default()).unwrap();
        let main = by_key(&uploads, "Tessa-Firemaw");
        assert_eq!(main.payload.bounty_posters.len(), 1, "a given-name-only target and a website poster are left out");
        let poster = &main.payload.bounty_posters[0];
        assert_eq!((poster.owner.name.as_str(), poster.target.name.as_str()), ("Tessa", "Brute"));
        assert_eq!((poster.gold, poster.until, poster.reason), (20000, 259700, 1));
        let pay = &main.payload.bounty_payments[0];
        assert_eq!((pay.poster_owner.name.as_str(), pay.poster_t), ("Tessa", 500));
        assert_eq!((pay.hunter.name.as_str(), pay.status.as_str(), pay.claimed_at, pay.t), ("Kestrel", "unpaid", 700, 800));
        assert_eq!((main.sent_after.posters, main.sent_after.payments), (600, 800));
        assert!(by_key(&uploads, "Alt-Firemaw").payload.bounty_posters.is_empty(), "only with the last character");
    }

    #[test]
    fn sends_the_tournament_results_the_character_confirmed() {
        let mut db = era_db();
        db["eventResults"] = json!({
            "gatebrawl:1:2": { "tournament": "gatebrawl", "round": 1, "match": 2, "a": "p1", "b": "p2", "winsA": 2, "winsB": 1,
              "t": 900, "organizer": "Tessa-Firemaw", "origin": "local" },
            "gatebrawl:1:3": { "tournament": "gatebrawl", "round": 1, "match": 3, "a": "p5", "b": "p6", "winsA": 0, "winsB": 0,
              "forfeit": "b", "t": 950, "organizer": "Tessa-Firemaw", "origin": "local" },
            "gatebrawl:1:4": { "tournament": "gatebrawl", "round": 1, "match": 4, "a": "p7", "b": "p8", "winsA": 2, "winsB": 0,
              "t": 960, "organizer": "Other-Firemaw", "origin": "peer" }
        });
        let uploads = build(&db, &ctx(Client::Era), |_| Sent::default()).unwrap();
        let main = by_key(&uploads, "Tessa-Firemaw");
        assert_eq!(main.payload.event_results.len(), 2, "only our own, confirmed here");
        let no_show = main.payload.event_results.iter().find(|r| r.r#match == 3).unwrap();
        assert_eq!((no_show.forfeit.as_deref(), no_show.entrant_a.as_str()), (Some("b"), "p5"));
        assert_eq!(main.sent_after.event_results, 950);
        let json = serde_json::to_value(&main.payload).unwrap();
        assert!(json["event_results"][0]["match"].is_number(), "sent as \"match\"");
        let again = build(&db, &ctx(Client::Era), |_| main.sent_after.clone()).unwrap();
        assert!(again.iter().find(|u| u.key == "Tessa-Firemaw").is_none_or(|u| u.payload.event_results.is_empty()), "not twice");
    }

    #[test]
    fn sends_the_deaths_other_headhunters_shared_with_the_last_character() {
        let mut db = era_db();
        db["reports"] = json!({
            "Rowena-Firemaw:500": { "id": "Rowena-Firemaw:500", "t": 500, "origin": "peer",
              "receivedBy": "Tessa-Firemaw", "receivedAt": 502, "receivedMap": 1413,
              "victim": { "key": "Rowena-Firemaw", "level": 30, "class": "HUNTER", "race": "Troll" },
              "killer": { "key": "Brute-Firemaw", "level": 45, "class": "WARRIOR", "race": "Human" },
              "mapID": 1413, "confidence": "exact", "classification": "coward" },
            "Tessa-Firemaw:100": { "id": "Tessa-Firemaw:100", "t": 100, "origin": "local",
              "victim": { "key": "Tessa-Firemaw", "level": 41 }, "killer": { "key": "Brute-Firemaw", "level": 60 },
              "confidence": "exact", "classification": "coward" },
            "Alt-Firemaw:600": { "id": "Alt-Firemaw:600", "t": 600, "origin": "relay",
              "victim": { "key": "Alt-Firemaw", "level": 20 }, "killer": { "key": "Brute-Firemaw", "level": 30 },
              "confidence": "exact", "classification": "normal" },
            "Sim-Firemaw:700": { "id": "Sim-Firemaw:700", "t": 700, "origin": "peer",
              "victim": { "key": "Sim-Firemaw", "level": 20 }, "killer": { "key": "Brute-Firemaw", "level": 30 },
              "confidence": "sim", "classification": "normal" }
        });
        let uploads = build(&db, &ctx(Client::Era), |_| Sent::default()).unwrap();
        let main = by_key(&uploads, "Tessa-Firemaw");
        assert_eq!(main.payload.shared_deaths.len(), 1, "not our own, not a simulation");
        let shared = &main.payload.shared_deaths[0];
        assert_eq!(shared.victim.name, "Rowena");
        assert_eq!(shared.victim.faction.as_deref(), Some("horde"));
        assert_eq!((shared.death.t, shared.death.victim_level), (500, 30));
        assert_eq!(shared.death.attackers[0].name.as_deref(), Some("Brute"));
        assert_eq!(shared.via.as_deref(), Some("peer"));
        assert_eq!(shared.received_by.as_deref(), Some("Tessa"));
        assert_eq!((shared.received_at, shared.received_map_id), (Some(502), Some(1413)));
        assert_eq!(shared.relayed_by, None);
        assert_eq!(main.sent_after.shared_deaths, 500);
        assert!(by_key(&uploads, "Alt-Firemaw").payload.shared_deaths.is_empty(), "only with the last character");
    }

    #[test]
    fn sends_who_passed_on_a_relayed_death() {
        let mut db = era_db();
        db["reports"] = json!({
            "Rowena-Firemaw:500": { "id": "Rowena-Firemaw:500", "t": 500, "origin": "relay", "relayedBy": "Corin-Firemaw",
              "victim": { "key": "Rowena-Firemaw", "level": 30 }, "killer": { "key": "Brute-Firemaw", "level": 45 },
              "confidence": "exact", "classification": "coward" }
        });
        let uploads = build(&db, &ctx(Client::Era), |_| Sent::default()).unwrap();
        let shared = &by_key(&uploads, "Tessa-Firemaw").payload.shared_deaths[0];
        assert_eq!(shared.via.as_deref(), Some("relay"));
        assert_eq!(shared.relayed_by.as_deref(), Some("Corin"));
        assert_eq!(shared.received_by, None, "older addon data has no receiver");
        let json = serde_json::to_value(shared).unwrap();
        assert!(json.get("received_by").is_none(), "left out, not null");
    }

    #[test]
    fn world_keys_name_every_character_world() {
        let era = world_keys(&era_db(), &ctx(Client::Era)).unwrap();
        assert_eq!(era.into_iter().collect::<Vec<_>>(), vec!["era|eu|Firemaw".to_string()], "both characters live on Firemaw");

        let forever = json!({ "meta": { "client": "forever", "player": { "key": "Tess Rider" } } });
        let context = Context { realm_type: "hardcore".into(), ..ctx(Client::Forever) };
        let keys = world_keys(&forever, &context).unwrap();
        assert_eq!(keys.into_iter().collect::<Vec<_>>(), vec!["forever|us|hardcore".to_string()], "Forever takes the install's realm type");
    }

    fn forever_db() -> Value {
        json!({
            "meta": { "client": "forever", "player": { "key": "Tess Rider", "level": 14, "server": 4620 } },
            "deaths": [
                { "id": "Mira Vale:100", "t": 100, "victim": { "key": "Mira Vale", "level": 10, "server": 4619 },
                  "killer": { "key": "Grim Tusk", "level": 12 }, "confidence": "exact", "classification": "normal" },
                { "id": "Mira Vale:200", "t": 200, "victim": { "key": "Mira Vale", "level": 11, "server": 4621 },
                  "killer": { "key": "Grim Tusk", "level": 12 }, "confidence": "exact", "classification": "normal" },
                { "id": "Tess Rider:150", "t": 150, "victim": { "key": "Tess Rider", "level": 13, "server": 4620 },
                  "killer": { "key": "Grim Tusk", "level": 12 }, "confidence": "exact", "classification": "normal" }
            ]
        })
    }

    #[test]
    fn a_duelist_takes_the_faction_of_the_race_first() {
        let d = json!({ "winner": "Troll Axe", "loser": "Wing Talon", "t": 100, "faction": "Alliance",
            "winnerRace": "Troll", "loserRace": "Skyborne", "winnerLevel": 14, "loserLevel": 14 });
        let duel = duel(&d, Client::Forever).unwrap();
        assert_eq!(duel.winner.faction.as_deref(), Some("horde"), "a Troll is Horde, whatever the duel says");
        assert_eq!(duel.loser.faction.as_deref(), Some("alliance"), "Skyborne: the duel's faction");
    }

    #[test]
    fn sends_the_enemies_seen_up_close_and_fills_a_killer_from_them() {
        let db = json!({
            "meta": { "client": "forever", "player": { "key": "Wren Gale", "level": 14, "server": 4620, "race": "Human", "faction": "Alliance" } },
            "deaths": [
                { "id": "Wren Gale:100", "t": 100, "victim": { "key": "Wren Gale", "level": 13, "race": "Human" },
                  "killer": { "key": "Sickgirl Fs", "level": 15 },
                  "confidence": "exact", "classification": "normal" }
            ],
            "enemies": {
                "Sickgirl Fs": { "key": "Sickgirl Fs", "class": "PRIEST", "race": "NightElf", "sex": 3, "faction": "Alliance", "lastSeen": 500 },
                "Grim Reaver": { "key": "Grim Reaver", "class": "WARRIOR", "race": "Orc", "sex": 2, "lastSeen": 400 },
                "Nobody Known": { "key": "Nobody Known", "lastSeen": 300 }
            }
        });
        let uploads = build(&db, &ctx(Client::Forever), |_| Sent::default()).unwrap();
        let wren = &by_key(&uploads, "Wren Gale");
        let killer = &wren.payload.deaths[0].attackers[0];
        assert_eq!((killer.sex, killer.race.as_deref(), killer.class.as_deref()), (Some(3), Some("night_elf"), Some("priest")), "filled from the enemy record");

        let known: Vec<(&str, Option<i64>)> = wren.payload.known_players.iter().map(|p| (p.name.as_str(), p.sex)).collect();
        assert_eq!(known, vec![("Sickgirl Fs", Some(3)), ("Grim Reaver", Some(2))], "newest first, only enemies with something to tell");
        assert_eq!(wren.sent_after.known_players, 500);

        let later = build(&db, &ctx(Client::Forever), |_| wren.sent_after.clone()).unwrap();
        assert!(later.is_empty(), "nothing new: no upload");
    }

    #[test]
    fn skyborne_takes_the_faction_the_game_gave() {
        let db = json!({
            "meta": { "client": "forever", "player": { "key": "Wren Gale", "level": 14, "server": 4620, "race": "Skyborne", "faction": "Alliance" } },
            "deaths": [
                { "id": "Wren Gale:100", "t": 100, "victim": { "key": "Wren Gale", "level": 13, "race": "Skyborne" },
                  "killer": { "key": "Storm Crest", "level": 15, "race": "Skyborne", "faction": "Horde" },
                  "assists": [
                      { "key": "Cloud Drifter", "level": 14, "race": "Skyborne" },
                      { "key": "Bone Grinder", "level": 16, "race": "Troll" }
                  ],
                  "confidence": "exact", "classification": "normal" }
            ]
        });
        let uploads = build(&db, &ctx(Client::Forever), |_| Sent::default()).unwrap();
        let wren = &by_key(&uploads, "Wren Gale").payload;
        assert_eq!((wren.character.race.as_deref(), wren.character.faction.as_deref()), (Some("skyborne"), Some("alliance")));

        let death = &wren.deaths[0];
        assert_eq!(death.victim_race.as_deref(), Some("skyborne"));
        let [killer, drifter, grinder] = &death.attackers[..] else { panic!("three attackers") };
        assert_eq!((killer.race.as_deref(), killer.faction.as_deref()), (Some("skyborne"), Some("horde")));
        assert_eq!((drifter.race.as_deref(), drifter.faction.as_deref()), (Some("skyborne"), None), "both factions play Skyborne");
        assert_eq!((grinder.race.as_deref(), grinder.faction.as_deref()), (Some("troll"), Some("horde")), "the race still tells");
    }

    #[test]
    fn a_new_forever_server_is_sent_once_even_without_records() {
        let db = json!({ "meta": { "client": "forever", "player": { "key": "Nib Sprocket", "level": 1, "server": 4620 } } });

        let uploads = build(&db, &ctx(Client::Forever), |_| Sent::default()).unwrap();
        let nib = by_key(&uploads, "Nib Sprocket");
        assert_eq!(nib.payload.character.server, Some(4620), "a character with nothing new still goes once");
        assert_eq!(nib.sent_after.server, Some(4620), "and the server is remembered");

        let same = build(&db, &ctx(Client::Forever), |_| Sent { server: Some(4620), ..Sent::default() }).unwrap();
        assert!(same.is_empty(), "the same server again: nothing to send");

        let moved = build(&db, &ctx(Client::Forever), |_| Sent { server: Some(4619), ..Sent::default() }).unwrap();
        assert_eq!(moved.len(), 1, "another server: sent again");
    }

    fn homes_db() -> Value {
        json!({
            "schemaVersion": 2,
            "meta": { "client": "forever", "home": "forever|4620", "player": { "key": "Nib Sprocket", "level": 1, "server": 4620 } },
            "zones": { "1429": { "name": "Elwynn Forest", "continent": 1415 } },
            "homes": {
                "forever|4619": {
                    "player": { "key": "Grim Pvp", "level": 20, "server": 4619, "race": "Orc", "class": "WARRIOR" },
                    "deaths": [
                        { "id": "Grim Pvp:100", "t": 100, "victim": { "key": "Grim Pvp", "level": 19, "server": 4619 },
                          "killer": { "key": "Tall Shadow", "level": 22 }, "confidence": "exact", "classification": "normal" }
                    ]
                },
                "forever|4620": {
                    "player": { "key": "Nib Sprocket", "level": 1, "server": 4620, "race": "Gnome", "class": "MAGE" }
                }
            }
        })
    }

    fn build_homes(db: &Value, context: &Context) -> Vec<(Option<String>, CharacterUpload)> {
        homes(db)
            .into_iter()
            .flat_map(|home| build(&home.db, context, |_| Sent::default()).unwrap().into_iter().map(move |u| (home.key.clone(), u)))
            .collect()
    }

    #[test]
    fn each_home_is_a_flat_file_with_its_own_player() {
        let list = homes(&homes_db());
        let keys: Vec<Option<&str>> = list.iter().map(|h| h.key.as_deref()).collect();
        assert_eq!(keys, vec![Some("forever|4619"), Some("forever|4620")]);
        assert_eq!(list.iter().map(|h| h.last).collect::<Vec<_>>(), vec![false, true], "meta.home is the home played last");

        let grim = &list[0].db;
        assert_eq!(grim["meta"]["player"]["key"], "Grim Pvp", "the home's player, not the root one");
        assert_eq!(grim["meta"]["client"], "forever");
        assert_eq!(grim["zones"]["1429"]["name"], "Elwynn Forest", "zones come from the root");
        assert!(grim.get("player").is_none() && grim.get("homes").is_none());

        let flat = forever_db();
        assert_eq!(homes(&flat), vec![Home { key: None, last: true, db: flat.clone() }], "an old flat file is one home without a key");
    }

    #[test]
    fn a_home_without_a_player_has_meta_without_a_player() {
        let db = json!({ "meta": { "client": "forever", "player": { "key": "Nib Sprocket" } }, "homes": { "forever|4621": { "deaths": [] } } });
        let list = homes(&db);
        assert_eq!(list.len(), 1);
        assert!(list[0].db["meta"].get("player").is_none());
        assert_eq!(list[0].db["meta"]["client"], "forever");
    }

    #[test]
    fn every_home_uploads_its_own_characters() {
        let uploads = build_homes(&homes_db(), &ctx(Client::Forever));
        assert_eq!(uploads.len(), 2);

        let (home, grim) = uploads.iter().find(|(_, u)| u.key == "Grim Pvp").expect("Grim Pvp");
        assert_eq!(home.as_deref(), Some("forever|4619"));
        assert_eq!(grim.payload.character.server, Some(4619));
        assert_eq!(grim.payload.deaths.len(), 1);
        assert_eq!(grim.payload.deaths[0].uid, "Grim Pvp:100");
        assert_eq!(grim.payload.character.race.as_deref(), Some("orc"));

        let (home, nib) = uploads.iter().find(|(_, u)| u.key == "Nib Sprocket").expect("Nib Sprocket");
        assert_eq!(home.as_deref(), Some("forever|4620"));
        assert_eq!(nib.payload.character.server, Some(4620));
        assert_eq!(nib.payload.character.level, Some(1));
        assert!(nib.payload.deaths.is_empty() && nib.payload.duels.is_empty(), "only the server goes");
        assert_eq!(nib.sent_after.server, Some(4620));
    }

    #[test]
    fn world_keys_come_from_every_home() {
        let context = Context { known_servers: BTreeSet::from([4619, 4620]), ..ctx(Client::Forever) };
        let keys: BTreeSet<String> = homes(&homes_db()).iter().flat_map(|home| world_keys(&home.db, &context).unwrap()).collect();
        assert_eq!(keys.into_iter().collect::<Vec<_>>(), vec!["forever|us|4619".to_string(), "forever|us|4620".to_string()]);

        let characters: Vec<String> = homes(&homes_db()).iter().flat_map(|home| characters_in(&home.db).into_keys()).collect();
        assert_eq!(characters, vec!["Grim Pvp".to_string(), "Nib Sprocket".to_string()]);
    }

    #[test]
    fn an_old_flat_file_builds_as_before() {
        let direct = build(&forever_db(), &ctx(Client::Forever), |_| Sent::default()).unwrap();
        let through_homes: Vec<CharacterUpload> = build_homes(&forever_db(), &ctx(Client::Forever))
            .into_iter()
            .map(|(home, upload)| {
                assert_eq!(home, None);
                upload
            })
            .collect();
        assert_eq!(through_homes, direct);
    }

    #[test]
    fn forever_characters_send_their_server() {
        let uploads = build(&forever_db(), &ctx(Client::Forever), |_| Sent::default()).unwrap();
        let tess = &by_key(&uploads, "Tess Rider").payload.character;
        assert_eq!(tess.server, Some(4620), "the last player's server from meta.player");
        assert_eq!(tess.realm_type.as_deref(), Some("pvp"), "the realm type is still sent");
        assert_eq!(by_key(&uploads, "Mira Vale").payload.character.server, Some(4621), "from the newest own death");

        let mut db = forever_db();
        db["meta"]["player"]["server"] = json!(0);
        let uploads = build(&db, &ctx(Client::Forever), |_| Sent::default()).unwrap();
        let json = serde_json::to_value(&by_key(&uploads, "Tess Rider").payload).unwrap();
        assert!(json["character"].get("server").is_none(), "no server when it is not a positive number");

        let mut db = era_db();
        db["meta"]["player"]["server"] = json!(4620);
        let uploads = build(&db, &ctx(Client::Era), |_| Sent::default()).unwrap();
        assert_eq!(by_key(&uploads, "Tessa-Firemaw").payload.character.server, None, "Era has realms, not servers");
    }

    #[test]
    fn forever_world_keys_use_the_servers_the_website_knows() {
        let context = Context { known_servers: BTreeSet::from([4620, 4621]), ..ctx(Client::Forever) };
        let keys: Vec<String> = world_keys(&forever_db(), &context).unwrap().into_iter().collect();
        assert_eq!(keys, vec!["forever|us|4620".to_string(), "forever|us|4621".to_string()]);

        let context = Context { known_servers: BTreeSet::from([4620]), ..ctx(Client::Forever) };
        let keys: Vec<String> = world_keys(&forever_db(), &context).unwrap().into_iter().collect();
        assert_eq!(keys, vec!["forever|us|4620".to_string(), "forever|us|pvp".to_string()], "an unknown server takes the realm type");

        let no_server = json!({ "meta": { "client": "forever", "player": { "key": "Tess Rider" } } });
        let keys: Vec<String> = world_keys(&no_server, &context).unwrap().into_iter().collect();
        assert_eq!(keys, vec!["forever|us|pvp".to_string()], "no server takes the realm type");

        let mut era = era_db();
        era["meta"]["player"]["server"] = json!(4620);
        let context = Context { known_servers: BTreeSet::from([4620]), ..ctx(Client::Era) };
        let keys: Vec<String> = world_keys(&era, &context).unwrap().into_iter().collect();
        assert_eq!(keys, vec!["era|eu|Firemaw".to_string()], "Era keeps the realm");
    }

    #[test]
    fn never_sends_back_what_the_website_sent() {
        let mut db = era_db();
        db["deaths"].as_array_mut().unwrap().push(json!(
            { "id": "Tessa-Firemaw:500", "t": 500, "origin": "website", "victim": { "key": "Tessa-Firemaw", "level": 42 },
              "killer": { "key": "Brute-Firemaw", "level": 50 }, "confidence": "exact", "classification": "fair" }
        ));
        db["marks"]["events"].as_array_mut().unwrap().push(json!(
            { "t": 510, "delta": 5, "reason": "catch", "outlaw": "Brute", "total": 11, "hunter": "Tessa-Firemaw", "origin": "website" }
        ));
        let uploads = build(&db, &ctx(Client::Era), |_| Sent::default()).unwrap();
        let tessa = by_key(&uploads, "Tessa-Firemaw");
        assert!(tessa.payload.deaths.iter().all(|d| d.t != 500), "a death from the website is not uploaded again");
        assert!(tessa.payload.bounty_events.iter().all(|e| e.t != 510), "a bounty event from the website is not uploaded again");
    }

    #[test]
    fn forever_names_have_no_realm_and_take_the_realm_type() {
        let db = json!({
            "meta": { "player": { "key": "Tess Rider", "race": "Human", "class": "PALADIN", "level": 14 } },
            "duels": { "x": { "winner": "Rot Brute", "loser": "Tess Rider", "t": 50, "winnerLevel": 16, "loserLevel": 14, "origin": "local", "faction": "Alliance" } }
        });
        let context = Context { region: Some("us".into()), ..ctx(Client::Forever) };
        let uploads = build(&db, &context, |_| Sent::default()).unwrap();
        let payload = &uploads[0].payload;
        assert_eq!(payload.character.client, Client::Forever);
        assert_eq!(payload.character.name, "Tess Rider");
        assert_eq!(payload.character.realm, None);
        assert_eq!(payload.character.realm_type.as_deref(), Some("pvp"));
        assert_eq!(payload.character.region, "us", "the setting fills in a missing meta.region");
        assert_eq!(payload.duels[0].winner.name, "Rot Brute");
    }

    /// `HH_SAVED_FILE=<path to HeadHunter.lua> cargo test --lib real_file -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn real_file() {
        let path = std::env::var("HH_SAVED_FILE").expect("HH_SAVED_FILE");
        let db = crate::lua::read_variable(&std::fs::read_to_string(&path).unwrap(), "HeadHunter_DB").unwrap();
        let context = Context { region: Some("us".into()), ..ctx(Client::Era) };
        let uploads = build(&db, &context, |_| Sent::default()).unwrap();
        println!("characters: {:?}", characters_in(&db));
        for upload in uploads {
            let p = &upload.payload;
            println!(
                "{} -> {} deaths, {} catches, {} duels, {} bounty events, {} zones, {} bytes",
                upload.key, p.deaths.len(), p.catches.len(), p.duels.len(), p.bounty_events.len(), p.zones.len(),
                serde_json::to_string(p).unwrap().len()
            );
        }
    }

    #[test]
    fn sends_the_glasses_raised_to_catches_once_with_the_last_character() {
        let db = json!({
            "meta": { "region": "us", "client": "forever", "player": { "key": "Tess Rider" } },
            "glasses": {
                "Grim Reaper:100:Rowan Ash": { "outlaw": "Grim Reaper", "caughtAt": 100, "t": 150, "by": "Rowan Ash", "origin": "peer" },
                "Grim Reaper:100:Tess Rider": { "outlaw": "Grim Reaper", "caughtAt": 100, "t": 160, "by": "Tess Rider", "origin": "local", "popup": true },
                "Old Gank:50:Tess Rider": { "outlaw": "Old Gank", "caughtAt": 50, "t": 60, "by": "Tess Rider", "origin": "website" }
            }
        });
        let uploads = build(&db, &ctx(Client::Forever), |_| Sent::default()).unwrap();
        let glasses = &uploads[0].payload.glasses;
        assert_eq!(glasses.len(), 2, "ours and a peer's, never one the website sent");
        assert_eq!(glasses[0].outlaw.name, "Grim Reaper");
        assert_eq!(glasses[0].caught_at, 100);
        assert_eq!(glasses[0].by.name, "Rowan Ash");
        assert!(!glasses[0].popup, "from the list");
        assert!(glasses[1].popup, "from the popup");
        assert_eq!(serde_json::to_value(&glasses[1]).unwrap()["popup"], true);
        assert!(serde_json::to_value(&glasses[0]).unwrap().get("popup").is_none(), "only sent when true");
        assert_eq!(uploads[0].sent_after.glasses, 160);

        let later = build(&db, &ctx(Client::Forever), |_| uploads[0].sent_after.clone()).unwrap();
        assert!(later.iter().all(|u| u.payload.glasses.is_empty()), "sent once");
    }

    #[test]
    fn sends_each_characters_own_honorable_kills_once_also_from_a_character_without_deaths() {
        let db = json!({
            "meta": { "region": "eu", "client": "era", "player": { "key": "Tessa-Firemaw", "faction": "Alliance" } },
            "honorKills": [
                { "t": 100, "seq": 1, "by": "Tessa-Firemaw", "mapID": 1417, "x": 0.42, "y": 0.65, "layer": 3,
                  "victim": "Gank-Stonespine", "honor": 12 },
                { "t": 100, "seq": 2, "by": "Tessa-Firemaw", "mapID": 1417, "x": 1.7 },
                { "t": 90, "seq": 1, "by": "Alt-Firemaw", "mapID": 1436 }
            ]
        });
        let uploads = build(&db, &ctx(Client::Era), |_| Sent::default()).unwrap();
        let of = |key: &str| uploads.iter().find(|u| u.key == key).unwrap();

        let tessa = &of("Tessa-Firemaw").payload.honor_kills;
        assert_eq!(tessa.len(), 2, "only her own");
        assert_eq!(tessa[0].victim.as_ref().map(|v| (v.name.as_str(), v.realm.as_deref())), Some(("Gank", Some("Stonespine"))));
        assert_eq!((tessa[0].map_id, tessa[0].layer, tessa[0].honor), (Some(1417), Some(3), Some(12)));
        assert_eq!(tessa[1].seq, 2);
        assert_eq!(tessa[1].x, None, "off the map");
        assert!(serde_json::to_value(&tessa[1]).unwrap().get("victim").is_none(), "no name, no victim");
        assert_eq!(of("Tessa-Firemaw").sent_after.honor_kills, 100);

        let alt = &of("Alt-Firemaw").payload;
        assert_eq!(alt.honor_kills.len(), 1, "a character with kills uploads them without any death");
        assert_eq!(alt.character.name, "Alt");

        let later = build(&db, &ctx(Client::Era), |key| uploads.iter().find(|u| u.key == key).unwrap().sent_after.clone()).unwrap();
        assert!(later.iter().all(|u| u.payload.honor_kills.is_empty()), "sent once");
    }

    #[test]
    fn each_home_takes_its_own_region_over_the_accounts_last_one() {
        let db = json!({
            "meta": { "client": "forever", "region": "cn", "home": "forever|6746" },
            "homes": {
                "forever|4613": { "region": "us", "player": { "key": "Cz Qs" }, "deaths": [] },
                "forever|6746": { "region": "cn", "player": { "key": "Niu Zhan" }, "deaths": [] },
                "forever|4619": { "player": { "key": "Old Home" }, "deaths": [] }
            }
        });
        let region = |key: &str| {
            let home = homes(&db).into_iter().find(|h| h.key.as_deref() == Some(key)).unwrap();
            text(&home.db["meta"]["region"])
        };

        assert_eq!(region("forever|4613").as_deref(), Some("us"), "a US server's home stays US");
        assert_eq!(region("forever|6746").as_deref(), Some("cn"));
        assert_eq!(region("forever|4619").as_deref(), Some("cn"), "an older addon's home falls back to the account's");
    }

    #[test]
    fn sends_each_characters_own_witness_records_once_never_ones_from_others() {
        let db = json!({
            "meta": { "region": "eu", "client": "era", "player": { "key": "Tessa-Firemaw", "faction": "Alliance" } },
            "witness": {
                "Gank-Stonespine:100:Tessa-Firemaw": { "outlaw": "Gank-Stonespine", "t": 100, "mapID": 1417, "x": 0.4,
                    "y": 0.6, "killer": "Rowan-Firemaw", "by": "Tessa-Firemaw", "origin": "local" },
                "Gank-Stonespine:101:Rowan-Firemaw": { "outlaw": "Gank-Stonespine", "t": 101, "by": "Rowan-Firemaw",
                    "origin": "peer" },
                "guid:Player-1-0A:102:Tessa-Firemaw": { "outlaw": "guid:Player-1-0A", "t": 102, "by": "Tessa-Firemaw",
                    "origin": "local" }
            }
        });
        let uploads = build(&db, &ctx(Client::Era), |_| Sent::default()).unwrap();
        let tessa = &uploads.iter().find(|u| u.key == "Tessa-Firemaw").unwrap();

        assert_eq!(tessa.payload.witnesses.len(), 1, "her own, with a name");
        let w = &tessa.payload.witnesses[0];
        assert_eq!((w.outlaw.name.as_str(), w.outlaw.realm.as_deref()), ("Gank", Some("Stonespine")));
        assert_eq!(w.killer.as_ref().map(|k| k.name.as_str()), Some("Rowan"));
        assert_eq!((w.t, w.map_id), (100, Some(1417)));
        assert!(uploads.iter().all(|u| u.key != "Rowan-Firemaw"), "a peer's record is not ours to send");

        let later = build(&db, &ctx(Client::Era), |key| uploads.iter().find(|u| u.key == key).unwrap().sent_after.clone()).unwrap();
        assert!(later.iter().all(|u| u.payload.witnesses.is_empty()), "sent once");
    }

    #[test]
    fn sends_whether_the_character_was_in_the_wars_zone() {
        let war = |present: Value| json!({ "id": "Tessa-Firemaw:1417:100", "zone": 1417, "started": 100, "ended": 200, "peak": 2,
            "present": present, "fighters": {} });
        assert_eq!(super::war(&war(json!(true)), Client::Era).unwrap().present, Some(true));
        assert_eq!(super::war(&war(json!(false)), Client::Era).unwrap().present, Some(false), "only heard of it");
        let old = super::war(&war(Value::Null), Client::Era).unwrap();
        assert_eq!(old.present, None, "an older addon does not say");
        assert!(serde_json::to_value(&old).unwrap().get("present").is_none());
    }

    #[test]
    fn sends_each_finished_war_once_with_the_players_seen_and_never_one_still_going() {
        let db = json!({
            "meta": { "region": "eu", "client": "era", "player": { "key": "Tessa-Firemaw", "faction": "Alliance" } },
            "wars": [
                { "id": "Tessa-Firemaw:1417:100", "by": "Tessa-Firemaw", "zone": 1417, "started": 100, "ended": 900,
                  "lastHot": 900, "peak": 2, "layers": [7, 3, 3], "kills": 4, "honor": 40, "deaths": 1,
                  "fighters": {
                      "Tessa-Firemaw": { "side": "ally", "faction": "Alliance", "first": 100, "last": 900 },
                      "Gank-Stonespine": { "side": "enemy", "faction": "Horde", "class": "ROGUE", "race": "Scourge",
                                           "level": 60, "guild": "Red Hand", "first": 200, "last": 800 },
                      "guid:Player-1-0001": { "side": "enemy", "first": 300, "last": 300 }
                  } },
                { "id": "Tessa-Firemaw:1436:2000", "by": "Tessa-Firemaw", "zone": 1436, "started": 2000,
                  "lastHot": 2100, "peak": 1, "layers": [], "kills": 0, "honor": 0, "deaths": 0, "fighters": {} },
                { "id": "Alt-Firemaw:1417:100", "by": "Alt-Firemaw", "zone": 1417, "started": 100, "ended": 700,
                  "lastHot": 700, "peak": 3, "layers": [], "kills": 1, "honor": 9, "deaths": 0, "fighters": {} }
            ]
        });
        let uploads = build(&db, &ctx(Client::Era), |_| Sent::default()).unwrap();
        let of = |key: &str| uploads.iter().find(|u| u.key == key).unwrap();

        let wars = &of("Tessa-Firemaw").payload.wars;
        assert_eq!(wars.len(), 1, "only the finished one");
        let war = &wars[0];
        assert_eq!((war.map_id, war.started, war.ended, war.peak_fire), (1417, 100, 900, 2));
        assert_eq!(war.layers, vec![3, 7], "each layer once");
        assert_eq!((war.kills, war.honor, war.deaths), (4, 40, 1));
        assert_eq!(war.fighters.len(), 2, "a player known only by GUID is left out");
        let gank = war.fighters.iter().find(|f| f.name == "Gank").unwrap();
        assert_eq!(gank.realm.as_deref(), Some("Stonespine"));
        assert_eq!(gank.side, "enemy");
        assert_eq!(gank.faction.as_deref(), Some("horde"));
        assert_eq!(gank.class.as_deref(), Some("rogue"));
        assert_eq!(gank.race.as_deref(), Some("undead"));
        assert_eq!(gank.guild.as_deref(), Some("Red Hand"));
        assert_eq!(of("Tessa-Firemaw").sent_after.wars, 900);
        assert_eq!(of("Alt-Firemaw").payload.wars.len(), 1, "a character's war goes with that character");

        let later = build(&db, &ctx(Client::Era), |key| uploads.iter().find(|u| u.key == key).unwrap().sent_after.clone()).unwrap();
        assert!(later.iter().all(|u| u.payload.wars.is_empty()), "sent once");
    }

    #[test]
    fn a_cut_of_honorable_kills_never_splits_one_second() {
        let kills = json!([
            { "t": 10, "seq": 1 }, { "t": 20, "seq": 1 }, { "t": 20, "seq": 2 }, { "t": 20, "seq": 3 }, { "t": 30, "seq": 1 }
        ]);
        let cut = whole_seconds_limited(values(&kills), 0, 3);
        assert_eq!(cut.iter().map(|k| int(&k["t"]).unwrap()).collect::<Vec<_>>(), vec![10], "second 20 waits whole");
        let next = whole_seconds_limited(values(&kills), 10, 3);
        assert_eq!(next.len(), 3, "then all of second 20");
        let one_second = json!([{ "t": 20, "seq": 1 }, { "t": 20, "seq": 2 }]);
        let alone = whole_seconds_limited(values(&one_second), 0, 1);
        assert_eq!(alone.len(), 1, "a second over the limit by itself is cut");
    }

    #[test]
    fn the_last_character_without_a_guild_says_so_others_say_nothing() {
        let db = json!({
            "meta": { "region": "eu", "client": "era", "player": { "key": "Tessa-Firemaw", "level": 42 } },
            "deaths": [
                { "id": "Alt-Firemaw:200", "t": 200, "victim": { "key": "Alt-Firemaw", "level": 20 },
                  "killer": { "key": "Brute-Firemaw", "level": 30 }, "confidence": "exact", "classification": "normal" },
                { "id": "Tessa-Firemaw:300", "t": 300, "victim": { "key": "Tessa-Firemaw", "level": 42 },
                  "killer": { "key": "Brute-Firemaw", "level": 30 }, "confidence": "exact", "classification": "normal" }
            ]
        });
        let uploads = build(&db, &ctx(Client::Era), |_| Sent::default()).unwrap();
        let json = |key: &str| serde_json::to_value(&uploads.iter().find(|u| u.key == key).unwrap().payload.character).unwrap();
        assert_eq!(json("Tessa-Firemaw")["guild"], Value::Null, "left the guild: null");
        assert!(json("Tessa-Firemaw").as_object().unwrap().contains_key("guild"));
        assert!(!json("Alt-Firemaw").as_object().unwrap().contains_key("guild"), "unknown: not sent");

        let mut db = db;
        db["meta"]["player"]["guild"] = json!("Night Watch");
        let uploads = build(&db, &ctx(Client::Era), |_| Sent::default()).unwrap();
        let tessa = uploads.iter().find(|u| u.key == "Tessa-Firemaw").unwrap();
        assert_eq!(tessa.payload.character.guild, Some(Some("Night Watch".into())));
    }

    #[test]
    fn the_saved_client_and_region_win_over_the_game() {
        let db = json!({ "meta": { "client": "forever", "region": "cn", "player": { "key": "Tess Rider" } } });
        let context = Context { client: None, region: Some("us".into()), ..ctx(Client::Era) };
        let uploads = build(&db, &context, |_| Sent::default()).unwrap();
        assert!(uploads.iter().all(|u| u.payload.character.client == Client::Forever && u.payload.character.region == "cn"));

        let db = json!({ "meta": { "player": { "key": "Tess Rider" } } });
        let context = Context { region: Some("cn".into()), ..ctx(Client::Forever) };
        let uploads = build(&db, &context, |_| Sent::default()).unwrap();
        assert!(uploads.iter().all(|u| u.payload.character.region == "cn"), "the game's region fills in a missing meta.region");
    }

    #[test]
    fn needs_a_client_and_a_region() {
        let db = json!({ "meta": { "player": { "key": "Tess-Firemaw" } } });
        let context = Context { region: None, ..ctx(Client::Era) };
        assert_eq!(build(&db, &context, |_| Sent::default()), Err(PayloadError::UnknownRegion));
        let context = Context { client: None, ..ctx(Client::Era) };
        assert_eq!(build(&db, &context, |_| Sent::default()), Err(PayloadError::UnknownClient));
    }
}
