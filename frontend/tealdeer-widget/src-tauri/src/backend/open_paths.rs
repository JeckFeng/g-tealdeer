use log::info;
use tauri::{AppHandle, Runtime};
use tauri_plugin_opener;

use crate::backend::tealdeer::get_show_paths_internal;

/// 打开自定义页面目录
#[tauri::command]
pub fn open_custom_pages_dir<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    info!("Opening custom pages directory");
    
    let paths = get_show_paths_internal(&app)?;
    
    if let Some(dir) = paths.custom_pages_dir {
        tauri_plugin_opener::open_path(&dir, None::<&str>)
            .map_err(|e| format!("Failed to open directory: {}", e))?;
        
        info!("Opened custom pages directory: {}", dir);
        Ok(())
    } else {
        Err("Custom pages directory is not available".to_string())
    }
}

/// 打开配置文件
#[tauri::command]
pub fn open_config_file<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    info!("Opening config file");
    
    let paths = get_show_paths_internal(&app)?;
    
    if let Some(path) = paths.config_path {
        tauri_plugin_opener::open_path(&path, None::<&str>)
            .map_err(|e| format!("Failed to open config file: {}", e))?;
        
        info!("Opened config file: {}", path);
        Ok(())
    } else {
        Err("Config file is not available".to_string())
    }
}

/// 打开日志目录
#[tauri::command]
pub fn open_log_directory(log_dir: String) -> Result<(), String> {
    info!("Opening log directory: {}", log_dir);
    
    tauri_plugin_opener::open_path(&log_dir, None::<&str>)
        .map_err(|e| format!("Failed to open log directory: {}", e))?;
    
    Ok(())
}

/// 打开自定义页面文件
#[tauri::command]
pub fn open_custom_page_file<R: Runtime>(
    app: AppHandle<R>,
    file_path: String
) -> Result<(), String> {
    info!("Opening custom page file: {}", file_path);
    
    // 验证路径安全性
    let paths = get_show_paths_internal(&app)?;
    if let Some(dir) = paths.custom_pages_dir {
        let file = std::path::Path::new(&file_path);
        let base = std::path::Path::new(&dir);
        
        // 确保文件在自定义页面目录内
        if !file.starts_with(base) {
            return Err("Access denied: file is outside custom pages directory".to_string());
        }
    }
    
    tauri_plugin_opener::open_path(&file_path, None::<&str>)
        .map_err(|e| format!("Failed to open file: {}", e))?;
    
    Ok(())
}

/// 打开快捷键页面目录
#[tauri::command]
pub fn open_shortcut_pages_dir<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    info!("Opening shortcut pages directory");
    
    let paths = get_show_paths_internal(&app)?;
    
    if let Some(dir) = paths.shortcut_pages_dir {
        tauri_plugin_opener::open_path(&dir, None::<&str>)
            .map_err(|e| format!("Failed to open directory: {}", e))?;
        
        info!("Opened shortcut pages directory: {}", dir);
        Ok(())
    } else {
        Err("Shortcut pages directory is not available".to_string())
    }
}

/// 打开快捷键页面文件
#[tauri::command]
pub fn open_shortcut_page_file<R: Runtime>(
    app: AppHandle<R>,
    file_path: String
) -> Result<(), String> {
    info!("Opening shortcut page file: {}", file_path);
    
    // 验证路径安全性
    let paths = get_show_paths_internal(&app)?;
    if let Some(dir) = paths.shortcut_pages_dir {
        let file = std::path::Path::new(&file_path);
        let base = std::path::Path::new(&dir);
        
        // 确保文件在快捷键页面目录内
        if !file.starts_with(base) {
            return Err("Access denied: file is outside shortcut pages directory".to_string());
        }
    }
    
    tauri_plugin_opener::open_path(&file_path, None::<&str>)
        .map_err(|e| format!("Failed to open file: {}", e))?;
    
    Ok(())
}
