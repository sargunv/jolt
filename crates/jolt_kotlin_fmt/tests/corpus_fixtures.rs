use std::path::PathBuf;

use jolt_fmt_ir::{FormatOptions, FormatSinkResult, SyntaxErrorPolicy};
use jolt_kotlin_fmt::format_source_to_sink;
use jolt_kotlin_syntax::{
    KotlinFile, KotlinFileItem, KotlinSyntaxField, KotlinSyntaxKind, KotlinSyntaxListPart,
    KotlinSyntaxView, parse_kotlin_file,
};
use jolt_test_support::{StringSink, collect_kotlin_files, read_to_string, workspace_root};

#[test]
fn imported_fixture_inputs_format_idempotently_and_parse() {
    for suite in ["ktfmt", "maplibre-compose"] {
        let root = fixture_root(suite);
        let options = FormatOptions::default();

        for path in collect_kotlin_files(&root) {
            let source = read_to_string(&path);
            let parse = parse_kotlin_file(&source);
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
            assert!(
                parse.diagnostics().is_empty(),
                "imported Kotlin source produced diagnostics for {}: {:#?}",
                path.display(),
                parse.diagnostics()
            );

            let formatted = match format_source(&source, options) {
                Ok(formatted) => formatted,
                Err(diagnostics) => panic!(
                    "formatter refused clean imported input {}: {diagnostics:#?}",
                    path.display()
                ),
            };

            let formatted_parse = parse_kotlin_file(&formatted);
            assert!(
                formatted_parse.diagnostics().is_empty(),
                "formatted output did not parse cleanly for {}: {:#?}\n{}",
                path.display(),
                formatted_parse.diagnostics(),
                formatted
            );
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
}

#[test]
fn deeply_nested_recovery_formats_idempotently_and_keeps_following_syntax() {
    let depth = 4096;
    let sources = [
        format!(
            "typealias Deep = {}Leaf{}\nclass Following\n",
            "Box<".repeat(depth),
            ">".repeat(depth)
        ),
        format!(
            "fun value() = {}true\nval following = 1\nclass Following\n",
            "! ".repeat(depth)
        ),
        format!(
            "typealias Deep = {}Leaf{}\nclass Following\n",
            "Box<@A(value = input as ".repeat(depth),
            ") Annotated>".repeat(depth)
        ),
        format!(
            "typealias Deep = {}suspend () -> Unit{}\nclass Following\n",
            "Box<".repeat(127),
            ">".repeat(127)
        ),
        format!(
            "typealias Deep = {}context() () -> Unit{}\nclass Following\n",
            "Box<".repeat(127),
            ">".repeat(127)
        ),
        format!(
            "fun value() = {}leaf\nclass Following\n",
            "target = ".repeat(depth)
        ),
        format!(
            "fun value() = {}input{}\nclass Following\n",
            "(".repeat(depth),
            ")".repeat(depth)
        ),
        format!(
            "fun value() = {}call {{ a; b }}\nval following = 1\nclass Following\n",
            "! ".repeat(127)
        ),
        format!(
            r#"fun value() = {}call({{ item -> "${{if (item) "${{nested}}" else "fallback"}}"; item }}, [first, second])
val following = 1
class Following
"#,
            "! ".repeat(127),
        ),
        format!(
            "fun choose(value: Int) = when (value) {{ in {}candidate -> 1; else -> 0 }}\nclass Following\n",
            "! ".repeat(127),
        ),
        format!(
            "{}{}\nclass Following\n",
            "fun nested() {".repeat(depth),
            "}".repeat(depth)
        ),
        format!(
            "{}{}\nclass Following\n",
            "class Nested {".repeat(depth),
            "}".repeat(depth)
        ),
        format!(
            "{}{}\nclass Following\n",
            "class Nested { fun nested() {".repeat(depth),
            "}}".repeat(depth)
        ),
        format!(
            "{}{}\nclass Following\n",
            "enum class Nested { Entry {".repeat(depth),
            "}}".repeat(depth)
        ),
        format!(
            "fun value() = {}leaf{}\nclass Following\n",
            "object { val nested = ".repeat(depth),
            "}".repeat(depth)
        ),
        format!(
            "{}fun denied() {{\n// @formatter:off\nif (ready) {{ first; second }}\n// @formatter:on\n}}\nval sibling = 1\n{}\nclass Following\n",
            "fun outer() {".repeat(128),
            "}".repeat(128),
        ),
    ];

    for source in sources {
        let parse = parse_kotlin_file(&source);
        let syntax = parse.syntax().expect("represented deep input");
        assert_eq!(syntax.source_text(), source);
        assert_top_level_following(syntax);
        assert_expected_adjacent_properties(syntax, &source);
        if source.contains("when (value)") {
            assert_structured_when_entries(syntax);
        }

        let formatted = format_source(&source, FormatOptions::default())
            .unwrap_or_else(|diagnostics| panic!("formatter blocked: {diagnostics:#?}"));
        let reparsed = parse_kotlin_file(&formatted);
        let reparsed_syntax = reparsed.syntax().expect("represented formatted output");
        assert_eq!(reparsed_syntax.source_text(), formatted);
        assert_top_level_following(reparsed_syntax);
        assert_expected_adjacent_properties(reparsed_syntax, &source);
        if source.contains("when (value)") {
            assert_structured_when_entries(reparsed_syntax);
        }
        let formatted_again = format_source(&formatted, FormatOptions::default())
            .unwrap_or_else(|diagnostics| panic!("second format blocked: {diagnostics:#?}"));
        assert_eq!(formatted_again, formatted);
    }
}

#[test]
fn deep_type_recovery_honors_when_entry_boundary() {
    let source = format!(
        "fun choose(value: Any) = when (value) {{ is {}Broken( -> 1; else -> 0 }}\nval following = 1\nclass Following\n",
        "Box<".repeat(128),
    );
    assert_when_recovery_keeps_following_syntax(&source, "val following");
}

#[test]
fn deep_type_recovery_preserves_overlapping_comma_boundary() {
    let source = format!(
        "fun choose(value: Any) = when (value) {{ is {}Broken(, String -> 1; else -> 0 }}\nval following = 1\nclass Following\n",
        "Box<".repeat(128),
    );
    assert_when_recovery_keeps_following_syntax(&source, "val following");
    assert_structured_kind_count(
        parse_kotlin_file(&source).syntax().unwrap(),
        KotlinSyntaxKind::WhenCondition,
        2,
    );
}

#[test]
fn deep_expression_recovery_honors_when_entry_boundary() {
    let source = format!(
        "fun choose(value: Int) = when (value) {{ in {}(candidate -> 1; else -> 0 }}\nval sibling = 1\nclass Following\n",
        "! ".repeat(127),
    );
    assert_when_recovery_keeps_following_syntax(&source, "val sibling");
}

fn assert_when_recovery_keeps_following_syntax(source: &str, property: &str) {
    let parse = parse_kotlin_file(source);
    let syntax = parse.syntax().expect("represented deep when input");
    assert_eq!(syntax.source_text(), source);
    assert_structured_when_entries(syntax);
    assert_top_level_property(syntax, property);
    assert_top_level_following(syntax);

    let formatted = format_source(source, FormatOptions::default())
        .unwrap_or_else(|diagnostics| panic!("formatter blocked: {diagnostics:#?}"));
    let reparsed = parse_kotlin_file(&formatted);
    let reparsed_syntax = reparsed
        .syntax()
        .expect("represented formatted when output");
    assert_eq!(reparsed_syntax.source_text(), formatted);
    assert_structured_when_entries(reparsed_syntax);
    assert_top_level_property(reparsed_syntax, property);
    assert_top_level_following(reparsed_syntax);
    assert_eq!(
        format_source(&formatted, FormatOptions::default()).expect("second format must complete"),
        formatted
    );
}

fn assert_top_level_following(syntax: KotlinFile<'_>) {
    let KotlinSyntaxField::Present(items) = syntax.items() else {
        panic!("deep input did not retain structured top-level items");
    };
    assert!(
        items.parts().any(|part| {
            let KotlinSyntaxListPart::Item(item) = part else {
                return false;
            };
            matches!(
                item.cast_family::<KotlinFileItem<'_>>(),
                Some(KotlinFileItem::ClassDeclaration(declaration))
                    if declaration
                        .source_text()
                        .trim_start()
                        .starts_with("class Following")
            )
        }),
        "deep recovery swallowed the following top-level class"
    );
}

fn assert_expected_adjacent_properties(syntax: KotlinFile<'_>, source: &str) {
    for expected in ["val following", "val sibling"] {
        if source.contains(expected) {
            assert_structured_property(syntax, expected);
        }
    }
}

fn assert_top_level_property(syntax: KotlinFile<'_>, expected_prefix: &str) {
    let KotlinSyntaxField::Present(items) = syntax.items() else {
        panic!("deep input did not retain structured top-level items");
    };
    assert!(
        items.parts().any(|part| {
            let KotlinSyntaxListPart::Item(item) = part else {
                return false;
            };
            matches!(
                item.cast_family::<KotlinFileItem<'_>>(),
                Some(KotlinFileItem::PropertyDeclaration(declaration))
                    if declaration.source_text().trim_start().starts_with(expected_prefix)
            )
        }),
        "deep recovery swallowed top-level property {expected_prefix:?}"
    );
}

fn assert_structured_property(syntax: KotlinFile<'_>, expected_prefix: &str) {
    let root = syntax.syntax_node().expect("Kotlin file has a root");
    let mut nodes = vec![root];
    while let Some(node) = nodes.pop() {
        let range = node.text_range();
        let node_source = &node.source()[range.start().get()..range.end().get()];
        if node.kind() == KotlinSyntaxKind::PropertyDeclaration
            && !node.is_directly_malformed()
            && node_source.trim_start().starts_with(expected_prefix)
        {
            return;
        }
        nodes.extend(node.children());
    }
    panic!("deep recovery swallowed structured property {expected_prefix:?}");
}

fn assert_structured_when_entries(syntax: KotlinFile<'_>) {
    let root = syntax.syntax_node().expect("Kotlin file has a root");
    let mut nodes = vec![root];
    let mut count = 0;
    let mut has_else = false;
    let mut entries = Vec::new();
    while let Some(node) = nodes.pop() {
        if node.kind() == KotlinSyntaxKind::WhenEntry && !node.is_directly_malformed() {
            count += 1;
            let range = node.text_range();
            let source = &node.source()[range.start().get()..range.end().get()];
            entries.push(source);
            has_else |= source.trim_start().starts_with("else -> 0");
        }
        nodes.extend(node.children());
    }
    assert!(
        count >= 2 && has_else,
        "deep recovery lost structured following when entry: count={count}, has_else={has_else}, entries={entries:#?}"
    );
}

fn assert_structured_kind_count(syntax: KotlinFile<'_>, kind: KotlinSyntaxKind, minimum: usize) {
    let root = syntax.syntax_node().expect("Kotlin file has a root");
    let mut nodes = vec![root];
    let mut count = 0;
    while let Some(node) = nodes.pop() {
        count += usize::from(node.kind() == kind && !node.is_directly_malformed());
        nodes.extend(node.children());
    }
    assert!(
        count >= minimum,
        "expected at least {minimum} structured {kind:?} nodes, found {count}"
    );
}

#[test]
fn deep_infix_and_type_formatter_spines_format_idempotently_and_keep_following_syntax() {
    let depth = 4096;
    let expression = |suffix: &str| {
        format!(
            "fun value() = root{}\nclass Following\n",
            suffix.repeat(depth)
        )
    };
    let ty = |suffix: &str, tail: &str| {
        format!(
            "typealias Deep = Leaf{}{}\nclass Following\n",
            suffix.repeat(depth),
            tail
        )
    };
    let sources = [
        expression(" + value"),
        expression(" as T"),
        expression(" + value as T"),
        ty("?", ""),
        ty("!!", ""),
        ty(" & Any", ""),
        ty(".()", " -> Unit"),
        ty("?!! & Any.()", " -> Unit"),
    ];

    assert_deep_formatter_spines(sources);
}

#[test]
fn deep_suffix_formatter_spines_format_idempotently_and_keep_following_syntax() {
    let depth = 4096;
    let expression = |suffix: &str| {
        format!(
            "fun value() = root{}\nclass Following\n",
            suffix.repeat(depth)
        )
    };
    let sources = [
        expression(".field"),
        expression("()"),
        expression("[0]"),
        expression("!!"),
        expression("::target"),
        expression(".field()[0]!!::target"),
    ];

    assert_deep_formatter_spines(sources);
}

fn assert_deep_formatter_spines(sources: impl IntoIterator<Item = String>) {
    for source in sources {
        let parse = parse_kotlin_file(&source);
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
        assert!(formatted.contains("class Following"));
        let reparsed = parse_kotlin_file(&formatted);
        assert_eq!(
            reparsed
                .syntax()
                .expect("represented formatted output")
                .source_text(),
            formatted
        );
        let formatted_again = format_source(&formatted, FormatOptions::default())
            .unwrap_or_else(|diagnostics| panic!("second format blocked: {diagnostics:#?}"));
        assert_eq!(formatted_again, formatted);
    }
}

#[test]
fn empty_excessive_body_keeps_claims_without_selecting_body_layout() {
    let source = format!(
        "fun value() = {}fun() {{}}{}\n",
        "(".repeat(63),
        ")".repeat(63)
    );
    let formatted = format_source(&source, FormatOptions::default())
        .unwrap_or_else(|diagnostics| panic!("formatter blocked: {diagnostics:#?}"));
    assert!(formatted.contains("fun() {}"), "{formatted}");
    assert!(!formatted.contains("fun() {\n"), "{formatted}");

    let source = format!(
        "fun value() = {}object {{}}{}\n",
        "(".repeat(63),
        ")".repeat(63)
    );
    let formatted = format_source(&source, FormatOptions::default())
        .unwrap_or_else(|diagnostics| panic!("formatter blocked: {diagnostics:#?}"));
    let body = formatted
        .split_once("object {")
        .and_then(|(_, body)| body.split_once('}').map(|(body, _)| body))
        .expect("formatted object body");
    assert_eq!(body.matches('\n').count(), 1, "{formatted}");
    let formatted_again = format_source(&formatted, FormatOptions::default())
        .unwrap_or_else(|diagnostics| panic!("second format blocked: {diagnostics:#?}"));
    assert_eq!(formatted_again, formatted);
}

#[test]
fn unclosed_excessive_bodies_format_idempotently() {
    for source in ["fun nested() {".repeat(130), "class Nested {".repeat(130)] {
        let parse = parse_kotlin_file(&source);
        assert_eq!(
            parse
                .syntax()
                .expect("represented unclosed input")
                .source_text(),
            source
        );
        let formatted = format_source(&source, FormatOptions::default())
            .unwrap_or_else(|diagnostics| panic!("formatter blocked: {diagnostics:#?}"));
        let formatted_again = format_source(&formatted, FormatOptions::default())
            .unwrap_or_else(|diagnostics| panic!("second format blocked: {diagnostics:#?}"));
        assert_eq!(formatted_again, formatted);
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

fn fixture_root(suite: &str) -> PathBuf {
    workspace_root(env!("CARGO_MANIFEST_DIR"))
        .join("tools/import/.imports")
        .join(suite)
        .join("source")
}
