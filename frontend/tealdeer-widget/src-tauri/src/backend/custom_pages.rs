use std::{
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

use log::{debug, info};
use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use crate::backend::tealdeer::get_show_paths;

#[derive(Debug, Clone, Deserialize)]
pub struct ExampleInput {
    pub desc: String,
    pub cmd: String,
}

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
pub struct CustomFileInfo {
    pub path: String,
    pub slug: String,
    pub bytes: u64,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CustomKind {
    Page,
    Patch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CustomStatus {
    Enabled,
    Disabled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FileClassification {
    slug: String,
    kind: CustomKind,
    status: CustomStatus,
}

#[tauri::command]
pub fn create_or_overwrite_page(
    app: AppHandle,
    req: NewPageRequest,
) -> Result<CustomFileInfo, String> {
    validate_command_input(&req.command)?;
    validate_summary(&req.summary)?;
    validate_examples(&req.examples)?;

    let slug = slugify(&req.command)?;
    let dir = custom_pages_dir(&app)?;
    fs::create_dir_all(&dir)
        .map_err(|e| format!("Failed to create custom pages dir: {e}"))?;

    let path = dir.join(format!("{slug}.page.md"));
    let contents = format_page(&req.command, &req.summary, &req.examples)?;
    write_file(&path, &contents)?;
    info!("Created custom page: {}", path.display());

    Ok(build_info(&path, slug))
}

#[tauri::command]
pub fn create_or_overwrite_patch(
    app: AppHandle,
    req: NewPatchRequest,
) -> Result<CustomFileInfo, String> {
    validate_command_input(&req.command)?;
    validate_examples(&req.examples)?;

    let slug = slugify(&req.command)?;
    let dir = custom_pages_dir(&app)?;
    fs::create_dir_all(&dir)
        .map_err(|e| format!("Failed to create custom pages dir: {e}"))?;

    let path = dir.join(format!("{slug}.patch.md"));
    let contents = format_patch(&req.command, &req.examples, req.include_header_in_patch)?;
    write_file(&path, &contents)?;
    info!("Created custom patch: {}", path.display());

    Ok(build_info(&path, slug))
}

#[tauri::command]
pub fn append_example_to_page(
    app: AppHandle,
    command: String,
    example: ExampleInput,
) -> Result<CustomFileInfo, String> {
    validate_command_input(&command)?;
    validate_examples(&[example.clone()])?;

    let slug = slugify(&command)?;
    let dir = custom_pages_dir(&app)?;
    let path = dir.join(format!("{slug}.page.md"));

    if !path.is_file() {
        return Err("Custom page does not exist for this command.".to_string());
    }

    let mut content = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read page file: {e}"))?;
    let block = format_example_block(&example)?;
    content = content.trim_end_matches('\n').to_string();
    content.push_str("\n\n");
    content.push_str(&block);
    write_file(&path, &content)?;
    info!("Appended example to page: {}", path.display());

    Ok(build_info(&path, slug))
}

#[tauri::command]
pub fn scan_custom_pages(app: AppHandle) -> Result<Vec<CustomEntry>, String> {
    let dir = custom_pages_dir(&app)?;
    let dir_entries = match fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(format!("Failed to read custom pages dir: {err}")),
    };

    let mut entries = Vec::new();
    for entry in dir_entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let file_name = match path.file_name().and_then(|name| name.to_str()) {
            Some(name) => name,
            None => continue,
        };
        let Some(classification) = classify_filename(file_name) else {
            continue;
        };

        let metadata = match fs::metadata(&path) {
            Ok(metadata) => metadata,
            Err(_) => continue,
        };
        let size = metadata.len();
        let mtime = metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .map(|duration| duration.as_secs())
            .unwrap_or(0);

        let mut summary = None;
        let mut examples_count = 0;
        if let Ok(contents) = fs::read_to_string(&path) {
            let (summary_text, count) = extract_summary_and_examples(&contents);
            summary = summary_text;
            examples_count = count;
        }

        entries.push(CustomEntry {
            command_slug: classification.slug.clone(),
            display_command: slug_to_display_command(&classification.slug),
            kind: classification.kind.as_str().to_string(),
            status: classification.status.as_str().to_string(),
            path: path.to_string_lossy().to_string(),
            mtime,
            size,
            summary,
            examples_count,
        });
    }

    entries.sort_by(|a, b| a.command_slug.cmp(&b.command_slug));
    debug!("Scanned {} custom entries", entries.len());
    Ok(entries)
}

#[tauri::command]
pub fn delete_custom_file(app: AppHandle, path: String) -> Result<(), String> {
    let (_dir, target) = resolve_existing_path(&app, &path)?;
    fs::remove_file(&target).map_err(|e| format!("Failed to delete file: {e}"))?;
    info!("Deleted custom file: {}", target.display());
    Ok(())
}

#[tauri::command]
pub fn disable_custom_file(app: AppHandle, path: String) -> Result<CustomFileInfo, String> {
    let (_dir, target) = resolve_existing_path(&app, &path)?;
    let new_path = disable_path(&target)?;
    let slug = classification_slug(&file_name_str(&new_path)?)?;
    info!("Disabled custom file: {}", new_path.display());
    Ok(build_info(&new_path, slug))
}

#[tauri::command]
pub fn enable_custom_file(app: AppHandle, path: String) -> Result<CustomFileInfo, String> {
    let (_dir, target) = resolve_existing_path(&app, &path)?;
    let new_path = enable_path(&target)?;
    let slug = classification_slug(&file_name_str(&new_path)?)?;
    info!("Enabled custom file: {}", new_path.display());
    Ok(build_info(&new_path, slug))
}

#[tauri::command]
pub fn read_custom_file(app: AppHandle, path: String) -> Result<String, String> {
    let (_dir, target) = resolve_existing_path(&app, &path)?;
    fs::read_to_string(&target).map_err(|e| format!("Failed to read file: {e}"))
}

fn custom_pages_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let show_paths = get_show_paths(app.clone())?;
    let dir = show_paths
        .custom_pages_dir
        .ok_or_else(|| "Custom pages dir is not configured.".to_string())?;
    Ok(PathBuf::from(dir))
}

fn resolve_existing_path(app: &AppHandle, raw_path: &str) -> Result<(PathBuf, PathBuf), String> {
    let dir = custom_pages_dir(app)?;
    let dir_canon = fs::canonicalize(&dir)
        .map_err(|e| format!("Failed to resolve custom pages dir: {e}"))?;
    let target = PathBuf::from(raw_path);
    if !target.is_absolute() {
        return Err("Path must be absolute.".to_string());
    }
    let target_canon =
        fs::canonicalize(&target).map_err(|e| format!("Failed to resolve file: {e}"))?;
    if !target_canon.starts_with(&dir_canon) {
        return Err("Path is outside the custom pages directory.".to_string());
    }
    Ok((dir_canon, target_canon))
}

fn file_name_str(path: &Path) -> Result<String, String> {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.to_string())
        .ok_or_else(|| "Invalid file name.".to_string())
}

