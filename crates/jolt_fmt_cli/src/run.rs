use std::{
    collections::HashSet,
    convert::Infallible,
    env,
    fmt::Write as _,
    fs,
    io::{self, Read as _, Write as _},
    num::NonZeroUsize,
    path::{Path, PathBuf},
    thread,
};

use jolt_diagnostics::Diagnostic;
use jolt_fmt_core::{FormatSinkResult, format_source_to_sink};
use jolt_fmt_ir::{RenderControl, RenderSink};
use jolt_text::{LineIndex, TextSize};
use rayon::prelude::*;

use crate::{
    args::{Cli, Command, FmtArgs},
    config::{CliError, ConfigResolver, absolutize, display_path},
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
            let mut resolver = ConfigResolver::new(
                cwd,
                root.clone(),
                args.format_options(),
                &args.include,
                &args.exclude,
                args.config.as_deref(),
                args.no_config,
            )?;
            let config = resolver.resolve_for_dir(&root)?;
            if seen.insert(path.clone()) {
                candidates.push(CandidateFile {
                    path,
                    language,
                    options: config.options,
                });
            }
            continue;
        }

        if path.is_dir() {
            let mut resolver = ConfigResolver::new(
                cwd,
                path.clone(),
                args.format_options(),
                &args.include,
                &args.exclude,
                args.config.as_deref(),
                args.no_config,
            )?;
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
    let mut resolver = ConfigResolver::new(
        cwd,
        cwd.to_path_buf(),
        args.format_options(),
        &args.include,
        &args.exclude,
        args.config.as_deref(),
        args.no_config,
    )?;
    let config = resolver.resolve_for_dir(&pseudo_parent)?;

    let mut source = String::new();
    io::stdin()
        .read_to_string(&mut source)
        .map_err(|error| CliError::new(format!("failed to read stdin as UTF-8: {error}")))?;

    let label = args
        .stdin_filename
        .as_deref()
        .map_or_else(|| "<stdin>".to_owned(), |path| path.display().to_string());

    if args.check {
        let mut sink = CompareSink::new(&source);
        let result = format_source_to_sink(&source, language, &config.options, &mut sink);
        emit_diagnostics(&label, &source, result_diagnostics(&result))?;

        if matches!(result, FormatSinkResult::Blocked { .. }) {
            return Err(CliError::new(format_check_summary(FormatRunStats {
                total: 1,
                failed: 1,
                changed: 0,
            })));
        }

        if sink.is_changed() {
            println!("{label}");
            return Err(CliError::new(format_check_summary(FormatRunStats {
                total: 1,
                failed: 0,
                changed: 1,
            })));
        }
        return Ok(());
    }

    let mut sink = BufferedIoSink::new(io::stdout().lock());
    let result = format_source_to_sink(&source, language, &config.options, &mut sink);
    emit_diagnostics(&label, &source, result_diagnostics(&result))?;
    match result {
        FormatSinkResult::Complete => {}
        FormatSinkResult::Blocked { .. } => return Err(CliError::new("formatting failed")),
        FormatSinkResult::Halted => {
            return Err(CliError::new("formatting halted before writing stdout"));
        }
        FormatSinkResult::SinkError { .. } => {
            unreachable!("buffered stdout sink cannot fail while rendering");
        }
    }
    if let Err(error) = sink.commit() {
        return Err(CliError::new(format!("failed to write stdout: {error}")));
    }

    Ok(())
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
        candidate.options,
        check,
    )
}

fn format_file(
    cwd: &Path,
    path: &Path,
    language: jolt_fmt_core::Language,
    options: jolt_fmt_core::FormatOptions,
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
    if check {
        return format_file_check(&label, &source, language, options);
    }

    format_file_write(cwd, path, &label, &source, language, options)
}

fn format_file_check(
    label: &str,
    source: &str,
    language: jolt_fmt_core::Language,
    options: jolt_fmt_core::FormatOptions,
) -> FileFormatResult {
    let mut sink = CompareSink::new(source);
    let result = format_source_to_sink(source, language, &options, &mut sink);
    let diagnostics = diagnostics_text(label, source, result_diagnostics(&result));

    if matches!(result, FormatSinkResult::Blocked { .. }) {
        return FileFormatResult {
            outcome: FileFormatOutcome {
                failed: true,
                changed: false,
            },
            diagnostics,
            check_output: String::new(),
            error: None,
        };
    }

    if !sink.is_changed() {
        return FileFormatResult {
            outcome: FileFormatOutcome::default(),
            diagnostics,
            check_output: String::new(),
            error: None,
        };
    }

    FileFormatResult {
        outcome: FileFormatOutcome {
            failed: false,
            changed: true,
        },
        diagnostics,
        check_output: format!("{label}\n"),
        error: None,
    }
}

fn format_file_write(
    cwd: &Path,
    path: &Path,
    label: &str,
    source: &str,
    language: jolt_fmt_core::Language,
    options: jolt_fmt_core::FormatOptions,
) -> FileFormatResult {
    let mut sink = BufferedFileSink::new(path, source);
    let result = format_source_to_sink(source, language, &options, &mut sink);
    let diagnostics = diagnostics_text(label, source, result_diagnostics(&result));

    finish_file_write(cwd, path, diagnostics, &result, sink)
}

