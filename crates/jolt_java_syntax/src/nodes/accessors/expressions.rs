use super::super::{
    ArgumentList, ArrayCreationExpression, ArrayInitializer, AssignmentExpression,
    BinaryExpression, Block, CastExpression, ClassBody, ConditionalExpression, DimExpression,
    Expression, FieldAccessExpression, JavaSyntaxKind, JavaSyntaxToken, LambdaExpression,
    LambdaParameter, LambdaParameterList, LiteralExpression, MethodInvocationExpression,
    NameExpression, ObjectCreationExpression, ParenthesizedExpression, PostfixExpression,
    SuperExpression, SwitchBlock, SwitchExpression, ThisExpression, Type, UnaryExpression, child,
    child_family, child_token, child_token_in, children, children_family, nth_child_family,
};
use super::helpers::{
    ASSIGNMENT_OPERATORS, BINARY_OPERATORS, is_literal_token, simple_keyword_token,
};

impl MethodInvocationExpression {
    #[must_use]
    pub fn receiver(&self) -> Option<Expression> {
        let elements = self.syntax.children_with_tokens().collect::<Vec<_>>();
        let [receiver, dot, _, _] = elements.as_slice() else {
            return None;
        };
        if !Expression::can_cast(receiver.kind()) || dot.kind() != JavaSyntaxKind::Dot {
            return None;
        }
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn name(&self) -> Option<JavaSyntaxToken> {
        self.syntax
            .children_with_tokens()
            .filter_map(jolt_syntax::SyntaxElement::into_token)
            .find(|token| token.kind() == JavaSyntaxKind::Identifier)
            .map(|syntax| JavaSyntaxToken { syntax })
    }

    #[must_use]
    pub fn simple_name(&self) -> Option<JavaSyntaxToken> {
        let elements = self.syntax.children_with_tokens().collect::<Vec<_>>();
        let [callee, arguments] = elements.as_slice() else {
            return None;
        };
        if callee.kind() != JavaSyntaxKind::NameExpression
            || arguments.kind() != JavaSyntaxKind::ArgumentList
        {
            return None;
        }
        child::<NameExpression>(&self.syntax)?.identifier()
    }

    #[must_use]
    pub fn arguments(&self) -> Option<ArgumentList> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        let elements = self.syntax.children_with_tokens().collect::<Vec<_>>();
        match elements.as_slice() {
            [callee, arguments] => {
                callee.kind() == JavaSyntaxKind::NameExpression
                    && arguments.kind() == JavaSyntaxKind::ArgumentList
            }
            [receiver, dot, name, arguments] => {
                Expression::can_cast(receiver.kind())
                    && dot.kind() == JavaSyntaxKind::Dot
                    && name.kind() == JavaSyntaxKind::Identifier
                    && arguments.kind() == JavaSyntaxKind::ArgumentList
            }
            _ => false,
        }
    }
}

impl ArgumentList {
    pub fn arguments(&self) -> impl Iterator<Item = Expression> + '_ {
        children_family(&self.syntax)
    }

    #[must_use]
    pub fn has_empty_layout_shape(&self) -> bool {
        self.syntax
            .children_with_tokens()
            .map(|element| element.kind())
            .eq([JavaSyntaxKind::LParen, JavaSyntaxKind::RParen])
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        let elements = self.syntax.children_with_tokens().collect::<Vec<_>>();
        let Some(first) = elements.first() else {
            return false;
        };
        let Some(last) = elements.last() else {
            return false;
        };
        if first.kind() != JavaSyntaxKind::LParen || last.kind() != JavaSyntaxKind::RParen {
            return false;
        }

        let inner = &elements[1..elements.len().saturating_sub(1)];
        if inner.is_empty() {
            return true;
        }
        if inner.len() % 2 == 0 {
            return false;
        }
        inner.iter().enumerate().all(|(index, element)| {
            if index % 2 == 0 {
                Expression::can_cast(element.kind())
            } else {
                element.kind() == JavaSyntaxKind::Comma
            }
        })
    }
}

