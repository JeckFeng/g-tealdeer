mod backend;

use log::LevelFilter;
use tauri_plugin_log::{Target, TargetKind, WEBVIEW_TARGET};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let log_level = if cfg!(debug_assertions) {
        LevelFilter::Debug
    } else {
        LevelFilter::Info
    };
    let mut log_targets = Vec::new();
    log_targets.push(
        Target::new(TargetKind::LogDir {
            file_name: Some("rust".into()),
        })
        .filter(|metadata| !metadata.target().starts_with(WEBVIEW_TARGET)),
    );
    log_targets.push(
        Target::new(TargetKind::LogDir {
            file_name: Some("webview".into()),
        })
        .filter(|metadata| metadata.target().starts_with(WEBVIEW_TARGET)),
    );
    if cfg!(debug_assertions) {
        log_targets.push(Target::new(TargetKind::Stdout));
    }

    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log_level)
                .targets(log_targets)
                .build(),
        )
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            backend::tealdeer::get_show_paths,
            backend::tealdeer::render_tldr,
            backend::tealdeer::preview_effective_output,
            backend::tealdeer::update_cache,
            backend::custom_pages::create_or_overwrite_page,
            backend::custom_pages::create_or_overwrite_patch,
            backend::custom_pages::append_example_to_page,
            backend::custom_pages::scan_custom_pages,
            backend::custom_pages::delete_custom_file,
            backend::custom_pages::disable_custom_file,
            backend::custom_pages::enable_custom_file,
            backend::custom_pages::read_custom_file,
            backend::settings::get_app_settings,
            backend::settings::set_app_settings,
            backend::settings::get_tealdeer_config,
            backend::settings::get_tealdeer_config_values,
            backend::settings::set_tealdeer_config,
            backend::open_paths::open_custom_pages_dir,
            backend::open_paths::open_config_file,
            backend::open_paths::open_log_directory,
            backend::open_paths::open_custom_page_file
        ])
        .setup(|app| {
            if let Err(err) = backend::tray_hotkey::setup(app.handle()) {
                eprintln!("Tray/hotkey setup failed: {err}");
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
