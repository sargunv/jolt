use jolt_kotlin_syntax::parse_kotlin_file;

#[test]
fn malformed_repeated_constructs_make_bounded_progress() {
    let small = assert_reconstructs(progress_case(64));
    let large = assert_reconstructs(progress_case(256));

    assert!(
        large < small * 6,
        "parser debug tree grew nonlinearly for repeated malformed input"
    );
}

#[test]
fn repeated_generic_delegation_specifiers_make_bounded_progress() {
    let small = assert_reconstructs(generic_delegation_case(32));
    let large = assert_reconstructs(generic_delegation_case(128));

    assert!(
        large < small * 6,
        "parser debug tree grew nonlinearly for repeated generic delegation specifiers"
    );
}

#[test]
fn newline_expression_starts_make_bounded_progress() {
    let small = assert_reconstructs(newline_expression_start_case(64));
    let large = assert_reconstructs(newline_expression_start_case(256));

    assert!(
        large < small * 6,
        "parser debug tree grew nonlinearly for repeated newline expression starts"
    );
}

#[test]
fn repeated_line_start_primary_expressions_make_bounded_progress() {
    let small = assert_reconstructs(line_start_primary_expression_case(32));
    let large = assert_reconstructs(line_start_primary_expression_case(128));

    assert!(
        large < small * 6,
        "parser debug tree grew nonlinearly for repeated line-start primary expressions"
    );
}

#[test]
fn repeated_malformed_generic_call_suffixes_make_bounded_progress() {
    let small = assert_reconstructs(malformed_generic_call_suffix_case(32));
    let large = assert_reconstructs(malformed_generic_call_suffix_case(128));

    assert!(
        large < small * 6,
        "parser debug tree grew nonlinearly for repeated malformed generic call suffixes"
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
        source.push_str(&format!(
            "interface Broken{index}<T> : Base<List<T>>, , Comparable<Broken{index}<T>>\n"
        ));
    }
    source
}

fn newline_expression_start_case(count: usize) -> String {
    let mut source = String::from("fun starts(flag: Boolean) {\n");
    for index in 0..count {
        source.push_str(&format!("    val value{index} = flag\n"));
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
        source.push_str(&format!("    val parenthesized{index} = flag\n"));
        source.push_str("    (if (flag) 0 else 1).toString()\n");
        source.push_str(&format!("    val conditional{index} = flag\n"));
        source.push_str("    if (flag) 0 else 1\n");
        source.push_str(&format!("    val matched{index} = flag\n"));
        source.push_str("    when (flag) { true -> 1 false -> 0 }\n");
        source.push_str(&format!("    val handled{index} = flag\n"));
        source.push_str("    try { 1 } catch (cause: Throwable) { 0 }\n");
        source.push_str(&format!("    val looped{index} = flag\n"));
        source.push_str("    while (flag) break\n");
        source.push_str(&format!("    val raised{index} = flag\n"));
        source.push_str("    throw error\n");
    }
    source.push_str("}\n");
    source
}

fn malformed_generic_call_suffix_case(count: usize) -> String {
    let mut source = String::from("fun calls(target: Target) {\n");
    for index in 0..count {
        source.push_str(&format!("    val call{index} = target<,>().next()\n"));
        source.push_str(&format!("    val reference{index} = target::<,>member\n"));
    }
    source.push_str("}\n");
    source
}

fn assert_reconstructs(source: String) -> usize {
    let parse = parse_kotlin_file(&source);
    let syntax = parse.syntax().expect("parser should build a syntax tree");

    assert_eq!(syntax.source_text(), source);
    format!("{parse:#?}").len()
}

fn repeated_commas_collection(count: usize) -> String {
    format!("val commas = [{}]", ",".repeat(count))
}
