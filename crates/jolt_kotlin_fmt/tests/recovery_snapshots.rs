use jolt_fmt_ir::{FormatOptions, FormatSinkResult};
use jolt_kotlin_fmt::format_source_to_sink;
use jolt_kotlin_syntax::parse_kotlin_file;
use jolt_test_support::{
    RepresentedTokenRemoval, SnapshotBuilder, StringSink, collect_kotlin_files,
    fixture_snapshot_name, kotlin_fixture_root, read_to_string, render_diagnostics,
    represented_comment_inventory, represented_token_loss_report, trivia_markers,
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
        let formatted = format_or_panic(&source, options, &path.display().to_string());
        let formatted_parse = parse_kotlin_file(&formatted);
        let formatted_syntax = formatted_parse.syntax().unwrap_or_else(|| {
            panic!(
                "formatted recovery output did not produce a represented tree for {}:\n{}",
                path.display(),
                formatted
            )
        });
        let token_loss = represented_token_loss_report(
            syntax.token_iter(),
            formatted_syntax.token_iter(),
            allowed_removed_tokens(&path),
        );
        let comment_loss = (represented_comment_inventory(syntax.token_iter())
            != represented_comment_inventory(formatted_syntax.token_iter()))
        .then_some("represented comment inventory changed\n");
        let expected_markers = trivia_markers(&source);
        let actual_markers = trivia_markers(&formatted);
        let marker_loss = (actual_markers != expected_markers).then(|| {
            format!(
                "trivia markers changed\nexpected: {expected_markers:#?}\nactual: {actual_markers:#?}\n"
            )
        });
        if !token_loss.is_empty() || comment_loss.is_some() || marker_loss.is_some() {
            conservation_failures.push(format!(
                "{}:\n{}{comment_loss}{marker_loss}",
                path.display(),
                token_loss,
                comment_loss = comment_loss.unwrap_or_default(),
                marker_loss = marker_loss.unwrap_or_default(),
            ));
        }
        let repeated = format_or_panic(&formatted, options, &path.display().to_string());
        if repeated != formatted {
            conservation_failures.push(format!(
                "{}:\nformatter output is not idempotent\nfirst:\n{formatted}\nsecond:\n{repeated}",
                path.display()
            ));
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

fn format_or_panic(source: &str, options: FormatOptions, label: &str) -> String {
    let mut sink = StringSink::default();
    match format_source_to_sink(source, &options, &mut sink) {
        FormatSinkResult::Complete => sink.into_string(),
        FormatSinkResult::Halted => {
            panic!("formatter unexpectedly halted with StringSink for {label}")
        }
        FormatSinkResult::Blocked { diagnostics } => {
            panic!("formatter blocked for {label}: {diagnostics:#?}")
        }
    }
}
