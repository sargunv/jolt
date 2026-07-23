mod grammar;
mod source;

use std::{borrow::Cow, fmt};

use jolt_diagnostics::{Diagnostic, DiagnosticCodeId, DiagnosticStage, Severity};
use jolt_syntax::{
    ParseEvents, SyntaxDiagnosticOwner, SyntaxTokenData, SyntaxTree, SyntaxTrivia,
    build_parser_syntax_tree,
};
use jolt_text::{TextRange, TextSize};

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
    ExcessiveTypeNesting,
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
            Self::ExcessiveTypeNesting => {
                DiagnosticCodeId::new("java.parse.excessive_type_nesting")
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
    #[inline]
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
        let syntax = self.syntax();
        jolt_syntax::fmt_parse_debug(
            f,
            syntax.as_ref().map(|syntax| syntax as &dyn fmt::Debug),
            &self.diagnostics,
        )
    }
}

/// Parses a Java compilation unit.
#[must_use]
#[inline]
pub fn parse_compilation_unit(source: &str) -> JavaParse<'_> {
    let mut normalized = normalize_unicode_escapes(source);
    let mut parse = Parser::new(normalized.source()).parse_compilation_unit();
    let mut diagnostics = normalized.take_diagnostics();

    if !normalized.has_replacements() {
        return finish_parse(normalized.into_source(), parse, &mut diagnostics);
    }

    remap_parse_to_raw_source(&normalized, &mut parse);
    normalized.remap_diagnostics(&mut diagnostics);
    finish_parse(Cow::Borrowed(source), parse, &mut diagnostics)
}

fn remap_parse_to_raw_source(
    normalized: &crate::lexer::NormalizedJavaSource<'_>,
    parse: &mut ParseEvents,
) {
    normalized.remap_diagnostics(&mut parse.diagnostics);

    for token in &mut parse.tokens {
        let full_range = token.full_text_range();
        let text_range = token.token_text_range();
        let leading = token.leading();
        let trailing = token.trailing();
        let mut trivia_start = full_range.start();
        for index in leading.clone() {
            trivia_start = remap_trivia(normalized, &mut parse.trivia, index, trivia_start);
        }

        trivia_start = text_range.end();
        for index in trailing.clone() {
            trivia_start = remap_trivia(normalized, &mut parse.trivia, index, trivia_start);
        }

        *token = SyntaxTokenData::new(
            token.raw_kind(),
            normalized.raw_range(full_range),
            normalized.raw_range(text_range),
            leading,
            trailing,
        );
    }
}

fn remap_trivia(
    normalized: &crate::lexer::NormalizedJavaSource<'_>,
    trivia: &mut [SyntaxTrivia],
    index: usize,
    normalized_start: TextSize,
) -> TextSize {
    let piece = trivia[index];
    let normalized_end = normalized_start + piece.text_len();
    let raw_range = normalized.raw_range(TextRange::new(normalized_start, normalized_end));
    trivia[index] = SyntaxTrivia::new(piece.kind(), raw_range.len());
    normalized_end
}

