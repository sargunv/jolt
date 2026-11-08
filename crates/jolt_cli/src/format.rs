//! `jolt format`.

use std::{
    collections::HashSet,
    env,
    fmt::Write as _,
    fs,
    io::{self, Read as _, Write as _},
    num::NonZeroUsize,
    ops::ControlFlow,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use clap::Args;
use ignore::WalkBuilder;
use jolt_config::FormatConfig;
use jolt_formatter::{FormatOptions, Language, OnSyntaxError, Outcome, Sink};
use rayon::prelude::*;

use crate::{
    Exit, Result,
    config::{ConfigArgs, Resolver, absolute},
};

#[derive(Debug, Args)]
pub(crate) struct FormatArgs {
    /// Check whether files are formatted without writing changes.
    #[arg(long)]
    check: bool,

    #[command(flatten)]
    format: FormatConfig,

    /// Format files even when they contain syntax errors.
    #[arg(long)]
    format_with_errors: bool,

    #[command(flatten)]
    config: ConfigArgs,

    /// Path-like name used for stdin language detection and diagnostics.
    #[arg(long)]
    stdin_filename: Option<PathBuf>,

    /// Number of formatter worker threads to use for filesystem paths.
    #[arg(long)]
    threads: Option<NonZeroUsize>,

    /// Files or directories to format. Use '-' to read stdin.
    paths: Vec<PathBuf>,
}

pub(crate) fn run(args: &FormatArgs) -> Result<Exit> {
    let cwd =
        env::current_dir().map_err(|error| format!("failed to read current directory: {error}"))?;
    let mut resolver = Resolver::new(&cwd, &args.config, args.format)?;
    if args.paths.iter().any(|path| path == Path::new("-")) {
        if args.paths.len() != 1 {
            return Err("'-' cannot be combined with filesystem paths".to_owned());
        }
        return run_stdin(&cwd, args, &mut resolver);
    }
    if args.stdin_filename.is_some() {
        return Err("--stdin-filename is only valid when formatting stdin with '-'".to_owned());
    }

    let started = Instant::now();
    let files = collect(&cwd, &args.paths, &mut resolver)?;
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(args.threads.map_or(0, NonZeroUsize::get))
        .build()
        .map_err(|error| format!("failed to start formatter threads: {error}"))?;
    let reports: Vec<Report> = pool.install(|| {
        files
            .par_iter()
            .map(|file| format_file(&cwd, file, args))
            .collect()
    });

    let mut stdout = io::stdout().lock();
    let mut stderr = io::stderr().lock();
    for report in &reports {
        stdout
            .write_all(report.stdout.as_bytes())
            .map_err(write_error("stdout"))?;
        stderr
            .write_all(report.stderr.as_bytes())
            .map_err(write_error("stderr"))?;
    }
    let tally = |status| {
        reports
            .iter()
            .filter(|report| report.status == status)
            .count()
    };
    let files = count(reports.len(), "file");
    let summary = if args.check {
        format!("Checked {files}")
    } else {
        format!("Reformatted {} of {files}", tally(Status::Reformatted))
    };
    writeln!(stderr, "{summary} in {}", elapsed(started.elapsed()))
        .map_err(write_error("stderr"))?;
    drop(stderr);
    end(tally(Status::Unformatted), tally(Status::Failed))
}

fn run_stdin(cwd: &Path, args: &FormatArgs, resolver: &mut Resolver) -> Result<Exit> {
    let (language, label, dir) = match &args.stdin_filename {
        Some(path) => {
            let language = Language::from_path(path).ok_or_else(|| {
                format!("{}: unsupported stdin filename extension", path.display())
            })?;
            let dir = absolute(cwd, path.parent().unwrap_or(Path::new("")));
            (language, path.display().to_string(), dir)
        }
        None => (Language::Java, "<stdin>".to_owned(), cwd.to_path_buf()),
    };
    let options = resolver.settings(&dir, &dir)?.options();

    let mut source = String::new();
    io::stdin()
        .read_to_string(&mut source)
        .map_err(|error| format!("failed to read stdin as UTF-8: {error}"))?;
    let mut formatted = String::new();
    if let Err(errors) = format_source(
        &label,
        &source,
        language,
        options,
        args.format_with_errors,
        &mut formatted,
    ) {
        io::stderr()
            .write_all(errors.as_bytes())
            .map_err(write_error("stderr"))?;
        return end(0, 1);
    }

    if args.check {
        if formatted == source {
            return Ok(Exit::Success);
        }
        writeln!(io::stdout(), "{label}").map_err(write_error("stdout"))?;
        return end(1, 0);
    }
    io::stdout()
        .write_all(formatted.as_bytes())
        .map_err(write_error("stdout"))?;
    Ok(Exit::Success)
}

struct Candidate {
    path: PathBuf,
    language: Language,
    options: FormatOptions,
}

/// The files to format: explicit files unless an exclude glob matches them,
/// and the selected source files under each directory, sorted by path.
fn collect(cwd: &Path, paths: &[PathBuf], resolver: &mut Resolver) -> Result<Vec<Candidate>> {
    let roots: Vec<PathBuf> = if paths.is_empty() {
        vec![cwd.to_path_buf()]
    } else {
        paths.iter().map(|path| absolute(cwd, path)).collect()
    };
    let mut seen = HashSet::new();
    let mut candidates = Vec::new();
    for root in roots {
        let mut found = Vec::new();
        if root.is_file() {
            let language = Language::from_path(&root).ok_or_else(|| {
                format!("{}: unsupported file extension", display_path(cwd, &root))
            })?;
            let dir = root.parent().unwrap_or(cwd);
            let settings = resolver.settings(dir, dir)?;
            if !settings.excludes(&root) {
                found.push(Candidate {
                    options: settings.options(),
                    path: root,
                    language,
                });
            }
        } else if root.is_dir() {
            for entry in WalkBuilder::new(&root).require_git(false).build() {
                let entry = entry.map_err(|error| error.to_string())?;
                if !entry.file_type().is_some_and(|kind| kind.is_file()) {
                    continue;
                }
                let path = entry.into_path();
                let Some(language) = Language::from_path(&path) else {
                    continue;
                };
                let settings = resolver.settings(path.parent().unwrap_or(&root), &root)?;
                if settings.selects(&path) {
                    found.push(Candidate {
                        options: settings.options(),
                        path,
                        language,
                    });
                }
            }
            found.sort_by(|left, right| left.path.cmp(&right.path));
        } else {
            return Err(format!("{}: path does not exist", display_path(cwd, &root)));
        }
        candidates.extend(
            found
                .into_iter()
                .filter(|candidate| seen.insert(candidate.path.clone())),
        );
    }
    Ok(candidates)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Status {
    Unchanged,
    /// `--check` found the file unformatted.
    Unformatted,
    /// The file was unformatted and is now written formatted.
    Reformatted,
    Failed,
}

/// What formatting one file did, and what to print for it.
struct Report {
    status: Status,
    stdout: String,
    stderr: String,
}

impl Report {
    fn new(status: Status) -> Self {
        Self {
            status,
            stdout: String::new(),
            stderr: String::new(),
        }
    }

    fn failed(stderr: String) -> Self {
        Self {
            stderr,
            ..Self::new(Status::Failed)
        }
    }
}

fn format_file(cwd: &Path, file: &Candidate, args: &FormatArgs) -> Report {
    let label = display_path(cwd, &file.path);
    let source = match fs::read_to_string(&file.path) {
        Ok(source) => source,
        Err(error) => return Report::failed(format!("{label}: failed to read file: {error}\n")),
    };
    if args.check {
        let mut compare = Compare {
            source: &source,
            at: 0,
        };
        return match format_source(
            &label,
            &source,
            file.language,
            file.options,
            args.format_with_errors,
            &mut compare,
        ) {
            Err(errors) => Report::failed(errors),
            Ok(false) if compare.at == source.len() => Report::new(Status::Unchanged),
            Ok(_) => Report {
                stdout: format!("{label}\n"),
                ..Report::new(Status::Unformatted)
            },
        };
    }
    let mut formatted = String::new();
    if let Err(errors) = format_source(
        &label,
        &source,
        file.language,
        file.options,
        args.format_with_errors,
        &mut formatted,
    ) {
        return Report::failed(errors);
    }
    if formatted == source {
        return Report::new(Status::Unchanged);
    }
    match fs::write(&file.path, formatted) {
        Ok(()) => Report::new(Status::Reformatted),
        Err(error) => Report::failed(format!(
            "{label}: failed to write formatted file: {error}\n"
        )),
    }
}

/// Compares formatted output with the source as it is written, and stops the
/// formatter at the first difference.
struct Compare<'s> {
    source: &'s str,
    /// The length of the source prefix the output matched so far.
    at: usize,
}

impl Sink for Compare<'_> {
    fn write(&mut self, chunk: &str) -> ControlFlow<()> {
        let end = self.at + chunk.len();
        if self.source.as_bytes().get(self.at..end) == Some(chunk.as_bytes()) {
            self.at = end;
            ControlFlow::Continue(())
        } else {
            ControlFlow::Break(())
        }
    }
}

