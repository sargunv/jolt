use super::{
    AssignmentExpression, BinaryExpression, ConditionalExpression, Doc, Expression,
    PostfixExpression, UnaryExpression, casts_patterns::format_instanceof_expression,
    format_expression, format_token_with_comments,
};
use crate::helpers::comments::token_has_comments;
use crate::helpers::recovery::format_required_field;
use jolt_fmt_ir::DocBuilder;
use jolt_java_syntax::{
    AssignmentTargetSyntax, ExpressionParentRole, JavaFamily, JavaNode, JavaOperator,
    JavaOperatorKind, JavaSyntaxField, JavaSyntaxNode, JavaSyntaxView, ParenthesizedExpression,
    binary_operator_precedence, is_bitwise_or_shift_operator, is_multiplicative_operator,
    is_shift_operator,
};

pub(super) fn format_assignment_expression<'source>(
    expression: &AssignmentExpression<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let left = format_required_field(expression.left(), doc, |left, doc| match left {
        AssignmentTargetSyntax::NameExpression(left) => format_expression(&left.into(), doc),
        AssignmentTargetSyntax::FieldAccessExpression(left) => format_expression(&left.into(), doc),
        AssignmentTargetSyntax::ArrayAccessExpression(left) => format_expression(&left.into(), doc),
        AssignmentTargetSyntax::BogusExpression(left) => format_expression(&left.into(), doc),
        AssignmentTargetSyntax::BogusAssignmentTarget(left) => {
            crate::helpers::recovery::format_malformed(&left, doc)
        }
    });
    let operator = format_required_field(expression.operator(), doc, |operator, doc| {
        let operator = match operator.as_operator() {
            Ok(operator) => operator,
            Err(error) => {
                doc.block_on_invariant(error.to_string());
                return Doc::nil();
            }
        };
        format_operator_with_comments(&operator, doc)
    });
    let right = format_required_field(expression.right(), doc, |right, doc| {
        format_expression(&right, doc)
    });

    assignment_expression(doc, left, operator, right)
}

pub(super) fn format_conditional_expression<'source>(
    expression: &ConditionalExpression<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let condition = format_required_field(expression.condition(), doc, |condition, doc| {
        format_expression(&condition, doc)
    });
    let question = format_required_field(expression.question(), doc, |token, doc| {
        format_token_with_comments(doc, &token)
    });
    let consequence = format_required_field(expression.then_expression(), doc, |value, doc| {
        format_expression(&value, doc)
    });
    let colon = format_required_field(expression.colon(), doc, |token, doc| {
        format_token_with_comments(doc, &token)
    });
    let alternative = format_required_field(expression.else_expression(), doc, |value, doc| {
        format_expression(&value, doc)
    });

    ternary_expression(
        doc,
        condition,
        question,
        consequence,
        colon,
        alternative,
        should_force_conditional_break(expression),
    )
}

