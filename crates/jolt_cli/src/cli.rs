use std::{
    env, fs,
    io::{self, Write as _},
    path::{Path, PathBuf},
};

use clap::{Args as ClapArgs, CommandFactory as _, Parser, Subcommand};
use clap_complete::{Shell, generate};
use clap_mangen::Man;

use crate::{
    config_schema::{self, SchemaKind},
    error::CliError,
    fmt::{self, config::default_file_config},
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

    /// Create a root Jolt config in the working directory.
    Init,

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
        Command::Init => init_config(),
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

fn init_config() -> Result<(), CliError> {
    let cwd = env::current_dir()
        .map_err(|error| CliError::new(format!("failed to read current directory: {error}")))?;
    if let Some(existing) = existing_config_path(&cwd) {
        return Err(CliError::new(format!(
            "{}: config already exists",
            existing.display()
        )));
    }

    let path = cwd.join("jolt.toml");
    fs::write(&path, initial_config_contents()?)?;
    eprintln!("Created {}", path.display());
    Ok(())
}

fn existing_config_path(cwd: &Path) -> Option<PathBuf> {
    [
        cwd.join("jolt.toml"),
        cwd.join(".config/jolt.toml"),
        cwd.join(".config/jolt/config.toml"),
    ]
    .into_iter()
    .find(|path| path.exists())
}

fn initial_config_contents() -> Result<String, CliError> {
    let config = default_file_config();
    let toml = toml_edit::ser::to_string_pretty(&config)
        .map_err(|error| CliError::new(format!("failed to serialize default config: {error}")))?;
    let mut document = toml
        .parse::<toml_edit::DocumentMut>()
        .map_err(|error| CliError::new(format!("failed to parse default config: {error}")))?;
    document
        .decor_mut()
        .set_prefix(format!("#:schema {}\n", jolt_schema_url()));
    Ok(document.to_string())
}

fn jolt_schema_url() -> String {
    format!(
        "{}/releases/download/{}/jolt-schema.json",
        env!("CARGO_PKG_REPOSITORY"),
        env!("CARGO_PKG_VERSION"),
    )
}
