use std::process::ExitCode;

use clap::Parser as _;

mod args;
mod config;
mod discover;
mod run;

fn main() -> ExitCode {
    let cli = args::Cli::parse();

    match run::run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
