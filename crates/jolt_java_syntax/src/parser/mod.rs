mod grammar;
mod source;

use jolt_syntax::{
    AstNode, Diagnostic, ParseOutput, SyntaxNode, SyntaxTree, TextRange, TextSize, Trivia,
    build_tree,
};

use crate::lexer::{NormalizedJavaSource, normalize_unicode_escapes};
use crate::{CompilationUnit, JavaSyntaxFactory};

use self::source::Parser;

/// Stable Java parser diagnostic codes.
pub(crate) mod codes {
    pub(crate) const EXPECTED: &str = "java.parse.expected_syntax";
    pub(crate) const UNEXPECTED: &str = "java.parse.unexpected_syntax";
    pub(crate) const TOO_DEEP: &str = "java.parse.excessive_syntax_nesting";
    pub(crate) const INVALID_STATEMENT_EXPRESSION: &str = "java.parse.invalid_statement_expression";
    pub(crate) const INVALID_RESOURCE_VARIABLE_ACCESS: &str =
        "java.parse.invalid_resource_variable_access";
    pub(crate) const INVALID_SWITCH_GUARD: &str = "java.parse.invalid_switch_guard";
    pub(crate) const UNQUALIFIED_YIELD_METHOD_INVOCATION: &str =
        "java.parse.unqualified_yield_method_invocation";
    pub(crate) const MISPLACED_RECEIVER_PARAMETER: &str = "java.parse.misplaced_receiver_parameter";
    pub(crate) const MISPLACED_CONSTRUCTOR_INVOCATION: &str =
        "java.parse.misplaced_constructor_invocation";
    pub(crate) const RESTRICTED_TYPE_IDENTIFIER: &str = "java.parse.restricted_type_identifier";
}

/// A parsed Java compilation unit. The tree always refers to the raw source:
/// Unicode escapes are translated into a temporary buffer for lexing, and
/// token and trivia ranges are mapped back to the raw text.
pub struct JavaParse<'s> {
    source: &'s str,
    tree: SyntaxTree,
    diagnostics: Vec<Diagnostic>,
}

impl<'s> JavaParse<'s> {
    /// # Panics
    ///
    /// Never: the parser always produces a compilation unit root.
    #[must_use]
    pub fn root(&self) -> CompilationUnit<'_> {
        CompilationUnit::cast(self.syntax()).expect("the root is a compilation unit")
    }

    #[must_use]
    pub fn syntax(&self) -> SyntaxNode<'_> {
        SyntaxNode::root(&self.tree, self.source)
    }

    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    #[must_use]
    pub const fn source(&self) -> &'s str {
        self.source
    }
}

/// Parses a Java compilation unit.
#[must_use]
pub fn parse(source: &str) -> JavaParse<'_> {
    let mut normalized = normalize_unicode_escapes(source);
    let mut output = Parser::new(normalized.source()).parse_compilation_unit();
    let mut diagnostics = normalized.take_diagnostics();
    if normalized.has_replacements() {
        remap_to_raw_source(&normalized, &mut output);
    }
    let (tree, mut parsed) = build_tree(source, output, &JavaSyntaxFactory);
    diagnostics.append(&mut parsed);
    JavaParse {
        source,
        tree,
        diagnostics,
    }
}

fn remap_to_raw_source(normalized: &NormalizedJavaSource<'_>, output: &mut ParseOutput) {
    normalized.remap_diagnostics(&mut output.diagnostics);
    for token in &mut output.tokens {
        let mut offset = token.full_start();
        for index in token.leading() {
            offset = remap_trivia(normalized, &mut output.trivia, index, offset);
        }
        offset = token.range().end();
        for index in token.trailing() {
            offset = remap_trivia(normalized, &mut output.trivia, index, offset);
        }
        let full_start = normalized
            .raw_range(TextRange::empty(token.full_start()))
            .start();
        token.set_ranges(full_start, normalized.raw_range(token.range()));
    }
}

fn remap_trivia(
    normalized: &NormalizedJavaSource<'_>,
    trivia: &mut [Trivia],
    index: usize,
    start: TextSize,
) -> TextSize {
    let piece = trivia[index];
    let end = start + TextSize::from(piece.len);
    let raw = normalized.raw_range(TextRange::new(start, end));
    trivia[index] = Trivia::new(piece.kind, raw.len());
    end
}
