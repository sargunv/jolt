use std::path::Path;

use jolt_java_fmt::{JavaFormatOptions, JavaFormatSinkResult, format_source_to_sink};
use jolt_test_support::{StringSink, read_to_string, workspace_root};

#[test]
fn layout_fit_boundary_fixtures_stay_within_width() {
    let root = workspace_root(env!("CARGO_MANIFEST_DIR")).join("fixtures/java");
    for name in [
        "properties/layout-fit-boundaries/expression-statement-assignment.java",
        "properties/layout-fit-boundaries/return-statement-ternary.java",
        "style/layout-fit-boundaries/local-variable-nested-ternary.java",
        "properties/layout-fit-boundaries/field-initializer.java",
        "style/layout-fit-boundaries/call-single-argument-ternary.java",
    ] {
        assert_no_line_exceeds_width(&root.join(name), 80);
    }
}

fn assert_no_line_exceeds_width(path: &Path, line_width: u16) {
    let source = read_to_string(path);
    let formatted = format_or_panic(&source, line_width);
    let offending = formatted
        .lines()
        .enumerate()
        .map(|(index, line)| (index + 1, line, line.chars().count()))
        .find(|(_, _, width)| *width > usize::from(line_width));

    assert!(
        offending.is_none(),
        "formatted line exceeded width {line_width} in {}:\n{}\nfirst offending line: {:?}",
        path.display(),
        formatted,
        offending
    );
}

fn format_or_panic(source: &str, line_width: u16) -> String {
    let options = JavaFormatOptions {
        line_width,
        ..JavaFormatOptions::default()
    };
    let mut sink = StringSink::default();

    match format_source_to_sink(source, &options, &mut sink) {
        JavaFormatSinkResult::Complete | JavaFormatSinkResult::Halted => sink.into_string(),
        JavaFormatSinkResult::Blocked { diagnostics } => {
            panic!("formatter diagnostics: {diagnostics:#?}")
        }
        JavaFormatSinkResult::SinkError { error } => match error {},
    }
}
