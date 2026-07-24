use std::path::PathBuf;

use jolt_fmt_ir::{FormatOptions, FormatSinkResult, SyntaxErrorPolicy};
use jolt_java_fmt::format_source_to_sink;
use jolt_java_syntax::{
    CompilationUnit, CompilationUnitItem, JavaSyntaxField, JavaSyntaxKind, JavaSyntaxListPart,
    JavaSyntaxView, parse_compilation_unit,
};
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
    // Generate threshold and stress inputs here: syntax-tree snapshots for these
    // regular nested shapes grow to hundreds of kilobytes without adding signal.
    let nested_type =
        |depth: usize, leaf: &str| format!("{}{}{}", "T<".repeat(depth), leaf, ">".repeat(depth));
    let depth = 4096;
    let ty = nested_type(depth, "Leaf");

    let mut alternating = String::with_capacity(depth * 18 + 4);
    alternating.push_str(&"T<@A((".repeat(depth));
    alternating.push_str("Leaf");
    alternating.push_str(&") value) Leaf>".repeat(depth));

    let malformed_leaf = format!("+ @A(({ty}) value) Leaf");
    let malformed = nested_type(129, &malformed_leaf);

    for (ty, expected_diagnostics) in [
        (nested_type(128, "Leaf"), Some((0, None))),
        (
            nested_type(129, "? extends Leaf"),
            Some((1, Some("java.parse.excessive_type_nesting"))),
        ),
        (ty, Some((1, Some("java.parse.excessive_type_nesting")))),
        (
            alternating,
            Some((1, Some("java.parse.excessive_syntax_nesting"))),
        ),
        (malformed, None),
    ] {
        let source = format!("class C {{ {ty} value; int following; }} class D {{}}");
        let parse = parse_compilation_unit(&source);
        if let Some((count, code)) = expected_diagnostics {
            assert_eq!(parse.diagnostics().len(), count);
            if let Some(code) = code {
                assert!(
                    parse.diagnostics().iter().all(|diagnostic| diagnostic.code
                        == jolt_diagnostics::DiagnosticCodeId::new(code)),
                    "unexpected generic-depth diagnostics: {:#?}",
                    parse.diagnostics()
                );
            }
        }
        let syntax = parse.syntax().expect("represented input");
        assert_eq!(syntax.source_text(), source);
        assert_structured_top_level_class(syntax, "D");
        assert_structured_node_containing(
            syntax,
            JavaSyntaxKind::FieldDeclaration,
            "int following;",
        );
        let formatted = format_source(&source, FormatOptions::default())
            .unwrap_or_else(|diagnostics| panic!("formatter blocked: {diagnostics:#?}"));
        assert!(formatted.contains("int following;"));
        assert!(formatted.contains("class D"));
        let reparsed = parse_compilation_unit(&formatted);
        let formatted_syntax = reparsed.syntax().expect("represented formatted output");
        assert_eq!(formatted_syntax.source_text(), formatted);
        assert_structured_top_level_class(formatted_syntax, "D");
        assert_structured_node_containing(
            formatted_syntax,
            JavaSyntaxKind::FieldDeclaration,
            "int following;",
        );
        assert_eq!(
            format_source(&formatted, FormatOptions::default())
                .expect("second format must complete"),
            formatted
        );
    }
}

