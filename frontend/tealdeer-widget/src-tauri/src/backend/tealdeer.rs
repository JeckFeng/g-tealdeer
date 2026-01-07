use log::{info, warn};
use serde::Serialize;
use tauri::{AppHandle, Runtime};

use tealdeer::{run, RunArgs, RunOutput};
use tealdeer::types::{ColorOptions, PlatformType};

use crate::backend::settings::ensure_app_config;

#[derive(Debug, Clone, Serialize)]
pub struct ShowPaths {
    pub config_dir: Option<String>,
    pub config_path: Option<String>,
    pub cache_dir: Option<String>,
    pub pages_dir: Option<String>,
    pub custom_pages_dir: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RenderResult {
    pub stdout: String,
    pub stderr: String,
    pub status: Option<i32>,
    pub timed_out: bool,
}

#[tauri::command]
pub fn get_show_paths<R: tauri::Runtime>(app: AppHandle<R>) -> Result<ShowPaths, String> {
    get_show_paths_internal(&app)
}

#[tauri::command]
pub fn render_tldr<R: tauri::Runtime>(
    app: AppHandle<R>,
    command_tokens: Vec<String>,
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
        language,
        platforms,
        raw,
        color,
        pager,
        no_auto_update,
    )
}

#[tauri::command]
pub fn update_cache<R: tauri::Runtime>(app: AppHandle<R>) -> Result<RenderResult, String> {
    update_cache_internal(&app)
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

    let args = RunArgs {
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
            "Custom pages dir" => paths.custom_pages_dir = value,
            _ => {}
        }
    }
    paths
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
