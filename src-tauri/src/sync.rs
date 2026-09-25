//! The sync engine: reads every account's HeadHunter.lua, builds one upload per
//! character (payload.rs), sends it (api.rs) and remembers what was sent (store.rs).
//! Runs when the game writes the file (after it settles), at start, on a timer and on
//! "Sync now"; retries later when the website cannot be reached.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use notify::{RecursiveMode, Watcher};
use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tauri_plugin_notification::NotificationExt;

use crate::api::{Api, DownloadOutcome, UploadOutcome, User};
use crate::installs::{self, Install};
use crate::payload::{self, Client, Context};
use crate::store::{self, DownloadResult, InstallSettings, Settings, State, SyncResult};
use crate::{config, download, lua};

/// Waits after a change so the game has finished writing the file.
const SETTLE: Duration = Duration::from_secs(5);
/// Waits between tries while the website cannot be reached.
const RETRY_STEPS: [u64; 4] = [30, 120, 600, 1800];

#[derive(Default)]
struct Schedule {
    change_at: Option<Instant>,
    next_interval: Option<Instant>,
    retry_at: Option<Instant>,
    retry_step: usize,
}

pub struct Engine {
    data_dir: PathBuf,
    api: Api,
    settings: Mutex<Settings>,
    state: Mutex<State>,
    schedule: Mutex<Schedule>,
    watcher: Mutex<Option<notify::RecommendedWatcher>>,
    problem: Mutex<Option<String>>,
    running: tokio::sync::Mutex<()>,
    syncing: AtomicBool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Status {
    pub build: &'static str,
    pub api_url: &'static str,
    pub version: &'static str,
    pub user: Option<User>,
    pub syncing: bool,
    pub last_run: Option<i64>,
    /// Seconds until the next timed sync, when the timer is on.
    pub next_run_in: Option<u64>,
    pub problem: Option<String>,
    pub installs: Vec<InstallStatus>,
}

#[derive(Debug, Clone, Serialize)]
pub struct InstallStatus {
    pub path: String,
    pub client: Client,
    pub addon_version: Option<String>,
    pub settings: InstallSettings,
    pub accounts: Vec<AccountStatus>,
    /// The last download of the website's data into this game.
    pub download: Option<DownloadResult>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AccountStatus {
    pub name: String,
    pub characters: Vec<CharacterStatus>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CharacterStatus {
    pub key: String,
    pub last_played: bool,
    pub result: Option<SyncResult>,
}

impl Engine {
    pub fn new(data_dir: PathBuf) -> Arc<Self> {
        let settings: Settings = store::load(&data_dir.join("settings.json"));
        let state: State = store::load(&data_dir.join("state.json"));
        Arc::new(Self {
            data_dir,
            api: Api::new(),
            settings: Mutex::new(settings),
            state: Mutex::new(state),
            schedule: Mutex::new(Schedule::default()),
            watcher: Mutex::new(None),
            problem: Mutex::new(None),
            running: tokio::sync::Mutex::new(()),
            syncing: AtomicBool::new(false),
        })
    }

    pub fn api(&self) -> &Api {
        &self.api
    }

    pub fn settings(&self) -> Settings {
        self.settings.lock().unwrap().clone()
    }

    pub fn save_settings(self: &Arc<Self>, settings: Settings) -> Result<(), String> {
        store::save(&self.data_dir.join("settings.json"), &settings).map_err(|e| e.to_string())?;
        *self.settings.lock().unwrap() = settings;
        self.plan_interval();
        self.rewatch();
        Ok(())
    }

    fn save_state(&self) {
        let state = self.state.lock().unwrap().clone();
        let _ = store::save(&self.data_dir.join("state.json"), &state);
    }

    pub fn signed_in(&self, user: User) {
        let mut settings = self.settings();
        settings.user_name = Some(user.name);
        settings.email = user.email;
        let _ = store::save(&self.data_dir.join("settings.json"), &settings);
        *self.settings.lock().unwrap() = settings;
        *self.problem.lock().unwrap() = None;
    }

    pub fn signed_out(&self) {
        store::clear_token();
        let mut settings = self.settings();
        settings.user_name = None;
        let _ = store::save(&self.data_dir.join("settings.json"), &settings);
        *self.settings.lock().unwrap() = settings;
    }

    fn user(&self) -> Option<User> {
        let settings = self.settings();
        store::token()?;
        Some(User { name: settings.user_name.unwrap_or_default(), email: settings.email })
    }

    pub fn installs(&self) -> Vec<Install> {
        installs::find(&self.settings().wow_folders)
    }

    fn install_settings(&self, install: &Install) -> InstallSettings {
        self.settings().installs.get(&install.path.to_string_lossy().to_string()).cloned().unwrap_or_default()
    }

    // ------------------------------------------------------------------ status

    pub fn status(&self) -> Status {
        let state = self.state.lock().unwrap().clone();
        let installs = self
            .installs()
            .into_iter()
            .map(|install| {
                let path = install.path.to_string_lossy().to_string();
                let accounts = install
                    .accounts
                    .iter()
                    .map(|account| {
                        let (characters, error) = match read_db(&account.saved_file) {
                            Ok(db) => (
                                payload::characters_in(&db),
                                payload::world_keys(&db, &context(&install, &self.install_settings(&install)))
                                    .err()
                                    .map(|e| e.to_string()),
                            ),
                            Err(e) => (Default::default(), Some(e)),
                        };
                        AccountStatus {
                            name: account.name.clone(),
                            characters: characters
                                .into_iter()
                                .map(|(key, last_played)| CharacterStatus {
                                    result: state.results.get(&store::state_key(&path, &account.name, &key)).cloned(),
                                    key,
                                    last_played,
                                })
                                .collect(),
                            error,
                        }
                    })
                    .collect();
                InstallStatus {
                    settings: self.install_settings(&install),
                    download: state.downloads.get(&path).cloned(),
                    path,
                    client: install.client,
                    addon_version: install.addon_version.clone(),
                    accounts,
                }
            })
            .collect();
        let next_run_in = self
            .schedule
            .lock()
            .unwrap()
            .next_interval
            .map(|at| at.saturating_duration_since(Instant::now()).as_secs());
        Status {
            build: config::BUILD,
            api_url: config::API_URL,
            version: env!("CARGO_PKG_VERSION"),
            user: self.user(),
            syncing: self.syncing.load(Ordering::SeqCst),
            last_run: state.last_run,
            next_run_in,
            problem: self.problem.lock().unwrap().clone(),
            installs,
        }
    }

    // ---------------------------------------------------------------- the run

    /// One sync of every account. Returns false when another run was busy.
    pub async fn sync(self: &Arc<Self>, app: &AppHandle) -> bool {
        let Ok(_guard) = self.running.try_lock() else { return false };
        self.syncing.store(true, Ordering::SeqCst);
        let _ = app.emit("status-changed", ());

        let outcome = self.sync_all(app).await;
        let now = now();
        {
            let mut schedule = self.schedule.lock().unwrap();
            match &outcome {
                Err(Retry(_)) => {
                    let step = schedule.retry_step.min(RETRY_STEPS.len() - 1);
                    schedule.retry_at = Some(Instant::now() + Duration::from_secs(RETRY_STEPS[step]));
                    schedule.retry_step += 1;
                }
                _ => {
                    schedule.retry_at = None;
                    schedule.retry_step = 0;
                }
            }
        }
        *self.problem.lock().unwrap() = outcome.err().map(|Retry(message)| message);
        self.state.lock().unwrap().last_run = Some(now);
        self.save_state();
        self.plan_interval();

        self.syncing.store(false, Ordering::SeqCst);
        let _ = app.emit("status-changed", ());
        true
    }

    async fn sync_all(self: &Arc<Self>, app: &AppHandle) -> Result<(), Retry> {
        let Some(token) = store::token() else {
            return Err(Retry("Sign in to start syncing.".into()));
        };
        let mut retry: Option<String> = None;

        for install in self.installs() {
            let install_settings = self.install_settings(&install);
            if !install_settings.enabled {
                continue;
            }
            let path = install.path.to_string_lossy().to_string();
            let mut worlds = std::collections::BTreeSet::new();
            for account in &install.accounts {
                let db = match read_db(&account.saved_file) {
                    Ok(db) => db,
                    Err(e) => {
                        retry.get_or_insert(e);
                        continue;
                    }
                };
                let context = context(&install, &install_settings);
                worlds.extend(payload::world_keys(&db, &context).unwrap_or_default());
                let sent = self.state.lock().unwrap().sent.clone();
                // An unknown region needs the player (a login or a setting), not a retry;
                // the account shows why on the status screen.
                let Ok(uploads) = payload::build(&db, &context, |key| {
                    sent.get(&store::state_key(&path, &account.name, key)).cloned().unwrap_or_default()
                }) else {
                    continue;
                };

                for upload in uploads {
                    let key = store::state_key(&path, &account.name, &upload.key);
                    let records = upload.payload.deaths.len()
                        + upload.payload.catches.len()
                        + upload.payload.duels.len()
                        + upload.payload.bounty_events.len();
                    let outcome = self.api.upload(&token, &upload.payload).await;
                    let (name, message, keep) = match &outcome {
                        UploadOutcome::Sent => ("sent", None, true),
                        UploadOutcome::AlreadyThere => ("already_there", None, true),
                        UploadOutcome::Claimed(m) => ("claimed", Some(m.clone()), false),
                        UploadOutcome::Rejected(m) => ("rejected", Some(m.clone()), false),
                        UploadOutcome::SignedOut => ("error", Some("Signed out on the website. Sign in again.".to_string()), false),
                        UploadOutcome::Retry(m) => ("retry", Some(m.clone()), false),
                    };
                    {
                        let mut state = self.state.lock().unwrap();
                        if keep {
                            state.sent.insert(key.clone(), upload.sent_after.clone());
                        }
                        state.results.insert(key, SyncResult { at: now(), outcome: name.into(), message: message.clone(), records });
                    }
                    self.save_state();
                    let _ = app.emit("status-changed", ());

                    match outcome {
                        UploadOutcome::SignedOut => {
                            self.signed_out();
                            notify(app, "Signed out", "HeadHunter Sync was signed out on the website. Sign in again to keep syncing.");
                            return Err(Retry("Signed out on the website. Sign in again.".into()));
                        }
                        UploadOutcome::Claimed(m) | UploadOutcome::Rejected(m) => {
                            notify(app, &format!("{} not synced", display_name(&upload.key)), &m);
                        }
                        UploadOutcome::Retry(m) => {
                            retry.get_or_insert(m);
                        }
                        _ => {}
                    }
                }
            }

            if install.addon_version.is_some() && !worlds.is_empty() {
                let worlds: Vec<String> = worlds.into_iter().collect();
                match self.download(&token, &install, &path, &worlds).await {
                    Err(DownloadStop::SignedOut) => {
                        self.signed_out();
                        notify(app, "Signed out", "HeadHunter Sync was signed out on the website. Sign in again to keep syncing.");
                        return Err(Retry("Signed out on the website. Sign in again.".into()));
                    }
                    Err(DownloadStop::Retry(m)) => {
                        retry.get_or_insert(m);
                    }
                    Ok(()) => {}
                }
                let _ = app.emit("status-changed", ());
            }
        }
        match retry {
            Some(message) => Err(Retry(message)),
            None => Ok(()),
        }
    }

    /// Fetches the website's data for these worlds and writes it into the game
    /// (download.rs). The ETag is only sent while the data file is still there.
    async fn download(&self, token: &str, install: &Install, path: &str, worlds: &[String]) -> Result<(), DownloadStop> {
        let previous = self.state.lock().unwrap().downloads.get(path).cloned();
        let etag = previous.as_ref().and_then(|p| p.etag.clone()).filter(|_| download::written(&install.path));
        let mut result = DownloadResult {
            at: now(),
            outcome: "unchanged".into(),
            message: None,
            written_at: previous.as_ref().and_then(|p| p.written_at),
            wanted: previous.as_ref().map_or(0, |p| p.wanted),
            etag: etag.clone(),
        };
        let stop = match self.api.download(token, worlds, etag.as_deref()).await {
            DownloadOutcome::Fresh(data, etag) => {
                match download::write(&install.path, &data) {
                    Ok(changed) => {
                        result.outcome = if changed { "updated" } else { "unchanged" }.into();
                        if changed || result.written_at.is_none() {
                            result.written_at = Some(now());
                        }
                        result.wanted = download::wanted_count(&data);
                        result.etag = etag;
                    }
                    Err(e) => {
                        result.outcome = "error".into();
                        result.message = Some(e);
                        result.etag = None;
                    }
                }
                None
            }
            DownloadOutcome::Unchanged => None,
            DownloadOutcome::SignedOut => {
                result.outcome = "error".into();
                result.message = Some("Signed out on the website. Sign in again.".into());
                Some(DownloadStop::SignedOut)
            }
            DownloadOutcome::Retry(m) => {
                result.outcome = "retry".into();
                result.message = Some(m.clone());
                Some(DownloadStop::Retry(m))
            }
        };
        self.state.lock().unwrap().downloads.insert(path.to_string(), result);
        self.save_state();
        stop.map_or(Ok(()), Err)
    }

    // --------------------------------------------------------------- schedule

    pub fn plan_interval(&self) {
        let minutes = self.settings().interval_minutes;
        self.schedule.lock().unwrap().next_interval =
            (minutes > 0).then(|| Instant::now() + Duration::from_secs(u64::from(minutes) * 60));
    }

    /// Syncs soon: at start, after a change or when asked.
    pub fn request(&self, after: Duration) {
        let at = Instant::now() + after;
        let mut schedule = self.schedule.lock().unwrap();
        schedule.change_at = Some(schedule.change_at.map_or(at, |current| current.max(at)));
    }

    /// Checks every two seconds whether a sync is due.
    pub fn start_scheduler(self: &Arc<Self>, app: AppHandle) {
        if self.settings().sync_on_start {
            self.request(Duration::from_secs(3));
        }
        self.plan_interval();
        let engine = Arc::clone(self);
        tauri::async_runtime::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(2)).await;
                let due = {
                    let mut schedule = engine.schedule.lock().unwrap();
                    let now = Instant::now();
                    let due = [schedule.change_at, schedule.next_interval, schedule.retry_at]
                        .iter()
                        .any(|at| at.is_some_and(|at| at <= now));
                    if due {
                        schedule.change_at = None;
                        schedule.retry_at = None;
                    }
                    due
                };
                if due {
                    engine.sync(&app).await;
                }
            }
        });
    }

    /// Watches each account's SavedVariables folder (the game replaces the file).
    pub fn rewatch(self: &Arc<Self>) {
        let mut slot = self.watcher.lock().unwrap();
        *slot = None;
        if !self.settings().sync_on_change {
            return;
        }
        let engine = Arc::downgrade(self);
        let watcher = notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
            let Ok(event) = event else { return };
            let ours = event
                .paths
                .iter()
                .any(|p| p.file_name().is_some_and(|n| n.eq_ignore_ascii_case("HeadHunter.lua")));
            if ours && !event.kind.is_access() {
                if let Some(engine) = engine.upgrade() {
                    engine.request(SETTLE);
                }
            }
        });
        let Ok(mut watcher) = watcher else { return };
        for install in self.installs() {
            for account in install.accounts {
                if let Some(dir) = account.saved_file.parent() {
                    let _ = watcher.watch(dir, RecursiveMode::NonRecursive);
                }
            }
        }
        *slot = Some(watcher);
    }
}

