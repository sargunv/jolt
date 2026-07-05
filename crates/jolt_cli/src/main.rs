use std::process::ExitCode;

use clap::Parser as _;

mod cli;
mod config_schema;
mod error;
mod fmt;

fn main() -> ExitCode {
    let cli = cli::Cli::parse();

    match cli::run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
