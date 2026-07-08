use jolt_fmt_ir::{Doc, concat, group, indent, line, space};
use jolt_kotlin_syntax::{
    AssignmentExpression, BinaryExpression, Expression, KotlinSyntaxKind, KotlinSyntaxToken,
    ParenthesizedExpression, PostfixExpression, UnaryExpression,
};

use crate::helpers::comments::{
    LeadingTrivia, TrailingTrivia, comment_forces_line, format_token, format_token_sequence,
    token_has_comments,
};
use crate::helpers::syntax_tokens::{FormatterInsertedToken, inserted_syntax_token};
use crate::rules::types::format_type_reference;

use super::{format_expression, format_expression_with_leading};

pub(super) fn format_parenthesized_expression<'source>(
    expression: &ParenthesizedExpression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    concat([
        expression
            .open_paren()
            .map_or_else(jolt_fmt_ir::nil, |open| {
                format_token(&open, leading, TrailingTrivia::RelocatedToEnclosingContext)
            }),
        expression
            .expression()
            .map_or_else(jolt_fmt_ir::nil, |inner| format_expression(&inner)),
        expression
            .close_paren()
            .map_or_else(jolt_fmt_ir::nil, |close| {
                format_token(&close, LeadingTrivia::Preserve, TrailingTrivia::Preserve)
            }),
    ])
}

pub(super) fn format_assignment_expression<'source>(
    expression: &AssignmentExpression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let Some(left) = expression.left() else {
        return format_recovered_assignment_without_left(expression, leading);
    };
    let Some(operator) = expression.operator_token() else {
        return format_expression_with_leading(&left, leading);
    };
    let Some(right) = expression.right() else {
        return concat([
            format_expression_with_leading(&left, leading),
            space(),
            format_token(&operator, LeadingTrivia::Preserve, TrailingTrivia::Preserve),
        ]);
    };

    group(concat([
        format_expression_with_leading(&left, leading),
        space(),
        format_token(&operator, LeadingTrivia::Preserve, TrailingTrivia::Preserve),
        indent(concat([line(), format_expression(&right)])),
    ]))
}

fn format_recovered_assignment_without_left<'source>(
    expression: &AssignmentExpression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    match (expression.operator_token(), expression.right()) {
        (Some(operator), Some(right)) => group(concat([
            format_token(&operator, leading, TrailingTrivia::Preserve),
            indent(concat([line(), format_expression(&right)])),
        ])),
        (Some(operator), None) => format_token(&operator, leading, TrailingTrivia::Preserve),
        (None, Some(right)) => format_expression_with_leading(&right, leading),
        (None, None) => format_token_sequence(expression.token_iter(), leading),
    }
}

pub(super) fn format_binary_expression<'source>(
    expression: &BinaryExpression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let Some(operator) = expression.operator_token() else {
        return expression
            .operands()
            .next()
            .map_or_else(jolt_fmt_ir::nil, |left| {
                format_expression_with_leading(&left, leading)
            });
    };
    if is_type_binary_operator(&operator) {
        return format_type_binary_expression(expression, &operator, leading);
    }

    let parent_operator = expression.operator_token();
    let (first, rest) = flatten_binary_expression(expression);
    let first = parent_operator.map_or_else(
        || format_expression_with_leading(&first, leading),
        |operator| format_binary_operand_with_leading(&first, &operator, leading),
    );
    binary_chain(first, rest)
}

fn format_type_binary_expression<'source>(
    expression: &BinaryExpression<'source>,
    operator: &KotlinSyntaxToken<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let Some(left) = expression.operands().next() else {
        return format_token_sequence(expression.token_iter(), leading);
    };
    let Some(ty) = expression.cast_type() else {
        return concat([
            format_expression_with_leading(&left, leading),
            space(),
            format_token(operator, LeadingTrivia::Preserve, TrailingTrivia::Preserve),
        ]);
    };

    let operator = format_binary_operator(operator);

    group(concat([
        format_expression_with_leading(&left, leading),
        space(),
        operator.doc,
        indent(concat([line(), format_type_reference(&ty)])),
    ]))
}

