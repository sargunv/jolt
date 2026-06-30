use super::super::{
    Annotation, ArgumentList, ArrayAccessExpression, ArrayCreationExpression, ArrayDimensions,
    ArrayInitializer, AssignmentExpression, BinaryExpression, Block, CastExpression, ClassBody,
    ClassLiteralExpression, ConditionalExpression, DimExpression, Expression,
    FieldAccessExpression, InstanceofExpression, JavaNode, JavaSyntaxKind, JavaSyntaxNode,
    JavaSyntaxToken, LambdaExpression, LambdaParameter, LambdaParameterList, LiteralExpression,
    LocalVariableDeclaration, MethodInvocationExpression, MethodReferenceExpression,
    NameExpression, ObjectCreationExpression, ParenthesizedExpression, Pattern, PostfixExpression,
    RecordPattern, SuperExpression, SwitchBlock, SwitchExpression, ThisExpression, Type,
    TypeArgumentList, TypePattern, UnaryExpression, VariableInitializerValue, child, child_family,
    child_token, child_token_in, children, children_family, nth_child_family,
};
use super::helpers::{ASSIGNMENT_OPERATORS, BINARY_OPERATORS, is_literal_token};

impl MethodInvocationExpression {
    #[must_use]
    pub fn receiver(&self) -> Option<Expression> {
        let elements = self.syntax.children_with_tokens().collect::<Vec<_>>();
        let Some(receiver) = elements.first() else {
            return None;
        };
        let Some(dot) = elements.get(1) else {
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
        direct_child(&self.syntax)
    }

    #[must_use]
    pub fn type_arguments(&self) -> Option<TypeArgumentList> {
        direct_child(&self.syntax)
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
            [receiver, dot, type_arguments, name, arguments] => {
                Expression::can_cast(receiver.kind())
                    && dot.kind() == JavaSyntaxKind::Dot
                    && type_arguments.kind() == JavaSyntaxKind::TypeArgumentList
                    && type_arguments
                        .clone()
                        .into_node()
                        .and_then(TypeArgumentList::cast)
                        .is_some_and(|type_arguments| {
                            type_arguments.simple_layout_parts().is_some()
                        })
                    && name.kind() == JavaSyntaxKind::Identifier
                    && arguments.kind() == JavaSyntaxKind::ArgumentList
            }
            _ => false,
        }
    }
}

fn direct_child<N: JavaNode>(syntax: &JavaSyntaxNode) -> Option<N> {
    syntax.children().find_map(N::cast)
}