/// Formats `source` into `sink`, returning whether the sink stopped it
/// early, or renders its syntax errors when they block formatting.
fn format_source<S: Sink>(
    label: &str,
    source: &str,
    language: Language,
    options: FormatOptions,
    format_with_errors: bool,
    sink: &mut S,
) -> std::result::Result<bool, String> {
    let on_error = if format_with_errors {
        OnSyntaxError::Format
    } else {
        OnSyntaxError::Refuse
    };
    match jolt_formatter::format(source, language, &options, on_error, sink) {
        Outcome::Formatted { halted } => Ok(halted),
        Outcome::Refused(errors) => {
            let mut text = String::new();
            for error in errors {
                let _ = writeln!(text, "{label}:{error}");
            }
            Err(text)
        }
    }
}

/// How a run ends when `unformatted` files need formatting and `failed` files
/// could not be formatted.
fn end(unformatted: usize, failed: usize) -> Result<Exit> {
    let mut problems = Vec::new();
    if unformatted > 0 {
        let verb = if unformatted == 1 { "needs" } else { "need" };
        problems.push(format!("{} {verb} formatting", count(unformatted, "file")));
    }
    if failed > 0 {
        problems.push(format!("{} could not be formatted", count(failed, "file")));
        return Err(problems.join(", "));
    }
    if problems.is_empty() {
        return Ok(Exit::Success);
    }
    writeln!(io::stderr(), "{}", problems.join(", ")).map_err(write_error("stderr"))?;
    Ok(Exit::Unformatted)
}

fn write_error(stream: &'static str) -> impl Fn(io::Error) -> String {
    move |error| format!("failed to write {stream}: {error}")
}

fn count(n: usize, noun: &str) -> String {
    if n == 1 {
        format!("1 {noun}")
    } else {
        format!("{n} {noun}s")
    }
}

fn elapsed(duration: Duration) -> String {
    if duration.as_secs() == 0 {
        format!("{}ms", duration.as_millis())
    } else {
        format!("{:.2}s", duration.as_secs_f64())
    }
}

fn display_path(cwd: &Path, path: &Path) -> String {
    path.strip_prefix(cwd).unwrap_or(path).display().to_string()
}
