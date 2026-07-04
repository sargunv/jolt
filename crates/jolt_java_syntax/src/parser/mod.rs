#[cfg(test)]
mod tests;

mod grammar;
mod source;

use std::fmt;

use jolt_diagnostics::{
    Diagnostic, DiagnosticCode, DiagnosticCodeId, DiagnosticStage, Severity, SyntaxOutcome,
};
use jolt_syntax::{
    SyntaxTokenData, SyntaxTree, SyntaxTrivia, TriviaKind as SyntaxTriviaKind, build_syntax_tree,
};

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
    tree: Option<SyntaxTree>,
    diagnostics: Vec<Diagnostic>,
    outcome: SyntaxOutcome,
}

impl JavaParse {
    /// Returns the parsed syntax tree root.
    #[must_use]
    pub fn syntax(&self) -> Option<CompilationUnit<'_>> {
        self.tree
            .as_ref()
            .and_then(|tree| cast_compilation_unit(JavaSyntaxNode::new_root(&self.source, tree)))
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
    let (tokens, trivia) = into_syntax_tokens(parse.tokens, parse.trivia);
    let tree = match build_syntax_tree(&parse.events, tokens, trivia) {
        Ok(tree) => tree,
        Err(error) => {
            diagnostics.push(invalid_event_stream_diagnostic(&error));
            return JavaParse {
                source,
                tree: None,
                diagnostics: std::mem::take(diagnostics),
                outcome: SyntaxOutcome::Aborted,
            };
        }
    };
    let (tree, parser_diagnostics) = tree.into_parts();
    diagnostics.extend(parser_diagnostics);
    let outcome = if diagnostics.iter().any(diagnostic_affects_syntax_tree) {
        SyntaxOutcome::Recovered
    } else {
        SyntaxOutcome::Clean
    };

    JavaParse {
        source,
        tree: Some(tree),
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

fn invalid_event_stream_diagnostic(error: &jolt_syntax::BuildSyntaxTreeError) -> Diagnostic {
    Diagnostic {
        code: JavaParseDiagnosticCode::InvalidEventStream.id(),
        severity: Severity::InternalError,
        stage: DiagnosticStage::Parser,
        message: format!("Jolt parser produced an invalid event stream: {error:?}"),
        range: None,
    }
}

fn into_syntax_tokens(
    tokens: Vec<ParserToken>,
    trivia: Vec<Trivia>,
) -> (Vec<SyntaxTokenData>, Vec<SyntaxTrivia>) {
    let syntax_trivia = trivia
        .into_iter()
        .map(|trivia| SyntaxTrivia::new(to_syntax_trivia_kind(trivia.kind), trivia.range.len()))
        .collect::<Vec<_>>();
    let syntax_tokens = tokens
        .into_iter()
        .map(|token| {
            SyntaxTokenData::new(
                token.kind.to_raw(),
                token.range,
                token.leading,
                token.trailing,
                &syntax_trivia,
            )
        })
        .collect();

    (syntax_tokens, syntax_trivia)
}

fn to_syntax_trivia_kind(kind: crate::TriviaKind) -> SyntaxTriviaKind {
    match kind {
        crate::TriviaKind::Whitespace => SyntaxTriviaKind::Whitespace,
        crate::TriviaKind::Newline => SyntaxTriviaKind::Newline,
        crate::TriviaKind::LineComment => SyntaxTriviaKind::LineComment,
        crate::TriviaKind::BlockComment => SyntaxTriviaKind::BlockComment,
        crate::TriviaKind::JavadocComment => SyntaxTriviaKind::DocComment,
        crate::TriviaKind::Ignored => SyntaxTriviaKind::Ignored,
    }
}
