use jolt_java_fmt::{FormatOptions, FormatSinkResult, format_source_to_sink};
use jolt_java_syntax::parse_compilation_unit;
use jolt_test_support::{
    StringSink, diagnostic_inventory, represented_comment_inventory, represented_token_loss_report,
    trivia_markers,
};

#[test]
#[ignore = "known regression: malformed import duplicates its comment"]
fn malformed_import_without_semicolon_is_idempotent() {
    assert_diagnostic_format_conserved(include_str!(
        "fixtures/malformed-import-without-semicolon.java"
    ));
}

#[test]
#[ignore = "known regression: incomplete module name is not idempotent"]
fn malformed_module_name_gap_is_idempotent() {
    assert_diagnostic_format_conserved(include_str!("fixtures/incomplete-module-name-gap.java"));
}

#[test]
#[ignore = "known regression: formatter-ignore recovery duplicates trivia"]
fn formatter_ignore_constructor_mutation_conserves_trivia() {
    assert_diagnostic_format_conserved(include_str!(
        "fixtures/formatter-ignore-constructor-missing-open-paren.java"
    ));
}

#[test]
#[ignore = "known regression: mixed line endings change comment ownership"]
fn mixed_line_ending_comment_mutation_is_idempotent() {
    assert_diagnostic_format_conserved(include_str!(
        "fixtures/mixed-line-ending-comment-missing-type.java"
    ));
}

fn assert_diagnostic_format_conserved(source: &str) {
    let before_parse = parse_compilation_unit(source);
    let before = before_parse
        .syntax()
        .expect("regression input must produce a represented tree");
    assert!(
        !before_parse.diagnostics().is_empty(),
        "regression input must exercise parser recovery"
    );

    let formatted = format(source);
    let after_parse = parse_compilation_unit(&formatted);
    let after = after_parse
        .syntax()
        .expect("formatted regression output must produce a represented tree");

    assert_eq!(
        diagnostic_inventory(before_parse.diagnostics()),
        diagnostic_inventory(after_parse.diagnostics()),
        "formatting changed parser diagnostic classification"
    );
    assert_eq!(
        represented_comment_inventory(before.token_iter()),
        represented_comment_inventory(after.token_iter()),
        "formatting changed represented comments"
    );
    assert_eq!(
        trivia_markers(&formatted),
        trivia_markers(source),
        "formatting changed trivia markers"
    );
    let token_loss = represented_token_loss_report(before.token_iter(), after.token_iter(), &[]);
    assert!(
        token_loss.is_empty(),
        "formatting lost represented tokens:\n{token_loss}"
    );
    assert_eq!(
        format(&formatted),
        formatted,
        "formatter output was not idempotent"
    );
}

fn format(source: &str) -> String {
    let mut sink = StringSink::default();
    match format_source_to_sink(source, &FormatOptions::default(), &mut sink) {
        FormatSinkResult::Complete => sink.into_string(),
        FormatSinkResult::Halted => panic!("formatter unexpectedly halted"),
        FormatSinkResult::Blocked { diagnostics } => {
            panic!("formatter blocked: {diagnostics:#?}")
        }
    }
}
