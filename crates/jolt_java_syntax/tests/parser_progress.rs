use jolt_java_syntax::parse_compilation_unit;

#[test]
fn repeated_malformed_members_make_bounded_progress() {
    let small = assert_reconstructs(malformed_members_case(32));
    let large = assert_reconstructs(malformed_members_case(128));

    assert!(
        large < small * 6,
        "parser debug tree grew nonlinearly for repeated malformed members"
    );
}

#[test]
fn repeated_malformed_switch_and_resource_constructs_make_bounded_progress() {
    let small = assert_reconstructs(malformed_statement_case(16));
    let large = assert_reconstructs(malformed_statement_case(64));

    assert!(
        large < small * 6,
        "parser debug tree grew nonlinearly for repeated malformed statements"
    );
}

fn malformed_members_case(count: usize) -> String {
    let mut source = String::from("class Progress {\n");
    for index in 0..count {
        source.push_str(&format!("  List<,> field{index};\n"));
        source.push_str(&format!("  void method{index}(int a, , int b) {{}}\n"));
        source.push_str(&format!("  <T, , U> T generic{index}() throws {{}}\n"));
    }
    source.push_str("}\n");
    source
}

fn malformed_statement_case(count: usize) -> String {
    let mut source = String::from("class Progress { void run(Object value) {\n");
    for index in 0..count {
        source.push_str(&format!("  call{index}(first, , second);\n"));
        source.push_str(&format!(
            "  switch (value) {{ case , null -> call{index}(); }}\n"
        ));
        source.push_str(&format!(
            "  try (var resource{index} = ; ) {{ call{index}(); }}\n"
        ));
    }
    source.push_str("} }\n");
    source
}

fn assert_reconstructs(source: String) -> usize {
    let parse = parse_compilation_unit(&source);
    let syntax = parse.syntax().expect("parser should build a syntax tree");

    assert_eq!(syntax.source_text(), source);
    format!("{parse:#?}").len()
}
