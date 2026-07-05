use std::{
    env,
    ffi::OsString,
    fs::{self, File},
    io::{self, Write as _},
    path::Path,
    process::ExitCode,
};

use clap::CommandFactory as _;

use crate::cli::{Cli, VERSION};

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
        Some(command) if command == "manpages" => match args.next() {
            Some(out_dir) => match generate_manpages(Path::new(&out_dir)) {
                Ok(()) => ExitCode::SUCCESS,
                Err(error) => {
                    eprintln!("failed to generate manpages: {error}");
                    ExitCode::FAILURE
                }
            },
            None => {
                eprintln!("missing manpage output directory");
                ExitCode::FAILURE
            }
        },
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

fn generate_manpages(out_dir: &Path) -> io::Result<()> {
    fs::create_dir_all(out_dir)?;
    let mut command = Cli::command().disable_help_subcommand(true);
    command.build();
    generate_manpage(&command, "jolt", "jolt", out_dir)
}

fn generate_manpage(
    command: &clap::Command,
    page_name: &str,
    invocation: &str,
    out_dir: &Path,
) -> io::Result<()> {
    let mut page_command = command
        .clone()
        .display_name(page_name)
        .bin_name(invocation)
        .disable_help_subcommand(true);
    page_command.build();

    let path = out_dir.join(format!("{page_name}.1"));
    let mut file = File::create(path)?;
    clap_mangen::Man::new(page_command)
        .source(format!("jolt {VERSION}"))
        .manual("Jolt Manual")
        .render(&mut file)?;
    file.flush()?;

    for subcommand in command
        .get_subcommands()
        .filter(|subcommand| !subcommand.is_hide_set())
    {
        let subcommand_name = subcommand.get_name();
        let subcommand_page_name = format!("{page_name}-{subcommand_name}");
        let subcommand_invocation = format!("{invocation} {subcommand_name}");
        generate_manpage(
            subcommand,
            &subcommand_page_name,
            &subcommand_invocation,
            out_dir,
        )?;
    }

    Ok(())
}

fn unexpected_argument(arg: OsString) -> ExitCode {
    eprintln!(
        "unexpected docs generation argument: {}",
        arg.to_string_lossy()
    );
    ExitCode::FAILURE
}
