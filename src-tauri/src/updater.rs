//! App updates (APP-006): production builds check the latest GitHub release on start
//! and every few hours. The player is asked first; the update is signed with the
//! project's key (tauri.conf.json "pubkey"), so only our own builds install.

use std::sync::Mutex;
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_updater::{Update, UpdaterExt};

use crate::config;

const CHECK_EVERY: Duration = Duration::from_secs(6 * 3600);

/// The update found by the last check, waiting for "Install".
#[derive(Default)]
pub struct Pending(Mutex<Option<Update>>);

#[derive(Debug, Clone, Serialize)]
pub struct UpdateInfo {
    pub version: String,
    pub notes: Option<String>,
}

/// Local builds talk to the local website and are never replaced by a release.
pub fn enabled() -> bool {
    config::is_production()
}

pub async fn check(app: &AppHandle) -> Result<Option<UpdateInfo>, String> {
    if !enabled() {
        return Ok(None);
    }
    let update = app
        .updater()
        .map_err(|e| e.to_string())?
        .check()
        .await
        .map_err(|e| format!("Could not check for updates: {e}"))?;
    let info = update.as_ref().map(|u| UpdateInfo { version: u.version.clone(), notes: u.body.clone() });
    *app.state::<Pending>().0.lock().unwrap() = update;
    Ok(info)
}

/// Downloads and installs the waiting update, then restarts on the new version.
pub async fn install(app: &AppHandle) -> Result<(), String> {
    let update = app.state::<Pending>().0.lock().unwrap().take().ok_or("No update is waiting.")?;
    update
        .download_and_install(|_, _| {}, || {})
        .await
        .map_err(|e| format!("The update failed: {e}"))?;
    app.restart()
}

/// Checks now and every CHECK_EVERY; tells the window and, once per version, the tray.
pub fn start_checks(app: AppHandle) {
    if !enabled() {
        return;
    }
    tauri::async_runtime::spawn(async move {
        let mut announced: Option<String> = None;
        loop {
            if let Ok(Some(info)) = check(&app).await {
                let _ = app.emit("update-available", &info);
                if announced.as_ref() != Some(&info.version) {
                    let _ = app
                        .notification()
                        .builder()
                        .title("HeadHunter Sync")
                        .body(format!("Version {} is ready. Open the app to install it.", info.version))
                        .show();
                    announced = Some(info.version);
                }
            }
            tokio::time::sleep(CHECK_EVERY).await;
        }
    });
}
