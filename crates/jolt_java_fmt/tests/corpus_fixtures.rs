use std::path::PathBuf;

use jolt_fmt_ir::{FormatOptions, FormatSinkResult};
use jolt_java_fmt::format_source_to_sink;
use jolt_java_syntax::parse_compilation_unit;
use jolt_test_support::{
    StringSink, collect_java_files, diagnostic_inventory, read_to_string,
    represented_comment_inventory, workspace_root,
};

#[test]
fn imported_fixture_inputs_format_idempotently_and_conserve_represented_syntax() {
    let mut conservation_failures = Vec::new();
    for suite in [
        "google-java-format",
        "palantir-java-format",
        "prettier-java",
    ] {
        let root = fixture_root(suite);
        let options = FormatOptions::default();

        for path in collect_java_files(&root) {
            let relative = path
                .strip_prefix(&root)
                .expect("collected fixture should be below its root")
                .to_string_lossy();
            let source = read_to_string(&path);
            let parse = parse_compilation_unit(&source);
            let syntax = parse.syntax().unwrap_or_else(|| {
                panic!(
                    "parser produced no represented tree for {}: {:#?}",
                    path.display(),
                    parse.diagnostics()
                )
            });
            assert_eq!(
                syntax.source_text(),
                source,
                "syntax tree did not reconstruct exactly for {}",
                path.display()
            );

            let formatted = match format_source(&source, options) {
                Ok(formatted) => formatted,
                Err(diagnostics) => panic!(
                    "formatter refused clean imported input {}: {diagnostics:#?}",
                    path.display()
                ),
            };

            let formatted_parse = parse_compilation_unit(&formatted);
            if parse.diagnostics().is_empty() {
                assert!(
                    formatted_parse.diagnostics().is_empty(),
                    "formatted clean input did not parse cleanly for {}: {:#?}\n{}",
                    path.display(),
                    formatted_parse.diagnostics(),
                    formatted
                );
            } else {
                assert_eq!(
                    diagnostic_inventory(parse.diagnostics()),
                    diagnostic_inventory(formatted_parse.diagnostics()),
                    "formatting changed parser diagnostics for {}:\n{}",
                    path.display(),
                    formatted
                );
            }
            let formatted_syntax = formatted_parse.syntax().unwrap_or_else(|| {
                panic!(
                    "formatted output produced no syntax tree for {}",
                    path.display()
                )
            });
            assert_eq!(
                formatted_syntax.source_text(),
                formatted,
                "formatted output did not reconstruct exactly for {}",
                path.display()
            );
            let before_comments = represented_comment_inventory(syntax.token_iter());
            let after_comments = represented_comment_inventory(formatted_syntax.token_iter());
            let failure = (before_comments != after_comments).then(|| {
                format!(
                    "represented comments changed\nbefore: {before_comments:#?}\nafter: {after_comments:#?}\n"
                )
            });
            if let Some(failure) = failure {
                conservation_failures.push(format!("{suite}/{relative}:\n{failure}"));
            }

            let formatted_again =
                format_source(&formatted, options).unwrap_or_else(|diagnostics| {
                    panic!(
                        "formatted output was not accepted by formatter for {}: {diagnostics:#?}",
                        path.display()
                    )
                });
            assert_eq!(
                formatted_again,
                formatted,
                "formatted output was not idempotent for {}",
                path.display()
            );

            let repeated = format_source(&source, options).unwrap_or_else(|diagnostics| {
                panic!(
                    "repeated formatting produced diagnostic(s) for {}: {diagnostics:#?}",
                    path.display()
                )
            });
            assert_eq!(
                repeated,
                formatted,
                "formatting was not deterministic for {}",
                path.display()
            );
        }
    }
    assert!(
        conservation_failures.is_empty(),
        "imported Java conservation failures:\n{}",
        conservation_failures.join("\n")
    );
}

#[test]
fn deeply_nested_generic_recovery_formats_without_panicking_or_losing_following_syntax() {
    let depth = 4096;
    let mut ty = String::with_capacity(depth * 3 + 4);
    ty.push_str(&"T<".repeat(depth));
    ty.push_str("Leaf");
    ty.push_str(&">".repeat(depth));
    let source = format!("class C {{ {ty} value; int following; }} class D {{}}");

    let parse = parse_compilation_unit(&source);
    assert_eq!(
        parse.syntax().expect("represented input").source_text(),
        source
    );
    let formatted = format_source(&source, FormatOptions::default())
        .unwrap_or_else(|diagnostics| panic!("formatter blocked: {diagnostics:#?}"));
    assert!(formatted.contains("int following;"));
    assert!(formatted.contains("class D"));
    let reparsed = parse_compilation_unit(&formatted);
    assert_eq!(
        reparsed
            .syntax()
            .expect("represented formatted output")
            .source_text(),
        formatted
    );
}

