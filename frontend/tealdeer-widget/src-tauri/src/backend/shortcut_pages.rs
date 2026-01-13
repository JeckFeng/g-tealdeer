//! Shortcut pages management

use std::{
    fs,
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use super::page_manager::{ExampleInput, FileInfo, PageManager};
use crate::backend::tealdeer::get_show_paths_internal;

#[derive(Debug, Clone, Deserialize)]
pub struct NewPageRequest {
    pub command: String,
    pub summary: String,
    pub examples: Vec<ExampleInput>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NewPatchRequest {
    pub command: String,
    pub examples: Vec<ExampleInput>,
    pub include_header_in_patch: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct CustomEntry {
    pub command_slug: String,
    pub display_command: String,
    pub kind: String,
    pub status: String,
    pub path: String,
    pub mtime: u64,
    pub size: u64,
    pub summary: Option<String>,
    pub examples_count: u32,
}

#[tauri::command]
pub fn create_or_overwrite_shortcut_page<R: tauri::Runtime>(
    app: AppHandle<R>,
    req: NewPageRequest,
) -> Result<FileInfo, String> {
    let slug = slugify(&req.command)?;
    let dir = shortcut_pages_dir(&app)?;
    PageManager::create_page(&dir, &slug, &req.command, &req.summary, &req.examples)
}

#[tauri::command]
pub fn create_or_overwrite_shortcut_patch<R: tauri::Runtime>(
    app: AppHandle<R>,
    req: NewPatchRequest,
) -> Result<FileInfo, String> {
    let slug = slugify(&req.command)?;
    let dir = shortcut_pages_dir(&app)?;
    PageManager::create_patch(&dir, &slug, &req.command, &req.examples, req.include_header_in_patch)
}

#[tauri::command]
pub fn append_example_to_shortcut_page<R: tauri::Runtime>(
    app: AppHandle<R>,
    command: String,
    example: ExampleInput,
) -> Result<FileInfo, String> {
    let slug = slugify(&command)?;
    let dir = shortcut_pages_dir(&app)?;
    let path = dir.join(format!("{slug}.page.md"));
    PageManager::append_example(&path, &example)
}

#[tauri::command]
pub fn scan_shortcut_pages<R: tauri::Runtime>(app: AppHandle<R>) -> Result<Vec<CustomEntry>, String> {
    let dir = shortcut_pages_dir(&app)?;
    if !dir.exists() {
        return Ok(vec![]);
    }

    let entries = fs::read_dir(&dir)
        .map_err(|e| format!("Failed to read dir: {e}"))?;

    let mut results = Vec::new();
    for entry in entries.flatten() {
        if let Ok(meta) = entry.metadata() {
            if meta.is_file() {
                if let Some(file_name) = entry.file_name().to_str() {
                    if let Some(classification) = classify_filename(file_name) {
                        let path = entry.path();
                        let mtime = meta.modified()
                            .ok()
                            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                            .map(|d| d.as_secs())
                            .unwrap_or(0);

                        let (summary, examples_count) = if classification.kind == "page" {
                            extract_summary_and_examples(&path)
                        } else {
                            (None, 0)
                        };

                        results.push(CustomEntry {
                            command_slug: classification.slug.clone(),
                            display_command: slug_to_display_command(&classification.slug),
                            kind: classification.kind,
                            status: classification.status,
                            path: path.display().to_string(),
                            mtime,
                            size: meta.len(),
                            summary,
                            examples_count,
                        });
                    }
                }
            }
        }
    }

    results.sort_by(|a, b| b.mtime.cmp(&a.mtime));
    Ok(results)
}

#[tauri::command]
pub fn delete_shortcut_file<R: tauri::Runtime>(
    app: AppHandle<R>,
    path: String,
) -> Result<(), String> {
    let (_, resolved) = resolve_existing_path(&app, &path)?;
    PageManager::delete_file(&resolved)
}

#[tauri::command]
pub fn disable_shortcut_file<R: tauri::Runtime>(
    app: AppHandle<R>,
    path: String,
) -> Result<FileInfo, String> {
    let (_, resolved) = resolve_existing_path(&app, &path)?;
    PageManager::disable_file(&resolved)
}

#[tauri::command]
pub fn enable_shortcut_file<R: tauri::Runtime>(
    app: AppHandle<R>,
    path: String,
) -> Result<FileInfo, String> {
    let (_, resolved) = resolve_existing_path(&app, &path)?;
    PageManager::enable_file(&resolved)
}

#[tauri::command]
pub fn read_shortcut_file<R: tauri::Runtime>(
    app: AppHandle<R>,
    path: String,
) -> Result<String, String> {
    let (_, resolved) = resolve_existing_path(&app, &path)?;
    PageManager::read_file(&resolved)
}

fn shortcut_pages_dir<R: tauri::Runtime>(app: &AppHandle<R>) -> Result<PathBuf, String> {
    let paths = get_show_paths_internal(app)?;
    paths.shortcut_pages_dir
        .ok_or_else(|| "Shortcut pages directory not configured".to_string())
        .map(PathBuf::from)
}

fn resolve_existing_path<R: tauri::Runtime>(
    app: &AppHandle<R>,
    raw_path: &str,
) -> Result<(PathBuf, PathBuf), String> {
    let dir = shortcut_pages_dir(app)?;
    let dir_canon = fs::canonicalize(&dir)
        .map_err(|e| format!("Failed to resolve shortcut pages dir: {e}"))?;
    let target = PathBuf::from(raw_path);
    if !target.is_absolute() {
        return Err("Path must be absolute.".to_string());
    }
    let target_canon =
        fs::canonicalize(&target).map_err(|e| format!("Failed to resolve file: {e}"))?;
    if !target_canon.starts_with(&dir_canon) {
        return Err("Path is outside the shortcut pages directory.".to_string());
    }
    Ok((dir_canon, target_canon))
}

fn classify_filename(file_name: &str) -> Option<FileClassification> {
    let (kind, status) = if file_name.ends_with(".page.md") {
        ("page", "enabled")
    } else if file_name.ends_with(".patch.md") {
        ("patch", "enabled")
    } else if file_name.ends_with(".page.md.disabled") {
        ("page", "disabled")
    } else if file_name.ends_with(".patch.md.disabled") {
        ("patch", "disabled")
    } else {
        return None;
    };

    let slug = file_name
        .trim_end_matches(".disabled")
        .trim_end_matches(".md")
        .trim_end_matches(".page")
        .trim_end_matches(".patch")
        .to_string();

    Some(FileClassification {
        slug,
        kind: kind.to_string(),
        status: status.to_string(),
    })
}

#[derive(Debug, Clone)]
struct FileClassification {
    slug: String,
    kind: String,
    status: String,
}

fn slug_to_display_command(slug: &str) -> String {
    slug.replace('-', " ")
}

fn extract_summary_and_examples(path: &Path) -> (Option<String>, u32) {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return (None, 0),
    };

    let mut summary = None;
    let mut examples_count = 0;

    for line in content.lines() {
        if line.starts_with('>') {
            summary = Some(line.trim_start_matches('>').trim().to_string());
        } else if line.starts_with('-') {
            examples_count += 1;
        }
    }

    (summary, examples_count)
}

fn slugify(command: &str) -> Result<String, String> {
    let trimmed = command.trim();
    if trimmed.is_empty() {
        return Err("Command cannot be empty".to_string());
    }
    if trimmed.contains('/') || trimmed.contains('\\') || trimmed.contains("..") {
        return Err("Command contains a path separator or traversal segment.".to_string());
    }

    let slug = trimmed
        .to_lowercase()
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
            ch
        } else if ch.is_whitespace() {
            '-'
        } else {
            '_'
        })
        .collect::<String>();

    Ok(slug)
}
