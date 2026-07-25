use jolt_fmt_ir::FormatOptions;
use jolt_java_fmt::format_source_to_sink;
use jolt_java_syntax::{JavaSyntaxKind, JavaSyntaxView, parse_compilation_unit};
use jolt_test_support::{
    CorpusLanguage, CorpusParseFacts, StructurePolicy, corpus_parse_facts, format_source_or_panic,
};

/// The only tree edits the Java formatter is allowed to make: it sorts imports and
/// modifiers, promotes a bare statement to a block, drops redundant separators, and
/// may normalize separator punctuation.
const STRUCTURE_POLICY: StructurePolicy<JavaSyntaxKind> = StructurePolicy {
    normalizable_punctuation: &[
        JavaSyntaxKind::Comma,
        JavaSyntaxKind::Semicolon,
        JavaSyntaxKind::LBrace,
        JavaSyntaxKind::RBrace,
        JavaSyntaxKind::LParen,
        JavaSyntaxKind::RParen,
    ],
    unordered_nodes: &[
        JavaSyntaxKind::ModifierList,
        JavaSyntaxKind::ModuleDirectiveList,
        JavaSyntaxKind::RequiresModifierList,
    ],
    reorderable_children: &[JavaSyntaxKind::ImportDeclaration],
    // Promoting `if (c) stmt;` to `if (c) { stmt; }` interposes this whole wrapper
    // chain, so each link is transparent when it holds a lone child. Eliding
    // `ParenthesizedExpression` costs no precedence coverage: operator nesting lives
    // in the `BinaryExpression` spine, so dropping a paren that actually mattered
    // still reshapes that spine and still fails.
    elidable_wrappers: &[
        JavaSyntaxKind::Block,
        JavaSyntaxKind::BlockStatementList,
        JavaSyntaxKind::BlockStatement,
        JavaSyntaxKind::ParenthesizedExpression,
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

    fn expects_parser_diagnostics(&self, relative: &str) -> bool {
        let Some(name) = relative.strip_prefix("syntax/parser/") else {
            return false;
        };
        name.starts_with("diagnoses-")
            || name.starts_with("recovers-")
            || name == "disambiguates-when-in-switch-labels--invalid-guard.java"
    }
}
