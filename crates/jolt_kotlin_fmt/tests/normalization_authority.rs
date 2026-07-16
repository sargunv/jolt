use jolt_kotlin_fmt::{FormatOptions, FormatSinkResult, format_source_to_sink};
use jolt_kotlin_syntax::parse_kotlin_file;
use jolt_test_support::StringSink;

#[test]
fn malformed_statement_is_a_local_separator_normalization_barrier() {
    let source = "fun f() {\nfirst;\n);\nsecond;\n}\n";
    let expected = "fun f() {\n  first\n  );\n  second\n}\n";

    assert_normalizes(source, expected);
}

#[test]
fn malformed_file_item_is_a_local_separator_normalization_barrier() {
    let source = "val first = 1;\n);\nval second = 2;\n";
    let expected = "val first = 1\n\n);\nval second = 2\n";

    assert_normalizes(source, expected);
}

#[test]
fn malformed_property_member_is_a_local_separator_normalization_barrier() {
    let source = "var x: Int\nget() = 1;\n);\nset(value) {};\n";
    let expected = "var x: Int\n  get() = 1\n\n);\nset(value) {}\n";

    assert_normalizes(source, expected);
}

#[test]
fn malformed_binary_owner_denies_parentheses_but_valid_neighbor_inserts_them() {
    let source = "val broken = a custom b +\nval clean = a custom b + c\n";
    let expected = "val broken = a custom b +\n\nval clean = a custom (b + c)\n";

    assert_normalizes(source, expected);
}

fn assert_normalizes(source: &str, expected: &str) {
    let before = diagnostic_inventory(source);
    let formatted = format(source);
    assert_eq!(formatted, expected);
    assert_eq!(diagnostic_inventory(&formatted), before);
    assert_eq!(format(&formatted), formatted);
}

fn format(source: &str) -> String {
    let mut sink = StringSink::default();
    assert_eq!(
        format_source_to_sink(source, &FormatOptions::default(), &mut sink),
        FormatSinkResult::Complete
    );
    sink.into_string()
}

fn diagnostic_inventory(source: &str) -> Vec<(String, String)> {
    parse_kotlin_file(source)
        .diagnostics()
        .iter()
        .map(|diagnostic| {
            (
                diagnostic.code.as_str().to_owned(),
                diagnostic.message.clone(),
            )
        })
        .collect()
}
