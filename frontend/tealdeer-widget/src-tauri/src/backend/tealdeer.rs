use log::{info, warn};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Runtime};

use tealdeer::{run, RunArgs, RunOutput};
use tealdeer::types::{ColorOptions, PageScope, PlatformType};

use crate::backend::settings::ensure_app_config;

#[derive(Debug, Clone, Serialize)]
pub struct ShowPaths {
    pub config_dir: Option<String>,
    pub config_path: Option<String>,
    pub cache_dir: Option<String>,
    pub pages_dir: Option<String>,
    pub shortcut_pages_dir: Option<String>,
    pub custom_pages_dir: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RenderResult {
    pub stdout: String,
    pub stderr: String,
    pub status: Option<i32>,
    pub timed_out: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchEntry {
    pub name: String,
    pub summary: Option<String>,
    pub scope: String,
    pub source: String,
}

#[tauri::command]
pub fn get_show_paths<R: tauri::Runtime>(app: AppHandle<R>) -> Result<ShowPaths, String> {
    get_show_paths_internal(&app)
}

#[tauri::command]
pub fn search_pages<R: tauri::Runtime>(
    app: AppHandle<R>,
    query: String,
    scope: Option<String>,
) -> Result<Vec<SearchEntry>, String> {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return Ok(Vec::new());
    }

    let scope_value = match scope.as_deref() {
        Some("command") => "command",
        Some("shortcut") => "shortcut",
        Some("all") | None => "all",
        Some(other) => {
            return Err(format!("Unsupported scope: {other}"));
        }
    };

    let show_paths = get_show_paths_internal(&app)?;
    let mut results = Vec::new();

    if scope_value == "all" || scope_value == "command" {
        let custom_entries = scan_custom_like_pages(show_paths.custom_pages_dir.as_deref())?;
        let mut custom_names = HashSet::new();
        for (name, summary) in custom_entries {
            custom_names.insert(name.to_lowercase());
            if matches_query(&name, summary.as_deref(), &query) {
                results.push(SearchEntry {
                    name,
                    summary,
                    scope: "command".to_string(),
                    source: "custom".to_string(),
                });
            }
        }

        let tldr_pages = list_tldr_pages(show_paths.pages_dir.as_deref())?;
        for name in tldr_pages {
            if custom_names.contains(&name.to_lowercase()) {
                continue;
            }
            if matches_query(&name, None, &query) {
                results.push(SearchEntry {
                    name,
                    summary: None,
                    scope: "command".to_string(),
                    source: "tldr".to_string(),
                });
            }
        }
    }

    if scope_value == "all" || scope_value == "shortcut" {
        let shortcut_entries = scan_custom_like_pages(show_paths.shortcut_pages_dir.as_deref())?;
        for (name, summary) in shortcut_entries {
            if matches_query(&name, summary.as_deref(), &query) {
                results.push(SearchEntry {
                    name,
                    summary,
                    scope: "shortcut".to_string(),
                    source: "shortcut".to_string(),
                });
            }
        }
    }

    results.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(results)
}

#[tauri::command]
pub fn list_commands<R: tauri::Runtime>(app: AppHandle<R>) -> Result<Vec<String>, String> {
    let show_paths = get_show_paths_internal(&app)?;
    let mut commands = Vec::new();

    let custom_entries = scan_custom_like_pages(show_paths.custom_pages_dir.as_deref())?;
    let mut seen = HashSet::new();
    for (name, _summary) in custom_entries {
        if seen.insert(name.to_lowercase()) {
            commands.push(name);
        }
    }

    let tldr_pages = list_tldr_pages(show_paths.pages_dir.as_deref())?;
    for name in tldr_pages {
        if seen.insert(name.to_lowercase()) {
            commands.push(name);
        }
    }

    commands.sort();
    Ok(commands)
}

#[tauri::command]
pub fn render_tldr<R: tauri::Runtime>(
    app: AppHandle<R>,
    command_tokens: Vec<String>,
    scope: Option<String>,
    language: Option<String>,
    platforms: Vec<String>,
    raw: bool,
    color: Option<String>,
    pager: bool,
    no_auto_update: bool,
) -> Result<RenderResult, String> {
    info!("Render request: {}", command_tokens.join(" "));
    let output = run_render(
        &app,
        command_tokens,
        scope,
        language,
        platforms,
        raw,
        color,
        pager,
        no_auto_update,
    )?;

    if output.exit_code != 0 {
        return Err(format!(
            "tldr failed with exit code {}: {}",
            output.exit_code,
            sanitize_err(&output.stderr)
        ));
    }

    Ok(RenderResult {
        stdout: output.stdout,
        stderr: output.stderr,
        status: Some(output.exit_code),
        timed_out: false,
    })
}

#[tauri::command]
pub fn preview_effective_output(
    app: AppHandle,
    command_tokens: Vec<String>,
    language: Option<String>,
    platforms: Vec<String>,
    raw: bool,
    color: Option<String>,
    pager: bool,
    no_auto_update: bool,
) -> Result<RenderResult, String> {
    render_tldr(
        app,
        command_tokens,
        None, // scope defaults to command
        language,
        platforms,
        raw,
        color,
        pager,
        no_auto_update,
    )
}

#[tauri::command]
pub async fn update_cache<R: tauri::Runtime>(
    app: AppHandle<R>,
    window: tauri::Window<R>
) -> Result<RenderResult, String> {
    // 在后台线程执行，避免阻塞 UI
    tokio::task::spawn_blocking(move || {
        // 发送开始事件
        let _ = window.emit("cache-update-progress", "Downloading tldr pages...");
        
        // 执行更新
        let result = update_cache_internal(&app);
        
        // 发送完成事件
        if result.is_ok() {
            let _ = window.emit("cache-update-progress", "Update complete!");
        }
        
        result
    })
    .await
    .map_err(|e| format!("Task failed: {}", e))?
}

pub(crate) fn update_cache_internal<R: Runtime>(
    app: &AppHandle<R>,
) -> Result<RenderResult, String> {
    info!("Running tldr --update");
    let output = run_update(app)?;
    if output.exit_code != 0 {
        warn!("tldr --update failed: {}", output.exit_code);
        return Err(format!(
            "tldr --update failed: {}",
            sanitize_err(&output.stderr)
        ));
    }

    Ok(RenderResult {
        stdout: output.stdout,
        stderr: output.stderr,
        status: Some(output.exit_code),
        timed_out: false,
    })
}

pub(crate) fn get_show_paths_internal<R: Runtime>(
    app: &AppHandle<R>,
) -> Result<ShowPaths, String> {
    let config_path = ensure_app_config(app)?;
    let args = RunArgs {
        scope: PageScope::Command,
        command: Vec::new(),
        list: false,
        edit_page: false,
        edit_patch: false,
        render: None,
        platforms: None,
        language: None,
        update: false,
        no_auto_update: true,
        clear_cache: false,
        config_path: Some(config_path),
        pager: false,
        raw: false,
        quiet: true,
        show_paths: true,
        seed_config: false,
        color: Some(ColorOptions::Never),
        enable_styles: Some(false),
        stdout_is_tty: false,
    };
    let output = run(args).map_err(|e| format!("{e:?}"))?;
    Ok(parse_show_paths(&output.stdout))
}

fn run_render<R: Runtime>(
    app: &AppHandle<R>,
    command_tokens: Vec<String>,
    scope: Option<String>,
    language: Option<String>,
    platforms: Vec<String>,
    raw: bool,
    color: Option<String>,
    pager: bool,
    no_auto_update: bool,
) -> Result<RunOutput, String> {
    let config_path = ensure_app_config(app)?;
    let platform_values = parse_platforms(&platforms)?;
    let color = parse_color(color.as_deref())?;

    let page_scope = match scope.as_deref() {
        Some("shortcut") => PageScope::Shortcut,
        _ => PageScope::Command,
    };
    
    let args = RunArgs {
        scope: page_scope,
        command: command_tokens,
        list: false,
        edit_page: false,
        edit_patch: false,
        render: None,
        platforms: platform_values,
        language,
        update: false,
        no_auto_update,
        clear_cache: false,
        config_path: Some(config_path),
        pager,
        raw,
        quiet: false,
        show_paths: false,
        seed_config: false,
        color,
        enable_styles: Some(false),
        stdout_is_tty: false,
    };
    run(args).map_err(|e| format!("{e:?}"))
}

fn run_update<R: Runtime>(app: &AppHandle<R>) -> Result<RunOutput, String> {
    let config_path = ensure_app_config(app)?;
    let args = RunArgs {
        scope: PageScope::Command,
        command: Vec::new(),
        list: false,
        edit_page: false,
        edit_patch: false,
        render: None,
        platforms: None,
        language: None,
        update: true,
        no_auto_update: true,
        clear_cache: false,
        config_path: Some(config_path),
        pager: false,
        raw: false,
        quiet: false,
        show_paths: false,
        seed_config: false,
        color: Some(ColorOptions::Never),
        enable_styles: Some(false),
        stdout_is_tty: false,
    };
    run(args).map_err(|e| format!("{e:?}"))
}

fn parse_platforms(values: &[String]) -> Result<Option<Vec<PlatformType>>, String> {
    let mut platforms = Vec::new();
    for value in values {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            continue;
        }
        platforms.push(parse_platform(trimmed)?);
    }
    if platforms.is_empty() {
        Ok(None)
    } else {
        Ok(Some(platforms))
    }
}

