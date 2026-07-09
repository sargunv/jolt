use jolt_fmt_ir::{FormatOptions, FormatSinkResult};
use jolt_kotlin_fmt::format_source_to_sink;
use jolt_kotlin_syntax::parse_kotlin_file;
use jolt_test_support::{
    SnapshotBuilder, StringSink, collect_kotlin_files, fixture_snapshot_name, kotlin_fixture_root,
    read_to_string, render_diagnostics,
};

#[test]
fn kotlin_recovery_formatter_snapshots() {
    let options = FormatOptions::default();
    let root = kotlin_fixture_root(env!("CARGO_MANIFEST_DIR"));
    let recovery_root = root.join("syntax/recovery");
    let paths = collect_kotlin_files(&recovery_root);

    assert!(!paths.is_empty(), "expected at least one recovery fixture");

    for path in paths {
        let source = read_to_string(&path);
        let parse = parse_kotlin_file(&source);
        let formatted = match parse.syntax() {
            Some(_syntax) => {
                let mut sink = StringSink::default();
                match format_source_to_sink(source.as_str(), &options, &mut sink) {
                    FormatSinkResult::Complete | FormatSinkResult::Halted => sink.into_string(),
                    FormatSinkResult::Blocked { diagnostics } => {
                        panic!("formatter blocked for {}: {diagnostics:#?}", path.display())
                    }
                }
            }
            None => source.clone(),
        };

        let snapshot = SnapshotBuilder::new()
            .section("input", &source)
            .section("formatted", &formatted)
            .section("diagnostics", render_diagnostics(parse.diagnostics()))
            .finish();

        insta::assert_snapshot!(fixture_snapshot_name(&recovery_root, &path), snapshot);
    }
}
