use super::leaves::format_leaf_token;
use super::{
    ArgumentList, CommaListItem, Doc, Expression, FieldAccessExpression, LeadingComments,
    MethodInvocationExpression, format_expression, format_expression_with_leading_comments,
    format_member_chain, format_member_dot, format_token_with_comments, format_type_argument_list,
    is_member_chain_child, parenthesized_list,
};
use crate::helpers::lists::recovered_comma_list_items;
use jolt_fmt_ir::DocBuilder;

pub(super) fn format_method_invocation_expression_with_leading_comments<'source>(
    expression: &MethodInvocationExpression<'source>,
    leading_comments: LeadingComments,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let expression = Expression::from(*expression);
    if !is_member_chain_child(&expression)
        && let Some(chain) = format_member_chain(expression, doc)
    {
        return chain;
    }
    let Expression::MethodInvocationExpression(expression) = expression else {
        return Doc::nil();
    };

    doc_group!(
        doc,
        doc_concat!(
            doc,
            [
                format_method_invocation_callee(&expression, leading_comments, doc),
                format_argument_list(expression.arguments(), doc),
            ]
        )
    )
}

pub(super) fn format_field_access_expression<'source>(
    expression: &FieldAccessExpression<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let expression = Expression::from(*expression);
    if !is_member_chain_child(&expression)
        && let Some(chain) = format_member_chain(expression, doc)
    {
        return chain;
    }
    let Expression::FieldAccessExpression(expression) = expression else {
        return Doc::nil();
    };
    let dot = expression.dot_token();

    doc_group!(
        doc,
        doc_concat!(
            doc,
            [
                expression
                    .receiver()
                    .map_or_else(Doc::nil, |receiver| format_expression(&receiver, doc),),
                format_member_dot(dot.as_ref(), doc),
                expression
                    .field_name()
                    .map_or_else(Doc::nil, |name| format_token_with_comments(doc, &name)),
                expression
                    .type_arguments()
                    .map_or_else(Doc::nil, |arguments| format_type_argument_list(
                        &arguments, doc
                    ),),
            ]
        ),
    )
}

fn format_method_invocation_callee<'source>(
    expression: &MethodInvocationExpression<'source>,
    leading_comments: LeadingComments,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let dot = expression.dot_token();
    let qualified_callee = doc_concat!(
        doc,
        [
            expression.qualifier().map_or_else(Doc::nil, |qualifier| {
                doc_concat!(
                    doc,
                    [
                        format_expression(&qualifier, doc),
                        format_member_dot(dot.as_ref(), doc),
                    ]
                )
            },),
            if expression.qualifier().is_none()
                && expression.direct_method_name().is_none()
                && (expression.dot_token().is_some() || expression.type_arguments().is_some())
            {
                expression
                    .simple_name_expression()
                    .map_or_else(Doc::nil, |name| {
                        format_expression_with_leading_comments(&name, leading_comments, doc)
                    })
            } else {
                Doc::nil()
            },
            expression
                .type_arguments()
                .map_or_else(Doc::nil, |arguments| format_type_argument_list(
                    &arguments, doc
                ),),
            expression
                .direct_method_name()
                .map_or_else(Doc::nil, |name| format_leaf_token(
                    &name,
                    leading_comments,
                    doc
                ),),
            if expression.qualifier().is_none() {
                format_member_dot(dot.as_ref(), doc)
            } else {
                Doc::nil()
            },
        ]
    );

    if expression.qualifier().is_some()
        || expression.type_arguments().is_some()
        || expression.direct_method_name().is_some()
        || expression.dot_token().is_some()
    {
        return qualified_callee;
    }

    expression
        .simple_name_expression()
        .map_or_else(Doc::nil, |name| {
            format_expression_with_leading_comments(&name, leading_comments, doc)
        })
}

pub(crate) fn format_argument_list<'source>(
    arguments: Option<ArgumentList<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let Some(arguments) = arguments else {
        return Doc::nil();
    };
    let open = arguments.open_paren();
    let close = arguments.close_paren();
    let items = argument_list_items(&arguments, doc);
    parenthesized_list(doc, open.as_ref(), close.as_ref(), items)
}

fn argument_list_items<'source, 'fmt>(
    arguments: &'fmt ArgumentList<'source>,
    doc: &'fmt mut DocBuilder<'source>,
) -> Vec<CommaListItem<'source>> {
    recovered_comma_list_items(doc, arguments.entries_with_recovered(), |entry, doc| {
        CommaListItem {
            doc: format_expression(&entry.argument, doc),
            comma: entry.comma,
        }
    })
}
