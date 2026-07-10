use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_kotlin_syntax::{
    AssignmentExpression, BinaryExpression, Expression, KotlinSyntaxKind, KotlinSyntaxToken,
    ParenthesizedExpression, PostfixExpression, UnaryExpression, operators_equivalent,
};

use crate::helpers::comments::{
    LeadingTrivia, TrailingTrivia, comment_forces_line, format_token, format_token_sequence,
    token_has_comments,
};
use crate::helpers::syntax_tokens::{FormatterInsertedToken, inserted_syntax_token};
use crate::rules::types::format_type_reference;

use super::{format_expression, format_expression_with_leading};

pub(super) fn format_parenthesized_expression<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &ParenthesizedExpression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let open = if let Some(open) = expression.open_paren() {
        format_token(
            doc,
            &open,
            leading,
            TrailingTrivia::RelocatedToEnclosingContext,
        )
    } else {
        doc.nil()
    };
    let inner = if let Some(inner) = expression.expression() {
        format_expression(doc, &inner)
    } else {
        doc.nil()
    };
    let close = if let Some(close) = expression.close_paren() {
        format_token(
            doc,
            &close,
            LeadingTrivia::Preserve,
            TrailingTrivia::Preserve,
        )
    } else {
        doc.nil()
    };
    doc.concat([open, inner, close])
}

pub(super) fn format_assignment_expression<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &AssignmentExpression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let Some(left) = expression.left() else {
        return format_recovered_assignment_without_left(doc, expression, leading);
    };
    let Some(operator) = expression.operator_token() else {
        return format_expression_with_leading(doc, &left, leading);
    };
    let Some(right) = expression.right() else {
        let left = format_expression_with_leading(doc, &left, leading);
        let space = doc.space();
        let operator = format_token(
            doc,
            &operator,
            LeadingTrivia::Preserve,
            TrailingTrivia::Preserve,
        );
        return doc.concat([left, space, operator]);
    };

    let left = format_expression_with_leading(doc, &left, leading);
    let space = doc.space();
    let operator = format_token(
        doc,
        &operator,
        LeadingTrivia::Preserve,
        TrailingTrivia::Preserve,
    );
    let line = doc.line();
    let right = format_expression(doc, &right);
    let right = doc.concat([line, right]);
    let right = doc.indent(right);
    let contents = doc.concat([left, space, operator, right]);
    doc.group(contents)
}

fn format_recovered_assignment_without_left<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &AssignmentExpression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    match (expression.operator_token(), expression.right()) {
        (Some(operator), Some(right)) => {
            let operator = format_token(doc, &operator, leading, TrailingTrivia::Preserve);
            let line = doc.line();
            let right = format_expression(doc, &right);
            let right = doc.concat([line, right]);
            let right = doc.indent(right);
            let contents = doc.concat([operator, right]);
            doc.group(contents)
        }
        (Some(operator), None) => format_token(doc, &operator, leading, TrailingTrivia::Preserve),
        (None, Some(right)) => format_expression_with_leading(doc, &right, leading),
        (None, None) => format_token_sequence(doc, expression.token_iter(), leading),
    }
}

pub(super) fn format_binary_expression<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &BinaryExpression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let Some(operator) = expression.operator_token() else {
        return if let Some(left) = expression.operands().next() {
            format_expression_with_leading(doc, &left, leading)
        } else {
            doc.nil()
        };
    };
    if expression.operands().nth(1).is_none() && expression.cast_type().is_none() {
        let left = expression.operands().next().map_or_else(Doc::nil, |left| {
            format_expression_with_leading(doc, &left, leading)
        });
        let space = doc.space();
        let operator = format_token(
            doc,
            &operator,
            LeadingTrivia::Preserve,
            TrailingTrivia::Preserve,
        );
        return doc.concat([left, space, operator]);
    }
    if is_type_binary_operator(doc, &operator) {
        return format_type_binary_expression(doc, expression, &operator, leading);
    }

    let parent_operator = expression.operator_token();
    let (first, rest) = flatten_binary_expression(doc, expression);
    let first = if let Some(operator) = parent_operator {
        format_binary_operand_with_leading(doc, &first, &operator, leading)
    } else {
        format_expression_with_leading(doc, &first, leading)
    };
    binary_chain(doc, first, rest)
}

fn format_type_binary_expression<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &BinaryExpression<'source>,
    operator: &KotlinSyntaxToken<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let Some(left) = expression.operands().next() else {
        return format_token_sequence(doc, expression.token_iter(), leading);
    };
    let Some(ty) = expression.cast_type() else {
        let left = format_expression_with_leading(doc, &left, leading);
        let space = doc.space();
        let operator = format_token(
            doc,
            operator,
            LeadingTrivia::Preserve,
            TrailingTrivia::Preserve,
        );
        return doc.concat([left, space, operator]);
    };

    let operator = format_binary_operator(doc, operator);

    let left = format_expression_with_leading(doc, &left, leading);
    let space = doc.space();
    let line = doc.line();
    let ty = format_type_reference(doc, &ty);
    let ty = doc.concat([line, ty]);
    let ty = doc.indent(ty);
    let contents = doc.concat([left, space, operator.doc, ty]);
    doc.group(contents)
}

