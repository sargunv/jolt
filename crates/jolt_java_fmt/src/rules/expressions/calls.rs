use super::leaves::format_leaf_token;
use super::{
    ArgumentList, CommaListItem, Doc, Expression, FieldAccessExpression, JavaFormatter,
    LeadingComments, MethodInvocationExpression, concat, format_expression,
    format_expression_with_leading_comments, format_member_chain, format_member_dot,
    format_token_with_comments, format_type_argument_list, group, is_member_chain_child,
    parenthesized_list,
};
use crate::helpers::lists::recovered_comma_list_items;

pub(super) fn format_method_invocation_expression_with_leading_comments<'source>(
    expression: &MethodInvocationExpression<'source>,
    leading_comments: LeadingComments,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    let expression = Expression::from(*expression);
    if !is_member_chain_child(&expression)
        && let Some(chain) = format_member_chain(expression, formatter)
    {
        return chain;
    }
    let Expression::MethodInvocationExpression(expression) = expression else {
        return jolt_fmt_ir::nil();
    };

    group(concat([
        format_method_invocation_callee(&expression, leading_comments, formatter),
        format_argument_list(expression.arguments(), formatter),
    ]))
}

pub(super) fn format_field_access_expression<'source>(
    expression: &FieldAccessExpression<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    let expression = Expression::from(*expression);
    if !is_member_chain_child(&expression)
        && let Some(chain) = format_member_chain(expression, formatter)
    {
        return chain;
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

fn format_method_invocation_callee<'source>(
    expression: &MethodInvocationExpression<'source>,
    leading_comments: LeadingComments,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    let dot = expression.dot_token();
    let qualified_callee = concat([
        expression
            .qualifier()
            .map_or_else(jolt_fmt_ir::nil, |qualifier| {
                concat([
                    format_expression(&qualifier, formatter),
                    format_member_dot(dot.as_ref()),
                ])
            }),
        if expression.qualifier().is_none()
            && expression.direct_method_name().is_none()
            && (expression.dot_token().is_some() || expression.type_arguments().is_some())
        {
            expression
                .simple_name_expression()
                .map_or_else(jolt_fmt_ir::nil, |name| {
                    format_expression_with_leading_comments(&name, leading_comments, formatter)
                })
        } else {
            jolt_fmt_ir::nil()
        },
        expression
            .type_arguments()
            .map_or_else(jolt_fmt_ir::nil, |arguments| {
                format_type_argument_list(&arguments, formatter)
            }),
        expression
            .direct_method_name()
            .map_or_else(jolt_fmt_ir::nil, |name| {
                format_leaf_token(&name, leading_comments)
            }),
        if expression.qualifier().is_none() {
            format_member_dot(dot.as_ref())
        } else {
            jolt_fmt_ir::nil()
        },
    ]);

    if expression.qualifier().is_some()
        || expression.type_arguments().is_some()
        || expression.direct_method_name().is_some()
        || expression.dot_token().is_some()
    {
        return qualified_callee;
    }

    expression
        .simple_name_expression()
        .map_or_else(jolt_fmt_ir::nil, |name| {
            format_expression_with_leading_comments(&name, leading_comments, formatter)
        })
}

pub(crate) fn format_argument_list<'source>(
    arguments: Option<ArgumentList<'source>>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    let Some(arguments) = arguments else {
        return jolt_fmt_ir::nil();
    };
    let open = arguments.open_paren();
    let close = arguments.close_paren();
    let items = argument_list_items(&arguments, formatter);
    parenthesized_list(open.as_ref(), close.as_ref(), items)
}

fn argument_list_items<'source, 'fmt>(
    arguments: &'fmt ArgumentList<'source>,
    formatter: &'fmt JavaFormatter<'_>,
) -> impl Iterator<Item = CommaListItem<'source>> + use<'source, 'fmt> {
    recovered_comma_list_items(arguments.entries_with_recovered(), |entry| CommaListItem {
        doc: format_expression(&entry.argument, formatter),
        comma: entry.comma,
    })
}
