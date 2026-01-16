use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter, Manager};
use crate::backend::settings;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FavoriteEntry {
    pub command: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum FavoriteEntryRaw {
    String(String),
    Object {
        command: String,
        #[serde(default)]
        description: Option<String>,
    },
}

#[derive(Debug, Deserialize)]
struct FavoritesRaw {
    #[serde(default)]
    version: u32,
    #[serde(default)]
    updated_at: u64,
    #[serde(default)]
    items: HashMap<String, Vec<FavoriteEntryRaw>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(from = "FavoritesRaw")]
pub struct Favorites {
    pub version: u32,
    pub updated_at: u64,
    pub items: HashMap<String, Vec<FavoriteEntry>>,
}

impl Default for Favorites {
    fn default() -> Self {
        Self {
            version: 2,
            updated_at: current_timestamp(),
            items: HashMap::new(),
        }
    }
}

impl From<FavoriteEntryRaw> for FavoriteEntry {
    fn from(raw: FavoriteEntryRaw) -> Self {
        match raw {
            FavoriteEntryRaw::String(command) => Self {
                command,
                description: String::new(),
            },
            FavoriteEntryRaw::Object {
                command,
                description,
            } => Self {
                command,
                description: description.unwrap_or_default(),
            },
        }
    }
}

impl From<FavoritesRaw> for Favorites {
    fn from(raw: FavoritesRaw) -> Self {
        let version = if raw.version == 0 { 2 } else { raw.version };
        let updated_at = if raw.updated_at == 0 {
            current_timestamp()
        } else {
            raw.updated_at
        };
        let items = raw
            .items
            .into_iter()
            .map(|(page, entries)| {
                let converted = entries
                    .into_iter()
                    .map(FavoriteEntry::from)
                    .collect::<Vec<_>>();
                (page, converted)
            })
            .collect();
        Self {
            version,
            updated_at,
            items,
        }
    }
}

fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

fn get_favorites_path(app: &AppHandle) -> Result<PathBuf, String> {
    let app_data_dir = settings::app_data_dir(app)?;
    
    // 确保目录存在
    fs::create_dir_all(&app_data_dir)
        .map_err(|e| format!("Failed to create app data dir: {}", e))?;
    
    Ok(app_data_dir.join("favorites.json"))
}

fn normalize_command(cmd: &str) -> String {
    // 标准化：trim + 折叠多余空白
    cmd.trim()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn write_favorites(path: &Path, favorites: &Favorites) -> Result<(), String> {
    let content = serde_json::to_string_pretty(favorites)
        .map_err(|e| format!("Failed to serialize favorites: {}", e))?;
    let tmp_path = path.with_extension("json.tmp");
    fs::write(&tmp_path, &content)
        .map_err(|e| format!("Failed to write favorites: {}", e))?;
    if fs::rename(&tmp_path, path).is_err() {
        fs::write(path, &content)
            .map_err(|e| format!("Failed to write favorites: {}", e))?;
        let _ = fs::remove_file(&tmp_path);
        return Ok(());
    }
    Ok(())
}

#[tauri::command]
pub fn get_favorites(app: AppHandle) -> Result<Favorites, String> {
    let path = get_favorites_path(&app)?;
    
    if !path.exists() {
        return Ok(Favorites::default());
    }
    
    let content = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read favorites: {}", e))?;
    
    let favorites: Favorites = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse favorites: {}", e))?;
    
    Ok(favorites)
}

#[tauri::command]
pub fn add_favorite(
    app: AppHandle,
    page_title: String,
    command: String,
    description: String,
) -> Result<Favorites, String> {
    let path = get_favorites_path(&app)?;
    let mut favorites = get_favorites(app.clone())?;

    let command = command.trim().to_string();
    let description = description.trim().to_string();
    let normalized = normalize_command(&command);
    
    // 获取或创建该页面的命令列表
    let commands = favorites.items.entry(page_title).or_insert_with(Vec::new);
    
    // 去重检查
    if !commands
        .iter()
        .any(|entry| normalize_command(&entry.command) == normalized)
    {
        commands.push(FavoriteEntry {
            command,
            description,
        });
        favorites.updated_at = current_timestamp();

        write_favorites(&path, &favorites)?;
        let _ = app.emit("favorites-updated", ());
    }
    
    Ok(favorites)
}

#[tauri::command]
pub fn remove_favorite(
    app: AppHandle,
    page_title: String,
    command: String,
) -> Result<Favorites, String> {
    let path = get_favorites_path(&app)?;
    let mut favorites = get_favorites(app.clone())?;
    
    let normalized = normalize_command(&command);
    
    if let Some(commands) = favorites.items.get_mut(&page_title) {
        commands.retain(|entry| normalize_command(&entry.command) != normalized);
        
        // 如果该页面没有收藏了，删除整个条目
        if commands.is_empty() {
            favorites.items.remove(&page_title);
        }
        
        favorites.updated_at = current_timestamp();

        write_favorites(&path, &favorites)?;
        let _ = app.emit("favorites-updated", ());
    }
    
    Ok(favorites)
}

#[tauri::command]
pub fn clear_favorites(app: AppHandle) -> Result<Favorites, String> {
    let path = get_favorites_path(&app)?;
    let favorites = Favorites::default();

    write_favorites(&path, &favorites)?;
    let _ = app.emit("favorites-updated", ());
    
    Ok(favorites)
}

#[tauri::command]
pub fn is_favorite(
    app: AppHandle,
    page_title: String,
    command: String,
) -> Result<bool, String> {
    let favorites = get_favorites(app)?;
    let normalized = normalize_command(&command);
    
    if let Some(commands) = favorites.items.get(&page_title) {
        Ok(commands
            .iter()
            .any(|entry| normalize_command(&entry.command) == normalized))
    } else {
        Ok(false)
    }
}
