use std::path::PathBuf;

use clap::{ArgAction, Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "jolt", version)]
#[command(about = "Simple, predictable, and opinionated Java formatter.")]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Command,
}

#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    /// Format source files.
    Fmt(FmtArgs),
}

#[derive(Debug, Args)]
#[allow(clippy::struct_excessive_bools)]
pub(crate) struct FmtArgs {
    /// Check whether files are formatted without writing changes.
    #[arg(long)]
    pub(crate) check: bool,

    /// Preferred maximum rendered line width.
    #[arg(long)]
    pub(crate) line_width: Option<u16>,

    /// Number of spaces per indentation level when using spaces.
    #[arg(long)]
    pub(crate) indent_width: Option<u8>,

    /// Use tabs for indentation.
    #[arg(long, action = ArgAction::SetTrue, conflicts_with = "spaces")]
    pub(crate) tabs: bool,

    /// Use spaces for indentation.
    #[arg(long, action = ArgAction::SetTrue, conflicts_with = "tabs")]
    pub(crate) spaces: bool,

    /// Include source files matching this glob.
    #[arg(long)]
    pub(crate) include: Vec<String>,

    /// Exclude source files matching this glob.
    #[arg(long)]
    pub(crate) exclude: Vec<String>,

    /// Load an explicit config file after discovered project configs.
    #[arg(long, conflicts_with = "no_config")]
    pub(crate) config: Option<PathBuf>,

    /// Disable discovered project configs.
    #[arg(long)]
    pub(crate) no_config: bool,

    /// Path-like name used for stdin language detection and diagnostics.
    #[arg(long)]
    pub(crate) stdin_filename: Option<PathBuf>,

    /// Files or directories to format. Use '-' to read stdin.
    pub(crate) paths: Vec<PathBuf>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct CliFormatOptions {
    pub(crate) line_width: Option<u16>,
    pub(crate) indent_width: Option<u8>,
    pub(crate) tabs: Option<bool>,
}

impl FmtArgs {
    pub(crate) fn format_options(&self) -> CliFormatOptions {
        let tabs = if self.tabs {
            Some(true)
        } else if self.spaces {
            Some(false)
        } else {
            None
        };

        CliFormatOptions {
            line_width: self.line_width,
            indent_width: self.indent_width,
            tabs,
        }
    }
}
