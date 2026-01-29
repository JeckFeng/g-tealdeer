use std::{
    env,
    fmt::Write as _,
    fs::create_dir_all,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{anyhow, Context, Result};
use log::debug;
use yansi::Paint;

use crate::{
    cache::{Cache, CacheConfig, PageLookupResult, TLDR_OLD_PAGES_DIR, TLDR_PAGES_DIR},
    config::{
        get_config_dir, make_default_config, Config, ConfigLoader, Language, PathWithSource,
        StyleConfig,
    },
    formatter::{highlight_lines, PageSnippet},
    output::render_page_to_string,
    types::{ColorOptions, LineType, PlatformType},
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
    pub fallback_from: Option<String>,
    pub fallback_filters: Vec<String>,
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

#[derive(Debug)]
struct ResolvedPage {
    lookup: PageLookupResult,
    resolved_name: String,
    requested_name: String,
    filters: Vec<String>,
    used_fallback: bool,
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
        render_page_to_output(
            output,
            &path,
            args.raw,
            enable_styles,
            args.pager,
            &config,
            None,
        )?;
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
    let dummy_pages_dir = config
        .directories
        .cache_dir
        .path()
        .join("__shortcut_only__");
    let pages_directory = match args.scope {
        crate::types::PageScope::Command => {
            config.directories.cache_dir.path().join(TLDR_PAGES_DIR)
        }
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

        let resolved = if shortcut_scope {
            cache.find_page(&command).map(|lookup| ResolvedPage {
                lookup,
                resolved_name: command.clone(),
                requested_name: command.clone(),
                filters: Vec::new(),
                used_fallback: false,
            })
        } else {
            resolve_page_with_fallback(&args.command, &cache)
        };

        let Some(resolved) = resolved else {
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

        let filters = if resolved.used_fallback && !resolved.filters.is_empty() {
            Some(resolved.filters.as_slice())
        } else {
            None
        };

        if resolved.used_fallback {
            output.fallback_from = Some(resolved.requested_name.clone());
            output.fallback_filters = resolved.filters.clone();
            if !args.quiet && !resolved.filters.is_empty() {
                push_warning(
                    output,
                    enable_styles,
                    &format!(
                        "No page for {}, showing {} (filtered by: {}).",
                        resolved.requested_name,
                        resolved.resolved_name,
                        resolved.filters.join(", ")
                    ),
                );
            }
        }

        render_page_to_output(
            output,
            &resolved.lookup,
            args.raw,
            enable_styles,
            args.pager,
            &config,
            filters,
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
    filters: Option<&[String]>,
) -> Result<()> {
    if let Some(filters) = filters {
        let (rendered, matched_any) = render_filtered_page(
            lookup_result,
            enable_markdown,
            enable_styles,
            use_pager,
            config,
            filters,
        )?;
        output.stdout.push_str(&rendered);
        if !matched_any {
            push_warning(
                output,
                enable_styles,
                &format!("No examples matched filters: {}", filters.join(", ")),
            );
        }
        return Ok(());
    }

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

struct LineWithRaw {
    line_type: LineType,
    raw_lines: Vec<String>,
}

fn render_filtered_page(
    lookup_result: &PageLookupResult,
    enable_markdown: bool,
    _enable_styles: bool,
    use_pager: bool,
    config: &Config,
    filters: &[String],
) -> Result<(String, bool)> {
    setup_pager_if_needed(use_pager, config);
    let mut reader = lookup_result.reader()?;
    let mut content = String::new();
    reader.read_to_string(&mut content)?;

    let records = collect_lines_with_raw(&content);
    let (filtered_records, matched_any) = filter_line_records(records, filters);

    if enable_markdown {
        let mut out = String::new();
        for record in filtered_records {
            for line in record.raw_lines {
                out.push_str(&line);
                out.push('\n');
            }
        }
        return Ok((out, matched_any));
    }

    let mut output = Vec::new();
    let mut process_snippet = |snip: PageSnippet<'_>| {
        if snip.is_empty() {
            Ok(())
        } else {
            print_snippet(&mut output, snip, &config.style).context("Failed to print snippet")
        }
    };

    let line_types = filtered_records
        .into_iter()
        .map(|record| record.line_type);

    highlight_lines(
        line_types,
        &mut process_snippet,
        !config.display.compact,
        config.display.show_title,
    )
    .context("Could not write to output")?;

    Ok((String::from_utf8(output).context("Output contained invalid UTF-8")?, matched_any))
}

fn setup_pager_if_needed(use_pager: bool, config: &Config) {
    if !(use_pager || config.display.use_pager) {
        return;
    }

    #[cfg(not(target_os = "windows"))]
    {
        use std::sync::Once;
        static INIT: Once = Once::new();
        INIT.call_once(|| pager::Pager::with_default_pager("less -R").setup());
    }
}

fn print_snippet(
    writer: &mut impl Write,
    snip: PageSnippet<'_>,
    style: &StyleConfig,
) -> std::io::Result<()> {
    use PageSnippet::*;

    match snip {
        CommandName(s) => write!(writer, "{}", s.paint(style.command_name)),
        Variable(s) => write!(writer, "{}", s.paint(style.example_variable)),
        NormalCode(s) => write!(writer, "{}", s.paint(style.example_code)),
        Description(s) => writeln!(writer, "  {}", s.paint(style.description)),
        Text(s) => writeln!(writer, "  {}", s.paint(style.example_text)),
        Title(s) => writeln!(writer, "  {}", s.paint(style.command_name)),
        Linebreak => writeln!(writer),
    }
}

fn collect_lines_with_raw(content: &str) -> Vec<LineWithRaw> {
    let lines: Vec<String> = content.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return Vec::new();
    }

    let mut records = Vec::new();
    let is_v1 = lines.first().map(|line| line.starts_with('#')).unwrap_or(false);

    let mut index = 0;
    if !is_v1 {
        let title_line = lines[0].clone();
        let mut raw_lines = vec![title_line.clone()];
        if lines.len() > 1 {
            raw_lines.push(lines[1].clone());
            index = 2;
        } else {
            index = 1;
        }
        records.push(LineWithRaw {
            line_type: LineType::Title(title_line.trim_end().to_string()),
            raw_lines,
        });
    }

    for line in lines.into_iter().skip(index) {
        let line_type = if is_v1 {
            LineType::from_v1(&line)
        } else {
            LineType::from(line.as_str())
        };
        records.push(LineWithRaw {
            line_type,
            raw_lines: vec![line],
        });
    }

    records
}

fn normalize_for_filter(value: &str) -> String {
    let mut out = String::new();
    for ch in value.to_lowercase().chars() {
        if ch == '-' || ch.is_whitespace() {
            out.push(' ');
        } else {
            out.push(ch);
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod fallback_filter_tests {
    use super::{filter_line_records, normalize_for_filter, resolve_page_with_fallback, LineWithRaw};
    use crate::{cache::Cache, cache::CacheConfig, config::Language, types::LineType, types::PlatformType};
    use std::fs;
    use tempfile::tempdir;

    fn build_record(line_type: LineType, raw: &str) -> LineWithRaw {
        LineWithRaw {
            line_type,
            raw_lines: vec![raw.to_string()],
        }
    }

    #[test]
    fn normalize_for_filter_splits_hyphen_and_space() {
        assert_eq!(normalize_for_filter("git-checkout"), "git checkout");
        assert_eq!(normalize_for_filter("git   checkout"), "git checkout");
    }

    #[test]
    fn filter_examples_and_matches_description_and_command() {
        let records = vec![
            build_record(LineType::Title("systemctl".to_string()), "# systemctl"),
            build_record(LineType::Description("desc".to_string()), "> desc"),
            build_record(LineType::ExampleText("Stop a unit".to_string()), "- Stop a unit"),
            build_record(LineType::ExampleCode("systemctl stop unit".to_string()), "`systemctl stop unit`"),
            build_record(LineType::ExampleText("Start a unit".to_string()), "- Start a unit"),
            build_record(LineType::ExampleCode("systemctl start unit".to_string()), "`systemctl start unit`"),
        ];

        let (filtered, matched) = filter_line_records(records, &["stop".to_string()]);
        let filtered_text: Vec<String> = filtered
            .iter()
            .filter_map(|record| match &record.line_type {
                LineType::ExampleText(text) => Some(text.clone()),
                _ => None,
            })
            .collect();

        assert!(matched);
        assert_eq!(filtered_text, vec!["Stop a unit".to_string()]);
    }

    #[test]
    fn filter_examples_handles_empty_result() {
        let records = vec![
            build_record(LineType::ExampleText("Start a unit".to_string()), "- Start a unit"),
            build_record(LineType::ExampleCode("systemctl start unit".to_string()), "`systemctl start unit`"),
        ];

        let (_filtered, matched) = filter_line_records(records, &["stop".to_string()]);
        assert!(!matched);
    }

    #[test]
    fn resolve_page_with_fallback_uses_first_match() {
        let dir = tempdir().expect("tempdir");
        let pages_dir = dir.path().join("pages.en").join("common");
        fs::create_dir_all(&pages_dir).expect("create pages dir");
        fs::write(pages_dir.join("systemctl.md"), "# systemctl").expect("write page");

        let config = CacheConfig {
            pages_directory: dir.path(),
            custom_pages_directory: None,
            platforms: &[PlatformType::Common],
            search_languages: &[Language("en")],
            download_languages: &[Language("en")],
        };
        let cache = Cache::open_or_create(config).expect("cache").0;

        let tokens = vec!["systemctl".to_string(), "stop".to_string()];
        let resolved = resolve_page_with_fallback(&tokens, &cache).expect("resolved");
        assert_eq!(resolved.resolved_name, "systemctl");
        assert_eq!(resolved.filters, vec!["stop".to_string()]);
        assert!(resolved.used_fallback);
    }
}

fn matches_filters(blob: &str, filters: &[String]) -> bool {
    let normalized_blob = normalize_for_filter(blob);
    filters.iter().all(|filter| {
        let normalized_filter = normalize_for_filter(filter);
        !normalized_filter.is_empty() && normalized_blob.contains(&normalized_filter)
    })
}

fn filter_line_records(
    records: Vec<LineWithRaw>,
    filters: &[String],
) -> (Vec<LineWithRaw>, bool) {
    if filters.is_empty() {
        return (records, true);
    }

    let mut output = Vec::new();
    let mut pending: Option<LineWithRaw> = None;
    let mut matched_any = false;

    for record in records {
        match record.line_type {
            LineType::ExampleText(text) => {
                if let Some(pending_record) = pending.take() {
                    output.push(pending_record);
                }
                pending = Some(LineWithRaw {
                    line_type: LineType::ExampleText(text),
                    raw_lines: record.raw_lines,
                });
            }
            LineType::ExampleCode(code) => {
                if let Some(pending_record) = pending.take() {
                    let pending_text = match &pending_record.line_type {
                        LineType::ExampleText(text) => text.as_str(),
                        _ => "",
                    };
                    let combined = format!("{pending_text} {code}");
                    if matches_filters(&combined, filters) {
                        output.push(pending_record);
                        output.push(LineWithRaw {
                            line_type: LineType::ExampleCode(code),
                            raw_lines: record.raw_lines,
                        });
                        matched_any = true;
                    }
                } else if matches_filters(&code, filters) {
                    output.push(LineWithRaw {
                        line_type: LineType::ExampleCode(code),
                        raw_lines: record.raw_lines,
                    });
                    matched_any = true;
                }
            }
            other => {
                if let Some(pending_record) = pending.take() {
                    output.push(pending_record);
                }
                output.push(LineWithRaw {
                    line_type: other,
                    raw_lines: record.raw_lines,
                });
            }
        }
    }

    if let Some(pending_record) = pending.take() {
        output.push(pending_record);
    }

    (output, matched_any)
}

fn resolve_page_with_fallback(tokens: &[String], cache: &Cache) -> Option<ResolvedPage> {
    if tokens.is_empty() {
        return None;
    }

    let requested_name = tokens.join("-").to_lowercase();
    for cut in (1..=tokens.len()).rev() {
        let candidate = tokens[..cut].join("-").to_lowercase();
        if let Some(lookup) = cache.find_page(&candidate) {
            let used_fallback = candidate != requested_name;
            let filters = if used_fallback {
                tokens[cut..]
                    .iter()
                    .map(|token| token.to_lowercase())
                    .collect()
            } else {
                Vec::new()
            };

            return Some(ResolvedPage {
                lookup,
                resolved_name: candidate,
                requested_name: requested_name.clone(),
                filters,
                used_fallback,
            });
        }
    }

    None
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
    use crate::types::PageScope;
    use std::fs;
    use std::path::Path;

    use tempfile::tempdir;

    fn base_args(config_path: PathBuf) -> RunArgs {
        RunArgs {
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
            show_paths: false,
            seed_config: false,
            color: None,
            enable_styles: None,
            stdout_is_tty: false,
        }
    }

    fn write_test_config(dir: &Path) -> PathBuf {
        let config_path = dir.join("config.toml");
        let cache_dir = dir.join("cache");
        let custom_pages_dir = dir.join("custom_pages");
        let shortcut_pages_dir = dir.join("shortcut_pages");

        let config = format!(
            r#"
[directories]
cache_dir = "{cache_dir}"
custom_pages_dir = "{custom_pages_dir}"
shortcut_pages_dir = "{shortcut_pages_dir}"

[search]
languages = ["en"]
platforms = ["common"]
"#,
            cache_dir = cache_dir.to_string_lossy().replace('\\', "\\\\"),
            custom_pages_dir = custom_pages_dir.to_string_lossy().replace('\\', "\\\\"),
            shortcut_pages_dir = shortcut_pages_dir.to_string_lossy().replace('\\', "\\\\"),
        );

        fs::write(&config_path, config).unwrap();
        config_path
    }

    fn write_page(dir: &Path, name: &str, content: &str) {
        let pages_dir = dir.join("cache").join("tldr-pages").join("pages.en").join("common");
        fs::create_dir_all(&pages_dir).unwrap();
        fs::write(pages_dir.join(format!("{name}.md")), content).unwrap();
    }

    #[test]
    fn run_fallback_filters_examples_for_common_commands() {
        let dir = tempdir().unwrap();
        let config_path = write_test_config(dir.path());

        write_page(
            dir.path(),
            "systemctl",
            "# systemctl\n> desc\n\n- Stop a unit:\n`systemctl stop unit`\n\n- Start a unit:\n`systemctl start unit`\n",
        );
        write_page(
            dir.path(),
            "git",
            "# git\n> desc\n\n- Checkout a branch:\n`git checkout branch`\n\n- Show status:\n`git status`\n",
        );
        write_page(
            dir.path(),
            "docker",
            "# docker\n> desc\n\n- Run a container:\n`docker run image`\n\n- List images:\n`docker images`\n",
        );

        let mut args = base_args(config_path.clone());
        args.raw = true;

        args.command = vec!["systemctl".into(), "stop".into()];
        let output = run(args.clone()).unwrap();
        assert_eq!(output.fallback_from.as_deref(), Some("systemctl-stop"));
        assert_eq!(output.fallback_filters, vec!["stop"]);
        assert!(output.stdout.contains("systemctl stop unit"));
        assert!(!output.stdout.contains("systemctl start unit"));

        args.command = vec!["git".into(), "checkout".into()];
        let output = run(args.clone()).unwrap();
        assert_eq!(output.fallback_from.as_deref(), Some("git-checkout"));
        assert!(output.stdout.contains("git checkout branch"));
        assert!(!output.stdout.contains("git status"));

        args.command = vec!["docker".into(), "run".into()];
        let output = run(args).unwrap();
        assert_eq!(output.fallback_from.as_deref(), Some("docker-run"));
        assert!(output.stdout.contains("docker run image"));
        assert!(!output.stdout.contains("docker images"));
    }

    #[test]
    fn shortcut_scope_does_not_use_fallback() {
        let dir = tempdir().unwrap();
        let config_path = write_test_config(dir.path());

        let mut args = base_args(config_path);
        args.scope = PageScope::Shortcut;
        args.command = vec!["systemctl".into(), "stop".into()];
        args.raw = true;

        let output = run(args).unwrap();
        assert_eq!(output.exit_code, 1);
        assert!(output.fallback_from.is_none());
        assert!(output.fallback_filters.is_empty());
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
