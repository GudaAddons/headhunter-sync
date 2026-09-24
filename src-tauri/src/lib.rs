//! HeadHunter Sync: reads the HeadHunter addon's saved data and uploads it to the
//! HeadHunter website. Lives in the system tray; the window shows status and settings.

pub mod api;
pub mod config;
pub mod installs;
pub mod lua;
pub mod payload;
pub mod store;
pub mod sync;

use std::sync::Arc;
use std::time::Duration;

use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Listener, Manager, State, WindowEvent};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};

use api::{UploadInfo, User};
use store::Settings;
use sync::{Engine, Status};

type Shared<'a> = State<'a, Arc<Engine>>;

#[tauri::command]
fn get_status(engine: Shared<'_>) -> Status {
    engine.status()
}

#[tauri::command]
async fn sign_in(app: AppHandle, engine: Shared<'_>, email: String, password: String) -> Result<User, String> {
    let (token, user) = engine.api().sign_in(email.trim(), &password, &sync::device_name()).await?;
    store::set_token(&token)?;
    engine.signed_in(user.clone());
    engine.request(Duration::from_secs(1));
    refresh_tray(&app, &engine);
    Ok(user)
}

#[tauri::command]
async fn sign_out(app: AppHandle, engine: Shared<'_>) -> Result<(), String> {
    if let Some(token) = store::token() {
        engine.api().sign_out(&token).await;
    }
    engine.signed_out();
    refresh_tray(&app, &engine);
    Ok(())
}

#[tauri::command]
fn sync_now(engine: Shared<'_>) {
    engine.request(Duration::ZERO);
}

#[tauri::command]
fn get_settings(engine: Shared<'_>) -> Settings {
    engine.settings()
}

#[tauri::command]
fn save_settings(app: AppHandle, engine: Shared<'_>, settings: Settings) -> Result<(), String> {
    let autostart = app.autolaunch();
    let result = if settings.start_with_system { autostart.enable() } else { autostart.disable() };
    result.map_err(|e| format!("Could not change the start with the system: {e}"))?;
    engine.save_settings(settings)
}

#[tauri::command]
async fn recent_uploads(engine: Shared<'_>) -> Result<Vec<UploadInfo>, String> {
    let token = store::token().ok_or("Sign in first.")?;
    engine.api().recent_uploads(&token).await
}

fn show_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn refresh_tray(app: &AppHandle, engine: &Engine) {
    if let Some(tray) = app.tray_by_id("main") {
        let status = engine.status();
        let text = match (&status.user, status.syncing) {
            (None, _) => "HeadHunter Sync: signed out".to_string(),
            (Some(_), true) => "HeadHunter Sync: syncing...".to_string(),
            (Some(user), false) => format!("HeadHunter Sync: {}", user.name),
        };
        let _ = tray.set_tooltip(Some(text));
    }
}

fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Open HeadHunter Sync", true, None::<&str>)?;
    let sync = MenuItem::with_id(app, "sync", "Sync now", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &sync, &quit])?;

    TrayIconBuilder::with_id("main")
        .icon(app.default_window_icon().cloned().expect("app icon"))
        .tooltip("HeadHunter Sync")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => show_window(app),
            "sync" => app.state::<Arc<Engine>>().request(Duration::ZERO),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                show_window(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| show_window(app)))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, Some(vec!["--minimized"])))
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            let engine = Engine::new(data_dir);
            app.manage(Arc::clone(&engine));
            build_tray(app.handle())?;
            engine.rewatch();
            engine.start_scheduler(app.handle().clone());

            let handle = app.handle().clone();
            let listener = app.handle().clone();
            listener.listen("status-changed", move |_| {
                let engine = handle.state::<Arc<Engine>>();
                refresh_tray(&handle, &engine);
            });

            if std::env::args().any(|arg| arg == "--minimized") {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.hide();
                }
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            // Closing the window keeps syncing in the tray; Quit is in the tray menu.
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_status,
            sign_in,
            sign_out,
            sync_now,
            get_settings,
            save_settings,
            recent_uploads
        ])
        .run(tauri::generate_context!())
        .expect("error while running HeadHunter Sync");
}