#[test]
fn deeply_nested_value_recovery_formats_without_panicking_or_losing_following_syntax() {
    // These regular near-limit trees make syntax snapshots enormous; generate
    // them while still exercising the public parser and formatter boundary.
    let depth = 4096;
    let annotation = format!(
        "{}@Leaf{} class C {{ int following; }} class D {{}}",
        "@A(".repeat(depth),
        ")".repeat(depth)
    );
    let parenthesized_edge = format!(
        "class C {{ Object value = {}input{}; int following; }} class D {{}}",
        "(".repeat(63),
        ")".repeat(63)
    );
    let annotation_edge = format!(
        "{} class C {{ int following; }} class D {{}}",
        format!(
            "{}condition ? left : right, following = value{}",
            "@A(".repeat(127),
            ")".repeat(127)
        )
    );
    let array_edge = format!(
        "class C {{ Object value = {}{{}}, sibling{}; int following; }} class D {{}}",
        "{".repeat(129),
        "}".repeat(129)
    );
    let sources = [
        (
            format!(
                "class C {{ Object value = {}true; int following; }} class D {{}}",
                "!".repeat(depth)
            ),
            None,
        ),
        (
            format!(
                "class C {{ Object value = {}leaf; int following; }} class D {{}}",
                "value = ".repeat(depth)
            ),
            None,
        ),
        (annotation, None),
        (
            format!(
                "class C {{ Object value = {}{}; int following; }} class D {{}}",
                "{".repeat(depth),
                "}".repeat(depth)
            ),
            None,
        ),
        (parenthesized_edge, Some(1)),
        (annotation_edge, Some(2)),
        (array_edge, Some(1)),
    ];

    for (source, diagnostics) in sources {
        let parse = parse_compilation_unit(&source);
        if let Some(diagnostics) = diagnostics {
            assert_eq!(parse.diagnostics().len(), diagnostics);
            assert!(
                parse.diagnostics().iter().all(|diagnostic| diagnostic.code
                    == jolt_diagnostics::DiagnosticCodeId::new(
                        "java.parse.excessive_syntax_nesting"
                    )),
                "unexpected value-depth diagnostics: {:#?}",
                parse.diagnostics()
            );
        }
        let syntax = parse.syntax().expect("represented input");
        assert_eq!(syntax.source_text(), source);
        assert_structured_top_level_class(syntax, "D");
        assert_structured_node_containing(
            syntax,
            JavaSyntaxKind::FieldDeclaration,
            "int following;",
        );
        let formatted = format_source(&source, FormatOptions::default())
            .unwrap_or_else(|diagnostics| panic!("formatter blocked: {diagnostics:#?}"));
        assert!(formatted.contains("int following;"));
        assert!(formatted.contains("class D"));
        let reparsed = parse_compilation_unit(&formatted);
        let formatted_syntax = reparsed.syntax().expect("represented formatted output");
        assert_eq!(formatted_syntax.source_text(), formatted);
        assert_structured_top_level_class(formatted_syntax, "D");
        assert_structured_node_containing(
            formatted_syntax,
            JavaSyntaxKind::FieldDeclaration,
            "int following;",
        );
        assert_eq!(
            format_source(&formatted, FormatOptions::default())
                .expect("second format must complete"),
            formatted
        );
    }
}

