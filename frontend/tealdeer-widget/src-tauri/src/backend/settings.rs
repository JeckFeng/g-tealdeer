use std::{
    env,
    fs,
    path::{Path, PathBuf},
};

use log::{debug, info};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, Runtime};
use toml::Value;

use crate::backend::tray_hotkey;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppSettings {
    pub color: String,
    pub hotkey_toggle: String,
    pub always_on_top: bool,
    pub theme: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            color: "auto".to_string(),
            hotkey_toggle: "Ctrl+Alt+T".to_string(),
            always_on_top: true,
            theme: "light".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TealdeerConfigValues {
    pub languages: Vec<String>,
    pub platforms: Vec<String>,
    pub auto_update: bool,
    pub auto_update_interval_hours: Option<u64>,
    pub use_pager: bool,
    pub archive_source: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TealdeerConfigPatch {
    pub languages: Option<Vec<String>>,
    pub platforms: Option<Vec<String>>,
    pub auto_update: Option<bool>,
    pub auto_update_interval_hours: Option<u64>,
    pub use_pager: Option<bool>,
    pub archive_source: Option<String>,
}

#[tauri::command]
pub fn get_app_settings<R: tauri::Runtime>(app: AppHandle<R>) -> Result<AppSettings, String> {
    read_app_settings(&app)
}

#[tauri::command]
pub fn set_app_settings<R: tauri::Runtime>(app: AppHandle<R>, settings: AppSettings) -> Result<(), String> {
    let current = read_app_settings(&app)?;
    tray_hotkey::apply_app_settings(&app, &current, &settings)?;
    write_app_settings(&app, &settings)
}

#[tauri::command]
pub fn get_tealdeer_config<R: tauri::Runtime>(app: AppHandle<R>) -> Result<String, String> {
    let path = ensure_app_config(&app)?;
    fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read config: {e}"))
}

#[tauri::command]
pub fn get_tealdeer_config_values<R: tauri::Runtime>(app: AppHandle<R>) -> Result<TealdeerConfigValues, String> {
    let path = ensure_app_config(&app)?;
    let value = read_toml_value(&path)?;
    Ok(TealdeerConfigValues {
        languages: read_string_array(&value, &["search", "languages"]),
        platforms: read_string_array(&value, &["search", "platforms"]),
        auto_update: read_bool(&value, &["updates", "auto_update"]).unwrap_or(false),
        auto_update_interval_hours: read_u64(&value, &["updates", "auto_update_interval_hours"]),
        use_pager: read_bool(&value, &["display", "use_pager"]).unwrap_or(false),
        archive_source: read_string(&value, &["updates", "archive_source"]),
    })
}

#[tauri::command]
pub fn set_tealdeer_config(
    app: AppHandle,
    patch: TealdeerConfigPatch,
) -> Result<(), String> {
    let path = ensure_app_config(&app)?;
    let mut value = read_toml_value(&path)?;
    ensure_app_directories(&app, &mut value)?;

    if let Some(languages) = patch.languages {
        set_string_array(&mut value, &["search", "languages"], languages);
    }
    if let Some(platforms) = patch.platforms {
        set_string_array(&mut value, &["search", "platforms"], platforms);
    }
    if let Some(auto_update) = patch.auto_update {
        set_bool(&mut value, &["updates", "auto_update"], auto_update);
    }
    if let Some(interval) = patch.auto_update_interval_hours {
        set_u64(&mut value, &["updates", "auto_update_interval_hours"], interval);
    }
    if let Some(use_pager) = patch.use_pager {
        set_bool(&mut value, &["display", "use_pager"], use_pager);
    }
    if let Some(archive_source) = patch.archive_source {
        set_string(&mut value, &["updates", "archive_source"], archive_source);
    }

    let result = write_toml_value(&path, &value);
    if result.is_ok() {
        info!("Updated tealdeer config: {}", path.display());
    }
    result
}

pub(crate) fn ensure_app_config<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, String> {
    let config_dir = app_data_dir(app)?;
    fs::create_dir_all(&config_dir)
        .map_err(|e| format!("Failed to create app config dir: {e}"))?;
    let config_path = config_dir.join("config.toml");

    let mut value = read_toml_value(&config_path)?;
    ensure_app_directories(app, &mut value)?;
    if read_bool(&value, &["display", "use_pager"]).is_none() {
        set_bool(&mut value, &["display", "use_pager"], false);
    }
    write_toml_value(&config_path, &value)?;
    env::set_var("TEALDEER_CONFIG_DIR", &config_dir);
    debug!("Ensured app config: {}", config_path.display());

    Ok(config_path)
}

pub(crate) fn app_data_dir<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map_err(|_| "Failed to resolve app data dir.".to_string())
}

fn ensure_app_directories<R: Runtime>(
    app: &AppHandle<R>,
    value: &mut Value,
) -> Result<(), String> {
    let config_dir = app_data_dir(app)?;
    let cache_dir = config_dir.join("cache");
    let pages_dir = config_dir.join("pages");
    let shortcut_pages_dir = config_dir.join("shortcut_pages");

    fs::create_dir_all(&cache_dir)
        .map_err(|e| format!("Failed to create cache dir: {e}"))?;
    fs::create_dir_all(&pages_dir)
        .map_err(|e| format!("Failed to create custom pages dir: {e}"))?;
    fs::create_dir_all(&shortcut_pages_dir)
        .map_err(|e| format!("Failed to create shortcut pages dir: {e}"))?;

    set_string(
        value,
        &["directories", "cache_dir"],
        cache_dir.to_string_lossy().to_string(),
    );
    set_string(
        value,
        &["directories", "custom_pages_dir"],
        pages_dir.to_string_lossy().to_string(),
    );
    set_string(
        value,
        &["directories", "shortcut_pages_dir"],
        shortcut_pages_dir.to_string_lossy().to_string(),
    );
    debug!(
        "Ensured app directories: cache={}, pages={}, shortcut_pages={}",
        cache_dir.display(),
        pages_dir.display(),
        shortcut_pages_dir.display()
    );
    Ok(())
}

pub(crate) fn read_app_settings<R: Runtime>(app: &AppHandle<R>) -> Result<AppSettings, String> {
    let path = app_settings_path(app)?;
    if !path.exists() {
        return Ok(AppSettings::default());
    }
    let contents = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read app settings: {e}"))?;
    serde_json::from_str(&contents).map_err(|e| format!("Invalid app settings: {e}"))
}

pub(crate) fn write_app_settings<R: Runtime>(
    app: &AppHandle<R>,
    settings: &AppSettings,
) -> Result<(), String> {
    let path = app_settings_path(app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create settings dir: {e}"))?;
    }
    let payload =
        serde_json::to_string_pretty(settings).map_err(|e| format!("Failed to serialize: {e}"))?;
    fs::write(&path, payload).map_err(|e| format!("Failed to write settings: {e}"))?;
    info!("Saved app settings: {}", path.display());
    Ok(())
}

fn app_settings_path<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, String> {
    app.path()
        .app_config_dir()
        .map(|dir| dir.join("settings.json"))
        .map_err(|_| "Failed to resolve app config dir.".to_string())
}

fn read_toml_value(path: &Path) -> Result<Value, String> {
    if !path.exists() {
        return Ok(Value::Table(toml::value::Table::new()));
    }
    let contents =
        fs::read_to_string(path).map_err(|e| format!("Failed to read config: {e}"))?;
    if contents.trim().is_empty() {
        return Ok(Value::Table(toml::value::Table::new()));
    }
    contents
        .parse::<Value>()
        .map_err(|e| format!("Failed to parse config: {e}"))
}

fn write_toml_value(path: &Path, value: &Value) -> Result<(), String> {
    let serialized = toml::to_string_pretty(value)
        .map_err(|e| format!("Failed to serialize config: {e}"))?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create config dir: {e}"))?;
    }
    fs::write(path, serialized).map_err(|e| format!("Failed to write config: {e}"))
}

