//! Functions for printing pages to the terminal

use std::io::{self, BufRead, Write};

use anyhow::{Context, Result};
use yansi::Paint;

use crate::{
    cache::PageLookupResult,
    config::{Config, StyleConfig},
    formatter::{highlight_lines, PageSnippet},
    line_iterator::LineIterator,
};

/// Set up display pager
///
/// SAFETY: this function may be called multiple times
#[cfg(not(target_os = "windows"))]
fn configure_pager() -> Option<&'static str> {
    use std::sync::Once;
    static INIT: Once = Once::new();
    INIT.call_once(|| pager::Pager::with_default_pager("less -R").setup());
    None
}

#[cfg(target_os = "windows")]
fn configure_pager() -> Option<&'static str> {
    Some("--pager flag not available on Windows!")
}

/// Render page content to the provided writer.
pub fn render_page<W: Write>(
    writer: &mut W,
    lookup_result: &PageLookupResult,
    enable_markdown: bool,
    enable_styles: bool,
    use_pager: bool,
    config: &Config,
    mut warn: Option<&mut dyn FnMut(&str)>,
) -> Result<()> {
    let _ = enable_styles;

    // Create reader from file(s)
    let reader = lookup_result.reader()?;

    // Configure pager if applicable
    if use_pager || config.display.use_pager {
        if let Some(message) = configure_pager() {
            if let Some(warn) = warn.as_mut() {
                warn(message);
            }
        }
    }

    if enable_markdown {
        // Print the raw markdown of the file.
        for line in reader.lines() {
            let line = line.context("Error while reading from a page")?;
            writeln!(writer, "{line}").context("Could not write to output")?;
        }
    } else {
        // Closure that processes a page snippet and writes it to stdout
        let mut process_snippet = |snip: PageSnippet<'_>| {
            if snip.is_empty() {
                Ok(())
            } else {
                print_snippet(writer, snip, &config.style).context("Failed to print snippet")
            }
        };

        // Print highlighted lines
        highlight_lines(
            LineIterator::new(reader),
            &mut process_snippet,
            !config.display.compact,
            config.display.show_title,
        )
        .context("Could not write to output")?;
    }

    // We're done outputting data, flush now!
    writer.flush().context("Could not flush output")?;

    Ok(())
}

/// Render a page into a String.
pub fn render_page_to_string(
    lookup_result: &PageLookupResult,
    enable_markdown: bool,
    enable_styles: bool,
    use_pager: bool,
    config: &Config,
    warn: Option<&mut dyn FnMut(&str)>,
) -> Result<String> {
    let mut buffer = Vec::new();
    render_page(
        &mut buffer,
        lookup_result,
        enable_markdown,
        enable_styles,
        use_pager,
        config,
        warn,
    )?;
    String::from_utf8(buffer).context("Output contained invalid UTF-8")
}

fn print_snippet(
    writer: &mut impl Write,
    snip: PageSnippet<'_>,
    style: &StyleConfig,
) -> io::Result<()> {
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
