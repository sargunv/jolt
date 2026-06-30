use std::cmp::Reverse;
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use jolt_diagnostics::DiagnosticStage;
use jolt_java_fmt::{JavaFormatProfile, JavaFormatStatus, format_java_source_with_profile};
use similar::{ChangeTag, TextDiff};

#[test]
fn google_java_format_oracle_scoreboard() {
    let summary = assert_profile(Profile {
        suite: "google-java-format",
        style: "google",
        java_profile: JavaFormatProfile::Google,
        expected_files: 209,
        invalid_upstream_fixtures: &["B26952926.java"],
    });

    insta::assert_snapshot!("google_java_format_oracle_scoreboard", summary.render());
}

#[test]
fn aosp_java_format_oracle_scoreboard() {
    let summary = assert_profile(Profile {
        suite: "google-java-format",
        style: "aosp",
        java_profile: JavaFormatProfile::Aosp,
        expected_files: 209,
        invalid_upstream_fixtures: &["B26952926.java"],
    });

    insta::assert_snapshot!("aosp_java_format_oracle_scoreboard", summary.render());
}

#[test]
fn palantir_java_format_oracle_scoreboard() {
    let summary = assert_profile(Profile {
        suite: "palantir-java-format",
        style: "palantir",
        java_profile: JavaFormatProfile::Palantir,
        expected_files: 226,
        invalid_upstream_fixtures: &["B26952926.java", "palantir-expression-lambda-2.java"],
    });

    insta::assert_snapshot!("palantir_java_format_oracle_scoreboard", summary.render());
}

#[derive(Clone, Copy)]
struct Profile<'a> {
    suite: &'a str,
    style: &'a str,
    java_profile: JavaFormatProfile,
    expected_files: usize,
    invalid_upstream_fixtures: &'a [&'a str],
}

