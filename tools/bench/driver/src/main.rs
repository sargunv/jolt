//! Single-threaded benchmark driver for formatter architecture measurements.

use std::env;
use std::fs;
use std::hint::black_box;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Instant;

use jolt_fmt_ir::{DocArenaMetrics, FormatOptions, FormatSinkResult, RenderControl, RenderSink};
use jolt_syntax::SyntaxTreeMetrics;
use serde::Serialize;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("jolt_bench_driver: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let args = Args::parse()?;
    let sources = Sources::read(&args.corpus, args.language)?;
    let structural =
        if matches!(args.measurement, Measurement::Timing) && matches!(args.mode, Mode::Format) {
            Some(collect_structural_metrics(&sources, args.language)?)
        } else {
            None
        };

    let mut operation = make_operation(&sources, args.language, args.mode)?;
    let output = match args.measurement {
        Measurement::Timing => {
            for _ in 0..args.warmups {
                black_box(operation()?);
            }
            let mut samples_ns = Vec::with_capacity(args.samples);
            for _ in 0..args.samples {
                let start = Instant::now();
                let rendered_bytes = operation()?;
                samples_ns.push(duration_ns(start.elapsed()));
                black_box(rendered_bytes);
            }
            Output {
                schema_version: 1,
                language: args.language,
                mode: args.mode,
                measurement: args.measurement,
                corpus: CorpusMetrics {
                    files: sources.items.len(),
                    source_bytes: sources.source_bytes,
                },
                timing: Some(TimingOutput {
                    warmups: args.warmups,
                    samples_ns,
                }),
                allocations: None,
                syntax: structural.map(|metrics| metrics.syntax),
                document: structural.map(|metrics| metrics.document),
            }
        }
        Measurement::Allocations => allocation_output(&args, &sources, &mut operation)?,
        Measurement::Memory => {
            black_box(operation()?);
            Output {
                schema_version: 1,
                language: args.language,
                mode: args.mode,
                measurement: args.measurement,
                corpus: CorpusMetrics {
                    files: sources.items.len(),
                    source_bytes: sources.source_bytes,
                },
                timing: None,
                allocations: None,
                syntax: None,
                document: None,
            }
        }
    };

    serde_json::to_writer(std::io::stdout(), &output)
        .map_err(|error| format!("could not write JSON: {error}"))?;
    println!();
    Ok(())
}

fn duration_ns(duration: std::time::Duration) -> u64 {
    duration
        .as_nanos()
        .try_into()
        .expect("a benchmark sample duration fits u64 nanoseconds")
}

#[cfg(feature = "allocations")]
fn allocation_output(
    args: &Args,
    sources: &Sources,
    operation: &mut dyn FnMut() -> Result<usize, String>,
) -> Result<Output, String> {
    for _ in 0..args.warmups {
        black_box(operation()?);
    }

    // Initialize allocation-counter's thread-local state outside samples.
    black_box(allocation_counter::measure(|| {}));
    let mut samples = Vec::with_capacity(args.samples);
    for _ in 0..args.samples {
        let mut operation_result = Ok(0);
        let info = allocation_counter::measure(|| {
            operation_result = operation();
        });
        let rendered_bytes = operation_result?;
        black_box(rendered_bytes);
        samples.push(AllocationSample {
            count_total: info.count_total,
            count_max: info.count_max,
            bytes_total: info.bytes_total,
            bytes_max: info.bytes_max,
        });
    }

    Ok(Output {
        schema_version: 1,
        language: args.language,
        mode: args.mode,
        measurement: args.measurement,
        corpus: CorpusMetrics {
            files: sources.items.len(),
            source_bytes: sources.source_bytes,
        },
        timing: None,
        allocations: Some(AllocationOutput {
            warmups: args.warmups,
            samples,
        }),
        syntax: None,
        document: None,
    })
}

#[cfg(not(feature = "allocations"))]
fn allocation_output(
    _args: &Args,
    _sources: &Sources,
    _operation: &mut dyn FnMut() -> Result<usize, String>,
) -> Result<Output, String> {
    Err("allocation measurement requires a binary built with --features allocations".into())
}

type Operation<'source> = Box<dyn FnMut() -> Result<usize, String> + 'source>;

