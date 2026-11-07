use std::{
    env, fs, io,
    path::{Path, PathBuf},
};

use clap::{CommandFactory as _, Parser, Subcommand};
use clap_complete::Shell;
use jolt_config::FormatConfig;
use jolt_formatter::Language;

use crate::{
    Result,
    config::{CONFIG_FILES, ConfigFile, Resolver, Settings, absolute},
    format::{self, ConfigArgs, FormatArgs},
    schema,
};

const VERSION: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    " (",
    env!("JOLT_COMMIT_SHORT"),
    ")"
);

#[derive(Debug, Parser)]
#[command(name = "jolt", version = VERSION, about = "Fast, opinionated Java formatter.")]
pub(crate) struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Format source files.
    #[command(visible_alias = "fmt")]
    Format(FormatArgs),

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
}

#[derive(Debug, Subcommand)]
enum ConfigCommand {
    /// Create a root Jolt config in the working directory.
    Init,

    /// List config files that apply to a path.
    List {
        /// File or directory to inspect.
        path: Option<PathBuf>,
    },

    /// Print the effective config for a path.
    Resolve {
        /// File or directory to inspect.
        path: Option<PathBuf>,
    },

    /// Print a JSON schema for configuration files.
    Schema {
        /// Print the dprint plugin configuration schema.
        #[arg(long)]
        dprint: bool,
    },
}

pub(crate) fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Command::Format(args) => format::run(&args),
        Command::Config { command } => match command {
            ConfigCommand::Init => init(),
            ConfigCommand::List { path } => {
                for source in settings_for(path.as_deref())?.0.sources {
                    println!("{}", source.display());
                }
                Ok(())
            }
            ConfigCommand::Resolve { path } => {
                let (settings, file) = settings_for(path.as_deref())?;
                print!("{}", to_toml(&settings.to_file())?);
                if let Some(file) = file {
                    let status = if settings.selects(&file) {
                        "selected"
                    } else {
                        "not selected"
                    };
                    println!("\n# target {} is {status}", file.display());
                }
                Ok(())
            }
            ConfigCommand::Schema { dprint } => {
                println!("{}", schema::render(dprint));
                Ok(())
            }
        },
        Command::Completions { shell } => {
            clap_complete::generate(shell, &mut Cli::command(), "jolt", &mut io::stdout());
            Ok(())
        }
    }
}

fn init() -> Result<()> {
    let cwd = current_dir()?;
    if let Some(existing) = CONFIG_FILES
        .iter()
        .map(|name| cwd.join(name))
        .find(|path| path.exists())
    {
        return Err(format!("{}: config already exists", existing.display()));
    }
    let schema_url = format!(
        "{}/releases/download/{}/jolt-schema.json",
        env!("CARGO_PKG_REPOSITORY"),
        env!("CARGO_PKG_VERSION"),
    );
    let path = cwd.join("jolt.toml");
    let contents = format!(
        "#:schema {schema_url}\n{}",
        to_toml(&ConfigFile::initial())?
    );
    fs::write(&path, contents).map_err(|error| format!("{}: {error}", path.display()))?;
    eprintln!("Created {}", path.display());
    Ok(())
}

/// The settings for a directory, or for a file (existing or not) and the file.
fn settings_for(path: Option<&Path>) -> Result<(Settings, Option<PathBuf>)> {
    let cwd = current_dir()?;
    let path = path.map_or_else(|| cwd.clone(), |path| absolute(&cwd, path));
    let (dir, file) = if path.is_file() || Language::from_path(&path).is_some() {
        (path.parent().unwrap_or(&cwd).to_path_buf(), Some(path))
    } else {
        (path, None)
    };
    let none = ConfigArgs::none();
    let settings =
        Resolver::new(&none.sources(&cwd, FormatConfig::default()), &dir)?.settings(&dir)?;
    Ok((settings, file))
}

fn to_toml(config: &ConfigFile) -> Result<String> {
    toml::to_string(config).map_err(|error| format!("failed to serialize config: {error}"))
}

fn current_dir() -> Result<PathBuf> {
    env::current_dir().map_err(|error| format!("failed to read current directory: {error}"))
}