pub(super) fn format_operator_spine<'source>(
    expression: Expression<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let Some(outer) = expression.syntax_node() else {
        doc.block_on_invariant("Java operator expression has no syntax node");
        return Doc::nil();
    };
    let mut current = expression;
    while let Some(inner) = operator_inner(current) {
        current = inner;
    }

    let Some(mut current_node) = current.syntax_node() else {
        doc.block_on_invariant("Java operator base has no syntax node");
        return Doc::nil();
    };
    let mut formatted = format_operator_base(&current, doc);
    let mut run: Option<PendingBinaryRun<'source>> = None;
    while current_node != outer {
        let Some((parent, parent_node, parentheses)) = operator_parent(current_node) else {
            doc.block_on_invariant("Java operator spine crossed an unexpected parent");
            break;
        };
        match parent {
            Expression::BinaryExpression(binary) => {
                let operator = binary_operator(&binary);
                let right = present(binary.right());
                if binary.is_recovery_free()
                    && let (Some(operator), Some(right)) = (operator, right)
                {
                    if let Some(run) = run.as_mut().filter(|run| {
                        run.parts.last().is_some_and(|(root_operator, _)| {
                            should_flatten_binary(operator.kind(), root_operator.kind())
                        })
                    }) {
                        run.owner = binary;
                        run.parts.push((operator, right));
                        run.removed_parentheses.extend(parentheses);
                    } else {
                        formatted = finish_pending_binary_run(formatted, run.take(), doc);
                        let mut parts = Vec::new();
                        let mut removed_parentheses = Vec::new();
                        parts.push((operator, right));
                        removed_parentheses.extend(parentheses);
                        run = Some(PendingBinaryRun {
                            owner: binary,
                            first_operand: current,
                            parts,
                            removed_parentheses,
                        });
                    }
                } else {
                    formatted = finish_pending_binary_run(formatted, run.take(), doc);
                    formatted = format_binary_fields(&binary, Some(formatted), doc);
                }
            }
            Expression::InstanceofExpression(instanceof) => {
                formatted = finish_pending_binary_run(formatted, run.take(), doc);
                formatted = format_instanceof_expression(&instanceof, Some(formatted), doc);
            }
            _ => {
                doc.block_on_invariant("Java operator parent was not binary or instanceof");
                break;
            }
        }
        current = parent;
        current_node = parent_node;
    }
    finish_pending_binary_run(formatted, run, doc)
}

fn format_binary_fields<'source>(
    expression: &BinaryExpression<'source>,
    left: Option<Doc<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let left = left.unwrap_or_else(|| {
        format_required_field(expression.left(), doc, |left, doc| {
            format_expression(&left, doc)
        })
    });
    let operator = format_required_field(expression.operator(), doc, |operator, doc| {
        let operator = match operator.as_operator() {
            Ok(operator) => operator,
            Err(error) => {
                doc.block_on_invariant(error.to_string());
                return Doc::nil();
            }
        };
        format_operator_with_comments(&operator, doc)
    });
    let right = format_required_field(expression.right(), doc, |right, doc| {
        format_expression(&right, doc)
    });
    doc_concat!(doc, [left, doc.space(), operator, doc.space(), right])
}

fn operator_inner(expression: Expression<'_>) -> Option<Expression<'_>> {
    match expression {
        Expression::BinaryExpression(binary) => {
            let left = present(binary.left())?;
            if let Expression::ParenthesizedExpression(parenthesized) = left
                && let Some(binary) = transparent_binary_left(&binary, &parenthesized)
            {
                return Some(Expression::BinaryExpression(binary));
            }
            Some(left)
        }
        Expression::InstanceofExpression(instanceof) => present(instanceof.expression()),
        _ => None,
    }
}

fn transparent_binary_left<'source>(
    parent: &BinaryExpression<'source>,
    parenthesized: &ParenthesizedExpression<'source>,
) -> Option<BinaryExpression<'source>> {
    if !parent.is_recovery_free()
        || !matches!(parenthesized.open_paren(), JavaSyntaxField::Present(token) if !token_has_comments(&token))
        || !matches!(parenthesized.close_paren(), JavaSyntaxField::Present(token) if !token_has_comments(&token))
    {
        return None;
    }
    let Expression::BinaryExpression(child) = present(parenthesized.expression())? else {
        return None;
    };
    should_flatten_binary(
        binary_operator(parent)?.kind(),
        binary_operator(&child)?.kind(),
    )
    .then_some(child)
}

fn operator_parent(
    current: JavaSyntaxNode<'_>,
) -> Option<(
    Expression<'_>,
    JavaSyntaxNode<'_>,
    Option<ParenthesizedExpression<'_>>,
)> {
    let parent = current.parent()?;
    if let Some(parenthesized) = ParenthesizedExpression::cast(parent) {
        let owner = parent.parent()?;
        return Some((
            Expression::BinaryExpression(BinaryExpression::cast(owner)?),
            owner,
            Some(parenthesized),
        ));
    }
    Some((Expression::cast(parent)?, parent, None))
}

fn format_operator_base<'source>(
    expression: &Expression<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    match expression {
        Expression::BinaryExpression(binary) => format_binary_fields(binary, None, doc),
        Expression::InstanceofExpression(instanceof) => {
            format_instanceof_expression(instanceof, None, doc)
        }
        _ => format_expression(expression, doc),
    }
}

