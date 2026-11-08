//! `jolt format`.

use std::{
    collections::{HashMap, HashSet, hash_map::Entry},
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
/// Paths that name one file or directory in different ways, through `..` or a
/// symlink, are the same path: it is walked once, under the spelling named
/// first.
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
        overlap: None,
        reports: Mutex::default(),
    };
    // Each named path that exists, once per spelling.
    let mut named = Vec::new();
    let mut spellings = HashSet::new();
    for (index, path) in paths.into_iter().enumerate() {
        if !spellings.insert(path.clone()) {
            continue;
        }
        match fs::metadata(&path) {
            Ok(metadata) if metadata.is_file() || metadata.is_dir() => {
                named.push((index, path, metadata));
            }
            _ => {
                let message = format!("{}: path does not exist\n", display_path(cwd, &path));
                walk.report(index, path, Report::unreadable(message));
            }
        }
    }
    // A named directory can contain another named path, so the paths
    // compare by location. Without one, only a file named twice can be found
    // twice, and its identity is cheaper to compare.
    let contained = named.len() > 1 && named.iter().any(|(_, _, metadata)| metadata.is_dir());
    let mut locations = HashMap::new();
    let mut files = HashSet::new();
    let mut builder: Option<WalkBuilder> = None;
    for (index, path, metadata) in named {
        let first = if contained {
            // A path that exists but doesn't resolve is its own location.
            let location = fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
            match locations.entry(location) {
                Entry::Occupied(_) => false,
                Entry::Vacant(entry) => {
                    entry.insert((index, path.clone()));
                    true
                }
            }
        } else {
            files.insert(file_id(&path, &metadata))
        };
        if !first {
            continue;
        }
        match &mut builder {
            Some(builder) => {
                builder.add(&path);
            }
            None => builder = Some(WalkBuilder::new(&path)),
        }
        walk.roots.insert(path, index);
    }
    // Sorted paths put each directory right before the paths under it.
    let mut sorted: Vec<&Path> = locations.keys().map(PathBuf::as_path).collect();
    sorted.sort_unstable();
    if sorted.windows(2).any(|pair| pair[1].starts_with(pair[0])) {
        walk.overlap = Some(Overlap {
            locations: locations
                .iter()
                .map(|(location, (_, root))| (root.clone(), location.clone()))
                .collect(),
            roots: locations,
            seen: Mutex::default(),
        });
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
    /// Each path named on the command line that the walk reads, by its
    /// absolute spelling, with the position where it is first named.
    roots: HashMap<PathBuf, usize>,
    /// Set when one root contains another, so the walk can find a path under
    /// both.
    overlap: Option<Overlap>,
    /// The reports so far, each by the position of its root and its path.
    reports: Mutex<Vec<((usize, PathBuf), Report)>>,
}

/// The roots by location, when one root contains another. A location is an
/// absolute path with `..` and symlinks resolved.
struct Overlap {
    /// Each root's position and spelling, by its location.
    roots: HashMap<PathBuf, (usize, PathBuf)>,
    /// Each root's location, by its spelling.
    locations: HashMap<PathBuf, PathBuf>,
    /// The locations of the files claimed for formatting and of the paths
    /// reported unreadable so far.
    seen: Mutex<HashSet<PathBuf>>,
}

