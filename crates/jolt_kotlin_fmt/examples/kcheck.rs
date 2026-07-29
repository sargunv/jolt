// Single-file checker: runs the same conservation checks as hunt.rs against
// the files passed on the command line and prints failures with details.
// Run with `cargo run -p jolt_kotlin_fmt --example check --release -- <files...>`.

use std::fs;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};

use jolt_fmt_ir::{FormatOptions, FormatSinkResult};
use jolt_kotlin_fmt::format_source_to_sink;
use jolt_kotlin_syntax::{KotlinSyntaxKind, KotlinSyntaxView, parse_kotlin_file};
use jolt_test_support::{
    CorpusParseFacts, StringSink, StructurePolicy, corpus_parse_facts, diagnostic_inventory,
};

const POLICY: StructurePolicy<KotlinSyntaxKind> = StructurePolicy {
    normalizable_punctuation: &[
        KotlinSyntaxKind::Comma,
        KotlinSyntaxKind::Semicolon,
        KotlinSyntaxKind::EolOrSemicolon,
        KotlinSyntaxKind::LParen,
        KotlinSyntaxKind::RParen,
    ],
    unordered_nodes: &[KotlinSyntaxKind::ImportDirectiveList],
    unordered_keywords: &[],
    unordered_keywords_ordered_children: &[],
    reorderable_children: &[],
    elidable_wrappers: &[KotlinSyntaxKind::ParenthesizedExpression],
    promoted_body_wrapper: None,
    brace_promoting_parents: &[],
    elidable_nodes: &[],
};

fn main() {
    std::panic::set_hook(Box::new(|_| {}));
    let mut any = false;
    for arg in std::env::args().skip(1) {
        // `path.kt:commentN` re-runs the hunt's probe insertion for boundary N
        // and prints the probed source.
        if let Some((file, probe)) = arg.rsplit_once(':')
            && probe
                .strip_prefix("comment")
                .is_some_and(|n| n.parse::<usize>().is_ok())
        {
            let path = PathBuf::from(file);
            let source = fs::read_to_string(&path).expect("read input");
            let ends = token_end_offsets(&source);
            let n: usize = probe
                .strip_prefix("comment")
                .and_then(|n| n.parse().ok())
                .expect("probe index");
            // Mirror hunt.rs's probe placement exactly.
            let index = if ends.len() <= 120 {
                n
            } else {
                n.wrapping_mul(2654435761usize.wrapping_mul(source.len() + 17)) % ends.len()
            };
            let mut probed = source.clone();
            probed.insert_str(ends[index], " /*hunt*/ ");
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
    parse_kotlin_file(source)
        .syntax()
        .and_then(|file| file.syntax_node())
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
    let parse = parse_kotlin_file(source);
    let root = parse.syntax().and_then(|file| file.syntax_node());
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
