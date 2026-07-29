// Single-file checker: runs the same conservation checks as hunt.rs against
// the files passed on the command line and prints failures with details.
// Run with `cargo run -p jolt_java_fmt --example check --release -- <files...>`.

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

fn main() {
    std::panic::set_hook(Box::new(|_| {}));
    let mut any = false;
    for arg in std::env::args().skip(1) {
        // `path.java:commentN` or `path.java:linecommentN` re-runs the hunt's
        // probe insertion for boundary N and prints the probed source.
        if let Some((file, probe)) = arg.rsplit_once(':')
            && (probe
                .strip_prefix("comment")
                .is_some_and(|n| n.parse::<usize>().is_ok())
                || probe
                    .strip_prefix("linecomment")
                    .is_some_and(|n| n.parse::<usize>().is_ok()))
        {
            let path = PathBuf::from(file);
            let source = fs::read_to_string(&path).expect("read input");
            let ends = token_end_offsets(&source);
            // Mirror hunt.rs's probe placement exactly.
            let (index, insert) = if let Some(n) = probe.strip_prefix("linecomment") {
                let n: usize = n.parse().expect("probe index");
                (
                    (n + 1).wrapping_mul(40503usize.wrapping_mul(source.len() + 31)) % ends.len(),
                    " //hunt\n",
                )
            } else {
                let n: usize = probe
                    .strip_prefix("comment")
                    .and_then(|n| n.parse().ok())
                    .expect("probe index");
                if ends.len() <= 120 {
                    (n, " /*hunt*/ ")
                } else {
                    (
                        n.wrapping_mul(2654435761usize.wrapping_mul(source.len() + 17))
                            % ends.len(),
                        " /*hunt*/ ",
                    )
                }
            };
            let mut probed = source.clone();
            probed.insert_str(ends[index], insert);
            println!("=== probed source (boundary {index} of {}):", ends.len());
            println!("{probed}");
            for failure in check_source(&path, &probed) {
                any = true;
                println!(
                    "[{}] {}\n{}\n",
                    failure.category,
                    failure.path.display(),
                    failure.detail
                );
            }
            continue;
        }
        let path = PathBuf::from(arg);
        for failure in check_file(&path) {
            any = true;
            println!(
                "[{}] {}\n{}\n",
                failure.category,
                failure.path.display(),
                failure.detail
            );
        }
    }
    if !any {
        println!("no failures");
    }
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

struct Failure {
    category: &'static str,
    path: PathBuf,
    detail: String,
}

fn facts(source: &str) -> CorpusParseFacts {
    let parse = parse_compilation_unit(source);
    let root = parse.syntax().and_then(|unit| unit.syntax_node());
    corpus_parse_facts(root, parse.diagnostics(), &POLICY)
}

fn format(source: &str) -> Result<String, String> {
    let mut sink = StringSink::default();
    match format_source_to_sink(source, &FormatOptions::default(), &mut sink) {
        FormatSinkResult::Complete => Ok(sink.into_string()),
        FormatSinkResult::Halted => Err("formatter halted".to_owned()),
        FormatSinkResult::Blocked { diagnostic } => {
            Err(format!("formatter blocked: {}", diagnostic.message))
        }
    }
}

fn check_file(path: &Path) -> Vec<Failure> {
    let source = fs::read_to_string(path).expect("read input");
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
            category: "panic",
            path,
            detail: message.to_owned(),
        }]
    })
}

fn check_source(path: &Path, source: &str) -> Vec<Failure> {
    let mut failures = Vec::new();
    let mut fail = |category: &'static str, detail: String| {
        failures.push(Failure {
            category,
            path: path.to_path_buf(),
            detail,
        })
    };

    let input = facts(source);
    if !input.has_tree {
        fail("no-tree", format!("diagnostics: {:?}", input.diagnostics));
        return failures;
    }
    let clean_input = input.diagnostics.is_empty();
    if !clean_input {
        fail(
            "input-dirty",
            format!("diagnostics: {:?}", input.diagnostics),
        );
    }

    let formatted = match format(source) {
        Ok(formatted) => formatted,
        Err(detail) => {
            fail("format-blocked", detail);
            return failures;
        }
    };
    let after = facts(&formatted);
    if !after.has_tree {
        fail("reparse-no-tree", String::new());
        return failures;
    }
    if clean_input && !after.diagnostics.is_empty() {
        fail(
            "reparse-dirty",
            format!("formatted output diagnostics: {:?}", after.diagnostics),
        );
    }
    if clean_input && input.structure != after.structure {
        let divergence = describe_divergence(&input.structure, &after.structure);
        fail("structure-changed", divergence);
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
    match format(&formatted) {
        Ok(repeated) if repeated != formatted => {
            fail(
                "not-idempotent",
                format!("--- first\n{formatted}\n--- second\n{repeated}"),
            );
        }
        Err(detail) => fail("reformat-blocked", detail),
        _ => {}
    }
    failures
}

fn describe_divergence(expected: &str, actual: &str) -> String {
    let prefix = expected
        .char_indices()
        .zip(actual.chars())
        .find(|((_, left), right)| left != right)
        .map_or_else(|| expected.len().min(actual.len()), |((index, _), _)| index);
    let start = prefix.saturating_sub(120);
    let end = (prefix + 120).min(expected.len()).min(actual.len());
    format!(
        "first divergence at byte {prefix}\nexpected: ...{}\nactual:   ...{}",
        &expected[start..end],
        &actual[start..end]
    )
}
