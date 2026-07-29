// Bug-hunting harness: runs the formatter with full conservation checks over
// the imported real-world/adversarial corpora in tools/import/.imports.
// Not part of the test suite; run with `cargo run -p jolt_java_fmt --example hunt --release`.
//
// Checks per file, at several line widths: clean reparse of formatted output,
// structure fingerprint conservation, comment inventory conservation, and
// idempotence. Additionally injects a block comment at pseudo-random token
// boundaries and re-runs the same checks on the probed source.

use std::fs;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};

use jolt_fmt_ir::{FormatOptions, FormatSinkResult};
use jolt_java_fmt::format_source_to_sink;
use jolt_java_syntax::{JavaSyntaxKind, JavaSyntaxView, parse_compilation_unit};
use jolt_test_support::{
    CorpusParseFacts, StringSink, StructurePolicy, corpus_parse_facts, diagnostic_inventory,
};

const POLICY: StructurePolicy<JavaSyntaxKind> = StructurePolicy {
    normalizable_punctuation: &[
        JavaSyntaxKind::Comma,
        JavaSyntaxKind::Semicolon,
        JavaSyntaxKind::LBrace,
        JavaSyntaxKind::RBrace,
        JavaSyntaxKind::LParen,
        JavaSyntaxKind::RParen,
    ],
    unordered_nodes: &[
        JavaSyntaxKind::ModuleDirectiveList,
        JavaSyntaxKind::RequiresModifierList,
    ],
    unordered_keywords: &[JavaSyntaxKind::ModifierList],
    unordered_keywords_ordered_children: &[JavaSyntaxKind::Annotation],
    reorderable_children: &[JavaSyntaxKind::ImportDeclaration],
    elidable_wrappers: &[
        JavaSyntaxKind::BlockStatementList,
        JavaSyntaxKind::BlockStatement,
        JavaSyntaxKind::ParenthesizedExpression,
    ],
    promoted_body_wrapper: Some(JavaSyntaxKind::Block),
    brace_promoting_parents: &[
        JavaSyntaxKind::IfStatement,
        JavaSyntaxKind::WhileStatement,
        JavaSyntaxKind::DoStatement,
        JavaSyntaxKind::BasicForStatement,
        JavaSyntaxKind::EnhancedForStatement,
    ],
    elidable_nodes: &[
        JavaSyntaxKind::EmptyStatement,
        JavaSyntaxKind::EmptyDeclaration,
    ],
};

const WIDTHS: &[u16] = &[40, 80, 120];
const COMMENT_PROBES: usize = 3;

struct Failure {
    category: String,
    path: PathBuf,
    detail: String,
}

