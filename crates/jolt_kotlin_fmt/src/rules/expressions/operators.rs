use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_kotlin_syntax::{
    AssignmentExpression, BinaryExpression, Expression, KotlinRoleElement, KotlinSyntaxField,
    KotlinSyntaxKind, KotlinSyntaxToken, ParenthesizedExpression, PostfixExpression,
    UnaryExpression,
};

use crate::helpers::comments::{LeadingTrivia, TrailingTrivia, comment_forces_line, format_token};
use crate::helpers::recovery::format_required_field;
use crate::helpers::syntax_tokens::inserted_syntax_token;
use crate::rules::types::format_type_reference;

use super::{format_expression, format_expression_with_leading};

pub(super) fn format_parenthesized_expression<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &ParenthesizedExpression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let open = format_required_field(expression.open_paren(), doc, |token, doc| {
        format_token(
            doc,
            &token,
            leading,
            TrailingTrivia::RelocatedToEnclosingContext,
        )
    });
    let inner = format_required_field(expression.expression(), doc, |inner, doc| {
        format_expression(doc, &inner)
    });
    let close = format_required_field(expression.close_paren(), doc, |token, doc| {
        format_token(
            doc,
            &token,
            LeadingTrivia::Preserve,
            TrailingTrivia::Preserve,
        )
    });
    doc.concat([open, inner, close])
}

pub(super) fn format_assignment_expression<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &AssignmentExpression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let left = format_required_field(expression.left(), doc, |left, doc| {
        format_expression_with_leading(doc, &left, leading)
    });
    let operator = format_required_field(expression.operator(), doc, |operator, doc| {
        format_token(
            doc,
            &operator,
            LeadingTrivia::Preserve,
            TrailingTrivia::Preserve,
        )
    });
    let right = format_required_field(expression.right(), doc, |right, doc| {
        format_expression(doc, &right)
    });

    let space = doc.space();
    let line = doc.line();
    let right = doc.concat([line, right]);
    let right = doc.indent(right);
    let contents = doc.concat([left, space, operator, right]);
    doc.group(contents)
}

pub(super) fn format_binary_expression<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &BinaryExpression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let Some(operator) = binary_operator(expression) else {
        return format_binary_fields(doc, expression, leading);
    };
    let Some(left) = present(expression.left()) else {
        return format_binary_fields(doc, expression, leading);
    };
    let Some(right) = present(expression.right()) else {
        return format_binary_fields(doc, expression, leading);
    };

    if is_type_binary_operator(&operator) {
        let left = format_expression_with_leading(doc, &left, leading);
        let operator_doc = format_binary_operator(doc, &operator).doc;
        let right =
            if let Some(ty) = right.cast_node::<jolt_kotlin_syntax::TypeReference<'source>>() {
                format_type_reference(doc, &ty)
            } else if let Some(expression) = right.cast_family::<Expression<'source>>() {
                format_expression(doc, &expression)
            } else {
                doc.block_on_invariant("type binary expression had an unsupported right role");
                Doc::nil()
            };
        let space = doc.space();
        let line = doc.line();
        let right = doc.concat([line, right]);
        let right = doc.indent(right);
        let contents = doc.concat([left, space, operator_doc, right]);
        return doc.group(contents);
    }

    let Some(right) = right.cast_family::<Expression<'source>>() else {
        return format_binary_fields(doc, expression, leading);
    };
    let root = Expression::from(*expression);
    let mut operands = Vec::new();
    let mut operators = Vec::new();
    collect_binary_chain(root, &mut operands, &mut operators);
    if operators.len() + 1 != operands.len() {
        let left = format_binary_operand_with_leading(doc, &left, &operator, leading);
        let right = format_binary_operand(doc, &right, &operator);
        let part = binary_chain_part(doc, operator, right);
        return binary_chain(doc, left, vec![part]);
    }

    let mut operands = operands.into_iter();
    let first = operands.next().unwrap_or(left);
    let first = format_binary_operand_with_leading(doc, &first, &operator, leading);
    let mut rest = Vec::new();
    for (operator, operand) in operators.into_iter().zip(operands) {
        let operand = format_binary_operand(doc, &operand, &operator);
        rest.push(binary_chain_part(doc, operator, operand));
    }
    binary_chain(doc, first, rest)
}

