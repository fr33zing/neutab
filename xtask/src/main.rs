// SPDX-License-Identifier: GPL-3.0-or-later

mod gen_config;

use clap::Parser;

#[derive(clap::Parser)]
struct Args {
    #[command(subcommand)]
    subcommand: Option<Subcommand>,
}

#[derive(clap::Subcommand)]
enum Subcommand {
    /// Generates a test config file using 'lorem ipsum' placeholder text
    GenConfig,
}

fn main() {
    let args = Args::parse();
    match &args.subcommand {
        Some(Subcommand::GenConfig) => gen_config::run(),
        None => println!("Please specify a subcommand. Run with `-h` for help."),
    };
}
