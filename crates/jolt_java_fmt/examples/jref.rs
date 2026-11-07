//! Compares the formatter against the reference outputs.
//!
//! - `jref <dir> [--line-width N] [--out DIR]`: checks `format(F) == F` for
//!   every `.java` file, and for the `formatted:` section of every `.snap`,
//!   under `<dir>`.
//! - `jref --input <in> --expected <out> [--line-width N] [--out DIR]`: checks
//!   `format(in) == out` file by file.
//!
//! Prints totals and writes each mismatch's actual output plus a summary to
//! `--out` (default `target/jref`). Every mismatch lists the placeholder kinds
//! its run hit, so a mismatch without placeholders is a real bug. Inputs with
//! syntax errors are skipped and counted.

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use jolt_fmt::FormatOptions;

#[path = "../tests/common/mod.rs"]
mod common;

struct Case {
    relative: PathBuf,
    input: String,
    expected: String,
}

fn collect(dir: &Path, files: &mut Vec<PathBuf>) {
    let entries =
        fs::read_dir(dir).unwrap_or_else(|error| panic!("read {}: {error}", dir.display()));
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, files);
        } else if matches!(
            path.extension().and_then(|e| e.to_str()),
            Some("java" | "snap")
        ) {
            files.push(path);
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

#[allow(clippy::too_many_lines)]
fn main() {
    let mut args = std::env::args().skip(1);
    let mut dir = None;
    let mut input = None;
    let mut expected = None;
    let mut width = 80;
    let mut out = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/jref"));
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--input" => input = args.next().map(PathBuf::from),
            "--expected" => expected = args.next().map(PathBuf::from),
            "--line-width" => width = args.next().and_then(|w| w.parse().ok()).expect("width"),
            "--out" => out = args.next().map(PathBuf::from).expect("out dir"),
            _ => dir = Some(PathBuf::from(arg)),
        }
    }
    let cases = match (dir, input, expected) {
        (Some(dir), None, None) => {
            let mut files = Vec::new();
            collect(&dir, &mut files);
            files.sort();
            files
                .into_iter()
                .filter_map(|path| {
                    let text = fs::read_to_string(&path).ok()?;
                    let text = if path.extension().is_some_and(|e| e == "snap") {
                        snapshot_output(&text)?
                    } else {
                        text
                    };
                    Some(Case {
                        relative: path.strip_prefix(&dir).unwrap_or(&path).to_path_buf(),
                        input: text.clone(),
                        expected: text,
                    })
                })
                .collect::<Vec<_>>()
        }
        (None, Some(input), Some(expected)) => {
            let mut files = Vec::new();
            collect(&input, &mut files);
            files.sort();
            files
                .into_iter()
                .filter(|path| path.extension().is_some_and(|e| e == "java"))
                .filter_map(|path| {
                    let relative = path.strip_prefix(&input).ok()?.to_path_buf();
                    Some(Case {
                        input: fs::read_to_string(&path).ok()?,
                        expected: fs::read_to_string(expected.join(&relative)).ok()?,
                        relative,
                    })
                })
                .collect()
        }
        _ => {
            eprintln!(
                "usage: jref <dir> | jref --input <in> --expected <out> [--line-width N] [--out DIR]"
            );
            std::process::exit(2);
        }
    };

    let _ = fs::remove_dir_all(&out);
    fs::create_dir_all(&out).expect("create output directory");
    let options = FormatOptions {
        line_width: width,
        ..FormatOptions::default()
    };
    let next = AtomicUsize::new(0);
    let skipped = AtomicUsize::new(0);
    let matched = AtomicUsize::new(0);
    let mismatches = Mutex::new(Vec::new());
    let threads = std::thread::available_parallelism().map_or(4, usize::from);
    std::thread::scope(|scope| {
        for _ in 0..threads {
            scope.spawn(|| {
                while let Some(case) = cases.get(next.fetch_add(1, Ordering::Relaxed)) {
                    if !jolt_java_syntax::parse(&case.input)
                        .diagnostics()
                        .is_empty()
                    {
                        skipped.fetch_add(1, Ordering::Relaxed);
                        continue;
                    }
                    let _ = jolt_java_fmt::take_placeholder_kinds();
                    let actual = common::format(&case.input, options);
                    let placeholders = jolt_java_fmt::take_placeholder_kinds();
                    if actual == case.expected {
                        matched.fetch_add(1, Ordering::Relaxed);
                        continue;
                    }
                    let line = actual
                        .lines()
                        .zip(case.expected.lines())
                        .position(|(a, e)| a != e)
                        .unwrap_or_else(|| {
                            actual.lines().count().min(case.expected.lines().count())
                        })
                        + 1;
                    let path = out.join(&case.relative).with_extension("actual.java");
                    if let Some(parent) = path.parent() {
                        let _ = fs::create_dir_all(parent);
                    }
                    let _ = fs::write(&path, &actual);
                    mismatches.lock().expect("mismatch list").push((
                        case.relative.clone(),
                        line,
                        placeholders,
                    ));
                }
            });
        }
    });
    let mut mismatches = mismatches.into_inner().expect("mismatch list");
    mismatches.sort();
    let mut summary = String::new();
    let mut real = 0;
    for (path, line, placeholders) in &mismatches {
        if placeholders.is_empty() {
            real += 1;
        }
        let _ = writeln!(
            summary,
            "{}: first difference at line {line}; placeholders: {}",
            path.display(),
            if placeholders.is_empty() {
                "none (bug)".to_owned()
            } else {
                placeholders.join(", ")
            }
        );
    }
    let totals = format!(
        "files {}, matched {}, mismatched {} ({} without placeholders), skipped (syntax errors) {}",
        cases.len(),
        matched.load(Ordering::Relaxed),
        mismatches.len(),
        real,
        skipped.load(Ordering::Relaxed)
    );
    let _ = writeln!(summary, "\n{totals}");
    fs::write(out.join("summary.txt"), &summary).expect("write summary");
    println!("{totals}");
    println!("summary: {}", out.join("summary.txt").display());
}