impl LiteralExpression {
    #[must_use]
    pub fn token(&self) -> Option<JavaSyntaxToken> {
        let tokens = self
            .syntax
            .children_with_tokens()
            .map(jolt_syntax::SyntaxElement::into_token)
            .collect::<Option<Vec<_>>>()?;
        let [token] = tokens.as_slice() else {
            return None;
        };
        is_literal_token(token.kind()).then(|| JavaSyntaxToken {
            syntax: token.clone(),
        })
    }

    #[must_use]
    pub fn has_simple_layout_shape(&self) -> bool {
        self.token().is_some()
    }
}

impl NameExpression {
    #[must_use]
    pub fn identifier(&self) -> Option<JavaSyntaxToken> {
        let tokens = self
            .syntax
            .children_with_tokens()
            .map(jolt_syntax::SyntaxElement::into_token)
            .collect::<Option<Vec<_>>>()?;
        let [token] = tokens.as_slice() else {
            return None;
        };
        (token.kind() == JavaSyntaxKind::Identifier).then(|| JavaSyntaxToken {
            syntax: token.clone(),
        })
    }

    #[must_use]
    pub fn has_simple_layout_shape(&self) -> bool {
        self.identifier().is_some()
    }
}

impl ThisExpression {
    #[must_use]
    pub fn token(&self) -> Option<JavaSyntaxToken> {
        simple_keyword_token(&self.syntax, JavaSyntaxKind::ThisKw)
    }

    #[must_use]
    pub fn has_simple_layout_shape(&self) -> bool {
        self.token().is_some()
    }
}

impl SuperExpression {
    #[must_use]
    pub fn token(&self) -> Option<JavaSyntaxToken> {
        simple_keyword_token(&self.syntax, JavaSyntaxKind::SuperKw)
    }

    #[must_use]
    pub fn has_simple_layout_shape(&self) -> bool {
        self.token().is_some()
    }
}

impl FieldAccessExpression {
    #[must_use]
    pub fn receiver(&self) -> Option<Expression> {
        nth_child_family(&self.syntax, 0)
    }

    #[must_use]
    pub fn name(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::Identifier)
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        let elements = self.syntax.children_with_tokens().collect::<Vec<_>>();
        matches!(
            elements.as_slice(),
            [receiver, dot, name]
                if Expression::can_cast(receiver.kind())
                    && dot.kind() == JavaSyntaxKind::Dot
                    && name.kind() == JavaSyntaxKind::Identifier
        )
    }
}
impl ParenthesizedExpression {
    #[must_use]
    pub fn expression(&self) -> Option<Expression> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        let elements = self.syntax.children_with_tokens().collect::<Vec<_>>();
        matches!(
            elements.as_slice(),
            [left, expression, right]
                if left.kind() == JavaSyntaxKind::LParen
                    && Expression::can_cast(expression.kind())
                    && right.kind() == JavaSyntaxKind::RParen
        )
    }
}

impl AssignmentExpression {
    #[must_use]
    pub fn left(&self) -> Option<Expression> {
        nth_child_family(&self.syntax, 0)
    }

    #[must_use]
    pub fn operator(&self) -> Option<JavaSyntaxToken> {
        child_token_in(&self.syntax, ASSIGNMENT_OPERATORS)
    }

    #[must_use]
    pub fn right(&self) -> Option<Expression> {
        nth_child_family(&self.syntax, 1)
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        let elements = self.syntax.children_with_tokens().collect::<Vec<_>>();
        matches!(
            elements.as_slice(),
            [left, operator, right]
                if Expression::can_cast(left.kind())
                    && ASSIGNMENT_OPERATORS.contains(&operator.kind())
                    && Expression::can_cast(right.kind())
        )
    }
}

impl ConditionalExpression {
    #[must_use]
    pub fn condition(&self) -> Option<Expression> {
        nth_child_family(&self.syntax, 0)
    }

    #[must_use]
    pub fn true_expression(&self) -> Option<Expression> {
        nth_child_family(&self.syntax, 1)
    }

    #[must_use]
    pub fn false_expression(&self) -> Option<Expression> {
        nth_child_family(&self.syntax, 2)
    }
}

impl BinaryExpression {
    #[must_use]
    pub fn left(&self) -> Option<Expression> {
        nth_child_family(&self.syntax, 0)
    }