fn flatten_binary_expression<'source>(
    expression: &BinaryExpression<'source>,
) -> (Expression<'source>, Vec<BinaryChainPart<'source>>) {
    let Some(operator) = expression.operator_token() else {
        return (
            expression
                .operands()
                .next()
                .unwrap_or_else(|| Expression::from(*expression)),
            expression
                .operands()
                .nth(1)
                .map(|right| BinaryChainPart {
                    operator: BinaryOperatorDoc {
                        doc: jolt_fmt_ir::nil(),
                        forces_line_after: false,
                    },
                    operand: format_expression(&right),
                    spaced: true,
                    break_before_operator: true,
                })
                .into_iter()
                .collect(),
        );
    };
    let root = Expression::from(*expression);
    let mut operands = Vec::new();
    let mut operators = Vec::new();
    collect_binary_chain(root, &mut operands, &mut operators);
    if operators.len() + 1 != operands.len() {
        return unflattened_binary_expression(expression, &operator);
    }

    let mut operands = operands.into_iter();
    let first = operands.next().unwrap_or(root);
    let rest = operators
        .into_iter()
        .zip(operands)
        .map(|(operator, operand)| BinaryChainPart {
            operator: format_binary_operator(&operator),
            operand: format_binary_operand(&operand, &operator),
            spaced: !is_range_operator(&operator),
            break_before_operator: can_break_before_operator(&operator),
        })
        .collect();

    (first, rest)
}

fn unflattened_binary_expression<'source>(
    expression: &BinaryExpression<'source>,
    operator: &KotlinSyntaxToken<'source>,
) -> (Expression<'source>, Vec<BinaryChainPart<'source>>) {
    (
        expression
            .operands()
            .next()
            .unwrap_or_else(|| Expression::from(*expression)),
        expression
            .operands()
            .nth(1)
            .map(|right| BinaryChainPart {
                operator: format_binary_operator(operator),
                operand: format_binary_operand(&right, operator),
                spaced: !is_range_operator(operator),
                break_before_operator: can_break_before_operator(operator),
            })
            .into_iter()
            .collect(),
    )
}

fn format_binary_operand<'source>(
    expression: &Expression<'source>,
    parent_operator: &KotlinSyntaxToken<'_>,
) -> Doc<'source> {
    format_binary_operand_doc(format_expression(expression), expression, parent_operator)
}

fn format_binary_operand_with_leading<'source>(
    expression: &Expression<'source>,
    parent_operator: &KotlinSyntaxToken<'_>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    format_binary_operand_doc(
        format_expression_with_leading(expression, leading),
        expression,
        parent_operator,
    )
}

fn format_binary_operand_doc<'source>(
    doc: Doc<'source>,
    expression: &Expression<'source>,
    parent_operator: &KotlinSyntaxToken<'_>,
) -> Doc<'source> {
    if should_parenthesize_binary_operand(expression, parent_operator) {
        return group(concat([
            // Intentional synthesized token: readability parentheses preserve
            // parsed precedence while making infix operands clear.
            inserted_syntax_token("(", FormatterInsertedToken::PrecedenceParenthesis),
            indent(concat([jolt_fmt_ir::soft_line(), doc])),
            jolt_fmt_ir::soft_line(),
            // Intentional synthesized token: closes the formatter-owned
            // readability parenthesis above.
            inserted_syntax_token(")", FormatterInsertedToken::PrecedenceParenthesis),
        ]));
    }

    doc
}

fn collect_binary_chain<'source>(
    expression: Expression<'source>,
    operands: &mut Vec<Expression<'source>>,
    operators: &mut Vec<KotlinSyntaxToken<'source>>,
) {
    let Some(binary) = binary_for_chain(expression) else {
        operands.push(expression);
        return;
    };
    let Some(operator) = binary.operator_token() else {
        operands.push(expression);
        return;
    };

    if let Some(left) = binary.operands().next() {
        collect_binary_left(left, &operator, operands, operators);
    }
    operators.push(operator);
    if let Some(right) = binary.operands().nth(1) {
        operands.push(right);
    }
}

fn collect_binary_left<'source>(
    expression: Expression<'source>,
    parent_operator: &KotlinSyntaxToken<'_>,
    operands: &mut Vec<Expression<'source>>,
    operators: &mut Vec<KotlinSyntaxToken<'source>>,
) {
    let Some(binary) = binary_for_chain(expression) else {
        operands.push(expression);
        return;
    };
    let Some(operator) = binary.operator_token() else {
        operands.push(expression);
        return;
    };

    if !should_flatten_binary(parent_operator, &operator) {
        operands.push(expression);
        return;
    }

    collect_binary_chain(Expression::from(binary), operands, operators);
}

