//! `jolt format`.

use std::{
    collections::{HashMap, HashSet},
    env,
    fmt::Write as _,
    fs,
    io::{self, Read as _, Write as _},
    num::NonZeroUsize,
    ops::ControlFlow,
    path::{Path, PathBuf},
    sync::{Mutex, MutexGuard, PoisonError},
    thread,
    time::{Duration, Instant},
};

use clap::Args;
use ignore::{DirEntry, WalkBuilder, WalkState};
use jolt_config::FormatConfig;
use jolt_formatter::{FormatOptions, Language, OnSyntaxError, Outcome, Sink};

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
    let resolver = Resolver::new(&cwd, &args.config, args.format)?;
    if args.paths.iter().any(|path| path == Path::new("-")) {
        if args.paths.len() != 1 {
            return Err("'-' cannot be combined with filesystem paths".to_owned());
        }
        return run_stdin(&cwd, args, &resolver);
    }
    if args.stdin_filename.is_some() {
        return Err("--stdin-filename is only valid when formatting stdin with '-'".to_owned());
    }

    let started = Instant::now();
    let reports = format_paths(&cwd, args, &resolver);

    let mut stdout = io::stdout().lock();
    let mut stderr = io::stderr().lock();
    // Every file under a broken config fails with the config's error.
    let mut printed = HashSet::new();
    for report in &reports {
        stdout
            .write_all(report.stdout.as_bytes())
            .map_err(write_error("stdout"))?;
        if !report.stderr.is_empty() && printed.insert(report.stderr.as_str()) {
            stderr
                .write_all(report.stderr.as_bytes())
                .map_err(write_error("stderr"))?;
        }
    }
    let tally = |status| {
        reports
            .iter()
            .filter(|report| report.status == status)
            .count()
    };
    let files = count(reports.len() - tally(Status::Unreadable), "file");
    let summary = if args.check {
        format!("Checked {files}")
    } else {
        format!("Reformatted {} of {files}", tally(Status::Reformatted))
    };
    writeln!(stderr, "{summary} in {}", elapsed(started.elapsed()))
        .map_err(write_error("stderr"))?;
    drop(stderr);
    end(
        tally(Status::Unformatted),
        tally(Status::Failed),
        tally(Status::Unreadable),
    )
}

fn run_stdin(cwd: &Path, args: &FormatArgs, resolver: &Resolver) -> Result<Exit> {
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
        return end(0, 1, 0);
    }

    if args.check {
        if formatted == source {
            return Ok(Exit::Success);
        }
        writeln!(io::stdout(), "{label}").map_err(write_error("stdout"))?;
        return end(1, 0, 0);
    }
    io::stdout()
        .write_all(formatted.as_bytes())
        .map_err(write_error("stdout"))?;
    Ok(Exit::Success)
}

/// Formats the files under the paths named on the command line (or the
/// working directory): each named file unless an exclude glob matches it, and
/// the selected source files under each named directory. The formatter
/// threads walk the directories and format each file as they find it. An error
/// fails one file or one path, and the run goes on.
///
/// Returns a report for each file and each path that could not be read, in
/// the order the paths are named, then by path.
fn format_paths(cwd: &Path, args: &FormatArgs, resolver: &Resolver) -> Vec<Report> {
    let paths = if args.paths.is_empty() {
        vec![cwd.to_path_buf()]
    } else {
        args.paths.iter().map(|path| absolute(cwd, path)).collect()
    };
    let mut walk = Walk {
        cwd,
        args,
        resolver,
        roots: HashMap::new(),
        seen: None,
        reports: Mutex::default(),
    };
    let mut builder: Option<WalkBuilder> = None;
    for (index, path) in paths.into_iter().enumerate() {
        if walk.roots.contains_key(&path) {
            continue;
        }
        if path.is_file() || path.is_dir() {
            match &mut builder {
                Some(builder) => {
                    builder.add(&path);
                }
                None => builder = Some(WalkBuilder::new(&path)),
            }
        } else {
            let message = format!("{}: path does not exist\n", display_path(cwd, &path));
            walk.report(index, path.clone(), Report::unreadable(message));
        }
        walk.roots.insert(path, index);
    }
    // Sorted paths put each directory right before the paths under it.
    let mut sorted: Vec<&Path> = walk.roots.keys().map(PathBuf::as_path).collect();
    sorted.sort_unstable();
    if sorted.windows(2).any(|pair| pair[1].starts_with(pair[0])) {
        walk.seen = Some(Mutex::default());
    }
    if let Some(mut builder) = builder {
        // `ignore` uses at most 12 threads unless told otherwise.
        let threads = args
            .threads
            .or_else(|| thread::available_parallelism().ok())
            .map_or(1, NonZeroUsize::get);
        builder
            .require_git(false)
            .threads(threads)
            .build_parallel()
            .run(|| {
                Box::new(|entry| {
                    walk.visit(entry);
                    WalkState::Continue
                })
            });
    }
    let mut reports = walk
        .reports
        .into_inner()
        .unwrap_or_else(PoisonError::into_inner);
    reports.sort_unstable_by(|(left, left_report), (right, right_report)| {
        left.cmp(right)
            .then_with(|| left_report.stderr.cmp(&right_report.stderr))
    });
    reports.into_iter().map(|(_, report)| report).collect()
}

