//! Turns the addon's `HeadHunter_DB` into the website's sync uploads
//! (`StoreUploadRequest` in the website repo), one upload per character.
//!
//! The saved data is account-wide: own deaths name their victim, catches their hunter,
//! bounty events their hunter (addon 0.1.4+). `meta.player` is the character played
//! last; witnessed duels, learned zones and bounty events without a hunter go with it.
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Client {
    Era,
    Forever,
}

/// What the file does not say but the upload needs.
#[derive(Debug, Clone)]
pub struct Context {
    /// From the install folder; `meta.client` wins when present.
    pub client: Client,
    /// Used when the addon has not saved `meta.region` yet.
    pub region: Option<String>,
    /// Forever worlds (pvp, pve, roleplay, hardcore): the addon cannot tell.
    pub realm_type: String,
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
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Character {
    pub client: Client,
    pub region: String,
    pub realm: Option<String>,
    pub realm_type: Option<String>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guild: Option<String>,
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
    #[error("the saved data does not say which region it is from; pick it in Settings")]
    UnknownRegion,
}

/// The WoW Forever beta runs only in the US region (Americas & Oceania) until the
/// release on 2026-11-04; Classic Era saves its region (addon 0.1.3+).
fn default_region(client: Client) -> Option<&'static str> {
    match client {
        Client::Forever => Some("us"),
        Client::Era => None,
    }
}

/// Every character's upload with records newer than `sent(key)`; characters with
/// nothing new are left out.
pub fn build(db: &Value, ctx: &Context, sent: impl Fn(&str) -> Sent) -> Result<Vec<CharacterUpload>, PayloadError> {
    let client = match text(&db["meta"]["client"]).as_deref() {
        Some("era") => Client::Era,
        Some("forever") => Client::Forever,
        _ => ctx.client,
    };
    let region = text(&db["meta"]["region"])
        .or_else(|| ctx.region.clone())
        .or_else(|| default_region(client).map(String::from))
        .ok_or(PayloadError::UnknownRegion)?;

    let deaths = own_deaths(db);
    let last_player = text(&db["meta"]["player"]["key"])
        .or_else(|| deaths.iter().max_by_key(|d| int(&d["t"])).and_then(|d| text(&d["victim"]["key"])));

    let mut keys: BTreeSet<String> = deaths.iter().filter_map(|d| text(&d["victim"]["key"])).collect();
    keys.extend(last_player.clone());

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
                values(&db["duels"]).filter(|d| text(&d["origin"]).as_deref() == Some("local") && !flag(&d["demo"])),
                already.duels,
                MAX_DUELS,
            );
            (src.iter().filter_map(|d| duel(d, client)).collect(), src)
        } else {
            (Vec::new(), Vec::new())
        };
        after.duels = max_t(&duels_src, already.duels);

        let events_src = newest_first_limited(
            values(&db["marks"]["events"]).filter(|e| match text(&e["hunter"]) {
                Some(hunter) => hunter == key,
                None => is_last,
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

        if deaths_out.is_empty() && catches.is_empty() && duels.is_empty() && bounty_events.is_empty() && zones.is_empty() {
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
            },
            sent_after: after,
        });
    }
    Ok(uploads)
}

fn character(db: &Value, key: &str, is_last: bool, client: Client, region: &str, ctx: &Context, deaths: &[&Value]) -> Character {
    let (name, realm) = split_key(key, client);
    let snapshot = if is_last {
        db["meta"]["player"].clone()
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
        name,
        faction: text(&snapshot["faction"]).map(|f| f.to_lowercase()).or_else(|| race.as_deref().and_then(faction_of_race)),
        class: text(&snapshot["class"]).map(|c| c.to_lowercase()),
        race: race.as_deref().and_then(map_race),
        sex: int(&snapshot["sex"]).filter(|s| *s == 2 || *s == 3),
        level: int(&snapshot["level"]).filter(|l| (1..=100).contains(l)),
        guild: text(&snapshot["guild"]).map(|g| g.chars().take(64).collect()),
    }
}

