use super::{JavaSyntaxKind, Parser, StopSet};

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
    pub(super) fn consume_statement_expression_until(&mut self, stops: &[JavaSyntaxKind]) {
        if self.at_eof() || stops.contains(&self.current_kind()) {
            let error = self.start();
            self.invalid_statement_expression_here("expected statement expression");
            self.complete(error, JavaSyntaxKind::ErrorNode);
            return;
        }

        let start_kind = self.current_kind();
        let expression = self.parse_expression();
        if !Self::is_statement_expression(expression.kind(), start_kind)
            && !Self::completed_is_error_node(&expression)
        {
            let error = self.precede(expression);
            self.invalid_statement_expression_here("expected statement expression");
            self.complete(error, JavaSyntaxKind::ErrorNode);
        }

        while !self.at_eof() && !stops.contains(&self.current_kind()) {
            let error = self.start();
            self.invalid_statement_expression_here("unexpected token in statement expression");
            self.bump();
            self.complete(error, JavaSyntaxKind::ErrorNode);
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
            let error = self.start();
            self.expected_here("expected expression");
            self.complete(error, JavaSyntaxKind::ErrorNode);
            return;
        }

        self.parse_expression();

        while !self.at_eof() && !stops.contains(self.current_kind()) {
            let error = self.start();
            self.unexpected_here("unexpected token in expression");
            self.bump();
            self.complete(error, JavaSyntaxKind::ErrorNode);
        }
    }

    pub(super) fn parse_expression_until_without_leading_lambda<'a>(
        &mut self,
        stops: impl Into<StopSet<'a>>,
    ) {
        let stops = stops.into();
        if self.at_eof() || stops.contains(self.current_kind()) {
            let error = self.start();
            self.expected_here("expected expression");
            self.complete(error, JavaSyntaxKind::ErrorNode);
            return;
        }

        self.parse_assignment_expression_without_leading_lambda();

        while !self.at_eof() && !stops.contains(self.current_kind()) {
            let error = self.start();
            self.unexpected_here("unexpected token in expression");
            self.bump();
            self.complete(error, JavaSyntaxKind::ErrorNode);
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
            || Self::completed_is_error_node(&lhs)
        {
            lhs
        } else {
            let error = self.precede(lhs);
            self.expected_here("expected assignment left-hand side");
            self.complete(error, JavaSyntaxKind::ErrorNode)
        };

        let assignment = self.precede(lhs);
        for _ in 0..operator_len {
            self.bump();
        }
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
            || Self::completed_is_error_node(&lhs)
        {
            lhs
        } else {
            let error = self.precede(lhs);
            self.expected_here("expected assignment left-hand side");
            self.complete(error, JavaSyntaxKind::ErrorNode)
        };

        let assignment = self.precede(lhs);
        for _ in 0..operator_len {
            self.bump();
        }
        self.parse_assignment_expression();
        self.complete(assignment, JavaSyntaxKind::AssignmentExpression)
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

        while let Some(operator_info) = self.binary_operator() {
            if operator_info.precedence < minimum_precedence {
                break;
            }

            let binary = self.precede(lhs);
            let operator = self.current_kind();
            for _ in 0..operator_info.len {
                self.bump();
            }

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
            self.parse_unary_expression_with_decimal_boundary_literal(
                operator == JavaSyntaxKind::Minus,
            );
            return self.complete(unary, JavaSyntaxKind::UnaryExpression);
        }

        if self.starts_cast_expression() {
            return self.parse_cast_expression();
        }

        self.parse_postfix_expression(allow_decimal_boundary_literal)
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

        loop {
            match self.current_kind() {
                JavaSyntaxKind::Lt if self.type_arguments_are_followed_by_double_colon() => {
                    self.parse_optional_type_argument_list();
                }
                JavaSyntaxKind::LParen if Self::can_call_with_argument_list(expression.kind()) => {
                    let invocation = self.precede(expression);
                    self.parse_argument_list();
                    expression =
                        self.complete(invocation, JavaSyntaxKind::MethodInvocationExpression);
                }
                JavaSyntaxKind::LBracket if self.nth_kind(1) == JavaSyntaxKind::RBracket => {
                    self.parse_array_dimensions();
                }
                JavaSyntaxKind::LBracket => {
                    let access = self.precede(expression);
                    self.bump();
                    self.parse_expression_until(&[JavaSyntaxKind::RBracket]);
                    self.expect(JavaSyntaxKind::RBracket, "expected `]` after array index");
                    expression = self.complete(access, JavaSyntaxKind::ArrayAccessExpression);
                }
                JavaSyntaxKind::Dot => {
                    expression = self.parse_dot_suffix(expression);
                }
                JavaSyntaxKind::DoubleColon => {
                    let reference = self.precede(expression);
                    self.parse_method_reference_suffix();
                    expression =
                        self.complete(reference, JavaSyntaxKind::MethodReferenceExpression);
                }
                JavaSyntaxKind::PlusPlus | JavaSyntaxKind::MinusMinus => {
                    let postfix = self.precede(expression);
                    self.bump();
                    expression = self.complete(postfix, JavaSyntaxKind::PostfixExpression);
                }
                _ => break,
            }
        }

        expression
    }

    pub(super) fn can_call_with_argument_list(kind: jolt_syntax::RawSyntaxKind) -> bool {
        matches!(
            JavaSyntaxKind::from_raw(kind),
            Some(JavaSyntaxKind::NameExpression | JavaSyntaxKind::FieldAccessExpression)
        )
    }

    pub(super) fn parse_dot_suffix(
        &mut self,
        expression: jolt_syntax::CompletedMarker,
    ) -> jolt_syntax::CompletedMarker {
        let suffix = self.precede(expression);
        self.expect(JavaSyntaxKind::Dot, "expected `.`");

        if self.eat(JavaSyntaxKind::ClassKw) {
            return self.complete(suffix, JavaSyntaxKind::ClassLiteralExpression);
        }

        if self.at(JavaSyntaxKind::NewKw) {
            self.parse_object_creation_after_new();
            return self.complete(suffix, JavaSyntaxKind::ObjectCreationExpression);
        }

        if self.eat(JavaSyntaxKind::ThisKw) {
            return self.complete(suffix, JavaSyntaxKind::ThisExpression);
        }

        if self.eat(JavaSyntaxKind::SuperKw) {
            return self.complete(suffix, JavaSyntaxKind::SuperExpression);
        }

        // Java string templates were preview syntax in JDK 21/22 and withdrawn
        // for JDK 23. Keep this parser shape for legacy preview sources only.
        if matches!(
            self.current_kind(),
            JavaSyntaxKind::StringLiteral | JavaSyntaxKind::TextBlockLiteral
        ) {
            self.parse_literal_expression(false);
            return self.complete(suffix, JavaSyntaxKind::TemplateExpression);
        }

        self.parse_optional_type_argument_list();
        self.expect_method_identifier("expected member name");
        if self.at(JavaSyntaxKind::Lt) && self.type_arguments_are_followed_by_double_colon() {
            self.parse_optional_type_argument_list();
        }
        if self.at(JavaSyntaxKind::LParen) {
            self.parse_argument_list();
            self.complete(suffix, JavaSyntaxKind::MethodInvocationExpression)
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
            self.unqualified_yield_method_invocation_here(
                "unqualified `yield` method invocation is not allowed",
            );
            self.bump();
            self.parse_argument_list();
            return self.complete(error, JavaSyntaxKind::ErrorNode);
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
            if !self.at(JavaSyntaxKind::RParen) {
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
                self.bump();
                self.parse_array_dimensions();
            }
            self.expect(JavaSyntaxKind::Dot, "expected `.` in class literal");
            self.expect(JavaSyntaxKind::ClassKw, "expected `class` in class literal");
            return self.complete(literal, JavaSyntaxKind::ClassLiteralExpression);
        }

        if self.at(JavaSyntaxKind::At) && self.nth_kind(1) != JavaSyntaxKind::InterfaceKw {
            let annotated = self.start();
            self.parse_annotations();
            if self.at_name_segment() {
                self.bump();
                return self.complete(annotated, JavaSyntaxKind::NameExpression);
            }

            self.expected_here("expected expression after annotation");
            return self.complete(annotated, JavaSyntaxKind::ErrorNode);
        }

        if self.at_name_segment() {
            let name = self.start();
            self.bump();
            return self.complete(name, JavaSyntaxKind::NameExpression);
        }

        let error = self.start();
        self.expected_here("expected expression");
        if !self.at_eof() {
            self.bump();
        }
        self.complete(error, JavaSyntaxKind::ErrorNode)
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
        self.parse_lambda_parameter();
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
        while !self.at_eof() && !self.at(JavaSyntaxKind::RParen) {
            let parameter = self.parse_lambda_parameter();
            if let Some(expected) = style {
                if parameter.style != expected {
                    let error = self.start();
                    self.expected_here("lambda parameters must use the same form");
                    self.complete(error, JavaSyntaxKind::ErrorNode);
                }
            } else {
                style = Some(parameter.style);
            }

            if parameter.varargs && !self.at(JavaSyntaxKind::RParen) && !self.at_eof() {
                let error = self.start();
                self.expected_here("varargs lambda parameter must be last");
                let consumed_comma = self.eat(JavaSyntaxKind::Comma);
                self.complete(error, JavaSyntaxKind::ErrorNode);
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

    pub(super) fn parse_lambda_parameter(&mut self) -> ParsedLambdaParameter {
        let parameter = self.start();
        let has_modifiers = self.parse_variable_modifiers();
        let starts_typed_parameter = self.starts_typed_lambda_parameter();
        let style = if has_modifiers && !starts_typed_parameter {
            LambdaParameterStyle::Explicit
        } else {
            self.current_lambda_parameter_style()
        };
        let mut varargs = false;
        if starts_typed_parameter {
            self.parse_local_variable_type();
            self.parse_annotations();
            varargs = self.eat(JavaSyntaxKind::Ellipsis);
            self.expect_variable_identifier("expected lambda parameter name");
            self.parse_array_dimensions();
        } else if has_modifiers {
            let error = self.start();
            self.expected_here("expected lambda parameter type after modifiers");
            self.expect_variable_identifier("expected lambda parameter name");
            self.complete(error, JavaSyntaxKind::ErrorNode);
        } else {
            self.expect_variable_identifier("expected lambda parameter name");
        }
        self.complete(parameter, JavaSyntaxKind::LambdaParameter);
        ParsedLambdaParameter { style, varargs }
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
        self.parse_object_creation_after_new();
        self.complete(creation, JavaSyntaxKind::ObjectCreationExpression)
    }

    pub(super) fn parse_object_creation_after_new(&mut self) {
        self.expect(JavaSyntaxKind::NewKw, "expected `new`");
        self.parse_optional_type_argument_list();
        self.parse_object_creation_type();
        if self.at(JavaSyntaxKind::LParen) {
            self.parse_argument_list();
        } else {
            let error = self.start();
            self.expected_here("expected constructor arguments");
            self.complete(error, JavaSyntaxKind::ErrorNode);
        }
        if self.at(JavaSyntaxKind::LBrace) {
            self.parse_type_body(JavaSyntaxKind::ClassBody, None);
        }
    }

    pub(super) fn parse_object_creation_type(&mut self) -> jolt_syntax::CompletedMarker {
        let ty = self.start();
        self.parse_annotations();

        if self.at_name_segment() {
            self.parse_class_type_to_instantiate_tail();
            return self.complete(ty, JavaSyntaxKind::ClassType);
        }

        if self.at_primitive_type() {
            self.expected_here("expected class type in object creation");
            self.bump();
        } else {
            self.expected_here("expected class type in object creation");
        }

        self.complete(ty, JavaSyntaxKind::ErrorNode)
    }

    pub(super) fn parse_class_type_to_instantiate_tail(&mut self) {
        let name = self.start();
        self.bump();
        let mut qualified = false;

        loop {
            if self.at(JavaSyntaxKind::Lt) && self.type_arguments_are_followed_by_dot() {
                let error = self.start();
                self.unexpected_here(
                    "type arguments in class instance creation must appear on the final type segment",
                );
                self.parse_optional_type_argument_list();
                self.complete(error, JavaSyntaxKind::ErrorNode);
            }

            if !self.at(JavaSyntaxKind::Dot) {
                break;
            }

            if !self.dot_is_followed_by_annotated_name() {
                break;
            }

            qualified = true;
            self.bump();
            self.parse_annotations();
            self.bump();
        }

        self.complete(
            name,
            if qualified {
                JavaSyntaxKind::QualifiedName
            } else {
                JavaSyntaxKind::Name
            },
        );
        self.parse_optional_type_argument_list();
    }

    pub(super) fn parse_array_creation_expression_fragment(
        &mut self,
    ) -> jolt_syntax::CompletedMarker {
        let creation = self.start();
        self.expect(JavaSyntaxKind::NewKw, "expected `new`");
        let ty = self.parse_type();
        let base_has_unsized_dimensions =
            JavaSyntaxKind::from_raw(ty.kind()) == Some(JavaSyntaxKind::ArrayType);

        let mut saw_dim_expression = false;
        while self.starts_dim_expression() {
            if base_has_unsized_dimensions {
                let error = self.start();
                self.unexpected_here("sized array dimension cannot follow unsized dimensions");
                self.parse_dim_expression();
                self.complete(error, JavaSyntaxKind::ErrorNode);
            } else {
                self.parse_dim_expression();
            }
            saw_dim_expression = true;
        }

        self.parse_array_dimensions();

        if self.at(JavaSyntaxKind::LBrace) {
            if saw_dim_expression {
                let error = self.start();
                self.unexpected_here("array initializer cannot follow dimension expressions");
                self.parse_array_initializer_fragment();
                self.complete(error, JavaSyntaxKind::ErrorNode);
            } else {
                self.parse_array_initializer_fragment();
            }
        } else if base_has_unsized_dimensions && !saw_dim_expression {
            let error = self.start();
            self.expected_here("expected array initializer or dimension expression");
            self.complete(error, JavaSyntaxKind::ErrorNode);
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
        while !self.at_eof() && !self.at(JavaSyntaxKind::RBrace) {
            if self.at(JavaSyntaxKind::LBrace) {
                self.parse_array_initializer_fragment();
            } else {
                self.parse_expression_until(&[JavaSyntaxKind::Comma, JavaSyntaxKind::RBrace]);
            }

            self.eat(JavaSyntaxKind::Comma);
        }
        self.expect(
            JavaSyntaxKind::RBrace,
            "expected `}` after array initializer",
        );
        self.complete(initializer, JavaSyntaxKind::ArrayInitializer);
    }

    pub(super) fn parse_argument_list(&mut self) {
        let arguments = self.start();
        self.expect(JavaSyntaxKind::LParen, "expected argument list");
        while !self.at_eof() && !self.at(JavaSyntaxKind::RParen) {
            self.parse_expression_until(&[JavaSyntaxKind::Comma, JavaSyntaxKind::RParen]);
            if !self.eat(JavaSyntaxKind::Comma) {
                break;
            }
        }
        self.expect(JavaSyntaxKind::RParen, "expected `)` after arguments");
        self.complete(arguments, JavaSyntaxKind::ArgumentList);
    }

    pub(super) fn parse_method_reference_suffix(&mut self) {
        self.expect(JavaSyntaxKind::DoubleColon, "expected `::`");
        self.parse_optional_type_argument_list();
        if self.at(JavaSyntaxKind::NewKw) {
            self.bump();
        } else {
            self.expect_method_identifier("expected method reference target");
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
