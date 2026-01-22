mod backend;

use log::LevelFilter;
use std::env;
use std::sync::OnceLock;
use tauri::{Manager, WindowEvent};
use tauri_plugin_log::{Target, TargetKind, WEBVIEW_TARGET};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let log_level = if cfg!(debug_assertions) {
        LevelFilter::Debug
    } else {
        LevelFilter::Info
    };
    setup_runtime_environment();
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .manage(backend::immersive_window::ImmersiveWindowManager::new())
        .invoke_handler(tauri::generate_handler![
            backend::tealdeer::get_show_paths,
            backend::tealdeer::render_tldr,
            backend::tealdeer::preview_effective_output,
            backend::tealdeer::update_cache,
            backend::tealdeer::search_pages,
            backend::tealdeer::list_commands,
            backend::custom_pages::create_or_overwrite_page,
            backend::custom_pages::create_or_overwrite_patch,
            backend::custom_pages::append_example_to_page,
            backend::custom_pages::scan_custom_pages,
            backend::custom_pages::delete_custom_file,
            backend::custom_pages::disable_custom_file,
            backend::custom_pages::enable_custom_file,
            backend::custom_pages::read_custom_file,
            backend::shortcut_pages::create_or_overwrite_shortcut_page,
            backend::shortcut_pages::create_or_overwrite_shortcut_patch,
            backend::shortcut_pages::append_example_to_shortcut_page,
            backend::shortcut_pages::scan_shortcut_pages,
            backend::shortcut_pages::delete_shortcut_file,
            backend::shortcut_pages::disable_shortcut_file,
            backend::shortcut_pages::enable_shortcut_file,
            backend::shortcut_pages::read_shortcut_file,
            backend::settings::get_app_settings,
            backend::settings::set_app_settings,
            backend::settings::get_log_dir,
            backend::settings::get_tealdeer_config,
            backend::settings::get_tealdeer_config_values,
            backend::settings::set_tealdeer_config,
            backend::open_paths::open_custom_pages_dir,
            backend::open_paths::open_config_file,
            backend::open_paths::open_log_directory,
            backend::open_paths::open_custom_page_file,
            backend::open_paths::open_shortcut_pages_dir,
            backend::open_paths::open_shortcut_page_file,
            backend::favorites::get_favorites,
            backend::favorites::add_favorite,
            backend::favorites::remove_favorite,
            backend::favorites::clear_favorites,
            backend::favorites::is_favorite,
            backend::immersive_window::open_immersive_window,
            backend::immersive_window::close_immersive_window,
            backend::immersive_window::get_immersive_window_state,
            backend::immersive_window::set_immersive_window_state
        ])
        .setup(move |app| {
            if let Err(err) = register_log_plugin(app.handle(), log_level) {
                eprintln!("Log plugin setup failed: {err}");
            }
            log_env_snapshot();
            log_runtime_info(app.handle());
            if let Err(err) = backend::tray_hotkey::setup(app.handle()) {
                eprintln!("Tray/hotkey setup failed: {err}");
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == "immersive" {
                if let WindowEvent::CloseRequested { .. } = event {
                    let app = window.app_handle();
                    if let Some(immersive_window) = app.get_webview_window("immersive") {
                        let manager =
                            app.state::<backend::immersive_window::ImmersiveWindowManager>();
                        let _ = manager.handle_window_close(app, &immersive_window);
                    }
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

pub fn setup_runtime_environment() {
    static ONCE: OnceLock<()> = OnceLock::new();
    if ONCE.set(()).is_err() {
        return;
    }
    apply_wayland_workaround();
}

fn apply_wayland_workaround() {
    if !cfg!(target_os = "linux") {
        return;
    }
    if env::var_os("TEALDEER_ALLOW_WAYLAND").is_some() {
        return;
    }
    let session = detect_session_backend();
    if !session.is_wayland {
        return;
    }
    let pre_gdk = env::var("GDK_BACKEND").ok();
    let pre_winit = env::var("WINIT_UNIX_BACKEND").ok();
    let pre_dmabuf = env::var("WEBKIT_DISABLE_DMABUF_RENDERER").ok();
    env::set_var("GDK_BACKEND", "x11");
    env::set_var("WINIT_UNIX_BACKEND", "x11");
    env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    let post_gdk = env::var("GDK_BACKEND").ok();
    let post_winit = env::var("WINIT_UNIX_BACKEND").ok();
    let post_dmabuf = env::var("WEBKIT_DISABLE_DMABUF_RENDERER").ok();
    eprintln!("[tealdeer_tile] Wayland detected; forcing X11 backend (set TEALDEER_ALLOW_WAYLAND=1 to opt out). Pre: GDK_BACKEND={pre_gdk:?}, WINIT_UNIX_BACKEND={pre_winit:?}, WEBKIT_DISABLE_DMABUF_RENDERER={pre_dmabuf:?}. Post: GDK_BACKEND={post_gdk:?}, WINIT_UNIX_BACKEND={post_winit:?}, WEBKIT_DISABLE_DMABUF_RENDERER={post_dmabuf:?}.");
}

fn log_env_snapshot() {
    let session = detect_session_backend();
    log::info!(
        "Session detect: backend={}, is_wayland={}, is_x11={}, is_xwayland={}, xdg_session_type={:?}, wayland_display={:?}, display={:?}",
        session.backend,
        session.is_wayland,
        session.is_x11,
        session.is_xwayland,
        session.xdg_session_type,
        session.wayland_display,
        session.display
    );
    log::info!(
        "Env snapshot: DISPLAY={:?}, GDK_BACKEND={:?}, WINIT_UNIX_BACKEND={:?}, WEBKIT_DISABLE_DMABUF_RENDERER={:?}, WEBKIT_DISABLE_COMPOSITING_MODE={:?}, WEBKIT_FORCE_SANDBOX={:?}, GTK_THEME={:?}, GTK_DEBUG={:?}, GTK_USE_PORTAL={:?}, XDG_RUNTIME_DIR={:?}, LIBGL_ALWAYS_SOFTWARE={:?}, MESA_LOADER_DRIVER_OVERRIDE={:?}, __GLX_VENDOR_LIBRARY_NAME={:?}, XDG_SESSION_TYPE={:?}, TEALDEER_ALLOW_WAYLAND={:?}",
        env::var("DISPLAY").ok(),
        env::var("GDK_BACKEND").ok(),
        env::var("WINIT_UNIX_BACKEND").ok(),
        env::var("WEBKIT_DISABLE_DMABUF_RENDERER").ok(),
        env::var("WEBKIT_DISABLE_COMPOSITING_MODE").ok(),
        env::var("WEBKIT_FORCE_SANDBOX").ok(),
        env::var("GTK_THEME").ok(),
        env::var("GTK_DEBUG").ok(),
        env::var("GTK_USE_PORTAL").ok(),
        env::var("XDG_RUNTIME_DIR").ok(),
        env::var("LIBGL_ALWAYS_SOFTWARE").ok(),
        env::var("MESA_LOADER_DRIVER_OVERRIDE").ok(),
        env::var("__GLX_VENDOR_LIBRARY_NAME").ok(),
        env::var("XDG_SESSION_TYPE").ok(),
        env::var("TEALDEER_ALLOW_WAYLAND").ok()
    );
}

fn log_runtime_info<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    let info = app.package_info();
    log::info!(
        "App info: name={}, version={}, identifier={}",
        info.name,
        info.version,
        app.config().identifier
    );
    if let Ok(exe) = env::current_exe() {
        log::info!("Process exe: {}", exe.display());
    }
    if let Ok(cwd) = env::current_dir() {
        log::info!("Process cwd: {}", cwd.display());
    }
    log::info!("Process id: {}", std::process::id());
    log::info!("Process args: {:?}", env::args().collect::<Vec<_>>());
    if let Ok(path) = app.path().app_data_dir() {
        log::info!("App data dir (tauri): {}", path.display());
    }
    if let Ok(path) = app.path().app_config_dir() {
        log::info!("App config dir (tauri): {}", path.display());
    }
    if let Ok(path) = app.path().app_log_dir() {
        log::info!("App log dir (tauri): {}", path.display());
    }
    if let Ok(path) = backend::settings::app_data_dir(app) {
        log::info!("App data dir (custom): {}", path.display());
    }
}

struct SessionBackend {
    backend: &'static str,
    is_wayland: bool,
    is_x11: bool,
    is_xwayland: bool,
    xdg_session_type: Option<String>,
    wayland_display: Option<String>,
    display: Option<String>,
}

fn detect_session_backend() -> SessionBackend {
    let xdg_session_type = env::var("XDG_SESSION_TYPE").ok();
    let wayland_display = env::var("WAYLAND_DISPLAY").ok();
    let display = env::var("DISPLAY").ok();

    let is_wayland = wayland_display.is_some()
        || xdg_session_type
            .as_deref()
            .map(|value| value.eq_ignore_ascii_case("wayland"))
            .unwrap_or(false);
    let is_x11 = display.is_some()
        || xdg_session_type
            .as_deref()
            .map(|value| value.eq_ignore_ascii_case("x11"))
            .unwrap_or(false);
    let is_xwayland = is_wayland && display.is_some();

    let backend = if is_xwayland {
        "xwayland"
    } else if is_wayland {
        "wayland"
    } else if is_x11 {
        "x11"
    } else {
        "unknown"
    };

    SessionBackend {
        backend,
        is_wayland,
        is_x11,
        is_xwayland,
        xdg_session_type,
        wayland_display,
        display,
    }
}

fn register_log_plugin<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    log_level: LevelFilter,
) -> Result<(), String> {
    let log_dir = backend::settings::app_data_dir(app)?.join("logs");
    let mut log_targets = Vec::new();
    log_targets.push(
        Target::new(TargetKind::Folder {
            path: log_dir.clone(),
            file_name: Some("rust".into()),
        })
        .filter(|metadata| !metadata.target().starts_with(WEBVIEW_TARGET)),
    );
    log_targets.push(
        Target::new(TargetKind::Folder {
            path: log_dir,
            file_name: Some("webview".into()),
        })
        .filter(|metadata| metadata.target().starts_with(WEBVIEW_TARGET)),
    );
    if cfg!(debug_assertions) {
        log_targets.push(Target::new(TargetKind::Stdout));
    }

    let plugin = tauri_plugin_log::Builder::new()
        .level(log_level)
        .targets(log_targets)
        .build();
    app.plugin(plugin)
        .map_err(|e| format!("Failed to register log plugin: {e}"))
}
