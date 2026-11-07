//! The corpus sweep (`mise run hunt`): the oracle at several widths and with
//! comment probes injected at token boundaries, over the imported corpora.

use std::collections::{BTreeMap, HashSet};
use std::fmt::Write as _;
use std::fs;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};

use jolt_fmt::FormatOptions;

use crate::oracle::{CorpusLanguage, check_source, parse_facts};

/// Which trivia a probe insertion lands in.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ProbePosition {
    /// Trailing trivia of the token before the boundary.
    Trailing,
    /// Leading trivia of the token after the boundary.
    Leading,
}

/// One comment shape the sweep injects at a token boundary.
pub struct ProbeShape {
    /// The name `<path>:<name><n>` uses to replay a probe.
    pub name: &'static str,
    pub insert: &'static str,
    pub position: ProbePosition,
    /// Distinguishes this shape's sampled boundaries from the others'.
    pub seed: usize,
}

pub const PROBE_SHAPES: &[ProbeShape] = &[
    ProbeShape {
        name: "comment",
        insert: " /*hunt*/ ",
        position: ProbePosition::Trailing,
        seed: 2_654_435_761,
    },
    ProbeShape {
        name: "linecomment",
        insert: " //hunt\n",
        position: ProbePosition::Trailing,
        seed: 2_246_822_519,
    },
    ProbeShape {
        name: "ownline",
        insert: "\n/*hunt*/\n",
        position: ProbePosition::Leading,
        seed: 40_503,
    },
    ProbeShape {
        name: "ownlinecomment",
        insert: "\n//hunt\n",
        position: ProbePosition::Leading,
        seed: 374_761_393,
    },
];

/// Boundary count up to which a shape probes every boundary.
pub const SWEEP_EXHAUSTIVE_BOUNDARY_LIMIT: usize = 120;
/// Boundaries each shape samples in a larger file.
pub const SWEEP_SAMPLED_PROBES: usize = 3;
/// The widths the sweep formats every file at.
pub const SWEEP_WIDTHS: &[u16] = &[40, 80, 120];
/// Samples kept per summary key.
pub const MAX_SAMPLES_PER_KEY: usize = 20;

/// The boundary indices `shape` probes in a file with `boundaries` token
/// boundaries.
#[must_use]
pub fn probe_boundaries(shape: &ProbeShape, source: &str, boundaries: usize) -> Vec<usize> {
    if boundaries == 0 {
        return Vec::new();
    }
    if boundaries <= SWEEP_EXHAUSTIVE_BOUNDARY_LIMIT {
        return (0..boundaries).collect();
    }
    (0..SWEEP_SAMPLED_PROBES)
        .map(|probe| {
            (probe + 1).wrapping_mul(shape.seed.wrapping_mul(source.len() + 17)) % boundaries
        })
        .collect()
}

/// Parses `<shape><n>` (longest shape name first).
#[must_use]
pub fn parse_probe_argument(probe: &str) -> Option<(&'static ProbeShape, usize)> {
    let mut shapes: Vec<&ProbeShape> = PROBE_SHAPES.iter().collect();
    shapes.sort_by_key(|shape| std::cmp::Reverse(shape.name.len()));
    shapes.into_iter().find_map(|shape| {
        let index = probe.strip_prefix(shape.name)?.parse().ok()?;
        Some((shape, index))
    })
}

#[must_use]
pub fn probed_source(source: &str, boundary: usize, shape: &ProbeShape) -> String {
    let mut probed = String::with_capacity(source.len() + shape.insert.len());
    probed.push_str(&source[..boundary]);
    probed.push_str(shape.insert);
    probed.push_str(&source[boundary..]);
    probed
}

/// One failure, labelled `<variant>:<check>`.
pub struct SweepFailure {
    pub category: String,
    pub detail: String,
}

impl SweepFailure {
    /// The category with the probe index stripped.
    #[must_use]
    pub fn summary_key(&self) -> String {
        let (probe, check) = self
            .category
            .split_once(':')
            .unwrap_or((self.category.as_str(), ""));
        let shape = probe.trim_end_matches(|ch: char| ch.is_ascii_digit());
        format!("{shape}:{check}")
    }
}

/// Checks one variant (already probed) at `options`.
#[must_use]
pub fn sweep_variant<C: CorpusLanguage>(
    language: &C,
    source: &str,
    options: &FormatOptions,
    label: &str,
) -> Vec<SweepFailure> {
    check_source(language, source, options)
        .into_iter()
        .map(|(check, detail)| SweepFailure {
            category: format!("{label}:{check}"),
            detail,
        })
        .collect()
}

