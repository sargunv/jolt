use std::{
    collections::HashSet,
    env,
    fmt::Write as _,
    fs,
    io::{self, Read as _, Write as _},
    num::NonZeroUsize,
    path::{Path, PathBuf},
    thread,
};

use jolt_fmt_core::{FormatStatus, LineIndex, TextSize, format_source};
use rayon::prelude::*;

use crate::{
    args::{Cli, Command, FmtArgs},
    config::{CliError, ConfigResolver, ResolvedConfig, absolutize, display_path},
    discover::{CandidateFile, detect_language, discover_files},
};

pub(crate) fn run(cli: Cli) -> Result<(), CliError> {
    match cli.command {
        Command::Fmt(args) => run_fmt(&args),
    }
}

fn run_fmt(args: &FmtArgs) -> Result<(), CliError> {
    let cwd = env::current_dir()
        .map_err(|error| CliError::new(format!("failed to read current directory: {error}")))?;

    if args.paths.iter().any(|path| path == Path::new("-")) {
        return run_stdin(&cwd, args);
    }

    if args.stdin_filename.is_some() {
        return Err(CliError::new(
            "--stdin-filename is only valid when formatting stdin with '-'",
        ));
    }

    let candidates = collect_candidates(&cwd, args)?;
    let results = format_candidates(&cwd, args, &candidates)?;

    let mut stats = FormatRunStats::default();
    for result in results {
        result.emit()?;
        stats.record(result.outcome);
    }

    if args.check && (stats.failed > 0 || stats.changed > 0) {
        return Err(CliError::new(format_check_summary(stats)));
    }

    if stats.failed > 0 {
        return Err(CliError::new("formatting failed"));
    }

    Ok(())
}

fn collect_candidates(cwd: &Path, args: &FmtArgs) -> Result<Vec<CandidateFile>, CliError> {
    let paths: Vec<PathBuf> = if args.paths.is_empty() {
        vec![cwd.to_path_buf()]
    } else {
        args.paths
            .iter()
            .map(|path| absolutize(cwd, path))
            .collect()
    };

    let mut candidates = Vec::new();
    let mut seen: HashSet<PathBuf> = HashSet::new();

    for path in paths {
        if path.is_file() {
            let language = detect_language(&path).unwrap_or(jolt_fmt_core::Language::Java);
            let root = path
                .parent()
                .map_or_else(|| cwd.to_path_buf(), Path::to_path_buf);
            let mut resolver = resolver_for(cwd, &root, args)?;
            let config = resolver.resolve_for_dir(&root)?;
            if seen.insert(path.clone()) {
                candidates.push(CandidateFile {
                    path,
                    language,
                    config,
                });
            }
            continue;
        }

        if path.is_dir() {
            let mut resolver = resolver_for(cwd, &path, args)?;
            for candidate in discover_files(&path, &mut resolver)? {
                if seen.insert(candidate.path.clone()) {
                    candidates.push(candidate);
                }
            }
            continue;
        }

        return Err(CliError::new(format!(
            "{}: path does not exist",
            display_path(cwd, &path)
        )));
    }

    Ok(candidates)
}

