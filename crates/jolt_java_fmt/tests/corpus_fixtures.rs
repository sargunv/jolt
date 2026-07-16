use std::path::PathBuf;

use jolt_java_fmt::{FormatOptions, FormatSinkResult, format_source_to_sink};
use jolt_java_syntax::{EmptyDeclaration, JavaNode, JavaSyntaxView, parse_compilation_unit};
use jolt_test_support::{
    DeferredReason, ImportedFormatterSummary, ObservedDeferredPath, RepresentedTokenRemoval,
    StringSink, assert_deferred_import_manifest, collect_java_files, read_to_string,
    represented_comment_inventory, represented_token_loss_report, workspace_root,
};

const CONSERVATION_PATHS: &[&str] = &[
    "package_and_imports/classWithMixedImports/classWithMixedImports.java",
    "package_and_imports/classWithOnlyNonStaticImports/classWithOnlyNonStaticImports.java",
    "package_and_imports/classWithOnlyStaticImports/classWithOnlyStaticImports.java",
    "package_and_imports/moduleWithMixedImports/moduleWithMixedImports.java",
    "package_and_imports/moduleWithOnlyNonStaticImports/moduleWithOnlyNonStaticImports.java",
    "package_and_imports/moduleWithOnlyStaticImports/moduleWithOnlyStaticImports.java",
    "template-expression/template-expression.java",
    "text-blocks/text-blocks.java",
];

#[test]
fn imported_fixture_inputs_format_idempotently_and_parse() {
    let mut deferred = Vec::new();
    let google_summary = assert_corpus("google-java-format", &mut deferred);
    let palantir_summary = assert_corpus("palantir-java-format", &mut deferred);
    let prettier_summary = assert_corpus("prettier-java", &mut deferred);

    let workspace = workspace_root(env!("CARGO_MANIFEST_DIR"));
    assert_deferred_import_manifest(
        &workspace,
        &workspace.join("tools/import/.imports"),
        &[
            "google-java-format/input",
            "palantir-java-format/input",
            "prettier-java/input",
        ],
        &[],
        &deferred,
    );

    insta::assert_snapshot!(
        "google_java_format_formatter_summary",
        google_summary.render()
    );
    insta::assert_snapshot!(
        "palantir_java_format_formatter_summary",
        palantir_summary.render()
    );
    insta::assert_snapshot!("prettier_java_formatter_summary", prettier_summary.render());
}

fn assert_corpus(
    suite: &str,
    deferred: &mut Vec<ObservedDeferredPath>,
) -> ImportedFormatterSummary {
    let root = fixture_root(suite);
    let files = collect_java_files(&root);
    let suite_name = format!("{suite}/input");
    let mut summary = ImportedFormatterSummary::new(suite, files.len());
    let options = FormatOptions::default();

    for path in files {
        let relative = path
            .strip_prefix(&root)
            .unwrap_or_else(|error| {
                panic!("{} is outside {}: {error}", path.display(), root.display())
            })
            .to_string_lossy()
            .replace('\\', "/");
        let source = read_to_string(&path);
        let parse = parse_compilation_unit(&source);
        let syntax = parse.syntax();
        let mut reasons = Vec::new();
        if syntax.is_none() {
            reasons.push(DeferredReason::NoSyntaxTree);
        } else if syntax.is_some_and(|syntax| syntax.source_text() != source) {
            summary.note_reconstruction_changed();
            reasons.push(DeferredReason::SyntaxReconstructionMismatch);
        }
        summary.record_diagnostics(parse.diagnostics());
        if !parse.diagnostics().is_empty() {
            summary.note_syntax_blocked();
            reasons.push(DeferredReason::ParserDiagnostics);
        }
        if !reasons.is_empty() {
            deferred.push(ObservedDeferredPath::new(
                &suite_name,
                &root,
                &path,
                reasons,
            ));
            continue;
        }
        let syntax = syntax.expect("active imported path has syntax");

        let formatted = match format_source(&source, options) {
            Ok(formatted) => formatted,
            Err(diagnostics) => panic!(
                "formatter refused clean imported input {}: {diagnostics:#?}",
                path.display()
            ),
        };
        summary.note_formatted();

        let formatted_parse = parse_compilation_unit(&formatted);
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
        if suite == "prettier-java" && CONSERVATION_PATHS.contains(&relative.as_str()) {
            let removals = syntax_authorized_removals(syntax);
            let token_loss = represented_token_loss_report(
                syntax.token_iter(),
                formatted_syntax.token_iter(),
                &removals,
            );
            assert!(
                token_loss.is_empty(),
                "formatter lost represented tokens for {}:\n{}",
                path.display(),
                token_loss
            );
            assert_eq!(
                represented_comment_inventory(syntax.token_iter()),
                represented_comment_inventory(formatted_syntax.token_iter()),
                "formatter changed represented comments for {}",
                path.display()
            );
        }

        let formatted_again = format_source(&formatted, options).unwrap_or_else(|diagnostics| {
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

    summary
}

fn syntax_authorized_removals(
    syntax: jolt_java_syntax::CompilationUnit<'_>,
) -> Vec<RepresentedTokenRemoval> {
    let Some(root) = syntax.syntax_node() else {
        return Vec::new();
    };
    let mut stack = vec![root];
    let mut redundant_semicolons = 0usize;
    while let Some(node) = stack.pop() {
        stack.extend(node.children());
        if EmptyDeclaration::cast(node)
            .is_some_and(|empty| empty.separator_removal_claim().is_some())
        {
            redundant_semicolons += 1;
        }
    }
    (redundant_semicolons != 0)
        .then_some(RepresentedTokenRemoval {
            source: ";",
            count: redundant_semicolons,
        })
        .into_iter()
        .collect()
}

fn format_source(
    source: &str,
    options: FormatOptions,
) -> Result<String, Vec<jolt_diagnostics::Diagnostic>> {
    let mut sink = StringSink::default();
    match format_source_to_sink(source, &options, &mut sink) {
        FormatSinkResult::Complete => Ok(sink.into_string()),
        FormatSinkResult::Halted => panic!("formatter unexpectedly halted with StringSink"),
        FormatSinkResult::Blocked { diagnostics } => Err(diagnostics),
    }
}

fn fixture_root(suite: &str) -> PathBuf {
    workspace_root(env!("CARGO_MANIFEST_DIR"))
        .join("tools/import/.imports")
        .join(suite)
        .join("input")
}