/// Every sweep check for one file: each width, then each probe shape at
/// width 80. A probe that makes clean source dirty is skipped (it landed in
/// something other than trivia).
#[must_use]
pub fn sweep_source<C: CorpusLanguage>(language: &C, source: &str) -> Vec<SweepFailure> {
    let mut failures = Vec::new();
    for &width in SWEEP_WIDTHS {
        let options = FormatOptions {
            line_width: width,
            ..FormatOptions::default()
        };
        failures.extend(sweep_variant(
            language,
            source,
            &options,
            &format!("w{width}"),
        ));
    }
    let boundaries = language.token_end_offsets(source);
    let clean = parse_facts(language, source).diagnostics.is_empty();
    let options = FormatOptions::default();
    for shape in PROBE_SHAPES {
        for (probe, &boundary) in probe_boundaries(shape, source, boundaries.len())
            .iter()
            .enumerate()
        {
            let probed = probed_source(source, boundaries[boundary], shape);
            if clean && !parse_facts(language, &probed).diagnostics.is_empty() {
                continue;
            }
            failures.extend(sweep_variant(
                language,
                &probed,
                &options,
                &format!("{}{probe}", shape.name),
            ));
        }
    }
    failures
}

/// Runs `check`, turning a panic into a `panic` failure.
pub fn catching(path: &Path, check: impl FnOnce() -> Vec<SweepFailure>) -> Vec<SweepFailure> {
    catch_unwind(AssertUnwindSafe(check)).unwrap_or_else(|payload| {
        let message = payload
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| payload.downcast_ref::<&str>().copied())
            .unwrap_or("<non-string panic>");
        vec![SweepFailure {
            category: "panic".to_owned(),
            detail: format!("{}: {message}", path.display()),
        }]
    })
}

/// What a sweep run needs beyond its language.
pub struct CorpusSweep<'a> {
    /// Directories under `tools/import/.imports`.
    pub suites: &'a [&'a str],
    pub extensions: &'a [&'a str],
    /// Directory under `target/hunt`.
    pub out_dir: &'a str,
    /// Failure categories already known to fire.
    pub known_open: &'a [&'a str],
}

#[derive(Default)]
struct Findings {
    counts: BTreeMap<String, usize>,
    retained: BTreeMap<String, usize>,
    samples: Vec<(PathBuf, SweepFailure)>,
}

impl Findings {
    fn push(&mut self, path: &Path, failure: SweepFailure) {
        let key = failure.summary_key();
        *self.counts.entry(key.clone()).or_default() += 1;
        let retained = self.retained.entry(key).or_default();
        if *retained < MAX_SAMPLES_PER_KEY {
            *retained += 1;
            self.samples.push((path.to_path_buf(), failure));
        }
    }

    fn merge(&mut self, other: Self) {
        for (key, count) in other.counts {
            *self.counts.entry(key).or_default() += count;
        }
        for (path, failure) in other.samples {
            let key = failure.summary_key();
            let retained = self.retained.entry(key).or_default();
            if *retained < MAX_SAMPLES_PER_KEY {
                *retained += 1;
                self.samples.push((path, failure));
            }
        }
    }
}

fn collect_files(dir: &Path, extensions: &[&str], files: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_files(&path, extensions, files);
        } else if path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extensions.contains(&extension))
        {
            files.push(path);
        }
    }
}

