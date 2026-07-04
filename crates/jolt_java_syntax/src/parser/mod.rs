#[cfg(test)]
mod tests;

mod grammar;
mod source;

use std::fmt;

use jolt_diagnostics::{
    Diagnostic, DiagnosticCode, DiagnosticCodeId, DiagnosticStage, Severity, SyntaxOutcome,
};
use jolt_syntax::{
    GreenTokenSource, GreenTriviaPiece, TriviaKind as GreenTriviaKind, build_green_tree,
};
use jolt_text::TextSize;

use crate::{
    CompilationUnit, Trivia,
    lexer::normalize_unicode_escapes,
    nodes::{JavaSyntaxNode, cast_compilation_unit},
};

use self::source::{Parser, ParserToken};

/// Stable Java parser diagnostic codes.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum JavaParseDiagnosticCode {
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
    source: String,
    root: Option<jolt_syntax::GreenNode>,
    diagnostics: Vec<Diagnostic>,
    outcome: SyntaxOutcome,
}

impl JavaParse {
    /// Returns the parsed syntax tree root.
    #[must_use]
    pub fn syntax(&self) -> Option<CompilationUnit<'_>> {
        self.root
            .clone()
            .and_then(|root| cast_compilation_unit(JavaSyntaxNode::new_root(&self.source, root)))
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
}

impl fmt::Debug for JavaParse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "syntax:")?;
        if let Some(syntax) = self.syntax() {
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
    let (source, mut diagnostics) = normalize_unicode_escapes(source);
    let parse = Parser::new(&source).parse_compilation_unit();
    finish_parse(source, parse, &mut diagnostics)
}

fn finish_parse(
    source: String,
    parse: source::ParseEvents,
    diagnostics: &mut Vec<Diagnostic>,
) -> JavaParse {
    diagnostics.extend(parse.diagnostics);
    let token_source = JavaGreenTokenSource::new(&source, &parse.tokens);
    let tree = match build_green_tree(&parse.events, &token_source) {
        Ok(tree) => tree,
        Err(error) => {
            diagnostics.push(invalid_event_stream_diagnostic(&error));
            return JavaParse {
                source,
                root: None,
                diagnostics: std::mem::take(diagnostics),
                outcome: SyntaxOutcome::Aborted,
            };
        }
    };
    let (root, parser_diagnostics) = tree.into_parts();
    diagnostics.extend(parser_diagnostics);
    let outcome = if diagnostics.iter().any(diagnostic_affects_syntax_tree) {
        SyntaxOutcome::Recovered
    } else {
        SyntaxOutcome::Clean
    };

    JavaParse {
        source,
        root: Some(root),
        diagnostics: std::mem::take(diagnostics),
        outcome,
    }
}

const fn diagnostic_affects_syntax_tree(diagnostic: &Diagnostic) -> bool {
    matches!(
        diagnostic.stage,
        DiagnosticStage::Lexer | DiagnosticStage::Parser
    ) && matches!(
        diagnostic.severity,
        Severity::Error | Severity::InternalError
    )
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

struct JavaGreenTokenSource<'source> {
    source: &'source str,
    tokens: &'source [ParserToken],
}

impl<'source> JavaGreenTokenSource<'source> {
    fn new(source: &'source str, tokens: &'source [ParserToken]) -> Self {
        Self { source, tokens }
    }

    fn token(&self, index: usize) -> &ParserToken {
        &self.tokens[index]
    }

    fn trivia_text_len(trivia: &Trivia) -> TextSize {
        trivia.range.len()
    }
}

impl GreenTokenSource for JavaGreenTokenSource<'_> {
    fn token_count(&self) -> usize {
        self.tokens.len()
    }

    fn token_kind(&self, index: usize) -> jolt_syntax::RawSyntaxKind {
        self.token(index).kind.to_raw()
    }

    fn token_text_len(&self, index: usize) -> TextSize {
        let range = self.token(index).range;
        TextSize::new(self.source[range.start().get()..range.end().get()].len())
    }

    fn leading_trivia(&self, index: usize) -> impl Iterator<Item = GreenTriviaPiece> {
        self.token(index).leading.iter().map(|trivia| {
            GreenTriviaPiece::new(
                to_green_trivia_kind(trivia.kind),
                Self::trivia_text_len(trivia),
            )
        })
    }

    fn trailing_trivia(&self, index: usize) -> impl Iterator<Item = GreenTriviaPiece> {
        self.token(index).trailing.iter().map(|trivia| {
            GreenTriviaPiece::new(
                to_green_trivia_kind(trivia.kind),
                Self::trivia_text_len(trivia),
            )
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