fn assert_profile(profile: Profile<'_>) -> OracleSummary {
    let input_root = fixture_root(profile.suite, "input");
    let expected_root = fixture_root(profile.suite, profile.style);
    let report_root = report_root(profile.suite, profile.style);

    assert!(
        input_root.is_dir(),
        "missing oracle input fixture directory: {}",
        input_root.display()
    );
    assert!(
        expected_root.is_dir(),
        "missing oracle expected fixture directory: {}",
        expected_root.display()
    );

    let mut input_paths = Vec::new();
    collect_java_files(&input_root, &mut input_paths);
    input_paths.sort();
    assert_eq!(
        input_paths.len(),
        profile.expected_files,
        "expected the pinned {}/{} Java formatter fixture corpus",
        profile.suite,
        profile.style
    );

    reset_report_dir(&report_root);

    let mut summary = OracleSummary::new(profile);
    for input_path in input_paths {
        let relative = input_path
            .strip_prefix(&input_root)
            .expect("fixture should be under input root");
        let expected_path = expected_root.join(relative);
        assert!(
            expected_path.is_file(),
            "missing oracle expected output fixture: {}",
            expected_path.display()
        );

        let source = fs::read_to_string(&input_path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", input_path.display()));
        let expected = fs::read_to_string(&expected_path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", expected_path.display()));
        let relative_name = relative.to_string_lossy().replace('\\', "/");

        if profile
            .invalid_upstream_fixtures
            .contains(&relative_name.as_str())
        {
            summary.invalid_upstream_fixtures_skipped += 1;
            continue;
        }

        let result = format_java_source_with_profile(&source, profile.java_profile);
        match result.status {
            JavaFormatStatus::Blocked => {
                let missing_rule_diagnostic: Option<&jolt_diagnostics::Diagnostic> = None;
                let kind = if result
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.stage == DiagnosticStage::Parser)
                {
                    summary.parse_blocked += 1;
                    BlockKind::Parse
                } else if missing_rule_diagnostic.is_some() {
                    summary.missing_rule_blocked += 1;
                    BlockKind::MissingRule
                } else {
                    summary.other_blocked += 1;
                    BlockKind::Other
                };
                summary.blocked.push(BlockedFile {
                    path: relative_name,
                    kind,
                    missing_rule_bucket: missing_rule_diagnostic
                        .map(|diagnostic| diagnostic.message.clone()),
                    diagnostics: result.diagnostics.iter().map(render_diagnostic).collect(),
                });
            }
            JavaFormatStatus::Formatted => {
                summary.formatted += 1;
                let actual = result
                    .formatted_source
                    .as_deref()
                    .expect("formatted result should include source");
                if actual == expected {
                    summary.exact_matches += 1;
                } else {
                    let diff = line_diff(&expected, actual);
                    summary.mismatches.push(Mismatch {
                        path: relative_name,
                        diff_size: diff.changed_line_count,
                        actual: actual.to_owned(),
                        diff: diff.text,
                    });
                }
            }
        }
    }

    write_reports(&summary, &report_root);
    summary
}

fn fixture_root(suite: &str, profile: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(".oracles/fixtures")
        .join(suite)
        .join(profile)
}

fn report_root(suite: &str, profile: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(".oracles/reports/java")
        .join(suite)
        .join(profile)
}

fn collect_java_files(root: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(root).unwrap_or_else(|error| {
        panic!(
            "failed to read fixture directory {}: {error}",
            root.display()
        )
    }) {
        let path = entry.expect("valid directory entry").path();
        if path.is_dir() {
            collect_java_files(&path, files);
        } else if path
            .extension()
            .is_some_and(|extension| extension == "java")
        {
            files.push(path);
        }
    }
}

fn reset_report_dir(report_root: &Path) {
    if report_root.exists() {
        fs::remove_dir_all(report_root).unwrap_or_else(|error| {
            panic!(
                "failed to clear oracle report directory {}: {error}",
                report_root.display()
            )
        });
    }
    fs::create_dir_all(report_root).unwrap_or_else(|error| {
        panic!(
            "failed to create oracle report directory {}: {error}",
            report_root.display()
        )
    });
}

struct OracleSummary {
    suite: String,
    profile: String,
    total_considered: usize,
    invalid_upstream_fixtures_skipped: usize,
    parse_blocked: usize,
    missing_rule_blocked: usize,
    other_blocked: usize,
    formatted: usize,
    exact_matches: usize,
    mismatches: Vec<Mismatch>,
    blocked: Vec<BlockedFile>,
}

impl OracleSummary {
    fn new(profile: Profile<'_>) -> Self {
        Self {
            suite: profile.suite.to_owned(),
            profile: profile.style.to_owned(),
            total_considered: profile.expected_files,
            invalid_upstream_fixtures_skipped: 0,
            parse_blocked: 0,
            missing_rule_blocked: 0,
            other_blocked: 0,
            formatted: 0,
            exact_matches: 0,
            mismatches: Vec::new(),
            blocked: Vec::new(),
        }
    }

    fn mismatching_formatted_files(&self) -> usize {
        self.mismatches.len()
    }

    fn aggregate_diff_size(&self) -> usize {
        self.mismatches
            .iter()
            .map(|mismatch| mismatch.diff_size)
            .sum()
    }

    fn exact_match_percentage_basis_points(&self) -> usize {
        let valid = self
            .total_considered
            .saturating_sub(self.invalid_upstream_fixtures_skipped);
        if valid == 0 {
            return 0;
        }
        self.exact_matches * 10_000 / valid
    }

    fn largest_diff(&self) -> Option<&Mismatch> {
        self.mismatches
            .iter()
            .max_by_key(|mismatch| mismatch.diff_size)
    }

    fn missing_rule_buckets(&self) -> Vec<MissingRuleBucket<'_>> {
        let mut counts = BTreeMap::<&str, usize>::new();
        for blocked in &self.blocked {
            let Some(bucket) = blocked.missing_rule_bucket.as_deref() else {
                continue;
            };
            *counts.entry(bucket).or_default() += 1;
        }
        let mut buckets = counts
            .into_iter()
            .map(|(message, count)| MissingRuleBucket { message, count })
            .collect::<Vec<_>>();
        buckets.sort_by_key(|bucket| (Reverse(bucket.count), bucket.message));
        buckets
    }

    fn render(&self) -> String {
        let mut output = String::new();
        writeln!(&mut output, "suite: {}", self.suite).expect("write summary");
        writeln!(&mut output, "profile: {}", self.profile).expect("write summary");
        writeln!(&mut output, "total considered: {}", self.total_considered)
            .expect("write summary");
        writeln!(
            &mut output,
            "invalid upstream fixtures skipped: {}",
            self.invalid_upstream_fixtures_skipped
        )
        .expect("write summary");
        writeln!(&mut output, "parse blocked: {}", self.parse_blocked).expect("write summary");
        writeln!(
            &mut output,
            "missing-rule blocked: {}",
            self.missing_rule_blocked
        )
        .expect("write summary");
        writeln!(&mut output, "other blocked: {}", self.other_blocked).expect("write summary");
        writeln!(&mut output, "formatted: {}", self.formatted).expect("write summary");
        writeln!(&mut output, "exact matches: {}", self.exact_matches).expect("write summary");
        let basis_points = self.exact_match_percentage_basis_points();
        writeln!(
            &mut output,
            "exact-match percentage: {}.{:02}%",
            basis_points / 100,
            basis_points % 100
        )
        .expect("write summary");
        writeln!(
            &mut output,
            "mismatching formatted files: {}",
            self.mismatching_formatted_files()
        )
        .expect("write summary");
        writeln!(
            &mut output,
            "aggregate diff size: {}",
            self.aggregate_diff_size()
        )
        .expect("write summary");
        match self.largest_diff() {
            Some(mismatch) => writeln!(
                &mut output,
                "largest per-file diff: {} ({})",
                mismatch.path, mismatch.diff_size
            )
            .expect("write summary"),
            None => output.push_str("largest per-file diff: <none> (0)\n"),
        }
        output.push_str("\nworst mismatches:\n");
        let mut worst = self.mismatches.iter().collect::<Vec<_>>();
        worst.sort_by_key(|mismatch| (Reverse(mismatch.diff_size), mismatch.path.as_str()));
        for mismatch in worst.into_iter().take(10) {
            writeln!(&mut output, "  {}: {}", mismatch.path, mismatch.diff_size)
                .expect("write summary");
        }
        if self.mismatches.is_empty() {
            output.push_str("  <none>: 0\n");
        }

        output.push_str("\ntop missing-rule blockers:\n");
        let missing_rule_buckets = self.missing_rule_buckets();
        for bucket in missing_rule_buckets.iter().take(10) {
            writeln!(&mut output, "  {}: {}", bucket.count, bucket.message).expect("write summary");
        }
        if missing_rule_buckets.is_empty() {
            output.push_str("  0: <none>\n");
        }

        output
    }
}