#[test]
fn deeply_nested_value_recovery_formats_without_panicking_or_losing_following_syntax() {
    let depth = 4096;
    let annotation = format!(
        "{}@Leaf{} class C {{ int following; }} class D {{}}",
        "@A(".repeat(depth),
        ")".repeat(depth)
    );
    let sources = [
        format!(
            "class C {{ Object value = {}true; int following; }} class D {{}}",
            "!".repeat(depth)
        ),
        annotation,
        format!(
            "class C {{ Object value = {}{}; int following; }} class D {{}}",
            "{".repeat(depth),
            "}".repeat(depth)
        ),
    ];

    for source in sources {
        let parse = parse_compilation_unit(&source);
        assert_eq!(
            parse.syntax().expect("represented input").source_text(),
            source
        );
        let formatted = format_source(&source, FormatOptions::default())
            .unwrap_or_else(|diagnostics| panic!("formatter blocked: {diagnostics:#?}"));
        assert!(formatted.contains("int following;"));
        assert!(formatted.contains("class D"));
        let reparsed = parse_compilation_unit(&formatted);
        assert_eq!(
            reparsed
                .syntax()
                .expect("represented formatted output")
                .source_text(),
            formatted
        );
    }
}

#[test]
fn deeply_nested_structural_recovery_formats_without_panicking_or_losing_following_syntax() {
    let depth = 4096;
    let pattern = format!("{}Tail value{}", "R(".repeat(depth), ")".repeat(depth));
    let sources = [
        format!(
            "class C {{ void m(Object value) {{ switch (value) {{ case Outer({pattern}, Following following): break; }} }} int following; }} class D {{}}"
        ),
        format!(
            "{}{} class D {{ int following; }}",
            "class C { ".repeat(depth),
            "}".repeat(depth)
        ),
        format!(
            "class C {{ void m() {{ {}; }} int following; }} class D {{}}",
            "label: ".repeat(depth)
        ),
        format!(
            "class C {{ Object value = {}true{}; int following; }} class D {{}}",
            "target = !new Object() { Object nested = ".repeat(depth),
            "; }".repeat(depth)
        ),
    ];

    for source in sources {
        let parse = parse_compilation_unit(&source);
        assert_eq!(
            parse.syntax().expect("represented input").source_text(),
            source
        );
        let formatted = format_source(&source, FormatOptions::default())
            .unwrap_or_else(|diagnostics| panic!("formatter blocked: {diagnostics:#?}"));
        assert!(formatted.contains("int following;"));
        assert!(formatted.contains("class D"));
        let reparsed = parse_compilation_unit(&formatted);
        assert_eq!(
            reparsed
                .syntax()
                .expect("represented formatted output")
                .source_text(),
            formatted
        );
        assert_eq!(
            format_source(&formatted, FormatOptions::default())
                .expect("second format must complete"),
            formatted
        );
    }
}

#[test]
fn deep_formatter_spines_format_idempotently_and_keep_following_syntax() {
    let depth = 4096;
    let expression = |suffix: &str| {
        format!(
            "class C {{ Object value = root{}; int following; }} class Following {{}}",
            suffix.repeat(depth)
        )
    };
    let sources = [
        expression(".field"),
        expression(".method()"),
        expression("[0]"),
        expression("++"),
        expression(".new Inner()"),
        expression(".this"),
        expression(".super"),
        expression(".field.method()[0]++.new Inner().this.super"),
        expression(" + value"),
        expression(" instanceof T"),
        expression(" + value instanceof T"),
    ];

    for source in sources {
        let parse = parse_compilation_unit(&source);
        assert!(
            parse.diagnostics().is_empty(),
            "deep formatter spine did not remain structured: {:#?}",
            parse.diagnostics()
        );
        assert_eq!(
            parse
                .syntax()
                .expect("represented deep formatter spine")
                .source_text(),
            source
        );

        let formatted = format_source(&source, FormatOptions::default())
            .unwrap_or_else(|diagnostics| panic!("formatter blocked: {diagnostics:#?}"));
        assert!(formatted.contains("int following;"));
        assert!(formatted.contains("class Following"));
        let reparsed = parse_compilation_unit(&formatted);
        assert_eq!(
            reparsed
                .syntax()
                .expect("represented formatted output")
                .source_text(),
            formatted
        );
        assert_eq!(
            format_source(&formatted, FormatOptions::default())
                .expect("second format must complete"),
            formatted
        );
    }
}

fn format_source(
    source: &str,
    options: FormatOptions,
) -> Result<String, Vec<jolt_diagnostics::Diagnostic>> {
    let mut sink = StringSink::default();
    match format_source_to_sink(source, &options, &mut sink) {
        FormatSinkResult::Complete => Ok(sink.into_string()),
        FormatSinkResult::Halted => panic!("formatter unexpectedly halted with StringSink"),
        FormatSinkResult::Blocked { diagnostic } => Err(vec![diagnostic]),
    }
}

fn fixture_root(suite: &str) -> PathBuf {
    workspace_root(env!("CARGO_MANIFEST_DIR"))
        .join("tools/import/.imports")
        .join(suite)
        .join("input")
}
