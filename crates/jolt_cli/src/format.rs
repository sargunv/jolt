//! `jolt format`.

use std::{
    collections::HashSet,
    env,
    fmt::Write as _,
    fs,
    io::{self, Read as _, Write as _},
    num::NonZeroUsize,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use clap::Args;
use ignore::WalkBuilder;
use jolt_config::FormatConfig;
use jolt_formatter::{FormatOptions, FormatResult, Language, LineIndex, OnSyntaxError};
use rayon::prelude::*;

use crate::{
    Result,
    config::{Resolver, Sources, absolute},
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

/// File selection and config loading.
#[derive(Debug, Args)]
pub(crate) struct ConfigArgs {
    /// Include source files matching this glob.
    #[arg(long)]
    include: Vec<String>,

    /// Exclude source files matching this glob.
    #[arg(long)]
    exclude: Vec<String>,

    /// Load only this config file.
    #[arg(long)]
    config: Option<PathBuf>,

    /// Disable discovered project configs.
    #[arg(long)]
    no_config: bool,
}

impl ConfigArgs {
    pub(crate) fn none() -> Self {
        Self {
            include: Vec::new(),
            exclude: Vec::new(),
            config: None,
            no_config: false,
        }
    }

    pub(crate) fn sources<'a>(&'a self, cwd: &'a Path, format: FormatConfig) -> Sources<'a> {
        Sources {
            cwd,
            format,
            include: &self.include,
            exclude: &self.exclude,
            config: self.config.as_deref(),
            no_config: self.no_config,
        }
    }
}

pub(crate) fn run(args: &FormatArgs) -> Result<()> {
    let cwd =
        env::current_dir().map_err(|error| format!("failed to read current directory: {error}"))?;
    let sources = args.config.sources(&cwd, args.format);
    if args.paths.iter().any(|path| path == Path::new("-")) {
        if args.paths.len() != 1 {
            return Err("'-' cannot be combined with filesystem paths".to_owned());
        }
        return run_stdin(&cwd, args, &sources);
    }
    if args.stdin_filename.is_some() {
        return Err("--stdin-filename is only valid when formatting stdin with '-'".to_owned());
    }

    let started = Instant::now();
    let files = collect(&cwd, &args.paths, &sources)?;
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

    let (mut changed, mut failed) = (0, 0);
    let mut stdout = io::stdout().lock();
    let mut stderr = io::stderr().lock();
    for report in &reports {
        changed += usize::from(report.status == Status::Changed);
        failed += usize::from(report.status == Status::Failed);
        stdout
            .write_all(report.stdout.as_bytes())
            .map_err(|error| error.to_string())?;
        stderr
            .write_all(report.stderr.as_bytes())
            .map_err(|error| error.to_string())?;
    }
    let verb = if args.check { "Checked" } else { "Formatted" };
    let _ = writeln!(
        stderr,
        "{verb} {} in {}",
        count(reports.len(), "file"),
        elapsed(started.elapsed())
    );

    if !args.check {
        changed = 0;
    }
    failure(changed, failed)
}

fn run_stdin(cwd: &Path, args: &FormatArgs, sources: &Sources<'_>) -> Result<()> {
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
    let options = Resolver::new(sources, &dir)?.settings(&dir)?.options();

    let mut source = String::new();
    io::stdin()
        .read_to_string(&mut source)
        .map_err(|error| format!("failed to read stdin as UTF-8: {error}"))?;
    let formatted = match format_source(&label, &source, language, options, args.format_with_errors)
    {
        Ok(formatted) => formatted,
        Err(diagnostics) => {
            eprint!("{diagnostics}");
            return failure(0, 1);
        }
    };

    if args.check {
        if formatted == source {
            return Ok(());
        }
        println!("{label}");
        return failure(1, 0);
    }
    io::stdout()
        .write_all(formatted.as_bytes())
        .map_err(|error| format!("failed to write stdout: {error}"))
}

struct Candidate {
    path: PathBuf,
    language: Language,
    options: FormatOptions,
}

/// The files to format: explicit files as given, and the selected source
/// files under each directory, sorted by path.
fn collect(cwd: &Path, paths: &[PathBuf], sources: &Sources<'_>) -> Result<Vec<Candidate>> {
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
            let options = Resolver::new(sources, dir)?.settings(dir)?.options();
            found.push(Candidate {
                path: root,
                language,
                options,
            });
        } else if root.is_dir() {
            let mut resolver = Resolver::new(sources, &root)?;
            for entry in WalkBuilder::new(&root).require_git(false).build() {
                let entry = entry.map_err(|error| error.to_string())?;
                if !entry.file_type().is_some_and(|kind| kind.is_file()) {
                    continue;
                }
                let path = entry.into_path();
                let Some(language) = Language::from_path(&path) else {
                    continue;
                };
                let settings = resolver.settings(path.parent().unwrap_or(&root))?;
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
    Changed,
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
    let formatted = match format_source(
        &label,
        &source,
        file.language,
        file.options,
        args.format_with_errors,
    ) {
        Ok(formatted) => formatted,
        Err(diagnostics) => return Report::failed(diagnostics),
    };
    if formatted == source {
        return Report::new(Status::Unchanged);
    }
    if args.check {
        return Report {
            stdout: format!("{label}\n"),
            ..Report::new(Status::Changed)
        };
    }
    match fs::write(&file.path, formatted) {
        Ok(()) => Report::new(Status::Changed),
        Err(error) => Report::failed(format!(
            "{label}: failed to write formatted file: {error}\n"
        )),
    }
}

/// Formats `source`, or renders its syntax errors when they block formatting.
fn format_source(
    label: &str,
    source: &str,
    language: Language,
    options: FormatOptions,
    format_with_errors: bool,
) -> std::result::Result<String, String> {
    let on_error = if format_with_errors {
        OnSyntaxError::Format
    } else {
        OnSyntaxError::Refuse
    };
    let mut output = String::new();
    match jolt_formatter::format(source, language, &options, on_error, &mut output) {
        FormatResult::Formatted { .. } => Ok(output),
        FormatResult::Refused(diagnostics) => {
            let lines = LineIndex::new(source);
            let mut text = String::new();
            for diagnostic in diagnostics {
                let at = lines.line_col(diagnostic.range.start());
                let _ = writeln!(
                    text,
                    "{label}:{}:{}: {}: {}",
                    at.line + 1,
                    at.col + 1,
                    diagnostic.code,
                    diagnostic.message
                );
            }
            Err(text)
        }
    }
}

/// The error for a run that left `changed` files unformatted and could not
/// format `failed` files, if any.
fn failure(changed: usize, failed: usize) -> Result<()> {
    let mut problems = Vec::new();
    if changed > 0 {
        let verb = if changed == 1 { "needs" } else { "need" };
        problems.push(format!("{} {verb} formatting", count(changed, "file")));
    }
    if failed > 0 {
        problems.push(format!("{} could not be formatted", count(failed, "file")));
    }
    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems.join(", "))
    }
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
