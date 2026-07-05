use std::process::ExitCode;

use clap::Parser as _;

mod cli;
mod config_schema;
#[cfg(feature = "docs-generation")]
mod docs_generation;
mod error;
mod fmt;

fn main() -> ExitCode {
    #[cfg(feature = "docs-generation")]
    if let Some(exit_code) = docs_generation::run_from_env_args() {
        return exit_code;
    }

    let cli = cli::Cli::parse();

    match cli::run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