impl ArgumentList {
    pub fn arguments(&self) -> impl Iterator<Item = Expression> + '_ {
        children_family(&self.syntax)
    }

    #[must_use]
    pub fn has_trailing_comma(&self) -> bool {
        let elements = self.syntax.children_with_tokens().collect::<Vec<_>>();
        elements
            .get(elements.len().saturating_sub(2))
            .is_some_and(|element| element.kind() == JavaSyntaxKind::Comma)
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

impl MethodReferenceExpression {
    #[must_use]
    pub fn expression_qualifier(&self) -> Option<Expression> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn type_qualifier(&self) -> Option<Type> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn dimensions(&self) -> Option<ArrayDimensions> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn qualifier_type_arguments(&self) -> Option<TypeArgumentList> {
        let elements = self.syntax.children_with_tokens().collect::<Vec<_>>();
        let double_colon_index = elements
            .iter()
            .position(|element| element.kind() == JavaSyntaxKind::DoubleColon)?;
        elements[..double_colon_index]
            .iter()
            .find(|element| element.kind() == JavaSyntaxKind::TypeArgumentList)
            .and_then(|element| element.clone().into_node())
            .and_then(TypeArgumentList::cast)
    }

    #[must_use]
    pub fn member_type_arguments(&self) -> Option<TypeArgumentList> {
        let elements = self.syntax.children_with_tokens().collect::<Vec<_>>();
        let double_colon_index = elements
            .iter()
            .position(|element| element.kind() == JavaSyntaxKind::DoubleColon)?;
        elements[double_colon_index + 1..]
            .iter()
            .find(|element| element.kind() == JavaSyntaxKind::TypeArgumentList)
            .and_then(|element| element.clone().into_node())
            .and_then(TypeArgumentList::cast)
    }

    #[must_use]
    pub fn name(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::Identifier)
    }

    #[must_use]
    pub fn is_constructor_reference(&self) -> bool {
        child_token(&self.syntax, JavaSyntaxKind::NewKw).is_some()
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        let elements = self.syntax.children_with_tokens().collect::<Vec<_>>();
        let Some(double_colon_index) = elements
            .iter()
            .position(|element| element.kind() == JavaSyntaxKind::DoubleColon)
        else {
            return false;
        };
        if double_colon_index == 0 {
            return false;
        }

        let qualifier = &elements[..double_colon_index];
        let member = &elements[double_colon_index + 1..];
        let qualifier_is_supported = match qualifier {
            [qualifier] => {
                Expression::can_cast(qualifier.kind()) || Type::can_cast(qualifier.kind())
            }
            [qualifier, dimensions] => {
                (Expression::can_cast(qualifier.kind()) || Type::can_cast(qualifier.kind()))
                    && dimensions.kind() == JavaSyntaxKind::ArrayDimensions
            }
            _ => false,
        };
        if !qualifier_is_supported {
            return false;
        }

        match member {
            [name] => {
                name.kind() == JavaSyntaxKind::Identifier || name.kind() == JavaSyntaxKind::NewKw
            }
            [type_arguments, name] => {
                type_arguments.kind() == JavaSyntaxKind::TypeArgumentList
                    && type_arguments
                        .clone()
                        .into_node()
                        .and_then(TypeArgumentList::cast)
                        .is_some_and(|type_arguments| {
                            type_arguments.simple_layout_parts().is_some()
                        })
                    && (name.kind() == JavaSyntaxKind::Identifier
                        || name.kind() == JavaSyntaxKind::NewKw)
            }
            _ => false,
        }
    }
}

impl ArrayAccessExpression {
    #[must_use]
    pub fn receiver(&self) -> Option<Expression> {
        nth_child_family(&self.syntax, 0)
    }

    #[must_use]
    pub fn index(&self) -> Option<Expression> {
        nth_child_family(&self.syntax, 1)
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        let elements = self.syntax.children_with_tokens().collect::<Vec<_>>();
        matches!(
            elements.as_slice(),
            [receiver, left, index, right]
                if Expression::can_cast(receiver.kind())
                    && left.kind() == JavaSyntaxKind::LBracket
                    && Expression::can_cast(index.kind())
                    && right.kind() == JavaSyntaxKind::RBracket
        )
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
    pub fn annotations(&self) -> impl Iterator<Item = Annotation> + '_ {
        children(&self.syntax)
    }

    #[must_use]
    pub fn identifier(&self) -> Option<JavaSyntaxToken> {
        self.syntax
            .children_with_tokens()
            .filter_map(jolt_syntax::SyntaxElement::into_token)
            .find(|token| token.kind() == JavaSyntaxKind::Identifier)
            .map(|syntax| JavaSyntaxToken { syntax })
    }

    #[must_use]
    pub fn has_simple_layout_shape(&self) -> bool {
        let mut saw_identifier = false;
        for element in self.syntax.children_with_tokens() {
            match element.kind() {
                JavaSyntaxKind::Annotation if !saw_identifier => {}
                JavaSyntaxKind::Identifier if !saw_identifier => saw_identifier = true,
                _ => return false,
            }
        }
        saw_identifier
    }
}

impl ThisExpression {
    #[must_use]
    pub fn receiver(&self) -> Option<Expression> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn token(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::ThisKw)
    }

    #[must_use]
    pub fn has_simple_layout_shape(&self) -> bool {
        self.token().is_some()
    }
}