fn parse_platform(value: &str) -> Result<PlatformType, String> {
    match value.to_ascii_lowercase().as_str() {
        "linux" => Ok(PlatformType::Linux),
        "macos" | "osx" => Ok(PlatformType::OsX),
        "windows" => Ok(PlatformType::Windows),
        "sunos" => Ok(PlatformType::SunOs),
        "android" => Ok(PlatformType::Android),
        "freebsd" => Ok(PlatformType::FreeBsd),
        "netbsd" => Ok(PlatformType::NetBsd),
        "openbsd" => Ok(PlatformType::OpenBsd),
        "common" => Ok(PlatformType::Common),
        _ => Err(format!("Unsupported platform: {value}")),
    }
}

fn parse_color(value: Option<&str>) -> Result<Option<ColorOptions>, String> {
    let Some(value) = value else {
        return Ok(None);
    };
    match value.to_ascii_lowercase().as_str() {
        "always" => Ok(Some(ColorOptions::Always)),
        "auto" => Ok(Some(ColorOptions::Auto)),
        "never" => Ok(Some(ColorOptions::Never)),
        _ => Err(format!("Unsupported color option: {value}")),
    }
}

fn parse_show_paths(output: &str) -> ShowPaths {
    let mut paths = ShowPaths {
        config_dir: None,
        config_path: None,
        cache_dir: None,
        pages_dir: None,
        shortcut_pages_dir: None,
        custom_pages_dir: None,
    };
    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let value = normalize_show_path_value(value);
        match key.trim() {
            "Config dir" => paths.config_dir = value,
            "Config path" => paths.config_path = value,
            "Cache dir" => paths.cache_dir = value,
            "Pages dir" => paths.pages_dir = value,
            "Shortcut pages dir" => paths.shortcut_pages_dir = value,
            "Custom pages dir" => paths.custom_pages_dir = value,
            _ => {}
        }
    }
    paths
}

