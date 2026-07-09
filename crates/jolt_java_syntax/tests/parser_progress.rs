use std::fmt::Write as _;

use jolt_java_syntax::parse_compilation_unit;

#[test]
fn repeated_malformed_members_make_bounded_progress() {
    let small = assert_reconstructs(&malformed_members_case(32));
    let large = assert_reconstructs(&malformed_members_case(128));

    assert!(
        large < small * 6,
        "parser debug tree grew nonlinearly for repeated malformed members"
    );
}

#[test]
fn repeated_malformed_switch_and_resource_constructs_make_bounded_progress() {
    let small = assert_reconstructs(&malformed_statement_case(16));
    let large = assert_reconstructs(&malformed_statement_case(64));

    assert!(
        large < small * 6,
        "parser debug tree grew nonlinearly for repeated malformed statements"
    );
}

fn malformed_members_case(count: usize) -> String {
    let mut source = String::from("class Progress {\n");
    for index in 0..count {
        writeln!(source, "  List<,> field{index};").expect("writing to a String should not fail");
        writeln!(source, "  void method{index}(int a, , int b) {{}}")
            .expect("writing to a String should not fail");
        writeln!(source, "  <T, , U> T generic{index}() throws {{}}")
            .expect("writing to a String should not fail");
    }
    source.push_str("}\n");
    source
}

fn malformed_statement_case(count: usize) -> String {
    let mut source = String::from("class Progress { void run(Object value) {\n");
    for index in 0..count {
        writeln!(source, "  call{index}(first, , second);")
            .expect("writing to a String should not fail");
        writeln!(
            source,
            "  switch (value) {{ case , null -> call{index}(); }}"
        )
        .expect("writing to a String should not fail");
        writeln!(
            source,
            "  try (var resource{index} = ; ) {{ call{index}(); }}"
        )
        .expect("writing to a String should not fail");
    }
    source.push_str("} }\n");
    source
}

fn assert_reconstructs(source: &str) -> usize {
    let parse = parse_compilation_unit(source);
    let syntax = parse.syntax().expect("parser should build a syntax tree");

    assert_eq!(syntax.source_text(), source);
    format!("{parse:#?}").len()
}
