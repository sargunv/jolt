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
    ExcessiveSyntaxNesting,
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
            Self::ExcessiveSyntaxNesting => {
                DiagnosticCodeId::new("kotlin.parse.excessive_syntax_nesting")
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
        let syntax = self.syntax();
        jolt_syntax::fmt_parse_debug(
            f,
            syntax.as_ref().map(|syntax| syntax as &dyn fmt::Debug),
            &self.diagnostics,
        )
    }
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

    fn check_unowned(source: &str, code: KotlinParseDiagnosticCode) {
        let parse = parse_kotlin_file(source);
        let index = parse
            .diagnostics()
            .iter()
            .position(|diagnostic| diagnostic.code == code.id())
            .expect("expected diagnostic");
        assert_eq!(parse.structural_diagnostic_owners()[index], None);
    }

    #[test]
    #[rustfmt::skip]
    fn file_item_diagnostics_own_the_declared_node_or_slot() {
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
    fn type_and_parameter_diagnostics_own_the_declared_node_or_slot() {
        check("typealias T =\n", "expected type", KotlinSyntaxKind::BogusType, None);
        check("typealias T = A.\n", "expected type segment", KotlinSyntaxKind::BogusUserTypeSegment, None);
        check("typealias T = A..B\n", "expected one '.' between type segments", KotlinSyntaxKind::UserTypeSegmentList, None);
        check_code("typealias T = Box<, A>\n", "malformed type argument list", KotlinParseDiagnosticCode::MalformedTypeArgumentList, KotlinSyntaxKind::BogusTypeArgument, None);
        check_code("typealias T = Box<*A>\n", "star projection cannot include a simultaneous type", KotlinParseDiagnosticCode::MalformedTypeArgumentList, KotlinSyntaxKind::BogusTypeArgument, None);
        check_code("fun <, T> f() {}\n", "expected type parameter between commas", KotlinParseDiagnosticCode::UnexpectedSyntax, KotlinSyntaxKind::BogusTypeParameter, None);
        check("fun <T Any> f() {}\n", "expected ':' before type parameter bound", KotlinSyntaxKind::TypeParameter, None);
        check("fun <T> f() T: Any {}\n", "expected 'where' before type constraints", KotlinSyntaxKind::TypeConstraintList, Some(crate::shape::type_constraint_list::Slot::where_token as u16));
        check("fun <T> f() where T Any {}\n", "expected ':' before type constraint bound", KotlinSyntaxKind::TypeConstraint, Some(crate::shape::type_constraint::Slot::colon as u16));
        check_code("fun <T> f() where T : Any, , T : Closeable {}\n", "expected type constraint between commas", KotlinParseDiagnosticCode::UnexpectedSyntax, KotlinSyntaxKind::BogusTypeConstraint, None);
        check_code("typealias T = (, A) -> Unit\n", "expected function type parameter between commas", KotlinParseDiagnosticCode::UnexpectedSyntax, KotlinSyntaxKind::BogusFunctionTypeParameter, None);
        check_code("context(, String) fun f() {}\n", "expected context parameter", KotlinParseDiagnosticCode::UnexpectedSyntax, KotlinSyntaxKind::BogusContextParameter, None);
        check_code("fun f(, x: Int) {}\n", "expected value parameter between commas", KotlinParseDiagnosticCode::UnexpectedSyntax, KotlinSyntaxKind::BogusValueParameter, None);
        check("fun f(x: Int 1) {}\n", "expected '=' before parameter default", KotlinSyntaxKind::ValueParameter, None);
        check("context(named: Int 1) fun f() {}\n", "expected '=' before context parameter default", KotlinSyntaxKind::ContextParameter, None);
    }

    #[test]
    #[rustfmt::skip]
    fn declaration_diagnostics_own_the_declared_node_or_slot() {
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
        check("enum class E { ), }\n", "expected enum entry name", KotlinSyntaxKind::Name, None);
        check("class C { + }\n", "unexpected orphan class member", KotlinSyntaxKind::BogusClassMember, None);
        check("class C { , }\n", "unexpected orphan class member comma", KotlinSyntaxKind::BogusClassMember, None);
        check("enum class E { A,,B }\n", "unexpected orphan class member comma", KotlinSyntaxKind::BogusClassMember, None);
        check("class C {\n", "expected '}' after class body", KotlinSyntaxKind::ClassBody, Some(crate::shape::class_body::Slot::close_brace as u16));
    }

    #[test]
    #[rustfmt::skip]
    fn expression_diagnostics_own_the_declared_node_or_slot() {
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
    fn valid_multiline_rhs_stays_expression_owned() {
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
    fn control_flow_diagnostics_own_the_declared_node_or_slot() {
        check("fun f() { if value }\n", "expected condition after 'if'", KotlinSyntaxKind::ParenthesizedExpression, Some(crate::shape::parenthesized_expression::Slot::expression as u16));
        check("fun f() { if (value) }\n", "expected branch after 'if' condition", KotlinSyntaxKind::BogusExpression, None);
        check("fun f() { if (value) else }\n", "expected branch after 'if' condition", KotlinSyntaxKind::BogusExpression, None);
        check("fun f() { if (value) else }\n", "expected branch after 'else'", KotlinSyntaxKind::BogusExpression, None);
        check("fun f() { when () {} }\n", "expected when subject expression", KotlinSyntaxKind::BogusExpression, None);
        check("fun f() { when (val value) {} }\n", "expected '=' in when subject", KotlinSyntaxKind::WhenSubject, Some(crate::shape::when_subject::Slot::assign as u16));
        check("fun f() { when (value) }\n", "expected '{' after when subject", KotlinSyntaxKind::WhenExpression, Some(crate::shape::when_expression::Slot::open_brace as u16));
        check("fun f() { when (value) }\n", "expected '}' after when", KotlinSyntaxKind::WhenExpression, Some(crate::shape::when_expression::Slot::close_brace as u16));
        check_code("fun f() { when (value) { , one -> 1 } }\n", "expected when condition between commas", KotlinParseDiagnosticCode::UnexpectedSyntax, KotlinSyntaxKind::BogusWhenCondition, None);
        check_unowned("fun f() { when { one if guard -> 1 } }\n", KotlinParseDiagnosticCode::InvalidWhenGuard);
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
        check("fun f() { while body }\n", "expected condition after 'while'", KotlinSyntaxKind::ParenthesizedExpression, Some(crate::shape::parenthesized_expression::Slot::expression as u16));
        check("fun f() { do {} (ready) }\n", "expected 'while' after do body", KotlinSyntaxKind::DoWhileStatement, Some(crate::shape::do_while_statement::Slot::while_token as u16));
        check_code("fun f() { break value }\n", "break and continue do not accept an expression", KotlinParseDiagnosticCode::UnexpectedSyntax, KotlinSyntaxKind::BogusExpression, None);
        check("fun f() { return@ }\n", "expected label name", KotlinSyntaxKind::LabelReference, Some(crate::shape::label_reference::Slot::label as u16));
        check("fun f() { throw\nval next = 1 }\n", "expected expression after 'throw'", KotlinSyntaxKind::BogusExpression, None);
        check("fun f() { if (true) { value }\n", "expected '}' after block", KotlinSyntaxKind::Block, Some(crate::shape::block::Slot::close_brace as u16));
    }

    #[test]
    fn valid_control_flow_is_diagnostic_free() {
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

    fn excessive_syntax_diagnostic_count(source: &str) -> usize {
        parse_kotlin_file(source)
            .diagnostics()
            .iter()
            .filter(|diagnostic| {
                diagnostic.code == KotlinParseDiagnosticCode::ExcessiveSyntaxNesting.id()
            })
            .count()
    }

    fn nested_type(depth: usize, leaf: &str) -> String {
        format!("{}{}{}", "Box<".repeat(depth), leaf, ">".repeat(depth))
    }

    fn count_nodes(source: &str, kind: KotlinSyntaxKind) -> usize {
        let parse = parse_kotlin_file(source);
        let root = parse.syntax().expect("represented Kotlin file");
        let mut nodes = vec![root.syntax_node().expect("physical Kotlin root")];
        let mut count = 0;
        while let Some(node) = nodes.pop() {
            nodes.extend(node.children());
            count += usize::from(node.kind() == kind);
        }
        count
    }

    #[test]
    fn type_nesting_has_an_exact_edge_and_required_function_recovery() {
        let clean = format!("typealias Deep = {}\n", nested_type(127, "Leaf"));
        assert_eq!(excessive_syntax_diagnostic_count(&clean), 0);

        let edge = format!(
            "typealias Deep = {}\nclass Following\n",
            nested_type(128, "Leaf")
        );
        check_code(
            &edge,
            "syntax is too deeply nested to parse safely",
            KotlinParseDiagnosticCode::ExcessiveSyntaxNesting,
            KotlinSyntaxKind::BogusType,
            None,
        );
        assert_eq!(excessive_syntax_diagnostic_count(&edge), 1);
        assert_eq!(count_nodes(&edge, KotlinSyntaxKind::ClassDeclaration), 1);

        for leaf in ["suspend () -> Unit", "context() () -> Unit"] {
            let source = format!("typealias Deep = {}\n", nested_type(127, leaf));
            check_code(
                &source,
                "syntax is too deeply nested to parse safely",
                KotlinParseDiagnosticCode::ExcessiveSyntaxNesting,
                KotlinSyntaxKind::FunctionType,
                None,
            );
            assert_eq!(excessive_syntax_diagnostic_count(&source), 1);
        }
    }

    #[test]
    fn expression_nesting_has_an_exact_edge_and_bounded_deep_recovery() {
        let parenthesized = |depth| {
            format!(
                "fun value() = {}input{}\n",
                "(".repeat(depth),
                ")".repeat(depth)
            )
        };
        assert_eq!(excessive_syntax_diagnostic_count(&parenthesized(63)), 0);
        let edge = parenthesized(64);
        check_code(
            &edge,
            "syntax is too deeply nested to parse safely",
            KotlinParseDiagnosticCode::ExcessiveSyntaxNesting,
            KotlinSyntaxKind::BogusExpression,
            None,
        );

        for expression in [
            format!("{}true", "! ".repeat(4096)),
            format!("{}leaf", "value = ".repeat(4096)),
            format!("{}input{}", "(".repeat(4096), ")".repeat(4096)),
        ] {
            let source =
                format!("fun value() = {expression}\nval following = 1\nclass Following\n");
            let parse = parse_kotlin_file(&source);
            assert_eq!(
                parse
                    .syntax()
                    .expect("represented expression")
                    .source_text(),
                source
            );
            assert_eq!(excessive_syntax_diagnostic_count(&source), 1);
            assert_eq!(
                count_nodes(&source, KotlinSyntaxKind::PropertyDeclaration),
                1
            );
            assert_eq!(count_nodes(&source, KotlinSyntaxKind::ClassDeclaration), 1);
        }
    }

    #[test]
    fn unary_and_assignment_nesting_have_an_exact_edge() {
        let unary = |depth| format!("fun value() = {}input\n", "! ".repeat(depth));
        let assignment = |depth| format!("fun value() = {}input\n", "target = ".repeat(depth));

        for clean in [unary(126), assignment(126)] {
            assert_eq!(excessive_syntax_diagnostic_count(&clean), 0, "{clean}");
        }
        for edge in [unary(127), assignment(127)] {
            assert_eq!(excessive_syntax_diagnostic_count(&edge), 1, "{edge}");
            assert_eq!(count_nodes(&edge, KotlinSyntaxKind::BogusExpression), 1);
        }
    }

    #[test]
    fn alternating_type_annotation_expression_recursion_is_bounded_and_lossless() {
        let depth = 4096;
        let source = format!(
            "typealias Deep = {}Leaf{}\nclass Following\n",
            "Box<@A(value = input as ".repeat(depth),
            ") Annotated>".repeat(depth),
        );
        let parse = parse_kotlin_file(&source);
        assert_eq!(
            parse
                .syntax()
                .expect("represented alternating input")
                .source_text(),
            source
        );
        assert_eq!(excessive_syntax_diagnostic_count(&source), 1);
        assert_eq!(count_nodes(&source, KotlinSyntaxKind::ClassDeclaration), 1);
    }

    #[test]
    fn excessive_expression_keeps_nested_lambda_semicolons_inside_recovery() {
        let source = format!(
            "fun value() = {}call {{ a; b }}\nval following = 1\n",
            "! ".repeat(127)
        );
        let parse = parse_kotlin_file(&source);
        assert_eq!(
            parse
                .syntax()
                .expect("represented lambda recovery")
                .source_text(),
            source
        );
        assert_eq!(excessive_syntax_diagnostic_count(&source), 1);
        assert_eq!(count_nodes(&source, KotlinSyntaxKind::BogusExpression), 1);
        assert_eq!(count_nodes(&source, KotlinSyntaxKind::NameExpression), 0);
        assert_eq!(
            count_nodes(&source, KotlinSyntaxKind::PropertyDeclaration),
            1
        );
    }

    #[test]
    fn excessive_expression_preserves_balanced_and_caller_boundaries() {
        let balanced = format!(
            r#"fun value() = {}call({{ item -> "${{if (item) "${{nested}}" else "fallback"}}"; item }}, [first, second])
val following = 1
"#,
            "! ".repeat(127),
        );
        let parse = parse_kotlin_file(&balanced);
        assert_eq!(
            parse
                .syntax()
                .expect("represented balanced recovery")
                .source_text(),
            balanced
        );
        assert_eq!(excessive_syntax_diagnostic_count(&balanced), 1);
        assert_eq!(count_nodes(&balanced, KotlinSyntaxKind::BogusExpression), 1);
        assert_eq!(
            count_nodes(&balanced, KotlinSyntaxKind::PropertyDeclaration),
            1
        );

        let arrow = format!(
            "fun choose(value: Int) = when (value) {{ in {}candidate -> 1; else -> 0 }}\nclass Following\n",
            "! ".repeat(127),
        );
        assert_eq!(excessive_syntax_diagnostic_count(&arrow), 1);
        assert_eq!(count_nodes(&arrow, KotlinSyntaxKind::WhenEntry), 2);
        assert_eq!(count_nodes(&arrow, KotlinSyntaxKind::ClassDeclaration), 1);
    }

    fn nested_blocks(depth: usize) -> String {
        format!("{}{}", "fun nested() {".repeat(depth), "}".repeat(depth))
    }

    fn nested_class_bodies(depth: usize) -> String {
        format!("{}{}", "class Nested {".repeat(depth), "}".repeat(depth))
    }

    #[test]
    fn block_and_class_body_nesting_have_exact_edges_and_structured_recovery() {
        for clean in [nested_blocks(128), nested_class_bodies(128)] {
            assert_eq!(excessive_syntax_diagnostic_count(&clean), 0);
        }

        let block_edge = format!("{}\nclass Following\n", nested_blocks(129));
        check_code(
            &block_edge,
            "syntax nesting exceeds 128 levels",
            KotlinParseDiagnosticCode::ExcessiveSyntaxNesting,
            KotlinSyntaxKind::BogusBlockItem,
            None,
        );
        assert_eq!(count_nodes(&block_edge, KotlinSyntaxKind::Block), 129);
        assert_eq!(
            count_nodes(&block_edge, KotlinSyntaxKind::BlockItemList),
            129
        );
        assert_eq!(
            count_nodes(&block_edge, KotlinSyntaxKind::ClassDeclaration),
            1
        );

        let class_edge = format!("{}\nclass Following\n", nested_class_bodies(129));
        check_code(
            &class_edge,
            "syntax nesting exceeds 128 levels",
            KotlinParseDiagnosticCode::ExcessiveSyntaxNesting,
            KotlinSyntaxKind::BogusClassMember,
            None,
        );
        assert_eq!(count_nodes(&class_edge, KotlinSyntaxKind::ClassBody), 129);
        assert_eq!(
            count_nodes(&class_edge, KotlinSyntaxKind::ClassMemberList),
            129
        );
        assert_eq!(
            count_nodes(&class_edge, KotlinSyntaxKind::ClassDeclaration),
            130
        );

        let nonempty = format!(
            "{}fun denied() {{\n// @formatter:off\nif (ready) {{ first; second }}\n// @formatter:on\n}}\nval sibling = 1\n{}\nclass Following\n",
            "fun outer() {".repeat(128),
            "}".repeat(128),
        );
        let parse = parse_kotlin_file(&nonempty);
        assert_eq!(
            parse
                .syntax()
                .expect("represented nonempty recovery")
                .source_text(),
            nonempty
        );
        assert_eq!(count_nodes(&nonempty, KotlinSyntaxKind::BogusBlockItem), 1);
        assert_eq!(
            count_nodes(&nonempty, KotlinSyntaxKind::PropertyDeclaration),
            1
        );
        assert_eq!(
            count_nodes(&nonempty, KotlinSyntaxKind::ClassDeclaration),
            1
        );

        for (source, list_kind) in [
            (
                "fun nested() {".repeat(129),
                KotlinSyntaxKind::BlockItemList,
            ),
            (
                "class Nested {".repeat(129),
                KotlinSyntaxKind::ClassMemberList,
            ),
        ] {
            let parse = parse_kotlin_file(&source);
            assert_eq!(
                parse
                    .syntax()
                    .expect("represented unclosed structural input")
                    .source_text(),
                source
            );
            assert_eq!(excessive_syntax_diagnostic_count(&source), 1);
            assert_eq!(count_nodes(&source, list_kind), 129);
            assert!(
                parse
                    .diagnostics()
                    .iter()
                    .any(|diagnostic| diagnostic.message.contains("expected '}'"))
            );
        }
    }

    #[test]
    fn deep_and_alternating_structural_recursion_is_bounded_and_lossless() {
        let alternating = |depth| {
            format!(
                "{}{}",
                "class Nested { fun nested() {".repeat(depth),
                "}}".repeat(depth)
            )
        };
        assert_eq!(excessive_syntax_diagnostic_count(&alternating(64)), 0);
        assert_eq!(excessive_syntax_diagnostic_count(&alternating(65)), 1);

        for (source, following_classes) in [
            (format!("{}\nclass Following\n", nested_blocks(4096)), 1),
            (
                format!("{}\nclass Following\n", nested_class_bodies(4096)),
                130,
            ),
            (format!("{}\nclass Following\n", alternating(4096)), 66),
        ] {
            let parse = parse_kotlin_file(&source);
            assert_eq!(
                parse
                    .syntax()
                    .expect("represented structural input")
                    .source_text(),
                source
            );
            assert_eq!(excessive_syntax_diagnostic_count(&source), 1);
            assert_eq!(
                count_nodes(&source, KotlinSyntaxKind::ClassDeclaration),
                following_classes
            );
        }
    }

    #[test]
    fn enum_entry_body_and_object_expression_reentry_are_bounded() {
        let enum_entry = format!(
            "{}{}\nclass Following\n",
            "enum class Nested { Entry {".repeat(4096),
            "}}".repeat(4096)
        );
        let parse = parse_kotlin_file(&enum_entry);
        assert_eq!(
            parse
                .syntax()
                .expect("represented enum-entry input")
                .source_text(),
            enum_entry
        );
        assert_eq!(excessive_syntax_diagnostic_count(&enum_entry), 1);
        assert_eq!(count_nodes(&enum_entry, KotlinSyntaxKind::EnumEntry), 64);
        assert_eq!(
            count_nodes(&enum_entry, KotlinSyntaxKind::ClassDeclaration),
            66
        );

        let object_expression = format!(
            "fun value() = {}leaf{}\nclass Following\n",
            "object { val nested = ".repeat(4096),
            "}".repeat(4096)
        );
        let parse = parse_kotlin_file(&object_expression);
        assert_eq!(
            parse
                .syntax()
                .expect("represented object-expression input")
                .source_text(),
            object_expression
        );
        assert_eq!(excessive_syntax_diagnostic_count(&object_expression), 1);
        assert_eq!(
            count_nodes(&object_expression, KotlinSyntaxKind::ClassDeclaration),
            1
        );
    }
}