fn ensure_table<'a>(value: &'a mut Value, path: &[&str]) -> &'a mut toml::value::Table {
    let mut current = value;
    for key in path {
        if !current.is_table() {
            *current = Value::Table(toml::value::Table::new());
        }
        let table = current.as_table_mut().expect("table ensured");
        current = table.entry((*key).to_string()).or_insert_with(|| {
            Value::Table(toml::value::Table::new())
        });
    }
    current.as_table_mut().expect("table ensured")
}

fn set_string(value: &mut Value, path: &[&str], data: String) {
    let (parent, key) = path.split_at(path.len() - 1);
    let table = ensure_table(value, parent);
    table.insert((*key.last().unwrap()).to_string(), Value::String(data));
}

fn set_bool(value: &mut Value, path: &[&str], data: bool) {
    let (parent, key) = path.split_at(path.len() - 1);
    let table = ensure_table(value, parent);
    table.insert((*key.last().unwrap()).to_string(), Value::Boolean(data));
}

fn set_u64(value: &mut Value, path: &[&str], data: u64) {
    let (parent, key) = path.split_at(path.len() - 1);
    let table = ensure_table(value, parent);
    table.insert(
        (*key.last().unwrap()).to_string(),
        Value::Integer(data as i64),
    );
}

fn set_string_array(value: &mut Value, path: &[&str], data: Vec<String>) {
    let (parent, key) = path.split_at(path.len() - 1);
    let table = ensure_table(value, parent);
    let items = data.into_iter().map(Value::String).collect();
    table.insert((*key.last().unwrap()).to_string(), Value::Array(items));
}

fn read_string(value: &Value, path: &[&str]) -> Option<String> {
    let node = find_value(value, path)?;
    node.as_str().map(|s| s.to_string())
}

fn read_bool(value: &Value, path: &[&str]) -> Option<bool> {
    let node = find_value(value, path)?;
    node.as_bool()
}

fn read_u64(value: &Value, path: &[&str]) -> Option<u64> {
    let node = find_value(value, path)?;
    node.as_integer().map(|num| num as u64)
}

fn read_string_array(value: &Value, path: &[&str]) -> Vec<String> {
    let node = match find_value(value, path) {
        Some(node) => node,
        None => return Vec::new(),
    };
    node.as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(|s| s.to_string()))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn find_value<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    let mut current = value;
    for key in path {
        current = current.as_table()?.get(*key)?;
    }
    Some(current)
}
