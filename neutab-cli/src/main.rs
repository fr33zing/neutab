//! Command line interface for neutab.
//! Create static new tab pages from a config file.

#![warn(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]

mod args;

use args::Args;
use neutab::{
    builder::{self, BuildError},
    resources::Resources,
};

use clap::Parser;
use tracing::error;
use tracing_subscriber::FmtSubscriber;

use std::{
    fs::{self, File},
    io, process,
};

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let resources = Resources {
        config: args.config.clone(),
        scss: args.scss.clone(),
        html: args.html.clone(),
    };

    let result = match args.output.clone().to_str() {
        Some("-") | None => build_to_stdout(args, resources).await,
        Some(file) => build_to_file(args, resources, file).await,
    };

    if let Err(e) = result {
        error!(error = format!("{}", e), "build failed");
        process::exit(1);
    }
}

/// Builds to stdout and logs to stderr.
async fn build_to_stdout(args: Args, resources: Resources) -> Result<(), BuildError> {
    let subscriber = FmtSubscriber::builder()
        .with_writer(io::stderr)
        .with_max_level(args.log_level.as_tracing_level())
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let mut output = io::stdout().lock();
    builder::build(resources, &mut output).await
}

/// Builds to the provided file path.
async fn build_to_file(args: Args, resources: Resources, file: &str) -> Result<(), BuildError> {
    let event_format = tracing_subscriber::fmt::format().without_time().pretty();
    let subscriber = FmtSubscriber::builder()
        .with_max_level(args.log_level.as_tracing_level())
        .event_format(event_format)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let mut output = File::create(file).expect("failed to create output file");
    builder::build(resources, &mut output).await?;

    if args.open {
        let canon = fs::canonicalize(file).expect("failed to canonicalize file");
        webbrowser::open(canon.to_str().expect("invalid path")).expect("failed to open browser");
    }

    Ok(())
}
