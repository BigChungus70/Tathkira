mod config;

use rand::Rng;
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::{Duration, Instant};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, TrayIconEvent};
use tauri::{Emitter, Manager, PhysicalPosition, State, WebviewWindowBuilder, WindowEvent};

use config::{load_config, Config};

const NOTIFICATION_WIDTH: u32 = 350;
const NOTIFICATION_HEIGHT: u32 = 80;

#[derive(Clone)]
struct AppState {
    config: Arc<RwLock<Config>>,
    snoozed_until: Arc<RwLock<Option<Instant>>>,
    dismiss_requested: Arc<AtomicBool>,
}

fn open_configured_window<R: tauri::Runtime>(app: &tauri::AppHandle<R>, label: &str) {
    if let Some(window) = app.get_webview_window(label) {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        let _ = app.emit("window-shown", label.to_string());
        return;
    }

    let window_config = app
        .config()
        .app
        .windows
        .iter()
        .find(|window| window.label == label)
        .cloned();

    if let Some(window_config) = window_config {
        if let Ok(window) = WebviewWindowBuilder::from_config(app, &window_config).and_then(|builder| builder.build()) {
            let _ = window.unminimize();
            let _ = window.show();
            let _ = window.set_focus();
            let _ = app.emit("window-shown", label.to_string());
        }
    }
}

fn notification_origin(
    position: &str,
    monitor: &tauri::Monitor,
    width: i32,
    height: i32,
) -> (i32, i32) {
    let size = monitor.size();
    let base = monitor.position();
    let margin = 24_i32;

    let x = match position {
        "top_left" | "middle_left" | "bottom_left" => base.x + margin,
        "top_center" | "middle_center" | "bottom_center" => {
            base.x + (size.width as i32 - width) / 2
        }
        _ => base.x + size.width as i32 - width - margin,
    };
    let y = match position {
        "top_left" | "top_center" | "top_right" => base.y + margin,
        "middle_left" | "middle_center" | "middle_right" => {
            base.y + (size.height as i32 - height) / 2
        }
        _ => base.y + size.height as i32 - height - margin,
    };

    let min_x = base.x + margin;
    let min_y = base.y + margin;
    let max_x = (base.x + size.width as i32 - width - margin).max(min_x);
    let max_y = (base.y + size.height as i32 - height - margin).max(min_y);

    (x.max(min_x).min(max_x), y.max(min_y).min(max_y))
}

fn hide_notification_window<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    if let Some(window) = app.get_webview_window("notification") {
        let _ = window.hide();
    }
}

fn hide_main_and_pool<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    for label in ["main", "pool"] {
        if let Some(window) = app.get_webview_window(label) {
            let _ = window.hide();
        }
    }
}

fn show_notification_window<R: tauri::Runtime>(app: &tauri::AppHandle<R>, state: &AppState) {
    let config = match state.config.read() {
        Ok(config) => config,
        Err(_) => return,
    };
    if config.texts.is_empty() {
        return;
    }

    let index = rand::thread_rng().gen_range(0..config.texts.len());
    let text = config.texts[index].clone();
    let duration_ms = config.display_seconds * 1000;
    let language = config.language.clone();
    let position = config.position.clone();
    drop(config);

    let window = match app.get_webview_window("notification") {
        Some(window) => window,
        None => return,
    };

    let _ = window.set_focusable(false);

    if let Ok(Some(monitor)) = window.current_monitor() {
        let (x, y) = notification_origin(
            &position,
            &monitor,
            NOTIFICATION_WIDTH as i32,
            NOTIFICATION_HEIGHT as i32,
        );
        let _ = window.set_position(PhysicalPosition::new(x, y));
    }

    let _ = app.emit(
        "show-notification",
        json!({ "text": text, "duration_ms": duration_ms, "language": language }),
    );
    let _ = window.show();
}