fn format_binary_fields<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &BinaryExpression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let left = format_required_field(expression.left(), doc, |left, doc| {
        format_expression_with_leading(doc, &left, leading)
    });
    let operator = format_required_field(expression.operator(), doc, |operator, doc| {
        if let Some(operator) = operator.token() {
            format_token(
                doc,
                &operator,
                LeadingTrivia::Preserve,
                TrailingTrivia::Preserve,
            )
        } else {
            doc.block_on_invariant("binary operator role was not a token");
            Doc::nil()
        }
    });
    let right = format_required_field(expression.right(), doc, |right, doc| {
        format_binary_right(doc, right)
    });
    let space = doc.space();
    let line = doc.line();
    let right = doc.concat([line, right]);
    let right = doc.indent(right);
    let contents = doc.concat([left, space, operator, right]);
    doc.group(contents)
}

fn format_binary_right<'source>(
    doc: &mut DocBuilder<'source>,
    right: KotlinRoleElement<'source>,
) -> Doc<'source> {
    if let Some(expression) = right.cast_family::<Expression<'source>>() {
        format_expression(doc, &expression)
    } else if let Some(ty) = right.cast_node::<jolt_kotlin_syntax::TypeReference<'source>>() {
        format_type_reference(doc, &ty)
    } else {
        doc.block_on_invariant("binary expression had an unsupported right role");
        Doc::nil()
    }
}

fn binary_operator<'source>(
    expression: &BinaryExpression<'source>,
) -> Option<KotlinSyntaxToken<'source>> {
    present(expression.operator())?.token()
}

fn present<T>(
    field: Result<KotlinSyntaxField<'_, T>, jolt_kotlin_syntax::KotlinSyntaxInvariantError>,
) -> Option<T> {
    match field.ok()? {
        KotlinSyntaxField::Present(value) => Some(value),
        KotlinSyntaxField::Missing(_) | KotlinSyntaxField::Malformed(_) => None,
    }
}

fn format_binary_operand<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &Expression<'source>,
    parent_operator: &KotlinSyntaxToken<'_>,
) -> Doc<'source> {
    let formatted = format_expression(doc, expression);
    format_binary_operand_doc(doc, formatted, expression, parent_operator)
}

fn format_binary_operand_with_leading<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &Expression<'source>,
    parent_operator: &KotlinSyntaxToken<'_>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let formatted = format_expression_with_leading(doc, expression, leading);
    format_binary_operand_doc(doc, formatted, expression, parent_operator)
}

fn format_binary_operand_doc<'source>(
    doc: &mut DocBuilder<'source>,
    formatted: Doc<'source>,
    expression: &Expression<'source>,
    parent_operator: &KotlinSyntaxToken<'_>,
) -> Doc<'source> {
    if !should_parenthesize_binary_operand(expression, parent_operator) {
        return formatted;
    }
    let Some(claims) = expression.precedence_parenthesis_claims() else {
        doc.block_on_invariant("valid Kotlin binary operand lacked normalization claims");
        return formatted;
    };
    let open = inserted_syntax_token(doc, claims.open);
    let line = doc.soft_line();
    let formatted = doc.concat([line, formatted]);
    let formatted = doc.indent(formatted);
    let trailing = doc.soft_line();
    let close = inserted_syntax_token(doc, claims.close);
    let contents = doc.concat([open, formatted, trailing, close]);
    doc.group(contents)
}

fn collect_binary_chain<'source>(
    expression: Expression<'source>,
    operands: &mut Vec<Expression<'source>>,
    operators: &mut Vec<KotlinSyntaxToken<'source>>,
) {
    let Expression::BinaryExpression(binary) = expression else {
        operands.push(expression);
        return;
    };
    let Some(operator) = binary_operator(&binary) else {
        operands.push(expression);
        return;
    };
    if is_type_binary_operator(&operator) {
        operands.push(expression);
        return;
    }
    let Some(left) = present(binary.left()) else {
        operands.push(expression);
        return;
    };
    let Some(right) =
        present(binary.right()).and_then(KotlinRoleElement::cast_family::<Expression>)
    else {
        operands.push(expression);
        return;
    };

    collect_binary_left(left, &operator, operands, operators);
    operators.push(operator);
    operands.push(right);
}

