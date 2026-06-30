use std::iter::Peekable;

use super::super::{Expression, JavaSyntaxKind, JavaSyntaxNode, JavaSyntaxToken};

pub(super) fn is_modifier_token(kind: JavaSyntaxKind) -> bool {
    matches!(
        kind,
        JavaSyntaxKind::PublicKw
            | JavaSyntaxKind::ProtectedKw
            | JavaSyntaxKind::PrivateKw
            | JavaSyntaxKind::AbstractKw
            | JavaSyntaxKind::StaticKw
            | JavaSyntaxKind::FinalKw
            | JavaSyntaxKind::TransientKw
            | JavaSyntaxKind::VolatileKw
            | JavaSyntaxKind::SynchronizedKw
            | JavaSyntaxKind::NativeKw
            | JavaSyntaxKind::StrictfpKw
            | JavaSyntaxKind::DefaultKw
    )
}

pub(super) const ASSIGNMENT_OPERATORS: &[JavaSyntaxKind] = &[
    JavaSyntaxKind::Assign,
    JavaSyntaxKind::PlusEq,
    JavaSyntaxKind::MinusEq,
    JavaSyntaxKind::StarEq,
    JavaSyntaxKind::SlashEq,
    JavaSyntaxKind::AmpEq,
    JavaSyntaxKind::BarEq,
    JavaSyntaxKind::CaretEq,
    JavaSyntaxKind::PercentEq,
    JavaSyntaxKind::LShiftEq,
    JavaSyntaxKind::RShiftEq,
    JavaSyntaxKind::UnsignedRShiftEq,
];

pub(super) const BINARY_OPERATORS: &[JavaSyntaxKind] = &[
    JavaSyntaxKind::InstanceofKw,
    JavaSyntaxKind::OrOr,
    JavaSyntaxKind::AndAnd,
    JavaSyntaxKind::Bar,
    JavaSyntaxKind::Caret,
    JavaSyntaxKind::Amp,
    JavaSyntaxKind::EqEq,
    JavaSyntaxKind::BangEq,
    JavaSyntaxKind::Lt,
    JavaSyntaxKind::Gt,
    JavaSyntaxKind::LtEq,
    JavaSyntaxKind::GtEq,
    JavaSyntaxKind::LShift,
    JavaSyntaxKind::RShift,
    JavaSyntaxKind::UnsignedRShift,
    JavaSyntaxKind::Plus,
    JavaSyntaxKind::Minus,
    JavaSyntaxKind::Star,
    JavaSyntaxKind::Slash,
    JavaSyntaxKind::Percent,
];

pub(super) fn simple_single_token(syntax: &JavaSyntaxNode) -> Option<Vec<JavaSyntaxToken>> {
    let tokens = syntax
        .children_with_tokens()
        .map(jolt_syntax::SyntaxElement::into_token)
        .collect::<Option<Vec<_>>>()?;
    let [token] = tokens.as_slice() else {
        return None;
    };
    Some(vec![JavaSyntaxToken {
        syntax: token.clone(),
    }])
}

pub(super) fn has_keyword_optional_expression_semicolon_shape(
    syntax: &JavaSyntaxNode,
    keyword_kind: JavaSyntaxKind,
    keyword_text: Option<&str>,
) -> bool {
    let elements = syntax.children_with_tokens().collect::<Vec<_>>();
    let [keyword, semicolon] = elements.as_slice() else {
        let [keyword, expression, semicolon] = elements.as_slice() else {
            return false;
        };
        return keyword_matches(keyword, keyword_kind, keyword_text)
            && Expression::can_cast(expression.kind())
            && semicolon.kind() == JavaSyntaxKind::Semicolon;
    };

    keyword_matches(keyword, keyword_kind, keyword_text)
        && semicolon.kind() == JavaSyntaxKind::Semicolon
}

pub(super) fn has_keyword_required_expression_semicolon_shape(
    syntax: &JavaSyntaxNode,
    keyword_kind: JavaSyntaxKind,
    keyword_text: Option<&str>,
) -> bool {
    let elements = syntax.children_with_tokens().collect::<Vec<_>>();
    let [keyword, expression, semicolon] = elements.as_slice() else {
        return false;
    };

    keyword_matches(keyword, keyword_kind, keyword_text)
        && Expression::can_cast(expression.kind())
        && semicolon.kind() == JavaSyntaxKind::Semicolon
}

pub(super) fn has_keyword_optional_label_semicolon_shape(
    syntax: &JavaSyntaxNode,
    keyword_kind: JavaSyntaxKind,
) -> bool {
    let elements = syntax.children_with_tokens().collect::<Vec<_>>();
    match elements.as_slice() {
        [keyword, semicolon] => {
            keyword_matches(keyword, keyword_kind, None)
                && semicolon.kind() == JavaSyntaxKind::Semicolon
        }
        [keyword, label, semicolon] => {
            keyword_matches(keyword, keyword_kind, None)
                && label.kind() == JavaSyntaxKind::Identifier
                && semicolon.kind() == JavaSyntaxKind::Semicolon
        }
        _ => false,
    }
}

fn keyword_matches(
    element: &jolt_syntax::SyntaxElement<crate::language::JavaLanguage>,
    expected: JavaSyntaxKind,
    expected_text: Option<&str>,
) -> bool {
    let Some(token) = element.clone().into_token() else {
        return false;
    };
    token.kind() == expected && expected_text.is_none_or(|text| token.text() == text)
}