fn run_stdin(cwd: &Path, args: &FmtArgs) -> Result<(), CliError> {
    if args.paths.len() != 1 {
        return Err(CliError::new(
            "'-' cannot be combined with filesystem paths in milestone 10",
        ));
    }

    let language = match &args.stdin_filename {
        Some(path) => detect_language(path).ok_or_else(|| {
            CliError::new(format!(
                "{}: unsupported stdin filename extension",
                path.display()
            ))
        })?,
        None => jolt_fmt_core::Language::Java,
    };

    let pseudo_parent = args
        .stdin_filename
        .as_deref()
        .and_then(Path::parent)
        .map_or_else(|| cwd.to_path_buf(), |path| absolutize(cwd, path));
    let mut resolver = resolver_for(cwd, cwd, args)?;
    let config = resolver.resolve_for_dir(&pseudo_parent)?;

    let mut source = String::new();
    io::stdin()
        .read_to_string(&mut source)
        .map_err(|error| CliError::new(format!("failed to read stdin as UTF-8: {error}")))?;

    let result = format_source(&source, language, &config.options);
    let label = args
        .stdin_filename
        .as_deref()
        .map_or_else(|| "<stdin>".to_owned(), |path| path.display().to_string());
    emit_diagnostics(&label, &source, &result.diagnostics)?;

    let Some(formatted) = result.formatted_source else {
        if args.check {
            return Err(CliError::new(format_check_summary(FormatRunStats {
                total: 1,
                failed: 1,
                changed: 0,
            })));
        }
        return Err(CliError::new("formatting failed"));
    };

    if args.check {
        if result.status == FormatStatus::Formatted {
            println!("{label}");
            return Err(CliError::new(format_check_summary(FormatRunStats {
                total: 1,
                failed: 0,
                changed: 1,
            })));
        }
        return Ok(());
    }

    print!("{formatted}");
    Ok(())
}

fn resolver_for(
    cwd: &Path,
    invocation_root: &Path,
    args: &FmtArgs,
) -> Result<ConfigResolver, CliError> {
    ConfigResolver::new(
        cwd,
        invocation_root.to_path_buf(),
        args.format_options(),
        &args.include,
        &args.exclude,
        args.config.as_deref(),
        args.no_config,
    )
}

fn format_candidates(
    cwd: &Path,
    args: &FmtArgs,
    candidates: &[CandidateFile],
) -> Result<Vec<FileFormatResult>, CliError> {
    let threads = args.threads.unwrap_or_else(default_thread_count).get();
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .map_err(|error| CliError::new(format!("failed to start formatter workers: {error}")))?;

    Ok(pool.install(|| {
        candidates
            .par_iter()
            .map(|candidate| format_candidate(cwd, candidate, args.check))
            .collect()
    }))
}

fn default_thread_count() -> NonZeroUsize {
    thread::available_parallelism().unwrap_or(NonZeroUsize::MIN)
}

fn format_candidate(cwd: &Path, candidate: &CandidateFile, check: bool) -> FileFormatResult {
    format_file(
        cwd,
        &candidate.path,
        candidate.language,
        &candidate.config,
        check,
    )
}

fn format_file(
    cwd: &Path,
    path: &Path,
    language: jolt_fmt_core::Language,
    config: &ResolvedConfig,
    check: bool,
) -> FileFormatResult {
    let label = display_path(cwd, path);
    let source = match fs::read_to_string(path) {
        Ok(source) => source,
        Err(error) => {
            return FileFormatResult::failed_with_error(format!(
                "{}: failed to read file: {error}",
                display_path(cwd, path)
            ));
        }
    };
    let result = format_source(&source, language, &config.options);
    let diagnostics = diagnostics_text(&label, &source, &result.diagnostics);

    let Some(formatted) = result.formatted_source else {
        return FileFormatResult {
            outcome: FileFormatOutcome {
                failed: true,
                changed: false,
            },
            diagnostics,
            check_output: String::new(),
            error: None,
        };
    };

    if formatted == source {
        return FileFormatResult {
            outcome: FileFormatOutcome::default(),
            diagnostics,
            check_output: String::new(),
            error: None,
        };
    }

    if check {
        return FileFormatResult {
            outcome: FileFormatOutcome {
                failed: false,
                changed: true,
            },
            diagnostics,
            check_output: format!("{label}\n"),
            error: None,
        };
    }

    if let Err(error) = fs::write(path, formatted) {
        return FileFormatResult {
            outcome: FileFormatOutcome {
                failed: true,
                changed: false,
            },
            diagnostics,
            check_output: String::new(),
            error: Some(format!(
                "{}: failed to write formatted file: {error}",
                display_path(cwd, path)
            )),
        };
    }

    FileFormatResult {
        outcome: FileFormatOutcome {
            failed: false,
            changed: true,
        },
        diagnostics,
        check_output: String::new(),
        error: None,
    }
}

