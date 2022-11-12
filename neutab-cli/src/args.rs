use std::path::PathBuf;

use clap::{ArgGroup, Parser, ValueEnum};

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
    /// Use -o- to write to stdout (disables logging)
    #[arg(short, long, value_name = "FILE", default_value = "neutab.html")]
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

#[derive(ValueEnum, Clone, Debug)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
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
