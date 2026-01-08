//! tealdeer library crate.

#![deny(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::enum_glob_use)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::similar_names)]
#![allow(clippy::struct_excessive_bools)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::unnecessary_debug_formatting)]

#[cfg(not(any(
    feature = "native-tls",
    feature = "rustls-with-webpki-roots",
    feature = "rustls-with-native-roots",
)))]
compile_error!(
    "at least one of the features \"native-tls\", \"rustls-with-webpki-roots\" or \"rustls-with-native-roots\" must be enabled"
);

use app_dirs::AppInfo;

mod cache;
pub mod cli;
mod config;
mod formatter;
mod line_iterator;
mod output;
pub mod types;
mod utils;

pub mod api;
pub mod extensions;

pub use api::{compute_enable_styles, make_seed_config, run, RunArgs, RunOutput, ShowPaths};
pub use cli::Cli;

pub(crate) const NAME: &str = "tealdeer";
pub(crate) const APP_INFO: AppInfo = AppInfo {
    name: NAME,
    author: NAME,
};
