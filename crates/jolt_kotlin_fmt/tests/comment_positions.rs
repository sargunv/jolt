use jolt_fmt_ir::{FormatOptions, FormatSinkResult};
use jolt_kotlin_fmt::format_source_to_sink;
use jolt_test_support::{StringSink, assert_comments_format_at_every_token_position};

mod common;

use common::KotlinCorpus;

/// Constructs whose tokens sit next to a rule that formats them explicitly.
const SOURCES: &[&str] = &[
    "class C : Base() { val a = 1 }",
    "class C<T : Any> { fun <R> f(x: R): R = x }",
    "interface I { fun f(): Int }",
    "object O { val a = 1 }",
    "enum class E { A, B }",
    "fun f(a: Int, b: String): Int { return a }",
    "val x: Int = 1",
    "val r = { a: Int -> a }",
    "fun f() { g(1, 2) }",
    "val v = ( 1 + 2 )",
    "fun f() { if (true) { g() } else { h() } }",
    "fun f(x: Int) { when (x) { 1 -> g() else -> h() } }",
    "fun f() { try { g() } catch (e: E) { h() } finally { i() } }",
    "fun f() { for (i in 1..2) { g() } while (true) { h() } }",
    "typealias A = List<Int>",
    "import a.b.C",
    "@Anno class C",
    "class C { val x by lazy { 1 } }",
    "fun f(a: Int?): String? = null",
    "fun f(vararg a: Int) { }",
    "class C(val a: Int) { constructor() : this(1) }",
    "fun f(a: Any) { val b = a as String }",
    "fun f(a: Int?) { val b = a ?: 0 }",
];

#[test]
fn comments_format_at_every_token_position() {
    assert_comments_format_at_every_token_position(
        &KotlinCorpus,
        |source| {
            let mut sink = StringSink::default();
            match format_source_to_sink(source, &FormatOptions::default(), &mut sink) {
                FormatSinkResult::Complete => Ok(sink.into_string()),
                FormatSinkResult::Halted => panic!("formatter halted with StringSink"),
                FormatSinkResult::Blocked { diagnostic } => Err(diagnostic),
            }
        },
        SOURCES,
    );
}