fn binary_for_chain(expression: Expression<'_>) -> Option<BinaryExpression<'_>> {
    let binary = match expression {
        Expression::BinaryExpression(binary) => binary,
        Expression::ParenthesizedExpression(parenthesized)
            if parenthesized
                .open_paren()
                .is_none_or(|token| !token_has_comments(&token))
                && parenthesized
                    .close_paren()
                    .is_none_or(|token| !token_has_comments(&token)) =>
        {
            match parenthesized.expression() {
                Some(Expression::BinaryExpression(binary)) => binary,
                _ => return None,
            }
        }
        _ => return None,
    };
    let operator = binary.operator_token()?;
    if is_type_binary_operator(&operator) {
        return None;
    }
    binary.operands().nth(1)?;
    Some(binary)
}

struct BinaryOperatorDoc<'source> {
    doc: Doc<'source>,
    forces_line_after: bool,
}

fn format_binary_operator<'source>(
    operator: &KotlinSyntaxToken<'source>,
) -> BinaryOperatorDoc<'source> {
    let forces_line_after = operator
        .trailing_comments()
        .any(|comment| comment_forces_line(&comment));
    if matches!(
        operator.kind(),
        KotlinSyntaxKind::Range | KotlinSyntaxKind::RangeUntil
    ) {
        return BinaryOperatorDoc {
            doc: format_token(operator, LeadingTrivia::Preserve, TrailingTrivia::Preserve),
            forces_line_after,
        };
    }

    BinaryOperatorDoc {
        doc: format_token(
            operator,
            LeadingTrivia::Preserve,
            TrailingTrivia::BeforeLineBreak,
        ),
        forces_line_after,
    }
}

struct BinaryChainPart<'source> {
    operator: BinaryOperatorDoc<'source>,
    operand: Doc<'source>,
    spaced: bool,
    break_before_operator: bool,
}

fn binary_chain<'source>(first: Doc<'source>, rest: Vec<BinaryChainPart<'source>>) -> Doc<'source> {
    if rest.is_empty() {
        return first;
    }

    group(concat([
        first,
        indent(concat(rest.into_iter().map(|part| {
            if !part.break_before_operator {
                concat([
                    space(),
                    part.operator.doc,
                    indent(concat([line(), part.operand])),
                ])
            } else if part.operator.forces_line_after {
                concat([line(), part.operator.doc, line(), part.operand])
            } else if part.spaced {
                concat([line(), part.operator.doc, space(), part.operand])
            } else {
                concat([part.operator.doc, part.operand])
            }
        }))),
    ]))
}

fn should_flatten_binary(
    parent_operator: &KotlinSyntaxToken<'_>,
    child_operator: &KotlinSyntaxToken<'_>,
) -> bool {
    let Some(parent_precedence) = binary_operator_precedence(parent_operator) else {
        return false;
    };
    let Some(child_precedence) = binary_operator_precedence(child_operator) else {
        return false;
    };
    if parent_precedence != child_precedence {
        return false;
    }

    if is_multiplicative_operator(parent_operator) && is_multiplicative_operator(child_operator) {
        return parent_operator.text() == child_operator.text()
            && parent_operator.kind() != KotlinSyntaxKind::Percent
            && child_operator.kind() != KotlinSyntaxKind::Percent;
    }

    true
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
    expression: &UnaryExpression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let Some(operator) = expression.operator_token() else {
        return expression
            .operand()
            .map_or_else(jolt_fmt_ir::nil, |operand| format_expression(&operand));
    };
    let Some(operand) = expression.operand() else {
        return format_token_sequence(expression.token_iter(), leading);
    };

    concat([
        format_token(&operator, leading, TrailingTrivia::Preserve),
        format_expression(&operand),
    ])
}

pub(super) fn format_postfix_expression<'source>(
    expression: &PostfixExpression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let Some(operand) = expression.operand() else {
        return format_token_sequence(expression.token_iter(), leading);
    };
    let Some(operator) = expression.operator_token() else {
        return format_expression_with_leading(&operand, leading);
    };

    concat([
        format_expression_with_leading(&operand, leading),
        format_token(&operator, LeadingTrivia::Preserve, TrailingTrivia::Preserve),
    ])
}