pub(super) fn format_unary_expression<'source>(
    expression: &UnaryExpression<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let needs_space = present(expression.operator())
        .zip(present(expression.operand()).and_then(|operand| operand.first_token()))
        .is_some_and(|(operator, operand)| {
            crate::helpers::lexical_safety::structured_tokens_need_space(&operator, &operand)
        });
    let operator = format_required_field(expression.operator(), doc, |operator, doc| {
        format_token_with_comments(doc, &operator)
    });
    let operand = format_required_field(expression.operand(), doc, |operand, doc| {
        format_expression(&operand, doc)
    });
    let separator = if needs_space { doc.space() } else { Doc::nil() };
    doc_concat!(doc, [operator, separator, operand])
}

pub(super) fn format_postfix_expression<'source>(
    expression: &PostfixExpression<'source>,
    operand: Option<Doc<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let operand = operand.unwrap_or_else(|| {
        format_required_field(expression.operand(), doc, |operand, doc| {
            format_expression(&operand, doc)
        })
    });
    let operator = format_required_field(expression.operator(), doc, |operator, doc| {
        format_token_with_comments(doc, &operator)
    });

    doc_concat!(doc, [operand, operator])
}

struct PendingBinaryRun<'source> {
    owner: BinaryExpression<'source>,
    first_operand: Expression<'source>,
    parts: Vec<(JavaOperator<'source>, Expression<'source>)>,
    removed_parentheses: Vec<ParenthesizedExpression<'source>>,
}

fn finish_pending_binary_run<'source>(
    formatted: Doc<'source>,
    run: Option<PendingBinaryRun<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let Some(run) = run else {
        return formatted;
    };
    let Some(root_operator_kind) = run
        .parts
        .last()
        .map(|(root_operator, _)| root_operator.kind())
    else {
        doc.block_on_invariant("Java binary run has no operator and right operand");
        return Doc::nil();
    };
    let first = format_binary_operand_doc(
        &run.owner,
        &run.first_operand,
        root_operator_kind,
        formatted,
        doc,
    );
    let rest = doc.concat_list(|rest| {
        for parentheses in run.removed_parentheses.into_iter().rev() {
            let removals = run.owner.redundant_parenthesis_removal_claims(&parentheses);
            if let Some(open) = removals.open {
                let removed = rest.removed_source(open);
                rest.push(removed);
            }
            if let Some(close) = removals.close {
                let removed = rest.removed_source(close);
                rest.push(removed);
            }
        }
        for (operator, operand) in run.parts {
            let operand = format_binary_operand(&run.owner, &operand, operator.kind(), rest);
            let operator = format_operator_with_comments(&operator, rest);
            let item = binary_chain_item(operator, operand, rest);
            rest.push(item);
        }
    });

    binary_chain(doc, first, rest)
}

fn format_binary_operand<'source>(
    owner: &BinaryExpression<'source>,
    expression: &Expression<'source>,
    parent_operator: JavaOperatorKind,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let formatted = format_expression(expression, doc);
    format_binary_operand_doc(owner, expression, parent_operator, formatted, doc)
}

fn format_binary_operand_doc<'source>(
    owner: &BinaryExpression<'source>,
    expression: &Expression<'source>,
    parent_operator: JavaOperatorKind,
    formatted: Doc<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    if should_parenthesize_binary_operand(expression, parent_operator) {
        let Some(claims) = owner.precedence_parenthesis_claims(expression) else {
            return formatted;
        };
        let open = doc.synthesized_source(claims.open);
        let line = doc.soft_line();
        let indented = doc_indent!(doc, doc_concat!(doc, [line, formatted]));
        let line = doc.soft_line();
        let close = doc.synthesized_source(claims.close);
        doc_group!(
            doc,
            doc_concat!(
                doc,
                [
                    // Intentional synthesized token: readability parentheses preserve
                    // the parsed precedence while making mixed binary precedence clear.
                    open, indented, line,
                    // Intentional synthesized token: closes the doc-owned
                    // readability parenthesis above.
                    close,
                ]
            )
        )
    } else {
        formatted
    }
}

