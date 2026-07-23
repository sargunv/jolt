use jolt_fmt_ir::{FormatOptions, FormatSinkResult};
use jolt_kotlin_fmt::format_source_to_sink;
use jolt_kotlin_syntax::parse_kotlin_file;
use jolt_test_support::{
    StringSink, assert_trivia_markers_conserved, collect_kotlin_files, kotlin_fixture_root,
};

#[test]
fn trivia_markers_are_conserved_by_formatter() {
    let root = kotlin_fixture_root(env!("CARGO_MANIFEST_DIR")).join("trivia");
    let files = collect_kotlin_files(&root);
    assert_trivia_markers_conserved(
        &files,
        |source, path| {
            let parse = parse_kotlin_file(source);
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
                FormatSinkResult::Blocked { diagnostics } => {
                    panic!(
                        "formatter diagnostics in {}: {diagnostics:#?}",
                        path.display()
                    )
                }
            }
        },
    );
}
