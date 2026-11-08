//! The corpus sweep (`mise run hunt`): the oracle at several widths and with
//! comment probes injected at token boundaries, over the imported corpora.

use std::collections::{BTreeMap, HashSet};
use std::fmt::Write as _;
use std::fs;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::time::Instant;

use jolt_fmt::FormatOptions;

use crate::oracle::{CorpusLanguage, check_source, parse_facts};
use crate::{files_with_extensions, options, par_fold};

/// One comment shape the sweep injects at a token boundary.
pub struct ProbeShape {
    /// The name `<path>:<name><n>` uses to replay a probe.
    pub name: &'static str,
    pub insert: &'static str,
    /// Distinguishes this shape's sampled boundaries from the others'.
    seed: usize,
}

/// The probe shapes. A shape that starts on the boundary's own line lands in
/// the trailing trivia of the token before it; one that starts with a line
/// break lands in the leading trivia of the token after it.
pub const PROBE_SHAPES: &[ProbeShape] = &[
    ProbeShape {
        name: "comment",
        insert: " /*hunt*/ ",
        seed: 2_654_435_761,
    },
    ProbeShape {
        name: "linecomment",
        insert: " //hunt\n",
        seed: 2_246_822_519,
    },
    ProbeShape {
        name: "ownline",
        insert: "\n/*hunt*/\n",
        seed: 40_503,
    },
    ProbeShape {
        name: "ownlinecomment",
        insert: "\n//hunt\n",
        seed: 374_761_393,
    },
];

/// Boundary count up to which a sampled suite probes every boundary.
const EXHAUSTIVE_BOUNDARY_LIMIT: usize = 120;
/// Boundaries each shape samples in a larger file of a sampled suite.
const SAMPLED_PROBES: usize = 8;
/// The widths the sweep formats every file at. A sampled suite probes at
/// [`PROBE_WIDTH`] only; an exhaustive one probes at every width.
const WIDTHS: &[u16] = &[40, 80, 120];
const PROBE_WIDTH: u16 = 80;
/// Files up to this size in an exhaustive suite get every probe at every
/// width and every one-token deletion; larger ones are sampled.
const EXHAUSTIVE_MAX_BYTES: usize = 4096;
/// Samples kept per summary key.
const MAX_SAMPLES_PER_KEY: usize = 20;

/// How thoroughly the sweep checks a suite.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Depth {
    /// Every boundary of small files; `SAMPLED_PROBES` per shape
    /// otherwise; probes at `PROBE_WIDTH`.
    Sampled,
    /// Every boundary at every width (and every one-token deletion when
    /// mutating) for files up to `EXHAUSTIVE_MAX_BYTES`; larger files
    /// are sampled.
    Exhaustive,
}

impl Depth {
    /// The depth `source` gets in a suite of this depth.
    #[must_use]
    pub const fn for_source(self, source: &str) -> Self {
        match self {
            Self::Exhaustive if source.len() <= EXHAUSTIVE_MAX_BYTES => Self::Exhaustive,
            _ => Self::Sampled,
        }
    }
}

/// One suite of the sweep: a directory relative to the repository root.
pub struct Suite<'a> {
    pub dir: &'a str,
    pub depth: Depth,
}

/// The boundary indices `shape` probes in a file with `boundaries` token
/// boundaries.
fn probe_boundaries(
    shape: &ProbeShape,
    source: &str,
    boundaries: usize,
    depth: Depth,
) -> Vec<usize> {
    if boundaries == 0 {
        return Vec::new();
    }
    if depth.for_source(source) == Depth::Exhaustive || boundaries <= EXHAUSTIVE_BOUNDARY_LIMIT {
        return (0..boundaries).collect();
    }
    (0..SAMPLED_PROBES)
        .map(|probe| {
            (probe + 1).wrapping_mul(shape.seed.wrapping_mul(source.len() + 17)) % boundaries
        })
        .collect()
}

/// One checked variant of a file, as `jcheck` replays it: `<shape><n>`, or
/// `<shape><n>@<width>` at a width other than `PROBE_WIDTH`, or
/// `delete<n>` (the `n`th non-empty token removed).
#[derive(Clone, Copy)]
pub enum Variant {
    Probe {
        shape: &'static ProbeShape,
        index: usize,
        width: u16,
    },
    Delete {
        index: usize,
    },
}

impl Variant {
    /// Parses a variant label (longest shape name first).
    #[must_use]
    pub fn parse(label: &str) -> Option<Self> {
        if let Some(index) = label.strip_prefix("delete") {
            return Some(Self::Delete {
                index: index.parse().ok()?,
            });
        }
        let (probe, width) = match label.split_once('@') {
            Some((probe, width)) => (probe, width.parse().ok()?),
            None => (label, PROBE_WIDTH),
        };
        let mut shapes: Vec<&'static ProbeShape> = PROBE_SHAPES.iter().collect();
        shapes.sort_by_key(|shape| std::cmp::Reverse(shape.name.len()));
        shapes.into_iter().find_map(|shape| {
            let index = probe.strip_prefix(shape.name)?.parse().ok()?;
            Some(Self::Probe {
                shape,
                index,
                width,
            })
        })
    }

    #[must_use]
    pub fn label(self) -> String {
        match self {
            Self::Probe {
                shape,
                index,
                width,
            } if width == PROBE_WIDTH => format!("{}{index}", shape.name),
            Self::Probe {
                shape,
                index,
                width,
            } => format!("{}{index}@{width}", shape.name),
            Self::Delete { index } => format!("delete{index}"),
        }
    }