fn present<T>(field: JavaSyntaxField<'_, T>) -> Option<T> {
    match field {
        JavaSyntaxField::Present(value) => Some(value),
        JavaSyntaxField::Missing(_) | JavaSyntaxField::Malformed(_) => None,
    }
}

fn binary_operator<'source>(
    expression: &BinaryExpression<'source>,
) -> Option<JavaOperator<'source>> {
    present(expression.operator())?.as_operator().ok()
}

fn format_operator_with_comments<'source>(
    operator: &JavaOperator<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    if let Some(token) = operator.as_single_token() {
        return format_token_with_comments(doc, token);
    }

    let components = operator.components();
    doc.concat_list(|docs| {
        for component in components {
            let component =
                crate::helpers::recovery::format_required_field(component, docs, |token, docs| {
                    format_token_with_comments(docs, &token)
                });
            docs.push(component);
        }
    })
}

fn assignment_expression<'source>(
    doc: &mut DocBuilder<'source>,
    left: Doc<'source>,
    operator: Doc<'source>,
    right: Doc<'source>,
) -> Doc<'source> {
    doc_group!(
        doc,
        doc_concat!(
            doc,
            [left, doc.space(), operator, assignment_rhs(right, doc)]
        )
    )
}

fn assignment_rhs<'source>(right: Doc<'source>, doc: &mut DocBuilder<'source>) -> Doc<'source> {
    doc_indent!(doc, doc_concat!(doc, [doc.line(), right]))
}

fn binary_chain<'source>(
    doc: &mut DocBuilder<'source>,
    first: Doc<'source>,
    rest: Doc<'source>,
) -> Doc<'source> {
    doc_group!(doc, doc_concat!(doc, [first, doc_indent!(doc, rest),]))
}

fn binary_chain_item<'source>(
    operator: Doc<'source>,
    operand: Doc<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let line = doc.line();
    let space = doc.space();
    doc_concat!(doc, [line, operator, space, operand])
}

fn ternary_expression<'source>(
    builder: &mut DocBuilder<'source>,
    condition: Doc<'source>,
    question: Doc<'source>,
    consequence: Doc<'source>,
    colon: Doc<'source>,
    alternative: Doc<'source>,
    force_break: bool,
) -> Doc<'source> {
    let line = builder.line();
    let question_space = builder.space();
    let line_after_consequence = builder.line();
    let colon_space = builder.space();
    let tail = doc_indent!(
        builder,
        doc_concat!(
            builder,
            [
                line,
                question,
                question_space,
                consequence,
                line_after_consequence,
                colon,
                colon_space,
                alternative,
            ]
        )
    );
    let doc = doc_concat!(builder, [condition, tail,]);

    if force_break {
        doc_force_group!(builder, doc)
    } else {
        doc_group!(builder, doc)
    }
}

fn should_force_conditional_break(expression: &ConditionalExpression<'_>) -> bool {
    matches!(
        Expression::from(*expression).parent_role(),
        Some(
            ExpressionParentRole::ConditionalCondition
                | ExpressionParentRole::ConditionalTrueExpression
                | ExpressionParentRole::ConditionalFalseExpression
        )
    )
}

fn should_flatten_binary(
    parent_operator: JavaOperatorKind,
    child_operator: JavaOperatorKind,
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

    if is_shift_operator(parent_operator) && is_shift_operator(child_operator) {
        return false;
    }

    if is_multiplicative_operator(parent_operator) && is_multiplicative_operator(child_operator) {
        return parent_operator == child_operator && parent_operator != JavaOperatorKind::Percent;
    }

    true
}

fn should_parenthesize_binary_operand(
    expression: &Expression,
    parent_operator: JavaOperatorKind,
) -> bool {
    if !is_bitwise_or_shift_operator(parent_operator) {
        return false;
    }

    matches!(expression, Expression::BinaryExpression(_))
}
