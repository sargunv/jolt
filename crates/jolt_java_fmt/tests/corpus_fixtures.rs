use std::fs;
use std::path::{Path, PathBuf};

use jolt_diagnostics::DiagnosticStage;
use jolt_java_fmt::{JavaFormatOptions, format_source};
use jolt_java_syntax::parse_compilation_unit;

#[test]
fn imported_fixture_inputs_format_idempotently_and_parse() {
    assert_corpus("google-java-format", 209);
    assert_corpus("palantir-java-format", 226);
    assert_corpus("prettier-java", 86);
}

fn assert_corpus(suite: &str, expected_files: usize) {
    let root = fixture_root(suite);
    let mut files = Vec::new();
    collect_java_files(&root, &mut files);

    files.sort();
    assert_eq!(
        files.len(),
        expected_files,
        "expected the pinned {suite} Java input fixture corpus"
    );

    for path in files {
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
        let first = format_source(&source, &JavaFormatOptions::default());

        assert!(
            first
                .diagnostics
                .iter()
                .all(|diagnostic| diagnostic.stage != DiagnosticStage::Formatter),
            "formatter diagnostic(s) in {}: {:#?}",
            path.display(),
            first.diagnostics
        );

        if allows_syntax_diagnostics(&path) {
            assert!(
                first.diagnostics.iter().any(|diagnostic| matches!(
                    diagnostic.stage,
                    DiagnosticStage::Lexer | DiagnosticStage::Parser
                )),
                "allowlisted syntax diagnostic fixture parsed cleanly and should be removed from the allowlist: {}",
                path.display()
            );
            assert!(
                first.formatted_source.is_none(),
                "invalid syntax fixture should not produce formatted output in {}",
                path.display()
            );
            continue;
        }

        assert!(
            first.diagnostics.is_empty(),
            "diagnostic(s) in {}: {:#?}",
            path.display(),
            first.diagnostics
        );

        let formatted = first
            .formatted_source
            .as_deref()
            .unwrap_or_else(|| panic!("formatter produced no output for {}", path.display()));
        let formatted_parse = parse_compilation_unit(formatted);
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

        let formatted_again = format_source(formatted, &JavaFormatOptions::default());
        assert!(
            formatted_again.diagnostics.is_empty(),
            "formatted output was not accepted by formatter for {}: {:#?}",
            path.display(),
            formatted_again.diagnostics
        );
        assert_eq!(
            formatted_again.formatted_source.as_deref(),
            Some(formatted),
            "formatted output was not idempotent for {}",
            path.display()
        );

        let repeated = format_source(&source, &JavaFormatOptions::default());
        assert!(
            repeated.diagnostics.is_empty(),
            "repeated formatting produced diagnostic(s) for {}: {:#?}",
            path.display(),
            repeated.diagnostics
        );
        assert_eq!(
            repeated.formatted_source.as_deref(),
            Some(formatted),
            "formatting was not deterministic for {}",
            path.display()
        );
    }
}

fn allows_syntax_diagnostics(path: &Path) -> bool {
    [
        // Intentionally invalid upstream Java: explicit constructor
        // invocations appear outside their valid constructor-body position.
        "google-java-format/input/B26952926.java",
        "palantir-java-format/input/B26952926.java",
        "palantir-java-format/input/palantir-expression-lambda-2.java",

        // Prettier expression-focused fixtures contain standalone expressions
        // that are not Java statement expressions.
        "prettier-java/input/binary_expressions/operator-position-end/operator-position-end.java",
        "prettier-java/input/binary_expressions/operator-position-start/operator-position-start.java",
        "prettier-java/input/comments/expression/expression.java",
        "prettier-java/input/conditional-expression/spaces/spaces.java",
        "prettier-java/input/conditional-expression/tabs/tabs.java",
        "prettier-java/input/expressions/expressions.java",
        "prettier-java/input/member_chain/member_chain.java",
        "prettier-java/input/try_catch/try_catch.java",

        // Unsupported Java 22-preview syntax, finalized in Java 25: flexible
        // constructor bodies allow statements before explicit constructor
        // invocations.
        "prettier-java/input/constructors/constructors.java",

        // Parser backlog plus fixture fragments: these lambda fixtures mix
        // standalone lambda snippets with Java 14 switch-rule lambda results;
        // one case also uses a Java 21 pattern-switch guard.
        "prettier-java/input/lambda/arrow-parens-always/arrow-parens-always.java",
        "prettier-java/input/lambda/arrow-parens-avoid/arrow-parens-avoid.java",

        // Intentionally invalid upstream Java: extra semicolons split the
        // import section before later import declarations.
        "prettier-java/input/package_and_imports/classWithMixedImports/classWithMixedImports.java",
        "prettier-java/input/package_and_imports/classWithOnlyNonStaticImports/classWithOnlyNonStaticImports.java",
        "prettier-java/input/package_and_imports/classWithOnlyStaticImports/classWithOnlyStaticImports.java",
        "prettier-java/input/package_and_imports/moduleWithMixedImports/moduleWithMixedImports.java",
        "prettier-java/input/package_and_imports/moduleWithOnlyNonStaticImports/moduleWithOnlyNonStaticImports.java",
        "prettier-java/input/package_and_imports/moduleWithOnlyStaticImports/moduleWithOnlyStaticImports.java",

        // Unsupported Java 21/22-preview syntax: string templates are not
        // tokenized or parsed by the Java syntax crate yet.
        "prettier-java/input/template-expression/template-expression.java",

        // Intentionally invalid upstream Java: unqualified `yield` method
        // invocations are rejected by the Java grammar.
        "prettier-java/input/yield-statement/yield-statement.java",
    ]
    .iter()
    .any(|suffix| path.ends_with(suffix))
}

fn fixture_root(suite: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(".fixtures/fixtures")
        .join(suite)
        .join("input")
}

fn collect_java_files(root: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(root).unwrap_or_else(|error| {
        panic!(
            "failed to read fixture directory {}: {error}",
            root.display()
        )
    }) {
        let path = entry.expect("valid directory entry").path();
        if path.is_dir() {
            collect_java_files(&path, files);
        } else if path
            .extension()
            .is_some_and(|extension| extension == "java")
        {
            files.push(path);
        }
    }
}
