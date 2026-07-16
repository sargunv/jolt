use super::{
    AssignmentExpression, BinaryExpression, ConditionalExpression, Doc, Expression,
    PostfixExpression, UnaryExpression, format_expression, format_token_with_comments,
};
use crate::helpers::comments::token_has_comments;
use crate::helpers::recovery::format_required_field;
use crate::helpers::syntax_tokens::inserted_syntax_token;
use jolt_fmt_ir::DocBuilder;
use jolt_java_syntax::{
    AssignmentTargetSyntax, ExpressionParentRole, JavaOperator, JavaOperatorKind, JavaSyntaxField,
    JavaSyntaxView, binary_operator_precedence, is_bitwise_or_shift_operator,
    is_multiplicative_operator, is_shift_operator,
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

pub(super) fn format_binary_expression<'source>(
    expression: &BinaryExpression<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    if expression.is_recovery_free()
        && let Some(operator) = binary_operator(expression)
    {
        let parent_operator = operator.kind();
        let (first, rest, has_rest) = flatten_binary_expression(expression, operator, doc);
        let first = format_binary_operand(&first, parent_operator, doc);
        return binary_chain(doc, first, rest, has_rest);
    }
    let left = format_required_field(expression.left(), doc, |left, doc| {
        format_expression(&left, doc)
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
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let operand = format_required_field(expression.operand(), doc, |operand, doc| {
        format_expression(&operand, doc)
    });
    let operator = format_required_field(expression.operator(), doc, |operator, doc| {
        format_token_with_comments(doc, &operator)
    });

    doc_concat!(doc, [operand, operator])
}

fn flatten_binary_expression<'source>(
    expression: &BinaryExpression<'source>,
    operator: JavaOperator<'source>,
    doc: &mut DocBuilder<'source>,
) -> (Expression<'source>, Doc<'source>, bool) {
    let root = Expression::from(*expression);
    let mut operands = Vec::new();
    let mut operators = Vec::new();
    let mut removed_parentheses = Vec::new();
    collect_binary_chain(
        *expression,
        operator,
        None,
        &mut operands,
        &mut operators,
        &mut removed_parentheses,
    );
    if operators.len() + 1 != operands.len() {
        return unflattened_binary_expression(expression, doc, &operator);
    }

    let mut operands = operands.into_iter();
    let first = operands.next().unwrap_or(root);
    let mut has_rest = false;
    let rest = doc.concat_list(|rest| {
        for parentheses in removed_parentheses {
            let removals = parentheses.redundant_parenthesis_removal_claims();
            if let Some(open) = removals.open {
                let removed = rest.removed_source(open);
                rest.push(removed);
            }
            if let Some(close) = removals.close {
                let removed = rest.removed_source(close);
                rest.push(removed);
            }
        }
        for (operator, operand) in operators.into_iter().zip(operands) {
            let operand = format_binary_operand(&operand, operator.kind(), rest);
            let operator = format_operator_with_comments(&operator, rest);
            let item = binary_chain_item(operator, operand, rest);
            rest.push(item);
        }
        has_rest = !rest.is_empty();
    });

    (first, rest, has_rest)
}

fn format_binary_operand<'source>(
    expression: &Expression<'source>,
    parent_operator: JavaOperatorKind,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let formatted = format_expression(expression, doc);
    if should_parenthesize_binary_operand(expression, parent_operator) {
        let Some(claims) = expression.precedence_parenthesis_claims() else {
            doc.block_on_invariant("valid binary operand lacked parenthesis normalization claims");
            return formatted;
        };
        let open = inserted_syntax_token(doc, claims.open);
        let line = doc.soft_line();
        let indented = doc_indent!(doc, doc_concat!(doc, [line, formatted]));
        let line = doc.soft_line();
        let close = inserted_syntax_token(doc, claims.close);
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

fn unflattened_binary_expression<'source>(
    expression: &BinaryExpression<'source>,
    doc: &mut DocBuilder<'source>,
    operator: &JavaOperator<'source>,
) -> (Expression<'source>, Doc<'source>, bool) {
    let operator = format_operator_with_comments(operator, doc);
    let right =
        present(expression.right()).map_or_else(Doc::nil, |right| format_expression(&right, doc));
    let rest = doc.concat_list(|rest| {
        let item = binary_chain_item(operator, right, rest);
        rest.push(item);
    });

    (
        present(expression.left()).unwrap_or_else(|| Expression::from(*expression)),
        rest,
        true,
    )
}

fn collect_binary_chain<'source>(
    binary: BinaryExpression<'source>,
    operator: JavaOperator<'source>,
    parentheses: Option<jolt_java_syntax::ParenthesizedExpression<'source>>,
    operands: &mut Vec<Expression<'source>>,
    operators: &mut Vec<JavaOperator<'source>>,
    removed_parentheses: &mut Vec<jolt_java_syntax::ParenthesizedExpression<'source>>,
) {
    removed_parentheses.extend(parentheses);

    if let Some(left) = present(binary.left()) {
        collect_binary_left(
            left,
            operator.kind(),
            operands,
            operators,
            removed_parentheses,
        );
    }
    operators.push(operator);
    if let Some(right) = present(binary.right()) {
        operands.push(right);
    }
}

fn collect_binary_left<'source>(
    expression: Expression<'source>,
    parent_operator: JavaOperatorKind,
    operands: &mut Vec<Expression<'source>>,
    operators: &mut Vec<JavaOperator<'source>>,
    removed_parentheses: &mut Vec<jolt_java_syntax::ParenthesizedExpression<'source>>,
) {
    let Some((binary, parentheses)) = binary_for_chain(expression) else {
        operands.push(expression);
        return;
    };
    let Some(operator) = binary_operator(&binary) else {
        operands.push(expression);
        return;
    };

    if !should_flatten_binary(parent_operator, operator.kind()) {
        operands.push(expression);
        return;
    }

    removed_parentheses.extend(parentheses);
    collect_binary_chain(
        binary,
        operator,
        None,
        operands,
        operators,
        removed_parentheses,
    );
}

fn binary_for_chain(
    expression: Expression<'_>,
) -> Option<(
    BinaryExpression<'_>,
    Option<jolt_java_syntax::ParenthesizedExpression<'_>>,
)> {
    match expression {
        Expression::BinaryExpression(binary) => Some((binary, None)),
        Expression::ParenthesizedExpression(parenthesized)
            if parenthesized
                .open_paren()
                .is_ok_and(|field| matches!(field, JavaSyntaxField::Present(token) if !token_has_comments(&token)))
                && parenthesized
                    .close_paren()
                    .is_ok_and(|field| matches!(field, JavaSyntaxField::Present(token) if !token_has_comments(&token))) =>
        {
            match present(parenthesized.expression()) {
                Some(Expression::BinaryExpression(binary)) => Some((binary, Some(parenthesized))),
                _ => None,
            }
        }
        _ => None,
    }
}

fn present<T>(
    field: Result<JavaSyntaxField<'_, T>, jolt_java_syntax::JavaSyntaxInvariantError>,
) -> Option<T> {
    match field.ok()? {
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
            let component = crate::helpers::recovery::format_required_field(
                Ok(component),
                docs,
                |token, docs| format_token_with_comments(docs, &token),
            );
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
    has_rest: bool,
) -> Doc<'source> {
    if !has_rest {
        return first;
    }

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
