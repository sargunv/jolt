use std::fmt::Write as _;

use jolt_kotlin_syntax::parse_kotlin_file;

#[test]
fn malformed_repeated_constructs_make_bounded_progress() {
    let small = assert_reconstructs(&progress_case(64));
    let large = assert_reconstructs(&progress_case(256));

    assert!(
        large < small * 6,
        "parser debug tree grew nonlinearly for repeated malformed input"
    );
}

#[test]
fn repeated_generic_delegation_specifiers_make_bounded_progress() {
    let small = assert_reconstructs(&generic_delegation_case(32));
    let large = assert_reconstructs(&generic_delegation_case(128));

    assert!(
        large < small * 6,
        "parser debug tree grew nonlinearly for repeated generic delegation specifiers"
    );
}

#[test]
fn newline_expression_starts_make_bounded_progress() {
    let small = assert_reconstructs(&newline_expression_start_case(64));
    let large = assert_reconstructs(&newline_expression_start_case(256));

    assert!(
        large < small * 6,
        "parser debug tree grew nonlinearly for repeated newline expression starts"
    );
}

#[test]
fn repeated_line_start_primary_expressions_make_bounded_progress() {
    let small = assert_reconstructs(&line_start_primary_expression_case(32));
    let large = assert_reconstructs(&line_start_primary_expression_case(128));

    assert!(
        large < small * 6,
        "parser debug tree grew nonlinearly for repeated line-start primary expressions"
    );
}

#[test]
fn repeated_malformed_generic_call_suffixes_make_bounded_progress() {
    let small = assert_reconstructs(&malformed_generic_call_suffix_case(32));
    let large = assert_reconstructs(&malformed_generic_call_suffix_case(128));

    assert!(
        large < small * 6,
        "parser debug tree grew nonlinearly for repeated malformed generic call suffixes"
    );
}

#[test]
fn repeated_malformed_control_flow_makes_bounded_progress() {
    let small = assert_reconstructs(&malformed_control_flow_case(32));
    let large = assert_reconstructs(&malformed_control_flow_case(128));

    assert!(
        large < small * 6,
        "parser debug tree grew nonlinearly for repeated malformed control flow"
    );
}

fn progress_case(repeated_commas: usize) -> String {
    format!(
        "{}\n{}\n{}\n{}\n",
        "class BrokenWhere<T> where : { fun body() = Unit }",
        "class BrokenDelegation : , , , { val value = 1 }",
        "val property: String get set private get() = \"ok\"",
        repeated_commas_collection(repeated_commas),
    )
}

fn generic_delegation_case(count: usize) -> String {
    let mut source = String::new();
    for index in 0..count {
        writeln!(
            source,
            "interface Broken{index}<T> : Base<List<T>>, , Comparable<Broken{index}<T>>"
        )
        .expect("writing to a String should not fail");
    }
    source
}

fn newline_expression_start_case(count: usize) -> String {
    let mut source = String::from("fun starts(flag: Boolean) {\n");
    for index in 0..count {
        writeln!(source, "    val value{index} = flag")
            .expect("writing to a String should not fail");
        source.push_str("    (if (flag) (0 until 10) else (10 downTo 0)).forEach { item ->\n");
        source.push_str("        item.toString()\n");
        source.push_str("    }\n");
    }
    source.push_str("}\n");
    source
}

fn line_start_primary_expression_case(count: usize) -> String {
    let mut source = String::from("fun starts(flag: Boolean, error: Throwable) {\n");
    for index in 0..count {
        writeln!(source, "    val parenthesized{index} = flag")
            .expect("writing to a String should not fail");
        source.push_str("    (if (flag) 0 else 1).toString()\n");
        writeln!(source, "    val conditional{index} = flag")
            .expect("writing to a String should not fail");
        source.push_str("    if (flag) 0 else 1\n");
        writeln!(source, "    val matched{index} = flag")
            .expect("writing to a String should not fail");
        source.push_str("    when (flag) { true -> 1 false -> 0 }\n");
        writeln!(source, "    val handled{index} = flag")
            .expect("writing to a String should not fail");
        source.push_str("    try { 1 } catch (cause: Throwable) { 0 }\n");
        writeln!(source, "    val looped{index} = flag")
            .expect("writing to a String should not fail");
        source.push_str("    while (flag) break\n");
        writeln!(source, "    val raised{index} = flag")
            .expect("writing to a String should not fail");
        source.push_str("    throw error\n");
    }
    source.push_str("}\n");
    source
}

fn malformed_generic_call_suffix_case(count: usize) -> String {
    let mut source = String::from("fun calls(target: Target) {\n");
    for index in 0..count {
        writeln!(source, "    val call{index} = target<,>().next()")
            .expect("writing to a String should not fail");
        writeln!(source, "    val reference{index} = target::<,>member")
            .expect("writing to a String should not fail");
    }
    source.push_str("}\n");
    source
}

fn malformed_control_flow_case(count: usize) -> String {
    let mut source = String::from("fun malformed(value: Any) {\n");
    for _ in 0..count {
        source.push_str("when (value) { , value missing\nnext -> }\n");
        source.push_str("try {} finally {} catch {}\n");
        source.push_str("for (in) while do {} (value)\n");
    }
    source.push_str("}\n");
    source
}

fn assert_reconstructs(source: &str) -> usize {
    let parse = parse_kotlin_file(source);
    let syntax = parse.syntax().expect("parser should build a syntax tree");

    assert_eq!(syntax.source_text(), source);
    format!("{parse:#?}").len()
}

fn repeated_commas_collection(count: usize) -> String {
    format!("val commas = [{}]", ",".repeat(count))
}