    #[must_use]
    pub fn operator(&self) -> Option<JavaSyntaxToken> {
        child_token_in(&self.syntax, BINARY_OPERATORS)
    }

    #[must_use]
    pub fn right(&self) -> Option<Expression> {
        nth_child_family(&self.syntax, 1)
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        let elements = self.syntax.children_with_tokens().collect::<Vec<_>>();
        matches!(
            elements.as_slice(),
            [left, operator, right]
                if Expression::can_cast(left.kind())
                    && BINARY_OPERATORS.contains(&operator.kind())
                    && Expression::can_cast(right.kind())
        )
    }
}

impl UnaryExpression {
    #[must_use]
    pub fn operator(&self) -> Option<JavaSyntaxToken> {
        child_token_in(
            &self.syntax,
            &[
                JavaSyntaxKind::PlusPlus,
                JavaSyntaxKind::MinusMinus,
                JavaSyntaxKind::Plus,
                JavaSyntaxKind::Minus,
                JavaSyntaxKind::Bang,
                JavaSyntaxKind::Tilde,
            ],
        )
    }

    #[must_use]
    pub fn operand(&self) -> Option<Expression> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        let elements = self.syntax.children_with_tokens().collect::<Vec<_>>();
        matches!(
            elements.as_slice(),
            [operator, operand]
                if [
                    JavaSyntaxKind::PlusPlus,
                    JavaSyntaxKind::MinusMinus,
                    JavaSyntaxKind::Plus,
                    JavaSyntaxKind::Minus,
                    JavaSyntaxKind::Bang,
                    JavaSyntaxKind::Tilde,
                ]
                .contains(&operator.kind())
                    && Expression::can_cast(operand.kind())
        )
    }
}

impl PostfixExpression {
    #[must_use]
    pub fn operand(&self) -> Option<Expression> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn operator(&self) -> Option<JavaSyntaxToken> {
        child_token_in(
            &self.syntax,
            &[JavaSyntaxKind::PlusPlus, JavaSyntaxKind::MinusMinus],
        )
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        let elements = self.syntax.children_with_tokens().collect::<Vec<_>>();
        matches!(
            elements.as_slice(),
            [operand, operator]
                if Expression::can_cast(operand.kind())
                    && [JavaSyntaxKind::PlusPlus, JavaSyntaxKind::MinusMinus]
                        .contains(&operator.kind())
        )
    }
}

impl CastExpression {
    #[must_use]
    pub fn ty(&self) -> Option<Type> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn expression(&self) -> Option<Expression> {
        child_family(&self.syntax)
    }
}

impl ObjectCreationExpression {
    #[must_use]
    pub fn ty(&self) -> Option<Type> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn arguments(&self) -> Option<ArgumentList> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn body(&self) -> Option<ClassBody> {
        child(&self.syntax)
    }
}

impl ArrayCreationExpression {
    #[must_use]
    pub fn ty(&self) -> Option<Type> {
        child_family(&self.syntax)
    }

    pub fn dimensions(&self) -> impl Iterator<Item = DimExpression> + '_ {
        children(&self.syntax)
    }

    #[must_use]
    pub fn initializer(&self) -> Option<ArrayInitializer> {
        child(&self.syntax)
    }
}

impl LambdaExpression {
    #[must_use]
    pub fn parameters(&self) -> Option<LambdaParameterList> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn expression_body(&self) -> Option<Expression> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn block_body(&self) -> Option<Block> {
        child(&self.syntax)
    }
}

impl LambdaParameterList {
    pub fn parameters(&self) -> impl Iterator<Item = LambdaParameter> + '_ {
        children(&self.syntax)
    }
}

impl LambdaParameter {
    #[must_use]
    pub fn ty(&self) -> Option<Type> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn name(&self) -> Option<JavaSyntaxToken> {
        child_token_in(
            &self.syntax,
            &[JavaSyntaxKind::Identifier, JavaSyntaxKind::UnderscoreKw],
        )
    }
}
impl SwitchExpression {
    #[must_use]
    pub fn selector(&self) -> Option<Expression> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn block(&self) -> Option<SwitchBlock> {
        child(&self.syntax)
    }
}
