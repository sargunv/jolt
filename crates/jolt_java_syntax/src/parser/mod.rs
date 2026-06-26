#[cfg(test)]
mod tests;

mod grammar;
mod source;

use std::fmt;

use jolt_diagnostics::{
    Diagnostic, DiagnosticCode, DiagnosticCodeId, DiagnosticStage, Severity, SyntaxOutcome,
};
use jolt_syntax::{
    GreenTokenSource, GreenTriviaPiece, SyntaxElement, SyntaxNode, SyntaxToken,
    TriviaKind as GreenTriviaKind, build_green_tree,
};

use crate::{JavaLanguage, JavaLexer, JavaSyntaxKind, Token, Trivia};

use self::source::{Parser, ParserToken};

/// A Java syntax node.
pub type JavaSyntaxNode = SyntaxNode<JavaLanguage>;

/// A Java syntax token.
pub type JavaSyntaxToken = SyntaxToken<JavaLanguage>;

/// A Java syntax node or token.
pub type JavaSyntaxElement = SyntaxElement<JavaLanguage>;

/// Stable Java parser diagnostic codes.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum JavaParseDiagnosticCode {
    ExpectedSyntax,
    UnexpectedSyntax,
    InvalidStatementExpression,
    InvalidResourceVariableAccess,
    InvalidSwitchGuard,
    UnqualifiedYieldMethodInvocation,
    DecimalIntegerBoundaryLiteral,
    MisplacedReceiverParameter,
    MisplacedConstructorInvocation,
    RestrictedTypeIdentifier,
    InvalidEventStream,
}

impl DiagnosticCode for JavaParseDiagnosticCode {
    fn id(&self) -> DiagnosticCodeId {
        match self {
            Self::ExpectedSyntax => DiagnosticCodeId::new("java.parse.expected_syntax"),
            Self::UnexpectedSyntax => DiagnosticCodeId::new("java.parse.unexpected_syntax"),
            Self::InvalidStatementExpression => {
                DiagnosticCodeId::new("java.parse.invalid_statement_expression")
            }
            Self::InvalidResourceVariableAccess => {
                DiagnosticCodeId::new("java.parse.invalid_resource_variable_access")
            }
            Self::InvalidSwitchGuard => DiagnosticCodeId::new("java.parse.invalid_switch_guard"),
            Self::UnqualifiedYieldMethodInvocation => {
                DiagnosticCodeId::new("java.parse.unqualified_yield_method_invocation")
            }
            Self::DecimalIntegerBoundaryLiteral => {
                DiagnosticCodeId::new("java.parse.decimal_integer_boundary_literal")
            }
            Self::MisplacedReceiverParameter => {
                DiagnosticCodeId::new("java.parse.misplaced_receiver_parameter")
            }
            Self::MisplacedConstructorInvocation => {
                DiagnosticCodeId::new("java.parse.misplaced_constructor_invocation")
            }
            Self::RestrictedTypeIdentifier => {
                DiagnosticCodeId::new("java.parse.restricted_type_identifier")
            }
            Self::InvalidEventStream => {
                DiagnosticCodeId::new("internal.syntax.invalid_event_stream")
            }
        }
    }
}

/// The result of parsing Java source text.
pub struct JavaParse {
    syntax: Option<JavaSyntaxNode>,
    diagnostics: Vec<Diagnostic>,
    outcome: SyntaxOutcome,
}

impl JavaParse {
    /// Returns the parsed syntax tree root.
    #[must_use]
    pub const fn syntax(&self) -> Option<&JavaSyntaxNode> {
        self.syntax.as_ref()
    }

    /// Returns parser diagnostics.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Returns the syntax production outcome.
    #[must_use]
    pub const fn outcome(&self) -> SyntaxOutcome {
        self.outcome
    }

    /// Splits this parse result into its syntax root, diagnostics, and outcome.
    #[must_use]
    pub fn into_parts(self) -> (Option<JavaSyntaxNode>, Vec<Diagnostic>, SyntaxOutcome) {
        (self.syntax, self.diagnostics, self.outcome)
    }
}

impl fmt::Debug for JavaParse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "syntax:")?;
        if let Some(syntax) = &self.syntax {
            writeln!(f, "{syntax:?}")?;
        } else {
            writeln!(f, "  (none)")?;
        }

        writeln!(f)?;
        writeln!(f, "outcome: {:?}", self.outcome)?;

        writeln!(f)?;
        writeln!(f, "diagnostics:")?;
        if self.diagnostics.is_empty() {
            writeln!(f, "  (none)")?;
        } else {
            for diagnostic in &self.diagnostics {
                fmt_diagnostic(f, diagnostic)?;
            }
        }

        Ok(())
    }
}

fn fmt_diagnostic(f: &mut fmt::Formatter<'_>, diagnostic: &Diagnostic) -> fmt::Result {
    write!(
        f,
        "  code={} severity={:?} stage={:?}",
        diagnostic.code, diagnostic.severity, diagnostic.stage
    )?;
    if let Some(range) = diagnostic.range {
        write!(f, " range={range}")?;
    } else {
        write!(f, " range=<none>")?;
    }
    writeln!(f, " message={:?}", diagnostic.message)
}

/// Parses a Java compilation unit.
#[must_use]
pub fn parse_compilation_unit(source: &str) -> JavaParse {
    let (tokens, diagnostics) = lex_tokens(source);
    let parse = Parser::new(source, tokens).parse_compilation_unit();
    finish_parse(source, diagnostics, &parse)
}

fn finish_parse(
    source: &str,
    mut diagnostics: Vec<Diagnostic>,
    parse: &source::ParseEvents,
) -> JavaParse {
    let token_source = JavaGreenTokenSource::new(source, &parse.tokens);
    let tree = match build_green_tree(&parse.events, &token_source) {
        Ok(tree) => tree,
        Err(error) => {
            diagnostics.push(invalid_event_stream_diagnostic(&error));
            return JavaParse {
                syntax: None,
                diagnostics,
                outcome: SyntaxOutcome::Aborted,
            };
        }
    };
    let (root, parser_diagnostics) = tree.into_parts();
    diagnostics.extend(parser_diagnostics);
    let outcome = if diagnostics.is_empty() {
        SyntaxOutcome::Clean
    } else {
        SyntaxOutcome::Recovered
    };

    JavaParse {
        syntax: Some(JavaSyntaxNode::new_root(root)),
        diagnostics,
        outcome,
    }
}

fn invalid_event_stream_diagnostic(error: &jolt_syntax::BuildGreenTreeError) -> Diagnostic {
    Diagnostic {
        code: JavaParseDiagnosticCode::InvalidEventStream.id(),
        severity: Severity::InternalError,
        stage: DiagnosticStage::Parser,
        message: format!("Jolt parser produced an invalid event stream: {error:?}"),
        range: None,
    }
}

fn lex_tokens(source: &str) -> (Vec<Token>, Vec<Diagnostic>) {
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
