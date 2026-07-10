use super::{
    AssignmentExpression, BinaryExpression, ConditionalExpression, Doc, Expression,
    PostfixExpression, UnaryExpression, format_expression, format_token_with_comments,
};
use crate::helpers::comments::token_has_comments;
use crate::helpers::comments::{
    LeadingTrivia, TrailingTrivia, format_token, format_token_sequence,
};
use crate::helpers::syntax_tokens::{FormatterInsertedToken, inserted_syntax_token};
use jolt_fmt_ir::DocBuilder;
use jolt_java_syntax::{ExpressionParentRole, JavaOperator};

pub(super) fn format_assignment_expression<'source>(
    expression: &AssignmentExpression<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let left = match expression.left() {
        Some(left) => format_expression(&left, doc),
        None => Doc::nil(),
    };
    let operator = match expression.operator() {
        Some(operator) => format_operator_with_comments(&operator, doc),
        None => Doc::nil(),
    };
    let right = match expression.right() {
        Some(right) => format_expression(&right, doc),
        None => Doc::nil(),
    };

    assignment_expression(doc, left, operator, right)
}

pub(super) fn format_conditional_expression<'source>(
    expression: &ConditionalExpression<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let condition = match expression.condition() {
        Some(condition) => format_expression(&condition, doc),
        None => Doc::nil(),
    };
    let question = match expression.question_token() {
        Some(token) => format_token_with_comments(doc, &token),
        None => Doc::nil(),
    };
    let consequence = match expression.true_expression() {
        Some(expression) => format_expression(&expression, doc),
        None => Doc::nil(),
    };
    let colon = match expression.colon_token() {
        Some(token) => format_token_with_comments(doc, &token),
        None => Doc::nil(),
    };
    let alternative = match expression.false_expression() {
        Some(expression) => format_expression(&expression, doc),
        None => Doc::nil(),
    };

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
    let Some(operator) = expression.operator() else {
        return format_binary_expression_without_operator(expression, doc);
    };

    if expression.left().is_none() {
        let operator = format_operator_with_comments(&operator, doc);
        let right = match expression.right() {
            Some(right) => format_expression(&right, doc),
            None => Doc::nil(),
        };
        let rest = doc.concat_list(|rest| {
            let item = binary_chain_item(operator, right, rest);
            rest.push(item);
        });
        let binary = binary_chain(doc, Doc::nil(), rest, true);
        return append_recovered_binary_tokens(expression, binary, doc);
    }

    let parent_operator = operator.text();
    let (first, rest, has_rest) = flatten_binary_expression(expression, doc);
    let first = format_binary_operand(&first, parent_operator, doc);
    let binary = binary_chain(doc, first, rest, has_rest);
    append_recovered_binary_tokens(expression, binary, doc)
}

fn format_binary_expression_without_operator<'source>(
    expression: &BinaryExpression<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let left = expression.left();
    let right = expression.right();
    if left.is_none() && right.is_none() {
        return format_token_sequence(doc, expression.token_iter(), LeadingTrivia::Preserve);
    }
    let left = match left {
        Some(left) => format_expression(&left, doc),
        None => Doc::nil(),
    };
    let right = match right {
        Some(right) => format_expression(&right, doc),
        None => Doc::nil(),
    };

    doc_concat!(doc, [left, right])
}

pub(super) fn format_unary_expression<'source>(
    expression: &UnaryExpression<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let operator = expression.operator().map_or_else(Doc::nil, |operator| {
        format_token_with_comments(doc, &operator)
    });
    let operand = expression
        .operand()
        .map_or_else(Doc::nil, |operand| format_expression(&operand, doc));

    let recovered = doc.concat_list(|tokens| {
        for token in expression.recovered_tokens() {
            let token = format_token(
                tokens,
                &token,
                LeadingTrivia::Preserve,
                TrailingTrivia::Preserve,
            );
            tokens.push(token);
        }
    });
    doc_concat!(doc, [operator, operand, recovered])
}

fn append_recovered_binary_tokens<'source>(
    expression: &BinaryExpression<'source>,
    binary: Doc<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let recovered = doc.concat_list(|tokens| {
        for token in expression.recovered_tokens() {
            let token = format_token(
                tokens,
                &token,
                LeadingTrivia::Preserve,
                TrailingTrivia::Preserve,
            );
            tokens.push(token);
        }
    });
    doc_concat!(doc, [binary, recovered])
}

pub(super) fn format_postfix_expression<'source>(
    expression: &PostfixExpression<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let operand = expression
        .operand()
        .map_or_else(Doc::nil, |operand| format_expression(&operand, doc));
    let operator = expression.operator().map_or_else(Doc::nil, |operator| {
        format_token_with_comments(doc, &operator)
    });

    doc_concat!(doc, [operand, operator])
}