fn make_operation(
    sources: &Sources,
    language: Language,
    mode: Mode,
) -> Result<Operation<'_>, String> {
    match (language, mode) {
        (Language::Java, Mode::Parse) => Ok(Box::new(|| {
            for item in &sources.items {
                black_box(jolt_java_syntax::parse_compilation_unit(&item.text));
            }
            Ok(0)
        })),
        (Language::Kotlin, Mode::Parse) => Ok(Box::new(|| {
            for item in &sources.items {
                black_box(jolt_kotlin_syntax::parse_kotlin_file(&item.text));
            }
            Ok(0)
        })),
        (Language::Java, Mode::Format) => {
            let parses = sources
                .items
                .iter()
                .map(|item| jolt_java_syntax::parse_compilation_unit(&item.text))
                .collect::<Vec<_>>();
            ensure_java_parses(&parses)?;
            Ok(Box::new(move || {
                let mut sink = CountingSink::default();
                for parse in &parses {
                    let syntax = parse
                        .syntax()
                        .expect("parses were checked before measurement");
                    let (result, _) = jolt_java_fmt::benchmark_format_syntax_to_sink(
                        &syntax,
                        &FormatOptions::default(),
                        &mut sink,
                    );
                    ensure_complete(result)?;
                }
                Ok(sink.bytes)
            }))
        }
        (Language::Kotlin, Mode::Format) => {
            let parses = sources
                .items
                .iter()
                .map(|item| jolt_kotlin_syntax::parse_kotlin_file(&item.text))
                .collect::<Vec<_>>();
            ensure_kotlin_parses(&parses)?;
            Ok(Box::new(move || {
                let mut sink = CountingSink::default();
                for parse in &parses {
                    let syntax = parse
                        .syntax()
                        .expect("parses were checked before measurement");
                    let (result, _) = jolt_kotlin_fmt::benchmark_format_syntax_to_sink(
                        &syntax,
                        &FormatOptions::default(),
                        &mut sink,
                    );
                    ensure_complete(result)?;
                }
                Ok(sink.bytes)
            }))
        }
        (Language::Java, Mode::EndToEnd) => Ok(Box::new(|| {
            let mut sink = CountingSink::default();
            for item in &sources.items {
                ensure_complete(jolt_java_fmt::format_source_to_sink(
                    &item.text,
                    &FormatOptions::default(),
                    jolt_fmt_ir::SyntaxErrorPolicy::Reject,
                    &mut sink,
                ))?;
            }
            Ok(sink.bytes)
        })),
        (Language::Kotlin, Mode::EndToEnd) => Ok(Box::new(|| {
            let mut sink = CountingSink::default();
            for item in &sources.items {
                ensure_complete(jolt_kotlin_fmt::format_source_to_sink(
                    &item.text,
                    &FormatOptions::default(),
                    jolt_fmt_ir::SyntaxErrorPolicy::Reject,
                    &mut sink,
                ))?;
            }
            Ok(sink.bytes)
        })),
    }
}

fn ensure_complete(result: FormatSinkResult) -> Result<(), String> {
    match result {
        FormatSinkResult::Complete => Ok(()),
        FormatSinkResult::Halted => Err("formatter sink halted unexpectedly".into()),
        FormatSinkResult::Blocked { diagnostic } => {
            Err(format!("formatter was blocked: {diagnostic:?}"))
        }
    }
}

#[derive(Default)]
struct CountingSink {
    bytes: usize,
}

impl RenderSink for CountingSink {
    fn write_str(&mut self, text: &str) -> RenderControl {
        self.bytes += text.len();
        RenderControl::Continue
    }
}

fn collect_structural_metrics(
    sources: &Sources,
    language: Language,
) -> Result<StructuralMetrics, String> {
    match language {
        Language::Java => {
            let mut metrics = StructuralMetrics::default();
            for item in &sources.items {
                let parse = jolt_java_syntax::parse_compilation_unit(&item.text);
                metrics.syntax += parse
                    .benchmark_metrics()
                    .ok_or("Java parser did not produce a compilation unit")?;
                let syntax = parse
                    .syntax()
                    .ok_or("Java parser did not produce a compilation unit")?;
                let mut sink = CountingSink::default();
                let (result, document) = jolt_java_fmt::benchmark_format_syntax_to_sink(
                    &syntax,
                    &FormatOptions::default(),
                    &mut sink,
                );
                ensure_complete(result)?;
                metrics.document += document;
            }
            Ok(metrics)
        }
        Language::Kotlin => {
            let mut metrics = StructuralMetrics::default();
            for item in &sources.items {
                let parse = jolt_kotlin_syntax::parse_kotlin_file(&item.text);
                metrics.syntax += parse
                    .benchmark_metrics()
                    .ok_or("Kotlin parser did not produce a file")?;
                let syntax = parse
                    .syntax()
                    .ok_or("Kotlin parser did not produce a file")?;
                let mut sink = CountingSink::default();
                let (result, document) = jolt_kotlin_fmt::benchmark_format_syntax_to_sink(
                    &syntax,
                    &FormatOptions::default(),
                    &mut sink,
                );
                ensure_complete(result)?;
                metrics.document += document;
            }
            Ok(metrics)
        }
    }
}