fn emit_diagnostics(
    label: &str,
    source: &str,
    diagnostics: &[jolt_fmt_core::Diagnostic],
) -> Result<(), CliError> {
    let mut stderr = io::stderr().lock();
    let line_index = LineIndex::new(source);

    for diagnostic in diagnostics {
        if let Some(range) = diagnostic.range {
            let position = line_index.line_col(TextSize::new(range.start().get()));
            writeln!(
                stderr,
                "{}:{}:{}: {}: {}",
                label,
                position.line + 1,
                position.column.get() + 1,
                diagnostic.code,
                diagnostic.message
            )
        } else {
            writeln!(
                stderr,
                "{}: {}: {}",
                label, diagnostic.code, diagnostic.message
            )
        }
        .map_err(|error| CliError::new(format!("failed to write diagnostics: {error}")))?;
    }

    Ok(())
}

fn diagnostics_text(
    label: &str,
    source: &str,
    diagnostics: &[jolt_fmt_core::Diagnostic],
) -> String {
    let mut text = String::new();
    let line_index = LineIndex::new(source);

    for diagnostic in diagnostics {
        if let Some(range) = diagnostic.range {
            let position = line_index.line_col(TextSize::new(range.start().get()));
            let _ = writeln!(
                text,
                "{}:{}:{}: {}: {}",
                label,
                position.line + 1,
                position.column.get() + 1,
                diagnostic.code,
                diagnostic.message
            );
        } else {
            let _ = writeln!(
                text,
                "{}: {}: {}",
                label, diagnostic.code, diagnostic.message
            );
        }
    }

    text
}

#[derive(Debug, Default)]
struct FileFormatResult {
    outcome: FileFormatOutcome,
    diagnostics: String,
    check_output: String,
    error: Option<String>,
}

impl FileFormatResult {
    fn failed_with_error(error: String) -> Self {
        Self {
            outcome: FileFormatOutcome {
                failed: true,
                changed: false,
            },
            diagnostics: String::new(),
            check_output: String::new(),
            error: Some(error),
        }
    }

    fn emit(&self) -> Result<(), CliError> {
        let mut stdout = io::stdout().lock();
        stdout
            .write_all(self.check_output.as_bytes())
            .map_err(|error| CliError::new(format!("failed to write check output: {error}")))?;

        let mut stderr = io::stderr().lock();
        stderr
            .write_all(self.diagnostics.as_bytes())
            .map_err(|error| CliError::new(format!("failed to write diagnostics: {error}")))?;
        if let Some(error) = &self.error {
            writeln!(stderr, "{error}")
                .map_err(|error| CliError::new(format!("failed to write diagnostics: {error}")))?;
        }

        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct FileFormatOutcome {
    failed: bool,
    changed: bool,
}

#[derive(Clone, Copy, Debug, Default)]
struct FormatRunStats {
    total: usize,
    failed: usize,
    changed: usize,
}

impl FormatRunStats {
    fn record(&mut self, outcome: FileFormatOutcome) {
        self.total += 1;
        self.failed += usize::from(outcome.failed);
        self.changed += usize::from(outcome.changed);
    }
}

fn format_check_summary(stats: FormatRunStats) -> String {
    match (stats.changed, stats.failed) {
        (changed, 0) => format!(
            "{changed} of {total} {files} {verb} not formatted",
            total = stats.total,
            files = plural(stats.total, "file", "files"),
            verb = if changed == 1 { "is" } else { "are" },
        ),
        (0, failed) => format!(
            "format check failed: {failed} of {total} {files} could not be formatted",
            total = stats.total,
            files = plural(stats.total, "file", "files"),
        ),
        (changed, failed) => format!(
            "format check failed: {changed} of {total} {files} {verb} not formatted; {failed} {failed_files} could not be formatted",
            total = stats.total,
            files = plural(stats.total, "file", "files"),
            verb = if changed == 1 { "is" } else { "are" },
            failed_files = plural(failed, "file", "files"),
        ),
    }
}

const fn plural(count: usize, singular: &'static str, plural: &'static str) -> &'static str {
    if count == 1 { singular } else { plural }
}