/// The account's own deaths (the `deaths` list), without demo or simulated ones.
fn own_deaths(db: &Value) -> Vec<&Value> {
    values(&db["deaths"])
        .filter(|d| !flag(&d["demo"]) && matches!(text(&d["confidence"]).as_deref(), Some("exact") | Some("inferred")))
        .collect()
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
        faction: race.as_deref().and_then(faction_of_race),
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
            faction: race.as_deref().and_then(faction_of_race),
            race: race.as_deref().and_then(map_race),
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
        faction: text(&d["faction"]).map(|f| f.to_lowercase()).filter(|f| f == "horde" || f == "alliance"),
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

fn zone_list(db: &Value) -> Vec<Zone> {
    let Some(map) = db["zones"].as_object() else { return Vec::new() };
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

/// The addon's race tokens to the website's names.
pub fn map_race(race: &str) -> Option<String> {
    let mapped = match race {
        "Human" => "human",
        "Dwarf" => "dwarf",
        "NightElf" => "night_elf",
        "Gnome" => "gnome",
        "Draenei" => "draenei",
        "Orc" => "orc",
        "Scourge" | "Undead" => "undead",
        "Tauren" => "tauren",
        "Troll" => "troll",
        "BloodElf" => "blood_elf",
        _ => return None,
    };
    Some(mapped.into())
}

pub fn faction_of_race(race: &str) -> Option<String> {
    let faction = match race {
        "Human" | "Dwarf" | "NightElf" | "Gnome" | "Draenei" => "alliance",
        "Orc" | "Scourge" | "Undead" | "Tauren" | "Troll" | "BloodElf" => "horde",
        _ => return None,
    };
    Some(faction.into())
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

fn values(v: &Value) -> Box<dyn Iterator<Item = &Value> + '_> {
    match v {
        Value::Array(list) => Box::new(list.iter()),
        Value::Object(map) => Box::new(map.values()),
        _ => Box::new(std::iter::empty()),
    }
}

fn text(v: &Value) -> Option<String> {
    v.as_str().map(str::trim).filter(|s| !s.is_empty()).map(String::from)
}

fn int(v: &Value) -> Option<i64> {
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
    if let Some(last) = text(&db["meta"]["player"]["key"]) {
        found.insert(last, true);
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn ctx(client: Client) -> Context {
        Context { client, region: None, realm_type: "pvp".into(), addon_version: "0.1.4".into() }
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
                  "killer": { "key": "Brute-Firemaw", "level": -1, "class": "WARRIOR", "race": "Human" },
                  "assists": [ { "name": "Sly", "guid": "Player-1-ABC", "level": 58, "class": "ROGUE", "race": "NightElf" } ],
                  "mapID": 1434, "x": 0.5, "y": 0.25, "confidence": "exact", "classification": "coward" },
                { "id": "Alt-Firemaw:200", "t": 200, "victim": { "key": "Alt-Firemaw", "level": 20, "class": "MAGE", "race": "Troll" },
                  "killer": { "key": "Brute-Firemaw", "level": 30, "race": "Human" },
                  "mapID": 1413, "confidence": "inferred", "classification": "normal" },
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
                  "winnerClass": "SHAMAN", "winnerRace": "Orc", "winnerLevel": 40, "loserClass": "MAGE", "loserRace": "Troll", "loserLevel": 38, "mapID": 1413, "origin": "local", "retreat": true },
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
        assert_eq!(main.duels.len(), 1, "only our own witnessed duels, no demo");
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
        let assist = &death.attackers[1];
        assert_eq!((assist.name.as_deref(), assist.given_name.as_deref(), assist.guid.as_deref()), (None, Some("Sly"), Some("Player-1-ABC")));
        assert_eq!(assist.race.as_deref(), Some("night_elf"));

        let duel = &main.duels[0];
        assert_eq!((duel.winner.name.as_str(), duel.winner.race.as_deref(), duel.faction.as_deref()), ("A", Some("orc"), Some("horde")));
        assert!(duel.retreat);

        let join = main.bounty_events.iter().find(|e| e.kind == "join").unwrap();
        assert_eq!(join.outlaw_rank.as_deref(), Some("most_wanted"));
        assert_eq!(join.outlaw.as_ref().unwrap().name, "Brute");

        let json = serde_json::to_value(main).unwrap();
        assert_eq!(json["bounty_events"][0]["type"], "catch");
        assert_eq!(json["character"]["realm_type"], serde_json::Value::Null);
        assert!(json["deaths"][0]["attackers"][1].get("skull").is_none(), "skull only when true");
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
    fn forever_defaults_to_the_us_region_and_era_needs_one() {
        let db = json!({ "meta": { "player": { "key": "Tess Rider" } } });
        let uploads = build(&db, &ctx(Client::Forever), |_| Sent::default()).unwrap();
        assert!(uploads.iter().all(|u| u.payload.character.region == "us"));

        let db = json!({ "meta": { "player": { "key": "Tess-Firemaw" } } });
        assert_eq!(build(&db, &ctx(Client::Era), |_| Sent::default()), Err(PayloadError::UnknownRegion));
    }
}
