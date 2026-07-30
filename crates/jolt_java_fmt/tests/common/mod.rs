// Shared by several test binaries, each of which uses a different part.
#![allow(dead_code)]

use jolt_fmt_ir::{FormatOptions, FormatSinkResult};
use jolt_java_fmt::format_source_to_sink;
use jolt_java_syntax::{JavaSyntaxKind, JavaSyntaxView, parse_compilation_unit};
use jolt_test_support::{
    CorpusLanguage, CorpusParseFacts, StringSink, StructurePolicy, corpus_parse_facts,
    format_source_or_panic,
};

/// The only tree edits the Java formatter is allowed to make: it sorts imports and
/// modifiers, promotes a bare statement to a block, drops redundant separators, and
/// may normalize separator punctuation.
pub(crate) const STRUCTURE_POLICY: StructurePolicy<JavaSyntaxKind> = StructurePolicy {
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
    // Only declaration modifiers are canonicalized. A `ParameterModifierList` admits
    // just `final` plus annotations, so it has no keyword order to canonicalize, and the
    // formatter preserves whichever spelling the source used.
    unordered_keywords: &[JavaSyntaxKind::ModifierList],
    // Annotations keep declaration order among themselves but may cross the
    // node-shaped `non-sealed` modifier when the formatter sorts the list.
    unordered_keywords_ordered_children: &[JavaSyntaxKind::Annotation],
    reorderable_children: &[JavaSyntaxKind::ImportDeclaration],
    // `BlockStatementList` and `BlockStatement` are the list plumbing that brace
    // promotion interposes; neither owns a brace, so eliding them cannot hide a lost
    // boundary. Eliding `ParenthesizedExpression` costs no precedence coverage either:
    // operator nesting lives in the `BinaryExpression` spine, so dropping a paren that
    // actually mattered still reshapes that spine and still fails.
    elidable_wrappers: &[
        JavaSyntaxKind::BlockStatementList,
        JavaSyntaxKind::BlockStatement,
        JavaSyntaxKind::ParenthesizedExpression,
    ],
    // A `Block` owns real braces, so it is transparent only where the formatter may
    // have synthesized it. A block written anywhere else stays in the fingerprint, and
    // flattening it -- which would change what a declaration inside it scopes over --
    // still fails.
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

pub(crate) struct JavaCorpus;

impl CorpusLanguage for JavaCorpus {
    fn language_name(&self) -> &'static str {
        "Java"
    }

    fn token_end_offsets(&self, source: &str) -> Vec<usize> {
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

    fn parse_facts(&self, source: &str) -> CorpusParseFacts {
        let parse = parse_compilation_unit(source);
        let root = parse.syntax().and_then(|unit| unit.syntax_node());
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
            || name == "disambiguates-when-in-switch-labels--invalid-guard.java"
    }
}
