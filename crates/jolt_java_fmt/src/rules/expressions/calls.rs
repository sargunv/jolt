use super::leaves::format_leaf_token;
use super::{
    ArgumentList, CommaListItem, Doc, Expression, ExpressionParentRole, FieldAccessExpression,
    JavaFormatter, LeadingComments, MethodInvocationExpression, concat, format_expression,
    format_expression_with_leading_comments, format_member_chain, format_member_dot,
    format_token_with_comments, format_type_argument_list, group, is_member_chain_child,
    parenthesized_list, text,
};

pub(super) fn format_method_invocation_expression_with_leading_comments(
    expression: &MethodInvocationExpression,
    leading_comments: LeadingComments,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    let expression = Expression::from(expression.clone());
    let parent_role = expression.parent_role();
    if !is_member_chain_child(&expression)
        && let Some(chain) = expression.member_chain()
    {
        return format_member_chain(&chain, formatter);
    }
    let Expression::MethodInvocationExpression(expression) = expression else {
        return jolt_fmt_ir::nil();
    };

    group(concat([
        format_method_invocation_callee(&expression, leading_comments, formatter),
        format_argument_list_for_parent_role(expression.arguments(), parent_role, formatter),
    ]))
}

pub(super) fn format_field_access_expression(
    expression: &FieldAccessExpression,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    let expression = Expression::from(expression.clone());
    if !is_member_chain_child(&expression)
        && let Some(chain) = expression.member_chain()
    {
        return format_member_chain(&chain, formatter);
    }
    let Expression::FieldAccessExpression(expression) = expression else {
        return jolt_fmt_ir::nil();
    };
    let dot = expression.dot_token();

    group(concat([
        expression
            .receiver()
            .map_or_else(jolt_fmt_ir::nil, |receiver| {
                format_expression(&receiver, formatter)
            }),
        format_member_dot(dot.as_ref()),
        expression
            .field_name()
            .map_or_else(jolt_fmt_ir::nil, |name| format_token_with_comments(&name)),
        expression
            .type_arguments()
            .map_or_else(jolt_fmt_ir::nil, |arguments| {
                format_type_argument_list(&arguments, formatter)
            }),
    ]))
}

fn format_method_invocation_callee(
    expression: &MethodInvocationExpression,
    leading_comments: LeadingComments,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    if let Some(name) = expression.direct_method_name() {
        let dot = expression.dot_token();
        return concat([
            expression
                .qualifier()
                .map_or_else(jolt_fmt_ir::nil, |qualifier| {
                    concat([
                        format_expression(&qualifier, formatter),
                        format_member_dot(dot.as_ref()),
                    ])
                }),
            expression
                .type_arguments()
                .map_or_else(jolt_fmt_ir::nil, |arguments| {
                    format_type_argument_list(&arguments, formatter)
                }),
            format_leaf_token(&name, leading_comments),
        ]);
    }

    expression
        .simple_name_expression()
        .map_or_else(jolt_fmt_ir::nil, |name| {
            format_expression_with_leading_comments(&name, leading_comments, formatter)
        })
}

pub(crate) fn format_argument_list(
    arguments: Option<ArgumentList>,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    let Some(arguments) = arguments else {
        return text("()");
    };
    let open = arguments.open_paren();
    let close = arguments.close_paren();
    parenthesized_list(
        open.as_ref(),
        close.as_ref(),
        arguments
            .entries()
            .map(|entry| CommaListItem {
                doc: format_expression(&entry.argument, formatter),
                comma: entry.comma,
            })
            .collect(),
    )
}

pub(super) fn format_argument_list_for_parent_role(
    arguments: Option<ArgumentList>,
    parent_role: Option<ExpressionParentRole>,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    let arguments = format_argument_list(arguments, formatter);
    if parent_role_has_continuation_indent(parent_role) {
        jolt_fmt_ir::dedent(arguments)
    } else {
        arguments
    }
}

const fn parent_role_has_continuation_indent(parent_role: Option<ExpressionParentRole>) -> bool {
    matches!(
        parent_role,
        Some(
            ExpressionParentRole::AssignmentRight
                | ExpressionParentRole::ReturnValue
                | ExpressionParentRole::ThrowValue
                | ExpressionParentRole::YieldValue
                | ExpressionParentRole::VariableInitializer
        )
    )
}
