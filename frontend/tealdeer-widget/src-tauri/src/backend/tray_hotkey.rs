use std::{fs, path::PathBuf};

use log::{info, warn};
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, Runtime,
};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

use crate::backend::{
    settings::{ensure_sidecar_config, read_app_settings, AppSettings},
    tealdeer,
};

const EVENT_NAVIGATE: &str = "tray-navigate";
const MENU_SHOW: &str = "tray_show";
const MENU_SEARCH: &str = "tray_search";
const MENU_NEW: &str = "tray_new";
const MENU_UPDATE: &str = "tray_update";
const MENU_OPEN_DIR: &str = "tray_open_dir";
const MENU_SETTINGS: &str = "tray_settings";
const MENU_QUIT: &str = "tray_quit";

pub fn setup<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let settings = match read_app_settings(app) {
        Ok(settings) => settings,
        Err(err) => {
            warn!("Failed to read app settings: {err}");
            AppSettings::default()
        }
    };

    if let Err(err) = apply_window_settings(app, &settings) {
        warn!("Failed to apply window settings: {err}");
    }
    if let Err(err) = register_hotkey(app, &settings.hotkey_toggle) {
        warn!("Failed to register global hotkey: {err}");
    }

    setup_tray(app)
}

pub fn apply_app_settings<R: Runtime>(
    app: &AppHandle<R>,
    previous: &AppSettings,
    next: &AppSettings,
) -> Result<(), String> {
    if previous.always_on_top != next.always_on_top {
        apply_window_settings(app, next)?;
    }
    if previous.hotkey_toggle != next.hotkey_toggle {
        register_hotkey(app, &next.hotkey_toggle)?;
    }
    Ok(())
}

fn setup_tray<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let menu = build_tray_menu(app)?;

    let mut builder = TrayIconBuilder::new()
        .menu(&menu)
        .on_menu_event(|app, event| {
            if let Err(err) = handle_menu_event(app, &event) {
                warn!("Tray menu error: {err}");
            }
        })
        .on_tray_icon_event(|tray, event| {
            if matches!(
                event,
                TrayIconEvent::Click { .. } | TrayIconEvent::DoubleClick { .. }
            ) {
                let _ = toggle_main_window(tray.app_handle());
            }
        });

    if let Some(icon) = app.default_window_icon().cloned() {
        builder = builder.icon(icon);
    }

    builder
        .build(app)
        .map_err(|e| format!("Failed to create tray icon: {e}"))?;
    info!("Tray icon initialized");
    Ok(())
}

fn build_tray_menu<R: Runtime>(app: &AppHandle<R>) -> Result<Menu<R>, String> {
    let menu = Menu::new(app).map_err(|e| format!("Failed to create tray menu: {e}"))?;

    let show = MenuItem::with_id(app, MENU_SHOW, "Show/Hide", true, None::<&str>)
        .map_err(|e| format!("Failed to create menu item: {e}"))?;
    let search = MenuItem::with_id(app, MENU_SEARCH, "Search", true, None::<&str>)
        .map_err(|e| format!("Failed to create menu item: {e}"))?;
    let new_page = MenuItem::with_id(app, MENU_NEW, "New Page", true, None::<&str>)
        .map_err(|e| format!("Failed to create menu item: {e}"))?;
    let update = MenuItem::with_id(app, MENU_UPDATE, "Update Cache", true, None::<&str>)
        .map_err(|e| format!("Failed to create menu item: {e}"))?;
    let open_dir = MenuItem::with_id(app, MENU_OPEN_DIR, "Open Custom Dir", true, None::<&str>)
        .map_err(|e| format!("Failed to create menu item: {e}"))?;
    let settings = MenuItem::with_id(app, MENU_SETTINGS, "Settings", true, None::<&str>)
        .map_err(|e| format!("Failed to create menu item: {e}"))?;
    let quit = MenuItem::with_id(app, MENU_QUIT, "Quit", true, None::<&str>)
        .map_err(|e| format!("Failed to create menu item: {e}"))?;

    menu.append(&show)
        .and_then(|_| menu.append(&search))
        .and_then(|_| menu.append(&new_page))
        .and_then(|_| menu.append(&update))
        .and_then(|_| menu.append(&open_dir))
        .and_then(|_| menu.append(&PredefinedMenuItem::separator(app)?))
        .and_then(|_| menu.append(&settings))
        .and_then(|_| menu.append(&quit))
        .map_err(|e| format!("Failed to build tray menu: {e}"))?;

    Ok(menu)
}

