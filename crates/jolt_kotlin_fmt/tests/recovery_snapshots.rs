use jolt_fmt_ir::FormatOptions;
use jolt_kotlin_fmt::format_source_to_sink;
use jolt_kotlin_syntax::parse_kotlin_file;
use jolt_test_support::{
    RepresentedTokenRemoval, SnapshotBuilder, collect_kotlin_files, fixture_snapshot_name,
    format_source_or_panic, formatter_conservation_failure, kotlin_fixture_root, read_to_string,
    render_diagnostics,
};

#[test]
fn kotlin_recovery_formatter_snapshots() {
    let options = FormatOptions::default();
    let root = kotlin_fixture_root(env!("CARGO_MANIFEST_DIR"));
    let recovery_root = root.join("syntax/recovery");
    let paths = collect_kotlin_files(&recovery_root);
    let mut conservation_failures = Vec::new();

    assert!(!paths.is_empty(), "expected at least one recovery fixture");

    for path in paths {
        let source = read_to_string(&path);
        let parse = parse_kotlin_file(&source);
        let syntax = parse.syntax().unwrap_or_else(|| {
            panic!(
                "recovery fixture did not produce a represented tree for {}",
                path.display()
            )
        });
        let formatted = format_source_or_panic(
            format_source_to_sink,
            &source,
            &options,
            &path.display().to_string(),
        );
        let formatted_parse = parse_kotlin_file(&formatted);
        let formatted_syntax = formatted_parse.syntax().unwrap_or_else(|| {
            panic!(
                "formatted recovery output did not produce a represented tree for {}:\n{}",
                path.display(),
                formatted
            )
        });
        let repeated = format_source_or_panic(
            format_source_to_sink,
            &formatted,
            &options,
            &path.display().to_string(),
        );
        if let Some(failure) = formatter_conservation_failure(
            path.display(),
            &source,
            &formatted,
            &repeated,
            syntax.token_iter(),
            formatted_syntax.token_iter(),
            allowed_removed_tokens(&path),
        ) {
            conservation_failures.push(failure);
        }

        let snapshot = SnapshotBuilder::new()
            .section("input", &source)
            .section("formatted", &formatted)
            .section("diagnostics", render_diagnostics(parse.diagnostics()))
            .finish();

        insta::assert_snapshot!(fixture_snapshot_name(&recovery_root, &path), snapshot);
    }

    assert!(
        conservation_failures.is_empty(),
        "formatter lost represented Kotlin source:\n{}",
        conservation_failures.join("\n")
    );
}

const NORMALIZATION_REMOVALS: &[RepresentedTokenRemoval] = &[RepresentedTokenRemoval {
    source: ";",
    count: usize::MAX,
}];

fn allowed_removed_tokens(path: &std::path::Path) -> &'static [RepresentedTokenRemoval] {
    if path
        .parent()
        .and_then(std::path::Path::file_name)
        .is_some_and(|name| name == "normalization")
    {
        NORMALIZATION_REMOVALS
    } else {
        &[]
    }
}
