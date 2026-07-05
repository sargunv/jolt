use std::io::{self, Write as _};

use clap::{CommandFactory as _, Parser, Subcommand};
use clap_complete::{Shell, generate};
use clap_mangen::Man;

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
    #[command(visible_alias = "fmt")]
    Format(fmt::Args),

    /// Generate shell completions.
    Completions {
        /// Shell to generate completions for.
        shell: Shell,
    },

    /// Generate a roff manpage.
    Manpage,
}

pub(crate) fn run(cli: Cli) -> Result<(), CliError> {
    match cli.command {
        Command::Format(args) => fmt::run(&args),
        Command::Completions { shell } => {
            let mut command = Cli::command();
            generate(shell, &mut command, "jolt", &mut io::stdout());
            Ok(())
        }
        Command::Manpage => {
            let command = Cli::command();
            let man = Man::new(command);
            let mut stdout = io::stdout().lock();
            man.render(&mut stdout)?;
            stdout.flush()?;
            Ok(())
        }
    }
}
