use std::process::ExitCode;

use clap::Parser as _;

mod cli;
mod config;
mod format;
mod schema;

/// Errors are messages for the user.
type Result<T> = std::result::Result<T, String>;

fn main() -> ExitCode {
    #[cfg(feature = "docs")]
    if std::env::args().nth(1).as_deref() == Some("__cli-reference") {
        let command = <cli::Cli as clap::CommandFactory>::command();
        print!("{}", clap_markdown::help_markdown_command(&command));
        return ExitCode::SUCCESS;
    }

    match cli::run(cli::Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