struct MissingRuleBucket<'a> {
    message: &'a str,
    count: usize,
}

struct Mismatch {
    path: String,
    diff_size: usize,
    actual: String,
    diff: String,
}

struct BlockedFile {
    path: String,
    kind: BlockKind,
    missing_rule_bucket: Option<String>,
    diagnostics: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum BlockKind {
    Parse,
    MissingRule,
    Other,
}

impl BlockKind {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Parse => "parse",
            Self::MissingRule => "missing-rule",
            Self::Other => "other",
        }
    }
}

fn write_reports(summary: &OracleSummary, report_root: &Path) {
    let mut mismatches = summary.mismatches.iter().collect::<Vec<_>>();
    mismatches.sort_by_key(|mismatch| (Reverse(mismatch.diff_size), mismatch.path.as_str()));
    let mut blocked = summary.blocked.iter().collect::<Vec<_>>();
    blocked.sort_by_key(|blocked| (blocked.kind.as_str(), blocked.path.as_str()));

    let mut index = String::new();
    write_index_summary(summary, &mut index);
    write_mismatch_reports(&mut index, report_root, &mismatches);
    write_blocked_reports(&mut index, report_root, &blocked);

    fs::write(report_root.join("index.md"), index).unwrap_or_else(|error| {
        panic!(
            "failed to write oracle report index {}: {error}",
            report_root.display()
        )
    });
}

fn write_index_summary(summary: &OracleSummary, mut index: &mut String) {
    writeln!(&mut index, "# {} / {}", summary.suite, summary.profile).expect("write index");
    writeln!(&mut index).expect("write index");
    writeln!(
        &mut index,
        "- mismatching formatted files: {}",
        summary.mismatching_formatted_files()
    )
    .expect("write index");
    writeln!(
        &mut index,
        "- aggregate diff size: {}",
        summary.aggregate_diff_size()
    )
    .expect("write index");
    writeln!(&mut index).expect("write index");
    index.push_str("## Missing Rule Buckets\n\n");
    let missing_rule_buckets = summary.missing_rule_buckets();
    if missing_rule_buckets.is_empty() {
        index.push_str("- <none>: 0\n");
    } else {
        for bucket in missing_rule_buckets {
            writeln!(&mut index, "- {}: {}", bucket.count, bucket.message).expect("write index");
        }
    }
    writeln!(&mut index).expect("write index");
}

