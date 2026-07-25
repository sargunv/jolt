use jolt_fmt_ir::{FormatOptions, FormatSinkResult};
use jolt_java_fmt::format_source_to_sink;
use jolt_test_support::{StringSink, assert_comments_format_at_every_token_position};

mod common;

use common::JavaCorpus;

/// Constructs whose tokens sit next to a rule that formats them explicitly.
const SOURCES: &[&str] = &[
    "class C extends Base { int a = 1; }",
    "class C<T extends Object> { <R> R f(R x) { return x; } }",
    "interface I { int f(); }",
    "enum E { A, B }",
    "class C { int f(int a, String b) { return a; } }",
    "class C { Runnable r = () -> run(); }",
    "class C { int v = ( 1 + 2 ); }",
    "class C { void f() { if (true) { g(); } else { h(); } } }",
    "class C { void f() { try { g(); } catch (E e) { h(); } finally { i(); } } }",
    "class C { void f() { for (int i = 0; i < 2; i++) { g(); } while (true) { h(); } } }",
    "import a.b.C;",
    "@Anno class C { }",
    "class C { void f(Object a) { String b = (String) a; } }",
    "record R(int a) { }",
    "class C { int f(boolean b) { return b ? 1 : 2; } }",
    "class C { int[] a = {1, 2}; void f(int... v) { } }",
    "class C { void f() throws E { } }",
    "class C { synchronized void f() { assert true; } }",
];

#[test]
fn comments_format_at_every_token_position() {
    assert_comments_format_at_every_token_position(
        &JavaCorpus,
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
