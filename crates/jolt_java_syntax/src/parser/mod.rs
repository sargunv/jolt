mod grammar;
mod source;

use std::{borrow::Cow, fmt};

use jolt_diagnostics::{Diagnostic, DiagnosticCodeId, DiagnosticStage, Severity};
use jolt_syntax::{
    SyntaxDiagnosticOwner, SyntaxTree, build_syntax_tree_with_factory_and_diagnostic_owners,
};

use crate::{
    CompilationUnit,
    lexer::normalize_unicode_escapes,
    nodes::{JavaSyntaxNode, cast_compilation_unit},
    shape::JavaSyntaxFactory,
};

use self::source::Parser;

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

impl JavaParseDiagnosticCode {
    pub(crate) const fn id(self) -> DiagnosticCodeId {
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
pub struct JavaParse<'source> {
    source: Cow<'source, str>,
    tree: Option<SyntaxTree>,
    diagnostics: Vec<Diagnostic>,
    diagnostic_owners: Vec<Option<SyntaxDiagnosticOwner>>,
}

impl JavaParse<'_> {
    /// Returns flat arena measurements for the benchmark driver.
    #[cfg(feature = "bench")]
    #[must_use]
    pub fn benchmark_metrics(&self) -> Option<jolt_syntax::SyntaxTreeMetrics> {
        self.tree.as_ref().map(SyntaxTree::benchmark_metrics)
    }

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

    /// Returns structural syntax owners parallel to [`Self::diagnostics`].
    /// Lexer and non-structural diagnostics have no owner.
    #[must_use]
    pub fn structural_diagnostic_owners(&self) -> &[Option<SyntaxDiagnosticOwner>] {
        &self.diagnostic_owners
    }
}

impl fmt::Debug for JavaParse<'_> {
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

/// Parses a Java compilation unit.
#[must_use]
pub fn parse_compilation_unit(source: &str) -> JavaParse<'_> {
    let (source, mut diagnostics) = normalize_unicode_escapes(source);
    let parse = Parser::new(&source).parse_compilation_unit();
    finish_parse(source, parse, &mut diagnostics)
}

fn finish_parse<'source>(
    source: Cow<'source, str>,
    parse: source::ParseEvents,
    diagnostics: &mut Vec<Diagnostic>,
) -> JavaParse<'source> {
    let unicode_diagnostic_count = diagnostics.len();
    diagnostics.extend(parse.diagnostics);
    let (tree, parse_diagnostic_owners) = match build_syntax_tree_with_factory_and_diagnostic_owners(
        &source,
        parse.events,
        parse.tokens,
        parse.trivia,
        &parse.diagnostic_owners,
        &JavaSyntaxFactory,
    ) {
        Ok(tree) => tree,
        Err(error) => {
            diagnostics.push(invalid_event_stream_diagnostic(&error));
            let diagnostic_owners = vec![None; diagnostics.len()];
            return JavaParse {
                source,
                tree: None,
                diagnostics: std::mem::take(diagnostics),
                diagnostic_owners,
            };
        }
    };
    let mut diagnostic_owners = vec![None; unicode_diagnostic_count];
    diagnostic_owners.extend(parse_diagnostic_owners);
    JavaParse {
        source,
        tree: Some(tree),
        diagnostics: std::mem::take(diagnostics),
        diagnostic_owners,
    }
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

#[cfg(test)]
mod tests {
    use jolt_diagnostics::DiagnosticStage;
    use jolt_syntax::SyntaxSlot;

    use crate::{JavaSyntaxKind, parse_compilation_unit};

    #[test]
    fn phase_eleven_structural_diagnostics_have_exact_syntax_owners() {
        for source in [
            "package ;\nimport foo + lost;\nmodule m { +; requires ; exports p to ; opens p target; uses ; provides s with ; }\n+;",
            "module missing requires dependency; class After {}",
        ] {
            let parse = parse_compilation_unit(source);
            let root = parse.syntax().expect("represented compilation unit");
            let mut nodes = vec![*root.syntax()];
            let mut cursor = 0;
            while let Some(node) = nodes.get(cursor).copied() {
                nodes.extend(node.children());
                cursor += 1;
            }
            let owners = parse.structural_diagnostic_owners();
            assert_eq!(owners.len(), parse.diagnostics().len());
            let mut owned_nodes = Vec::new();
            for (diagnostic, owner) in parse.diagnostics().iter().zip(owners) {
                if diagnostic.stage != DiagnosticStage::Parser {
                    continue;
                }
                let owner = owner.unwrap_or_else(|| panic!("unowned diagnostic: {diagnostic:?}"));
                let node = nodes
                    .iter()
                    .copied()
                    .find(|node| node.id() == owner.node())
                    .unwrap_or_else(|| panic!("owner node is not reachable: {diagnostic:?}"));
                if let Some(slot) = owner.slot() {
                    assert!(
                        matches!(node.slot_at(slot as usize), Some(SyntaxSlot::Empty)),
                        "diagnostic does not own an empty slot: {diagnostic:?}"
                    );
                }
                owned_nodes.push(owner.node());
            }
            for node in nodes {
                if matches!(
                    node.kind(),
                    JavaSyntaxKind::BogusCompilationUnitItem
                        | JavaSyntaxKind::BogusImportSuffix
                        | JavaSyntaxKind::BogusModuleDirective
                ) {
                    assert!(node.is_directly_malformed());
                    assert!(
                        owned_nodes.contains(&node.id()),
                        "direct malformed Phase 11 owner has no diagnostic: {:?}",
                        node.kind()
                    );
                }
            }
        }
    }
}
