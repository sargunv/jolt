use super::{EXPRESSION_STOPS, JavaSyntaxKind, Parser, StopSet, support::SeparatedElement};
use crate::parser::codes;
use jolt_syntax::CompletedMarker;

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
}

impl Parser<'_> {
    fn expected_bogus_expression(
        &mut self,
        message: &str,
        consume: bool,
    ) -> jolt_syntax::CompletedMarker {
        let expression = self.start();
        self.error_expected(message);
        if consume && !self.at_eof() {
            self.bump();
        }
        self.complete(expression, JavaSyntaxKind::Bogus)
    }

    fn completed_is_bogus_expression(expression: CompletedMarker) -> bool {
        JavaSyntaxKind::from_raw(expression.kind()) == JavaSyntaxKind::Bogus
    }

    pub(super) fn consume_statement_expression_until(&mut self, stops: &[JavaSyntaxKind]) {
        if self.at_eof() || stops.contains(&self.current()) {
            let error = self.start();
            self.error(
                codes::INVALID_STATEMENT_EXPRESSION,
                "expected statement expression",
            );
            self.complete(error, JavaSyntaxKind::Bogus);
            return;
        }

        let start_kind = self.current();
        let mut expression = self.parse_expression();
        if !Self::is_statement_expression(expression.kind(), start_kind)
            && !Self::completed_is_bogus_expression(expression)
        {
            let error = self.precede(expression);
            self.error(
                codes::INVALID_STATEMENT_EXPRESSION,
                "expected statement expression",
            );
            expression = self.complete(error, JavaSyntaxKind::Bogus);
        }

        let stop = |kind| kind == JavaSyntaxKind::RBrace || stops.contains(&kind);
        if !self.at_eof() && !stop(self.current()) {
            let error = self.precede(expression);
            self.error(
                codes::INVALID_STATEMENT_EXPRESSION,
                "unexpected token in statement expression",
            );
            self.skip_balanced(stop);
            self.complete(error, JavaSyntaxKind::Bogus);
        }
    }

    pub(super) fn is_statement_expression(
        expression: jolt_syntax::RawSyntaxKind,
        start_kind: JavaSyntaxKind,
    ) -> bool {
        matches!(
            JavaSyntaxKind::from_raw(expression),
            JavaSyntaxKind::AssignmentExpression
                | JavaSyntaxKind::MethodInvocationExpression
                | JavaSyntaxKind::ObjectCreationExpression
                | JavaSyntaxKind::PostfixExpression
        ) || JavaSyntaxKind::from_raw(expression) == JavaSyntaxKind::UnaryExpression
            && matches!(
                start_kind,
                JavaSyntaxKind::PlusPlus | JavaSyntaxKind::MinusMinus
            )
    }

    pub(super) fn parse_expression_until<'a>(&mut self, stops: impl Into<StopSet<'a>>) {
        let stops = stops.into();
        let stop = |kind| Self::ends_enclosing_statement(kind) || stops.contains(kind);
        if self.at_eof() || stop(self.current()) {
            self.expected_bogus_expression("expected expression", false);
            return;
        }

        let expression = self.parse_expression();
        if !self.at_eof() && !stop(self.current()) {
            let error = self.precede(expression);
            self.error_unexpected("unexpected token in expression");
            self.skip_balanced(stop);
            self.complete(error, JavaSyntaxKind::Bogus);
        }
    }

    pub(super) fn parse_expression_until_without_leading_lambda<'a>(
        &mut self,
        stops: impl Into<StopSet<'a>>,
    ) {
        let stops = stops.into();
        let stop = |kind| Self::ends_enclosing_statement(kind) || stops.contains(kind);
        if self.at_eof() || stop(self.current()) {
            self.expected_bogus_expression("expected expression", false);
            return;
        }

        let expression = self.parse_assignment_expression_without_leading_lambda();
        if !self.at_eof() && !stop(self.current()) {
            let error = self.precede(expression);
            self.error_unexpected("unexpected token in expression");
            self.skip_balanced(stop);
            self.complete(error, JavaSyntaxKind::Bogus);
        }
    }

    /// A `;` or `}` outside the brackets an expression opened ends the
    /// enclosing statement, body or block: no expression contains one, so
    /// recovery never skips past it into the next statement.
    pub(super) const fn ends_enclosing_statement(kind: JavaSyntaxKind) -> bool {
        matches!(kind, JavaSyntaxKind::Semicolon | JavaSyntaxKind::RBrace)
    }

    /// The end of a list of expressions closed by `close`: `close` itself, or
    /// a token that ends the enclosing statement when `close` is missing.
    pub(super) fn at_expression_list_close(&mut self, close: JavaSyntaxKind) -> bool {
        self.at(close) || Self::ends_enclosing_statement(self.current())
    }

    pub(super) fn parse_expression(&mut self) -> jolt_syntax::CompletedMarker {
        self.parse_assignment_expression()
    }

    pub(super) fn parse_assignment_expression(&mut self) -> jolt_syntax::CompletedMarker {
        if let Some(expression) = self.nested(Self::parse_assignment_expression_inner) {
            return expression;
        }

        self.recover_too_deep(EXPRESSION_STOPS)
    }

    fn parse_assignment_expression_inner(&mut self) -> jolt_syntax::CompletedMarker {
        if self.starts_lambda_expression() {
            return self.parse_lambda_expression();
        }

        let lhs = self.parse_conditional_expression();
        self.parse_assignment_rest(lhs)
    }

    pub(super) fn parse_assignment_expression_without_leading_lambda(&mut self) -> CompletedMarker {
        let lhs = self.parse_conditional_expression();
        self.parse_assignment_rest(lhs)
    }

    /// Parses `= value` after `lhs` when an assignment operator follows.
    fn parse_assignment_rest(&mut self, lhs: CompletedMarker) -> CompletedMarker {
        if !self.at_assignment_operator() {
            return lhs;
        }

        let lhs = if Self::is_assignment_left_hand_side(lhs.kind())
            || Self::completed_is_bogus_expression(lhs)
        {
            lhs
        } else {
            let error = self.precede(lhs);
            self.error_expected("expected assignment left-hand side");
            self.complete(error, JavaSyntaxKind::Bogus)
        };

        let assignment = self.precede(lhs);
        self.bump();
        self.parse_assignment_expression();
        self.complete(assignment, JavaSyntaxKind::AssignmentExpression)
    }

    pub(super) fn is_assignment_left_hand_side(kind: jolt_syntax::RawSyntaxKind) -> bool {
        matches!(
            JavaSyntaxKind::from_raw(kind),
            JavaSyntaxKind::NameExpression
                | JavaSyntaxKind::FieldAccessExpression
                | JavaSyntaxKind::ArrayAccessExpression
        )
    }

    pub(super) fn parse_conditional_expression(&mut self) -> jolt_syntax::CompletedMarker {
        let condition = self.parse_binary_expression(0);
        if !self.eat(JavaSyntaxKind::Question) {
            return condition;
        }

        let conditional = self.precede(condition);
        self.parse_expression();
        self.expect(
            JavaSyntaxKind::Colon,
            "expected `:` in conditional expression",
        );
        self.parse_assignment_expression();
        self.complete(conditional, JavaSyntaxKind::ConditionalExpression)
    }

    pub(super) fn parse_binary_expression(
        &mut self,
        minimum_precedence: u8,
    ) -> jolt_syntax::CompletedMarker {
        let mut lhs = self.parse_unary_expression();

        let mut guard = self.loop_guard();
        while let Some(precedence) = self.binary_operator() {
            if precedence < minimum_precedence || !self.check(&mut guard) {
                break;
            }

            let binary = self.precede(lhs);
            let operator = self.current();
            self.bump();

            if operator == JavaSyntaxKind::InstanceofKw {
                if let Some(pattern_start) = self.pattern_start() {
                    self.parse_pattern_until(
                        pattern_start,
                        &[
                            JavaSyntaxKind::Semicolon,
                            JavaSyntaxKind::RParen,
                            JavaSyntaxKind::RBracket,
                            JavaSyntaxKind::RBrace,
                            JavaSyntaxKind::Comma,
                            JavaSyntaxKind::Colon,
                            JavaSyntaxKind::Question,
                            JavaSyntaxKind::Arrow,
                        ],
                    );
                } else {
                    self.parse_reference_type();
                }
            } else {
                self.parse_binary_expression(precedence + 1);
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

    pub(super) fn parse_unary_expression(&mut self) -> jolt_syntax::CompletedMarker {
        self.parse_unary_expression_with_decimal_boundary_literal(false)
    }

    pub(super) fn parse_unary_expression_with_decimal_boundary_literal(
        &mut self,
        allow_decimal_boundary_literal: bool,
    ) -> jolt_syntax::CompletedMarker {
        if let Some(expression) = self
            .nested(|parser| parser.parse_unary_expression_inner(allow_decimal_boundary_literal))
        {
            return expression;
        }

        self.recover_too_deep(EXPRESSION_STOPS)
    }

    fn parse_unary_expression_inner(
        &mut self,
        allow_decimal_boundary_literal: bool,
    ) -> jolt_syntax::CompletedMarker {
        if matches!(
            self.current(),
            JavaSyntaxKind::PlusPlus
                | JavaSyntaxKind::MinusMinus
                | JavaSyntaxKind::Plus
                | JavaSyntaxKind::Minus
                | JavaSyntaxKind::Bang
                | JavaSyntaxKind::Tilde
        ) {
            let unary = self.start();
            let operator = self.current();
            self.bump();
            if Self::is_expression_recovery_boundary(self.current()) {
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
        self.expect(JavaSyntaxKind::LParen, "expected `(` in cast expression");
        let mut lookahead = self.lookahead();
        lookahead.skip_annotations();
        let is_primitive_cast = lookahead.at_primitive_type_start();
        self.parse_intersection_type();
        self.expect(JavaSyntaxKind::RParen, "expected `)` after cast type");

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
            JavaSyntaxKind::from_raw(expression.kind()) == JavaSyntaxKind::NameExpression;

        let mut guard = self.loop_guard();
        while self.check(&mut guard) {
            match self.current() {
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
                JavaSyntaxKind::LBracket if self.nth(1) == JavaSyntaxKind::RBracket => {
                    self.parse_array_dimensions();
                }
                JavaSyntaxKind::LBracket => {
                    let access = self.precede(expression);
                    self.bump();
                    self.parse_expression_until(&[JavaSyntaxKind::RBracket]);
                    self.expect(JavaSyntaxKind::RBracket, "expected `]` after array index");
                    expression = self.complete(access, JavaSyntaxKind::ArrayAccessExpression);
                    type_name_like = false;
                }
                JavaSyntaxKind::Dot => {
                    expression = self.parse_dot_suffix(expression, type_name_like);
                    type_name_like = type_name_like
                        && JavaSyntaxKind::from_raw(expression.kind())
                            == JavaSyntaxKind::FieldAccessExpression;
                }
                JavaSyntaxKind::DoubleColon => {
                    expression = self.method_reference_receiver(expression);
                    let reference = self.precede(expression);
                    self.parse_method_reference_suffix();
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
        JavaSyntaxKind::from_raw(kind) == JavaSyntaxKind::NameExpression
    }

    pub(super) fn parse_dot_suffix(
        &mut self,
        expression: jolt_syntax::CompletedMarker,
        type_name_like: bool,
    ) -> jolt_syntax::CompletedMarker {
        if self.nth(1) == JavaSyntaxKind::ClassKw {
            let target = self.class_literal_target(expression, type_name_like);
            let suffix = self.precede(target);
            self.bump();
            self.bump();
            return self.complete(suffix, JavaSyntaxKind::ClassLiteralExpression);
        }

        self.bump();

        if self.at(JavaSyntaxKind::NewKw) {
            let suffix = self.precede(expression);
            self.parse_object_creation_after_new();
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

        let suffix = self.precede(expression);
        self.parse_optional_type_argument_list();
        self.expect_named_identifier("expected member name");
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
        if self.at_contextual("yield") && self.nth(1) == JavaSyntaxKind::LParen {
            let error = self.start();
            self.error(
                codes::UNQUALIFIED_YIELD_METHOD_INVOCATION,
                "unqualified `yield` method invocation is not allowed",
            );
            self.bump();
            self.parse_argument_list();
            return self.complete(error, JavaSyntaxKind::Bogus);
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
                self.error_expected("expected expression");
            } else {
                self.parse_expression_until(&[JavaSyntaxKind::RParen]);
            }
            self.expect(JavaSyntaxKind::RParen, "expected `)` after expression");
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
            self.expect(JavaSyntaxKind::Dot, "expected `.` in class literal");
            self.expect(JavaSyntaxKind::ClassKw, "expected `class` in class literal");
            return self.complete(literal, JavaSyntaxKind::ClassLiteralExpression);
        }

        if self.at(JavaSyntaxKind::At) && self.nth(1) != JavaSyntaxKind::InterfaceKw {
            let annotated = self.start();
            self.parse_annotations();
            if self.at_name_segment() {
                self.bump();
                return self.complete(annotated, JavaSyntaxKind::NameExpression);
            }

            self.error_expected("expected expression after annotation");
            return self.complete(annotated, JavaSyntaxKind::Bogus);
        }

        if self.at_name_segment() {
            let name = self.start();
            self.bump();
            return self.complete(name, JavaSyntaxKind::NameExpression);
        }

        let consume = !Self::is_expression_recovery_boundary(self.current());
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
                && lookahead.nth(1) == JavaSyntaxKind::RBracket
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
        self.error_expected_at(1, "expected type name before class literal");
        self.complete(bogus, JavaSyntaxKind::Bogus)
    }

    fn method_reference_receiver(
        &mut self,
        receiver: jolt_syntax::CompletedMarker,
    ) -> jolt_syntax::CompletedMarker {
        if matches!(
            JavaSyntaxKind::from_raw(receiver.kind()),
            JavaSyntaxKind::LiteralExpression
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
        ) {
            return receiver;
        }

        let bogus = self.precede(receiver);
        self.error_expected("expected valid method reference receiver");
        self.complete(bogus, JavaSyntaxKind::Bogus)
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
        self.parse_lambda_parameter(None);
        self.complete(parameters, JavaSyntaxKind::LambdaParameterList);
        self.expect(
            JavaSyntaxKind::Arrow,
            "expected `->` after lambda parameter",
        );
        self.parse_lambda_body();
        self.complete(lambda, JavaSyntaxKind::LambdaExpression)
    }

    pub(super) fn parse_parenthesized_lambda_expression_fragment(
        &mut self,
    ) -> jolt_syntax::CompletedMarker {
        let lambda = self.start();
        self.expect(JavaSyntaxKind::LParen, "expected lambda parameter list");
        let list = self.start();
        let mut style = None;
        self.parse_comma_separated(
            "expected lambda parameter name",
            |parser| parser.at(JavaSyntaxKind::RParen),
            |parser| {
                let parameter = parser.parse_lambda_parameter(style);
                if style.is_none() {
                    style = Some(parameter.style);
                }

                // A varargs parameter must be last, so it decides for itself
                // whether a separator may follow.
                if parameter.varargs && !parser.at(JavaSyntaxKind::RParen) && !parser.at_eof() {
                    return if parser.eat(JavaSyntaxKind::Comma) {
                        SeparatedElement::ConsumedSeparator
                    } else {
                        SeparatedElement::Stop
                    };
                }

                SeparatedElement::Parsed
            },
        );
        self.complete(list, JavaSyntaxKind::LambdaParameterList);
        self.expect(
            JavaSyntaxKind::RParen,
            "expected `)` after lambda parameters",
        );
        self.expect(
            JavaSyntaxKind::Arrow,
            "expected `->` after lambda parameters",
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

    pub(super) fn parse_lambda_parameter(
        &mut self,
        expected_style: Option<LambdaParameterStyle>,
    ) -> ParsedLambdaParameter {
        let parameter = self.start();
        if self.at(JavaSyntaxKind::Comma) {
            self.error_expected("expected lambda parameter name");
            self.bump();
            self.complete(parameter, JavaSyntaxKind::LambdaParameter);
            return ParsedLambdaParameter {
                style: expected_style.unwrap_or(LambdaParameterStyle::Implicit),
                varargs: false,
            };
        }
        let (has_modifiers, has_var_modifier) = self.parse_lambda_modifiers();
        let starts_typed_parameter = self.starts_typed_lambda_parameter();
        let style = if has_var_modifier {
            LambdaParameterStyle::Var
        } else if has_modifiers || starts_typed_parameter {
            LambdaParameterStyle::Explicit
        } else {
            LambdaParameterStyle::Implicit
        };
        let mut varargs = false;
        if has_var_modifier {
            self.parse_annotations();
            self.expect_variable_identifier("expected lambda parameter name", true);
            self.parse_array_dimensions();
        } else if starts_typed_parameter {
            self.parse_local_variable_type();
            self.parse_annotations();
            varargs = self.eat(JavaSyntaxKind::Ellipsis);
            self.expect_variable_identifier("expected lambda parameter name", true);
            self.parse_array_dimensions();
        } else if has_modifiers {
            self.parse_annotations();
            self.error_expected("expected lambda parameter type after modifiers");
            self.expect_variable_identifier("expected lambda parameter name", true);
        } else {
            self.parse_annotations();
            self.expect_variable_identifier("expected lambda parameter name", true);
        }
        if expected_style.is_some_and(|expected| style != expected) {
            self.error_expected("lambda parameters must use the same form");
        }
        if varargs && !self.at(JavaSyntaxKind::RParen) && !self.at_eof() {
            self.error_expected("varargs lambda parameter must be last");
        }
        self.complete(parameter, JavaSyntaxKind::LambdaParameter);
        ParsedLambdaParameter { style, varargs }
    }

    fn parse_lambda_modifiers(&mut self) -> (bool, bool) {
        let modifiers = self.start();
        let mut saw_modifier = false;
        let mut saw_var = false;
        let mut guard = self.loop_guard();
        while self.check(&mut guard) {
            if self.at(JavaSyntaxKind::At) && self.nth(1) != JavaSyntaxKind::InterfaceKw {
                saw_modifier = true;
                self.parse_annotation();
            } else if self.at(JavaSyntaxKind::FinalKw) {
                saw_modifier = true;
                self.bump();
            } else if self.at_contextual("var")
                && !matches!(self.nth(1), JavaSyntaxKind::Dot | JavaSyntaxKind::Arrow)
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
        self.parse_object_creation_after_new();
        self.complete(creation, JavaSyntaxKind::ObjectCreationExpression)
    }

    pub(super) fn parse_object_creation_after_new(&mut self) {
        self.bump();
        self.parse_optional_type_argument_list();
        self.parse_object_creation_type();
        if self.at(JavaSyntaxKind::LParen) {
            self.parse_argument_list();
        } else {
            self.error_expected("expected constructor arguments");
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
        self.error_expected("expected class type in object creation");
        if self.at_primitive_type() {
            self.bump();
        }

        self.complete(ty, JavaSyntaxKind::Bogus)
    }

    pub(super) fn parse_class_type_to_instantiate_tail(&mut self) {
        if self.nth(1) != JavaSyntaxKind::Dot {
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
        // Unguarded: every further iteration first bumps a `.`.
        loop {
            let segment = self.start();
            self.parse_annotations();
            self.expect_type_identifier("expected class type segment");
            if self.at(JavaSyntaxKind::Lt) && self.type_arguments_are_followed_by_dot() {
                self.error_unexpected(
                    "type arguments in class instance creation must appear on the final type segment",
                );
                self.parse_optional_type_argument_list();
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
        self.bump();
        let ty = self.parse_type();
        let ty = if matches!(
            JavaSyntaxKind::from_raw(ty.kind()),
            JavaSyntaxKind::PrimitiveType | JavaSyntaxKind::ClassType | JavaSyntaxKind::ArrayType
        ) {
            ty
        } else {
            let bogus = self.precede(ty);
            self.error_expected("expected array element type");
            self.complete(bogus, JavaSyntaxKind::Bogus)
        };
        let base_has_unsized_dimensions =
            JavaSyntaxKind::from_raw(ty.kind()) == JavaSyntaxKind::ArrayType;

        let dimensions = self.start();
        let mut saw_dim_expression = false;
        let mut guard = self.loop_guard();
        while self.starts_dim_expression() && self.check(&mut guard) {
            if base_has_unsized_dimensions {
                self.error_unexpected("sized array dimension cannot follow unsized dimensions");
            }
            self.parse_dim_expression();
            saw_dim_expression = true;
        }
        self.complete(dimensions, JavaSyntaxKind::DimExpressionList);

        self.parse_array_dimensions();

        if self.at(JavaSyntaxKind::LBrace) {
            if saw_dim_expression {
                self.error_unexpected("array initializer cannot follow dimension expressions");
            }
            self.parse_array_initializer_fragment();
        } else if base_has_unsized_dimensions && !saw_dim_expression {
            self.error_expected("expected array initializer or dimension expression");
        }

        self.complete(creation, JavaSyntaxKind::ArrayCreationExpression)
    }

    pub(super) fn parse_dim_expression(&mut self) {
        let dim = self.start();
        self.parse_annotations();
        self.expect(JavaSyntaxKind::LBracket, "expected `[`");
        self.parse_expression_until(&[JavaSyntaxKind::RBracket]);
        self.expect(JavaSyntaxKind::RBracket, "expected `]`");
        self.complete(dim, JavaSyntaxKind::DimExpression);
    }

    pub(super) fn parse_array_initializer_fragment(&mut self) {
        let initializer = self.start();
        self.expect(JavaSyntaxKind::LBrace, "expected array initializer");
        let values = self.start();
        let mut guard = self.loop_guard();
        while !self.at_expression_list_close(JavaSyntaxKind::RBrace) && self.check(&mut guard) {
            if self.at(JavaSyntaxKind::LBrace) {
                if self
                    .nested(Self::parse_array_initializer_fragment)
                    .is_none()
                {
                    self.recover_too_deep(EXPRESSION_STOPS);
                }
            } else {
                self.parse_expression_until(&[JavaSyntaxKind::Comma, JavaSyntaxKind::RBrace]);
            }

            self.eat(JavaSyntaxKind::Comma);
        }
        self.complete(values, JavaSyntaxKind::VariableInitializerList);
        self.expect(
            JavaSyntaxKind::RBrace,
            "expected `}` after array initializer",
        );
        self.complete(initializer, JavaSyntaxKind::ArrayInitializer);
    }

    pub(super) fn parse_argument_list(&mut self) {
        let arguments = self.start();
        self.expect(JavaSyntaxKind::LParen, "expected argument list");
        let expressions = self.start();
        self.parse_comma_separated(
            "expected argument",
            |parser| parser.at_expression_list_close(JavaSyntaxKind::RParen),
            |parser| {
                parser.parse_expression_until(&[JavaSyntaxKind::Comma, JavaSyntaxKind::RParen]);
            },
        );
        self.complete(expressions, JavaSyntaxKind::ExpressionList);
        self.expect(JavaSyntaxKind::RParen, "expected `)` after arguments");
        self.complete(arguments, JavaSyntaxKind::ArgumentList);
    }

    pub(super) fn parse_method_reference_suffix(&mut self) {
        self.expect(JavaSyntaxKind::DoubleColon, "expected `::`");
        self.parse_optional_type_argument_list();
        if self.at(JavaSyntaxKind::NewKw) || self.at_name_segment() {
            self.bump();
        } else {
            self.error_expected("expected method reference target");
        }
    }

    pub(super) fn parse_literal_expression(
        &mut self,
        allow_decimal_boundary_literal: bool,
    ) -> jolt_syntax::CompletedMarker {
        let literal = self.start();
        if !allow_decimal_boundary_literal && self.at_decimal_integer_boundary_literal() {
            self.error(
                codes::DECIMAL_INTEGER_BOUNDARY_LITERAL,
                "decimal integer boundary literal may appear only as the operand of unary minus",
            );
        }
        self.bump();
        self.complete(literal, JavaSyntaxKind::LiteralExpression)
    }

    pub(super) fn at_decimal_integer_boundary_literal(&mut self) -> bool {
        if self.current() != JavaSyntaxKind::IntegerLiteral {
            return false;
        }

        let normalized = self.current_text().replace('_', "");

        normalized == "2147483648"
            || matches!(
                normalized.strip_suffix(['l', 'L']),
                Some("9223372036854775808")
            )
    }
}