impl SuperExpression {
    #[must_use]
    pub fn receiver(&self) -> Option<Expression> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn token(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::SuperKw)
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
    pub fn type_arguments(&self) -> Option<TypeArgumentList> {
        child(&self.syntax)
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

impl ClassLiteralExpression {
    #[must_use]
    pub fn expression(&self) -> Option<Expression> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn ty(&self) -> Option<Type> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn dimensions(&self) -> Option<ArrayDimensions> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn primitive_or_void_token(&self) -> Option<JavaSyntaxToken> {
        self.syntax
            .children_with_tokens()
            .filter_map(jolt_syntax::SyntaxElement::into_token)
            .find(|token| is_primitive_or_void_keyword(token.kind()))
            .map(|syntax| JavaSyntaxToken { syntax })
    }

    #[must_use]
    pub fn class_token(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::ClassKw)
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        let elements = self.syntax.children_with_tokens().collect::<Vec<_>>();
        let [qualifier, rest @ ..] = elements.as_slice() else {
            return false;
        };
        if !(Expression::can_cast(qualifier.kind())
            || Type::can_cast(qualifier.kind())
            || is_primitive_or_void_keyword(qualifier.kind()))
        {
            return false;
        }

        matches!(
            rest,
            [dot, class]
                if dot.kind() == JavaSyntaxKind::Dot && class.kind() == JavaSyntaxKind::ClassKw
        ) || matches!(
            rest,
            [dimensions, dot, class]
                if dimensions.kind() == JavaSyntaxKind::ArrayDimensions
                    && dot.kind() == JavaSyntaxKind::Dot
                    && class.kind() == JavaSyntaxKind::ClassKw
        )
    }
}

fn is_primitive_or_void_keyword(kind: JavaSyntaxKind) -> bool {
    matches!(
        kind,
        JavaSyntaxKind::BooleanKw
            | JavaSyntaxKind::ByteKw
            | JavaSyntaxKind::CharKw
            | JavaSyntaxKind::DoubleKw
            | JavaSyntaxKind::FloatKw
            | JavaSyntaxKind::IntKw
            | JavaSyntaxKind::LongKw
            | JavaSyntaxKind::ShortKw
            | JavaSyntaxKind::VoidKw
    )
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

impl InstanceofExpression {
    #[must_use]
    pub fn expression(&self) -> Option<Expression> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn ty(&self) -> Option<Type> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn pattern(&self) -> Option<Pattern> {
        child_family(&self.syntax)
    }
}

impl TypePattern {
    #[must_use]
    pub fn local_variable_declaration(&self) -> Option<LocalVariableDeclaration> {
        child(&self.syntax)
    }
}

impl RecordPattern {
    #[must_use]
    pub fn ty(&self) -> Option<Type> {
        child_family(&self.syntax)
    }

    pub fn components(&self) -> impl Iterator<Item = super::super::ComponentPattern> + '_ {
        children(&self.syntax)
    }
}

impl super::super::ComponentPattern {
    #[must_use]
    pub fn pattern(&self) -> Option<Pattern> {
        child_family(&self.syntax)
    }
}

impl super::super::MatchAllPattern {
    #[must_use]
    pub fn token(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::UnderscoreKw)
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
    pub fn qualifier(&self) -> Option<Expression> {
        let elements = self.syntax.children_with_tokens().collect::<Vec<_>>();
        let Some(qualifier) = elements.first() else {
            return None;
        };
        let Some(dot) = elements.get(1) else {
            return None;
        };
        if !Expression::can_cast(qualifier.kind()) || dot.kind() != JavaSyntaxKind::Dot {
            return None;
        }
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn type_arguments(&self) -> Option<TypeArgumentList> {
        child(&self.syntax)
    }

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

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        let elements = self.syntax.children_with_tokens().collect::<Vec<_>>();
        let mut index = 0;
        if elements.get(index).is_some_and(|element| {
            Expression::can_cast(element.kind())
                && elements
                    .get(index + 1)
                    .is_some_and(|dot| dot.kind() == JavaSyntaxKind::Dot)
        }) {
            index += 2;
        }
        if !elements
            .get(index)
            .is_some_and(|element| element.kind() == JavaSyntaxKind::NewKw)
        {
            return false;
        }
        index += 1;
        if elements
            .get(index)
            .is_some_and(|element| element.kind() == JavaSyntaxKind::TypeArgumentList)
        {
            let Some(arguments) = elements
                .get(index)
                .and_then(|element| element.clone().into_node())
                .and_then(TypeArgumentList::cast)
            else {
                return false;
            };
            if arguments.simple_layout_parts().is_none() {
                return false;
            }
            index += 1;
        }
        if !elements
            .get(index)
            .is_some_and(|element| Type::can_cast(element.kind()))
        {
            return false;
        }
        index += 1;
        if !elements
            .get(index)
            .is_some_and(|element| element.kind() == JavaSyntaxKind::ArgumentList)
        {
            return false;
        }
        index += 1;
        if elements
            .get(index)
            .is_some_and(|element| element.kind() == JavaSyntaxKind::ClassBody)
        {
            index += 1;
        }
        index == elements.len()
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
    pub fn trailing_dimensions(&self) -> Option<ArrayDimensions> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn initializer(&self) -> Option<ArrayInitializer> {
        child(&self.syntax)
    }
}

impl DimExpression {
    pub fn annotations(&self) -> impl Iterator<Item = Annotation> + '_ {
        children(&self.syntax)
    }