fn flatten_binary_expression<'source>(
    doc: &mut DocBuilder<'source>,
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
                        doc: doc.nil(),
                        forces_line_after: false,
                    },
                    operand: format_expression(doc, &right),
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
    collect_binary_chain(doc, root, &mut operands, &mut operators);
    if operators.len() + 1 != operands.len() {
        return unflattened_binary_expression(doc, expression, &operator);
    }

    let mut operands = operands.into_iter();
    let first = operands.next().unwrap_or(root);
    let rest = operators
        .into_iter()
        .zip(operands)
        .map(|(operator, operand)| BinaryChainPart {
            operator: format_binary_operator(doc, &operator),
            operand: format_binary_operand(doc, &operand, &operator),
            spaced: !is_range_operator(doc, &operator),
            break_before_operator: can_break_before_operator(doc, &operator),
        })
        .collect();

    (first, rest)
}

fn unflattened_binary_expression<'source>(
    doc: &mut DocBuilder<'source>,
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
                operator: format_binary_operator(doc, operator),
                operand: format_binary_operand(doc, &right, operator),
                spaced: !is_range_operator(doc, operator),
                break_before_operator: can_break_before_operator(doc, operator),
            })
            .into_iter()
            .collect(),
    )
}

fn format_binary_operand<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &Expression<'source>,
    parent_operator: &KotlinSyntaxToken<'_>,
) -> Doc<'source> {
    let expression_doc = format_expression(doc, expression);
    format_binary_operand_doc(doc, expression_doc, expression, parent_operator)
}

fn format_binary_operand_with_leading<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &Expression<'source>,
    parent_operator: &KotlinSyntaxToken<'_>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let expression_doc = format_expression_with_leading(doc, expression, leading);
    format_binary_operand_doc(doc, expression_doc, expression, parent_operator)
}

fn format_binary_operand_doc<'source>(
    doc: &mut DocBuilder<'source>,
    operand_doc: Doc<'source>,
    expression: &Expression<'source>,
    parent_operator: &KotlinSyntaxToken<'_>,
) -> Doc<'source> {
    if should_parenthesize_binary_operand(doc, expression, parent_operator) {
        let open = inserted_syntax_token(doc, "(", FormatterInsertedToken::PrecedenceParenthesis);
        let soft_line = doc.soft_line();
        let operand_doc = doc.concat([soft_line, operand_doc]);
        let operand_doc = doc.indent(operand_doc);
        let trailing = doc.soft_line();
        let close = inserted_syntax_token(doc, ")", FormatterInsertedToken::PrecedenceParenthesis);
        let contents = doc.concat([open, operand_doc, trailing, close]);
        return doc.group(contents);
    }

    operand_doc
}

fn collect_binary_chain<'source>(
    doc: &mut DocBuilder<'source>,
    expression: Expression<'source>,
    operands: &mut Vec<Expression<'source>>,
    operators: &mut Vec<KotlinSyntaxToken<'source>>,
) {
    let Some(binary) = binary_for_chain(doc, expression) else {
        operands.push(expression);
        return;
    };
    let Some(operator) = binary.operator_token() else {
        operands.push(expression);
        return;
    };

    if let Some(left) = binary.operands().next() {
        collect_binary_left(doc, left, &operator, operands, operators);
    }
    operators.push(operator);
    if let Some(right) = binary.operands().nth(1) {
        operands.push(right);
    }
}

fn collect_binary_left<'source>(
    doc: &mut DocBuilder<'source>,
    expression: Expression<'source>,
    parent_operator: &KotlinSyntaxToken<'_>,
    operands: &mut Vec<Expression<'source>>,
    operators: &mut Vec<KotlinSyntaxToken<'source>>,
) {
    let Some(binary) = binary_for_chain(doc, expression) else {
        operands.push(expression);
        return;
    };
    let Some(operator) = binary.operator_token() else {
        operands.push(expression);
        return;
    };

    if !should_flatten_binary(doc, parent_operator, &operator) {
        operands.push(expression);
        return;
    }

    collect_binary_chain(doc, Expression::from(binary), operands, operators);
}

