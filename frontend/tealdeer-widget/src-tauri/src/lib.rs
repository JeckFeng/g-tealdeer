mod backend;

use log::LevelFilter;
use tauri::{Manager, WindowEvent};
use tauri_plugin_log::{Target, TargetKind, WEBVIEW_TARGET};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let log_level = if cfg!(debug_assertions) {
        LevelFilter::Debug
    } else {
        LevelFilter::Info
    };
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
                        let manager = app.state::<backend::immersive_window::ImmersiveWindowManager>();
                        let _ = manager.handle_window_close(&app, &immersive_window);
                    }
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
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