    #[must_use]
    pub fn expression(&self) -> Option<Expression> {
        child_family(&self.syntax)
    }
}

impl ArrayInitializer {
    pub fn values(&self) -> impl Iterator<Item = VariableInitializerValue> + '_ {
        children_family(&self.syntax)
    }

    #[must_use]
    pub fn has_trailing_comma(&self) -> bool {
        let elements = self.syntax.children_with_tokens().collect::<Vec<_>>();
        elements
            .get(elements.len().saturating_sub(2))
            .is_some_and(|element| element.kind() == JavaSyntaxKind::Comma)
    }
}

impl LambdaExpression {
    #[must_use]
    pub fn has_empty_parameter_list(&self) -> bool {
        let elements = self.syntax.children_with_tokens().collect::<Vec<_>>();
        matches!(
            elements.as_slice(),
            [left, parameters, right, arrow, body]
                if left.kind() == JavaSyntaxKind::LParen
                    && parameters.kind() == JavaSyntaxKind::LambdaParameterList
                    && parameters
                        .clone()
                        .into_node()
                        .and_then(LambdaParameterList::cast)
                        .is_some_and(|parameters| parameters.is_empty())
                    && right.kind() == JavaSyntaxKind::RParen
                    && arrow.kind() == JavaSyntaxKind::Arrow
                    && (Expression::can_cast(body.kind()) || body.kind() == JavaSyntaxKind::Block)
        )
    }

    #[must_use]
    pub fn single_parameter(&self) -> Option<LambdaParameter> {
        let elements = self.syntax.children_with_tokens().collect::<Vec<_>>();
        matches!(elements.as_slice(), [parameter, arrow, body]
            if parameter.kind() == JavaSyntaxKind::LambdaParameter
                && arrow.kind() == JavaSyntaxKind::Arrow
                && (Expression::can_cast(body.kind()) || body.kind() == JavaSyntaxKind::Block))
        .then(|| child(&self.syntax))
        .flatten()
    }

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

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        let elements = self.syntax.children_with_tokens().collect::<Vec<_>>();
        match elements.as_slice() {
            [parameter, arrow, body] => {
                parameter.kind() == JavaSyntaxKind::LambdaParameter
                    && LambdaParameter::cast(parameter.clone().into_node().expect("parameter node"))
                        .is_some_and(|parameter| parameter.has_supported_layout_shape())
                    && arrow.kind() == JavaSyntaxKind::Arrow
                    && (Expression::can_cast(body.kind()) || body.kind() == JavaSyntaxKind::Block)
            }
            [left, parameters, right, arrow, body] => {
                left.kind() == JavaSyntaxKind::LParen
                    && parameters.kind() == JavaSyntaxKind::LambdaParameterList
                    && LambdaParameterList::cast(
                        parameters.clone().into_node().expect("parameter list node"),
                    )
                    .is_some_and(|parameters| {
                        parameters.is_empty() || parameters.has_supported_layout_shape()
                    })
                    && right.kind() == JavaSyntaxKind::RParen
                    && arrow.kind() == JavaSyntaxKind::Arrow
                    && (Expression::can_cast(body.kind()) || body.kind() == JavaSyntaxKind::Block)
            }
            _ => false,
        }
    }
}