fn binary_for_chain<'source>(
    doc: &mut DocBuilder<'_>,
    expression: Expression<'source>,
) -> Option<BinaryExpression<'source>> {
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
    if is_type_binary_operator(doc, &operator) {
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
    doc: &mut DocBuilder<'source>,
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
            doc: format_token(
                doc,
                operator,
                LeadingTrivia::Preserve,
                TrailingTrivia::Preserve,
            ),
            forces_line_after,
        };
    }

    BinaryOperatorDoc {
        doc: format_token(
            doc,
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

fn binary_chain<'source>(
    doc: &mut DocBuilder<'source>,
    first: Doc<'source>,
    rest: Vec<BinaryChainPart<'source>>,
) -> Doc<'source> {
    if rest.is_empty() {
        return first;
    }

    let rest = rest
        .into_iter()
        .map(|part| {
            if !part.break_before_operator {
                let space = doc.space();
                let line = doc.line();
                let operand = doc.concat([line, part.operand]);
                let operand = doc.indent(operand);
                doc.concat([space, part.operator.doc, operand])
            } else if part.operator.forces_line_after {
                let before = doc.line();
                let after = doc.line();
                doc.concat([before, part.operator.doc, after, part.operand])
            } else if part.spaced {
                let line = doc.line();
                let space = doc.space();
                doc.concat([line, part.operator.doc, space, part.operand])
            } else {
                doc.concat([part.operator.doc, part.operand])
            }
        })
        .collect::<Vec<_>>();
    let rest = doc.concat(rest);
    let rest = doc.indent(rest);
    let contents = doc.concat([first, rest]);
    doc.group(contents)
}

fn should_flatten_binary(
    doc: &mut DocBuilder<'_>,
    parent_operator: &KotlinSyntaxToken<'_>,
    child_operator: &KotlinSyntaxToken<'_>,
) -> bool {
    let Some(parent_precedence) = binary_operator_precedence(doc, parent_operator) else {
        return false;
    };
    let Some(child_precedence) = binary_operator_precedence(doc, child_operator) else {
        return false;
    };
    if parent_precedence != child_precedence {
        return false;
    }

    if is_multiplicative_operator(doc, parent_operator)
        && is_multiplicative_operator(doc, child_operator)
    {
        return operators_equivalent(parent_operator, child_operator)
            && parent_operator.kind() != KotlinSyntaxKind::Percent
            && child_operator.kind() != KotlinSyntaxKind::Percent;
    }

    true
}

fn should_parenthesize_binary_operand(
    _doc: &mut DocBuilder<'_>,
    expression: &Expression<'_>,
    parent_operator: &KotlinSyntaxToken<'_>,
) -> bool {
    parent_operator.kind() == KotlinSyntaxKind::Identifier
        && matches!(expression, Expression::BinaryExpression(_))
}

fn binary_operator_precedence(
    _doc: &mut DocBuilder<'_>,
    operator: &KotlinSyntaxToken<'_>,
) -> Option<u8> {
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

fn is_type_binary_operator(_doc: &mut DocBuilder<'_>, operator: &KotlinSyntaxToken<'_>) -> bool {
    matches!(
        operator.kind(),
        KotlinSyntaxKind::AsKw
            | KotlinSyntaxKind::AsSafe
            | KotlinSyntaxKind::IsKw
            | KotlinSyntaxKind::NotIs
    )
}

fn is_range_operator(_doc: &mut DocBuilder<'_>, operator: &KotlinSyntaxToken<'_>) -> bool {
    matches!(
        operator.kind(),
        KotlinSyntaxKind::Range | KotlinSyntaxKind::RangeUntil
    )
}

fn can_break_before_operator(_doc: &mut DocBuilder<'_>, operator: &KotlinSyntaxToken<'_>) -> bool {
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

fn is_multiplicative_operator(_doc: &mut DocBuilder<'_>, operator: &KotlinSyntaxToken<'_>) -> bool {
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
    let Some(operator) = expression.operator_token() else {
        return if let Some(operand) = expression.operand() {
            format_expression(doc, &operand)
        } else {
            doc.nil()
        };
    };
    let Some(operand) = expression.operand() else {
        return format_token_sequence(doc, expression.token_iter(), leading);
    };

    let operator = format_token(doc, &operator, leading, TrailingTrivia::Preserve);
    let operand = format_expression_with_leading(doc, &operand, LeadingTrivia::Preserve);
    doc.concat([operator, operand])
}

pub(super) fn format_postfix_expression<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &PostfixExpression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let Some(operand) = expression.operand() else {
        return format_token_sequence(doc, expression.token_iter(), leading);
    };
    let Some(operator) = expression.operator_token() else {
        return format_expression_with_leading(doc, &operand, leading);
    };

    let operand = format_expression_with_leading(doc, &operand, leading);
    let operator = format_token(
        doc,
        &operator,
        LeadingTrivia::Preserve,
        TrailingTrivia::Preserve,
    );
    doc.concat([operand, operator])
}