fn ensure_java_parses(parses: &[jolt_java_syntax::JavaParse<'_>]) -> Result<(), String> {
    if parses.iter().all(|parse| parse.syntax().is_some()) {
        Ok(())
    } else {
        Err("Java parser did not produce a compilation unit for every source".into())
    }
}

fn ensure_kotlin_parses(parses: &[jolt_kotlin_syntax::KotlinParse<'_>]) -> Result<(), String> {
    if parses.iter().all(|parse| parse.syntax().is_some()) {
        Ok(())
    } else {
        Err("Kotlin parser did not produce a file for every source".into())
    }
}

#[derive(Clone, Copy, Default)]
struct StructuralMetrics {
    syntax: SyntaxMetrics,
    document: DocumentMetrics,
}

#[derive(Clone, Copy, Default, Serialize)]
struct SyntaxMetrics {
    nodes: usize,
    children: usize,
    tokens: usize,
    trivia: usize,
    logical_bytes: usize,
    reserved_bytes: usize,
}

impl std::ops::AddAssign<SyntaxTreeMetrics> for SyntaxMetrics {
    fn add_assign(&mut self, other: SyntaxTreeMetrics) {
        self.nodes += other.nodes;
        self.children += other.children;
        self.tokens += other.tokens;
        self.trivia += other.trivia;
        self.logical_bytes += other.logical_bytes;
        self.reserved_bytes += other.reserved_bytes;
    }
}

#[derive(Clone, Copy, Default, Serialize)]
struct DocumentMetrics {
    nodes: usize,
    children: usize,
    logical_bytes: usize,
    reserved_bytes: usize,
}

impl std::ops::AddAssign<DocArenaMetrics> for DocumentMetrics {
    fn add_assign(&mut self, other: DocArenaMetrics) {
        self.nodes += other.nodes;
        self.children += other.children;
        self.logical_bytes += other.logical_bytes;
        self.reserved_bytes += other.reserved_bytes;
    }
}

struct Sources {
    items: Vec<Source>,
    source_bytes: usize,
}

impl Sources {
    fn read(root: &Path, language: Language) -> Result<Self, String> {
        let extensions: &[&str] = match language {
            Language::Java => &["java"],
            Language::Kotlin => &["kt", "kts"],
        };
        let mut paths = Vec::new();
        collect_paths(root, extensions, &mut paths)?;
        paths.sort();
        if paths.is_empty() {
            return Err(format!(
                "corpus {} contains no {} files",
                root.display(),
                extensions
                    .iter()
                    .map(|extension| format!(".{extension}"))
                    .collect::<Vec<_>>()
                    .join(" or ")
            ));
        }
        let mut items = Vec::with_capacity(paths.len());
        let mut source_bytes = 0;
        for path in paths {
            let text = fs::read_to_string(&path)
                .map_err(|error| format!("could not read {}: {error}", path.display()))?;
            source_bytes += text.len();
            items.push(Source { text });
        }
        Ok(Self {
            items,
            source_bytes,
        })
    }
}

struct Source {
    text: String,
}

fn collect_paths(root: &Path, extensions: &[&str], paths: &mut Vec<PathBuf>) -> Result<(), String> {
    if root.is_file() {
        if root
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|extension| extensions.contains(&extension))
        {
            paths.push(root.to_owned());
        }
        return Ok(());
    }
    let entries = fs::read_dir(root)
        .map_err(|error| format!("could not read corpus {}: {error}", root.display()))?;
    for entry in entries {
        let entry = entry.map_err(|error| format!("could not read corpus entry: {error}"))?;
        let path = entry.path();
        if path.is_dir() {
            collect_paths(&path, extensions, paths)?;
        } else if path
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|extension| extensions.contains(&extension))
        {
            paths.push(path);
        }
    }
    Ok(())
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "kebab-case")]
enum Language {
    Java,
    Kotlin,
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "kebab-case")]
enum Mode {
    Parse,
    Format,
    EndToEnd,
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "kebab-case")]
enum Measurement {
    Timing,
    Allocations,
    Memory,
}

