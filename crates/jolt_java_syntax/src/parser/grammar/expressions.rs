impl Parser<'_> {
    fn consume_shallow_expression_until(&mut self, stops: &[JavaSyntaxKind]) {
        self.parse_expression_until(stops);
    }

    fn consume_statement_expression_until(&mut self, stops: &[JavaSyntaxKind]) {
        if self.at_eof() || stops.contains(&self.current_kind()) {
            let error = self.start();
            self.error_here("expected statement expression");
            self.complete(error, JavaSyntaxKind::ErrorNode);
            return;
        }

        let start_kind = self.current_kind();
        let expression = self.parse_expression();
        if !Self::is_statement_expression(expression.kind(), start_kind) {
            self.error_here("expected statement expression");
        }

        while !self.at_eof() && !stops.contains(&self.current_kind()) {
            let error = self.start();
            self.error_here("unexpected token in statement expression");
            self.bump();
            self.complete(error, JavaSyntaxKind::ErrorNode);
        }
    }

    fn is_statement_expression(
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

    fn parse_expression_until(&mut self, stops: &[JavaSyntaxKind]) {
        if self.at_eof() || stops.contains(&self.current_kind()) {
            let error = self.start();
            self.error_here("expected expression");
            self.complete(error, JavaSyntaxKind::ErrorNode);
            return;
        }

        self.parse_expression();

        while !self.at_eof() && !stops.contains(&self.current_kind()) {
            let error = self.start();
            self.error_here("unexpected token in expression");
            self.bump();
            self.complete(error, JavaSyntaxKind::ErrorNode);
        }
    }

    fn parse_expression(&mut self) -> jolt_syntax::CompletedMarker {
        self.parse_assignment_expression()
    }

    fn parse_assignment_expression(&mut self) -> jolt_syntax::CompletedMarker {
        if self.starts_lambda_expression() {
            return self.parse_lambda_expression();
        }

        let lhs = self.parse_conditional_expression();
        if !self.at_assignment_operator() {
            return lhs;
        }

        let assignment = self.precede(lhs);
        self.bump();
        self.parse_assignment_expression();
        self.complete(assignment, JavaSyntaxKind::AssignmentExpression)
    }

    fn parse_conditional_expression(&mut self) -> jolt_syntax::CompletedMarker {
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

    fn parse_binary_expression(&mut self, minimum_precedence: u8) -> jolt_syntax::CompletedMarker {
        let mut lhs = self.parse_unary_expression();

        while let Some(precedence) = self.binary_operator_precedence() {
            if precedence < minimum_precedence {
                break;
            }

            let binary = self.precede(lhs);
            let operator = self.current_kind();
            self.bump();

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
                    self.parse_type();
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

    fn parse_unary_expression(&mut self) -> jolt_syntax::CompletedMarker {
        self.parse_unary_expression_with_decimal_boundary_literal(false)
    }

    fn parse_unary_expression_with_decimal_boundary_literal(
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

    fn parse_cast_expression(&mut self) -> jolt_syntax::CompletedMarker {
        let cast = self.start();
        self.expect(JavaSyntaxKind::LParen, "expected `(` in cast expression");
        let type_start = self.skip_annotations_from(self.position());
        let is_primitive_cast = self.is_primitive_type_start_at(type_start);
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

    fn parse_postfix_expression(
        &mut self,
        allow_decimal_boundary_literal: bool,
    ) -> jolt_syntax::CompletedMarker {
        let mut expression = self.parse_primary_expression(allow_decimal_boundary_literal);

        loop {
            match self.current_kind() {
                JavaSyntaxKind::Lt if self.type_arguments_are_followed_by_double_colon() => {
                    self.parse_optional_type_argument_list();
                }
                JavaSyntaxKind::LParen => {
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

    fn parse_dot_suffix(
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

        self.parse_optional_type_argument_list();
        self.expect_method_identifier("expected member name");
        self.parse_optional_type_argument_list();
        if self.at(JavaSyntaxKind::LParen) {
            self.parse_argument_list();
            self.complete(suffix, JavaSyntaxKind::MethodInvocationExpression)
        } else {
            self.complete(suffix, JavaSyntaxKind::FieldAccessExpression)
        }
    }

    fn parse_primary_expression(
        &mut self,
        allow_decimal_boundary_literal: bool,
    ) -> jolt_syntax::CompletedMarker {
        if self.at_contextual("yield") && self.nth_kind(1) == JavaSyntaxKind::LParen {
            let error = self.start();
            self.error_here("unqualified `yield` method invocation is not allowed");
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

        if self.starts_primitive_array_method_reference_type() {
            return self.parse_primitive_array_method_reference_type();
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

            self.error_here("expected expression after annotation");
            return self.complete(annotated, JavaSyntaxKind::ErrorNode);
        }

        if self.at_name_segment() {
            let name = self.start();
            self.bump();
            return self.complete(name, JavaSyntaxKind::NameExpression);
        }

        let error = self.start();
        self.error_here("expected expression");
        if !self.at_eof() {
            self.bump();
        }
        self.complete(error, JavaSyntaxKind::ErrorNode)
    }

    fn starts_primitive_array_method_reference_type(&self) -> bool {
        if !self.at_primitive_type() {
            return false;
        }

        let mut index = self.position() + 1;
        let mut saw_array_dimensions = false;
        while self.kind_at(index) == JavaSyntaxKind::LBracket
            && self.kind_at(index + 1) == JavaSyntaxKind::RBracket
        {
            saw_array_dimensions = true;
            index += 2;
        }

        saw_array_dimensions && self.kind_at(index) == JavaSyntaxKind::DoubleColon
    }

    fn parse_primitive_array_method_reference_type(&mut self) -> jolt_syntax::CompletedMarker {
        let primitive = self.start();
        self.bump();
        let completed = self.complete(primitive, JavaSyntaxKind::PrimitiveType);
        let array = self.precede(completed);
        self.parse_array_dimensions();
        self.complete(array, JavaSyntaxKind::ArrayType)
    }

    fn parse_lambda_expression(&mut self) -> jolt_syntax::CompletedMarker {
        if self.at(JavaSyntaxKind::LParen) {
            self.parse_parenthesized_lambda_expression_fragment()
        } else {
            self.parse_unparenthesized_lambda_expression()
        }
    }

    fn parse_unparenthesized_lambda_expression(&mut self) -> jolt_syntax::CompletedMarker {
        let lambda = self.start();
        self.parse_lambda_parameter();
        self.expect(
            JavaSyntaxKind::Arrow,
            "expected `->` after lambda parameter",
        );
        self.parse_lambda_body();
        self.complete(lambda, JavaSyntaxKind::LambdaExpression)
    }

    fn parse_parenthesized_lambda_expression_fragment(&mut self) -> jolt_syntax::CompletedMarker {
        let lambda = self.start();
        self.expect(JavaSyntaxKind::LParen, "expected lambda parameter list");
        let list = self.start();
        while !self.at_eof() && !self.at(JavaSyntaxKind::RParen) {
            self.parse_lambda_parameter();
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

    fn parse_lambda_body(&mut self) {
        if self.at(JavaSyntaxKind::LBrace) {
            self.parse_block();
        } else {
            self.parse_assignment_expression();
        }
    }

    fn parse_lambda_parameter(&mut self) {
        let parameter = self.start();
        self.parse_variable_modifiers();
        if self.starts_typed_lambda_parameter() {
            self.parse_local_variable_type();
            self.parse_annotations();
            self.eat(JavaSyntaxKind::Ellipsis);
            self.expect_variable_identifier("expected lambda parameter name");
            self.parse_array_dimensions();
        } else {
            self.expect_variable_identifier("expected lambda parameter name");
        }
        self.complete(parameter, JavaSyntaxKind::LambdaParameter);
    }

    fn parse_new_expression_fragment(&mut self) -> jolt_syntax::CompletedMarker {
        if self.new_expression_is_array_creation() {
            self.parse_array_creation_expression_fragment()
        } else {
            self.parse_object_creation_expression_fragment()
        }
    }

    fn parse_object_creation_expression_fragment(&mut self) -> jolt_syntax::CompletedMarker {
        let creation = self.start();
        self.parse_object_creation_after_new();
        self.complete(creation, JavaSyntaxKind::ObjectCreationExpression)
    }

    fn parse_object_creation_after_new(&mut self) {
        self.expect(JavaSyntaxKind::NewKw, "expected `new`");
        self.parse_optional_type_argument_list();
        self.parse_type();
        if self.at(JavaSyntaxKind::LParen) {
            self.parse_argument_list();
        }
        if self.at(JavaSyntaxKind::LBrace) {
            self.parse_type_body(JavaSyntaxKind::ClassBody, None);
        }
    }

    fn parse_array_creation_expression_fragment(&mut self) -> jolt_syntax::CompletedMarker {
        let creation = self.start();
        self.expect(JavaSyntaxKind::NewKw, "expected `new`");
        self.parse_type();

        while self.starts_dim_expression() {
            self.parse_dim_expression();
        }

        self.parse_array_dimensions();

        if self.at(JavaSyntaxKind::LBrace) {
            self.parse_array_initializer_fragment();
        }

        self.complete(creation, JavaSyntaxKind::ArrayCreationExpression)
    }

    fn parse_dim_expression(&mut self) {
        let dim = self.start();
        self.parse_annotations();
        self.expect(JavaSyntaxKind::LBracket, "expected `[`");
        self.consume_shallow_expression_until(&[JavaSyntaxKind::RBracket]);
        self.expect(JavaSyntaxKind::RBracket, "expected `]`");
        self.complete(dim, JavaSyntaxKind::DimExpression);
    }

    fn parse_array_initializer_fragment(&mut self) {
        let initializer = self.start();
        self.expect(JavaSyntaxKind::LBrace, "expected array initializer");
        while !self.at_eof() && !self.at(JavaSyntaxKind::RBrace) {
            if self.at(JavaSyntaxKind::LBrace) {
                self.parse_array_initializer_fragment();
            } else {
                self.consume_shallow_expression_until(&[
                    JavaSyntaxKind::Comma,
                    JavaSyntaxKind::RBrace,
                ]);
            }

            self.eat(JavaSyntaxKind::Comma);
        }
        self.expect(
            JavaSyntaxKind::RBrace,
            "expected `}` after array initializer",
        );
        self.complete(initializer, JavaSyntaxKind::ArrayInitializer);
    }

    fn parse_argument_list(&mut self) {
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

    fn parse_method_reference_suffix(&mut self) {
        self.expect(JavaSyntaxKind::DoubleColon, "expected `::`");
        self.parse_optional_type_argument_list();
        if self.at(JavaSyntaxKind::NewKw) {
            self.bump();
        } else {
            self.expect_method_identifier("expected method reference target");
        }
    }

    fn parse_literal_expression(
        &mut self,
        allow_decimal_boundary_literal: bool,
    ) -> jolt_syntax::CompletedMarker {
        let literal = self.start();
        if !allow_decimal_boundary_literal && self.at_decimal_integer_boundary_literal() {
            self.error_here(
                "decimal integer boundary literal may appear only as the operand of unary minus",
            );
        }
        self.bump();
        self.complete(literal, JavaSyntaxKind::LiteralExpression)
    }

    fn at_decimal_integer_boundary_literal(&self) -> bool {
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
