use std::process::ExitCode;

use clap::Parser as _;

mod cli;
mod config;
mod format;
mod schema;

/// Errors are messages for the user. A run that ends with one exits with
/// status 2.
type Result<T> = std::result::Result<T, String>;

/// How a run without errors ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Exit {
    /// Status 0.
    Success,
    /// Status 1: `--check` found files that need formatting.
    Unformatted,
}

fn main() -> ExitCode {
    #[cfg(feature = "docs")]
    if std::env::args().nth(1).as_deref() == Some("__cli-reference") {
        let command = <cli::Cli as clap::CommandFactory>::command();
        print!("{}", clap_markdown::help_markdown_command(&command));
        return ExitCode::SUCCESS;
    }

    match cli::run(cli::Cli::parse()) {
        Ok(Exit::Success) => ExitCode::SUCCESS,
        Ok(Exit::Unformatted) => ExitCode::from(1),
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(2)
        }
    }
}