struct Args {
    corpus: PathBuf,
    language: Language,
    mode: Mode,
    measurement: Measurement,
    samples: usize,
    warmups: usize,
}

impl Args {
    fn parse() -> Result<Self, String> {
        let mut corpus = None;
        let mut language = None;
        let mut mode = None;
        let mut measurement = None;
        let mut samples = None;
        let mut warmups = None;
        let mut args = env::args().skip(1);
        while let Some(flag) = args.next() {
            let value = args
                .next()
                .ok_or_else(|| format!("missing value for {flag}"))?;
            match flag.as_str() {
                "--corpus" => corpus = Some(PathBuf::from(value)),
                "--language" => {
                    language = Some(match value.as_str() {
                        "java" => Language::Java,
                        "kotlin" => Language::Kotlin,
                        _ => return Err(format!("unsupported language: {value}")),
                    });
                }
                "--mode" => {
                    mode = Some(match value.as_str() {
                        "parse" => Mode::Parse,
                        "format" => Mode::Format,
                        "end-to-end" => Mode::EndToEnd,
                        _ => return Err(format!("unsupported mode: {value}")),
                    });
                }
                "--measurement" => {
                    measurement = Some(match value.as_str() {
                        "timing" => Measurement::Timing,
                        "allocations" => Measurement::Allocations,
                        "memory" => Measurement::Memory,
                        _ => return Err(format!("unsupported measurement: {value}")),
                    });
                }
                "--samples" => samples = Some(parse_count("samples", &value)?),
                "--warmups" => warmups = Some(parse_count("warmups", &value)?),
                _ => return Err(format!("unsupported argument: {flag}")),
            }
        }
        let measurement = measurement.ok_or("missing --measurement")?;
        let (samples, warmups) = if matches!(measurement, Measurement::Memory) {
            let samples = samples.unwrap_or(1);
            let warmups = warmups.unwrap_or(0);
            if samples != 1 || warmups != 0 {
                return Err("memory measurement requires exactly 1 sample and 0 warmups".into());
            }
            (samples, warmups)
        } else {
            let samples = samples.ok_or("missing --samples")?;
            if samples == 0 {
                return Err("--samples must be greater than zero".into());
            }
            (samples, warmups.ok_or("missing --warmups")?)
        };
        Ok(Self {
            corpus: corpus.ok_or("missing --corpus")?,
            language: language.ok_or("missing --language")?,
            mode: mode.ok_or("missing --mode")?,
            measurement,
            samples,
            warmups,
        })
    }
}

fn parse_count(name: &str, value: &str) -> Result<usize, String> {
    value
        .parse()
        .map_err(|error| format!("invalid --{name} value {value:?}: {error}"))
}

#[derive(Serialize)]
struct Output {
    schema_version: u8,
    language: Language,
    mode: Mode,
    measurement: Measurement,
    corpus: CorpusMetrics,
    #[serde(skip_serializing_if = "Option::is_none")]
    timing: Option<TimingOutput>,
    #[serde(skip_serializing_if = "Option::is_none")]
    allocations: Option<AllocationOutput>,
    #[serde(skip_serializing_if = "Option::is_none")]
    syntax: Option<SyntaxMetrics>,
    #[serde(skip_serializing_if = "Option::is_none")]
    document: Option<DocumentMetrics>,
}

#[derive(Serialize)]
struct CorpusMetrics {
    files: usize,
    source_bytes: usize,
}

#[derive(Serialize)]
struct TimingOutput {
    warmups: usize,
    samples_ns: Vec<u64>,
}

#[derive(Serialize)]
struct AllocationOutput {
    warmups: usize,
    samples: Vec<AllocationSample>,
}

#[derive(Serialize)]
struct AllocationSample {
    count_total: u64,
    count_max: u64,
    bytes_total: u64,
    bytes_max: u64,
}