pub(super) fn has_braced_block_statement_layout_shape(syntax: &JavaSyntaxNode) -> bool {
    let kinds = syntax
        .children_with_tokens()
        .map(|element| element.kind())
        .collect::<Vec<_>>();
    matches!(kinds.first(), Some(JavaSyntaxKind::LBrace))
        && matches!(kinds.last(), Some(JavaSyntaxKind::RBrace))
        && kinds[1..kinds.len().saturating_sub(1)]
            .iter()
            .all(|kind| *kind == JavaSyntaxKind::BlockStatement)
}

pub(super) fn has_method_declaration_layout_shape(syntax: &JavaSyntaxNode) -> bool {
    let mut cursor = syntax_kind_cursor(syntax);
    cursor.eat(JavaSyntaxKind::ModifierList);
    cursor.eat(JavaSyntaxKind::TypeParameterList);
    while cursor.eat(JavaSyntaxKind::Annotation) {}
    if !cursor.eat_one_of(&[
        JavaSyntaxKind::PrimitiveType,
        JavaSyntaxKind::VoidType,
        JavaSyntaxKind::ClassType,
        JavaSyntaxKind::ArrayType,
    ]) {
        return false;
    }
    if !cursor.eat(JavaSyntaxKind::Identifier) {
        return false;
    }
    if !cursor.eat(JavaSyntaxKind::LParen) {
        return false;
    }
    cursor.eat(JavaSyntaxKind::FormalParameterList);
    if !cursor.eat(JavaSyntaxKind::RParen) {
        return false;
    }
    cursor.eat(JavaSyntaxKind::ArrayDimensions);
    cursor.eat(JavaSyntaxKind::ThrowsClause);

    cursor.eat_one_of(&[JavaSyntaxKind::Block, JavaSyntaxKind::Semicolon]) && cursor.is_done()
}

pub(super) fn has_constructor_declaration_layout_shape(syntax: &JavaSyntaxNode) -> bool {
    let mut cursor = syntax_kind_cursor(syntax);
    cursor.eat(JavaSyntaxKind::ModifierList);
    cursor.eat(JavaSyntaxKind::TypeParameterList);
    if !cursor.eat(JavaSyntaxKind::Identifier) {
        return false;
    }
    if !cursor.eat(JavaSyntaxKind::LParen) {
        return false;
    }
    cursor.eat(JavaSyntaxKind::FormalParameterList);
    if !cursor.eat(JavaSyntaxKind::RParen) {
        return false;
    }
    cursor.eat(JavaSyntaxKind::ThrowsClause);

    cursor.eat(JavaSyntaxKind::ConstructorBody) && cursor.is_done()
}

struct SyntaxKindCursor<I>
where
    I: Iterator<Item = JavaSyntaxKind>,
{
    kinds: Peekable<I>,
}

fn syntax_kind_cursor(
    syntax: &JavaSyntaxNode,
) -> SyntaxKindCursor<impl Iterator<Item = JavaSyntaxKind> + '_> {
    SyntaxKindCursor {
        kinds: syntax
            .children_with_tokens()
            .map(|element| element.kind())
            .peekable(),
    }
}

impl<I> SyntaxKindCursor<I>
where
    I: Iterator<Item = JavaSyntaxKind>,
{
    fn eat(&mut self, expected: JavaSyntaxKind) -> bool {
        self.eat_if(|kind| kind == expected)
    }

    fn eat_one_of(&mut self, expected: &[JavaSyntaxKind]) -> bool {
        self.eat_if(|kind| expected.contains(&kind))
    }

    fn eat_if(&mut self, predicate: impl FnOnce(JavaSyntaxKind) -> bool) -> bool {
        let Some(kind) = self.kinds.peek().copied() else {
            return false;
        };
        if !predicate(kind) {
            return false;
        }

        self.kinds.next();
        true
    }

    fn is_done(mut self) -> bool {
        self.kinds.next().is_none()
    }
}

pub(super) fn has_angle_comma_list_layout_shape(
    syntax: &JavaSyntaxNode,
    element_kind: JavaSyntaxKind,
) -> bool {
    let elements = syntax.children_with_tokens().collect::<Vec<_>>();
    let Some(first) = elements.first() else {
        return false;
    };
    let Some(last) = elements.last() else {
        return false;
    };
    first.kind() == JavaSyntaxKind::Lt
        && last.kind() == JavaSyntaxKind::Gt
        && has_comma_separated_elements(&elements[1..elements.len().saturating_sub(1)], |kind| {
            kind == element_kind
        })
}

pub(super) fn has_comma_list_layout_shape(
    syntax: &JavaSyntaxNode,
    element_kind: JavaSyntaxKind,
) -> bool {
    let elements = syntax.children_with_tokens().collect::<Vec<_>>();
    has_comma_separated_elements(&elements, |kind| kind == element_kind)
}

pub(super) fn has_comma_separated_elements(
    elements: &[jolt_syntax::SyntaxElement<crate::language::JavaLanguage>],
    is_element: impl Fn(JavaSyntaxKind) -> bool,
) -> bool {
    !elements.is_empty()
        && elements.len() % 2 == 1
        && elements.iter().enumerate().all(|(index, element)| {
            if index % 2 == 0 {
                is_element(element.kind())
            } else {
                element.kind() == JavaSyntaxKind::Comma
            }
        })
}

pub(super) fn is_literal_token(kind: JavaSyntaxKind) -> bool {
    matches!(
        kind,
        JavaSyntaxKind::IntegerLiteral
            | JavaSyntaxKind::FloatingPointLiteral
            | JavaSyntaxKind::BooleanLiteral
            | JavaSyntaxKind::CharacterLiteral
            | JavaSyntaxKind::StringLiteral
            | JavaSyntaxKind::TextBlockLiteral
            | JavaSyntaxKind::NullLiteral
    )
}
