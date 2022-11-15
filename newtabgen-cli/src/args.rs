// SPDX-License-Identifier: GPL-3.0-or-later

//! Defines command line arguments by providing the `[Args]` struct.

use std::path::PathBuf;

use clap::{ArgGroup, Parser, ValueEnum};

/// Defines command line arguments.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(group(
    ArgGroup::new("source")
        .required(true)
        .args(["config", "example"])
))]
#[command(group(
    ArgGroup::new("logging")
        .args(["log_level", "silent"])
))]
pub(crate) struct Args {
    /// Configuration file
    pub config: Option<PathBuf>,

    /// Output file
    ///
    /// Use -o- to output to stdout and log to stderr.
    #[arg(short, long, value_name = "FILE", default_value = "newtabgen.html")]
    pub output: PathBuf,

    /// Preview output in default browser
    #[arg(long)]
    pub open: bool,

    /// Override default template with provided HTML file
    #[arg(long, value_name = "FILE")]
    pub html: Option<PathBuf>,

    /// Override default styles with provided SCSS file
    #[arg(long, value_name = "FILE")]
    pub scss: Option<PathBuf>,

    /// Build using an example config
    #[arg(long)]
    pub example: bool,

    /// Log level
    #[arg(short, long, value_enum, value_name = "LEVEL", default_value_t = LogLevel::Info)]
    pub log_level: LogLevel,

    /// Disable all logging
    #[arg(short, long)]
    pub silent: bool,
}

/// 1:1 with [`tracing::Level`] to aid in argument parsing, since tracing's levels are structs.
#[derive(ValueEnum, Clone, Debug)]
pub(crate) enum LogLevel {
    /// Corresponds to `tracing::Level::TRACE`.
    Trace,

    /// Corresponds to `tracing::Level::DEBUG`.
    Debug,

    /// Corresponds to `tracing::Level::INFO`.
    Info,

    /// Corresponds to `tracing::Level::WARN`.
    Warn,

    /// Corresponds to `tracing::Level::ERROR`.
    Error,
}

impl LogLevel {
    /// Converts the [`LogLevel`] to the corresponding [`tracing::Level`].
    pub fn as_tracing_level(&self) -> tracing::Level {
        match self {
            LogLevel::Trace => tracing::Level::TRACE,
            LogLevel::Debug => tracing::Level::DEBUG,
            LogLevel::Info => tracing::Level::INFO,
            LogLevel::Warn => tracing::Level::WARN,
            LogLevel::Error => tracing::Level::ERROR,
        }
    }
}
