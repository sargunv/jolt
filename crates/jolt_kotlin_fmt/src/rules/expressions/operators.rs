use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_kotlin_syntax::{
    AssignmentExpression, BinaryExpression, BinaryExpressionRightSyntax,
    BinaryExpressionRightValue, BinaryOperatorSyntax, Expression, KotlinFamily, KotlinSyntaxField,
    KotlinSyntaxKind, KotlinSyntaxToken, KotlinSyntaxView, ParenthesizedExpression,
    PostfixExpression, UnaryExpression,
};

use crate::helpers::comments::{LeadingTrivia, TrailingTrivia, comment_forces_line, format_token};
use crate::helpers::recovery::format_required_field;
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
    let Some(outer) = expression.syntax_node() else {
        doc.block_on_invariant("Kotlin binary expression has no syntax node");
        return Doc::nil();
    };
    let mut current = Expression::from(*expression);

    while let Expression::BinaryExpression(binary) = current {
        let Some(left) = present(binary.left()) else {
            break;
        };
        current = left;
    }

    let Some(mut current_node) = current.syntax_node() else {
        doc.block_on_invariant("Kotlin binary base has no syntax node");
        return Doc::nil();
    };
    let base = match current {
        Expression::BinaryExpression(binary) => format_binary_fields(doc, &binary, leading),
        _ => format_expression_with_leading(doc, &current, leading),
    };
    let mut formatted = base;
    let mut run = None;

    while current_node != outer {
        let Some(parent_node) = current_node.parent() else {
            doc.block_on_invariant("binary left spine ended before its outer expression");
            break;
        };
        let Some(Expression::BinaryExpression(binary)) = Expression::cast(parent_node) else {
            doc.block_on_invariant("binary left spine crossed a non-binary parent");
            break;
        };
        let operator = binary_operator(&binary);
        let right = present(binary.right());
        match (operator, right) {
            (Some(operator), Some(right)) if is_type_binary_operator(&operator) => {
                let left = finish_pending_binary_run(doc, formatted, run.take());
                let operator = format_binary_operator(doc, &operator).doc;
                let right = format_binary_right(doc, &right);
                let space = doc.space();
                let line = doc.line();
                let right = doc.concat([line, right]);
                let right = doc.indent(right);
                let contents = doc.concat([left, space, operator, right]);
                formatted = doc.group(contents);
            }
            (Some(operator), Some(right)) => {
                let Some(right) = binary_right_expression(&right) else {
                    let left = finish_pending_binary_run(doc, formatted, run.take());
                    formatted = format_binary_fields_with_left(doc, &binary, left);
                    current = Expression::from(binary);
                    current_node = parent_node;
                    continue;
                };
                if let Some(run) = run.as_mut().filter(|run| {
                    run.parts.last().is_some_and(|(root_operator, _)| {
                        should_flatten_binary(&operator, root_operator)
                    })
                }) {
                    run.owner = binary;
                    run.parts.push((operator, right));
                } else {
                    formatted = finish_pending_binary_run(doc, formatted, run.take());
                    let mut parts = Vec::with_capacity(4);
                    parts.push((operator, right));
                    run = Some(PendingBinaryRun {
                        owner: binary,
                        first_operand: current,
                        parts,
                    });
                }
            }
            _ => {
                let left = finish_pending_binary_run(doc, formatted, run.take());
                formatted = format_binary_fields_with_left(doc, &binary, left);
            }
        }

        current = Expression::from(binary);
        current_node = parent_node;
    }

    finish_pending_binary_run(doc, formatted, run)
}

struct PendingBinaryRun<'source> {
    owner: BinaryExpression<'source>,
    first_operand: Expression<'source>,
    parts: Vec<(KotlinSyntaxToken<'source>, Expression<'source>)>,
}

fn finish_pending_binary_run<'source>(
    doc: &mut DocBuilder<'source>,
    formatted: Doc<'source>,
    run: Option<PendingBinaryRun<'source>>,
) -> Doc<'source> {
    let Some(run) = run else {
        return formatted;
    };
    let Some((root_operator, _)) = run.parts.last() else {
        doc.block_on_invariant("binary run has no operator");
        return formatted;
    };
    let first = format_binary_operand_doc(
        doc,
        formatted,
        &run.owner,
        &run.first_operand,
        root_operator,
    );
    let keep_infix_chain_flat = run.parts.len() > 1
        && run
            .parts
            .iter()
            .all(|(operator, _)| operator.kind() == KotlinSyntaxKind::Identifier);
    let rest = doc.concat_list(|docs| {
        for (operator, operand) in run.parts {
            let operand = format_binary_operand(docs, &run.owner, &operand, &operator);
            let spaced = !is_range_operator(&operator);
            let break_before_operator = can_break_before_operator(&operator);
            let operator = format_binary_operator(docs, &operator);
            let part = if !break_before_operator {
                let space = docs.space();
                let line = if keep_infix_chain_flat {
                    docs.space()
                } else {
                    docs.line()
                };
                let operand = docs.concat([line, operand]);
                let operand = docs.indent(operand);
                docs.concat([space, operator.doc, operand])
            } else if operator.forces_line_after {
                let before = docs.line();
                let after = docs.line();
                docs.concat([before, operator.doc, after, operand])
            } else if spaced {
                let line = docs.line();
                let space = docs.space();
                docs.concat([line, operator.doc, space, operand])
            } else {
                docs.concat([operator.doc, operand])
            };
            docs.push(part);
        }
    });
    let rest = doc.indent(rest);
    let contents = doc.concat([first, rest]);
    doc.group(contents)
}