fn classify_filename(file_name: &str) -> Option<FileClassification> {
    if let Some(slug) = file_name.strip_suffix(".page.md.disabled") {
        return classify_slug(slug).map(|slug| FileClassification {
            slug,
            kind: CustomKind::Page,
            status: CustomStatus::Disabled,
        });
    }
    if let Some(slug) = file_name.strip_suffix(".patch.md.disabled") {
        return classify_slug(slug).map(|slug| FileClassification {
            slug,
            kind: CustomKind::Patch,
            status: CustomStatus::Disabled,
        });
    }
    if let Some(slug) = file_name.strip_suffix(".page.md") {
        return classify_slug(slug).map(|slug| FileClassification {
            slug,
            kind: CustomKind::Page,
            status: CustomStatus::Enabled,
        });
    }
    if let Some(slug) = file_name.strip_suffix(".patch.md") {
        return classify_slug(slug).map(|slug| FileClassification {
            slug,
            kind: CustomKind::Patch,
            status: CustomStatus::Enabled,
        });
    }
    None
}

fn classification_slug(file_name: &str) -> Result<String, String> {
    classify_filename(file_name)
        .map(|classification| classification.slug)
        .ok_or_else(|| "Unsupported file name.".to_string())
}

fn classify_slug(slug: &str) -> Option<String> {
    let trimmed = slug.trim_matches('-').trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn slug_to_display_command(slug: &str) -> String {
    slug.split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn extract_summary_and_examples(content: &str) -> (Option<String>, u32) {
    let mut summary = None;
    let mut examples = 0;

    for line in content.lines() {
        let trimmed = line.trim_start();
        if summary.is_none() && trimmed.starts_with('>') {
            let text = trimmed.trim_start_matches('>').trim();
            if !text.is_empty() {
                summary = Some(text.to_string());
            }
        }
        if trimmed.starts_with("- ") {
            examples += 1;
        }
    }

    (summary, examples)
}

impl CustomKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Page => "page",
            Self::Patch => "patch",
        }
    }
}