struct Retry(String);

enum DownloadStop {
    SignedOut,
    Retry(String),
}

/// What the saved file cannot say: the region comes from the setting, else from the
/// game's own config (`SET portal`); the addon's saved region still wins over both.
fn context(install: &Install, settings: &InstallSettings) -> Context {
    Context {
        client: install.client,
        region: settings.region.clone().or_else(|| install.portal_region.clone()),
        realm_type: settings.realm_type.clone(),
        addon_version: install.addon_version.clone().unwrap_or_else(|| "unknown".into()),
    }
}

fn read_db(path: &std::path::Path) -> Result<serde_json::Value, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("Cannot read {}: {e}", path.display()))?;
    lua::read_variable(&text, "HeadHunter_DB").map_err(|e| format!("{}: {e}", path.display()))
}

/// "Name-Realm" shows as "Name".
fn display_name(key: &str) -> String {
    key.split('-').next().unwrap_or(key).to_string()
}

fn notify(app: &AppHandle, title: &str, body: &str) {
    let _ = app.notification().builder().title(title).body(body).show();
}

pub fn now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or_default()
}

/// The name this PC signs in with (one token per device on the website).
pub fn device_name() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .map(|name| format!("HeadHunter Sync on {name}"))
        .unwrap_or_else(|_| "HeadHunter Sync".into())
}