    /// The options the sweep checks the variant at.
    #[must_use]
    pub fn options(self) -> FormatOptions {
        match self {
            Self::Probe { width, .. } => options(width),
            Self::Delete { .. } => options(PROBE_WIDTH),
        }
    }

    /// The variant's source, or `None` when its index is out of range.
    #[must_use]
    pub fn source<C: CorpusLanguage>(
        self,
        language: &C,
        source: &str,
        depth: Depth,
    ) -> Option<String> {
        match self {
            Self::Probe { shape, index, .. } => {
                let boundaries = language.token_end_offsets(source);
                let selected = probe_boundaries(shape, source, boundaries.len(), depth);
                let boundary = boundaries[*selected.get(index)?];
                Some(probed_source(source, boundary, shape))
            }
            Self::Delete { index } => {
                let (start, end) = *language.token_ranges(source).get(index)?;
                Some(format!("{}{}", &source[..start], &source[end..]))
            }
        }
    }
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
    /// The category with the variant's index stripped.
    fn summary_key(&self) -> String {
        let (label, check) = self
            .category
            .split_once(':')
            .unwrap_or((self.category.as_str(), ""));
        let (variant, width) = match label.split_once('@') {
            Some((variant, _)) => (variant, &label[variant.len()..]),
            None => (label, ""),
        };
        let shape = variant.trim_end_matches(|ch: char| ch.is_ascii_digit());
        format!("{shape}{width}:{check}")
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

/// Every sweep check for one file: each width, then each probe shape, then
/// (in an exhaustive suite) each one-token deletion. A probe that makes clean
/// source dirty is skipped (it landed in something other than trivia).
#[must_use]
pub fn sweep_source<C: CorpusLanguage>(
    language: &C,
    source: &str,
    depth: Depth,
) -> Vec<SweepFailure> {
    let depth = depth.for_source(source);
    let mut failures = Vec::new();
    for &width in WIDTHS {
        failures.extend(sweep_variant(
            language,
            source,
            &options(width),
            &format!("w{width}"),
        ));
    }
    let boundaries = language.token_end_offsets(source);
    let clean = parse_facts(language, source).diagnostics.is_empty();
    let widths = match depth {
        Depth::Sampled => &[PROBE_WIDTH][..],
        Depth::Exhaustive => WIDTHS,
    };
    for shape in PROBE_SHAPES {
        let selected = probe_boundaries(shape, source, boundaries.len(), depth);
        for (index, &boundary) in selected.iter().enumerate() {
            let probed = probed_source(source, boundaries[boundary], shape);
            if clean && !parse_facts(language, &probed).diagnostics.is_empty() {
                continue;
            }
            for &width in widths {
                let variant = Variant::Probe {
                    shape,
                    index,
                    width,
                };
                failures.extend(sweep_variant(
                    language,
                    &probed,
                    &options(width),
                    &variant.label(),
                ));
            }
        }
    }
    if depth == Depth::Exhaustive {
        for (index, &(start, end)) in language.token_ranges(source).iter().enumerate() {
            let deleted = format!("{}{}", &source[..start], &source[end..]);
            failures.extend(sweep_variant(
                language,
                &deleted,
                &options(PROBE_WIDTH),
                &Variant::Delete { index }.label(),
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
    pub suites: &'a [Suite<'a>],
    /// Failure categories already known to fire.
    pub known_open: &'a [&'a str],
}

#[derive(Default)]
struct Findings {
    counts: BTreeMap<String, usize>,
    by_file: BTreeMap<PathBuf, usize>,
    retained: BTreeMap<String, usize>,
    samples: Vec<(PathBuf, SweepFailure)>,
}

impl Findings {
    fn push(&mut self, path: &Path, failure: SweepFailure) {
        let key = failure.summary_key();
        *self.counts.entry(key.clone()).or_default() += 1;
        *self.by_file.entry(path.to_path_buf()).or_default() += 1;
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
        for (path, count) in other.by_file {
            *self.by_file.entry(path).or_default() += count;
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

/// Runs the sweep over the files with the language's extension and writes a
/// bounded report under `target/hunt/<language>`, returning
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
    let mut files = Vec::new();
    for suite in sweep.suites {
        let dir = repo_root.join(suite.dir);
        assert!(dir.is_dir(), "missing corpus suite {}", dir.display());
        let found = files_with_extensions(&dir, &[language.extension()]);
        files.extend(found.into_iter().map(|path| (path, suite.depth)));
    }
    files.sort_by(|left, right| left.0.cmp(&right.0));
    let out_dir = repo_root
        .join("target/hunt")
        .join(language.name().to_ascii_lowercase());
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
    let mut by_file: Vec<(&PathBuf, &usize)> = findings.by_file.iter().collect();
    by_file.sort_by_key(|&(_, count)| std::cmp::Reverse(*count));
    for (path, count) in by_file.into_iter().take(40) {
        let _ = writeln!(report, "failures: {count} {}", path.display());
    }
    for (path, ms) in slow.iter().take(20) {
        let _ = writeln!(report, "slow: {ms}ms {}", path.display());
    }

    report
}

/// Sweeps `files` on every core, returning the findings and the slow files.
fn sweep_files<C: CorpusLanguage>(
    language: &C,
    files: &[(PathBuf, Depth)],
) -> (Findings, Vec<(PathBuf, u128)>) {
    par_fold(
        files,
        |(findings, slow): &mut (Findings, Vec<(PathBuf, u128)>), (path, depth)| {
            let started = Instant::now();
            let failures = match fs::read_to_string(path) {
                Ok(source) => catching(path, || sweep_source(language, &source, *depth)),
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
        },
        |(findings, slow), (other_findings, other_slow)| {
            findings.merge(other_findings);
            slow.extend(other_slow);
        },
    )
}
