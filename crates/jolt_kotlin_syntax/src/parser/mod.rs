mod grammar;
mod source;

use std::fmt;

use jolt_diagnostics::{Diagnostic, DiagnosticCodeId, DiagnosticStage, Severity};
use jolt_syntax::{SyntaxDiagnosticOwner, SyntaxTree, build_parser_syntax_tree};

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
    #[inline]
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
#[inline]
pub fn parse_kotlin_file(source: &str) -> KotlinParse<'_> {
    let parse = Parser::new(source).parse_kotlin_file();
    finish_parse(source, parse)
}

fn finish_parse(source: &str, parse: source::ParseEvents) -> KotlinParse<'_> {
    let mut diagnostics = parse.diagnostics;
    let (tree, diagnostic_owners) = match build_parser_syntax_tree(
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

fn invalid_event_stream_diagnostic(error: &jolt_syntax::BuildSyntaxTreeError) -> Diagnostic {
    Diagnostic {
        code: KotlinParseDiagnosticCode::InvalidEventStream.id(),
        severity: Severity::InternalError,
        stage: DiagnosticStage::Parser,
        message: format!("Jolt parser produced an invalid event stream: {error:?}"),
        range: None,
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

    #[test]
    #[rustfmt::skip]
    fn phase_eighteen_diagnostics_own_the_declared_node_or_slot() {
        check("fun () {}\n", "expected function name", KotlinSyntaxKind::BogusCallableDeclarationName, None);
        check("fun named {}\n", "expected value parameter list", KotlinSyntaxKind::ValueParameterList, None);
        check("val = 1\n", "expected property binding", KotlinSyntaxKind::BogusPropertyBinding, None);
        check("class C { constructor() this() }\n", "expected ':' before constructor delegation", KotlinSyntaxKind::ConstructorDelegation, Some(crate::shape::constructor_delegation::Slot::colon as u16));
        check("class C Base()\n", "expected ':' before delegation specifiers", KotlinSyntaxKind::DelegationClause, Some(crate::shape::delegation_clause::Slot::colon as u16));
        check("typealias Alias String\n", "expected '=' in typealias", KotlinSyntaxKind::TypeAliasDeclaration, Some(crate::shape::type_alias_declaration::Slot::assign as u16));
        check("val x: Int get() value\n", "expected '=' before property accessor expression", KotlinSyntaxKind::ExpressionBody, Some(crate::shape::expression_body::Slot::assign as u16));
        check("fun f() =\n", "expected declaration body expression", KotlinSyntaxKind::ExpressionBody, Some(crate::shape::expression_body::Slot::expression as u16));
        check("val x: Int get() =\n", "expected declaration body expression", KotlinSyntaxKind::ExpressionBody, Some(crate::shape::expression_body::Slot::expression as u16));
        check_code("val x: Int get() {} = value\n", "property accessor has both block and expression bodies", KotlinParseDiagnosticCode::UnexpectedSyntax, KotlinSyntaxKind::BogusDeclarationBody, None);
        check("val (x) value\n", "expected property initializer operator", KotlinSyntaxKind::PropertyInitializer, Some(crate::shape::property_initializer::Slot::operator as u16));
        check("val x by\n", "expected property initializer expression", KotlinSyntaxKind::PropertyInitializer, Some(crate::shape::property_initializer::Slot::expression as u16));
        check("val answer =\nval next = 1\n", "expected property initializer expression", KotlinSyntaxKind::PropertyInitializer, Some(crate::shape::property_initializer::Slot::expression as u16));
        check("val delegated by\nval next = 1\n", "expected property initializer expression", KotlinSyntaxKind::PropertyInitializer, Some(crate::shape::property_initializer::Slot::expression as u16));
        check("val x: Int field\nget() = x\n", "expected '=' after backing field", KotlinSyntaxKind::ExplicitBackingField, Some(crate::shape::explicit_backing_field::Slot::assign as u16));
        check("val x: Int field =\nget() = x\n", "expected backing field expression", KotlinSyntaxKind::ExplicitBackingField, Some(crate::shape::explicit_backing_field::Slot::expression as u16));
        check("class C { constructor(): }\n", "expected constructor delegation call", KotlinSyntaxKind::ConstructorDelegationCall, Some(crate::shape::constructor_delegation_call::Slot::expression as u16));
        check("class C : Base by {}\n", "expected delegation expression after 'by'", KotlinSyntaxKind::DelegationByClause, Some(crate::shape::delegation_by_clause::Slot::delegate as u16));
        check("class C : {}\n", "expected delegation specifier", KotlinSyntaxKind::BogusDelegationSpecifier, None);
        check_code("class C : Base, , Other {}\n", "expected delegation specifier between commas", KotlinParseDiagnosticCode::UnexpectedSyntax, KotlinSyntaxKind::BogusDelegationSpecifier, None);
        check("fun Receiver.() {}\n", "expected name", KotlinSyntaxKind::Name, Some(crate::shape::name::Slot::identifier as u16));
        check("fun Receiver member() {}\n", "expected receiver separator", KotlinSyntaxKind::CallableName, Some(crate::shape::callable_name::Slot::dot as u16));
        check("enum class E { ), }\n", "expected enum entry name", KotlinSyntaxKind::EnumEntry, None);
        check("class C { + }\n", "unexpected orphan class member", KotlinSyntaxKind::BogusClassMember, None);
        check("class C { , }\n", "unexpected orphan class member comma", KotlinSyntaxKind::BogusClassMember, None);
        check("enum class E { A,,B }\n", "unexpected orphan class member comma", KotlinSyntaxKind::BogusClassMember, None);
        check("class C {\n", "expected '}' after class body", KotlinSyntaxKind::ClassBody, Some(crate::shape::class_body::Slot::close_brace as u16));
    }

    #[test]
    #[rustfmt::skip]
    fn phase_nineteen_diagnostics_own_the_declared_node_or_slot() {
        check("fun f() = 1 +\n", "expected expression after operator", KotlinSyntaxKind::BogusExpression, None);
        check("fun f() = 1 +\nval next = 2\n", "expected expression after operator", KotlinSyntaxKind::BogusExpression, None);
        check("fun f() = !\n", "expected expression", KotlinSyntaxKind::BogusExpression, None);
        check_code("fun f() = 1 = 2\n", "invalid assignment target", KotlinParseDiagnosticCode::InvalidAssignmentTarget, KotlinSyntaxKind::BogusExpression, None);
        check("fun f() = target.\n", "expected member name", KotlinSyntaxKind::BogusNavigationSelector, None);
        check("fun f() = target::\n", "expected callable reference name", KotlinSyntaxKind::CallableReferenceTarget, Some(crate::shape::callable_reference_target::Slot::target as u16));
        check("fun f() = call(, value)\n", "expected list item", KotlinSyntaxKind::BogusValueArgument, None);
        check("fun f() = call(,)\n", "expected list item", KotlinSyntaxKind::BogusValueArgument, None);
        check("fun f() = [,]\n", "expected list item", KotlinSyntaxKind::BogusValueArgument, None);
        check("fun f() = value[, index]\n", "expected list item", KotlinSyntaxKind::BogusValueArgument, None);
        check("fun f() = value[index\n", "expected ']'", KotlinSyntaxKind::IndexExpression, Some(crate::shape::index_expression::Slot::close_bracket as u16));
        check("fun f() = call(value\n", "expected ')' after arguments", KotlinSyntaxKind::ValueArgumentList, Some(crate::shape::value_argument_list::Slot::close_paren as u16));
        check_code("fun f() = { , value -> value }\n", "expected lambda parameter between commas", KotlinParseDiagnosticCode::UnexpectedSyntax, KotlinSyntaxKind::BogusLambdaParameter, None);
        check_code("fun f() = { , -> 1 }\n", "expected lambda parameter between commas", KotlinParseDiagnosticCode::UnexpectedSyntax, KotlinSyntaxKind::BogusLambdaParameter, None);
        check("fun f() = ()\n", "expected parenthesized expression", KotlinSyntaxKind::BogusExpression, None);
        check("fun f() = [value\n", "expected ']' after collection literal", KotlinSyntaxKind::CollectionLiteralExpression, Some(crate::shape::collection_literal_expression::Slot::close_bracket as u16));
        check("val f = fun()\n", "expected anonymous function body", KotlinSyntaxKind::BogusDeclarationBody, None);
        check("val f = fun()\nval next = 1\n", "expected anonymous function body", KotlinSyntaxKind::BogusDeclarationBody, None);
        check("val f = fun {}\n", "expected value parameter list", KotlinSyntaxKind::ValueParameterList, None);
        check("val x = object\n", "expected object body", KotlinSyntaxKind::ClassBody, None);
        check("val x = object : A\n", "expected object body", KotlinSyntaxKind::ClassBody, None);
        check("val x = object\nval next = 1\n", "expected object body", KotlinSyntaxKind::ClassBody, None);
        check("val x = object : A\nval next = 1\n", "expected object body", KotlinSyntaxKind::ClassBody, None);
        check("val x = this@\n", "expected label name", KotlinSyntaxKind::LabelReference, Some(crate::shape::label_reference::Slot::label as u16));
        check("val x = super@\n", "expected label name", KotlinSyntaxKind::LabelReference, Some(crate::shape::label_reference::Slot::label as u16));
    }

    #[test]
    fn phase_nineteen_valid_multiline_rhs_stays_expression_owned() {
        for source in [
            "fun f() = 1 +\n !value\n",
            "val f =\n fun(value: Int) = value\n",
            "fun f() = predicate &&\n fun(value: Int) = value\n",
            "val f = fun() = 1\n",
            "val f = fun() {}\n",
            "val x = object {}\n",
            "val x = object : A {}\n",
            "val x = this@owner\n",
            "val x = super<Base>@owner\n",
        ] {
            let parse = parse_kotlin_file(source);
            assert!(
                parse.diagnostics().is_empty(),
                "valid multiline RHS produced diagnostics for {source:?}: {:?}",
                parse.diagnostics(),
            );
        }
    }

    #[test]
    #[rustfmt::skip]
    fn phase_twenty_diagnostics_own_the_declared_node_or_slot() {
        check("fun f() { if value }\n", "expected condition after 'if'", KotlinSyntaxKind::BogusExpression, None);
        check("fun f() { if (value) }\n", "expected branch after 'if' condition", KotlinSyntaxKind::BogusExpression, None);
        check("fun f() { if (value) else }\n", "expected branch after 'if' condition", KotlinSyntaxKind::BogusExpression, None);
        check("fun f() { if (value) else }\n", "expected branch after 'else'", KotlinSyntaxKind::BogusExpression, None);
        check("fun f() { when () {} }\n", "expected when subject expression", KotlinSyntaxKind::BogusExpression, None);
        check("fun f() { when (val value) {} }\n", "expected '=' in when subject", KotlinSyntaxKind::WhenSubject, Some(crate::shape::when_subject::Slot::assign as u16));
        check("fun f() { when (value) }\n", "expected '{' after when subject", KotlinSyntaxKind::WhenExpression, Some(crate::shape::when_expression::Slot::open_brace as u16));
        check("fun f() { when (value) }\n", "expected '}' after when", KotlinSyntaxKind::WhenExpression, Some(crate::shape::when_expression::Slot::close_brace as u16));
        check_code("fun f() { when (value) { , one -> 1 } }\n", "expected when condition between commas", KotlinParseDiagnosticCode::UnexpectedSyntax, KotlinSyntaxKind::BogusWhenCondition, None);
        check_code("fun f() { when { one if guard -> 1 } }\n", "when guard requires a subject", KotlinParseDiagnosticCode::InvalidWhenGuard, KotlinSyntaxKind::WhenGuard, None);
        check("fun f() { when (value) { one value\n two -> 2 } }\n", "expected '->' in when entry", KotlinSyntaxKind::WhenEntry, Some(crate::shape::when_entry::Slot::arrow as u16));
        check("fun f() { when (value) { one -> } }\n", "expected when entry body", KotlinSyntaxKind::BogusExpression, None);
        check("fun f() { try {} }\n", "expected 'catch' or 'finally' after try block", KotlinSyntaxKind::TryExpression, None);
        check("fun f() { try catch {} }\n", "expected block after 'try'", KotlinSyntaxKind::Block, None);
        check("fun f() { try {} catch {} }\n", "expected catch parameter", KotlinSyntaxKind::CatchParameter, None);
        check("fun f() { try {} catch (cause Throwable) {} }\n", "expected ':' in catch parameter", KotlinSyntaxKind::CatchParameter, Some(crate::shape::catch_parameter::Slot::colon as u16));
        check("fun f() { try {} catch (cause: Throwable) }\n", "expected block after 'catch'", KotlinSyntaxKind::Block, None);
        check_code("fun f() { try {} finally {} catch (late: Throwable) {} }\n", "catch clause must precede finally", KotlinParseDiagnosticCode::UnexpectedSyntax, KotlinSyntaxKind::BogusTryClause, None);
        check("fun f() { for (in items) {} }\n", "expected loop variable", KotlinSyntaxKind::ForVariable, None);
        check("fun f() { for (item items) {} }\n", "expected 'in' after loop variable", KotlinSyntaxKind::ForStatement, Some(crate::shape::for_statement::Slot::in_token as u16));
        check("fun f() { for (item in) {} }\n", "expected loop iterable", KotlinSyntaxKind::BogusExpression, None);
        check("fun f() { while body }\n", "expected condition after 'while'", KotlinSyntaxKind::BogusExpression, None);
        check("fun f() { do {} (ready) }\n", "expected 'while' after do body", KotlinSyntaxKind::DoWhileStatement, Some(crate::shape::do_while_statement::Slot::while_token as u16));
        check_code("fun f() { break value }\n", "break and continue do not accept an expression", KotlinParseDiagnosticCode::UnexpectedSyntax, KotlinSyntaxKind::BogusExpression, None);
        check("fun f() { return@ }\n", "expected label name", KotlinSyntaxKind::LabelReference, Some(crate::shape::label_reference::Slot::label as u16));
        check("fun f() { throw\nval next = 1 }\n", "expected expression after 'throw'", KotlinSyntaxKind::BogusExpression, None);
        check("fun f() { if (true) { value }\n", "expected '}' after block", KotlinSyntaxKind::Block, Some(crate::shape::block::Slot::close_brace as u16));
    }

    #[test]
    fn phase_twenty_valid_control_flow_is_diagnostic_free() {
        for source in [
            "fun f() { ; }\n",
            "fun f() { if (ready); else {} }\n",
            "fun f() { when (value) {\none, -> 1\nelse -> 0\n} }\n",
            "fun f() { when (value) {\nfirst(value)\n  && second(value)\n  && third(value) -> 1\nelse -> 0\n} }\n",
            "fun f() { when (var value: Any = source) {} }\n",
            "fun f() { try {} catch (cause: Throwable) {} finally {} }\n",
            "fun f() {\nfor (item in items) {}\nwhile (ready);\ndo {} while (ready)\n}\n",
            "fun f() { return@owner value }\n",
        ] {
            let parse = parse_kotlin_file(source);
            assert!(
                parse.diagnostics().is_empty(),
                "valid control flow produced diagnostics for {source:?}: {:?}",
                parse.diagnostics(),
            );
        }
    }
}