fn collect_binary_left<'source>(
    expression: Expression<'source>,
    parent_operator: &KotlinSyntaxToken<'_>,
    operands: &mut Vec<Expression<'source>>,
    operators: &mut Vec<KotlinSyntaxToken<'source>>,
) {
    let Expression::BinaryExpression(binary) = expression else {
        operands.push(expression);
        return;
    };
    let Some(operator) = binary_operator(&binary) else {
        operands.push(expression);
        return;
    };
    if !should_flatten_binary(parent_operator, &operator) {
        operands.push(expression);
        return;
    }
    collect_binary_chain(Expression::from(binary), operands, operators);
}

struct BinaryOperatorDoc<'source> {
    doc: Doc<'source>,
    forces_line_after: bool,
}

fn format_binary_operator<'source>(
    doc: &mut DocBuilder<'source>,
    operator: &KotlinSyntaxToken<'source>,
) -> BinaryOperatorDoc<'source> {
    let forces_line_after = operator
        .trailing_comments()
        .any(|comment| comment_forces_line(&comment));
    let trailing = if is_range_operator(operator) {
        TrailingTrivia::Preserve
    } else {
        TrailingTrivia::BeforeLineBreak
    };
    BinaryOperatorDoc {
        doc: format_token(doc, operator, LeadingTrivia::Preserve, trailing),
        forces_line_after,
    }
}

struct BinaryChainPart<'source> {
    operator: BinaryOperatorDoc<'source>,
    operand: Doc<'source>,
    spaced: bool,
    break_before_operator: bool,
}

fn binary_chain_part<'source>(
    doc: &mut DocBuilder<'source>,
    operator: KotlinSyntaxToken<'source>,
    operand: Doc<'source>,
) -> BinaryChainPart<'source> {
    BinaryChainPart {
        operator: format_binary_operator(doc, &operator),
        operand,
        spaced: !is_range_operator(&operator),
        break_before_operator: can_break_before_operator(&operator),
    }
}

fn binary_chain<'source>(
    doc: &mut DocBuilder<'source>,
    first: Doc<'source>,
    rest: Vec<BinaryChainPart<'source>>,
) -> Doc<'source> {
    if rest.is_empty() {
        return first;
    }
    let mut docs = Vec::new();
    for part in rest {
        if !part.break_before_operator {
            let space = doc.space();
            let line = doc.line();
            let operand = doc.concat([line, part.operand]);
            let operand = doc.indent(operand);
            docs.push(doc.concat([space, part.operator.doc, operand]));
        } else if part.operator.forces_line_after {
            let before = doc.line();
            let after = doc.line();
            docs.push(doc.concat([before, part.operator.doc, after, part.operand]));
        } else if part.spaced {
            let line = doc.line();
            let space = doc.space();
            docs.push(doc.concat([line, part.operator.doc, space, part.operand]));
        } else {
            docs.push(doc.concat([part.operator.doc, part.operand]));
        }
    }
    let rest = doc.concat(docs);
    let rest = doc.indent(rest);
    let contents = doc.concat([first, rest]);
    doc.group(contents)
}

fn should_flatten_binary(
    parent_operator: &KotlinSyntaxToken<'_>,
    child_operator: &KotlinSyntaxToken<'_>,
) -> bool {
    if binary_operator_precedence(parent_operator) != binary_operator_precedence(child_operator) {
        return false;
    }
    if is_multiplicative_operator(parent_operator) && is_multiplicative_operator(child_operator) {
        return operators_equivalent(parent_operator, child_operator)
            && parent_operator.kind() != KotlinSyntaxKind::Percent;
    }
    true
}

fn operators_equivalent(left: &KotlinSyntaxToken<'_>, right: &KotlinSyntaxToken<'_>) -> bool {
    left.kind() == right.kind()
        && (left.kind() != KotlinSyntaxKind::Identifier || left.text() == right.text())
}