fn main() {
    // Panics are caught per file and recorded; keep the output readable.
    std::panic::set_hook(Box::new(|_| {}));

    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .to_path_buf();
    let imports = root.join("tools/import/.imports");
    let mut files = Vec::new();
    for suite in [
        "google-java-format/input",
        "palantir-java-format/input",
        "prettier-java/input",
        "spring-framework",
    ] {
        collect(&imports.join(suite), &mut files);
    }
    files.sort();
    files.retain(|path| !path.ends_with("B26952926.java")); // intentionally invalid upstream

    let out_dir = root.join("target/hunt/java");
    let _ = fs::remove_dir_all(&out_dir);
    fs::create_dir_all(&out_dir).unwrap();

    let threads = std::thread::available_parallelism().map_or(4, usize::from);
    let chunk_size = files.len().div_ceil(threads);
    let files: &[PathBuf] = &files;
    let (mut all_failures, mut total_slow): (Vec<Failure>, Vec<(PathBuf, u128)>) =
        std::thread::scope(|scope| {
            let mut handles = Vec::new();
            for chunk in files.chunks(chunk_size.max(1)) {
                handles.push(scope.spawn(move || {
                    let mut failures = Vec::new();
                    let mut slow = Vec::new();
                    for path in chunk {
                        let started = std::time::Instant::now();
                        failures.extend(check_file(path));
                        let elapsed = started.elapsed().as_millis();
                        if elapsed > 1000 {
                            slow.push((path.clone(), elapsed));
                        }
                    }
                    (failures, slow)
                }));
            }
            let mut all_failures = Vec::new();
            let mut total_slow = Vec::new();
            for handle in handles {
                let (failures, slow) = handle.join().unwrap();
                all_failures.extend(failures);
                total_slow.extend(slow);
            }
            (all_failures, total_slow)
        });

    all_failures.sort_by(|a, b| (&a.category, &a.path).cmp(&(&b.category, &b.path)));
    all_failures
        .dedup_by(|a, b| a.category == b.category && a.path == b.path && a.detail == b.detail);
    let mut report = String::new();
    let mut counters: std::collections::BTreeMap<String, usize> = Default::default();
    for (index, failure) in all_failures.iter().enumerate() {
        *counters.entry(failure.category.clone()).or_default() += 1;
        report.push_str(&format!(
            "[{}] {}\n    {}\n",
            failure.category,
            failure.path.display(),
            failure.detail.replace('\n', "\n    ")
        ));
        let repro_dir = out_dir.join(&failure.category);
        fs::create_dir_all(&repro_dir).unwrap();
        if let Ok(source) = fs::read_to_string(&failure.path) {
            fs::write(
                repro_dir.join(format!(
                    "{index:04}-{}",
                    failure.path.file_name().unwrap().to_string_lossy()
                )),
                format!("// repro from: {}\n{source}", failure.path.display()),
            )
            .unwrap();
        }
    }
    report.push_str(&format!("\nsummary: {counters:?}\n"));
    total_slow.sort_by_key(|(_, ms)| std::cmp::Reverse(*ms));
    for (path, ms) in total_slow.iter().take(20) {
        report.push_str(&format!("slow: {ms}ms {}\n", path.display()));
    }
    fs::write(out_dir.join("report.txt"), &report).unwrap();
    println!(
        "checked {} files, {} failures: {:?}",
        files.len(),
        all_failures.len(),
        counters
    );
    println!("report: {}", out_dir.join("report.txt").display());
}

fn collect(dir: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, files);
        } else if path.extension().is_some_and(|ext| ext == "java") {
            files.push(path);
        }
    }
}

fn facts(source: &str) -> CorpusParseFacts {
    let parse = parse_compilation_unit(source);
    let root = parse.syntax().and_then(|unit| unit.syntax_node());
    corpus_parse_facts(root, parse.diagnostics(), &POLICY)
}

fn format(source: &str, options: &FormatOptions) -> Result<String, String> {
    let mut sink = StringSink::default();
    match format_source_to_sink(source, options, &mut sink) {
        FormatSinkResult::Complete => Ok(sink.into_string()),
        FormatSinkResult::Halted => Err("formatter halted".to_owned()),
        FormatSinkResult::Blocked { diagnostic } => {
            Err(format!("formatter blocked: {}", diagnostic.message))
        }
    }
}

fn check_file(path: &Path) -> Vec<Failure> {
    let source = match fs::read_to_string(path) {
        Ok(source) => source,
        Err(error) => {
            return vec![Failure {
                category: "read-error".to_owned(),
                path: path.to_path_buf(),
                detail: error.to_string(),
            }];
        }
    };
    let path = path.to_path_buf();
    let closure_path = path.clone();
    catch_unwind(AssertUnwindSafe(move || {
        check_source(&closure_path, &source)
    }))
    .unwrap_or_else(|payload| {
        let message = payload
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| payload.downcast_ref::<&str>().copied())
            .unwrap_or("<non-string panic>");
        vec![Failure {
            category: "panic".to_owned(),
            path,
            detail: message.to_owned(),
        }]
    })
}

