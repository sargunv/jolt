use std::{
    env, fs,
    io::{self, Write as _},
    path::{Path, PathBuf},
};

use clap::{Args as ClapArgs, CommandFactory as _, Parser, Subcommand};
use clap_complete::{Shell, generate};

use crate::{
    config_schema::{self, SchemaKind},
    error::CliError,
    fmt::{
        self, CliFormatOptions,
        config::{self as fmt_config, default_file_config},
        detect_language,
    },
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
}

#[derive(Debug, Subcommand)]
enum ConfigCommand {
    /// Create a root Jolt config in the working directory.
    Init,

    /// List config files that apply to a path.
    List(ConfigTargetArgs),

    /// Print the effective config for a path.
    Resolve(ConfigTargetArgs),

    /// Print a JSON schema for configuration files.
    Schema(ConfigSchemaArgs),
}

#[derive(Debug, ClapArgs)]
struct ConfigTargetArgs {
    /// File or directory to inspect.
    path: Option<PathBuf>,
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
            ConfigCommand::Init => init_config(),
            ConfigCommand::List(args) => config_list(&args),
            ConfigCommand::Resolve(args) => config_resolve(&args),
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
    }
}

fn config_list(args: &ConfigTargetArgs) -> Result<(), CliError> {
    let cwd = current_dir()?;
    let target = ConfigTarget::new(&cwd, args.path.as_deref());
    let paths = fmt_config::discovered_config_paths_for_dir(&target.dir, &target.dir)?;

    let mut stdout = io::stdout().lock();
    for path in paths {
        writeln!(stdout, "{}", path.display())?;
    }
    stdout.flush()?;
    Ok(())
}

fn config_resolve(args: &ConfigTargetArgs) -> Result<(), CliError> {
    let cwd = current_dir()?;
    let target = ConfigTarget::new(&cwd, args.path.as_deref());
    let mut config_graph = fmt_config::ConfigGraph::new(
        &cwd,
        target.dir.clone(),
        CliFormatOptions::default(),
        &[],
        &[],
        None,
        false,
    )?;
    let config = config_graph.resolve_for_dir(&target.dir)?;
    let rendered = fmt_config::render_resolved_config(&config, target.file.as_deref())?;

    let mut stdout = io::stdout().lock();
    stdout.write_all(rendered.as_bytes())?;
    stdout.flush()?;
    Ok(())
}

fn init_config() -> Result<(), CliError> {
    let cwd = current_dir()?;
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

fn current_dir() -> Result<PathBuf, CliError> {
    env::current_dir()
        .map_err(|error| CliError::new(format!("failed to read current directory: {error}")))
}

struct ConfigTarget {
    dir: PathBuf,
    file: Option<PathBuf>,
}

impl ConfigTarget {
    fn new(cwd: &Path, path: Option<&Path>) -> Self {
        let path = path.map_or_else(
            || cwd.to_path_buf(),
            |path| fmt_config::absolutize(cwd, path),
        );

        if path.is_file() || is_supported_source_path(&path) {
            let dir = path
                .parent()
                .map_or_else(|| cwd.to_path_buf(), Path::to_path_buf);
            return Self {
                dir,
                file: Some(path),
            };
        }

        Self {
            dir: path,
            file: None,
        }
    }
}

fn is_supported_source_path(path: &Path) -> bool {
    detect_language(path).is_some()
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