fn format_binary_fields<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &BinaryExpression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let left = format_required_field(expression.left(), doc, |left, doc| {
        format_expression_with_leading(doc, &left, leading)
    });
    format_binary_fields_with_left(doc, expression, left)
}

fn format_binary_fields_with_left<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &BinaryExpression<'source>,
    left: Doc<'source>,
) -> Doc<'source> {
    let has_right = matches!(
        expression.right(),
        KotlinSyntaxField::Present(ref right) if right.first_token().is_some()
    ) || matches!(
        expression.right(),
        KotlinSyntaxField::Malformed(ref malformed) if malformed.first_token().is_some()
    );
    let operator = format_required_field(expression.operator(), doc, |operator, doc| {
        let operator = match operator.classify() {
            Ok(
                BinaryOperatorSyntax::Operator(operator)
                | BinaryOperatorSyntax::InfixFunction(operator),
            ) => operator,
            Err(error) => {
                doc.block_on_invariant(error.to_string());
                return Doc::nil();
            }
        };
        format_token(
            doc,
            &operator,
            LeadingTrivia::Preserve,
            TrailingTrivia::Preserve,
        )
    });
    let right = format_required_field(expression.right(), doc, |right, doc| {
        format_binary_right(doc, &right)
    });
    let space = doc.space();
    let line = if has_right { doc.line() } else { Doc::nil() };
    let right = doc.concat([line, right]);
    let right = doc.indent(right);
    let contents = doc.concat([left, space, operator, right]);
    doc.group(contents)
}

fn format_binary_right<'source>(
    doc: &mut DocBuilder<'source>,
    right: &BinaryExpressionRightValue<'source>,
) -> Doc<'source> {
    match (*right).classify() {
        Ok(BinaryExpressionRightSyntax::Expression(expression)) => {
            format_expression(doc, &expression)
        }
        Ok(BinaryExpressionRightSyntax::TypeReference(ty)) => format_type_reference(doc, &ty),
        Err(error) => {
            doc.block_on_invariant(error.to_string());
            Doc::nil()
        }
    }
}

fn binary_right_expression<'source>(
    right: &BinaryExpressionRightValue<'source>,
) -> Option<Expression<'source>> {
    match (*right).classify().ok()? {
        BinaryExpressionRightSyntax::Expression(expression) => Some(expression),
        BinaryExpressionRightSyntax::TypeReference(_) => None,
    }
}

fn binary_operator<'source>(
    expression: &BinaryExpression<'source>,
) -> Option<KotlinSyntaxToken<'source>> {
    match present(expression.operator())?.classify().ok()? {
        BinaryOperatorSyntax::Operator(operator)
        | BinaryOperatorSyntax::InfixFunction(operator) => Some(operator),
    }
}

fn present<T>(field: KotlinSyntaxField<'_, T>) -> Option<T> {
    match field {
        KotlinSyntaxField::Present(value) => Some(value),
        KotlinSyntaxField::Missing(_) | KotlinSyntaxField::Malformed(_) => None,
    }
}

fn format_binary_operand<'source>(
    doc: &mut DocBuilder<'source>,
    owner: &BinaryExpression<'source>,
    expression: &Expression<'source>,
    parent_operator: &KotlinSyntaxToken<'_>,
) -> Doc<'source> {
    let formatted = format_expression(doc, expression);
    format_binary_operand_doc(doc, formatted, owner, expression, parent_operator)
}

fn format_binary_operand_doc<'source>(
    doc: &mut DocBuilder<'source>,
    formatted: Doc<'source>,
    owner: &BinaryExpression<'source>,
    expression: &Expression<'source>,
    parent_operator: &KotlinSyntaxToken<'_>,
) -> Doc<'source> {
    if !should_parenthesize_binary_operand(expression, parent_operator) {
        return formatted;
    }
    let Some(claims) = owner.precedence_parenthesis_claims(expression) else {
        return formatted;
    };
    let open = doc.synthesized_source(claims.open);
    let line = doc.soft_line();
    let formatted = doc.concat([line, formatted]);
    let formatted = doc.indent(formatted);
    let trailing = doc.soft_line();
    let close = doc.synthesized_source(claims.close);
    let contents = doc.concat([open, formatted, trailing, close]);
    doc.group(contents)
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
    operand: Option<Doc<'source>>,
) -> Doc<'source> {
    let operand = operand.unwrap_or_else(|| {
        format_required_field(expression.operand(), doc, |operand, doc| {
            format_expression_with_leading(doc, &operand, leading)
        })
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