/// What the formatter threads share while they walk the paths of one run.
struct Walk<'a> {
    cwd: &'a Path,
    args: &'a FormatArgs,
    resolver: &'a Resolver,
    /// Each path named on the command line, with the position where it is
    /// first named.
    roots: HashMap<PathBuf, usize>,
    /// The files claimed for formatting so far, when one root contains
    /// another and the walk can find a file under both.
    seen: Option<Mutex<HashSet<PathBuf>>>,
    /// The reports so far, each by the position of its root and its path.
    reports: Mutex<Vec<((usize, PathBuf), Report)>>,
}

impl Walk<'_> {
    fn visit(&self, entry: std::result::Result<DirEntry, ignore::Error>) {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => return self.unreadable(&error),
        };
        if !entry.file_type().is_some_and(|kind| kind.is_file()) {
            return;
        }
        let named = entry.depth() == 0;
        let path = entry.into_path();
        let Some((index, root)) = self.root(&path) else {
            return;
        };
        let Some(language) = Language::from_path(&path) else {
            if named {
                let message = format!(
                    "{}: unsupported file extension\n",
                    display_path(self.cwd, &path)
                );
                self.report(index, path, Report::failed(message));
            }
            return;
        };
        // The settings come from the first root that contains the file, so
        // they don't depend on which root's walk finds it first.
        let dir = path.parent().unwrap_or(self.cwd);
        let target = if root == path { dir } else { root };
        let settings = match self.resolver.settings(dir, target) {
            Ok(settings) => settings,
            Err(error) => {
                if self.claim(&path) {
                    self.report(index, path, Report::failed(format!("{error}\n")));
                }
                return;
            }
        };
        let selected = if named {
            !settings.excludes(&path)
        } else {
            settings.selects(&path)
        };
        if selected && self.claim(&path) {
            let report = format_file(self.cwd, &path, language, settings.options(), self.args);
            self.report(index, path, report);
        }
    }

    /// The first root named on the command line that contains `path`, and
    /// its position.
    fn root(&self, path: &Path) -> Option<(usize, &Path)> {
        if self.roots.len() == 1 {
            return self
                .roots
                .iter()
                .next()
                .map(|(root, &index)| (index, root.as_path()));
        }
        let mut containing = path
            .ancestors()
            .filter_map(|ancestor| self.roots.get_key_value(ancestor));
        let first = if self.seen.is_none() {
            // No root contains another, so the nearest is the only one.
            containing.next()
        } else {
            containing.min_by_key(|(_, index)| **index)
        };
        first.map(|(root, &index)| (index, root.as_path()))
    }

    /// Whether this thread is the first to claim `path`.
    fn claim(&self, path: &Path) -> bool {
        self.seen
            .as_ref()
            .is_none_or(|seen| lock(seen).insert(path.to_path_buf()))
    }

    fn unreadable(&self, error: &ignore::Error) {
        let (path, cause) = error_path(error);
        let Some(path) = path else {
            let report = Report::unreadable(format!("{error}\n"));
            return self.report(usize::MAX, PathBuf::new(), report);
        };
        let index = self.root(path).map_or(usize::MAX, |(index, _)| index);
        let message = format!("{}: {cause}\n", display_path(self.cwd, path));
        self.report(index, path.to_path_buf(), Report::unreadable(message));
    }

    fn report(&self, root: usize, path: PathBuf, report: Report) {
        lock(&self.reports).push(((root, path), report));
    }
}

/// The path a walk error is about, if it names one, and the error without
/// the path.
fn error_path(error: &ignore::Error) -> (Option<&Path>, &ignore::Error) {
    match error {
        ignore::Error::WithPath { path, err } => (Some(path), err),
        ignore::Error::WithDepth { err, .. } => error_path(err),
        _ => (None, error),
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Status {
    Unchanged,
    /// `--check` found the file unformatted.
    Unformatted,
    /// The file was unformatted and is now written formatted.
    Reformatted,
    Failed,
    /// A named path that doesn't exist, or a directory the walk could not
    /// read.
    Unreadable,
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

    fn unreadable(stderr: String) -> Self {
        Self {
            stderr,
            ..Self::new(Status::Unreadable)
        }
    }
}

fn format_file(
    cwd: &Path,
    path: &Path,
    language: Language,
    options: FormatOptions,
    args: &FormatArgs,
) -> Report {
    let label = display_path(cwd, path);
    let source = match fs::read_to_string(path) {
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
            language,
            options,
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
        language,
        options,
        args.format_with_errors,
        &mut formatted,
    ) {
        return Report::failed(errors);
    }
    if formatted == source {
        return Report::new(Status::Unchanged);
    }
    match fs::write(path, formatted) {
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

/// How a run ends when `unformatted` files need formatting, `failed` files
/// could not be formatted and `unreadable` paths could not be read.
fn end(unformatted: usize, failed: usize, unreadable: usize) -> Result<Exit> {
    let mut problems = Vec::new();
    if unformatted > 0 {
        let verb = if unformatted == 1 { "needs" } else { "need" };
        problems.push(format!("{} {verb} formatting", count(unformatted, "file")));
    }
    if failed > 0 {
        problems.push(format!("{} could not be formatted", count(failed, "file")));
    }
    if unreadable > 0 {
        problems.push(format!("{} could not be read", count(unreadable, "path")));
    }
    if failed > 0 || unreadable > 0 {
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
