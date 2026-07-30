// Shared by several test binaries, each of which uses a different part.
#![allow(dead_code)]

use jolt_fmt_ir::{FormatOptions, FormatSinkResult};
use jolt_kotlin_fmt::format_source_to_sink;
use jolt_kotlin_syntax::{KotlinSyntaxKind, KotlinSyntaxView, parse_kotlin_file};
use jolt_test_support::{
    CorpusLanguage, CorpusParseFacts, StringSink, StructurePolicy, corpus_parse_facts,
    format_source_or_panic,
};

/// The only tree edits the Kotlin formatter is allowed to make: it sorts the import
/// list, adds clarifying precedence parentheses, and may normalize separators.
pub(crate) const STRUCTURE_POLICY: StructurePolicy<KotlinSyntaxKind> = StructurePolicy {
    normalizable_punctuation: &[
        KotlinSyntaxKind::Comma,
        // `;;` is not in the syntax layer's separator-removal vocabulary, so the
        // formatter must keep it and the fingerprint must stay sensitive to it.
        KotlinSyntaxKind::Semicolon,
        KotlinSyntaxKind::EolOrSemicolon,
        KotlinSyntaxKind::LParen,
        KotlinSyntaxKind::RParen,
    ],
    unordered_nodes: &[KotlinSyntaxKind::ImportDirectiveList],
    unordered_keywords: &[],
    unordered_keywords_ordered_children: &[],
    reorderable_children: &[],
    // Eliding `ParenthesizedExpression` costs no precedence coverage: operator nesting
    // lives in the `BinaryExpression` spine, so a paren that actually mattered still
    // reshapes that spine and still fails.
    elidable_wrappers: &[KotlinSyntaxKind::ParenthesizedExpression],
    promoted_body_wrapper: None,
    brace_promoting_parents: &[],
    elidable_nodes: &[],
};

pub(crate) struct KotlinCorpus;

impl CorpusLanguage for KotlinCorpus {
    fn language_name(&self) -> &'static str {
        "Kotlin"
    }

    fn token_end_offsets(&self, source: &str) -> Vec<usize> {
        parse_kotlin_file(source)
            .syntax()
            .and_then(|file| file.syntax_node())
            .map(|root| {
                // A string literal lexes into an `OpenQuote`, its parts, and a
                // `ClosingQuote`, so the boundaries between them are inside the
                // literal and cannot hold trivia. Depth counting also drops the
                // interpolation code inside `${...}`, where a comment would be
                // legal: a real coverage limit, taken because the alternative is
                // tracking template-entry nesting for a handful of positions.
                let mut offsets = Vec::new();
                let mut string_depth = 0usize;
                for token in root.tokens().filter(|token| !token.text().is_empty()) {
                    match token.kind() {
                        KotlinSyntaxKind::OpenQuote => string_depth += 1,
                        KotlinSyntaxKind::ClosingQuote => {
                            string_depth = string_depth.saturating_sub(1);
                        }
                        _ => {}
                    }
                    if string_depth == 0 {
                        offsets.push(token.text_range().end().get());
                    }
                }
                offsets
            })
            .unwrap_or_default()
    }

    fn parse_facts(&self, source: &str) -> CorpusParseFacts {
        let parse = parse_kotlin_file(source);
        let root = parse.syntax().and_then(|file| file.syntax_node());
        corpus_parse_facts(root, parse.diagnostics(), &STRUCTURE_POLICY)
    }

    fn format(&self, source: &str, label: &str) -> String {
        format_source_or_panic(
            format_source_to_sink,
            source,
            &FormatOptions::default(),
            label,
        )
    }

    fn try_format(&self, source: &str, options: &FormatOptions) -> Result<String, String> {
        let mut sink = StringSink::default();
        match format_source_to_sink(source, options, &mut sink) {
            FormatSinkResult::Complete => Ok(sink.into_string()),
            FormatSinkResult::Halted => Err("formatter halted".to_owned()),
            FormatSinkResult::Blocked { diagnostic } => {
                Err(format!("formatter blocked: {}", diagnostic.message))
            }
        }
    }

    fn expects_parser_diagnostics(&self, relative: &str) -> bool {
        let Some(name) = relative.strip_prefix("syntax/parser/") else {
            return false;
        };
        name.starts_with("diagnoses-")
            || name.starts_with("recovers-")
            || name == "parses-destructuring-square-preview.kt"
    }
}
