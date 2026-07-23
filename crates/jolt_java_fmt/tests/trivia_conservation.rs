use jolt_fmt_ir::{FormatOptions, FormatSinkResult};
use jolt_java_fmt::format_source_to_sink;
use jolt_java_syntax::parse_compilation_unit;
use jolt_test_support::{
    StringSink, assert_trivia_markers_conserved, collect_java_files, java_fixture_root,
};

#[test]
fn trivia_markers_are_conserved_by_formatter() {
    let root = java_fixture_root(env!("CARGO_MANIFEST_DIR")).join("trivia");
    let files = collect_java_files(&root);
    assert_trivia_markers_conserved(
        &files,
        |source, path| {
            let parse = parse_compilation_unit(source);
            assert!(
                parse.diagnostics().is_empty(),
                "trivia fixture must parse cleanly before formatting: {}\n{:#?}",
                path.display(),
                parse.diagnostics()
            );
        },
        |source, path| {
            let mut sink = StringSink::default();
            match format_source_to_sink(source, &FormatOptions::default(), &mut sink) {
                FormatSinkResult::Complete => sink.into_string(),
                FormatSinkResult::Halted => panic!(
                    "formatter unexpectedly halted with StringSink in {}",
                    path.display()
                ),
                FormatSinkResult::Blocked { diagnostic } => {
                    panic!(
                        "formatter diagnostic in {}: {diagnostic:#?}",
                        path.display()
                    )
                }
            }
        },
    );
}
