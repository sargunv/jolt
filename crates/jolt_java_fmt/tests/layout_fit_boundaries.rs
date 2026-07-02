use jolt_java_fmt::{JavaFormatOptions, format_source};

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

#[test]
fn fit_does_not_measure_past_statement_boundary() {
    let argument = ident(54);
    assert_formats_to(
        &format!(
            r#"
class Example {{
  void method() {{
    call({argument} ? true : false);
    next();
  }}
}}
"#
        ),
        &format!(
            "class Example {{\n  void method() {{\n    call({argument} ? true : false);\n    next();\n  }}\n}}\n"
        ),
        80,
    );
}

#[test]
fn local_variable_boundary_fix_keeps_nested_ternary_flat() {
    let condition = ident(45);
    assert_formats_to(
        &format!(
            r#"
class Example {{
  void method() {{
    boolean value = {condition} ? true : false;
  }}
}}
"#
        ),
        &format!(
            "class Example {{\n  void method() {{\n    boolean value =\n      {condition} ? true : false;\n  }}\n}}\n"
        ),
        80,
    );
}

#[test]
fn call_boundary_fix_keeps_single_argument_ternary_flat() {
    let condition = ident(55);
    assert_formats_to(
        &format!(
            r#"
class Example {{
  void method() {{
    call({condition} ? true : false);
  }}
}}
"#
        ),
        &format!(
            "class Example {{\n  void method() {{\n    call(\n      {condition} ? true : false\n    );\n  }}\n}}\n"
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

fn assert_formats_to(source: &str, expected: &str, line_width: u16) {
    let formatted = format_or_panic(source, line_width);
    assert_eq!(formatted, expected);
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
