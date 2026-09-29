//! HeadHunter Sync: reads the HeadHunter addon's saved data and uploads it to the
//! HeadHunter website. Lives in the system tray; the window shows status and settings.

pub mod api;
pub mod browser_auth;
pub mod config;
pub mod download;
pub mod i18n;
pub mod installs;
pub mod lua;
pub mod payload;
pub mod store;
pub mod sync;
pub mod updater;

use std::sync::{Arc, Mutex};
use std::time::Duration;

use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Listener, Manager, State, WindowEvent};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};
use tauri_plugin_opener::OpenerExt;
use tokio::sync::oneshot;

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
    signed_in(&app, &engine, &token, user)
}

/// The browser sign-in waiting for the player; a new one or Cancel stops it.
#[derive(Default)]
struct BrowserSignIn(Mutex<Option<oneshot::Sender<()>>>);

#[tauri::command]
async fn sign_in_with_browser(app: AppHandle, engine: Shared<'_>, pending: State<'_, BrowserSignIn>) -> Result<User, String> {
    let (cancel, cancelled) = oneshot::channel();
    if let Some(previous) = pending.0.lock().unwrap().replace(cancel) {
        let _ = previous.send(());
    }

    let callback = browser_auth::Callback::bind().await?;
    let pkce = browser_auth::Pkce::new();
    let state = browser_auth::random_token();
    let device = sync::device_name();
    let url = browser_auth::connect_url(&pkce, &state, &callback.redirect_uri, &device);
    app.opener().open_url(url, None::<&str>).map_err(|e| i18n::tr("Could not open the browser: :error", &[("error", &e.to_string())]))?;

    let code = tokio::select! {
        result = tokio::time::timeout(browser_auth::WAIT, callback.code(&state)) => {
            result.map_err(|_| i18n::t("The sign in took too long. Try again."))??
        }
        _ = cancelled => return Err(i18n::t("Sign in was cancelled.")),
    };
    show_window(&app);
    let (token, user) = engine.api().exchange(&code, &pkce.verifier, &callback.redirect_uri, &device).await?;
    signed_in(&app, &engine, &token, user)
}

#[tauri::command]
fn cancel_browser_sign_in(pending: State<'_, BrowserSignIn>) {
    if let Some(cancel) = pending.0.lock().unwrap().take() {
        let _ = cancel.send(());
    }
}

fn signed_in(app: &AppHandle, engine: &Engine, token: &str, user: User) -> Result<User, String> {
    store::set_token(token)?;
    engine.signed_in(user.clone());
    engine.request(Duration::from_secs(1));
    refresh_tray(app, engine);
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

/// The window's language for the "auto" setting ("zh-CN" from the system).
#[tauri::command]
fn set_system_language(app: AppHandle, engine: Shared<'_>, language: String) {
    if engine.set_system_language(&language) {
        let _ = app.emit("status-changed", ());
    }
}

#[tauri::command]
fn get_settings(engine: Shared<'_>) -> Settings {
    engine.settings()
}

#[tauri::command]
fn save_settings(app: AppHandle, engine: Shared<'_>, settings: Settings) -> Result<(), String> {
    let autostart = app.autolaunch();
    let result = if settings.start_with_system { autostart.enable() } else { autostart.disable() };
    result.map_err(|e| i18n::tr("Could not change the start with the system: :error", &[("error", &e.to_string())]))?;
    engine.save_settings(settings)?;
    // The language may have changed: the tray and the window follow
    let _ = app.emit("status-changed", ());
    Ok(())
}

/// "Start with Windows" is on by default: a fresh install registers it on the first run.
/// Only the production build, so local and dev builds never start with the system.
fn apply_start_with_system(app: &AppHandle, engine: &Engine) {
    if !config::is_production() {
        return;
    }
    let autostart = app.autolaunch();
    let wanted = engine.settings().start_with_system;
    if autostart.is_enabled().unwrap_or(!wanted) != wanted {
        let _ = if wanted { autostart.enable() } else { autostart.disable() };
    }
}

#[tauri::command]
async fn recent_uploads(engine: Shared<'_>) -> Result<Vec<UploadInfo>, String> {
    let token = store::token().ok_or_else(|| i18n::t("Sign in first."))?;
    engine.api().recent_uploads(&token).await
}

#[tauri::command]
async fn check_update(app: AppHandle) -> Result<Option<updater::UpdateInfo>, String> {
    updater::check(&app).await
}

#[tauri::command]
async fn install_update(app: AppHandle) -> Result<(), String> {
    updater::install(&app).await
}

fn show_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

/// The language of the tray menu, so it is built again when the language changes.
#[derive(Default)]
struct TrayLanguage(Mutex<&'static str>);

fn refresh_tray(app: &AppHandle, engine: &Engine) {
    if let Some(tray) = app.tray_by_id("main") {
        let status = engine.status();
        let text = match (&status.user, status.syncing) {
            (None, _) => i18n::t("HeadHunter Sync: signed out"),
            (Some(_), true) => i18n::t("HeadHunter Sync: syncing..."),
            (Some(user), false) => i18n::tr("HeadHunter Sync: :name", &[("name", &user.name)]),
        };
        let _ = tray.set_tooltip(Some(text));

        let tray_language = app.state::<TrayLanguage>();
        let mut language = tray_language.0.lock().unwrap();
        if *language != i18n::current() {
            if let Ok(menu) = tray_menu(app) {
                let _ = tray.set_menu(Some(menu));
                *language = i18n::current();
            }
        }
    }
}

fn tray_menu(app: &AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let open = MenuItem::with_id(app, "open", i18n::t("Open HeadHunter Sync"), true, None::<&str>)?;
    let sync = MenuItem::with_id(app, "sync", i18n::t("Sync now"), true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", i18n::t("Quit"), true, None::<&str>)?;
    Menu::with_items(app, &[&open, &sync, &quit])
}

fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    let menu = tray_menu(app)?;
    *app.state::<TrayLanguage>().0.lock().unwrap() = i18n::current();

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
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, Some(vec!["--minimized"])))
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            let engine = Engine::new(data_dir);
            app.manage(Arc::clone(&engine));
            app.manage(BrowserSignIn::default());
            app.manage(updater::Pending::default());
            app.manage(TrayLanguage::default());
            build_tray(app.handle())?;
            apply_start_with_system(app.handle(), &engine);
            engine.rewatch();
            engine.start_scheduler(app.handle().clone());
            updater::start_checks(app.handle().clone());

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
            sign_in_with_browser,
            cancel_browser_sign_in,
            sign_out,
            sync_now,
            get_settings,
            save_settings,
            set_system_language,
            recent_uploads,
            check_update,
            install_update
        ])
        .run(tauri::generate_context!())
        .expect("error while running HeadHunter Sync");
}
