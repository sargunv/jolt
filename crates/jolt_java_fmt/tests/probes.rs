//! Every comment probe shape at every token boundary of the comment and
//! trivia fixtures and of a list of small constructs, checked with the
//! oracle. The sweep (`mise run hunt`) covers the imported corpora.

use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use jolt_fmt::FormatOptions;
use jolt_test_support::{
    CorpusLanguage, PROBE_SHAPES, ProbeShape, collect_java_files, java_fixture_root, parse_facts,
    probed_source, read_to_string, sweep_variant,
};

mod common;

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
    "package p; import b.B; import a.A; import static c.C; class C { void f() { a.b().c(d, e + f * g); } }",
];

fn probe_all(
    language: &common::JavaCorpus,
    source: &str,
    label: &str,
    shape: &ProbeShape,
    failures: &mut Vec<String>,
) {
    let clean = parse_facts(language, source).diagnostics.is_empty();
    let options = FormatOptions::default();
    for boundary in language.token_end_offsets(source) {
        {
            let probed = probed_source(source, boundary, shape);
            if clean && !parse_facts(language, &probed).diagnostics.is_empty() {
                continue;
            }
            for failure in sweep_variant(language, &probed, &options, shape.name) {
                failures.push(format!(
                    "{label} @{boundary} [{}]\n{probed}\n{}",
                    failure.category, failure.detail
                ));
            }
        }
    }
}

#[test]
fn comment_probes_pass_the_oracle() {
    let root = java_fixture_root(env!("CARGO_MANIFEST_DIR"));
    let mut cases: Vec<(String, String)> = Vec::new();
    for dir in ["style/comments", "trivia"] {
        for path in collect_java_files(&root.join(dir)) {
            let label = path
                .strip_prefix(&root)
                .unwrap_or(&path)
                .display()
                .to_string();
            cases.push((label, read_to_string(&path)));
        }
    }
    cases.extend(
        SOURCES
            .iter()
            .map(|source| ((*source).to_owned(), (*source).to_owned())),
    );
    let next = AtomicUsize::new(0);
    let failures = Mutex::new(Vec::new());
    let threads = std::thread::available_parallelism().map_or(4, usize::from);
    std::thread::scope(|scope| {
        for _ in 0..threads {
            scope.spawn(|| {
                loop {
                    let index = next.fetch_add(1, Ordering::Relaxed);
                    let Some((label, source)) = cases.get(index / PROBE_SHAPES.len()) else {
                        break;
                    };
                    let shape = &PROBE_SHAPES[index % PROBE_SHAPES.len()];
                    let mut found = Vec::new();
                    probe_all(&common::JavaCorpus, source, label, shape, &mut found);
                    failures.lock().expect("failure list").extend(found);
                }
            });
        }
    });
    let failures = failures.into_inner().expect("failure list");
    assert!(
        failures.is_empty(),
        "{} probe failures:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
