use std::path::PathBuf;

use jolt_java_fmt::{FormatOptions, FormatSinkResult, format_source_to_sink};
use jolt_java_syntax::{
    BinaryExpression, EmptyDeclaration, EmptyStatement, EnumBody, Guard, JavaNode, JavaSyntaxField,
    JavaSyntaxListPart, JavaSyntaxView, LambdaExpression, ParenthesizedExpression,
    ResourceSpecification, parse_compilation_unit,
};
use jolt_test_support::{
    RepresentedTokenRemoval, StringSink, collect_java_files, diagnostic_inventory, read_to_string,
    represented_comment_inventory, represented_token_loss_report, workspace_root,
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
            let removals = syntax_authorized_removals(syntax);
            let token_loss = represented_token_loss_report(
                syntax.token_iter(),
                formatted_syntax.token_iter(),
                &removals,
            );
            let before_comments = represented_comment_inventory(syntax.token_iter());
            let after_comments = represented_comment_inventory(formatted_syntax.token_iter());
            let failure = (!token_loss.is_empty() || before_comments != after_comments).then(|| {
                format!(
                    "{token_loss}{}",
                    if before_comments == after_comments {
                        String::new()
                    } else {
                        format!(
                            "represented comments changed\nbefore: {before_comments:#?}\nafter: {after_comments:#?}\n"
                        )
                    }
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

fn syntax_authorized_removals(
    syntax: jolt_java_syntax::CompilationUnit<'_>,
) -> Vec<RepresentedTokenRemoval> {
    let Some(root) = syntax.syntax_node() else {
        return Vec::new();
    };
    let mut stack = vec![root];
    let mut redundant_commas = 0usize;
    let mut redundant_semicolons = 0usize;
    let mut redundant_open_parentheses = 0usize;
    let mut redundant_close_parentheses = 0usize;
    while let Some(node) = stack.pop() {
        stack.extend(node.children());
        if EmptyDeclaration::cast(node)
            .is_some_and(|empty| empty.separator_removal_claim().is_some())
        {
            redundant_semicolons += 1;
        }
        if EmptyStatement::cast(node).is_some_and(|empty| empty.separator_removal_claim().is_some())
        {
            redundant_semicolons += 1;
        }
        if ResourceSpecification::cast(node)
            .is_some_and(|resources| resources.trailing_separator_removal_claim().is_some())
        {
            redundant_semicolons += 1;
        }
        if let Some(body) = EnumBody::cast(node) {
            redundant_semicolons +=
                usize::from(body.redundant_body_separator_removal_claim().is_some());
            let trailing_constant_comma_is_redundant = body.constants().is_ok_and(|field| {
                let JavaSyntaxField::Present(constants) = field else {
                    return false;
                };
                constants.parts().last().is_some_and(|part| {
                    let Ok(JavaSyntaxListPart::Separator(comma)) = part else {
                        return false;
                    };
                    body.redundant_constant_separator_removal_claim(&comma)
                        .is_some()
                })
            });
            redundant_commas += usize::from(trailing_constant_comma_is_redundant);
        }
        let parentheses = if let Some(guard) = Guard::cast(node) {
            Some(guard.redundant_parenthesis_removal_claims())
        } else if let Some(lambda) = LambdaExpression::cast(node) {
            if lambda.simple_parameter_parenthesis_removal().is_some() {
                redundant_open_parentheses += 1;
                redundant_close_parentheses += 1;
            }
            None
        } else {
            ParenthesizedExpression::cast(node).and_then(binary_parenthesis_removals)
        };
        if let Some(parentheses) = parentheses {
            redundant_open_parentheses += usize::from(parentheses.open.is_some());
            redundant_close_parentheses += usize::from(parentheses.close.is_some());
        }
    }
    [
        (redundant_semicolons != 0).then_some(RepresentedTokenRemoval {
            source: ";",
            count: redundant_semicolons,
        }),
        (redundant_commas != 0).then_some(RepresentedTokenRemoval {
            source: ",",
            count: redundant_commas,
        }),
        (redundant_open_parentheses != 0).then_some(RepresentedTokenRemoval {
            source: "(",
            count: redundant_open_parentheses,
        }),
        (redundant_close_parentheses != 0).then_some(RepresentedTokenRemoval {
            source: ")",
            count: redundant_close_parentheses,
        }),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn binary_parenthesis_removals(
    expression: ParenthesizedExpression<'_>,
) -> Option<jolt_java_syntax::JavaDelimiterRemoval<'_>> {
    let mut ancestor = expression.syntax_node().and_then(|node| node.parent());
    while let Some(node) = ancestor {
        if let Some(binary) = BinaryExpression::cast(node) {
            return Some(binary.redundant_parenthesis_removal_claims(&expression));
        }
        ancestor = node.parent();
    }
    None
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
