use jolt_fmt_ir::{FormatOptions, FormatSinkResult};
use jolt_java_fmt::format_source_to_sink;
use jolt_java_syntax::parse_compilation_unit;
use jolt_test_support::{
    SnapshotBuilder, StringSink, collect_java_files, fixture_snapshot_name, java_fixture_root,
    read_to_string, render_diagnostics,
};

#[test]
fn java_recovery_formatter_snapshots() {
    let options = FormatOptions::default();
    let root = java_fixture_root(env!("CARGO_MANIFEST_DIR"));
    let recovery_root = root.join("syntax/recovery");
    let paths = collect_java_files(&recovery_root);

    assert!(!paths.is_empty(), "expected at least one recovery fixture");

    for path in paths {
        let source = read_to_string(&path);
        let parse = parse_compilation_unit(&source);
        assert!(
            parse.syntax().is_some(),
            "recovery fixture did not produce a represented tree for {}",
            path.display()
        );
        let formatted = format_or_panic(&source, options, &path.display().to_string());
        let formatted_parse = parse_compilation_unit(&formatted);
        assert!(
            formatted_parse.syntax().is_some(),
            "formatted recovery output did not produce a represented tree for {}:\n{}",
            path.display(),
            formatted
        );
        let repeated = format_or_panic(&formatted, options, &path.display().to_string());
        assert_eq!(
            repeated,
            formatted,
            "recovery formatter output was not idempotent for {}",
            path.display()
        );

        let snapshot = SnapshotBuilder::new()
            .section("input", &source)
            .section("formatted", &formatted)
            .section("diagnostics", render_diagnostics(parse.diagnostics()))
            .finish();

        insta::assert_snapshot!(fixture_snapshot_name(&recovery_root, &path), snapshot);
    }
}

fn format_or_panic(source: &str, options: FormatOptions, label: &str) -> String {
    let mut sink = StringSink::default();
    match format_source_to_sink(source, &options, &mut sink) {
        FormatSinkResult::Complete | FormatSinkResult::Halted => sink.into_string(),
        FormatSinkResult::Blocked { diagnostics } => {
            panic!("formatter blocked for {label}: {diagnostics:#?}")
        }
    }
}
