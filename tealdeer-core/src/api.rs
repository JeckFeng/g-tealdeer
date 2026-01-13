use std::{
    env,
    fmt::Write as _,
    fs::create_dir_all,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{anyhow, Context, Result};
use log::debug;

use crate::{
    cache::{Cache, CacheConfig, PageLookupResult, TLDR_OLD_PAGES_DIR, TLDR_PAGES_DIR},
    config::{get_config_dir, make_default_config, Config, ConfigLoader, Language, PathWithSource},
    output::render_page_to_string,
    types::{ColorOptions, PlatformType},
    utils::{format_error, format_warning},
};

#[derive(Debug, Clone)]
pub struct RunArgs {
    pub scope: crate::types::PageScope,
    pub command: Vec<String>,
    pub list: bool,
    pub edit_page: bool,
    pub edit_patch: bool,
    pub render: Option<PathBuf>,
    pub platforms: Option<Vec<PlatformType>>,
    pub language: Option<String>,
    pub update: bool,
    pub no_auto_update: bool,
    pub clear_cache: bool,
    pub config_path: Option<PathBuf>,
    pub pager: bool,
    pub raw: bool,
    pub quiet: bool,
    pub show_paths: bool,
    pub seed_config: bool,
    pub color: Option<ColorOptions>,
    pub enable_styles: Option<bool>,
    pub stdout_is_tty: bool,
}

#[derive(Debug, Clone, Default)]
pub struct RunOutput {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ShowPaths {
    pub config_dir: Option<String>,
    pub config_path: Option<String>,
    pub cache_dir: Option<String>,
    pub pages_dir: Option<String>,
    pub shortcut_pages_dir: Option<String>,
    pub custom_pages_dir: Option<String>,
}

#[must_use]
pub fn compute_enable_styles(color: ColorOptions, stdout_is_tty: bool, no_color_env: bool) -> bool {
    match color {
        ColorOptions::Always => {
            yansi::enable();
            true
        }
        ColorOptions::Auto => no_color_env && stdout_is_tty,
        ColorOptions::Never => false,
    }
}

/// Creates a seed configuration file.
///
/// # Errors
///
/// Returns an error if the configuration file cannot be created or written to.
pub fn make_seed_config(path: Option<&Path>) -> Result<PathBuf> {
    make_default_config(path)
}

/// Runs the tealdeer application with the given arguments.
///
/// # Errors
///
/// Returns an error if:
/// - Configuration cannot be loaded
/// - Cache operations fail
/// - Page rendering fails
/// - File I/O operations fail
pub fn run(args: RunArgs) -> Result<RunOutput> {
    let mut output = RunOutput::default();
    let color = args.color.unwrap_or_default();
    let enable_styles = args.enable_styles.unwrap_or_else(|| {
        compute_enable_styles(color, args.stdout_is_tty, env::var_os("NO_COLOR").is_none())
    });

    match run_inner(args, enable_styles, &mut output) {
        Ok(exit_code) => output.exit_code = exit_code,
        Err(error) => {
            push_error(&mut output, enable_styles, &error);
            output.exit_code = 1;
        }
    }

    Ok(output)
}

fn run_inner(args: RunArgs, enable_styles: bool, output: &mut RunOutput) -> Result<i32> {
    // Look up config file, if none is found fall back to default config.
    debug!("Loading config");
    let config_loader = match &args.config_path {
        Some(path) if !args.seed_config => {
            ConfigLoader::read(path.clone()).context("Could not read config from given path")?
        }
        _ => {
            ConfigLoader::read_default_path().context("Could not read config from default path")?
        }
    };
    let (mut config, warnings) = config_loader.load()?;
    for warning in warnings {
        push_raw_warning(output, &warning);
    }

    // Override styles if needed
    if !enable_styles {
        config.style = crate::config::StyleConfig::default();
    }

    let custom_pages_dir = match args.scope {
        crate::types::PageScope::Command => config.directories.custom_pages_dir.as_ref(),
        crate::types::PageScope::Shortcut => config.directories.shortcut_pages_dir.as_ref(),
    }
    .map(PathWithSource::path);

    // Note: According to the TLDR client spec, page names must be transparently
    // lowercased before lookup:
    // https://github.com/tldr-pages/tldr/blob/main/CLIENT-SPECIFICATION.md#page-names
    let command = args.command.join("-").to_lowercase();

    if args.edit_patch || args.edit_page {
        let file_name = if args.edit_patch {
            format!("{command}.patch.md")
        } else {
            format!("{command}.page.md")
        };

        custom_pages_dir
            .context("To edit custom pages/patches, please specify a custom pages directory.")
            .and_then(|custom_pages_dir| spawn_editor(output, custom_pages_dir, &file_name))?;

        return Ok(0);
    }

    // Show various paths
    if args.show_paths {
        let (_paths, formatted) = format_show_paths(&config);
        output.stdout.push_str(&formatted);
    }

    // Create a basic config and exit
    if args.seed_config {
        create_config(output, args.config_path.as_deref())?;
        return Ok(0);
    }

    // If a local file was passed in, render it and exit
    if let Some(file) = args.render {
        let path = PageLookupResult::with_page(file);
        render_page_to_output(output, &path, args.raw, enable_styles, args.pager, &config)?;
        return Ok(0);
    }

    if let Some(platforms) = args.platforms {
        config.search.platforms = platforms;
        if !config.search.platforms.contains(&PlatformType::Common) {
            config.search.platforms.push(PlatformType::Common);
        }
    }

    let (search_languages, download_languages): (&[_], &[_]) = match args.language.as_deref() {
        Some(lang) => (&[Language(lang)], &[Language(lang)]),
        None => (&config.search.languages, &config.updates.download_languages),
    };

    let shortcut_scope = matches!(args.scope, crate::types::PageScope::Shortcut);

    // For shortcut scope, use a dummy path to skip TLDR cache lookup
    let dummy_pages_dir = config.directories.cache_dir.path().join("__shortcut_only__");
    let pages_directory = match args.scope {
        crate::types::PageScope::Command => config.directories.cache_dir.path().join(TLDR_PAGES_DIR),
        crate::types::PageScope::Shortcut => {
            // Create the dummy directory if it doesn't exist
            let _ = std::fs::create_dir_all(&dummy_pages_dir);
            dummy_pages_dir
        }
    };
    
    let cache_config = CacheConfig {
        pages_directory: &pages_directory,
        custom_pages_directory: custom_pages_dir,
        platforms: &config.search.platforms,
        search_languages,
        download_languages,
    };

    // TODO: remove in tealdeer 1.9
    let old_config = CacheConfig {
        pages_directory: &config.directories.cache_dir.path().join(TLDR_OLD_PAGES_DIR),
        ..cache_config
    };
    if let Ok(Some(old_cache)) = Cache::open(old_config) {
        old_cache.clear()?;
        push_stderr_line(output, "Cleared pages from old cache location.");
    }

    if args.clear_cache {
        if let Some(cache) = Cache::open(cache_config)? {
            clear_cache(output, cache, args.quiet)?;
        }
        return Ok(0);
    }

    let cache = if shortcut_scope {
        // Shortcut scope never downloads or updates cache data.
        let (cache, _created) = Cache::open_or_create(cache_config)?;
        cache
    } else if args.update || config.updates.auto_update && !args.no_auto_update {
        let (mut cache, was_created) = Cache::open_or_create(cache_config)?;
        if was_created {
            push_stderr_line(
                output,
                &format!(
                    "Successfully created cache directory `{}`.",
                    cache.config().pages_directory.display()
                ),
            );
        }
        if was_created || args.update || cache.age()? >= config.updates.auto_update_interval {
            update_cache(
                output,
                &mut cache,
                config.updates.archive_source,
                config.updates.tls_backend,
                args.quiet,
            )?;
        }

        cache
    } else if args.list || !command.is_empty() {
        // Cache is needed for these commands to work
        let Some(cache) = Cache::open(cache_config)? else {
            push_error(
                output,
                enable_styles,
                &anyhow!("Page cache not found. Please run `tldr --update` to download the cache."),
            );
            push_stdout_line(
                output,
                "\nNote: You can optionally enable automatic cache updates by adding the",
            );
            push_stdout_line(output, "following config to your config file:\n");
            push_stdout_line(output, "  [updates]");
            push_stdout_line(output, "  auto_update = true\n");
            push_stdout_line(
                output,
                "The path to your config file can be looked up with `tldr --show-paths`.",
            );
            push_stdout_line(
                output,
                "To create an initial config file, use `tldr --seed-config`.\n",
            );
            push_stdout_line(output, "You can find more tips and tricks in our docs:\n");
            push_stdout_line(
                output,
                "  https://tealdeer-rs.github.io/tealdeer/config_updates.html",
            );

            return Ok(1);
        };

        let age = cache.age()?;
        if age > crate::config::MAX_CACHE_AGE && !args.quiet {
            push_warning(
                output,
                enable_styles,
                &format!(
                    "The cache hasn't been updated for {} days.\n\
                     You should probably run `tldr --update` soon.",
                    age.as_secs() / 24 / 3600
                ),
            );
        }

        cache
    } else {
        // There is nothing left to do
        return Ok(0);
    };

    if args.list {
        for page in cache.list_pages()? {
            push_stdout_line(output, &page);
        }

        return Ok(0);
    }

    // Show command from cache
    if !command.is_empty() {
        // TODO: Remove this check 1 year after version 1.7.0 was released
        if cache.old_custom_pages_exist()? {
            push_warning(
                output,
                enable_styles,
                &format!(
                    "Custom pages using the old naming convention were found in {}.\n\
                     Please rename them to follow the new convention:\n\
                     - `<name>.page` → `<name>.page.md`\n\
                     - `<name>.patch` → `<name>.patch.md`",
                    cache
                        .config()
                        .custom_pages_directory
                        .expect("Old custom pages can only exist in custom pages directory")
                        .display(),
                ),
            );
        }

        let Some(lookup_result) = cache.find_page(&command) else {
            if !args.quiet {
                push_warning(
                    output,
                    enable_styles,
                    &format!(
                        "Page `{}` not found in cache.\n\
                         Try updating with `tldr --update`, or submit a pull request to:\n\
                         https://github.com/tldr-pages/tldr",
                        &command
                    ),
                );
            }

            return Ok(1);
        };

        render_page_to_output(
            output,
            &lookup_result,
            args.raw,
            enable_styles,
            args.pager,
            &config,
        )?;
    }

    Ok(0)
}

fn clear_cache(output: &mut RunOutput, cache: Cache, quietly: bool) -> Result<()> {
    let cache_dir = cache.config().pages_directory.display();
    cache.clear().context("Could not clear cache")?;
    if !quietly {
        push_stderr_line(
            output,
            &format!("Successfully cleared cache at `{cache_dir}`."),
        );
    }
    Ok(())
}

fn update_cache(
    output: &mut RunOutput,
    cache: &mut Cache,
    archive_source: &str,
    tls_backend: crate::config::TlsBackend,
    quietly: bool,
) -> Result<()> {
    let downloaded_languages = cache
        .update(archive_source, tls_backend)
        .context("Could not update cache")?;
    if !quietly {
        push_stderr_line(output, "Successfully updated cache.");
        let language_strings: Vec<_> = downloaded_languages
            .into_iter()
            .map(|lang| lang.0)
            .collect();
        let mut line = String::from("Pages for the following languages were downloaded: ");
        if language_strings.is_empty() {
            line.push_str("(none)");
        } else {
            line.push_str(&language_strings.join(", "));
        }
        push_stderr_line(output, &line);
    }
    Ok(())
}

fn format_show_paths(config: &Config) -> (ShowPaths, String) {
    let config_dir_display = get_config_dir().map_or_else(
        |e| format!("[Error: {e}]"),
        |(mut path, source)| {
            path.push(""); // Trailing path separator
            match path.to_str() {
                Some(path) => format!("{path} ({source})"),
                None => "[Invalid]".to_string(),
            }
        },
    );
    let config_path_display = config.file_path.to_string();
    let cache_dir_display = config.directories.cache_dir.to_string();
    let pages_dir_display = {
        let mut path = config.directories.cache_dir.path.clone();
        path.push(TLDR_PAGES_DIR);
        path.push(""); // Trailing path separator
        path.display().to_string()
    };
    let custom_pages_dir_display = match config.directories.custom_pages_dir {
        Some(ref path_with_source) => path_with_source.to_string(),
        None => "[None]".to_string(),
    };
    let shortcut_pages_dir_display = match config.directories.shortcut_pages_dir {
        Some(ref path_with_source) => path_with_source.to_string(),
        None => "[None]".to_string(),
    };

    let formatted = format!(
        "Config dir:       {config_dir_display}\n\
Config path:      {config_path_display}\n\
Cache dir:        {cache_dir_display}\n\
Pages dir:        {pages_dir_display}\n\
Shortcut pages dir: {shortcut_pages_dir_display}\n\
Custom pages dir: {custom_pages_dir_display}\n"
    );

    let show_paths = ShowPaths {
        config_dir: normalize_show_path_value(&config_dir_display),
        config_path: normalize_show_path_value(&config_path_display),
        cache_dir: normalize_show_path_value(&cache_dir_display),
        pages_dir: normalize_show_path_value(&pages_dir_display),
        shortcut_pages_dir: normalize_show_path_value(&shortcut_pages_dir_display),
        custom_pages_dir: normalize_show_path_value(&custom_pages_dir_display),
    };

    (show_paths, formatted)
}

fn normalize_show_path_value(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed == "[None]" {
        None
    } else if trimmed.ends_with(')') {
        trimmed
            .rsplit_once(" (")
            .map_or(trimmed, |(path, _)| path)
            .to_string()
            .into()
    } else {
        Some(trimmed.to_string())
    }
}

fn create_config(output: &mut RunOutput, path: Option<&Path>) -> Result<()> {
    let config_file_path = make_default_config(path).context("Could not create seed config")?;
    push_stderr_line(
        output,
        &format!(
            "Successfully created seed config file here: {}",
            config_file_path.to_str().unwrap()
        ),
    );
    Ok(())
}

fn spawn_editor(output: &mut RunOutput, custom_pages_dir: &Path, file_name: &str) -> Result<()> {
    create_dir_all(custom_pages_dir).context("Failed to create custom pages directory")?;

    let custom_page_path = custom_pages_dir.join(file_name);
    let Some(custom_page_path) = custom_page_path.to_str() else {
        return Err(anyhow!("`custom_page_path.to_str()` failed"));
    };
    let Ok(editor) = env::var("EDITOR") else {
        return Err(anyhow!(
            "To edit a custom page, please set the `EDITOR` environment variable."
        ));
    };
    push_stdout_line(output, &format!("Editing {custom_page_path:?}"));

    let status = Command::new(&editor).arg(custom_page_path).status()?;
    if !status.success() {
        return Err(anyhow!("{editor} exit with code {:?}", status.code()));
    }
    Ok(())
}

fn render_page_to_output(
    output: &mut RunOutput,
    lookup_result: &PageLookupResult,
    enable_markdown: bool,
    enable_styles: bool,
    use_pager: bool,
    config: &Config,
) -> Result<()> {
    let mut warn = |message: &str| push_warning(output, enable_styles, message);
    let rendered = render_page_to_string(
        lookup_result,
        enable_markdown,
        enable_styles,
        use_pager,
        config,
        Some(&mut warn),
    )?;
    output.stdout.push_str(&rendered);
    Ok(())
}

fn push_stdout_line(output: &mut RunOutput, line: &str) {
    let _ = writeln!(output.stdout, "{line}");
}

fn push_stderr_line(output: &mut RunOutput, line: &str) {
    let _ = writeln!(output.stderr, "{line}");
}

fn push_warning(output: &mut RunOutput, enable_styles: bool, message: &str) {
    output.warnings.push(message.to_string());
    let formatted = format_warning(enable_styles, message);
    push_stderr_line(output, &formatted);
}

fn push_raw_warning(output: &mut RunOutput, message: &str) {
    output.warnings.push(message.to_string());
    push_stderr_line(output, message);
}

fn push_error(output: &mut RunOutput, enable_styles: bool, error: &anyhow::Error) {
    let formatted = format_error(enable_styles, error);
    push_stderr_line(output, &formatted);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    use tempfile::tempdir;

    fn base_args(config_path: PathBuf) -> RunArgs {
        RunArgs {
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
            show_paths: false,
            seed_config: false,
            color: None,
            enable_styles: None,
            stdout_is_tty: false,
        }
    }

    #[test]
    fn run_renders_markdown_from_file() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.toml");
        make_seed_config(Some(&config_path)).unwrap();

        let page_path = dir.path().join("example.page.md");
        let page = "# foo\n> bar\n\n- list files:\n`ls -la`\n";
        fs::write(&page_path, page).unwrap();

        let mut args = base_args(config_path);
        args.render = Some(page_path);
        args.raw = true;

        let output = run(args).unwrap();
        assert_eq!(output.exit_code, 0);
        assert!(output.stderr.is_empty());
        assert_eq!(output.stdout.trim_end(), page.trim_end());
    }

    #[test]
    fn run_show_paths_includes_config_path() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.toml");
        make_seed_config(Some(&config_path)).unwrap();

        let mut args = base_args(config_path.clone());
        args.show_paths = true;

        let output = run(args).unwrap();
        assert_eq!(output.exit_code, 0);
        assert!(output.stdout.contains("Config path:"));
        let config_path_str = config_path.to_string_lossy();
        assert!(output.stdout.contains(config_path_str.as_ref()));
    }
}