/// Runs the sweep and writes a bounded report under `target/hunt`, returning
/// the exit code: non-zero when a category is outside `known_open` or a
/// `known_open` entry no longer fires.
///
/// # Panics
///
/// When a suite directory is missing or the report cannot be written.
pub fn run_corpus_sweep<C: CorpusLanguage>(
    language: &C,
    repo_root: &Path,
    sweep: &CorpusSweep<'_>,
) -> i32 {
    std::panic::set_hook(Box::new(|_| {}));
    let imports = repo_root.join("tools/import/.imports");
    let mut files = Vec::new();
    for suite in sweep.suites {
        let dir = imports.join(suite);
        assert!(dir.is_dir(), "missing corpus suite {}", dir.display());
        collect_files(&dir, sweep.extensions, &mut files);
    }
    files.sort();
    let out_dir = repo_root.join("target/hunt").join(sweep.out_dir);
    let _ = fs::remove_dir_all(&out_dir);
    fs::create_dir_all(&out_dir).expect("create hunt output directory");

    let (mut findings, mut slow) = sweep_files(language, &files);

    let mut report = write_report(&mut findings, &mut slow, &out_dir);

    let observed: HashSet<&str> = findings.counts.keys().map(String::as_str).collect();
    let unexpected: Vec<&str> = observed
        .iter()
        .copied()
        .filter(|key| !sweep.known_open.contains(key))
        .collect();
    let stale: Vec<&str> = sweep
        .known_open
        .iter()
        .copied()
        .filter(|key| !observed.contains(key))
        .collect();
    let mut verdict = String::new();
    if !unexpected.is_empty() {
        let _ = writeln!(
            verdict,
            "\nREGRESSION: categories outside KNOWN_OPEN: {unexpected:?}"
        );
    }
    if !stale.is_empty() {
        let _ = writeln!(
            verdict,
            "\nFIXED: KNOWN_OPEN entries that no longer fire: {stale:?}"
        );
    }
    if verdict.is_empty() {
        let _ = writeln!(
            verdict,
            "\nsweep matches the {} known-open categories",
            sweep.known_open.len()
        );
    }
    report.push_str(&verdict);
    fs::write(out_dir.join("report.txt"), &report).expect("write report");
    println!(
        "checked {} files, {} failures: {:?}",
        files.len(),
        findings.counts.values().sum::<usize>(),
        findings.counts
    );
    println!("report: {}", out_dir.join("report.txt").display());
    print!("{verdict}");
    i32::from(!(unexpected.is_empty() && stale.is_empty()))
}

/// Writes one repro per retained sample and renders the report body.
fn write_report(findings: &mut Findings, slow: &mut [(PathBuf, u128)], out_dir: &Path) -> String {
    slow.sort_by_key(|(_, ms)| std::cmp::Reverse(*ms));
    findings
        .samples
        .sort_by(|(left_path, left), (right_path, right)| {
            (&left.category, left_path).cmp(&(&right.category, right_path))
        });
    let mut report = String::new();
    for (path, failure) in &findings.samples {
        let repro_dir = out_dir.join(failure.summary_key().replace(':', "-"));
        fs::create_dir_all(&repro_dir).expect("create repro directory");
        if let (Ok(source), Some(name)) = (fs::read_to_string(path), path.file_name()) {
            fs::write(
                repro_dir.join(name),
                format!(
                    "// repro from: {} ({})\n{source}",
                    path.display(),
                    failure.category
                ),
            )
            .expect("write repro");
        }
        let _ = writeln!(
            report,
            "[{}] {}\n    {}",
            failure.category,
            path.display(),
            failure.detail.replace('\n', "\n    ")
        );
    }
    for (key, count) in &findings.counts {
        let elided = count - findings.retained.get(key).copied().unwrap_or_default();
        if elided > 0 {
            let _ = writeln!(
                report,
                "\nelided {elided} further [{key}] sample(s) of {count}"
            );
        }
    }
    let _ = writeln!(report, "\nsummary: {:?}", findings.counts);
    for (path, ms) in slow.iter().take(20) {
        let _ = writeln!(report, "slow: {ms}ms {}", path.display());
    }

    report
}

/// Sweeps `files` on every core, returning the findings and the slow files.
fn sweep_files<C: CorpusLanguage>(
    language: &C,
    files: &[PathBuf],
) -> (Findings, Vec<(PathBuf, u128)>) {
    let threads = std::thread::available_parallelism().map_or(4, usize::from);
    let next = std::sync::atomic::AtomicUsize::new(0);
    std::thread::scope(|scope| {
        let workers: Vec<_> = (0..threads)
            .map(|_| {
                scope.spawn(|| {
                    let mut findings = Findings::default();
                    let mut slow = Vec::new();
                    loop {
                        let index = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        let Some(path) = files.get(index) else {
                            break;
                        };
                        let started = std::time::Instant::now();
                        let failures = match fs::read_to_string(path) {
                            Ok(source) => catching(path, || sweep_source(language, &source)),
                            Err(error) => vec![SweepFailure {
                                category: "read-error".to_owned(),
                                detail: error.to_string(),
                            }],
                        };
                        for failure in failures {
                            findings.push(path, failure);
                        }
                        let elapsed = started.elapsed().as_millis();
                        if elapsed > 1000 {
                            slow.push((path.clone(), elapsed));
                        }
                    }
                    (findings, slow)
                })
            })
            .collect();
        let mut all = Findings::default();
        let mut all_slow = Vec::new();
        for worker in workers {
            let (findings, slow) = worker.join().expect("sweep worker panicked");
            all.merge(findings);
            all_slow.extend(slow);
        }
        (all, all_slow)
    })
}
