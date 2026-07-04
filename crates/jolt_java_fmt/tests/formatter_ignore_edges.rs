use jolt_java_fmt::JavaFormatOptions;

mod support;
use support::format_source;

#[test]
fn ignored_range_can_end_at_eof_without_trailing_newline() {
    assert_formats(
        "// @formatter:off\nclass Raw {int x=1+2;}\n// @formatter:on",
        "// @formatter:off\nclass Raw {int x=1+2;}\n// @formatter:on\n",
    );
}

#[test]
fn crlf_ignored_range_preserves_raw_source_with_normalized_output_lines() {
    assert_formats(
        concat!(
            "class Example {\r\n",
            "void run() {\r\n",
            "int before=1;\r\n",
            "// @formatter:off\r\n",
            "int raw=1+2;\r\n",
            "call( a,b );\r\n",
            "// @formatter:on\r\n",
            "int after=3;\r\n",
            "}\r\n",
            "}\r\n",
        ),
        concat!(
            "class Example {\n",
            "  void run() {\n",
            "    int before = 1;\n",
            "    // @formatter:off\n",
            "    int raw=1+2;\n",
            "    call( a,b );\n",
            "    // @formatter:on\n",
            "    int after = 3;\n",
            "  }\n",
            "}\n",
        ),
    );
}

#[track_caller]
fn assert_formats(input: &str, expected: &str) {
    let result = format_source(input, &JavaFormatOptions::default());
    assert!(
        result.diagnostics.is_empty(),
        "formatter diagnostics: {:#?}",
        result.diagnostics
    );
    let formatted = result
        .formatted_source
        .as_deref()
        .expect("formatter should produce output");
    assert_eq!(formatted, expected);

    let repeated = format_source(formatted, &JavaFormatOptions::default());
    assert!(
        repeated.diagnostics.is_empty(),
        "formatter diagnostics after repeat format: {:#?}",
        repeated.diagnostics
    );
    assert_eq!(
        repeated.formatted_source.as_deref(),
        Some(expected),
        "formatter output should be idempotent"
    );
}
