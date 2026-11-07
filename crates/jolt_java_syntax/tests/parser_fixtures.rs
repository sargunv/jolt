use std::path::Path;

use jolt_java_syntax::{JavaLanguage, ModuleDeclaration, ModuleDirective, parse};
use jolt_syntax::{AstNode, SyntaxNode, debug_tree};
use jolt_test_support::{
    SnapshotBuilder, collect_java_files, fixture_manifest, fixture_snapshot_name,
    java_fixture_root, read_to_string, render_diagnostics, workspace_root,
};

#[test]
fn syntax_fixture_snapshots() {
    let root = java_fixture_root(env!("CARGO_MANIFEST_DIR"));
    let paths = collect_java_files(&root.join("syntax"));
    insta::assert_snapshot!("fixture_manifest", fixture_manifest(&root, &paths));

    for path in paths {
        let source = read_to_string(&path);
        let parse = parse(&source);
        let snapshot = SnapshotBuilder::new()
            .section("input", &source)
            .section("syntax", debug_tree::<JavaLanguage>(parse.syntax()))
            .section("diagnostics", render_diagnostics(parse.diagnostics()))
            .finish();
        insta::assert_snapshot!(fixture_snapshot_name(&root, &path), snapshot);
    }
}

#[test]
fn every_fixture_parses_losslessly() {
    let root = java_fixture_root(env!("CARGO_MANIFEST_DIR"));
    for path in collect_java_files(&root) {
        assert_lossless(&path);
    }
}

#[test]
fn imported_suites_parse_losslessly() {
    let imports = workspace_root(env!("CARGO_MANIFEST_DIR")).join("tools/import/.imports");
    for suite in [
        "google-java-format",
        "palantir-java-format",
        "prettier-java",
    ] {
        for path in collect_java_files(&imports.join(suite).join("input")) {
            assert_lossless(&path);
        }
    }
}

fn assert_lossless(path: &Path) {
    let source = read_to_string(path);
    let parse = parse(&source);
    let mut rebuilt = String::with_capacity(source.len());
    for token in parse.syntax().tokens() {
        token
            .leading()
            .for_each(|piece| rebuilt.push_str(piece.text));
        rebuilt.push_str(token.text());
        token
            .trailing()
            .for_each(|piece| rebuilt.push_str(piece.text));
    }
    assert_eq!(rebuilt, source, "{} did not round-trip", path.display());
    if parse.diagnostics().is_empty() {
        let root = parse.syntax();
        assert!(
            root.is_recovery_free()
                && !root
                    .descendants()
                    .any(|element| element.as_node().is_some_and(SyntaxNode::is_malformed)),
            "{}: a file without diagnostics has recovery in its tree",
            path.display()
        );
    }
}

/// Token soup and pathological nesting must terminate and stay lossless. The
/// loop and depth guards are active in every build profile, so a debug run
/// covers release behavior (debug only adds assertions).
#[test]
fn adversarial_inputs_terminate_losslessly() {
    let nested = deep_nestings();
    let mut inputs = token_soups();
    inputs.extend(linear_scan_shapes());
    // Debug frames are far larger than release ones; the depth budget is
    // calibrated for release on a 1 MiB stack.
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || {
            for source in &nested {
                assert!(
                    parse(source)
                        .diagnostics()
                        .iter()
                        .any(|diagnostic| diagnostic.code == "java.parse.excessive_syntax_nesting"),
                    "{} did not reach the depth limit",
                    &source[..40]
                );
            }
            for source in nested.into_iter().chain(inputs) {
                assert_eq!(rebuild(&parse(&source)), source);
            }
        })
        .expect("spawn")
        .join()
        .expect("adversarial inputs parse");
}

fn rebuild(parse: &jolt_java_syntax::JavaParse<'_>) -> String {
    let mut rebuilt = String::new();
    for token in parse.syntax().tokens() {
        token
            .leading()
            .for_each(|piece| rebuilt.push_str(piece.text));
        rebuilt.push_str(token.text());
        token
            .trailing()
            .for_each(|piece| rebuilt.push_str(piece.text));
    }
    rebuilt
}

