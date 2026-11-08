//! Compares the formatter with expected outputs, such as the baseline
//! outputs `tools/baseline.sh` writes.
//!
//! - `jref <dir> [--line-width N] [--out DIR]`: checks `format(F) == F` for
//!   every `.java` file, and for the `formatted:` section of every `.snap`,
//!   under `<dir>`.
//! - `jref --input <in> --expected <out> [--line-width N] [--out DIR]`: checks
//!   `format(in) == out` file by file.
//!
//! Prints totals and writes each mismatch's actual output plus a summary to
//! `--out` (default `target/jref`). Inputs with syntax errors are skipped and
//! counted, and so are inputs without an expected output. Exits 1 when an
//! output differs or is missing, and 2 on a usage error.

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use jolt_fmt::FormatOptions;
use jolt_test_support::{files_with_extensions, par_fold, read_to_string, workspace_root};

#[path = "../tests/common/mod.rs"]
mod common;

const USAGE: &str =
    "usage: jref <dir> | jref --input <in> --expected <out> [--line-width N] [--out DIR]";

struct Args {
    cases: Cases,
    width: u16,
    out: PathBuf,
}

enum Cases {
    /// Every file under the directory should format to itself.
    Stable(PathBuf),
    /// Every file under `input` should format to its counterpart under
    /// `expected`.
    Expected { input: PathBuf, expected: PathBuf },
}

impl Args {
    fn parse() -> Result<Self, String> {
        let mut args = std::env::args().skip(1);
        let (mut dir, mut input, mut expected) = (None, None, None);
        let mut width = 80;
        let mut out = workspace_root(env!("CARGO_MANIFEST_DIR")).join("target/jref");
        while let Some(arg) = args.next() {
            let mut value = || args.next().ok_or_else(|| format!("{arg} needs a value"));
            match arg.as_str() {
                "--input" => input = Some(PathBuf::from(value()?)),
                "--expected" => expected = Some(PathBuf::from(value()?)),
                "--line-width" => {
                    width = value()?
                        .parse()
                        .map_err(|error| format!("--line-width: {error}"))?;
                }
                "--out" => out = PathBuf::from(value()?),
                flag if flag.starts_with('-') => return Err(format!("unknown flag {flag}")),
                _ if dir.is_some() => return Err(format!("unexpected argument {arg}")),
                _ => dir = Some(PathBuf::from(arg)),
            }
        }
        let cases = match (dir, input, expected) {
            (Some(dir), None, None) => Cases::Stable(dir),
            (None, Some(input), Some(expected)) => Cases::Expected { input, expected },
            _ => return Err("give a directory, or both --input and --expected".to_owned()),
        };
        Ok(Self { cases, width, out })
    }
}

struct Case {
    relative: PathBuf,
    input: String,
    /// `None` when the expected output is missing.
    expected: Option<String>,
}

impl Cases {
    fn collect(&self) -> Vec<Case> {
        let relative = |root: &Path, path: &Path| {
            path.strip_prefix(root)
                .expect("a collected file is under its root")
                .to_path_buf()
        };
        match self {
            Self::Stable(dir) => files_with_extensions(dir, &["java", "snap"])
                .into_iter()
                .filter_map(|path| {
                    let text = read_to_string(&path);
                    let text = if path.extension().is_some_and(|e| e == "snap") {
                        snapshot_output(&text)?
                    } else {
                        text
                    };
                    Some(Case {
                        relative: relative(dir, &path),
                        expected: Some(text.clone()),
                        input: text,
                    })
                })
                .collect(),
            Self::Expected { input, expected } => files_with_extensions(input, &["java"])
                .into_iter()
                .map(|path| {
                    let relative = relative(input, &path);
                    let expected = expected.join(&relative);
                    Case {
                        input: read_to_string(&path),
                        expected: expected.is_file().then(|| read_to_string(&expected)),
                        relative,
                    }
                })
                .collect(),
        }
    }
}

/// The `formatted:` section of a snapshot, or `None` when it has none.
fn snapshot_output(text: &str) -> Option<String> {
    let start = text.find("\nformatted:\n")? + "\nformatted:\n".len();
    let rest = &text[start..];
    let end = rest.find("\ndiagnostics:\n")?;
    Some(rest[..end].to_owned())
}

#[derive(Default)]
struct Tally {
    matched: usize,
    skipped: usize,
    missing: Vec<PathBuf>,
    /// Each mismatch with the line of its first difference.
    mismatches: Vec<(PathBuf, usize)>,
}

impl Tally {
    fn merge(&mut self, other: Self) {
        self.matched += other.matched;
        self.skipped += other.skipped;
        self.missing.extend(other.missing);
        self.mismatches.extend(other.mismatches);
    }
}

fn main() -> ExitCode {
    let args = match Args::parse() {
        Ok(args) => args,
        Err(error) => {
            eprintln!("{error}\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    let cases = args.cases.collect();
    let out = &args.out;
    let _ = fs::remove_dir_all(out);
    fs::create_dir_all(out).expect("create output directory");
    let options = FormatOptions {
        line_width: args.width,
        ..FormatOptions::default()
    };

    let tally = par_fold(
        &cases,
        |tally: &mut Tally, case| {
            let Some(expected) = &case.expected else {
                tally.missing.push(case.relative.clone());
                return;
            };
            if !jolt_java_syntax::parse(&case.input)
                .diagnostics()
                .is_empty()
            {
                tally.skipped += 1;
                return;
            }
            let actual = common::format(&case.input, options);
            if actual == *expected {
                tally.matched += 1;
                return;
            }
            let line = actual
                .lines()
                .zip(expected.lines())
                .position(|(a, e)| a != e)
                .unwrap_or_else(|| actual.lines().count().min(expected.lines().count()))
                + 1;
            let path = out.join(&case.relative).with_extension("actual.java");
            fs::create_dir_all(path.parent().expect("an output file has a parent"))
                .expect("create output directory");
            fs::write(&path, &actual).expect("write actual output");
            tally.mismatches.push((case.relative.clone(), line));
        },
        Tally::merge,
    );

    let mut summary = String::new();
    for (path, line) in &tally.mismatches {
        let _ = writeln!(
            summary,
            "{}: first difference at line {line}",
            path.display()
        );
    }
    for path in &tally.missing {
        let _ = writeln!(summary, "{}: no expected output", path.display());
    }
    let totals = format!(
        "files {}, matched {}, mismatched {}, missing expected {}, skipped (syntax errors) {}",
        cases.len(),
        tally.matched,
        tally.mismatches.len(),
        tally.missing.len(),
        tally.skipped
    );
    let _ = writeln!(summary, "\n{totals}");
    fs::write(out.join("summary.txt"), &summary).expect("write summary");
    println!("{totals}");
    println!("summary: {}", out.join("summary.txt").display());
    if tally.mismatches.is_empty() && tally.missing.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
