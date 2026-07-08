use jolt_java_fmt::{JavaFormatOptions, JavaFormatSinkResult, format_source_to_sink};
use jolt_java_syntax::parse_compilation_unit;
use jolt_test_support::{
    SnapshotBuilder, StringSink, collect_java_files, fixture_snapshot_name, java_fixture_root,
    read_to_string, render_diagnostics,
};

#[test]
fn java_corpus_formatter_snapshots() {
    let options = JavaFormatOptions::default();
    let root = java_fixture_root(env!("CARGO_MANIFEST_DIR"));
    let mut formatted_cases = 0usize;
    let mut manifest_entries = Vec::new();

    for path in collect_java_files(&root) {
        let relative = path
            .strip_prefix(&root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        if path
            .strip_prefix(&root)
            .is_ok_and(|relative| relative.starts_with("syntax/lexer"))
        {
            manifest_entries.push(format!("skip lexer {relative}"));
            continue;
        }

        let source = read_to_string(&path);
        let parse = parse_compilation_unit(&source);
        if !parse.diagnostics().is_empty() || parse.syntax().is_none() {
            manifest_entries.push(format!("skip diagnostics {relative}"));
            continue;
        }

        manifest_entries.push(format!("format {relative}"));
        formatted_cases += 1;
        let formatted = format_or_panic(&source, &options, &path.display().to_string());
        let formatted_parse = parse_compilation_unit(&formatted);
        assert!(
            formatted_parse.diagnostics().is_empty(),
            "formatted output did not parse cleanly for {}: {:#?}\n{}",
            path.display(),
            formatted_parse.diagnostics(),
            formatted
        );
        assert!(
            formatted_parse.syntax().is_some(),
            "formatted output produced no syntax tree for {}",
            path.display()
        );

        let repeated = format_or_panic(&formatted, &options, &path.display().to_string());
        assert_eq!(
            repeated,
            formatted,
            "formatter output was not idempotent for {}",
            path.display()
        );

        let snapshot = SnapshotBuilder::new()
            .section("formatted", &formatted)
            .section("diagnostics", render_diagnostics(&[]))
            .finish();

        insta::assert_snapshot!(fixture_snapshot_name(&root, &path), snapshot);
    }

    assert!(
        formatted_cases > 0,
        "expected at least one valid Java formatter corpus fixture"
    );
    insta::assert_snapshot!("formatter_fixture_manifest", manifest_entries.join("\n"));
}

fn format_or_panic(source: &str, options: &JavaFormatOptions, label: &str) -> String {
    let mut sink = StringSink::default();
    match format_source_to_sink(source, options, &mut sink) {
        JavaFormatSinkResult::Complete | JavaFormatSinkResult::Halted => sink.into_string(),
        JavaFormatSinkResult::Blocked { diagnostics } => {
            panic!("formatter diagnostics in {label}: {diagnostics:#?}")
        }
        JavaFormatSinkResult::SinkError { error } => match error {},
    }
}
