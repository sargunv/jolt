use jolt_kotlin_fmt::{FormatOptions, FormatSinkResult, format_source_to_sink};
use jolt_kotlin_syntax::parse_kotlin_file;
use jolt_test_support::{
    SnapshotBuilder, StringSink, collect_kotlin_files, diagnostic_inventory, fixture_snapshot_name,
    kotlin_fixture_root, read_to_string, render_diagnostics, represented_comment_inventory,
    represented_token_loss_report, trivia_markers,
};

#[test]
fn kotlin_corpus_formatter_snapshots() {
    let options = FormatOptions::default();
    let root = kotlin_fixture_root(env!("CARGO_MANIFEST_DIR"));
    let mut formatted_cases = 0usize;
    let mut conservation_failures = Vec::new();

    for path in collect_kotlin_files(&root) {
        let relative = path
            .strip_prefix(&root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        let source = read_to_string(&path);
        let parse = parse_kotlin_file(&source);
        assert!(
            parse.syntax().is_some(),
            "Kotlin formatter corpus fixture produced no represented tree: {}",
            path.display()
        );
        let dedicated_audit =
            relative.starts_with("syntax/lexer") || relative.starts_with("syntax/recovery");
        let expected_parser_diagnostics = expects_parser_diagnostics(&relative);
        if !dedicated_audit {
            assert_eq!(
                !parse.diagnostics().is_empty(),
                expected_parser_diagnostics,
                "Kotlin formatter corpus route changed for {relative}: diagnostics={:#?}",
                parse.diagnostics()
            );
        }
        let audit_only = dedicated_audit || expected_parser_diagnostics;
        if audit_only {
            if let Some(failure) = audit_diagnostic_source(&source, options, &relative) {
                conservation_failures.push(failure);
            }
            continue;
        }

        formatted_cases += 1;
        let formatted = format_or_panic(&source, options, &path.display().to_string());
        let formatted_parse = parse_kotlin_file(&formatted);
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
        let token_loss = represented_token_loss_report(
            parse.syntax().expect("syntax checked above").token_iter(),
            formatted_parse
                .syntax()
                .expect("formatted syntax checked above")
                .token_iter(),
            &[],
        );
        let expected_markers = trivia_markers(&source);
        let actual_markers = trivia_markers(&formatted);
        if !token_loss.is_empty() || actual_markers != expected_markers {
            conservation_failures.push(format!(
                "{relative}:\n{token_loss}{}",
                if actual_markers == expected_markers {
                    String::new()
                } else {
                    format!(
                        "trivia markers changed\nexpected: {expected_markers:#?}\nactual: {actual_markers:#?}\n"
                    )
                }
            ));
        }

        let repeated = format_or_panic(&formatted, options, &path.display().to_string());
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
        "expected at least one valid Kotlin formatter corpus fixture"
    );
    assert!(
        conservation_failures.is_empty(),
        "formatter lost represented Kotlin source:\n{}",
        conservation_failures.join("\n")
    );
}

fn expects_parser_diagnostics(relative: &str) -> bool {
    let Some(name) = relative.strip_prefix("syntax/parser/") else {
        return false;
    };
    name.starts_with("diagnoses-")
        || name.starts_with("recovers-")
        || name == "parses-destructuring-square-preview.kt"
}

fn audit_diagnostic_source(source: &str, options: FormatOptions, label: &str) -> Option<String> {
    let before_parse = parse_kotlin_file(source);
    let before = before_parse.syntax()?;
    let formatted = format_or_panic(source, options, label);
    let after_parse = parse_kotlin_file(&formatted);
    let after = after_parse.syntax();
    let Some(after) = after else {
        return Some(format!("{label}: formatted output has no represented tree"));
    };
    let comments_changed = represented_comment_inventory(before.token_iter())
        != represented_comment_inventory(after.token_iter());
    let expected_markers = trivia_markers(source);
    let actual_markers = trivia_markers(&formatted);
    let repeated = format_or_panic(&formatted, options, label);

    let mut failures = String::new();
    if diagnostic_inventory(before_parse.diagnostics())
        != diagnostic_inventory(after_parse.diagnostics())
    {
        failures.push_str("parser diagnostic classification changed\n");
    }
    if comments_changed {
        failures.push_str("represented comment inventory changed\n");
    }
    if actual_markers != expected_markers {
        write!(
            failures,
            "trivia markers changed\nexpected: {expected_markers:#?}\nactual: {actual_markers:#?}\n"
        )
        .expect("writing to a String cannot fail");
    }
    if repeated != formatted {
        write!(
            failures,
            "formatter output is not idempotent\nfirst:\n{formatted}\nsecond:\n{repeated}\n"
        )
        .expect("writing to a String cannot fail");
    }
    (!failures.is_empty()).then(|| format!("{label}:\ninput:\n{source}\n{failures}"))
}

fn format_or_panic(source: &str, options: FormatOptions, label: &str) -> String {
    let mut sink = StringSink::default();
    match format_source_to_sink(source, &options, &mut sink) {
        FormatSinkResult::Complete => sink.into_string(),
        FormatSinkResult::Halted => {
            panic!("formatter unexpectedly halted with StringSink in {label}")
        }
        FormatSinkResult::Blocked { diagnostics } => {
            panic!("formatter diagnostics in {label}: {diagnostics:#?}\ninput:\n{source}")
        }
    }
}
use std::fmt::Write;
