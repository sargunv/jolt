use super::{JavaParserExt, JavaSyntaxKind, Parser, StopSet};
use jolt_syntax::{DiagnosticMarker, Marker, NodeAnchor, UnresolvedDiagnosticOwner};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum LambdaParameterStyle {
    Explicit,
    Implicit,
    Var,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct ParsedLambdaParameter {
    style: LambdaParameterStyle,
    varargs: bool,
    owner: NodeAnchor,
}

impl Parser<'_> {
    fn complete_bogus_expression(
        &mut self,
        expression: Marker,
        diagnostic: DiagnosticMarker,
    ) -> jolt_syntax::CompletedMarker {
        self.complete_owned_bogus(expression, diagnostic, JavaSyntaxKind::BogusExpression)
    }

    fn complete_owned_bogus(
        &mut self,
        node: Marker,
        diagnostic: DiagnosticMarker,
        kind: JavaSyntaxKind,
    ) -> jolt_syntax::CompletedMarker {
        self.own_diagnostic(diagnostic, UnresolvedDiagnosticOwner::node(node.anchor()));
        self.complete(node, kind)
    }

    fn expected_bogus_expression(
        &mut self,
        message: &str,
        consume: bool,
    ) -> jolt_syntax::CompletedMarker {
        let expression = self.start();
        let diagnostic = self.expected_here(message);
        if consume && !self.at_eof() {
            self.bump();
        }
        self.complete_bogus_expression(expression, diagnostic)
    }

    fn completed_is_bogus_expression(expression: &jolt_syntax::CompletedMarker) -> bool {
        JavaSyntaxKind::from_raw(expression.kind()) == Some(JavaSyntaxKind::BogusExpression)
    }

    pub(super) fn consume_statement_expression_until(&mut self, stops: &[JavaSyntaxKind]) {
        if self.at_eof() || stops.contains(&self.current_kind()) {
            let error = self.start();
            let diagnostic =
                self.invalid_statement_expression_here("expected statement expression");
            self.complete_bogus_expression(error, diagnostic);
            return;
        }

        let start_kind = self.current_kind();
        let mut expression = self.parse_expression();
        if !Self::is_statement_expression(expression.kind(), start_kind)
            && !Self::completed_is_bogus_expression(&expression)
        {
            let error = self.precede(expression);
            let diagnostic =
                self.invalid_statement_expression_here("expected statement expression");
            expression = self.complete_bogus_expression(error, diagnostic);
        }

        if !self.at_eof() && !stops.contains(&self.current_kind()) {
            let error = self.precede(expression);
            let diagnostic =
                self.invalid_statement_expression_here("unexpected token in statement expression");
            while !self.at_eof() && !stops.contains(&self.current_kind()) {
                self.bump();
            }
            self.complete_bogus_expression(error, diagnostic);
        }
    }

    pub(super) fn is_statement_expression(
        expression: jolt_syntax::RawSyntaxKind,
        start_kind: JavaSyntaxKind,
    ) -> bool {
        matches!(
            JavaSyntaxKind::from_raw(expression),
            Some(
                JavaSyntaxKind::AssignmentExpression
                    | JavaSyntaxKind::MethodInvocationExpression
                    | JavaSyntaxKind::ObjectCreationExpression
                    | JavaSyntaxKind::PostfixExpression
            )
        ) || JavaSyntaxKind::from_raw(expression) == Some(JavaSyntaxKind::UnaryExpression)
            && matches!(
                start_kind,
                JavaSyntaxKind::PlusPlus | JavaSyntaxKind::MinusMinus
            )
    }

    pub(super) fn parse_expression_until<'a>(&mut self, stops: impl Into<StopSet<'a>>) {
        let stops = stops.into();
        if self.at_eof() || stops.contains(self.current_kind()) {
            self.expected_bogus_expression("expected expression", false);
            return;
        }

        let expression = self.parse_expression();

        if !self.at_eof() && !stops.contains(self.current_kind()) {
            let error = self.precede(expression);
            let diagnostic = self.unexpected_here("unexpected token in expression");
            while !self.at_eof() && !stops.contains(self.current_kind()) {
                self.bump();
            }
            self.complete_bogus_expression(error, diagnostic);
        }
    }

    pub(super) fn parse_expression_until_without_leading_lambda<'a>(
        &mut self,
        stops: impl Into<StopSet<'a>>,
    ) {
        let stops = stops.into();
        if self.at_eof() || stops.contains(self.current_kind()) {
            self.expected_bogus_expression("expected expression", false);
            return;
        }

        let expression = self.parse_assignment_expression_without_leading_lambda();

        if !self.at_eof() && !stops.contains(self.current_kind()) {
            let error = self.precede(expression);
            let diagnostic = self.unexpected_here("unexpected token in expression");
            while !self.at_eof() && !stops.contains(self.current_kind()) {
                self.bump();
            }
            self.complete_bogus_expression(error, diagnostic);
        }
    }

    pub(super) fn parse_expression(&mut self) -> jolt_syntax::CompletedMarker {
        self.parse_assignment_expression()
    }

    pub(super) fn parse_assignment_expression(&mut self) -> jolt_syntax::CompletedMarker {
        if self.starts_lambda_expression() {
            return self.parse_lambda_expression();
        }

        let lhs = self.parse_conditional_expression();
        let Some(operator_len) = self.assignment_operator_len() else {
            return lhs;
        };

        let lhs = if Self::is_assignment_left_hand_side(lhs.kind())
            || Self::completed_is_bogus_expression(&lhs)
        {
            lhs
        } else {
            let error = self.precede(lhs);
            let diagnostic = self.expected_here("expected assignment left-hand side");
            self.complete_owned_bogus(error, diagnostic, JavaSyntaxKind::BogusAssignmentTarget)
        };

        let assignment = self.precede(lhs);
        self.parse_assignment_operator(operator_len);
        self.parse_assignment_expression();
        self.complete(assignment, JavaSyntaxKind::AssignmentExpression)
    }

    pub(super) fn parse_assignment_expression_without_leading_lambda(
        &mut self,
    ) -> jolt_syntax::CompletedMarker {
        let lhs = self.parse_conditional_expression();
        let Some(operator_len) = self.assignment_operator_len() else {
            return lhs;
        };

        let lhs = if Self::is_assignment_left_hand_side(lhs.kind())
            || Self::completed_is_bogus_expression(&lhs)
        {
            lhs
        } else {
            let error = self.precede(lhs);
            let diagnostic = self.expected_here("expected assignment left-hand side");
            self.complete_owned_bogus(error, diagnostic, JavaSyntaxKind::BogusAssignmentTarget)
        };

        let assignment = self.precede(lhs);
        self.parse_assignment_operator(operator_len);
        self.parse_assignment_expression();
        self.complete(assignment, JavaSyntaxKind::AssignmentExpression)
    }

    fn parse_assignment_operator(&mut self, operator_len: usize) {
        let composite_kind = match operator_len {
            3 => Some(JavaSyntaxKind::RightShiftAssignmentOperator),
            4 => Some(JavaSyntaxKind::UnsignedRightShiftAssignmentOperator),
            _ => None,
        };
        let composite = composite_kind.map(|_| self.start());
        for _ in 0..operator_len {
            self.bump();
        }
        if let (Some(operator), Some(kind)) = (composite, composite_kind) {
            self.complete(operator, kind);
        }
    }

    pub(super) fn is_assignment_left_hand_side(kind: jolt_syntax::RawSyntaxKind) -> bool {
        matches!(
            JavaSyntaxKind::from_raw(kind),
            Some(
                JavaSyntaxKind::NameExpression
                    | JavaSyntaxKind::FieldAccessExpression
                    | JavaSyntaxKind::ArrayAccessExpression
            )
        )
    }

    pub(super) fn parse_conditional_expression(&mut self) -> jolt_syntax::CompletedMarker {
        let condition = self.parse_binary_expression(0);
        if !self.eat(JavaSyntaxKind::Question) {
            return condition;
        }

        let conditional = self.precede(condition);
        self.parse_expression();
        self.expect_owned(
            JavaSyntaxKind::Colon,
            "expected `:` in conditional expression",
            conditional.anchor(),
            crate::shape::conditional_expression::Slot::colon as u16,
        );
        self.parse_assignment_expression();
        self.complete(conditional, JavaSyntaxKind::ConditionalExpression)
    }

    pub(super) fn parse_binary_expression(
        &mut self,
        minimum_precedence: u8,
    ) -> jolt_syntax::CompletedMarker {
        let mut lhs = self.parse_unary_expression();

        while let Some(operator_info) = self.binary_operator() {
            if operator_info.precedence < minimum_precedence {
                break;
            }

            let binary = self.precede(lhs);
            let operator = self.current_kind();
            self.parse_binary_operator(operator_info.len);

            if operator == JavaSyntaxKind::InstanceofKw {
                if self.starts_pattern() {
                    self.parse_pattern_until(&[
                        JavaSyntaxKind::Semicolon,
                        JavaSyntaxKind::RParen,
                        JavaSyntaxKind::RBracket,
                        JavaSyntaxKind::Comma,
                        JavaSyntaxKind::Colon,
                    ]);
                } else {
                    self.parse_reference_type();
                }
            } else {
                self.parse_binary_expression(operator_info.precedence + 1);
            }

            let expression_kind = if operator == JavaSyntaxKind::InstanceofKw {
                JavaSyntaxKind::InstanceofExpression
            } else {
                JavaSyntaxKind::BinaryExpression
            };
            lhs = self.complete(binary, expression_kind);
        }

        lhs
    }

    fn parse_binary_operator(&mut self, operator_len: usize) {
        let composite_kind = match operator_len {
            2 if self.nth_kind(1) == JavaSyntaxKind::Assign => {
                Some(JavaSyntaxKind::GreaterThanOrEqualOperator)
            }
            2 => Some(JavaSyntaxKind::RightShiftOperator),
            3 => Some(JavaSyntaxKind::UnsignedRightShiftOperator),
            _ => None,
        };
        let composite = composite_kind.map(|_| self.start());
        for _ in 0..operator_len {
            self.bump();
        }
        if let (Some(operator), Some(kind)) = (composite, composite_kind) {
            self.complete(operator, kind);
        }
    }

    pub(super) fn parse_unary_expression(&mut self) -> jolt_syntax::CompletedMarker {
        self.parse_unary_expression_with_decimal_boundary_literal(false)
    }

    pub(super) fn parse_unary_expression_with_decimal_boundary_literal(
        &mut self,
        allow_decimal_boundary_literal: bool,
    ) -> jolt_syntax::CompletedMarker {
        if matches!(
            self.current_kind(),
            JavaSyntaxKind::PlusPlus
                | JavaSyntaxKind::MinusMinus
                | JavaSyntaxKind::Plus
                | JavaSyntaxKind::Minus
                | JavaSyntaxKind::Bang
                | JavaSyntaxKind::Tilde
        ) {
            let unary = self.start();
            let operator = self.current_kind();
            self.bump();
            if Self::is_expression_recovery_boundary(self.current_kind()) {
                self.expected_bogus_expression("expected expression", false);
            } else {
                self.parse_unary_expression_with_decimal_boundary_literal(
                    operator == JavaSyntaxKind::Minus,
                );
            }
            return self.complete(unary, JavaSyntaxKind::UnaryExpression);
        }

        if self.starts_cast_expression() {
            return self.parse_cast_expression();
        }

        self.parse_postfix_expression(allow_decimal_boundary_literal)
    }

    /// Delimiters owned by the surrounding grammar must remain available when
    /// a prefix operator has no operand. Consuming one here would make the
    /// malformed expression own the next statement or list entry as recovery.
    const fn is_expression_recovery_boundary(kind: JavaSyntaxKind) -> bool {
        matches!(
            kind,
            JavaSyntaxKind::Semicolon
                | JavaSyntaxKind::Comma
                | JavaSyntaxKind::Colon
                | JavaSyntaxKind::RParen
                | JavaSyntaxKind::RBracket
                | JavaSyntaxKind::RBrace
        )
    }

    pub(super) fn parse_cast_expression(&mut self) -> jolt_syntax::CompletedMarker {
        let cast = self.start();
        self.expect_owned(
            JavaSyntaxKind::LParen,
            "expected `(` in cast expression",
            cast.anchor(),
            crate::shape::cast_expression::Slot::open_paren as u16,
        );
        let mut lookahead = self.lookahead();
        lookahead.skip_annotations();
        let is_primitive_cast = lookahead.at_primitive_type_start();
        self.parse_intersection_type();
        self.expect_owned(
            JavaSyntaxKind::RParen,
            "expected `)` after cast type",
            cast.anchor(),
            crate::shape::cast_expression::Slot::close_paren as u16,
        );

        if is_primitive_cast {
            self.parse_unary_expression();
        } else if self.starts_lambda_expression() {
            self.parse_lambda_expression();
        } else {
            self.parse_unary_expression_with_decimal_boundary_literal(false);
        }

        self.complete(cast, JavaSyntaxKind::CastExpression)
    }

    pub(super) fn parse_postfix_expression(
        &mut self,
        allow_decimal_boundary_literal: bool,
    ) -> jolt_syntax::CompletedMarker {
        let mut expression = self.parse_primary_expression(allow_decimal_boundary_literal);
        let mut type_name_like =
            JavaSyntaxKind::from_raw(expression.kind()) == Some(JavaSyntaxKind::NameExpression);

        loop {
            match self.current_kind() {
                JavaSyntaxKind::Lt if self.type_arguments_are_followed_by_double_colon() => {
                    self.parse_optional_type_argument_list();
                }
                JavaSyntaxKind::LParen if Self::can_call_with_argument_list(expression.kind()) => {
                    self.parse_argument_list();
                    let form = self.precede(expression);
                    let form = self.complete(form, JavaSyntaxKind::UnqualifiedMethodInvocation);
                    let invocation = self.precede(form);
                    expression =
                        self.complete(invocation, JavaSyntaxKind::MethodInvocationExpression);
                    type_name_like = false;
                }
                JavaSyntaxKind::LBracket if self.nth_kind(1) == JavaSyntaxKind::RBracket => {
                    self.parse_array_dimensions();
                }
                JavaSyntaxKind::LBracket => {
                    let access = self.precede(expression);
                    self.bump();
                    self.parse_expression_until(&[JavaSyntaxKind::RBracket]);
                    self.expect_owned(
                        JavaSyntaxKind::RBracket,
                        "expected `]` after array index",
                        access.anchor(),
                        crate::shape::array_access_expression::Slot::close_bracket as u16,
                    );
                    expression = self.complete(access, JavaSyntaxKind::ArrayAccessExpression);
                    type_name_like = false;
                }
                JavaSyntaxKind::Dot => {
                    expression = self.parse_dot_suffix(expression, type_name_like);
                    type_name_like = type_name_like
                        && JavaSyntaxKind::from_raw(expression.kind())
                            == Some(JavaSyntaxKind::FieldAccessExpression);
                }
                JavaSyntaxKind::DoubleColon => {
                    expression = self.method_reference_receiver(expression);
                    let reference = self.precede(expression);
                    self.parse_method_reference_suffix(reference.anchor());
                    expression =
                        self.complete(reference, JavaSyntaxKind::MethodReferenceExpression);
                    type_name_like = false;
                }
                JavaSyntaxKind::PlusPlus | JavaSyntaxKind::MinusMinus => {
                    let postfix = self.precede(expression);
                    self.bump();
                    expression = self.complete(postfix, JavaSyntaxKind::PostfixExpression);
                    type_name_like = false;
                }
                _ => break,
            }
        }

        expression
    }

    pub(super) fn can_call_with_argument_list(kind: jolt_syntax::RawSyntaxKind) -> bool {
        JavaSyntaxKind::from_raw(kind) == Some(JavaSyntaxKind::NameExpression)
    }

    pub(super) fn parse_dot_suffix(
        &mut self,
        expression: jolt_syntax::CompletedMarker,
        type_name_like: bool,
    ) -> jolt_syntax::CompletedMarker {
        self.expect(JavaSyntaxKind::Dot, "expected `.`");

        if self.at(JavaSyntaxKind::ClassKw) {
            let expression = self.class_literal_target(expression, type_name_like);
            self.bump();
            let suffix = self.precede(expression);
            return self.complete(suffix, JavaSyntaxKind::ClassLiteralExpression);
        }

        if self.at(JavaSyntaxKind::NewKw) {
            let suffix = self.precede(expression);
            self.parse_object_creation_after_new(suffix.anchor());
            return self.complete(suffix, JavaSyntaxKind::ObjectCreationExpression);
        }

        if self.eat(JavaSyntaxKind::ThisKw) {
            let suffix = self.precede(expression);
            return self.complete(suffix, JavaSyntaxKind::ThisExpression);
        }

        if self.eat(JavaSyntaxKind::SuperKw) {
            let suffix = self.precede(expression);
            return self.complete(suffix, JavaSyntaxKind::SuperExpression);
        }

        // Java string templates were preview syntax in JDK 21/22 and withdrawn
        // for JDK 23. Keep this parser shape for legacy preview sources only.
        if matches!(
            self.current_kind(),
            JavaSyntaxKind::StringLiteral | JavaSyntaxKind::TextBlockLiteral
        ) {
            self.parse_literal_expression(false);
            let suffix = self.precede(expression);
            return self.complete(suffix, JavaSyntaxKind::TemplateExpression);
        }

        let suffix = self.precede(expression);
        self.parse_optional_type_argument_list();
        let name_slot = if self.at(JavaSyntaxKind::LParen) {
            crate::shape::qualified_method_invocation::Slot::name as u16
        } else {
            crate::shape::field_access_expression::Slot::name as u16
        };
        self.expect_method_identifier_owned("expected member name", suffix.anchor(), name_slot);
        if self.at(JavaSyntaxKind::Lt) && self.type_arguments_are_followed_by_double_colon() {
            self.parse_optional_type_argument_list();
        }
        if self.at(JavaSyntaxKind::LParen) {
            self.parse_argument_list();
            let form = self.complete(suffix, JavaSyntaxKind::QualifiedMethodInvocation);
            let invocation = self.precede(form);
            self.complete(invocation, JavaSyntaxKind::MethodInvocationExpression)
        } else {
            self.complete(suffix, JavaSyntaxKind::FieldAccessExpression)
        }
    }

    pub(super) fn parse_primary_expression(
        &mut self,
        allow_decimal_boundary_literal: bool,
    ) -> jolt_syntax::CompletedMarker {
        if self.at_contextual("yield") && self.nth_kind(1) == JavaSyntaxKind::LParen {
            let error = self.start();
            let diagnostic = self.unqualified_yield_method_invocation_here(
                "unqualified `yield` method invocation is not allowed",
            );
            self.bump();
            self.parse_argument_list();
            return self.complete_bogus_expression(error, diagnostic);
        }

        if self.starts_literal_expression() {
            return self.parse_literal_expression(allow_decimal_boundary_literal);
        }

        if self.at(JavaSyntaxKind::ThisKw) {
            let this = self.start();
            self.bump();
            return self.complete(this, JavaSyntaxKind::ThisExpression);
        }

        if self.at(JavaSyntaxKind::SuperKw) {
            let super_expression = self.start();
            self.bump();
            return self.complete(super_expression, JavaSyntaxKind::SuperExpression);
        }

        if self.at(JavaSyntaxKind::SwitchKw) {
            return self.parse_switch_expression_fragment();
        }

        if self.at(JavaSyntaxKind::NewKw) {
            return self.parse_new_expression_fragment();
        }

        if self.at(JavaSyntaxKind::LParen) {
            let parenthesized = self.start();
            self.bump();
            if self.at(JavaSyntaxKind::RParen) {
                self.expected_owned_slot(
                    "expected expression",
                    parenthesized.anchor(),
                    crate::shape::parenthesized_expression::Slot::expression as u16,
                );
            } else {
                self.parse_expression_until(&[JavaSyntaxKind::RParen]);
            }
            self.expect_owned(
                JavaSyntaxKind::RParen,
                "expected `)` after expression",
                parenthesized.anchor(),
                crate::shape::parenthesized_expression::Slot::close_paren as u16,
            );
            return self.complete(parenthesized, JavaSyntaxKind::ParenthesizedExpression);
        }

        if self.starts_array_method_reference_type() {
            return self.parse_array_method_reference_type();
        }

        if self.starts_primitive_or_void_class_literal() {
            let literal = self.start();
            if self.at(JavaSyntaxKind::VoidKw) {
                self.parse_void_type();
            } else {
                let primitive = self.start();
                self.bump();
                self.complete(primitive, JavaSyntaxKind::PrimitiveType);
                self.parse_array_dimensions();
            }
            self.expect_owned(
                JavaSyntaxKind::Dot,
                "expected `.` in class literal",
                literal.anchor(),
                crate::shape::class_literal_expression::Slot::dot as u16,
            );
            self.expect_owned(
                JavaSyntaxKind::ClassKw,
                "expected `class` in class literal",
                literal.anchor(),
                crate::shape::class_literal_expression::Slot::class_keyword as u16,
            );
            return self.complete(literal, JavaSyntaxKind::ClassLiteralExpression);
        }

        if self.at(JavaSyntaxKind::At) && self.nth_kind(1) != JavaSyntaxKind::InterfaceKw {
            let annotated = self.start();
            self.parse_annotations();
            if self.at_name_segment() {
                self.bump();
                return self.complete(annotated, JavaSyntaxKind::NameExpression);
            }

            let diagnostic = self.expected_here("expected expression after annotation");
            return self.complete_bogus_expression(annotated, diagnostic);
        }

        if self.at_name_segment() {
            let name = self.start();
            self.bump();
            return self.complete(name, JavaSyntaxKind::NameExpression);
        }

        let consume = !Self::is_expression_recovery_boundary(self.current_kind());
        self.expected_bogus_expression("expected expression", consume)
    }

    pub(super) fn starts_array_method_reference_type(&mut self) -> bool {
        let mut lookahead = self.lookahead();
        if !lookahead.at_non_void_type_start() || !lookahead.skip_type_base() {
            return false;
        }

        let mut saw_array_dimensions = false;
        loop {
            lookahead.skip_annotations();
            if lookahead.at(JavaSyntaxKind::LBracket)
                && lookahead.nth_kind(1) == JavaSyntaxKind::RBracket
            {
                saw_array_dimensions = true;
                lookahead.bump();
                lookahead.bump();
            } else {
                break;
            }
        }

        saw_array_dimensions && lookahead.at(JavaSyntaxKind::DoubleColon)
    }

    fn class_literal_target(
        &mut self,
        target: jolt_syntax::CompletedMarker,
        type_name_like: bool,
    ) -> jolt_syntax::CompletedMarker {
        if type_name_like {
            return target;
        }

        let bogus = self.precede(target);
        let diagnostic = self.expected_here("expected type name before class literal");
        self.complete_owned_bogus(bogus, diagnostic, JavaSyntaxKind::BogusClassLiteralTarget)
    }

    fn method_reference_receiver(
        &mut self,
        receiver: jolt_syntax::CompletedMarker,
    ) -> jolt_syntax::CompletedMarker {
        if matches!(
            JavaSyntaxKind::from_raw(receiver.kind()),
            Some(
                JavaSyntaxKind::LiteralExpression
                    | JavaSyntaxKind::TemplateExpression
                    | JavaSyntaxKind::NameExpression
                    | JavaSyntaxKind::ThisExpression
                    | JavaSyntaxKind::SuperExpression
                    | JavaSyntaxKind::ParenthesizedExpression
                    | JavaSyntaxKind::ClassLiteralExpression
                    | JavaSyntaxKind::FieldAccessExpression
                    | JavaSyntaxKind::ArrayAccessExpression
                    | JavaSyntaxKind::MethodInvocationExpression
                    | JavaSyntaxKind::ObjectCreationExpression
                    | JavaSyntaxKind::ArrayCreationExpression
                    | JavaSyntaxKind::SwitchExpression
                    | JavaSyntaxKind::ClassType
                    | JavaSyntaxKind::ArrayType
            )
        ) {
            return receiver;
        }

        let bogus = self.precede(receiver);
        let diagnostic = self.expected_here("expected valid method reference receiver");
        self.complete_owned_bogus(
            bogus,
            diagnostic,
            JavaSyntaxKind::BogusMethodReferenceReceiver,
        )
    }

    pub(super) fn parse_array_method_reference_type(&mut self) -> jolt_syntax::CompletedMarker {
        self.parse_type()
    }

    pub(super) fn parse_lambda_expression(&mut self) -> jolt_syntax::CompletedMarker {
        if self.at(JavaSyntaxKind::LParen) {
            self.parse_parenthesized_lambda_expression_fragment()
        } else {
            self.parse_unparenthesized_lambda_expression()
        }
    }

    pub(super) fn parse_unparenthesized_lambda_expression(
        &mut self,
    ) -> jolt_syntax::CompletedMarker {
        let lambda = self.start();
        let parameters = self.start();
        self.parse_lambda_parameter();
        self.complete(parameters, JavaSyntaxKind::LambdaParameterList);
        self.expect_owned(
            JavaSyntaxKind::Arrow,
            "expected `->` after lambda parameter",
            lambda.anchor(),
            crate::shape::lambda_expression::Slot::arrow as u16,
        );
        self.parse_lambda_body();
        self.complete(lambda, JavaSyntaxKind::LambdaExpression)
    }

    pub(super) fn parse_parenthesized_lambda_expression_fragment(
        &mut self,
    ) -> jolt_syntax::CompletedMarker {
        let lambda = self.start();
        self.expect_owned(
            JavaSyntaxKind::LParen,
            "expected lambda parameter list",
            lambda.anchor(),
            crate::shape::lambda_expression::Slot::open_paren as u16,
        );
        let list = self.start();
        let mut style = None;
        while !self.at_eof() && !self.at(JavaSyntaxKind::RParen) {
            let parameter = self.parse_lambda_parameter();
            if let Some(expected) = style {
                if parameter.style != expected {
                    let diagnostic = self.expected_here("lambda parameters must use the same form");
                    self.own_diagnostic(
                        diagnostic,
                        UnresolvedDiagnosticOwner::node(parameter.owner),
                    );
                }
            } else {
                style = Some(parameter.style);
            }

            if parameter.varargs && !self.at(JavaSyntaxKind::RParen) && !self.at_eof() {
                let diagnostic = self.expected_here("varargs lambda parameter must be last");
                self.own_diagnostic(diagnostic, UnresolvedDiagnosticOwner::node(parameter.owner));
                let consumed_comma = self.eat(JavaSyntaxKind::Comma);
                if consumed_comma {
                    continue;
                }
                break;
            }

            if !self.eat(JavaSyntaxKind::Comma) {
                break;
            }
        }
        self.complete(list, JavaSyntaxKind::LambdaParameterList);
        self.expect_owned(
            JavaSyntaxKind::RParen,
            "expected `)` after lambda parameters",
            lambda.anchor(),
            crate::shape::lambda_expression::Slot::close_paren as u16,
        );
        self.expect_owned(
            JavaSyntaxKind::Arrow,
            "expected `->` after lambda parameters",
            lambda.anchor(),
            crate::shape::lambda_expression::Slot::arrow as u16,
        );
        self.parse_lambda_body();
        self.complete(lambda, JavaSyntaxKind::LambdaExpression)
    }

    pub(super) fn parse_lambda_body(&mut self) {
        if self.at(JavaSyntaxKind::LBrace) {
            self.parse_block();
        } else {
            self.parse_assignment_expression();
        }
    }

    pub(super) fn parse_lambda_parameter(&mut self) -> ParsedLambdaParameter {
        let parameter = self.start();
        let owner = parameter.anchor();
        let (has_modifiers, has_var_modifier) = self.parse_lambda_modifiers();
        let starts_typed_parameter = self.starts_typed_lambda_parameter();
        let style = if has_var_modifier {
            LambdaParameterStyle::Var
        } else if has_modifiers && !starts_typed_parameter {
            LambdaParameterStyle::Explicit
        } else {
            self.current_lambda_parameter_style()
        };
        let mut varargs = false;
        if has_var_modifier {
            self.parse_annotations();
            self.expect_variable_identifier_owned(
                "expected lambda parameter name",
                owner,
                crate::shape::lambda_parameter::Slot::name as u16,
                true,
            );
            self.parse_array_dimensions();
        } else if starts_typed_parameter {
            self.parse_local_variable_type();
            self.parse_annotations();
            varargs = self.eat(JavaSyntaxKind::Ellipsis);
            self.expect_variable_identifier_owned(
                "expected lambda parameter name",
                owner,
                crate::shape::lambda_parameter::Slot::name as u16,
                true,
            );
            self.parse_array_dimensions();
        } else if has_modifiers {
            self.parse_annotations();
            self.expected_owned_slot(
                "expected lambda parameter type after modifiers",
                owner,
                crate::shape::lambda_parameter::Slot::r#type as u16,
            );
            self.expect_variable_identifier_owned(
                "expected lambda parameter name",
                owner,
                crate::shape::lambda_parameter::Slot::name as u16,
                true,
            );
        } else {
            self.parse_annotations();
            self.expect_variable_identifier_owned(
                "expected lambda parameter name",
                owner,
                crate::shape::lambda_parameter::Slot::name as u16,
                true,
            );
        }
        self.complete(parameter, JavaSyntaxKind::LambdaParameter);
        ParsedLambdaParameter {
            style,
            varargs,
            owner,
        }
    }

    fn parse_lambda_modifiers(&mut self) -> (bool, bool) {
        let modifiers = self.start();
        let mut saw_modifier = false;
        let mut saw_var = false;
        loop {
            if self.at(JavaSyntaxKind::At) && self.nth_kind(1) != JavaSyntaxKind::InterfaceKw {
                saw_modifier = true;
                self.parse_annotation();
            } else if self.at(JavaSyntaxKind::FinalKw) {
                saw_modifier = true;
                self.bump();
            } else if self.at_contextual("var")
                && !matches!(
                    self.nth_kind(1),
                    JavaSyntaxKind::Dot | JavaSyntaxKind::Arrow
                )
            {
                saw_modifier = true;
                saw_var = true;
                self.bump();
                break;
            } else {
                break;
            }
        }
        self.complete(modifiers, JavaSyntaxKind::LambdaModifierList);
        (saw_modifier, saw_var)
    }

    pub(super) fn current_lambda_parameter_style(&mut self) -> LambdaParameterStyle {
        if !self.starts_typed_lambda_parameter() {
            return LambdaParameterStyle::Implicit;
        }

        if self.at_contextual("var") && self.nth_kind(1) != JavaSyntaxKind::Dot {
            LambdaParameterStyle::Var
        } else {
            LambdaParameterStyle::Explicit
        }
    }

    pub(super) fn parse_new_expression_fragment(&mut self) -> jolt_syntax::CompletedMarker {
        if self.new_expression_is_array_creation() {
            self.parse_array_creation_expression_fragment()
        } else {
            self.parse_object_creation_expression_fragment()
        }
    }

    pub(super) fn parse_object_creation_expression_fragment(
        &mut self,
    ) -> jolt_syntax::CompletedMarker {
        let creation = self.start();
        self.parse_object_creation_after_new(creation.anchor());
        self.complete(creation, JavaSyntaxKind::ObjectCreationExpression)
    }

    pub(super) fn parse_object_creation_after_new(&mut self, owner: NodeAnchor) {
        self.expect(JavaSyntaxKind::NewKw, "expected `new`");
        self.parse_optional_type_argument_list();
        self.parse_object_creation_type();
        if self.at(JavaSyntaxKind::LParen) {
            self.parse_argument_list();
        } else {
            self.expected_owned_slot(
                "expected constructor arguments",
                owner,
                crate::shape::object_creation_expression::Slot::arguments as u16,
            );
        }
        if self.at(JavaSyntaxKind::LBrace) {
            self.parse_type_body(JavaSyntaxKind::ClassBody, None);
        }
    }

    pub(super) fn parse_object_creation_type(&mut self) -> jolt_syntax::CompletedMarker {
        let ty = self.start();
        let mut lookahead = self.lookahead();
        lookahead.skip_annotations();

        if lookahead.at_name_segment() {
            let segments = self.start();
            let segment = self.start();
            self.parse_annotations();
            self.parse_class_type_to_instantiate_tail();
            self.complete(segment, JavaSyntaxKind::ClassTypeSegmentNode);
            self.complete(segments, JavaSyntaxKind::ClassTypeSegmentList);
            return self.complete(ty, JavaSyntaxKind::ClassType);
        }

        self.parse_annotations();
        let diagnostic = if self.at_primitive_type() {
            let diagnostic = self.expected_here("expected class type in object creation");
            self.bump();
            diagnostic
        } else {
            self.expected_here("expected class type in object creation")
        };

        self.own_diagnostic(diagnostic, UnresolvedDiagnosticOwner::node(ty.anchor()));
        self.complete(ty, JavaSyntaxKind::BogusObjectCreationType)
    }

    pub(super) fn parse_class_type_to_instantiate_tail(&mut self) {
        if self.nth_kind(1) != JavaSyntaxKind::Dot {
            let name = self.start();
            self.bump();
            self.complete(name, JavaSyntaxKind::Name);
            self.parse_optional_type_argument_list();
            return;
        }

        let name = self.start();
        let first_segment = self.start();
        self.parse_annotations();
        self.bump();
        self.complete(first_segment, JavaSyntaxKind::QualifiedNameSegmentNode);
        self.bump();

        let remaining_segments = self.start();
        loop {
            let segment = self.start();
            self.parse_annotations();
            self.expect_type_identifier(
                "expected class type segment",
                segment.anchor(),
                crate::shape::qualified_name_segment_node::Slot::identifier as u16,
            );

            if self.at(JavaSyntaxKind::Lt) && self.type_arguments_are_followed_by_dot() {
                let diagnostic = self.unexpected_here(
                    "type arguments in class instance creation must appear on the final type segment",
                );
                self.parse_optional_type_argument_list();
                self.own_diagnostic(
                    diagnostic,
                    UnresolvedDiagnosticOwner::node(segment.anchor()),
                );
            }
            self.complete(segment, JavaSyntaxKind::QualifiedNameSegmentNode);

            if !self.at(JavaSyntaxKind::Dot) {
                break;
            }

            if !self.dot_is_followed_by_annotated_name() {
                break;
            }

            self.bump();
        }

        self.complete(remaining_segments, JavaSyntaxKind::NameSegmentDotList);
        self.complete(name, JavaSyntaxKind::QualifiedName);
        self.parse_optional_type_argument_list();
    }

    pub(super) fn parse_array_creation_expression_fragment(
        &mut self,
    ) -> jolt_syntax::CompletedMarker {
        let creation = self.start();
        self.expect(JavaSyntaxKind::NewKw, "expected `new`");
        let ty = self.parse_type();
        let ty = if matches!(
            JavaSyntaxKind::from_raw(ty.kind()),
            Some(
                JavaSyntaxKind::PrimitiveType
                    | JavaSyntaxKind::ClassType
                    | JavaSyntaxKind::ArrayType
            )
        ) {
            ty
        } else {
            let bogus = self.precede(ty);
            let diagnostic = self.expected_here("expected array element type");
            self.own_diagnostic(diagnostic, UnresolvedDiagnosticOwner::node(bogus.anchor()));
            self.complete(bogus, JavaSyntaxKind::BogusArrayCreationType)
        };
        let base_has_unsized_dimensions =
            JavaSyntaxKind::from_raw(ty.kind()) == Some(JavaSyntaxKind::ArrayType);

        let dimensions = self.start();
        let mut saw_dim_expression = false;
        while self.starts_dim_expression() {
            if base_has_unsized_dimensions {
                let diagnostic =
                    self.unexpected_here("sized array dimension cannot follow unsized dimensions");
                let owner = self.parse_dim_expression();
                self.own_diagnostic(diagnostic, UnresolvedDiagnosticOwner::node(owner));
            } else {
                self.parse_dim_expression();
            }
            saw_dim_expression = true;
        }
        self.complete(dimensions, JavaSyntaxKind::DimExpressionList);

        self.parse_array_dimensions();

        if self.at(JavaSyntaxKind::LBrace) {
            if saw_dim_expression {
                let diagnostic =
                    self.unexpected_here("array initializer cannot follow dimension expressions");
                let owner = self.parse_array_initializer_fragment();
                self.own_diagnostic(diagnostic, UnresolvedDiagnosticOwner::node(owner));
            } else {
                self.parse_array_initializer_fragment();
            }
        } else if base_has_unsized_dimensions && !saw_dim_expression {
            self.expected_owned_slot(
                "expected array initializer or dimension expression",
                creation.anchor(),
                crate::shape::array_creation_expression::Slot::initializer as u16,
            );
        }

        self.complete(creation, JavaSyntaxKind::ArrayCreationExpression)
    }

    pub(super) fn parse_dim_expression(&mut self) -> NodeAnchor {
        let dim = self.start();
        let owner = dim.anchor();
        self.parse_annotations();
        self.expect_owned(
            JavaSyntaxKind::LBracket,
            "expected `[`",
            owner,
            crate::shape::dim_expression::Slot::open_bracket as u16,
        );
        self.parse_expression_until(&[JavaSyntaxKind::RBracket]);
        self.expect_owned(
            JavaSyntaxKind::RBracket,
            "expected `]`",
            owner,
            crate::shape::dim_expression::Slot::close_bracket as u16,
        );
        self.complete(dim, JavaSyntaxKind::DimExpression);
        owner
    }

    pub(super) fn parse_array_initializer_fragment(&mut self) -> NodeAnchor {
        let initializer = self.start();
        let owner = initializer.anchor();
        self.expect_owned(
            JavaSyntaxKind::LBrace,
            "expected array initializer",
            owner,
            crate::shape::array_initializer::Slot::open_brace as u16,
        );
        let values = self.start();
        while !self.at_eof() && !self.at(JavaSyntaxKind::RBrace) {
            if self.at(JavaSyntaxKind::LBrace) {
                self.parse_array_initializer_fragment();
            } else {
                self.parse_expression_until(&[JavaSyntaxKind::Comma, JavaSyntaxKind::RBrace]);
            }

            self.eat(JavaSyntaxKind::Comma);
        }
        self.complete(values, JavaSyntaxKind::VariableInitializerList);
        self.expect_owned(
            JavaSyntaxKind::RBrace,
            "expected `}` after array initializer",
            owner,
            crate::shape::array_initializer::Slot::close_brace as u16,
        );
        self.complete(initializer, JavaSyntaxKind::ArrayInitializer);
        owner
    }

    pub(super) fn parse_argument_list(&mut self) {
        let arguments = self.start();
        self.expect_owned(
            JavaSyntaxKind::LParen,
            "expected argument list",
            arguments.anchor(),
            crate::shape::argument_list::Slot::open_paren as u16,
        );
        let expressions = self.start();
        while !self.at_eof() && !self.at(JavaSyntaxKind::RParen) {
            self.parse_expression_until(&[JavaSyntaxKind::Comma, JavaSyntaxKind::RParen]);
            if !self.eat(JavaSyntaxKind::Comma) {
                break;
            }
        }
        self.complete(expressions, JavaSyntaxKind::ExpressionList);
        self.expect_owned(
            JavaSyntaxKind::RParen,
            "expected `)` after arguments",
            arguments.anchor(),
            crate::shape::argument_list::Slot::close_paren as u16,
        );
        self.complete(arguments, JavaSyntaxKind::ArgumentList);
    }

    pub(super) fn parse_method_reference_suffix(&mut self, owner: NodeAnchor) {
        self.expect_owned(
            JavaSyntaxKind::DoubleColon,
            "expected `::`",
            owner,
            crate::shape::method_reference_expression::Slot::double_colon as u16,
        );
        self.parse_optional_type_argument_list();
        if self.at(JavaSyntaxKind::NewKw) || self.at_name_segment() {
            self.bump();
        } else {
            self.expected_owned_slot(
                "expected method reference target",
                owner,
                crate::shape::method_reference_expression::Slot::target as u16,
            );
        }
    }

    pub(super) fn parse_literal_expression(
        &mut self,
        allow_decimal_boundary_literal: bool,
    ) -> jolt_syntax::CompletedMarker {
        let literal = self.start();
        if !allow_decimal_boundary_literal && self.at_decimal_integer_boundary_literal() {
            self.decimal_integer_boundary_literal_here(
                "decimal integer boundary literal may appear only as the operand of unary minus",
            );
        }
        self.bump();
        self.complete(literal, JavaSyntaxKind::LiteralExpression)
    }

    pub(super) fn at_decimal_integer_boundary_literal(&mut self) -> bool {
        if self.current_kind() != JavaSyntaxKind::IntegerLiteral {
            return false;
        }

        let Some(text) = self.current_text() else {
            return false;
        };
        let normalized = text.replace('_', "");

        normalized == "2147483648"
            || matches!(
                normalized.strip_suffix(['l', 'L']),
                Some("9223372036854775808")
            )
    }
}
