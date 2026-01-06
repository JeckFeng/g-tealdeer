use std::{
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

use log::{debug, info, warn};
use serde::Serialize;
use tauri::{AppHandle, Manager, Runtime};

use crate::backend::{
    process::{run_command, CommandSpec},
    settings::ensure_sidecar_config,
};

const DEFAULT_RENDER_TIMEOUT: Duration = Duration::from_secs(5);
const DEFAULT_SHOW_PATHS_TIMEOUT: Duration = Duration::from_secs(150);
const DEFAULT_VERSION_TIMEOUT: Duration = Duration::from_secs(3);
const DEFAULT_UPDATE_TIMEOUT: Duration = Duration::from_secs(20);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum BackendKind {
    System,
    Sidecar,
}

#[derive(Debug, Clone, Serialize)]
pub struct BackendBinaryInfo {
    pub kind: BackendKind,
    pub path: String,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BackendInfo {
    pub system: Option<BackendBinaryInfo>,
    pub sidecar: Option<BackendBinaryInfo>,
    pub active: Option<BackendBinaryInfo>,
}

#[derive(Debug, Default, Clone, Serialize)]
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

#[derive(Debug, Clone)]
struct BackendResolved {
    kind: BackendKind,
    path: PathBuf,
    version: Option<String>,
    config_dir: Option<PathBuf>,
}

#[derive(Debug, Clone)]
struct RenderOptions {
    command_tokens: Vec<String>,
    language: Option<String>,
    platforms: Vec<String>,
    color: Option<String>,
    no_auto_update: bool,
}

#[tauri::command]
pub fn detect_backend(app: AppHandle) -> Result<BackendInfo, String> {
    let (system, sidecar) = resolve_backends(&app);
    let active = system.clone().or(sidecar.clone());
    let active_kind = active
        .as_ref()
        .map(|backend| match backend.kind {
            BackendKind::System => "system",
            BackendKind::Sidecar => "sidecar",
        })
        .unwrap_or("none");
    info!("Backend detection: active={active_kind}");
    debug!(
        "Backend paths: system={:?}, sidecar={:?}",
        system.as_ref().map(|backend| backend.path.clone()),
        sidecar.as_ref().map(|backend| backend.path.clone())
    );

    Ok(BackendInfo {
        system: system.map(BackendResolved::into_info),
        sidecar: sidecar.map(BackendResolved::into_info),
        active: active.map(BackendResolved::into_info),
    })
}

#[tauri::command]
pub fn get_show_paths(app: AppHandle) -> Result<ShowPaths, String> {
    get_show_paths_internal(&app)
}

#[tauri::command]
pub fn render_tldr(
    app: AppHandle,
    command_tokens: Vec<String>,
    language: Option<String>,
    platforms: Vec<String>,
    raw: bool,
    color: Option<String>,
    pager: bool,
    no_auto_update: bool,
) -> Result<RenderResult, String> {
    let _ = raw;
    let _ = pager;
    validate_command_tokens(&command_tokens)?;
    let command = command_tokens.join(" ");
    info!("Render request: {command}");

    let backend = resolve_active_backend(&app)?;
    let options = RenderOptions {
        command_tokens,
        language,
        platforms,
        color,
        no_auto_update,
    };
    let args = build_render_args(&options)?;
    let output = run_tldr_command(&backend.path, args, DEFAULT_RENDER_TIMEOUT, backend.env())?;

    if output.timed_out {
        warn!("Render timed out: {command}");
        return Err("tldr command timed out".to_string());
    }

    if output.status != Some(0) {
        warn!(
            "Render failed: {command} status={:?}",
            output.status
        );
        return Err(format!(
            "tldr failed with exit code {:?}: {}",
            output.status,
            sanitize_err(&output.stderr)
        ));
    }

    Ok(RenderResult {
        stdout: output.stdout,
        stderr: output.stderr,
        status: output.status,
        timed_out: output.timed_out,
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
pub fn update_cache(app: AppHandle) -> Result<RenderResult, String> {
    update_cache_internal(&app)
}

fn resolve_backends<R: Runtime>(
    app: &AppHandle<R>,
) -> (Option<BackendResolved>, Option<BackendResolved>) {
    let system = find_system_binary().map(|path| BackendResolved {
        kind: BackendKind::System,
        version: read_version(&path),
        path,
        config_dir: None,
    });
    let sidecar = find_sidecar_binary(app).map(|path| BackendResolved {
        kind: BackendKind::Sidecar,
        version: read_version(&path),
        path,
        config_dir: None,
    });
    (system, sidecar)
}

fn resolve_active_backend<R: Runtime>(app: &AppHandle<R>) -> Result<BackendResolved, String> {
    let (system, sidecar) = resolve_backends(app);
    if let Some(backend) = system {
        return Ok(backend);
    }
    if let Some(mut backend) = sidecar {
        let config_dir = ensure_sidecar_config(app)?;
        backend.config_dir = Some(config_dir);
        return Ok(backend);
    }
    Err("No tealdeer binary found (system or sidecar).".to_string())
}

pub(crate) fn update_cache_internal<R: Runtime>(
    app: &AppHandle<R>,
) -> Result<RenderResult, String> {
    info!("Running tldr --update");
    let backend = resolve_active_backend(app)?;
    let output = run_tldr_command(
        &backend.path,
        vec!["--update".to_string()],
        DEFAULT_UPDATE_TIMEOUT,
        backend.env(),
    )?;

    if output.timed_out {
        warn!("tldr --update timed out");
        return Err("tldr --update timed out".to_string());
    }

    if output.status != Some(0) {
        warn!("tldr --update failed: {:?}", output.status);
        return Err(format!(
            "tldr --update failed: {}",
            sanitize_err(&output.stderr)
        ));
    }

    Ok(output)
}

pub(crate) fn get_show_paths_internal<R: Runtime>(app: &AppHandle<R>) -> Result<ShowPaths, String> {
    let backend = resolve_active_backend(app)?;
    let output = run_tldr_command(
        &backend.path,
        vec!["--show-paths".to_string()],
        DEFAULT_SHOW_PATHS_TIMEOUT,
        backend.env(),
    )?;

    if output.timed_out {
        return Err("tldr --show-paths timed out".to_string());
    }

    if output.status != Some(0) {
        return Err(format!(
            "tldr --show-paths failed: {}",
            sanitize_err(&output.stderr)
        ));
    }

    Ok(parse_show_paths(&output.stdout))
}

fn find_system_binary() -> Option<PathBuf> {
    for name in ["tldr", "tealdeer"] {
        if let Ok(path) = which::which(name) {
            return Some(path);
        }
    }
    None
}

fn find_sidecar_binary<R: Runtime>(app: &AppHandle<R>) -> Option<PathBuf> {
    let resource_dir = app.path().resource_dir().ok()?;
    for name in ["tldr", "tealdeer"] {
        let candidate = resource_dir.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }

    let entries = fs::read_dir(&resource_dir).ok()?;
    for entry in entries.flatten() {
        let file_name = entry.file_name();
        let name = file_name.to_string_lossy();
        if name.starts_with("tldr") || name.starts_with("tealdeer") {
            let path = entry.path();
            if path.is_file() {
                return Some(path);
            }
        }
    }

    None
}

fn read_version(path: &Path) -> Option<String> {
    let output = run_tldr_command(
        path,
        vec!["--version".to_string()],
        DEFAULT_VERSION_TIMEOUT,
        vec![("PAGER".to_string(), "cat".to_string())],
    )
    .ok()?;

    if output.timed_out || output.status != Some(0) {
        return None;
    }

    let trimmed = output.stdout.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn run_tldr_command(
    path: &Path,
    args: Vec<String>,
    timeout: Duration,
    env: Vec<(String, String)>,
) -> Result<RenderResult, String> {
    let spec = CommandSpec {
        program: path.to_path_buf(),
        args,
        env,
    };
    let output = run_command(&spec, timeout)?;
    Ok(RenderResult {
        stdout: output.stdout,
        stderr: output.stderr,
        status: output.status,
        timed_out: output.timed_out,
    })
}

fn parse_show_paths(output: &str) -> ShowPaths {
    let mut paths = ShowPaths::default();
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

fn validate_command_tokens(tokens: &[String]) -> Result<(), String> {
    let command = tokens.join(" ").trim().to_string();
    if command.is_empty() {
        return Err("Command is required.".to_string());
    }
    for token in tokens {
        if token.contains('/') || token.contains('\\') {
            return Err("Command contains a path separator.".to_string());
        }
        if token.contains("..") {
            return Err("Command contains a path traversal segment ('..').".to_string());
        }
    }
    Ok(())
}

fn build_render_args(options: &RenderOptions) -> Result<Vec<String>, String> {
    validate_command_tokens(&options.command_tokens)?;
    let mut args = Vec::new();
    args.push("--raw".to_string());

    if let Some(color) = &options.color {
        args.push("--color".to_string());
        args.push(color.to_string());
    }

    for platform in &options.platforms {
        if !platform.trim().is_empty() {
            args.push("--platform".to_string());
            args.push(platform.to_string());
        }
    }

    if let Some(language) = &options.language {
        if !language.trim().is_empty() {
            args.push("--language".to_string());
            args.push(language.to_string());
        }
    }

    if options.no_auto_update {
        args.push("--no-auto-update".to_string());
    }

    args.extend(options.command_tokens.clone());
    Ok(args)
}

impl BackendResolved {
    fn env(&self) -> Vec<(String, String)> {
        let mut env = vec![("PAGER".to_string(), "cat".to_string())];
        if let Some(dir) = &self.config_dir {
            env.push((
                "TEALDEER_CONFIG_DIR".to_string(),
                dir.to_string_lossy().to_string(),
            ));
        }
        env
    }

    fn into_info(self) -> BackendBinaryInfo {
        BackendBinaryInfo {
            kind: self.kind,
            path: self.path.to_string_lossy().to_string(),
            version: self.version,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn validate_command_rejects_path_separators() {
        assert!(validate_command_tokens(&["git/".into()]).is_err());
        assert!(validate_command_tokens(&["git".into(), "log".into(), "..".into()]).is_err());
        assert!(validate_command_tokens(&["git".into(), "log\\".into()]).is_err());
    }

    #[test]
    fn build_render_args_forces_raw() {
        let options = RenderOptions {
            command_tokens: vec!["git".into(), "log".into()],
            language: Some("zh".into()),
            platforms: vec!["linux".into(), "common".into()],
            color: Some("never".into()),
            no_auto_update: true,
        };

        let args = build_render_args(&options).expect("args should build");
        assert_eq!(
            args,
            vec![
                "--raw",
                "--color",
                "never",
                "--platform",
                "linux",
                "--platform",
                "common",
                "--language",
                "zh",
                "--no-auto-update",
                "git",
                "log"
            ]
        );
    }

    #[test]
    fn integration_render_tldr_raw() {
        let Ok(bin) = std::env::var("TEALDEER_TEST_BIN") else {
            return;
        };
        let path = PathBuf::from(bin);
        let output = run_tldr_command(
            &path,
            vec!["--raw".to_string(), "--no-auto-update".to_string(), "tar".to_string()],
            Duration::from_secs(5),
            Vec::new(),
        )
        .expect("tldr command");
        assert_eq!(output.status, Some(0), "tldr exit code");
        assert!(
            !output.stdout.trim().is_empty(),
            "expected non-empty output"
        );
    }
}
