use std::env;
use std::fs;
use std::io::{self, Read as _};
use std::process::ExitCode;

use jolt_java_fmt::{JavaFormatOptions, format_source};

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let Some(command) = args.next() else {
        return usage();
    };

    match command.as_str() {
        "java" => format_java(args.next().as_deref()),
        "-h" | "--help" => usage(),
        _ => {
            eprintln!("unknown command: {command}");
            usage()
        }
    }
}

fn format_java(path: Option<&str>) -> ExitCode {
    let source = match read_source(path) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("failed to read input: {error}");
            return ExitCode::FAILURE;
        }
    };

    let result = format_source(&source, &JavaFormatOptions::default());
    for diagnostic in &result.diagnostics {
        eprintln!("{}: {}", diagnostic.code, diagnostic.message);
    }

    let Some(formatted) = result.formatted_source else {
        return ExitCode::FAILURE;
    };

    print!("{formatted}");
    ExitCode::SUCCESS
}

fn read_source(path: Option<&str>) -> io::Result<String> {
    match path {
        Some("-") | None => {
            let mut source = String::new();
            io::stdin().read_to_string(&mut source)?;
            Ok(source)
        }
        Some(path) => fs::read_to_string(path),
    }
}

fn usage() -> ExitCode {
    eprintln!("usage: jolt java [path|-]");
    ExitCode::FAILURE
}