/// A path the walk found, placed under the first root that contains it.
struct Place<'w> {
    /// The root's position.
    index: usize,
    /// The root's absolute spelling.
    root: &'w Path,
    /// The path, spelled under the root.
    path: PathBuf,
    /// The path's location when roots overlap, for [`Walk::claim`].
    location: Option<PathBuf>,
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
        let depth = entry.depth();
        let named = depth == 0;
        let path = entry.into_path();
        let language = Language::from_path(&path);
        if language.is_none() && !named {
            return;
        }
        let Some(Place {
            index,
            root,
            path,
            location,
        }) = self.place(path, Some(depth))
        else {
            return;
        };
        let Some(language) = language else {
            let message = format!(
                "{}: unsupported file extension\n",
                display_path(self.cwd, &path)
            );
            return self.report(index, path, Report::failed(message));
        };
        // The settings come from the first root that contains the file, so
        // they don't depend on which root's walk finds it first.
        let dir = path.parent().unwrap_or(self.cwd);
        let target = if root == path { dir } else { root };
        let settings = match self.resolver.settings(dir, target) {
            Ok(settings) => settings,
            Err(error) => {
                if self.claim(location) {
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
        if selected && self.claim(location) {
            let report = format_file(self.cwd, &path, language, settings.options(), self.args);
            self.report(index, path, report);
        }
    }

    /// Places `path`, which the walk found under a root, under the first root
    /// named on the command line that contains it.
    ///
    /// `depth` is how far below its root the walk found `path`, if known.
    /// Without it, the nearest root that contains `path` stands in for the
    /// root whose walk found it.
    fn place(&self, path: PathBuf, depth: Option<usize>) -> Option<Place<'_>> {
        let (root, &walked) = if self.roots.len() == 1 {
            self.roots.iter().next()?
        } else if let Some(depth) = depth {
            self.roots.get_key_value(path.ancestors().nth(depth)?)?
        } else {
            path.ancestors()
                .find_map(|ancestor| self.roots.get_key_value(ancestor))?
        };
        let Some(overlap) = &self.overlap else {
            // No root contains another, so this one is the only one.
            return Some(Place {
                index: walked,
                root,
                path,
                location: None,
            });
        };
        // The walk doesn't follow symlinks below a root, so the rest of the
        // path names the file or directory where it is.
        let location = join(overlap.locations.get(root)?, path.strip_prefix(root).ok()?);
        let (first, (index, root)) = location
            .ancestors()
            .filter_map(|ancestor| overlap.roots.get_key_value(ancestor))
            .min_by_key(|(_, (index, _))| *index)?;
        let path = if *index == walked {
            path
        } else {
            join(root, location.strip_prefix(first).ok()?)
        };
        Some(Place {
            index: *index,
            root,
            path,
            location: Some(location),
        })
    }

    /// Whether this thread is the first to claim the path at `location`.
    fn claim(&self, location: Option<PathBuf>) -> bool {
        match (&self.overlap, location) {
            (Some(overlap), Some(location)) => lock(&overlap.seen).insert(location),
            _ => true,
        }
    }

    fn unreadable(&self, error: &ignore::Error) {
        let (path, cause) = error_path(error);
        let Some(path) = path else {
            let report = Report::unreadable(format!("{error}\n"));
            return self.report(usize::MAX, PathBuf::new(), report);
        };
        let (index, path) = match self.place(path.to_path_buf(), None) {
            Some(place) => {
                // The walks of two overlapping roots can both fail to read a
                // path.
                if !self.claim(place.location) {
                    return;
                }
                (place.index, place.path)
            }
            None => (usize::MAX, path.to_path_buf()),
        };
        let message = format!("{}: {cause}\n", display_path(self.cwd, &path));
        self.report(index, path, Report::unreadable(message));
    }

    fn report(&self, root: usize, path: PathBuf, report: Report) {
        lock(&self.reports).push(((root, path), report));
    }
}

/// What tells two named files apart: their device and inode.
#[cfg(unix)]
fn file_id(_path: &Path, metadata: &fs::Metadata) -> (u64, u64) {
    use std::os::unix::fs::MetadataExt as _;
    (metadata.dev(), metadata.ino())
}

/// What tells two named files apart: their location.
#[cfg(not(unix))]
fn file_id(path: &Path, _metadata: &fs::Metadata) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// `base` joined with `rest`, without the trailing separator that joining an
/// empty path adds.
fn join(base: &Path, rest: &Path) -> PathBuf {
    if rest.as_os_str().is_empty() {
        base.to_path_buf()
    } else {
        base.join(rest)
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