impl CustomStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Enabled => "enabled",
            Self::Disabled => "disabled",
        }
    }
}

fn format_page(command: &str, summary: &str, examples: &[ExampleInput]) -> Result<String, String> {
    let mut content = String::new();
    content.push_str("# ");
    content.push_str(command.trim());
    content.push('\n');
    content.push_str("> ");
    content.push_str(summary.trim());
    content.push('\n');
    content.push('\n');

    for (idx, example) in examples.iter().enumerate() {
        content.push_str(&format_example_block(example)?);
        if idx + 1 < examples.len() {
            content.push('\n');
        }
    }

    Ok(content)
}

fn format_patch(
    command: &str,
    examples: &[ExampleInput],
    include_header: bool,
) -> Result<String, String> {
    let mut content = String::new();
    if include_header {
        content.push_str("# ");
        content.push_str(command.trim());
        content.push_str(" (custom patch)\n\n");
    }

    for (idx, example) in examples.iter().enumerate() {
        content.push_str(&format_example_block(example)?);
        if idx + 1 < examples.len() {
            content.push('\n');
        }
    }

    Ok(content)
}

fn format_example_block(example: &ExampleInput) -> Result<String, String> {
    let desc = example.desc.trim();
    let cmd = example.cmd.trim();
    if desc.is_empty() || cmd.is_empty() {
        return Err("Example description and command are required.".to_string());
    }
    Ok(format!("- {desc}:\n\n`{cmd}`\n"))
}

fn write_file(path: &Path, contents: &str) -> Result<(), String> {
    fs::write(path, contents).map_err(|e| format!("Failed to write file: {e}"))
}

fn build_info(path: &Path, slug: String) -> CustomFileInfo {
    let bytes = path.metadata().map(|m| m.len()).unwrap_or(0);
    CustomFileInfo {
        path: path.to_string_lossy().to_string(),
        slug,
        bytes,
    }
}

fn validate_summary(summary: &str) -> Result<(), String> {
    if summary.trim().is_empty() {
        Err("Summary is required.".to_string())
    } else {
        Ok(())
    }
}

fn validate_examples(examples: &[ExampleInput]) -> Result<(), String> {
    if examples.is_empty() {
        return Err("At least one example is required.".to_string());
    }
    for example in examples {
        if example.desc.trim().is_empty() || example.cmd.trim().is_empty() {
            return Err("Example description and command are required.".to_string());
        }
    }
    Ok(())
}

fn validate_command_input(command: &str) -> Result<(), String> {
    let trimmed = command.trim();
    if trimmed.is_empty() {
        return Err("Command is required.".to_string());
    }
    if trimmed.contains('/') || trimmed.contains('\\') || trimmed.contains("..") {
        return Err("Command contains a path separator or traversal segment.".to_string());
    }
    Ok(())
}

fn disable_path(target: &Path) -> Result<PathBuf, String> {
    let file_name = file_name_str(target)?;
    if file_name.ends_with(".disabled") {
        return Err("File is already disabled.".to_string());
    }
    if !file_name.ends_with(".page.md") && !file_name.ends_with(".patch.md") {
        return Err("Unsupported file type.".to_string());
    }

    let new_name = format!("{file_name}.disabled");
    let new_path = target
        .parent()
        .ok_or_else(|| "Invalid file path.".to_string())?
        .join(&new_name);
    if new_path.exists() {
        return Err("Disabled file already exists.".to_string());
    }

    fs::rename(target, &new_path)
        .map_err(|e| format!("Failed to disable file: {e}"))?;
    Ok(new_path)
}

fn enable_path(target: &Path) -> Result<PathBuf, String> {
    let file_name = file_name_str(target)?;
    if !file_name.ends_with(".disabled") {
        return Err("File is already enabled.".to_string());
    }

    let new_name = file_name
        .strip_suffix(".disabled")
        .ok_or_else(|| "Invalid disabled file name.".to_string())?;
    if !new_name.ends_with(".page.md") && !new_name.ends_with(".patch.md") {
        return Err("Unsupported file type.".to_string());
    }

    let new_path = target
        .parent()
        .ok_or_else(|| "Invalid file path.".to_string())?
        .join(new_name);
    if new_path.exists() {
        return Err("Enabled file already exists.".to_string());
    }

    fs::rename(target, &new_path)
        .map_err(|e| format!("Failed to enable file: {e}"))?;
    Ok(new_path)
}

