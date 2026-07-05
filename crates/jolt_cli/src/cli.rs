use clap::{Parser, Subcommand};

use crate::{error::CliError, fmt};

#[derive(Debug, Parser)]
#[command(name = "jolt", version)]
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
