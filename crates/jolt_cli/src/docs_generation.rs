use std::{env, ffi::OsString, process::ExitCode};

use clap::CommandFactory as _;

use crate::cli::Cli;

pub(crate) fn run_from_env_args() -> Option<ExitCode> {
    let mut args = env::args_os();
    let _program = args.next()?;
    if args.next().as_deref() != Some(std::ffi::OsStr::new("__docs")) {
        return None;
    }

    let exit_code = match args.next().as_deref() {
        Some(command) if command == "cli-reference" => {
            print_cli_reference();
            ExitCode::SUCCESS
        }
        Some(command) => {
            eprintln!(
                "unknown docs generation command: {}",
                command.to_string_lossy()
            );
            ExitCode::FAILURE
        }
        None => {
            eprintln!("missing docs generation command");
            ExitCode::FAILURE
        }
    };

    if let Some(extra) = args.next() {
        return Some(unexpected_argument(extra));
    }

    Some(exit_code)
}

fn print_cli_reference() {
    let command = Cli::command();
    print!("{}", clap_markdown::help_markdown_command(&command));
}

fn unexpected_argument(arg: OsString) -> ExitCode {
    eprintln!(
        "unexpected docs generation argument: {}",
        arg.to_string_lossy()
    );
    ExitCode::FAILURE
}
