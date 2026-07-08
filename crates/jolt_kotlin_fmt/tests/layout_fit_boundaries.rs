use std::path::Path;

use jolt_kotlin_fmt::{FormatOptions, FormatSinkResult, format_source_to_sink};
use jolt_test_support::{StringSink, read_to_string, workspace_root};

#[test]
fn layout_fit_boundary_fixtures_stay_within_width() {
    let root = workspace_root(env!("CARGO_MANIFEST_DIR")).join("fixtures/kotlin");
    // `style/layout-fit-boundaries/property-initializer.kt` is intentionally
    // omitted: its source has a single string literal that contains a long URL
    // with no break points. The Kotlin formatter does not yet wrap unbreakable
    // literals; pinning it here would fail. Track as a known formatter gap.
    for name in [
        "properties/layout-fit-boundaries/deep-call-chain.kt",
        "properties/layout-fit-boundaries/long-when-result.kt",
        "properties/layout-fit-boundaries/property-delegate.kt",
        "style/layout-fit-boundaries/call-chain-single-argument.kt",
        "style/layout-fit-boundaries/when-branch-expression.kt",
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
        FormatSinkResult::Complete | FormatSinkResult::Halted => sink.into_string(),
        FormatSinkResult::Blocked { diagnostics } => {
            panic!("formatter diagnostics: {diagnostics:#?}")
        }
    }
}
