//! tealdeer CLI entry point.

use std::{
    io::{self, IsTerminal, Write},
    process::ExitCode,
};

use clap::Parser;

use tealdeer_core::{run, types::PageScope, Cli, RunArgs};

#[cfg(feature = "logging")]
fn init_log() {
    env_logger::init();
}

#[cfg(not(feature = "logging"))]
fn init_log() {}

fn main() -> ExitCode {
    // Initialize logger
    init_log();

    // Parse arguments
    let args = Cli::parse();
    let stdout_is_tty = io::stdout().is_terminal();

    let run_args = RunArgs {
        command: args.command,
        scope: if args.shortcut {
            PageScope::Shortcut
        } else {
            PageScope::Command
        },
        list: args.list,
        edit_page: args.edit_page,
        edit_patch: args.edit_patch,
        render: args.render,
        platforms: args.platforms,
        language: args.language,
        update: args.update,
        no_auto_update: args.no_auto_update,
        clear_cache: args.clear_cache,
        config_path: args.config_path,
        pager: args.pager,
        raw: args.raw,
        quiet: args.quiet,
        show_paths: args.show_paths,
        seed_config: args.seed_config,
        color: args.color,
        enable_styles: None,
        stdout_is_tty,
    };

    match run(run_args) {
        Ok(output) => {
            if !output.stdout.is_empty() {
                let _ = io::stdout().write_all(output.stdout.as_bytes());
            }
            if !output.stderr.is_empty() {
                let _ = io::stderr().write_all(output.stderr.as_bytes());
            }
            if output.exit_code == 0 {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Err(error) => {
            eprintln!("{error:?}");
            ExitCode::FAILURE
        }
    }
}