fn start_timer<R: tauri::Runtime>(app: tauri::AppHandle<R>, state: AppState) {
    thread::spawn(move || {
        let mut next_due = Instant::now();
        let mut hide_at: Option<Instant> = None;
        loop {
            thread::sleep(Duration::from_millis(100));
            let now = Instant::now();

            {
                let mut snoozed = state.snoozed_until.write().unwrap();
                if let Some(until) = *snoozed {
                    if now < until {
                        if hide_at.take().is_some() {
                            hide_notification_window(&app);
                            state.dismiss_requested.store(false, Ordering::SeqCst);
                        }
                        continue;
                    }
                    *snoozed = None;
                }
            }

            if let Some(until) = hide_at {
                if state.dismiss_requested.load(Ordering::SeqCst) || now >= until {
                    hide_notification_window(&app);
                    hide_at = None;
                    state.dismiss_requested.store(false, Ordering::SeqCst);
                }
            }

            if now < next_due {
                continue;
            }

            let (interval, display) = {
                let config = state.config.read().unwrap();
                (
                    Duration::from_secs(config.interval_minutes),
                    Duration::from_secs(config.display_seconds),
                )
            };
            next_due = now + interval;
            state.dismiss_requested.store(false, Ordering::SeqCst);

            if hide_at.take().is_some() {
                hide_notification_window(&app);
            }

            show_notification_window(&app, &state);
            hide_at = Some(now + display);
        }
    });
}

fn asset_json<R: tauri::Runtime>(app: &tauri::AppHandle<R>, path: &str) -> Value {
    app.asset_resolver()
        .get(path.to_string())
        .and_then(|asset| serde_json::from_slice(&asset.bytes).ok())
        .unwrap_or_default()
}

fn locale_json<R: tauri::Runtime>(app: &tauri::AppHandle<R>, language: &str) -> Value {
    let manifest = asset_json(app, "locales/index.json");
    let file = manifest
        .get("languages")
        .and_then(|value| value.as_array())
        .and_then(|languages| {
            languages.iter().find(|entry| {
                entry.get("code").and_then(|code| code.as_str()) == Some(language)
            })
        })
        .and_then(|entry| entry.get("file").and_then(|file| file.as_str()))
        .unwrap_or("en.json");
    asset_json(app, &format!("locales/{}", file))
}

fn locale_text(locale: &Value, key: &str) -> String {
    let mut value = locale;
    for part in key.split('.') {
        match value.get(part) {
            Some(next) => value = next,
            None => return key.to_string(),
        }
    }
    value.as_str().unwrap_or(key).to_string()
}

fn build_tray_menu<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    language: &str,
) -> tauri::Result<Menu<R>> {
    let locale = locale_json(app, language);
    let settings_item = MenuItem::with_id(
        app,
        "settings",
        locale_text(&locale, "tray.settings"),
        true,
        None::<&str>,
    )?;
    let pool_item = MenuItem::with_id(
        app,
        "pool",
        locale_text(&locale, "actions.managePool"),
        true,
        None::<&str>,
    )?;
    let snooze_15_item = MenuItem::with_id(
        app,
        "snooze_15",
        locale_text(&locale, "tray.snooze15"),
        true,
        None::<&str>,
    )?;
    let snooze_60_item = MenuItem::with_id(
        app,
        "snooze_60",
        locale_text(&locale, "tray.snooze60"),
        true,
        None::<&str>,
    )?;
    let quit_item = MenuItem::with_id(
        app,
        "quit",
        locale_text(&locale, "actions.quit"),
        true,
        None::<&str>,
    )?;

    Menu::with_items(
        app,
        &[
            &settings_item,
            &pool_item,
            &snooze_15_item,
            &snooze_60_item,
            &quit_item,
        ],
    )
}

fn refresh_tray<R: tauri::Runtime>(app: &tauri::AppHandle<R>, language: &str) {
    if let Some(tray) = app.tray_by_id("main-tray") {
        if let Ok(menu) = build_tray_menu(app, language) {
            let _ = tray.set_menu(Some(menu));
        }
    }
}

