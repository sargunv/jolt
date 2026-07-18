use std::collections::{BTreeMap, BTreeSet};

use jolt_kotlin_fmt::{FormatOptions, FormatSinkResult, format_source_to_sink};
use jolt_kotlin_syntax::parse_kotlin_file;
use jolt_test_support::{
    SnapshotBuilder, StringSink, collect_kotlin_files, deterministic_token_removal_candidates,
    diagnostic_inventory, fixture_snapshot_name, kotlin_fixture_root, read_to_string,
    render_diagnostics, represented_comment_inventory, represented_token_loss_report,
    trivia_markers,
};

#[test]
fn kotlin_corpus_formatter_snapshots() {
    let options = FormatOptions::default();
    let root = kotlin_fixture_root(env!("CARGO_MANIFEST_DIR"));
    let mut formatted_cases = 0usize;
    let mut manifest_entries = Vec::new();
    let mut conservation_failures = Vec::new();

    for path in collect_kotlin_files(&root) {
        let relative = path
            .strip_prefix(&root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        let source = read_to_string(&path);
        let parse = parse_kotlin_file(&source);
        if parse.syntax().is_none() {
            manifest_entries.push(format!("skip no syntax {relative}"));
            continue;
        }
        let dedicated_audit =
            relative.starts_with("syntax/lexer") || relative.starts_with("syntax/recovery");
        let audit_only = dedicated_audit || !parse.diagnostics().is_empty();
        if audit_only {
            manifest_entries.push(format!(
                "audit {} {relative}",
                if dedicated_audit {
                    "syntax"
                } else {
                    "diagnostics"
                }
            ));
            if let Some(failure) = audit_diagnostic_source(&source, options, &relative) {
                conservation_failures.push(failure);
            }
            continue;
        }

        manifest_entries.push(format!("format {relative}"));
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
    insta::assert_snapshot!("formatter_fixture_manifest", manifest_entries.join("\n"));
    assert!(
        conservation_failures.is_empty(),
        "formatter lost represented Kotlin source:\n{}",
        conservation_failures.join("\n")
    );
}

#[test]
fn deterministic_kotlin_recovery_mutations() {
    let options = FormatOptions::default();
    let root = kotlin_fixture_root(env!("CARGO_MANIFEST_DIR"));
    let mut fixture_families = BTreeSet::new();
    let mut cases = BTreeMap::new();

    for path in collect_kotlin_files(&root) {
        let family = path
            .parent()
            .and_then(|parent| parent.strip_prefix(&root).ok())
            .expect("fixture should have a family below the Kotlin fixture root")
            .to_string_lossy()
            .replace('\\', "/");
        fixture_families.insert(family.clone());
        if cases.contains_key(&family) {
            continue;
        }
        let source = read_to_string(&path);
        let parse = parse_kotlin_file(&source);
        let Some(syntax) = parse.syntax() else {
            continue;
        };
        let mutations = deterministic_token_removal_candidates(&source, syntax.token_iter())
            .into_iter()
            .filter(|mutation| {
                let parse = parse_kotlin_file(mutation);
                parse.syntax().is_some() && !parse.diagnostics().is_empty()
            })
            .take(2)
            .collect::<Vec<_>>();
        if !mutations.is_empty() {
            cases.insert(family, (path, mutations));
        }
    }

    assert_eq!(
        cases.keys().collect::<Vec<_>>(),
        fixture_families.iter().collect::<Vec<_>>(),
        "every Kotlin fixture family should provide a represented malformed mutation"
    );
    let mut failures = Vec::new();
    for (family, (path, mutations)) in cases {
        for (index, mutation) in mutations.into_iter().enumerate() {
            let label = format!("{family} mutation {index} ({})", path.display());
            if let Some(failure) = audit_diagnostic_source(&mutation, options, &label) {
                failures.push(failure);
            }
        }
    }
    assert!(
        failures.is_empty(),
        "deterministic Kotlin recovery mutations failed:\n{}",
        failures.join("\n")
    );
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
