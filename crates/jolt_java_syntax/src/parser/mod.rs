#[cfg(test)]
mod tests;

use jolt_syntax::{
    BuildGreenTreeError, Event, GreenTokenSource, GreenTriviaPiece, ParseDiagnostic, SyntaxElement,
    SyntaxNode, SyntaxToken, TriviaKind as GreenTriviaKind, build_green_tree,
};

use crate::{JavaLanguage, JavaLexer, JavaSyntaxKind, LexerDiagnostic, Token, Trivia};

/// A Java syntax node.
pub type JavaSyntaxNode = SyntaxNode<JavaLanguage>;

/// A Java syntax token.
pub type JavaSyntaxToken = SyntaxToken<JavaLanguage>;

/// A Java syntax node or token.
pub type JavaSyntaxElement = SyntaxElement<JavaLanguage>;

/// The result of parsing Java source text.
pub struct JavaParse {
    syntax: JavaSyntaxNode,
    diagnostics: Vec<ParseDiagnostic>,
    lexer_diagnostics: Vec<LexerDiagnostic>,
}

impl JavaParse {
    /// Returns the parsed syntax tree root.
    #[must_use]
    pub const fn syntax(&self) -> &JavaSyntaxNode {
        &self.syntax
    }

    /// Returns parser diagnostics.
    #[must_use]
    pub fn diagnostics(&self) -> &[ParseDiagnostic] {
        &self.diagnostics
    }

    /// Returns lexer diagnostics produced while parsing.
    #[must_use]
    pub fn lexer_diagnostics(&self) -> &[LexerDiagnostic] {
        &self.lexer_diagnostics
    }

    /// Splits this parse result into its syntax root and diagnostics.
    #[must_use]
    pub fn into_parts(self) -> (JavaSyntaxNode, Vec<ParseDiagnostic>, Vec<LexerDiagnostic>) {
        (self.syntax, self.diagnostics, self.lexer_diagnostics)
    }
}

/// Parses a Java compilation unit.
///
/// This is currently an API shell: it creates a lossless `CompilationUnit`
/// root and leaves grammar structure for the real parser implementation.
///
/// # Panics
///
/// Panics if the parser shell emits an internally invalid event stream.
#[must_use]
pub fn parse_compilation_unit(source: &str) -> JavaParse {
    // Temporary parser shell: collect the streaming lexer output so the existing
    // shared event sink can read tokens by index. The public parser API should
    // remain compatible with a future direct streaming implementation.
    let (tokens, lexer_diagnostics) = lex_tokens(source);
    let token_source = JavaGreenTokenSource::new(source, &tokens);
    let tree = build_compilation_unit_tree(&token_source)
        .expect("parser shell must emit structurally valid green tree events");
    let (root, diagnostics) = tree.into_parts();

    JavaParse {
        syntax: JavaSyntaxNode::new_root(root),
        diagnostics,
        lexer_diagnostics,
    }
}

fn lex_tokens(source: &str) -> (Vec<Token>, Vec<LexerDiagnostic>) {
    let mut lexer = JavaLexer::new(source);
    let mut tokens = Vec::new();

    loop {
        let token = lexer.next_token();
        let at_eof = token.kind == JavaSyntaxKind::Eof;
        tokens.push(token);

        if at_eof {
            break;
        }
    }

    let diagnostics = lexer.finish();

    (tokens, diagnostics)
}

fn build_compilation_unit_tree(
    token_source: &JavaGreenTokenSource<'_>,
) -> Result<jolt_syntax::GreenTree, BuildGreenTreeError> {
    let mut events = Vec::with_capacity(token_source.token_count().saturating_add(2));
    events.push(Event::start_node(JavaSyntaxKind::CompilationUnit.to_raw()));
    events.extend((0..token_source.token_count()).map(|_| Event::Token));
    events.push(Event::FinishNode);

    build_green_tree(&events, token_source)
}

struct JavaGreenTokenSource<'source> {
    source: &'source str,
    tokens: &'source [Token],
}

impl<'source> JavaGreenTokenSource<'source> {
    const fn new(source: &'source str, tokens: &'source [Token]) -> Self {
        Self { source, tokens }
    }

    fn token(&self, index: usize) -> &Token {
        &self.tokens[index]
    }

    fn trivia_text(&self, trivia: &Trivia) -> &'source str {
        &self.source[trivia.range.start().get()..trivia.range.end().get()]
    }
}

impl GreenTokenSource for JavaGreenTokenSource<'_> {
    fn token_count(&self) -> usize {
        self.tokens.len()
    }

    fn token_kind(&self, index: usize) -> jolt_syntax::RawSyntaxKind {
        self.token(index).kind.to_raw()
    }

    fn token_text(&self, index: usize) -> &str {
        let range = self.token(index).range;
        &self.source[range.start().get()..range.end().get()]
    }

    fn leading_trivia(&self, index: usize) -> impl Iterator<Item = GreenTriviaPiece<'_>> {
        self.token(index).leading.iter().map(|trivia| {
            GreenTriviaPiece::new(to_green_trivia_kind(trivia.kind), self.trivia_text(trivia))
        })
    }

    fn trailing_trivia(&self, index: usize) -> impl Iterator<Item = GreenTriviaPiece<'_>> {
        self.token(index).trailing.iter().map(|trivia| {
            GreenTriviaPiece::new(to_green_trivia_kind(trivia.kind), self.trivia_text(trivia))
        })
    }
}

fn to_green_trivia_kind(kind: crate::TriviaKind) -> GreenTriviaKind {
    match kind {
        crate::TriviaKind::Whitespace => GreenTriviaKind::Whitespace,
        crate::TriviaKind::Newline => GreenTriviaKind::Newline,
        crate::TriviaKind::LineComment => GreenTriviaKind::LineComment,
        crate::TriviaKind::BlockComment => GreenTriviaKind::BlockComment,
        crate::TriviaKind::JavadocComment => GreenTriviaKind::DocComment,
        crate::TriviaKind::Ignored => GreenTriviaKind::Ignored,
    }
}
