//! Stack-depth tests: unbounded left-deep spines on a small stack, and the
//! deepest nesting of each recursive construct (past the parser's depth
//! limit, so its recovery is exercised too) on the 1 MiB WASM stack.

use jolt_fmt::FormatOptions;
use jolt_syntax::MAX_DEPTH;
use jolt_test_support::check_source;

mod common;

fn on_stack(stack: usize, sources: Vec<String>) {
    std::thread::Builder::new()
        .stack_size(stack)
        .spawn(move || {
            for source in sources {
                let failures =
                    check_source(&common::JavaCorpus, &source, &FormatOptions::default());
                assert!(
                    failures.is_empty(),
                    "oracle failures for {}…: {:?}",
                    &source[..source.len().min(120)],
                    failures.iter().map(|(check, _)| check).collect::<Vec<_>>()
                );
                let formatted = common::format(&source, FormatOptions::default());
                assert!(formatted.contains("int following;"), "{formatted}");
            }
        })
        .expect("spawn test thread")
        .join()
        .expect("formatting overflowed or panicked");
}

#[test]
fn deep_spines_format_on_a_small_stack() {
    let depth = 20_000;
    let field = |suffix: &str| {
        format!(
            "class C {{ Object value = root{}; int following; }}",
            suffix.repeat(depth)
        )
    };
    let chain = format!(
        "class C {{ void m() {{ if (a) {{}}{} }} int following; }}",
        " else if (b) {}".repeat(depth)
    );
    // javac reads nested inline tags by recursion; the formatter prints a
    // comment that nests them this deep as written.
    let tags = format!("{}text{}", "{@summary ".repeat(depth), "}".repeat(depth));
    on_stack(
        256 * 1024,
        vec![
            chain,
            format!("class C {{\n  /** {tags} */\n  int following;\n}}"),
            format!("class C {{\n  /// {tags}\n  int following;\n}}"),
            format!("class C {{\n  /** <a title=\"{tags}\">link</a> */\n  int following;\n}}"),
            field(" + value"),
            field(".method()"),
            field("[0]"),
            field(".new Inner()"),
            field(".field"),
            field(".field.method()[0].new Inner()"),
        ],
    );
}

#[test]
fn deepest_nesting_formats_on_the_wasm_stack() {
    let depth = 4096;
    let nested = |open: &str, close: &str, leaf: &str| {
        format!("{}{leaf}{}", open.repeat(depth), close.repeat(depth))
    };
    let field = |value: String| format!("class C {{ Object value = {value}; int following; }}");
    let statement = |body: String| format!("class C {{ void m() {{ {body} }} int following; }}");
    on_stack(
        1024 * 1024,
        vec![
            field(nested("(", ")", "leaf")),
            field(nested("f(", ")", "leaf")),
            field(nested("!", "", "leaf")),
            field(nested("a = ", "", "leaf")),
            field(nested("c ? d : ", "", "leaf")),
            field(nested("() -> ", "", "leaf")),
            field(nested("new int[] {", "}", "1")),
            field(nested("new Object() { Object o = ", "; }", "leaf")),
            statement(nested("{ ", " }", "sibling();")),
            statement(nested("if (a) ", "", "b();")),
            // Brace promotion must not nest the output deeper than the input
            // (two levels per `if` would pass `MAX_DEPTH`).
            statement(format!(
                "{}b();",
                "if (a) ".repeat(MAX_DEPTH as usize / 2 + 6)
            )),
            statement(nested("label: ", "", ";")),
            format!(
                "{} int following; {}",
                nested("class C { ", " }", ""),
                "class D {}"
            ),
            format!(
                "class C {{ {} value; int following; }}",
                nested("T<", ">", "Leaf")
            ),
            format!(
                "{}@Leaf{} class C {{ int following; }}",
                "@A(".repeat(depth),
                ")".repeat(depth)
            ),
        ],
    );
}