fn finish_parse<'source>(
    source: Cow<'source, str>,
    parse: source::ParseEvents,
    diagnostics: &mut Vec<Diagnostic>,
) -> JavaParse<'source> {
    let unicode_diagnostic_count = diagnostics.len();
    diagnostics.extend(parse.diagnostics);
    let (tree, parse_diagnostic_owners) = match build_parser_syntax_tree(
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
    use jolt_test_support::assert_exact_diagnostic_owner;

    use crate::{
        ConstructorDeclaration, JavaNode, JavaSyntaxField, JavaSyntaxKind, JavaSyntaxView,
        parse_compilation_unit,
    };

    use super::JavaParseDiagnosticCode;

    #[rustfmt::skip]
    fn check(source: &str, code: jolt_diagnostics::DiagnosticCodeId, message: &str, kind: JavaSyntaxKind, slot: Option<u16>) {
        let parse = parse_compilation_unit(source);
        let root = parse.syntax().expect("represented compilation unit");
        assert_exact_diagnostic_owner(
            *root.syntax(), parse.diagnostics(), parse.structural_diagnostic_owners(),
            code, message, kind, slot,
        );
    }

    fn count_nodes(source: &str, kind: JavaSyntaxKind) -> usize {
        let parse = parse_compilation_unit(source);
        let root = parse.syntax().expect("represented compilation unit");
        let mut nodes = vec![*root.syntax()];
        let mut count = 0;
        while let Some(node) = nodes.pop() {
            nodes.extend(node.children());
            count += usize::from(node.kind() == kind);
        }
        count
    }

    fn count_nodes_owned_by(
        source: &str,
        owner_kind: JavaSyntaxKind,
        kind: JavaSyntaxKind,
    ) -> usize {
        let parse = parse_compilation_unit(source);
        let root = parse.syntax().expect("represented compilation unit");
        let mut nodes = vec![*root.syntax()];
        let owner = loop {
            let node = nodes.pop().expect("expected owner node");
            nodes.extend(node.children());
            if node.kind() == owner_kind {
                break node;
            }
        };
        let mut descendants = vec![owner];
        let mut count = 0;
        while let Some(node) = descendants.pop() {
            descendants.extend(node.children());
            count += usize::from(node.kind() == kind);
        }
        count
    }

    #[test]
    fn missing_constructor_open_paren_keeps_constructor_shape() {
        let parse = parse_compilation_unit("class C { C int value) throws Exception {} }");
        let root = parse.syntax().expect("represented compilation unit");
        let mut nodes = vec![*root.syntax()];
        let constructor = loop {
            let node = nodes.pop().expect("constructor declaration");
            nodes.extend(node.children());
            if let Some(constructor) = ConstructorDeclaration::cast(node) {
                break constructor;
            }
        };

        assert!(matches!(
            constructor.open_paren(),
            JavaSyntaxField::Missing(_)
        ));
        assert!(matches!(
            constructor.close_paren(),
            JavaSyntaxField::Present(_)
        ));
        assert!(matches!(
            constructor.body(),
            JavaSyntaxField::Present(body)
                if body.syntax_node().is_some_and(|node| node.kind() == JavaSyntaxKind::ConstructorBody)
        ));
    }

    #[test]
    fn missing_constructor_open_paren_probe_matches_consuming_grammar() {
        for source in [
            "class C { C int x int y) {} }",
            "class C { C C... this) {} }",
            "class C { C @() int x) {} }",
            "class C { C List<+ String> value) {} }",
            "class C { C int x) throws List<+ String> {} }",
            "class C { C) throws {} }",
            "class C { C int value) throws , @Thrown pkg.Error {} }",
            "class C { C @A(values = {1, 2}) java.util.Map<@B String, java.util.List<@C Integer[]>>[] value) throws pkg.@D Error<@E String>.Nested {} }",
            "class Outer { class C { C @A Outer.@B Inner Outer.this) {} } }",
        ] {
            let parse = parse_compilation_unit(source);
            let root = parse.syntax().expect("represented compilation unit");
            let mut nodes = vec![*root.syntax()];
            let mut constructors = Vec::new();
            while let Some(node) = nodes.pop() {
                nodes.extend(node.children());
                if let Some(constructor) = ConstructorDeclaration::cast(node) {
                    constructors.push(constructor);
                }
            }
            assert_eq!(
                constructors.len(),
                1,
                "constructor dispatch diverged for `{source}`",
            );
            let constructor = &constructors[0];
            assert!(
                matches!(constructor.close_paren(), JavaSyntaxField::Present(_)),
                "constructor recovery did not consume `)` for `{source}`",
            );
            assert!(
                matches!(constructor.body(), JavaSyntaxField::Present(_)),
                "constructor recovery did not reach its body for `{source}`",
            );
        }
    }

    #[test]
    fn missing_constructor_open_paren_throws_probe_stops_before_neighboring_members() {
        let annotation_braces = "class C { C int x) throws @A({E.class}) E; C field; }";
        assert_eq!(
            count_nodes(annotation_braces, JavaSyntaxKind::ConstructorDeclaration),
            0
        );
        assert_eq!(
            count_nodes(annotation_braces, JavaSyntaxKind::FieldDeclaration),
            1
        );

        let unterminated_annotation = "class C { C int x) throws @A(E; C field; }";
        assert_eq!(
            count_nodes(unterminated_annotation, JavaSyntaxKind::ClassDeclaration),
            1,
        );
        assert_eq!(
            count_nodes(unterminated_annotation, JavaSyntaxKind::FieldDeclaration),
            1,
        );

        let qualified_suffix = (0..256)
            .map(|index| format!("p{index}"))
            .collect::<Vec<_>>()
            .join(".");
        let long_rejected_fragment =
            format!("class C {{ C int x) throws E {qualified_suffix} +; C field; }}");
        assert_eq!(
            count_nodes(&long_rejected_fragment, JavaSyntaxKind::FieldDeclaration),
            1,
        );

        let annotations = (0..256)
            .map(|index| format!("@A{index}"))
            .collect::<Vec<_>>()
            .join(" ");
        let long_type_argument_recovery =
            format!("class C {{ java.util.List<+ {annotations} String> field; }}");
        assert_eq!(
            count_nodes(
                &long_type_argument_recovery,
                JavaSyntaxKind::FieldDeclaration,
            ),
            1,
        );

        let nested_parameter_delimiter = "class C { C foo(bar) int p) throws E C field; }";
        assert_eq!(
            count_nodes(nested_parameter_delimiter, JavaSyntaxKind::FieldDeclaration,),
            1,
        );

        let nested_parameter_semicolon = "class C { C foo(a; b) int p) throws E C field; }";
        assert_eq!(
            count_nodes(nested_parameter_semicolon, JavaSyntaxKind::FieldDeclaration,),
            1,
        );

        let type_body_boundary = "class C { C foo } int x) {}";
        assert_eq!(
            count_nodes(type_body_boundary, JavaSyntaxKind::ConstructorDeclaration),
            0,
        );

        let unmatched_paren_before_type_body_boundary = "class C { C foo(int x } int y)) {}";
        assert_eq!(
            count_nodes(
                unmatched_paren_before_type_body_boundary,
                JavaSyntaxKind::ConstructorDeclaration,
            ),
            0,
        );

        let following_method = "class C { C int x) throws E void m() {} }";
        assert_eq!(
            count_nodes(following_method, JavaSyntaxKind::ConstructorDeclaration),
            0
        );
        assert_eq!(
            count_nodes(following_method, JavaSyntaxKind::MethodDeclaration),
            1
        );

        let following_field = "class C { C int x) throws E C field; }";
        assert_eq!(
            count_nodes(following_field, JavaSyntaxKind::ConstructorDeclaration),
            0
        );
        assert_eq!(
            count_nodes(following_field, JavaSyntaxKind::FieldDeclaration),
            1
        );

        let annotated_field = "class C { C int x) throws E @A C field; }";
        assert_eq!(
            count_nodes(annotated_field, JavaSyntaxKind::FieldDeclaration),
            1
        );
        assert_eq!(
            count_nodes_owned_by(
                annotated_field,
                JavaSyntaxKind::FieldDeclaration,
                JavaSyntaxKind::Annotation,
            ),
            1,
        );

        let annotated_method = "class C { C int x) throws E @A void m() {} }";
        assert_eq!(
            count_nodes(annotated_method, JavaSyntaxKind::MethodDeclaration),
            1
        );
        assert_eq!(
            count_nodes_owned_by(
                annotated_method,
                JavaSyntaxKind::MethodDeclaration,
                JavaSyntaxKind::Annotation,
            ),
            1,
        );

        let following_compact_constructor = "record R(int v) { R int x) throws E R {} }";
        assert_eq!(
            count_nodes(
                following_compact_constructor,
                JavaSyntaxKind::CompactConstructorDeclaration,
            ),
            1,
        );
        assert_eq!(
            count_nodes_owned_by(
                following_compact_constructor,
                JavaSyntaxKind::CompactConstructorDeclaration,
                JavaSyntaxKind::ConstructorBody,
            ),
            1,
        );
    }

    #[test]
    fn missing_constructor_open_paren_recovery_does_not_steal_neighboring_declarations() {
        let parse = parse_compilation_unit(
            "class C { C() {} C field; C method() {} } record R(int value) { R {} }",
        );
        let root = parse.syntax().expect("represented compilation unit");
        let mut nodes = vec![*root.syntax()];
        let mut fields = 0;
        let mut methods = 0;
        let mut constructors = 0;
        let mut compact_constructors = 0;
        while let Some(node) = nodes.pop() {
            nodes.extend(node.children());
            match node.kind() {
                JavaSyntaxKind::FieldDeclaration => fields += 1,
                JavaSyntaxKind::MethodDeclaration => methods += 1,
                JavaSyntaxKind::ConstructorDeclaration => constructors += 1,
                JavaSyntaxKind::CompactConstructorDeclaration => compact_constructors += 1,
                _ => {}
            }
        }

        assert_eq!(fields, 1);
        assert_eq!(methods, 1);
        assert_eq!(constructors, 1);
        assert_eq!(compact_constructors, 1);
    }

    #[test]
    #[rustfmt::skip] // Keep the owner matrix one case per line.
    fn declaration_header_diagnostics_own_the_declared_node_or_slot() {
        let expected = JavaParseDiagnosticCode::ExpectedSyntax.id();
        let unexpected = JavaParseDiagnosticCode::UnexpectedSyntax.id();
        macro_rules! slot { ($src:literal, $msg:literal, $kind:ident, $shape:ident, $slot:ident) => {
            check($src, expected, $msg, JavaSyntaxKind::$kind, Some(crate::shape::$shape::Slot::$slot as u16));
        }; }

        slot!("interface {}", "expected type name", InterfaceDeclaration, interface_declaration, name);
        slot!("class C extends {}", "expected type", ExtendsClause, extends_clause, types);
        slot!("class C { int ; }", "expected variable name", VariableDeclarator, variable_declarator, name);
        slot!("class C { <T>() {} }", "expected constructor name", ConstructorDeclaration, constructor_declaration, name);
        slot!("class C { void (); }", "expected method name", MethodDeclaration, method_declaration, name);
        slot!("class C { void f(String) {} }", "expected parameter name", FormalParameter, formal_parameter, name);
        slot!("record R(int) {}", "expected record component name", RecordComponent, record_component, name);
        slot!("@interface A { int (); }", "expected annotation element name", AnnotationElementDeclaration, annotation_element_declaration, name);
        slot!("class C<T {}", "expected `>` after type parameters", TypeParameterList, type_parameter_list, close_angle);
        slot!("@interface A { int value() default { 1", "expected `}` after annotation array initializer", AnnotationArrayInitializer, annotation_array_initializer, close_brace);
        check("native class C {}", unexpected, "invalid type modifier", JavaSyntaxKind::BogusModifier, None);
        check("class C { void f(public int x) {} }", unexpected, "invalid parameter modifier", JavaSyntaxKind::BogusModifier, None);
        check("@interface A { A value() default @A(first,,second); }", expected, "expected annotation argument", JavaSyntaxKind::BogusAnnotationArgument, None);
        check("class C<T extends int> {}", expected, "expected class or interface type", JavaSyntaxKind::BogusType, None);
        check("class C { void f(String value, C this) {} }", JavaParseDiagnosticCode::MisplacedReceiverParameter.id(), "receiver parameter must be first", JavaSyntaxKind::BogusFormalParameter, None);
        check("class C { void f(@A final C this) {} }", unexpected, "invalid receiver parameter modifier", JavaSyntaxKind::BogusFormalParameter, None);
        check("class C { void f(C... this) {} }", unexpected, "invalid receiver parameter", JavaSyntaxKind::BogusFormalParameter, None);
        check("class var {}", JavaParseDiagnosticCode::RestrictedTypeIdentifier.id(), "expected type name", JavaSyntaxKind::ClassDeclaration, None);
    }

    #[test]
    #[rustfmt::skip] // Keep the owner matrix one case per line.
    fn declaration_body_diagnostics_own_the_declared_node_or_slot() {
        let expected = JavaParseDiagnosticCode::ExpectedSyntax.id();
        let unexpected = JavaParseDiagnosticCode::UnexpectedSyntax.id();
        macro_rules! slot { ($src:literal, $msg:literal, $kind:ident, $shape:ident, $slot:ident) => {
            check($src, expected, $msg, JavaSyntaxKind::$kind, Some(crate::shape::$shape::Slot::$slot as u16));
        }; }

        slot!("class C", "expected type body", ClassBody, class_body, open_brace);
        slot!("class C {", "expected `}` after type body", ClassBody, class_body, close_brace);
        slot!("record R {}", "expected record header", RecordDeclaration, record_declaration, open_paren);
        slot!("record R(int value {}", "expected `)` after record header", RecordDeclaration, record_declaration, close_paren);
        slot!("class C { int value }", "expected `;` after field declaration", FieldDeclaration, field_declaration, semicolon);
        slot!("class C { void f(int value {}", "expected `)` after parameters", MethodDeclaration, method_declaration, close_paren);
        slot!("class C { void f() }", "expected method body", MethodDeclaration, method_declaration, body);
        slot!("class C { C() }", "expected constructor body", ConstructorBody, constructor_body, open_brace);
        slot!("class C { C() {", "expected `}` after constructor body", ConstructorBody, constructor_body, close_brace);
        slot!("@interface A { int value(); int missing() }", "expected `;` after annotation element", AnnotationElementDeclaration, annotation_element_declaration, semicolon);
        slot!("enum E { , }", "expected enum constant name", EnumConstant, enum_constant, name);
        check("class C { +; }", unexpected, "unexpected token in type body", JavaSyntaxKind::BogusClassBodyMember, None);
        check("class C { C() { this(); this(); } }", JavaParseDiagnosticCode::MisplacedConstructorInvocation.id(), "constructor body must have at most one explicit constructor invocation", JavaSyntaxKind::BogusConstructorBodyEntry, None);
        check("class C { C() { <T>lost this(); } }", expected, "expected `this` or `super` in constructor invocation", JavaSyntaxKind::BogusConstructorBodyEntry, None);
    }

    #[test]
    #[rustfmt::skip] // Keep the owner matrix one case per line.
    fn expression_diagnostics_own_the_declared_node_or_slot() {
        let expected = JavaParseDiagnosticCode::ExpectedSyntax.id();
        macro_rules! slot { ($src:literal, $msg:literal, $kind:ident, $shape:ident, $slot:ident) => {
            check($src, expected, $msg, JavaSyntaxKind::$kind, Some(crate::shape::$shape::Slot::$slot as u16));
        }; }

        slot!("class C { Object x = value ? left right; }", "expected `:` in conditional expression", ConditionalExpression, conditional_expression, colon);
        slot!("class C { Object x = values[index; }", "expected `]` after array index", ArrayAccessExpression, array_access_expression, close_bracket);
        slot!("class C { Object x = new Value; }", "expected constructor arguments", ObjectCreationExpression, object_creation_expression, arguments);
        slot!("class C { Object x = new int[1; }", "expected `]`", DimExpression, dim_expression, close_bracket);
        slot!("class C { Object x = new int[] { 1", "expected `}` after array initializer", ArrayInitializer, array_initializer, close_brace);
        slot!("class C { Object x = value::; }", "expected method reference target", MethodReferenceExpression, method_reference_expression, target);
        slot!("class C { Object x = (value; }", "expected `)` after expression", ParenthesizedExpression, parenthesized_expression, close_paren);
        slot!("class C { Object x = (); }", "expected expression", ParenthesizedExpression, parenthesized_expression, expression);
        slot!("class C { void f() { a.; } }", "expected member name", FieldAccessExpression, field_access_expression, name);
        slot!("class C { void f() { a.(); } }", "expected member name", QualifiedMethodInvocation, qualified_method_invocation, name);
        slot!("class C { Object x = (int) -> 1; }", "expected lambda parameter name", LambdaParameter, lambda_parameter, name);
        slot!("class C { boolean f(Object x) { return x instanceof Point(var y", "expected `)` after record pattern", RecordPattern, record_pattern, close_paren);
        slot!("class C { boolean f(Object x) { return x instanceof Point(String); } }", "expected pattern variable name", TypePattern, type_pattern, name);
        check("class C { void f() { 1 = 2; } }", expected, "expected assignment left-hand side", JavaSyntaxKind::BogusAssignmentTarget, None);
        check("class C { Object x = new int(); }", expected, "expected class type in object creation", JavaSyntaxKind::BogusObjectCreationType, None);
        check("class C { Class<?> x = new Object().class; }", expected, "expected type name before class literal", JavaSyntaxKind::BogusClassLiteralTarget, None);
        check("class C { Class<?> x = (value).class; }", expected, "expected type name before class literal", JavaSyntaxKind::BogusClassLiteralTarget, None);
        check("class C { Class<?> x = (value).field.class; }", expected, "expected type name before class literal", JavaSyntaxKind::BogusClassLiteralTarget, None);
        check("class C { Class<?> x = new Object().field.class; }", expected, "expected type name before class literal", JavaSyntaxKind::BogusClassLiteralTarget, None);
        check("class C { Class<?> x = 1 .class; }", expected, "expected type name before class literal", JavaSyntaxKind::BogusClassLiteralTarget, None);
        check("class C { Class<?> x = void[].class; }", expected, "expected type name before class literal", JavaSyntaxKind::BogusClassLiteralTarget, None);
        check("class C { Object x = value++::target; }", expected, "expected valid method reference receiver", JavaSyntaxKind::BogusMethodReferenceReceiver, None);
        check("class C { Object x = a::b::c; }", expected, "expected valid method reference receiver", JavaSyntaxKind::BogusMethodReferenceReceiver, None);
        check("class C { int x = +; }", expected, "expected expression", JavaSyntaxKind::BogusExpression, None);
        check("class C { boolean f(Object x) { return x instanceof int value; } }", expected, "expected reference type", JavaSyntaxKind::BogusType, None);
        check("class C { boolean f(Object x) { return x instanceof var value; } }", expected, "expected reference type", JavaSyntaxKind::BogusType, None);
        check("class C { boolean f(Object x) { return x instanceof int(String s); } }", expected, "expected class or interface type", JavaSyntaxKind::BogusType, None);
        check("class C { boolean f(Object x) { return x instanceof Point(String s = value); } }", JavaParseDiagnosticCode::UnexpectedSyntax.id(), "invalid type pattern declaration", JavaSyntaxKind::BogusPattern, None);
    }

    #[test]
    #[rustfmt::skip] // Keep the owner matrix one case per line.
    fn statement_diagnostics_own_the_declared_node_or_slot() {
        let expected = JavaParseDiagnosticCode::ExpectedSyntax.id();
        macro_rules! slot { ($src:literal, $msg:literal, $kind:ident, $shape:ident, $slot:ident) => {
            check($src, expected, $msg, JavaSyntaxKind::$kind, Some(crate::shape::$shape::Slot::$slot as u16));
        }; }

        check("class C { void f(Object xs) { for (Object x = value : xs) {} } }", expected, "enhanced for variable must not have an initializer", JavaSyntaxKind::BogusEnhancedForVariable, None);
        check("class C { void f() { try (Resource value) {} } }", expected, "expected resource initializer", JavaSyntaxKind::BogusResourceValue, None);
        check("class C { void f() { try (make()) {} } }", JavaParseDiagnosticCode::InvalidResourceVariableAccess.id(), "expected resource variable declaration or variable access", JavaSyntaxKind::BogusResourceValue, None);
        check("class C { void f() { try () {} } }", expected, "expected resource", JavaSyntaxKind::BogusResourceValue, None);
        check("class C { void f(int x) { switch (x) { use(); case 1 -> use(); } } }", JavaParseDiagnosticCode::UnexpectedSyntax.id(), "expected switch label", JavaSyntaxKind::BogusSwitchEntry, None);
        check("class C { void f(int x) { switch (x) { case 1 when ok -> use(); } } }", JavaParseDiagnosticCode::InvalidSwitchGuard.id(), "switch guard requires a pattern", JavaSyntaxKind::BogusSwitchGuard, None);
        check("class C { void f(int x) { switch (x) { case 1 value -> use(); } } }", JavaParseDiagnosticCode::UnexpectedSyntax.id(), "unexpected token in case constant", JavaSyntaxKind::BogusSwitchLabelItem, None);
        check("class C { void f() { try {} } }", expected, "expected `catch` or `finally` after try block", JavaSyntaxKind::BogusStatement, None);
        slot!("class C { void f(boolean ok) { if (ok) else use(); } }", "expected statement", IfStatement, if_statement, then_branch);
        slot!("class C { void f(boolean ok) { do while (ok); } }", "expected statement", DoStatement, do_statement, body);
        slot!("class C { void f(Object lock) { synchronized (lock) } }", "expected synchronized body", SynchronizedStatement, synchronized_statement, body);
        slot!("class C { void f(int x) { switch (x) } }", "expected switch block", SwitchStatement, switch_statement, body);
        slot!("class C { void f() { try {} catch (Exception e) finally {} } }", "expected catch body", CatchClause, catch_clause, body);
        slot!("class C { void f() { try {} finally } }", "expected finally body", FinallyClause, finally_clause, body);
    }
}