fn write_mismatch_reports(mut index: &mut String, report_root: &Path, mismatches: &[&Mismatch]) {
    index.push_str("## Mismatches\n\n");
    for mismatch in mismatches {
        let artifact_name = artifact_name(&mismatch.path);
        writeln!(
            &mut index,
            "- {}: {} ({})",
            mismatch.path, mismatch.diff_size, artifact_name
        )
        .expect("write index");

        let artifact = report_root.join(&artifact_name);
        let mut report = String::new();
        writeln!(&mut report, "# {}", mismatch.path).expect("write report");
        writeln!(&mut report).expect("write report");
        writeln!(&mut report, "## Diff").expect("write report");
        report.push_str("```diff\n");
        report.push_str(&mismatch.diff);
        if !mismatch.diff.ends_with('\n') {
            report.push('\n');
        }
        report.push_str("```\n\n");
        writeln!(&mut report, "## Actual").expect("write report");
        report.push_str("```java\n");
        report.push_str(&mismatch.actual);
        if !mismatch.actual.ends_with('\n') {
            report.push('\n');
        }
        report.push_str("```\n");
        fs::write(&artifact, report).unwrap_or_else(|error| {
            panic!(
                "failed to write oracle report {}: {error}",
                artifact.display()
            )
        });
    }
}

fn write_blocked_reports(mut index: &mut String, report_root: &Path, blocked: &[&BlockedFile]) {
    index.push_str("\n## Blocked\n\n");
    for blocked_file in blocked {
        let artifact_name = "blocked_".to_owned() + &artifact_name(&blocked_file.path);
        writeln!(
            &mut index,
            "- {}: {} ({})",
            blocked_file.path,
            blocked_file.kind.as_str(),
            artifact_name
        )
        .expect("write index");

        let artifact = report_root.join(&artifact_name);
        let mut report = String::new();
        writeln!(&mut report, "# {}", blocked_file.path).expect("write report");
        writeln!(&mut report).expect("write report");
        writeln!(&mut report, "kind: {}", blocked_file.kind.as_str()).expect("write report");
        writeln!(&mut report).expect("write report");
        writeln!(&mut report, "## Diagnostics").expect("write report");
        if blocked_file.diagnostics.is_empty() {
            report.push_str("- <none>\n");
        } else {
            for diagnostic in &blocked_file.diagnostics {
                writeln!(&mut report, "- {diagnostic}").expect("write report");
            }
        }
        fs::write(&artifact, report).unwrap_or_else(|error| {
            panic!(
                "failed to write oracle report {}: {error}",
                artifact.display()
            )
        });
    }
}

fn render_diagnostic(diagnostic: &jolt_diagnostics::Diagnostic) -> String {
    format!(
        "{} {:?} {:?} range={:?}: {}",
        diagnostic.code.as_str(),
        diagnostic.stage,
        diagnostic.severity,
        diagnostic.range,
        diagnostic.message
    )
}

fn artifact_name(path: &str) -> String {
    path.chars()
        .map(|ch| match ch {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '.' | '-' | '_' => ch,
            _ => '_',
        })
        .collect::<String>()
        + ".md"
}

struct DiffReport {
    text: String,
    changed_line_count: usize,
}

fn line_diff(expected: &str, actual: &str) -> DiffReport {
    if expected == actual {
        return DiffReport {
            text: String::new(),
            changed_line_count: 0,
        };
    }

    let diff = TextDiff::from_lines(expected, actual);
    let mut output = String::new();
    output.push_str("--- expected\n+++ actual\n");
    let mut changed_line_count = 0;

    for change in diff.iter_all_changes() {
        let marker = match change.tag() {
            ChangeTag::Delete => {
                changed_line_count += 1;
                '-'
            }
            ChangeTag::Insert => {
                changed_line_count += 1;
                '+'
            }
            ChangeTag::Equal => ' ',
        };
        let line = change
            .as_str()
            .expect("line changes from UTF-8 inputs should remain UTF-8");
        writeln!(&mut output, "{marker}{}", line.escape_debug()).expect("write diff");
    }

    DiffReport {
        text: output,
        changed_line_count,
    }
}

#[test]
fn diff_size_counts_insertions_without_offset_churn() {
    let diff = line_diff("a\nb\nc\n", "a\nx\nb\nc\n");

    assert_eq!(diff.changed_line_count, 1);
    assert!(diff.text.contains("+x\\n"));
}

#[test]
fn diff_size_counts_replacements_as_delete_plus_add() {
    let diff = line_diff("a\nb\nc\n", "a\nx\nc\n");

    assert_eq!(diff.changed_line_count, 2);
    assert!(diff.text.contains("-b\\n"));
    assert!(diff.text.contains("+x\\n"));
}
