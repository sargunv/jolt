use super::leaves::format_leaf_token;
use super::{
    ArgumentList, CommaListItem, Doc, Expression, FieldAccessExpression, LeadingComments,
    MethodInvocationExpression, format_expression, format_expression_with_leading_comments,
    format_member_chain, format_member_dot, format_token_with_comments, format_type_argument_list,
    is_member_chain_child, parenthesized_list,
};
use crate::helpers::lists::syntax_comma_list_items;
use crate::helpers::recovery::{
    JavaFormatField, format_optional_field, format_required_field, resolve_required_delimiter,
    resolve_required_field,
};
use jolt_fmt_ir::DocBuilder;
use jolt_java_syntax::{
    NameExpression, QualifiedInvocationName, QualifiedMethodInvocation, UnqualifiedInvocationName,
    UnqualifiedMethodInvocation,
};

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
            [format_required_field(
                expression.form(),
                doc,
                |form, doc| {
                    if let Some(qualified) = form.cast_node::<QualifiedMethodInvocation<'source>>()
                    {
                        format_qualified_method_invocation(&qualified, leading_comments, doc)
                    } else if let Some(unqualified) =
                        form.cast_node::<UnqualifiedMethodInvocation<'source>>()
                    {
                        format_unqualified_method_invocation(&unqualified, leading_comments, doc)
                    } else {
                        doc.block_on_invariant("method invocation form had an unknown shape");
                        Doc::nil()
                    }
                }
            )]
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
    doc_group!(
        doc,
        doc_concat!(
            doc,
            [
                format_required_field(expression.receiver(), doc, |receiver, doc| {
                    format_expression(&receiver, doc)
                }),
                format_required_field(expression.dot(), doc, |dot, doc| {
                    format_member_dot(Some(&dot), doc)
                }),
                format_required_field(expression.name(), doc, |name, doc| {
                    format_token_with_comments(doc, &name)
                }),
                format_optional_field(expression.type_arguments(), doc, |arguments, doc| {
                    format_type_argument_list(&arguments, doc)
                }),
            ]
        ),
    )
}

fn format_qualified_method_invocation<'source>(
    expression: &QualifiedMethodInvocation<'source>,
    leading_comments: LeadingComments,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    doc_concat!(
        doc,
        [
            format_required_field(expression.receiver(), doc, |receiver, doc| {
                format_expression(&receiver, doc)
            }),
            format_required_field(expression.dot(), doc, |dot, doc| {
                format_member_dot(Some(&dot), doc)
            }),
            format_optional_field(expression.type_arguments(), doc, |arguments, doc| {
                format_type_argument_list(&arguments, doc)
            }),
            format_required_field(expression.name(), doc, |name, doc| {
                format_qualified_invocation_name(name, leading_comments, doc)
            }),
            format_required_field(expression.arguments(), doc, |arguments, doc| {
                format_argument_list(Some(arguments), doc)
            }),
        ]
    )
}

fn format_unqualified_method_invocation<'source>(
    expression: &UnqualifiedMethodInvocation<'source>,
    leading_comments: LeadingComments,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    doc_concat!(
        doc,
        [
            format_optional_field(expression.type_arguments(), doc, |arguments, doc| {
                format_type_argument_list(&arguments, doc)
            }),
            format_required_field(expression.name(), doc, |name, doc| {
                format_unqualified_invocation_name(name, leading_comments, doc)
            }),
            format_required_field(expression.arguments(), doc, |arguments, doc| {
                format_argument_list(Some(arguments), doc)
            }),
        ]
    )
}

pub(super) fn format_qualified_invocation_name<'source>(
    name: QualifiedInvocationName<'source>,
    leading_comments: LeadingComments,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    format_invocation_name_parts(
        name.token(),
        name.cast_node::<NameExpression<'source>>(),
        leading_comments,
        doc,
    )
}

fn format_unqualified_invocation_name<'source>(
    name: UnqualifiedInvocationName<'source>,
    leading_comments: LeadingComments,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    format_invocation_name_parts(
        name.token(),
        name.cast_node::<NameExpression<'source>>(),
        leading_comments,
        doc,
    )
}

fn format_invocation_name_parts<'source>(
    token: Option<jolt_java_syntax::JavaSyntaxToken<'source>>,
    name: Option<NameExpression<'source>>,
    leading_comments: LeadingComments,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    if let Some(token) = token {
        format_leaf_token(&token, leading_comments, doc)
    } else if let Some(name) = name {
        format_expression_with_leading_comments(&name.into(), leading_comments, doc)
    } else {
        doc.block_on_invariant("method invocation name had an unknown shape");
        Doc::nil()
    }
}

pub(crate) fn format_argument_list<'source>(
    arguments: Option<ArgumentList<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let Some(arguments) = arguments else {
        return Doc::nil();
    };
    let open = resolve_required_delimiter(arguments.open_paren(), doc);
    let close = resolve_required_delimiter(arguments.close_paren(), doc);
    let items = argument_list_items(&arguments, doc);
    parenthesized_list(doc, open, close, items)
}

fn argument_list_items<'source, 'fmt>(
    arguments: &'fmt ArgumentList<'source>,
    doc: &'fmt mut DocBuilder<'source>,
) -> Vec<CommaListItem<'source>> {
    match resolve_required_field(arguments.arguments(), doc) {
        JavaFormatField::Present(arguments) => {
            syntax_comma_list_items(doc, arguments.parts(), |argument, doc| {
                format_expression(&argument, doc)
            })
        }
        JavaFormatField::Malformed(recovery) => vec![CommaListItem {
            doc: recovery,
            comma: None,
        }],
    }
}
