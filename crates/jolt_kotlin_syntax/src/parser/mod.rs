mod grammar;
mod source;

use std::fmt;

use jolt_diagnostics::{Diagnostic, DiagnosticCodeId, DiagnosticStage, Severity};
use jolt_syntax::{SyntaxTree, build_syntax_tree};

use crate::{
    KotlinFile,
    nodes::{KotlinSyntaxNode, cast_kotlin_file},
};

use self::source::Parser;

/// Stable Kotlin parser diagnostic codes.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum KotlinParseDiagnosticCode {
    ExpectedSyntax,
    UnexpectedSyntax,
    InvalidAssignmentTarget,
    MalformedTypeArgumentList,
    InvalidWhenGuard,
    ReservedCallableReferenceCall,
    InvalidEventStream,
}

impl KotlinParseDiagnosticCode {
    pub(crate) const fn id(self) -> DiagnosticCodeId {
        match self {
            Self::ExpectedSyntax => DiagnosticCodeId::new("kotlin.parse.expected_syntax"),
            Self::UnexpectedSyntax => DiagnosticCodeId::new("kotlin.parse.unexpected_syntax"),
            Self::InvalidAssignmentTarget => {
                DiagnosticCodeId::new("kotlin.parse.invalid_assignment_target")
            }
            Self::MalformedTypeArgumentList => {
                DiagnosticCodeId::new("kotlin.parse.malformed_type_argument_list")
            }
            Self::InvalidWhenGuard => DiagnosticCodeId::new("kotlin.parse.invalid_when_guard"),
            Self::ReservedCallableReferenceCall => {
                DiagnosticCodeId::new("kotlin.parse.reserved_callable_reference_call")
            }
            Self::InvalidEventStream => {
                DiagnosticCodeId::new("internal.syntax.invalid_event_stream")
            }
        }
    }
}

/// The result of parsing Kotlin source text.
pub struct KotlinParse<'source> {
    source: &'source str,
    tree: Option<SyntaxTree>,
    diagnostics: Vec<Diagnostic>,
}

impl KotlinParse<'_> {
    /// Returns flat arena measurements for the benchmark driver.
    #[cfg(feature = "bench")]
    #[must_use]
    pub fn benchmark_metrics(&self) -> Option<jolt_syntax::SyntaxTreeMetrics> {
        self.tree.as_ref().map(SyntaxTree::benchmark_metrics)
    }

    /// Returns the parsed syntax tree root.
    #[must_use]
    pub fn syntax(&self) -> Option<KotlinFile<'_>> {
        self.tree
            .as_ref()
            .and_then(|tree| cast_kotlin_file(KotlinSyntaxNode::new_root(self.source, tree)))
    }

    /// Returns lexer and parser diagnostics.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
}

impl fmt::Debug for KotlinParse<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "syntax:")?;
        if let Some(syntax) = self.syntax() {
            writeln!(f, "{syntax:?}")?;
        } else {
            writeln!(f, "  (none)")?;
        }

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

/// Parses a Kotlin file.
#[must_use]
pub fn parse_kotlin_file(source: &str) -> KotlinParse<'_> {
    let parse = Parser::new(source).parse_kotlin_file();
    finish_parse(source, parse)
}

fn finish_parse(source: &str, parse: source::ParseEvents) -> KotlinParse<'_> {
    let mut diagnostics = parse.diagnostics;
    let tree = match build_syntax_tree(parse.events, parse.tokens, parse.trivia) {
        Ok(tree) => tree,
        Err(error) => {
            diagnostics.push(invalid_event_stream_diagnostic(&error));
            return KotlinParse {
                source,
                tree: None,
                diagnostics,
            };
        }
    };
    let (tree, parser_diagnostics) = tree;
    diagnostics.extend(parser_diagnostics);

    KotlinParse {
        source,
        tree: Some(tree),
        diagnostics,
    }
}

fn invalid_event_stream_diagnostic(error: &jolt_syntax::BuildSyntaxTreeError) -> Diagnostic {
    Diagnostic {
        code: KotlinParseDiagnosticCode::InvalidEventStream.id(),
        severity: Severity::InternalError,
        stage: DiagnosticStage::Parser,
        message: format!("Jolt parser produced an invalid event stream: {error:?}"),
        range: None,
    }
}
