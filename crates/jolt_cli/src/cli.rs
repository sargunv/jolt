use clap::{Parser, Subcommand};

use crate::{error::CliError, fmt};

pub(crate) const VERSION: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    " (",
    env!("JOLT_COMMIT_SHORT"),
    ")"
);

#[derive(Debug, Parser)]
#[command(name = "jolt", version = VERSION)]
#[command(about = "Fast, opinionated JVM and Kotlin Multiplatform project tooling.")]
pub(crate) struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Format source files.
    Fmt(fmt::Args),
}

pub(crate) fn run(cli: Cli) -> Result<(), CliError> {
    match cli.command {
        Command::Fmt(args) => fmt::run(&args),
    }
}