impl LambdaParameterList {
    pub fn parameters(&self) -> impl Iterator<Item = LambdaParameter> + '_ {
        children(&self.syntax)
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.syntax.children_with_tokens().next().is_none()
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        let elements = self.syntax.children_with_tokens().collect::<Vec<_>>();
        let Some(first) = elements.first() else {
            return false;
        };
        if first.kind() != JavaSyntaxKind::LambdaParameter
            || !LambdaParameter::cast(first.clone().into_node().expect("parameter node"))
                .is_some_and(|parameter| parameter.has_supported_layout_shape())
        {
            return false;
        }

        let mut expect_comma = true;
        for element in &elements[1..] {
            if expect_comma {
                if element.kind() != JavaSyntaxKind::Comma {
                    return false;
                }
            } else if element.kind() != JavaSyntaxKind::LambdaParameter
                || !LambdaParameter::cast(element.clone().into_node().expect("parameter node"))
                    .is_some_and(|parameter| parameter.has_supported_layout_shape())
            {
                return false;
            }
            expect_comma = !expect_comma;
        }

        expect_comma
    }
}

impl LambdaParameter {
    #[must_use]
    pub fn final_token(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::FinalKw)
    }

    #[must_use]
    pub fn ty(&self) -> Option<Type> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn name(&self) -> Option<JavaSyntaxToken> {
        self.syntax
            .children_with_tokens()
            .filter_map(jolt_syntax::SyntaxElement::into_token)
            .filter(|token| {
                matches!(
                    token.kind(),
                    JavaSyntaxKind::Identifier | JavaSyntaxKind::UnderscoreKw
                )
            })
            .last()
            .map(|syntax| JavaSyntaxToken { syntax })
    }

    #[must_use]
    pub fn ellipsis(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::Ellipsis)
    }

    #[must_use]
    pub fn var_token(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::Identifier).filter(|token| token.text() == "var")
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        let elements = self.syntax.children_with_tokens().collect::<Vec<_>>();
        match elements.as_slice() {
            [name] => matches!(
                name.kind(),
                JavaSyntaxKind::Identifier | JavaSyntaxKind::UnderscoreKw
            ),
            [prefix, name] => {
                (prefix.kind() == JavaSyntaxKind::FinalKw || is_lambda_parameter_prefix(prefix))
                    && matches!(
                        name.kind(),
                        JavaSyntaxKind::Identifier | JavaSyntaxKind::UnderscoreKw
                    )
            }
            [first, second, name] => {
                ((first.kind() == JavaSyntaxKind::FinalKw && is_lambda_parameter_prefix(second))
                    || (Type::can_cast(first.kind()) && second.kind() == JavaSyntaxKind::Ellipsis))
                    && matches!(
                        name.kind(),
                        JavaSyntaxKind::Identifier | JavaSyntaxKind::UnderscoreKw
                    )
            }
            [final_kw, ty, ellipsis, name] => {
                final_kw.kind() == JavaSyntaxKind::FinalKw
                    && Type::can_cast(ty.kind())
                    && ellipsis.kind() == JavaSyntaxKind::Ellipsis
                    && matches!(
                        name.kind(),
                        JavaSyntaxKind::Identifier | JavaSyntaxKind::UnderscoreKw
                    )
            }
            _ => false,
        }
    }
}

fn is_lambda_parameter_prefix(
    element: &jolt_syntax::SyntaxElement<crate::language::JavaLanguage>,
) -> bool {
    Type::can_cast(element.kind())
        || (element.kind() == JavaSyntaxKind::Identifier
            && element
                .clone()
                .into_token()
                .is_some_and(|token| token.text() == "var"))
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