fn check_source(path: &Path, source: &str) -> Vec<Failure> {
    let mut failures = Vec::new();
    for &width in WIDTHS {
        let options = FormatOptions {
            line_width: width,
            ..FormatOptions::default()
        };
        check_variant(path, source, &options, &format!("w{width}"), &mut failures);
    }

    // Inject comments at token boundaries and re-check. Small files get a
    // block comment at every boundary; large files get pseudo-random probes,
    // plus a couple of line-comment probes everywhere.
    let ends = token_end_offsets(source);
    if !ends.is_empty() {
        let options = FormatOptions::default();
        if ends.len() <= 120 {
            for (probe, &end) in ends.iter().enumerate() {
                let mut probed = source.to_owned();
                probed.insert_str(end, " /*hunt*/ ");
                check_variant(
                    path,
                    &probed,
                    &options,
                    &format!("comment{probe}"),
                    &mut failures,
                );
            }
        } else {
            for probe in 0..COMMENT_PROBES {
                let index = (probe * 2654435761usize.wrapping_mul(source.len() + 17)) % ends.len();
                let mut probed = source.to_owned();
                probed.insert_str(ends[index], " /*hunt*/ ");
                check_variant(
                    path,
                    &probed,
                    &options,
                    &format!("comment{probe}"),
                    &mut failures,
                );
            }
        }
        for probe in 0..2 {
            let index = ((probe + 1) * 40503usize.wrapping_mul(source.len() + 31)) % ends.len();
            let mut probed = source.to_owned();
            probed.insert_str(ends[index], " //hunt\n");
            check_variant(
                path,
                &probed,
                &options,
                &format!("linecomment{probe}"),
                &mut failures,
            );
        }
    }
    failures
}

fn token_end_offsets(source: &str) -> Vec<usize> {
    parse_compilation_unit(source)
        .syntax()
        .and_then(|unit| unit.syntax_node())
        .map(|root| {
            root.tokens()
                .filter(|token| !token.text().is_empty())
                .map(|token| token.text_range().end().get())
                .collect()
        })
        .unwrap_or_default()
}

fn check_variant(
    path: &Path,
    source: &str,
    options: &FormatOptions,
    label: &str,
    failures: &mut Vec<Failure>,
) {
    let mut fail = |category: &str, detail: String| {
        failures.push(Failure {
            category: format!("{label}:{category}"),
            path: path.to_path_buf(),
            detail,
        })
    };

    let input = facts(source);
    if !input.has_tree {
        fail("no-tree", format!("diagnostics: {:?}", input.diagnostics));
        return;
    }
    let clean_input = input.diagnostics.is_empty();

    let formatted = match format(source, options) {
        Ok(formatted) => formatted,
        Err(detail) => {
            fail("format-blocked", detail);
            return;
        }
    };
    let after = facts(&formatted);
    if !after.has_tree {
        fail("reparse-no-tree", String::new());
        return;
    }
    if clean_input && !after.diagnostics.is_empty() {
        fail(
            "reparse-dirty",
            format!("formatted output diagnostics: {:?}", after.diagnostics),
        );
    }
    if !clean_input
        && diagnostic_inventory(&input.diagnostics) != diagnostic_inventory(&after.diagnostics)
    {
        fail("diag-inventory-changed", String::new());
    }
    if clean_input && input.structure != after.structure {
        fail("structure-changed", String::new());
    }
    if input.comment_inventory != after.comment_inventory {
        fail(
            "comments-changed",
            format!(
                "expected: {:?}\nactual: {:?}",
                input.comment_inventory, after.comment_inventory
            ),
        );
    }
    match format(&formatted, options) {
        Ok(repeated) if repeated != formatted => {
            fail(
                "not-idempotent",
                format!("--- first\n{formatted}\n--- second\n{repeated}"),
            );
        }
        Err(detail) => fail("reformat-blocked", detail),
        _ => {}
    }
}