#[test]
fn deeply_nested_structural_recovery_formats_without_panicking_or_losing_following_syntax() {
    // Keep structural thresholds generated: the equivalent syntax snapshot trial
    // was about 1.65 MB and mostly repeated indentation and wrapper nodes.
    let depth = 4096;
    let pattern = format!("{}Tail value{}", "R(".repeat(depth), ")".repeat(depth));
    let pattern_edge = format!(
        "{}@A(value = (left)) R(Tail t){}",
        "R(".repeat(125),
        ")".repeat(125)
    );
    let nested_body = |open: &str| {
        format!(
            "{}{} class D {{ int following; }}",
            open.repeat(depth),
            "}".repeat(depth)
        )
    };
    let labeled = |tail: &str| {
        format!(
            "class C {{ void m() {{ {}{tail} }} int following; }} class D {{}}",
            "label: ".repeat(127)
        )
    };
    let sources = [
        (
            format!(
                "class C {{ void m(Object value) {{ switch (value) {{ case Outer({pattern}, Following following): break; }} }} int following; }} class D {{}}"
            ),
            "Following following",
        ),
        (
            format!(
                "class C {{ void m(Object value) {{ switch (value) {{ case Outer({pattern_edge}, Following following): break; }} }} int following; }} class D {{}}"
            ),
            "@A(value = (left)) R(Tail t)",
        ),
        (nested_body("class C { "), "class D"),
        (nested_body("record R() { "), "class D"),
        (nested_body("interface I { "), "class D"),
        (nested_body("@interface A { "), "class D"),
        (nested_body("enum E { ; "), "class D"),
        (
            format!(
                "class C {{ void m() {{ {}; }} int following; }} class D {{}}",
                "label: ".repeat(depth)
            ),
            "int following;",
        ),
        (
            labeled("if (true) one(); else two(); sibling();"),
            "else two()",
        ),
        (
            labeled("do one(); while (true); sibling();"),
            "while (true)",
        ),
        (
            labeled("try {} catch (E e) {} finally {} sibling();"),
            "catch (E e)",
        ),
        (
            labeled("switch (value) { case 1: one(); default: two(); } sibling();"),
            "default: two()",
        ),
        (
            format!(
                "class C {{ void m() {{ {} ; sibling();{} }} int following; }} class D {{}}",
                "{ ".repeat(depth),
                " }".repeat(depth)
            ),
            "sibling()",
        ),
        (
            format!(
                "class C {{ Object value = {}true{}; int following; }} class D {{}}",
                "target = !new Object() { Object nested = ".repeat(depth),
                "; }".repeat(depth)
            ),
            "int following;",
        ),
    ];

    for (source, retained) in sources {
        let parse = parse_compilation_unit(&source);
        assert_eq!(parse.diagnostics().len(), 1);
        let syntax = parse.syntax().expect("represented input");
        assert_eq!(syntax.source_text(), source);
        assert_structured_top_level_class(syntax, "D");
        assert_structured_node_containing(
            syntax,
            JavaSyntaxKind::FieldDeclaration,
            "int following;",
        );
        let formatted = format_source(&source, FormatOptions::default())
            .unwrap_or_else(|diagnostics| panic!("formatter blocked: {diagnostics:#?}"));
        assert!(formatted.contains("int following;"));
        assert!(formatted.contains("class D"));
        assert!(formatted.contains(retained));
        let reparsed = parse_compilation_unit(&formatted);
        let formatted_syntax = reparsed.syntax().expect("represented formatted output");
        assert_eq!(formatted_syntax.source_text(), formatted);
        assert_structured_top_level_class(formatted_syntax, "D");
        assert_structured_node_containing(
            formatted_syntax,
            JavaSyntaxKind::FieldDeclaration,
            "int following;",
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
        let syntax = parse.syntax().expect("represented deep formatter spine");
        assert_eq!(syntax.source_text(), source);
        assert_structured_top_level_class(syntax, "Following");

        let formatted = format_source(&source, FormatOptions::default())
            .unwrap_or_else(|diagnostics| panic!("formatter blocked: {diagnostics:#?}"));
        assert!(formatted.contains("int following;"));
        assert!(formatted.contains("class Following"));
        let reparsed = parse_compilation_unit(&formatted);
        let formatted_syntax = reparsed.syntax().expect("represented formatted output");
        assert_eq!(formatted_syntax.source_text(), formatted);
        assert_structured_top_level_class(formatted_syntax, "Following");
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
    match format_source_to_sink(source, &options, SyntaxErrorPolicy::Format, &mut sink) {
        FormatSinkResult::Complete => Ok(sink.into_string()),
        FormatSinkResult::Halted => panic!("formatter unexpectedly halted with StringSink"),
        FormatSinkResult::Blocked { diagnostic } => Err(vec![diagnostic]),
    }
}

fn assert_structured_top_level_class(unit: CompilationUnit<'_>, expected_name: &str) {
    let JavaSyntaxField::Present(items) = unit.items() else {
        panic!("compilation-unit items must remain structured");
    };
    assert!(items.parts().any(|part| {
        matches!(
            part,
            JavaSyntaxListPart::Item(CompilationUnitItem::ClassDeclaration(declaration))
                if matches!(
                    declaration.name(),
                    JavaSyntaxField::Present(name) if name.text() == expected_name
                )
        )
    }));
}

fn assert_structured_node_containing(
    unit: CompilationUnit<'_>,
    expected_kind: JavaSyntaxKind,
    expected_source: &str,
) {
    let root = unit.syntax_node().expect("compilation unit has a root");
    let mut nodes = vec![root];
    while let Some(node) = nodes.pop() {
        let range = node.text_range();
        let node_source = &node.source()[range.start().get()..range.end().get()];
        if node.kind() == expected_kind
            && !node.is_directly_malformed()
            && node_source.contains(expected_source)
        {
            return;
        }
        nodes.extend(node.children());
    }
    panic!("missing structured {expected_kind:?} containing {expected_source:?}");
}

fn fixture_root(suite: &str) -> PathBuf {
    workspace_root(env!("CARGO_MANIFEST_DIR"))
        .join("tools/import/.imports")
        .join(suite)
        .join("input")
}
