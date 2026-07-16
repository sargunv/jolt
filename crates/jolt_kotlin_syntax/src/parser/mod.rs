mod grammar;
mod source;

use std::fmt;

use jolt_diagnostics::{Diagnostic, DiagnosticCodeId, DiagnosticStage, Severity};
use jolt_syntax::{
    SyntaxDiagnosticOwner, SyntaxTree, build_syntax_tree_with_factory_and_diagnostic_owners,
};

use crate::{
    KotlinFile,
    nodes::{KotlinSyntaxNode, cast_kotlin_file},
    shape::KotlinSyntaxFactory,
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
    diagnostic_owners: Vec<Option<SyntaxDiagnosticOwner>>,
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

    /// Returns structural syntax owners parallel to [`Self::diagnostics`].
    /// Lexer and non-structural diagnostics have no owner.
    #[must_use]
    pub fn structural_diagnostic_owners(&self) -> &[Option<SyntaxDiagnosticOwner>] {
        &self.diagnostic_owners
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
    let (tree, diagnostic_owners) = match build_syntax_tree_with_factory_and_diagnostic_owners(
        source,
        parse.events,
        parse.tokens,
        parse.trivia,
        &parse.diagnostic_owners,
        &KotlinSyntaxFactory,
    ) {
        Ok(tree) => tree,
        Err(error) => {
            diagnostics.push(invalid_event_stream_diagnostic(&error));
            let diagnostic_owners = vec![None; diagnostics.len()];
            return KotlinParse {
                source,
                tree: None,
                diagnostics,
                diagnostic_owners,
            };
        }
    };
    KotlinParse {
        source,
        tree: Some(tree),
        diagnostics,
        diagnostic_owners,
    }
}

#[cfg(test)]
mod tests {
    use jolt_test_support::assert_exact_diagnostic_owner;

    use crate::{KotlinSyntaxKind, KotlinSyntaxView, parse_kotlin_file};

    use super::KotlinParseDiagnosticCode;

    #[rustfmt::skip]
    fn check(source: &str, message: &str, kind: KotlinSyntaxKind, slot: Option<u16>) {
        check_code(
            source,
            message,
            if message.starts_with("unexpected") {
                KotlinParseDiagnosticCode::UnexpectedSyntax
            } else {
                KotlinParseDiagnosticCode::ExpectedSyntax
            },
            kind,
            slot,
        );
    }

    #[rustfmt::skip]
    fn check_code(source: &str, message: &str, code: KotlinParseDiagnosticCode, kind: KotlinSyntaxKind, slot: Option<u16>) {
        let parse = parse_kotlin_file(source);
        let root = parse.syntax().expect("represented Kotlin file");
        assert_exact_diagnostic_owner(
            root.syntax_node().expect("physical Kotlin root"),
            parse.diagnostics(),
            parse.structural_diagnostic_owners(),
            code.id(),
            message,
            kind,
            slot,
        );
    }

    #[test]
    #[rustfmt::skip]
    fn phase_sixteen_diagnostics_own_the_declared_node_or_slot() {
        check("package\n", "expected name", KotlinSyntaxKind::Name, Some(crate::shape::name::Slot::identifier as u16));
        check("import sample*\n", "expected `.` before import star", KotlinSyntaxKind::ImportOnDemandSuffix, Some(crate::shape::import_on_demand_suffix::Slot::dot as u16));
        check("import sample as\n", "expected name", KotlinSyntaxKind::Name, Some(crate::shape::name::Slot::identifier as u16));
        check("package sample unexpected\n", "unexpected token in package header", KotlinSyntaxKind::BogusPackageSuffix, None);
        check("import sample unexpected\n", "unexpected token in import directive", KotlinSyntaxKind::BogusImportSuffix, None);
        check("package first\npackage second\n", "unexpected package header after file header", KotlinSyntaxKind::PackageHeader, None);
        check("class C\nimport sample.Name\n", "unexpected import after file item", KotlinSyntaxKind::ImportDirectiveList, None);
        check("}\n", "unexpected closing brace at top level", KotlinSyntaxKind::BogusKotlinFileItem, None);
    }

    #[test]
    #[rustfmt::skip]
    fn phase_seventeen_diagnostics_own_the_declared_node_or_slot() {
        check("typealias T =\n", "expected type", KotlinSyntaxKind::BogusType, None);
        check("typealias T = A.\n", "expected type segment", KotlinSyntaxKind::BogusUserTypeSegment, None);
        check("typealias T = A..B\n", "expected one '.' between type segments", KotlinSyntaxKind::BogusUserTypeSegment, None);
        check_code("typealias T = Box<, A>\n", "malformed type argument list", KotlinParseDiagnosticCode::MalformedTypeArgumentList, KotlinSyntaxKind::BogusTypeArgument, None);
        check_code("typealias T = Box<*A>\n", "star projection cannot include a simultaneous type", KotlinParseDiagnosticCode::MalformedTypeArgumentList, KotlinSyntaxKind::BogusTypeArgument, None);
        check_code("fun <, T> f() {}\n", "expected type parameter between commas", KotlinParseDiagnosticCode::UnexpectedSyntax, KotlinSyntaxKind::BogusTypeParameter, None);
        check("fun <T Any> f() {}\n", "expected ':' before type parameter bound", KotlinSyntaxKind::TypeParameter, Some(crate::shape::type_parameter::Slot::colon as u16));
        check("fun <T> f() T: Any {}\n", "expected 'where' before type constraints", KotlinSyntaxKind::TypeConstraintList, Some(crate::shape::type_constraint_list::Slot::where_token as u16));
        check("fun <T> f() where T Any {}\n", "expected ':' before type constraint bound", KotlinSyntaxKind::TypeConstraint, Some(crate::shape::type_constraint::Slot::colon as u16));
        check_code("fun <T> f() where T : Any, , T : Closeable {}\n", "expected type constraint between commas", KotlinParseDiagnosticCode::UnexpectedSyntax, KotlinSyntaxKind::BogusTypeConstraint, None);
        check_code("typealias T = (, A) -> Unit\n", "expected function type parameter between commas", KotlinParseDiagnosticCode::UnexpectedSyntax, KotlinSyntaxKind::BogusFunctionTypeParameter, None);
        check_code("context(, String) fun f() {}\n", "expected context parameter", KotlinParseDiagnosticCode::UnexpectedSyntax, KotlinSyntaxKind::BogusContextParameter, None);
        check_code("fun f(, x: Int) {}\n", "expected value parameter between commas", KotlinParseDiagnosticCode::UnexpectedSyntax, KotlinSyntaxKind::BogusValueParameter, None);
        check("fun f(x: Int 1) {}\n", "expected '=' before parameter default", KotlinSyntaxKind::ValueParameter, Some(crate::shape::value_parameter::Slot::assign as u16));
        check("context(named: Int 1) fun f() {}\n", "expected '=' before context parameter default", KotlinSyntaxKind::ContextParameter, Some(crate::shape::context_parameter::Slot::assign as u16));
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