fn setup_tray<R: tauri::Runtime>(app: &tauri::AppHandle<R>, language: &str) -> tauri::Result<()> {
    let menu = build_tray_menu(app, language)?;
    let icon = app.default_window_icon().cloned().unwrap();

    tauri::tray::TrayIconBuilder::with_id("main-tray")
        .icon(icon)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::DoubleClick {
                button: MouseButton::Left,
                ..
            } = event
            {
                open_configured_window(tray.app_handle(), "main");
            }
        })
        .on_menu_event(move |app, event| match event.id().as_ref() {
            "settings" => open_configured_window(app, "main"),
            "pool" => open_configured_window(app, "pool"),
            "snooze_15" => {
                let state = app.state::<AppState>();
                let mut snoozed = state.snoozed_until.write().unwrap();
                *snoozed = Some(Instant::now() + Duration::from_secs(15 * 60));
            }
            "snooze_60" => {
                let state = app.state::<AppState>();
                let mut snoozed = state.snoozed_until.write().unwrap();
                *snoozed = Some(Instant::now() + Duration::from_secs(60 * 60));
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;

    Ok(())
}

#[tauri::command]
fn get_config(state: State<'_, AppState>) -> Result<Config, String> {
    let config = state.config.read().map_err(|_| "lock error".to_string())?;
    Ok(config.clone())
}

#[tauri::command]
fn save_config<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    state: State<'_, AppState>,
    config: Config,
) -> Result<(), String> {
    let language = {
        let mut current = state.config.write().map_err(|_| "lock error".to_string())?;
        *current = config;
        current.clamp();
        config::save_config(&app, &current).map_err(|error| error.to_string())?;
        current.language.clone()
    };
    refresh_tray(&app, &language);
    app.emit("pool-changed", ()).map_err(|error| error.to_string())?;
    Ok(())
}

#[tauri::command]
fn snooze(state: State<'_, AppState>, minutes: u64) -> Result<(), String> {
    let minutes = minutes.clamp(1, 1440);
    let mut snoozed = state
        .snoozed_until
        .write()
        .map_err(|_| "lock error".to_string())?;
    *snoozed = Some(Instant::now() + Duration::from_secs(minutes * 60));
    Ok(())
}

#[tauri::command]
fn cancel_snooze(state: State<'_, AppState>) -> Result<(), String> {
    let mut snoozed = state
        .snoozed_until
        .write()
        .map_err(|_| "lock error".to_string())?;
    *snoozed = None;
    Ok(())
}

#[tauri::command]
fn get_snooze(state: State<'_, AppState>) -> Result<Option<u64>, String> {
    let snoozed = state
        .snoozed_until
        .read()
        .map_err(|_| "lock error".to_string())?;
    Ok(snoozed.map(|until| {
        let now = Instant::now();
        if now >= until {
            0
        } else {
            until.duration_since(now).as_secs()
        }
    }))
}

#[tauri::command]
fn hide_notification(state: State<'_, AppState>) {
    state.dismiss_requested.store(true, Ordering::SeqCst);
}

#[tauri::command]
fn reposition_notification<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    state: State<'_, AppState>,
    width: u32,
    height: u32,
) {
    let position = match state.config.read() {
        Ok(config) => config.position.clone(),
        Err(_) => return,
    };
    if let Some(window) = app.get_webview_window("notification") {
        if let Ok(Some(monitor)) = window.current_monitor() {
            let (x, y) = notification_origin(&position, &monitor, width as i32, height as i32);
            let _ = window.set_position(PhysicalPosition::new(x, y));
        }
    }
}

#[tauri::command]
fn show_settings<R: tauri::Runtime>(app: tauri::AppHandle<R>) {
    open_configured_window(&app, "main");
}

#[tauri::command]
fn show_pool<R: tauri::Runtime>(app: tauri::AppHandle<R>) {
    let main_visible = app
        .get_webview_window("main")
        .and_then(|window| window.is_visible().ok())
        .unwrap_or(false);
    if !main_visible {
        open_configured_window(&app, "main");
        return;
    }
    open_configured_window(&app, "pool");
}

#[tauri::command]
fn hide_main<R: tauri::Runtime>(app: tauri::AppHandle<R>) {
    hide_main_and_pool(&app);
}

#[tauri::command]
fn quit_app<R: tauri::Runtime>(app: tauri::AppHandle<R>) {
    app.exit(0);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .on_window_event(|window, event| {
            if matches!(window.label(), "main" | "pool") {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    if window.label() == "main" {
                        hide_main_and_pool(window.app_handle());
                    } else {
                        let _ = window.hide();
                    }
                }
            }
        })
        .setup(|app| {
            let config = load_config(app.handle())?;
            let language = config.language.clone();
            let state = AppState {
                config: Arc::new(RwLock::new(config)),
                snoozed_until: Arc::new(RwLock::new(None)),
                dismiss_requested: Arc::new(AtomicBool::new(false)),
            };
            app.manage(state.clone());

            setup_tray(app.handle(), &language)?;
            start_timer(app.handle().clone(), state);

            let _ = app.get_webview_window("main").map(|window| window.hide());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_config,
            save_config,
            snooze,
            cancel_snooze,
            get_snooze,
            hide_notification,
            reposition_notification,
            show_settings,
            show_pool,
            hide_main,
            quit_app
        ])
        .run(tauri::generate_context!())
        .expect("error while running Tathkira");
}
