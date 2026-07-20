use jolt_java_fmt::{FormatOptions, FormatSinkResult, format_source_to_sink};
use jolt_java_syntax::parse_compilation_unit;
use jolt_test_support::StringSink;

#[test]
fn malformed_import_and_control_owners_are_local_normalization_barriers() {
    let source = "import z.Z;\nimport broken +;\nimport c.C;\nimport a.A;\nclass C { void f(boolean ok) { if () ; while (ok) ; } }\n";
    let expected = "import z.Z;\n\nimport broken+;\n\nimport a.A;\nimport c.C;\n\nclass C {\n  void f(boolean ok) {\n    if () ;\n    while (ok) {\n    }\n  }\n}\n";

    assert_normalizes(source, expected);
}

#[test]
fn malformed_module_directive_is_a_local_reorder_barrier() {
    let source = "module m { uses z.Z; uses y.Y; requires +; uses c.C; uses a.A; }\n";
    let expected =
        "module m {\n  uses y.Y;\n  uses z.Z;\n\n  requires +;\n\n  uses a.A;\n  uses c.C;\n}\n";

    assert_normalizes(source, expected);
}

#[test]
fn requires_modifiers_ignore_malformed_sibling_fields() {
    let source = "module m { requires transitive static + a.b; }\n";
    let expected = "module m {\n  requires static transitive + a.b;\n}\n";

    assert_normalizes(source, expected);
}

#[test]
fn malformed_binary_owner_denies_parentheses_but_valid_neighbor_inserts_them() {
    let source = "class C { boolean f(int a, int b) { return a & b == ; } boolean g(int a, int b) { return a & b == 0; } }\n";
    let expected = "class C {\n  boolean f(int a, int b) {\n    return a & b == ;\n  }\n\n  boolean g(int a, int b) {\n    return a & (b == 0);\n  }\n}\n";

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
    parse_compilation_unit(source)
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