fn slugify(command: &str) -> Result<String, String> {
    validate_command_input(command)?;
    let trimmed = command.trim();
    let mut slug = String::new();
    let mut last_was_dash = false;

    for ch in trimmed.chars() {
        let replacement = if ch.is_whitespace() {
            '-'
        } else if is_allowed_slug_char(ch) {
            ch
        } else {
            '-'
        };

        if replacement == '-' {
            if !last_was_dash {
                slug.push('-');
                last_was_dash = true;
            }
        } else {
            slug.push(replacement);
            last_was_dash = false;
        }
    }

    let slug = slug.trim_matches('-').to_lowercase();
    if slug.is_empty() {
        Err("Command resolved to an empty slug.".to_string())
    } else {
        Ok(slug)
    }
}

fn is_allowed_slug_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '+' | '-')
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn slugify_rejects_path_segments() {
        assert!(slugify("../git").is_err());
        assert!(slugify("git/commit").is_err());
        assert!(slugify("git\\commit").is_err());
    }

    #[test]
    fn slugify_normalizes_whitespace() {
        let slug = slugify("git   log").expect("slug should be created");
        assert_eq!(slug, "git-log");
    }

    #[test]
    fn format_page_requires_summary() {
        let err = validate_summary("  ").unwrap_err();
        assert!(err.contains("Summary"));
    }

    #[test]
    fn classify_filename_handles_disabled() {
        let classified = classify_filename("git-log.page.md.disabled").expect("classified");
        assert_eq!(classified.slug, "git-log");
        assert_eq!(classified.kind, CustomKind::Page);
        assert_eq!(classified.status, CustomStatus::Disabled);
    }

    #[test]
    fn classify_filename_handles_enabled_patch() {
        let classified = classify_filename("cargo-add.patch.md").expect("classified");
        assert_eq!(classified.slug, "cargo-add");
        assert_eq!(classified.kind, CustomKind::Patch);
        assert_eq!(classified.status, CustomStatus::Enabled);
    }

    #[test]
    fn extract_summary_and_examples_counts() {
        let content = "# git log\n> Summary line\n\n- Example one:\n`git log`";
        let (summary, count) = extract_summary_and_examples(content);
        assert_eq!(summary.as_deref(), Some("Summary line"));
        assert_eq!(count, 1);
    }

    #[test]
    fn format_page_contains_sections() {
        let examples = vec![ExampleInput {
            desc: "List commits".to_string(),
            cmd: "git log".to_string(),
        }];
        let content = format_page("git log", "Show commits", &examples).expect("page");
        assert!(content.starts_with("# git log\n> Show commits\n\n"));
        assert!(content.contains("- List commits:\n\n`git log`"));
    }

    #[test]
    fn format_patch_includes_optional_header() {
        let examples = vec![ExampleInput {
            desc: "Show last 5".to_string(),
            cmd: "git log -n 5".to_string(),
        }];
        let content = format_patch("git log", &examples, true).expect("patch");
        assert!(content.starts_with("# git log (custom patch)\n\n"));
        assert!(content.contains("- Show last 5:\n\n`git log -n 5`"));
    }

    #[test]
    fn format_example_block_matches_append_style() {
        let example = ExampleInput {
            desc: "List tags".to_string(),
            cmd: "git tag".to_string(),
        };
        let block = format_example_block(&example).expect("block");
        assert_eq!(block, "- List tags:\n\n`git tag`\n");
    }

    #[test]
    fn disable_path_fails_on_conflict() {
        let temp = tempdir().expect("tempdir");
        let page = temp.path().join("git-log.page.md");
        let disabled = temp.path().join("git-log.page.md.disabled");
        fs::write(&page, "# git log").expect("write page");
        fs::write(&disabled, "# disabled").expect("write disabled");
        let err = disable_path(&page).unwrap_err();
        assert!(err.contains("already exists"));
    }

    #[test]
    fn enable_path_fails_on_conflict() {
        let temp = tempdir().expect("tempdir");
        let page = temp.path().join("git-log.page.md");
        let disabled = temp.path().join("git-log.page.md.disabled");
        fs::write(&page, "# git log").expect("write page");
        fs::write(&disabled, "# disabled").expect("write disabled");
        let err = enable_path(&disabled).unwrap_err();
        assert!(err.contains("already exists"));
    }
}