fn handle_menu_event<R: Runtime>(
    app: &AppHandle<R>,
    event: &tauri::menu::MenuEvent,
) -> Result<(), String> {
    match event.id().as_ref() {
        MENU_SHOW => toggle_main_window(app),
        MENU_SEARCH => show_and_navigate(app, "search"),
        MENU_NEW => show_and_navigate(app, "new"),
        MENU_UPDATE => {
            let app = app.clone();
            tauri::async_runtime::spawn_blocking(move || {
                if let Err(err) = tealdeer::update_cache_internal(&app) {
                    warn!("Cache update failed: {err}");
                }
            });
            Ok(())
        }
        MENU_OPEN_DIR => open_custom_dir(app),
        MENU_SETTINGS => show_and_navigate(app, "settings"),
        MENU_QUIT => {
            app.exit(0);
            Ok(())
        }
        _ => Ok(()),
    }
}

fn show_and_navigate<R: Runtime>(app: &AppHandle<R>, tab: &str) -> Result<(), String> {
    show_main_window(app)?;
    app.emit(EVENT_NAVIGATE, tab)
        .map_err(|e| format!("Failed to emit navigation event: {e}"))?;
    Ok(())
}

fn open_custom_dir<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let dir = resolve_custom_pages_dir(app)?;
    fs::create_dir_all(&dir).map_err(|e| format!("Failed to create custom dir: {e}"))?;
    tauri_plugin_opener::open_path(&dir, None::<&str>)
        .map_err(|e| format!("Failed to open custom dir: {e}"))
}

fn resolve_custom_pages_dir<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, String> {
    if let Ok(paths) = tealdeer::get_show_paths_internal(app) {
        if let Some(dir) = paths.custom_pages_dir {
            return Ok(PathBuf::from(dir));
        }
    }
    let config_dir = ensure_sidecar_config(app)?;
    Ok(config_dir.join("pages"))
}

fn apply_window_settings<R: Runtime>(
    app: &AppHandle<R>,
    settings: &AppSettings,
) -> Result<(), String> {
    let window = main_window(app)?;
    window
        .set_always_on_top(settings.always_on_top)
        .map_err(|e| format!("Failed to set always-on-top: {e}"))?;
    Ok(())
}

fn register_hotkey<R: Runtime>(app: &AppHandle<R>, hotkey: &str) -> Result<(), String> {
    let shortcut = hotkey.trim();
    let global = app.global_shortcut();
    let _ = global.unregister_all();
    if shortcut.is_empty() {
        return Ok(());
    }
    global
        .on_shortcut(shortcut, |app, _shortcut, event| {
            if event.state == ShortcutState::Pressed {
                let _ = toggle_main_window(app);
            }
        })
        .map_err(|e| format!("Failed to register hotkey: {e}"))?;
    Ok(())
}

fn toggle_main_window<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let window = main_window(app)?;
    let visible = window
        .is_visible()
        .map_err(|e| format!("Failed to read window visibility: {e}"))?;
    if visible {
        window
            .hide()
            .map_err(|e| format!("Failed to hide window: {e}"))?;
    } else {
        show_main_window(app)?;
    }
    Ok(())
}

fn show_main_window<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let window = main_window(app)?;
    window
        .show()
        .map_err(|e| format!("Failed to show window: {e}"))?;
    let _ = window.unminimize();
    let _ = window.set_focus();
    Ok(())
}

fn main_window<R: Runtime>(app: &AppHandle<R>) -> Result<tauri::WebviewWindow<R>, String> {
    app.get_webview_window("main")
        .ok_or_else(|| "Main window not available.".to_string())
}
