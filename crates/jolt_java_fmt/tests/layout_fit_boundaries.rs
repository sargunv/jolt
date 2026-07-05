use jolt_java_fmt::JavaFormatOptions;

mod support;
use support::format_source;

#[test]
fn expression_statement_semicolon_participates_in_assignment_fit() {
    assert_no_line_exceeds_width(
        r#"
class Example {
  void method() {
    currentEstimate = (currentEstimate + xxxxxxxxxxxxx / currentEstimate) / 2.0f;
  }
}
"#,
        80,
    );
}

#[test]
fn return_statement_semicolon_participates_in_ternary_fit() {
    assert_no_line_exceeds_width(
        &format!(
            r#"
class Example {{
  boolean method() {{
    return {} ? true : false;
  }}
}}
"#,
            ident(54)
        ),
        80,
    );
}

#[test]
fn local_variable_semicolon_participates_in_initializer_fit() {
    assert_no_line_exceeds_width(
        &format!(
            r#"
class Example {{
  void method() {{
    boolean value = {} ? true : false;
  }}
}}
"#,
            ident(45)
        ),
        80,
    );
}

#[test]
fn field_semicolon_participates_in_initializer_fit() {
    assert_no_line_exceeds_width(
        &format!(
            r#"
class Example {{
  boolean value = {} ? true : false;
}}
"#,
            ident(47)
        ),
        80,
    );
}

#[test]
fn expression_statement_semicolon_participates_in_call_fit() {
    assert_no_line_exceeds_width(
        &format!(
            r#"
class Example {{
  void method() {{
    call({} ? true : false);
  }}
}}
"#,
            ident(55)
        ),
        80,
    );
}

fn assert_no_line_exceeds_width(source: &str, line_width: u16) {
    let formatted = format_or_panic(source, line_width);
    let offending = formatted
        .lines()
        .enumerate()
        .map(|(index, line)| (index + 1, line, line.chars().count()))
        .find(|(_, _, width)| *width > usize::from(line_width));

    assert!(
        offending.is_none(),
        "formatted line exceeded width {line_width}:\n{}\nfirst offending line: {:?}",
        formatted,
        offending
    );
}

fn format_or_panic(source: &str, line_width: u16) -> String {
    let options = JavaFormatOptions {
        line_width,
        ..JavaFormatOptions::default()
    };
    let result = format_source(source, &options);

    assert!(
        result.diagnostics.is_empty(),
        "formatter diagnostics: {:#?}",
        result.diagnostics
    );

    result
        .formatted_source
        .expect("formatter should produce output")
}

fn ident(width: usize) -> String {
    "x".repeat(width)
}
