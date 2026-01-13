//! Generic page manager for custom pages and shortcut pages

use std::{
    fs,
    path::{Path, PathBuf},
};

use log::info;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
pub struct ExampleInput {
    pub desc: String,
    pub cmd: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct FileInfo {
    pub path: String,
    pub slug: String,
    pub bytes: u64,
}

/// Generic page manager operations
pub struct PageManager;

impl PageManager {
    pub fn create_page(
        dir: &Path,
        slug: &str,
        command: &str,
        summary: &str,
        examples: &[ExampleInput],
    ) -> Result<FileInfo, String> {
        Self::validate_command_input(command)?;
        Self::validate_summary(summary)?;
        Self::validate_examples(examples)?;
        fs::create_dir_all(dir).map_err(|e| format!("Failed to create dir: {e}"))?;
        
        let path = dir.join(format!("{slug}.page.md"));
        let contents = Self::format_page(command, summary, examples)?;
        Self::write_file(&path, &contents)?;
        info!("Created page: {}", path.display());
        
        Ok(Self::build_info(&path, slug.to_string()))
    }

    pub fn create_patch(
        dir: &Path,
        slug: &str,
        command: &str,
        examples: &[ExampleInput],
        include_header: bool,
    ) -> Result<FileInfo, String> {
        Self::validate_command_input(command)?;
        Self::validate_examples(examples)?;
        fs::create_dir_all(dir).map_err(|e| format!("Failed to create dir: {e}"))?;
        
        let path = dir.join(format!("{slug}.patch.md"));
        let contents = Self::format_patch(command, examples, include_header)?;
        Self::write_file(&path, &contents)?;
        info!("Created patch: {}", path.display());
        
        Ok(Self::build_info(&path, slug.to_string()))
    }

    pub fn append_example(path: &Path, example: &ExampleInput) -> Result<FileInfo, String> {
        Self::validate_examples(std::slice::from_ref(example))?;
        if !path.is_file() {
            return Err("Target page does not exist.".to_string());
        }
        let mut content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read file: {e}"))?;
        
        let example_block = Self::format_example_block(example)?;
        content = content.trim_end_matches('\n').to_string();
        content.push_str("\n\n");
        content.push_str(&example_block);
        
        Self::write_file(path, &content)?;
        info!("Appended example to: {}", path.display());
        let slug = Self::extract_slug(path)?;
        Ok(Self::build_info(path, slug))
    }

    pub fn delete_file(path: &Path) -> Result<(), String> {
        fs::remove_file(path).map_err(|e| format!("Failed to delete: {e}"))?;
        info!("Deleted: {}", path.display());
        Ok(())
    }

    pub fn disable_file(path: &Path) -> Result<FileInfo, String> {
        let new_path = Self::disable_path(path)?;
        let slug = Self::extract_slug(&new_path)?;
        Ok(Self::build_info(&new_path, slug))
    }

    pub fn enable_file(path: &Path) -> Result<FileInfo, String> {
        let new_path = Self::enable_path(path)?;
        let slug = Self::extract_slug(&new_path)?;
        Ok(Self::build_info(&new_path, slug))
    }

    pub fn read_file(path: &Path) -> Result<String, String> {
        fs::read_to_string(path).map_err(|e| format!("Failed to read: {e}"))
    }

    fn format_page(command: &str, summary: &str, examples: &[ExampleInput]) -> Result<String, String> {
        let mut md = format!("# {}\n\n> {}\n\n", command.trim(), summary.trim());
        for (idx, ex) in examples.iter().enumerate() {
            md.push_str(&Self::format_example_block(ex)?);
            if idx + 1 < examples.len() {
                md.push('\n');
            }
        }
        Ok(md)
    }

    fn format_patch(command: &str, examples: &[ExampleInput], include_header: bool) -> Result<String, String> {
        let mut md = String::new();
        if include_header {
            md.push_str(&format!("# {}\n\n", command.trim()));
        }
        for (idx, ex) in examples.iter().enumerate() {
            md.push_str(&Self::format_example_block(ex)?);
            if idx + 1 < examples.len() {
                md.push('\n');
            }
        }
        Ok(md)
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
        fs::write(path, contents).map_err(|e| format!("Failed to write: {e}"))
    }

    fn build_info(path: &Path, slug: String) -> FileInfo {
        let bytes = fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        FileInfo {
            path: path.display().to_string(),
            slug,
            bytes,
        }
    }

    fn disable_path(target: &Path) -> Result<PathBuf, String> {
        let file_name = Self::file_name_str(target)?;
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
            .join(new_name);
        if new_path.exists() {
            return Err("Disabled file already exists.".to_string());
        }
        fs::rename(target, &new_path).map_err(|e| format!("Failed to disable: {e}"))?;
        Ok(new_path)
    }

    fn enable_path(target: &Path) -> Result<PathBuf, String> {
        let file_name = Self::file_name_str(target)?;
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
        fs::rename(target, &new_path).map_err(|e| format!("Failed to enable: {e}"))?;
        Ok(new_path)
    }

    fn extract_slug(path: &Path) -> Result<String, String> {
        let file_name = Self::file_name_str(path)?;
        let base = file_name
            .strip_suffix(".disabled")
            .unwrap_or(file_name.as_str());
        let slug = if let Some(slug) = base.strip_suffix(".page.md") {
            slug
        } else if let Some(slug) = base.strip_suffix(".patch.md") {
            slug
        } else {
            return Err("Unsupported file name.".to_string());
        };
        let trimmed = slug.trim_matches('-').trim();
        if trimmed.is_empty() {
            return Err("Invalid file name.".to_string());
        }
        Ok(trimmed.to_string())
    }

    fn file_name_str(path: &Path) -> Result<String, String> {
        path.file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.to_string())
            .ok_or_else(|| "Invalid file name.".to_string())
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
}
