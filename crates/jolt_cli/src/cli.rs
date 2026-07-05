use std::io::{self, Write as _};

use clap::{Args as ClapArgs, CommandFactory as _, Parser, Subcommand};
use clap_complete::{Shell, generate};
use clap_mangen::Man;

use crate::{
    config_schema::{self, SchemaKind},
    error::CliError,
    fmt,
};

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

    /// Inspect Jolt configuration.
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },

    /// Generate shell completions.
    Completions {
        /// Shell to generate completions for.
        shell: Shell,
    },

    /// Generate a roff manpage.
    Manpage,
}

#[derive(Debug, Subcommand)]
enum ConfigCommand {
    /// Print a JSON schema for configuration files.
    Schema(ConfigSchemaArgs),
}

#[derive(Debug, ClapArgs)]
struct ConfigSchemaArgs {
    /// Print the dprint plugin configuration schema.
    #[arg(long)]
    dprint: bool,
}

pub(crate) fn run(cli: Cli) -> Result<(), CliError> {
    match cli.command {
        Command::Format(args) => fmt::run(&args),
        Command::Config { command } => match command {
            ConfigCommand::Schema(args) => {
                let kind = if args.dprint {
                    SchemaKind::Dprint
                } else {
                    SchemaKind::Jolt
                };
                let mut stdout = io::stdout().lock();
                config_schema::write_schema(kind, &mut stdout)?;
                stdout.flush()?;
                Ok(())
            }
        },
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