fn should_parenthesize_binary_operand(
    expression: &Expression<'_>,
    parent_operator: &KotlinSyntaxToken<'_>,
) -> bool {
    parent_operator.kind() == KotlinSyntaxKind::Identifier
        && matches!(expression, Expression::BinaryExpression(_))
}

fn binary_operator_precedence(operator: &KotlinSyntaxToken<'_>) -> Option<u8> {
    match operator.kind() {
        KotlinSyntaxKind::OrOr => Some(1),
        KotlinSyntaxKind::AndAnd => Some(2),
        KotlinSyntaxKind::EqEq
        | KotlinSyntaxKind::BangEq
        | KotlinSyntaxKind::EqEqEq
        | KotlinSyntaxKind::BangEqEqEq => Some(3),
        KotlinSyntaxKind::Lt
        | KotlinSyntaxKind::LtEq
        | KotlinSyntaxKind::Gt
        | KotlinSyntaxKind::GtEq
        | KotlinSyntaxKind::InKw
        | KotlinSyntaxKind::NotIn
        | KotlinSyntaxKind::IsKw
        | KotlinSyntaxKind::NotIs => Some(4),
        KotlinSyntaxKind::AsKw | KotlinSyntaxKind::AsSafe => Some(5),
        KotlinSyntaxKind::Elvis | KotlinSyntaxKind::Identifier => Some(6),
        KotlinSyntaxKind::Range | KotlinSyntaxKind::RangeUntil => Some(7),
        KotlinSyntaxKind::Plus | KotlinSyntaxKind::Minus => Some(8),
        KotlinSyntaxKind::Star
        | KotlinSyntaxKind::Slash
        | KotlinSyntaxKind::Percent
        | KotlinSyntaxKind::Amp => Some(9),
        _ => None,
    }
}

fn is_type_binary_operator(operator: &KotlinSyntaxToken<'_>) -> bool {
    matches!(
        operator.kind(),
        KotlinSyntaxKind::AsKw
            | KotlinSyntaxKind::AsSafe
            | KotlinSyntaxKind::IsKw
            | KotlinSyntaxKind::NotIs
    )
}

fn is_range_operator(operator: &KotlinSyntaxToken<'_>) -> bool {
    matches!(
        operator.kind(),
        KotlinSyntaxKind::Range | KotlinSyntaxKind::RangeUntil
    )
}

fn can_break_before_operator(operator: &KotlinSyntaxToken<'_>) -> bool {
    !matches!(
        operator.kind(),
        KotlinSyntaxKind::InKw
            | KotlinSyntaxKind::NotIn
            | KotlinSyntaxKind::IsKw
            | KotlinSyntaxKind::NotIs
            | KotlinSyntaxKind::AsKw
            | KotlinSyntaxKind::AsSafe
            | KotlinSyntaxKind::Identifier
    )
}

fn is_multiplicative_operator(operator: &KotlinSyntaxToken<'_>) -> bool {
    matches!(
        operator.kind(),
        KotlinSyntaxKind::Star | KotlinSyntaxKind::Slash | KotlinSyntaxKind::Percent
    )
}

pub(super) fn format_unary_expression<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &UnaryExpression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let operator = format_required_field(expression.operator(), doc, |operator, doc| {
        format_token(doc, &operator, leading, TrailingTrivia::Preserve)
    });
    let operand = format_required_field(expression.operand(), doc, |operand, doc| {
        format_expression_with_leading(doc, &operand, LeadingTrivia::Preserve)
    });
    doc.concat([operator, operand])
}

pub(super) fn format_postfix_expression<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &PostfixExpression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let operand = format_required_field(expression.operand(), doc, |operand, doc| {
        format_expression_with_leading(doc, &operand, leading)
    });
    let operator = format_required_field(expression.operator(), doc, |operator, doc| {
        format_token(
            doc,
            &operator,
            LeadingTrivia::Preserve,
            TrailingTrivia::Preserve,
        )
    });
    doc.concat([operand, operator])
}