fn flatten_binary_expression<'source>(
    expression: &BinaryExpression<'source>,
    doc: &mut DocBuilder<'source>,
) -> (Expression<'source>, Doc<'source>, bool) {
    let Some(operator) = expression.operator() else {
        let mut has_rest = false;
        let rest = doc.concat_list(|rest| {
            if let Some(right) = expression.right() {
                let right = format_expression(&right, rest);
                let item = binary_chain_item(Doc::nil(), right, rest);
                rest.push(item);
            }
            has_rest = !rest.is_empty();
        });
        return (
            expression
                .left()
                .unwrap_or_else(|| Expression::from(*expression)),
            rest,
            has_rest,
        );
    };
    let root = Expression::from(*expression);
    let mut operands = Vec::new();
    let mut operators = Vec::new();
    collect_binary_chain(root, &mut operands, &mut operators);
    if operators.len() + 1 != operands.len() {
        return unflattened_binary_expression(expression, doc, &operator);
    }

    let mut operands = operands.into_iter();
    let first = operands.next().unwrap_or(root);
    let mut has_rest = false;
    let rest = doc.concat_list(|rest| {
        for (operator, operand) in operators.into_iter().zip(operands) {
            let operand = format_binary_operand(&operand, operator.text(), rest);
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
    parent_operator: &str,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let formatted = format_expression(expression, doc);
    if should_parenthesize_binary_operand(expression, parent_operator) {
        let open = inserted_syntax_token(doc, "(", FormatterInsertedToken::PrecedenceParenthesis);
        let line = doc.soft_line();
        let indented = doc_indent!(doc, doc_concat!(doc, [line, formatted]));
        let line = doc.soft_line();
        let close = inserted_syntax_token(doc, ")", FormatterInsertedToken::PrecedenceParenthesis);
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
    let right = expression
        .right()
        .map_or_else(Doc::nil, |right| format_expression(&right, doc));
    let rest = doc.concat_list(|rest| {
        let item = binary_chain_item(operator, right, rest);
        rest.push(item);
    });

    (
        expression
            .left()
            .unwrap_or_else(|| Expression::from(*expression)),
        rest,
        true,
    )
}

fn collect_binary_chain<'source>(
    expression: Expression<'source>,
    operands: &mut Vec<Expression<'source>>,
    operators: &mut Vec<JavaOperator<'source>>,
) {
    let Some(binary) = binary_for_chain(expression) else {
        operands.push(expression);
        return;
    };
    let Some(operator) = binary.operator() else {
        operands.push(expression);
        return;
    };

    if let Some(left) = binary.left() {
        collect_binary_left(left, operator.text(), operands, operators);
    }
    operators.push(operator);
    if let Some(right) = binary.right() {
        operands.push(right);
    }
}

fn collect_binary_left<'source>(
    expression: Expression<'source>,
    parent_operator: &str,
    operands: &mut Vec<Expression<'source>>,
    operators: &mut Vec<JavaOperator<'source>>,
) {
    let Some(binary) = binary_for_chain(expression) else {
        operands.push(expression);
        return;
    };
    let Some(operator) = binary.operator() else {
        operands.push(expression);
        return;
    };

    if !should_flatten_binary(parent_operator, operator.text()) {
        operands.push(expression);
        return;
    }

    collect_binary_chain(Expression::from(binary), operands, operators);
}

fn binary_for_chain(expression: Expression<'_>) -> Option<BinaryExpression<'_>> {
    match expression {
        Expression::BinaryExpression(binary) => Some(binary),
        Expression::ParenthesizedExpression(parenthesized)
            if parenthesized
                .open_paren()
                .is_none_or(|token| !token_has_comments(&token))
                && parenthesized
                    .close_paren()
                    .is_none_or(|token| !token_has_comments(&token)) =>
        {
            match parenthesized.expression() {
                Some(Expression::BinaryExpression(binary)) => Some(binary),
                _ => None,
            }
        }
        _ => None,
    }
}

fn format_operator_with_comments<'source>(
    operator: &JavaOperator<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    if let Some(token) = operator.as_single_token() {
        return format_token_with_comments(doc, token);
    }

    let mut tokens = operator.tokens().enumerate().peekable();
    doc.concat_list(|docs| {
        while let Some((index, token)) = tokens.next() {
            let is_first = index == 0;
            let is_last = tokens.peek().is_none();
            let token = format_token(
                docs,
                &token,
                if is_first {
                    LeadingTrivia::Preserve
                } else {
                    LeadingTrivia::SuppressAlreadyHandled
                },
                if is_last {
                    TrailingTrivia::Preserve
                } else {
                    TrailingTrivia::RelocatedToEnclosingContext
                },
            );
            docs.push(token);
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

fn should_flatten_binary(parent_operator: &str, child_operator: &str) -> bool {
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
        return parent_operator == child_operator
            && parent_operator != "%"
            && child_operator != "%";
    }

    true
}

fn should_parenthesize_binary_operand(expression: &Expression, parent_operator: &str) -> bool {
    if !is_bitwise_or_shift_operator(parent_operator) {
        return false;
    }

    matches!(expression, Expression::BinaryExpression(_))
}

fn binary_operator_precedence(operator: &str) -> Option<u8> {
    Some(match operator {
        "||" => 1,
        "&&" => 2,
        "|" => 3,
        "^" => 4,
        "&" => 5,
        "==" | "!=" => 6,
        "<" | ">" | "<=" | ">=" => 7,
        "<<" | ">>" | ">>>" => 8,
        "+" | "-" => 9,
        "*" | "/" | "%" => 10,
        _ => return None,
    })
}

fn is_shift_operator(operator: &str) -> bool {
    matches!(operator, "<<" | ">>" | ">>>")
}

fn is_bitwise_or_shift_operator(operator: &str) -> bool {
    matches!(operator, "|" | "^" | "&" | "<<" | ">>" | ">>>")
}

fn is_multiplicative_operator(operator: &str) -> bool {
    matches!(operator, "*" | "/" | "%")
}