fn matches_query(name: &str, summary: Option<&str>, query: &str) -> bool {
    let name_lower = name.to_lowercase();
    if name_lower.contains(query) {
        return true;
    }
    summary
        .map(|text| text.to_lowercase().contains(query))
        .unwrap_or(false)
}

fn list_tldr_pages(pages_dir: Option<&str>) -> Result<Vec<String>, String> {
    let Some(pages_dir) = pages_dir else {
        return Ok(Vec::new());
    };
    let root = PathBuf::from(pages_dir);
    if !root.is_dir() {
        return Ok(Vec::new());
    }

    let mut pages = Vec::new();
    let lang_dirs = match fs::read_dir(&root) {
        Ok(dirs) => dirs,
        Err(err) => return Err(format!("Failed to read pages dir: {err}")),
    };

    for lang_entry in lang_dirs.flatten() {
        let lang_path = lang_entry.path();
        if !lang_path.is_dir() {
            continue;
        }
        let platform_dirs = match fs::read_dir(&lang_path) {
            Ok(dirs) => dirs,
            Err(_) => continue,
        };
        for platform_entry in platform_dirs.flatten() {
            let platform_path = platform_entry.path();
            if !platform_path.is_dir() {
                continue;
            }
            let entries = match fs::read_dir(&platform_path) {
                Ok(entries) => entries,
                Err(_) => continue,
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_file() {
                    continue;
                }
                if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
                    continue;
                }
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    pages.push(stem.to_string());
                }
            }
        }
    }

    pages.sort();
    pages.dedup();
    Ok(pages)
}

fn scan_custom_like_pages(dir: Option<&str>) -> Result<Vec<(String, Option<String>)>, String> {
    let Some(dir) = dir else {
        return Ok(Vec::new());
    };
    let root = PathBuf::from(dir);
    if !root.is_dir() {
        return Ok(Vec::new());
    }

    let entries = match fs::read_dir(&root) {
        Ok(entries) => entries,
        Err(err) => return Err(format!("Failed to read dir: {err}")),
    };

    let mut results = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let file_name = match path.file_name().and_then(|name| name.to_str()) {
            Some(name) => name,
            None => continue,
        };
        if file_name.ends_with(".disabled") {
            continue;
        }
        let Some(slug) = file_name.strip_suffix(".page.md") else {
            continue;
        };
        let summary = extract_summary(&path);
        results.push((slug.to_string(), summary));
    }

    Ok(results)
}

fn extract_summary(path: &Path) -> Option<String> {
    let content = fs::read_to_string(path).ok()?;
    for line in content.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with('>') {
            let text = trimmed.trim_start_matches('>').trim();
            if !text.is_empty() {
                return Some(text.to_string());
            }
        }
    }
    None
}

fn normalize_show_path_value(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed == "[None]" {
        None
    } else {
        let cleaned = if trimmed.ends_with(')') {
            trimmed
                .rsplit_once(" (")
                .map(|(path, _)| path)
                .unwrap_or(trimmed)
        } else {
            trimmed
        };
        Some(cleaned.to_string())
    }
}

fn sanitize_err(message: &str) -> String {
    let trimmed = message.trim();
    if trimmed.is_empty() {
        "Unknown error".to_string()
    } else {
        trimmed.to_string()
    }
}
