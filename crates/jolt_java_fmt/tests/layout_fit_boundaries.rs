use std::path::Path;

use jolt_fmt_ir::{FormatOptions, FormatSinkResult};
use jolt_java_fmt::format_source_to_sink;
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
    jolt_test_support::assert_no_line_exceeds_width(
        &formatted,
        &path.display().to_string(),
        line_width,
    );
}

fn format_or_panic(source: &str, line_width: u16) -> String {
    let options = FormatOptions {
        line_width,
        ..FormatOptions::default()
    };
    let mut sink = StringSink::default();

    match format_source_to_sink(source, &options, &mut sink) {
        FormatSinkResult::Complete => sink.into_string(),
        FormatSinkResult::Halted => panic!("formatter unexpectedly halted with StringSink"),
        FormatSinkResult::Blocked { diagnostic } => {
            panic!("formatter diagnostic: {diagnostic:#?}")
        }
    }
}