/// Deterministic pseudo-random sequences of Java tokens and trivia.
fn token_soups() -> Vec<String> {
    let vocabulary: Vec<&str> =
        "class|interface|enum|record|@|(|)|{|}|[|]|<|>|>>|>>>|>=|>>=|>>>=|,|;\
        |.|...|::|->|=|?|:|+|-|!|~|&&|||||&|||^|case|default|switch|if|else|for|while|do|try\
        |catch|finally|new|this|super|return|yield|var|sealed|non|permits|module|requires\
        |exports|import|package|static|final|public|void|int|x|T|1|\"s\"|'c'|/* c */|// c\n\
        |\n|instanceof|when|throws|extends|implements|_"
            .split('|')
            .collect();
    let mut inputs = Vec::new();
    let mut seed = 0x2545_f491_4f6c_dd1d_u64;
    for length in [1, 2, 3, 5, 8, 13, 40, 200, 2000] {
        for _ in 0..64 {
            let mut source = String::new();
            for _ in 0..length {
                seed ^= seed << 13;
                seed ^= seed >> 7;
                seed ^= seed << 17;
                let index = usize::try_from(seed % vocabulary.len() as u64).expect("index fits");
                source.push_str(vocabulary[index]);
                source.push(' ');
            }
            inputs.push(source);
        }
    }
    inputs
}

/// Every recursive grammar owner nested far past the depth limit.
fn deep_nestings() -> Vec<String> {
    const DEPTH: usize = 5000;
    let shapes: [(&str, &str, &str, &str); 13] = [
        ("(", "x", ")", "class C { void m() { # } }"),
        ("{", "", "}", "class C { void m() { # } }"),
        ("List<", "T", ">", "class C { # f; }"),
        ("@A(", "1", ")", "class C { # void m() {} }"),
        ("class C { ", "", " }", "#"),
        ("void m() { class D { ", "", " } }", "class C { # }"),
        ("x = ", "1", "", "class C { void m() { #; } }"),
        ("-", "1", "", "class C { void m() { x = #; } }"),
        ("new int[] {", "1", "}", "class C { void m() { x = #; } }"),
        ("if (x) ", ";", "", "class C { void m() { # } }"),
        (
            "switch (x) { case 1 -> ",
            "1;",
            " }",
            "class C { void m() { # } }",
        ),
        (
            "R(",
            "R r",
            ")",
            "class C { void m() { x = o instanceof #; } }",
        ),
        ("() -> ", "x", "", "class C { void m() { x = #; } }"),
    ];
    shapes
        .iter()
        .map(|(open, middle, close, template)| {
            let body = format!("{}{middle}{}", open.repeat(DEPTH), close.repeat(DEPTH));
            template.replace('#', &body)
        })
        .collect()
}

/// Shapes that once made header scans rescan the rest of a body per member,
/// statement, or parameter, plus long unbalanced runs.
fn linear_scan_shapes() -> Vec<String> {
    const COUNT: usize = 20_000;
    let mut inputs = Vec::new();
    for unit in ["x ", "x() ", "a b c ", "public "] {
        inputs.push(format!("class C {{ {} }}", unit.repeat(COUNT)));
    }
    for unit in ["x ", "A<"] {
        inputs.push(format!("class C {{ C() {{ {} }} }}", unit.repeat(COUNT)));
        inputs.push(format!(
            "class C {{ void m() {{ x = {} }} }}",
            unit.repeat(COUNT)
        ));
    }
    inputs.push(format!(
        "class C {{ void m({}) {{}} }}",
        "a b c ".repeat(COUNT)
    ));
    for unit in ["(", ")", "{", "}", "[", "]", "<", ">", "@A("] {
        inputs.push(unit.repeat(COUNT));
    }
    inputs
}

#[test]
fn bogus_list_items_stay_typed_items() {
    let parse = parse("module m { requires a; + ; exports p; }");
    let module = parse
        .syntax()
        .descendants()
        .filter_map(jolt_syntax::SyntaxElement::as_node)
        .find_map(ModuleDeclaration::cast)
        .expect("module declaration");
    let bogus = module
        .directives()
        .expect("directives")
        .items()
        .find_map(|directive| match directive {
            ModuleDirective::Bogus(bogus) => Some(bogus),
            _ => None,
        })
        .expect("a bogus directive item");
    assert!(bogus.syntax().is_malformed());
}
