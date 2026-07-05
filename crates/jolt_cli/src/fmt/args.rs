use std::{num::NonZeroUsize, path::PathBuf};

use clap::Args as ClapArgs;

#[derive(Debug, ClapArgs)]
#[allow(clippy::struct_excessive_bools)]
pub(crate) struct Args {
    /// Check whether files are formatted without writing changes.
    #[arg(long)]
    pub(crate) check: bool,

    /// Preferred maximum rendered line width.
    #[arg(long)]
    pub(crate) line_width: Option<u16>,

    /// Number of spaces per indentation level when using spaces.
    #[arg(long)]
    pub(crate) indent_width: Option<u8>,

    /// Whether to use tabs for indentation.
    #[arg(long)]
    pub(crate) use_tabs: Option<bool>,

    /// Include source files matching this glob.
    #[arg(long)]
    pub(crate) include: Vec<String>,

    /// Exclude source files matching this glob.
    #[arg(long)]
    pub(crate) exclude: Vec<String>,

    /// Load only this explicit config file.
    #[arg(long)]
    pub(crate) config: Option<PathBuf>,

    /// Disable discovered project configs.
    #[arg(long)]
    pub(crate) no_config: bool,

    /// Path-like name used for stdin language detection and diagnostics.
    #[arg(long)]
    pub(crate) stdin_filename: Option<PathBuf>,

    /// Number of formatter worker threads to use for filesystem paths.
    #[arg(long)]
    pub(crate) threads: Option<NonZeroUsize>,

    /// Files or directories to format. Use '-' to read stdin.
    pub(crate) paths: Vec<PathBuf>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct CliFormatOptions {
    pub(crate) line_width: Option<u16>,
    pub(crate) indent_width: Option<u8>,
    pub(crate) use_tabs: Option<bool>,
}

impl Args {
    pub(crate) fn format_options(&self) -> CliFormatOptions {
        CliFormatOptions {
            line_width: self.line_width,
            indent_width: self.indent_width,
            use_tabs: self.use_tabs,
        }
    }
}
