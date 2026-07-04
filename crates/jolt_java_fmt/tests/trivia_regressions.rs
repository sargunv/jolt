use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use jolt_java_fmt::JavaFormatOptions;

mod support;
use jolt_java_syntax::parse_compilation_unit;
use support::format_source;

#[test]
fn trivia_marker_regressions_preserve_all_comments() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/trivia");
    let mut fixtures = Vec::new();
    collect_java_files(&root, &mut fixtures);
    fixtures.sort();

    assert!(
        !fixtures.is_empty(),
        "expected at least one Java trivia regression fixture under {}",
        root.display()
    );

    let mut failures = Vec::new();

    for fixture in fixtures {
        let source = fs::read_to_string(&fixture)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", fixture.display()));
        let expected_markers = collect_trivia_markers(&source);
        assert!(
            !expected_markers.is_empty(),
            "trivia regression fixture has no JOLT-TRIVIA markers: {}",
            fixture.display()
        );

        let first = format_source(&source, &JavaFormatOptions::default());
        if !first.diagnostics.is_empty() {
            failures.push(format!(
                "{}: formatter diagnostics: {:#?}",
                fixture.display(),
                first.diagnostics
            ));
            continue;
        }

        let Some(formatted) = first.formatted_source.as_deref() else {
            failures.push(format!(
                "{}: formatter blocked without output",
                fixture.display()
            ));
            continue;
        };

        let actual_markers = collect_trivia_markers(formatted);
        let missing = expected_markers
            .difference(&actual_markers)
            .cloned()
            .collect::<Vec<_>>();
        if !missing.is_empty() {
            failures.push(format!(
                "{}: dropped trivia marker(s): {}",
                fixture.display(),
                missing.join(", ")
            ));
            continue;
        }

        let formatted_parse = parse_compilation_unit(formatted);
        if !formatted_parse.diagnostics().is_empty() || formatted_parse.syntax().is_none() {
            failures.push(format!(
                "{}: formatted output did not parse cleanly: {:#?}\n{}",
                fixture.display(),
                formatted_parse.diagnostics(),
                formatted
            ));
            continue;
        }

        let formatted_again = format_source(formatted, &JavaFormatOptions::default());
        if !formatted_again.diagnostics.is_empty()
            || formatted_again.formatted_source.as_deref() != Some(formatted)
        {
            failures.push(format!(
                "{}: formatted output was not idempotent: {:#?}",
                fixture.display(),
                formatted_again.diagnostics
            ));
        }
    }

    if !failures.is_empty() {
        panic!(
            "{} Java trivia regression failure(s):\n- {}",
            failures.len(),
            failures.join("\n- ")
        );
    }
}

fn collect_java_files(root: &Path, fixtures: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(root).unwrap_or_else(|error| {
        panic!(
            "failed to read trivia fixture directory {}: {error}",
            root.display()
        )
    }) {
        let path = entry.expect("valid directory entry").path();
        if path.is_dir() {
            collect_java_files(&path, fixtures);
        } else if path
            .extension()
            .is_some_and(|extension| extension == "java")
        {
            fixtures.push(path);
        }
    }
}

fn collect_trivia_markers(source: &str) -> BTreeSet<String> {
    let mut markers = BTreeSet::new();
    let mut rest = source;

    while let Some(index) = rest.find("JOLT-TRIVIA:") {
        let marker_start = index + "JOLT-TRIVIA:".len();
        let marker_tail = &rest[marker_start..];
        let marker_len = marker_tail
            .chars()
            .take_while(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_'))
            .map(char::len_utf8)
            .sum();
        assert!(marker_len > 0, "empty JOLT-TRIVIA marker in source");

        let marker = &marker_tail[..marker_len];
        markers.insert(format!("JOLT-TRIVIA:{marker}"));
        rest = &marker_tail[marker_len..];
    }

    markers
}
