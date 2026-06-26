#[cfg(test)]
mod tests;

mod grammar;
mod source;

use std::fmt;

use jolt_syntax::{
    GreenTokenSource, GreenTriviaPiece, ParseDiagnostic, ParseDiagnosticKind, SyntaxElement,
    SyntaxNode, SyntaxToken, TriviaKind as GreenTriviaKind, build_green_tree,
};

use crate::{JavaLanguage, JavaLexer, JavaSyntaxKind, LexerDiagnostic, Token, Trivia};

use self::source::{Parser, ParserToken};

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

impl fmt::Debug for JavaParse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "syntax:")?;
        writeln!(f, "{:?}", self.syntax)?;

        writeln!(f)?;
        writeln!(f, "parser diagnostics:")?;
        if self.diagnostics.is_empty() {
            writeln!(f, "  (none)")?;
        } else {
            for diagnostic in &self.diagnostics {
                write!(f, "  ")?;
                fmt_parse_diagnostic_kind(f, diagnostic.kind())?;
                writeln!(f)?;
            }
        }

        writeln!(f)?;
        writeln!(f, "lexer diagnostics:")?;
        if self.lexer_diagnostics.is_empty() {
            writeln!(f, "  (none)")?;
        } else {
            for diagnostic in &self.lexer_diagnostics {
                writeln!(f, "  {:?}", diagnostic.kind)?;
            }
        }

        Ok(())
    }
}

fn fmt_parse_diagnostic_kind(
    f: &mut fmt::Formatter<'_>,
    kind: &ParseDiagnosticKind,
) -> fmt::Result {
    match kind {
        ParseDiagnosticKind::Message(message) => f.write_str(message),
    }
}

/// Parses a Java compilation unit.
///
/// # Panics
///
/// Panics if the parser emits an internally invalid event stream.
#[must_use]
pub fn parse_compilation_unit(source: &str) -> JavaParse {
    let (tokens, lexer_diagnostics) = lex_tokens(source);
    let parse = Parser::new(source, tokens).parse_compilation_unit();
    let token_source = JavaGreenTokenSource::new(source, &parse.tokens);
    let tree = build_green_tree(&parse.events, &token_source)
        .expect("parser must emit structurally valid green tree events");
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

struct JavaGreenTokenSource<'source> {
    source: &'source str,
    tokens: &'source [ParserToken],
}

impl<'source> JavaGreenTokenSource<'source> {
    const fn new(source: &'source str, tokens: &'source [ParserToken]) -> Self {
        Self { source, tokens }
    }

    fn token(&self, index: usize) -> &ParserToken {
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