fn finish_file_write(
    cwd: &Path,
    path: &Path,
    diagnostics: String,
    result: &FormatSinkResult<Infallible>,
    sink: BufferedFileSink<'_>,
) -> FileFormatResult {
    if matches!(result, FormatSinkResult::Blocked { .. }) {
        return FileFormatResult {
            outcome: FileFormatOutcome {
                failed: true,
                changed: false,
            },
            diagnostics,
            check_output: String::new(),
            error: None,
        };
    }

    if matches!(result, FormatSinkResult::Halted) {
        return FileFormatResult {
            outcome: FileFormatOutcome {
                failed: true,
                changed: false,
            },
            diagnostics,
            check_output: String::new(),
            error: Some(format!(
                "{}: formatting halted unexpectedly",
                display_path(cwd, path)
            )),
        };
    }

    if matches!(result, FormatSinkResult::SinkError { .. }) {
        unreachable!("buffered file sink cannot fail while rendering");
    }

    let changed = sink.is_changed();
    if changed && let Err(error) = sink.commit() {
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
            changed,
        },
        diagnostics,
        check_output: String::new(),
        error: None,
    }
}

struct BufferedIoSink<W> {
    writer: W,
    contents: String,
}

impl<W> BufferedIoSink<W> {
    fn new(writer: W) -> Self {
        Self {
            writer,
            contents: String::new(),
        }
    }
}

impl<W: io::Write> BufferedIoSink<W> {
    fn commit(mut self) -> io::Result<()> {
        self.writer.write_all(self.contents.as_bytes())
    }
}

impl<W> RenderSink for BufferedIoSink<W> {
    type Error = Infallible;

    fn write_str(&mut self, text: &str) -> Result<RenderControl, Self::Error> {
        self.contents.push_str(text);
        Ok(RenderControl::Continue)
    }
}

struct CompareSink<'a> {
    expected: &'a [u8],
    offset: usize,
    matches: bool,
}

impl<'a> CompareSink<'a> {
    fn new(expected: &'a str) -> Self {
        Self {
            expected: expected.as_bytes(),
            offset: 0,
            matches: true,
        }
    }

    fn is_changed(&self) -> bool {
        !self.matches || self.offset != self.expected.len()
    }
}

impl RenderSink for CompareSink<'_> {
    type Error = Infallible;

    fn write_str(&mut self, text: &str) -> Result<RenderControl, Self::Error> {
        if !self.matches {
            self.offset += text.len();
            return Ok(RenderControl::Halt);
        }

        compare_chunk(self.expected, &mut self.offset, &mut self.matches, text);

        if self.matches {
            Ok(RenderControl::Continue)
        } else {
            Ok(RenderControl::Halt)
        }
    }
}

fn compare_chunk(expected: &[u8], offset: &mut usize, matches: &mut bool, text: &str) {
    if !*matches {
        *offset += text.len();
        return;
    }

    let bytes = text.as_bytes();
    let remaining = expected.get(*offset..).unwrap_or_default();
    let overlap = remaining.len().min(bytes.len());
    if remaining[..overlap] != bytes[..overlap] || bytes.len() > remaining.len() {
        *matches = false;
    }
    *offset += bytes.len();
}

struct BufferedFileSink<'a> {
    path: &'a Path,
    expected: &'a [u8],
    offset: usize,
    matches: bool,
    contents: String,
}

impl<'a> BufferedFileSink<'a> {
    fn new(path: &'a Path, expected: &'a str) -> Self {
        Self {
            path,
            expected: expected.as_bytes(),
            offset: 0,
            matches: true,
            contents: String::new(),
        }
    }

    fn is_changed(&self) -> bool {
        !self.matches || self.offset != self.expected.len()
    }

    fn commit(self) -> io::Result<()> {
        fs::write(self.path, self.contents)
    }
}

impl RenderSink for BufferedFileSink<'_> {
    type Error = Infallible;

    fn write_str(&mut self, text: &str) -> Result<RenderControl, Self::Error> {
        self.contents.push_str(text);
        compare_chunk(self.expected, &mut self.offset, &mut self.matches, text);
        Ok(RenderControl::Continue)
    }
}

fn emit_diagnostics(label: &str, source: &str, diagnostics: &[Diagnostic]) -> Result<(), CliError> {
    let mut stderr = io::stderr().lock();
    stderr
        .write_all(diagnostics_text(label, source, diagnostics).as_bytes())
        .map_err(|error| CliError::new(format!("failed to write diagnostics: {error}")))
}

fn diagnostics_text(label: &str, source: &str, diagnostics: &[Diagnostic]) -> String {
    if diagnostics.is_empty() {
        return String::new();
    }

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

fn result_diagnostics<E>(result: &FormatSinkResult<E>) -> &[Diagnostic] {
    match result {
        FormatSinkResult::Blocked { diagnostics } => diagnostics,
        FormatSinkResult::Complete
        | FormatSinkResult::Halted
        | FormatSinkResult::SinkError { .. } => &[],
    }
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
