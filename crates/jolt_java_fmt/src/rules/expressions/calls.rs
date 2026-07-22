use super::leaves::format_leaf_token;
use super::{
    ArgumentList, CommaListItem, Doc, Expression, FieldAccessExpression, LeadingComments,
    MethodInvocationExpression, delimited_comma_list, format_expression,
    format_expression_with_leading_comments, format_member_chain, format_member_dot,
    format_token_with_comments, format_type_argument_list, is_member_chain_child,
};
use crate::helpers::lists::syntax_comma_list_items;
use crate::helpers::recovery::{
    JavaFormatField, format_malformed, format_optional_field, format_required_field,
    resolve_required_delimiter, resolve_required_field,
};
use jolt_fmt_ir::DocBuilder;
use jolt_java_syntax::{
    InvocationNameSyntax, MethodInvocationFormSyntax, QualifiedInvocationName,
    QualifiedMethodInvocation, UnqualifiedInvocationName, UnqualifiedMethodInvocation,
};

pub(super) fn format_method_invocation_expression_with_leading_comments<'source>(
    expression: &MethodInvocationExpression<'source>,
    leading_comments: LeadingComments,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let expression_family = Expression::from(*expression);
    if !is_member_chain_child(&expression_family)
        && let Some(chain) = format_member_chain(expression_family, doc)
    {
        return chain;
    }

    doc_group!(
        doc,
        doc_concat!(
            doc,
            [format_required_field(
                expression.form(),
                doc,
                |form, doc| match form {
                    MethodInvocationFormSyntax::QualifiedMethodInvocation(qualified) => {
                        format_qualified_method_invocation(&qualified, leading_comments, doc)
                    }
                    MethodInvocationFormSyntax::UnqualifiedMethodInvocation(unqualified) => {
                        format_unqualified_method_invocation(&unqualified, leading_comments, doc)
                    }
                    MethodInvocationFormSyntax::BogusMethodInvocationForm(bogus) => {
                        format_malformed(&bogus, doc)
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
    let expression_family = Expression::from(*expression);
    if !is_member_chain_child(&expression_family)
        && let Some(chain) = format_member_chain(expression_family, doc)
    {
        return chain;
    }
    doc_group!(
        doc,
        doc_concat!(
            doc,
            [
                format_required_field(expression.receiver(), doc, |receiver, doc| {
                    format_expression(&receiver, doc)
                }),
                format_required_field(expression.dot(), doc, |dot, doc| {
                    format_member_dot(&dot, doc)
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
                format_member_dot(&dot, doc)
            }),
            format_optional_field(expression.type_arguments(), doc, |arguments, doc| {
                format_type_argument_list(&arguments, doc)
            }),
            format_required_field(expression.name(), doc, |name, doc| {
                format_qualified_invocation_name(name, leading_comments, doc)
            }),
            format_required_field(expression.arguments(), doc, |arguments, doc| {
                format_argument_list(arguments, doc)
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
                format_argument_list(arguments, doc)
            }),
        ]
    )
}

pub(super) fn format_qualified_invocation_name<'source>(
    name: QualifiedInvocationName<'source>,
    leading_comments: LeadingComments,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    format_invocation_name(name.classify(), leading_comments, doc)
}

fn format_unqualified_invocation_name<'source>(
    name: UnqualifiedInvocationName<'source>,
    leading_comments: LeadingComments,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    format_invocation_name(name.classify(), leading_comments, doc)
}

fn format_invocation_name<'source>(
    name: Result<InvocationNameSyntax<'source>, jolt_java_syntax::JavaSyntaxInvariantError>,
    leading_comments: LeadingComments,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    match name {
        Ok(InvocationNameSyntax::Identifier(token)) => {
            format_leaf_token(&token, leading_comments, doc)
        }
        Ok(InvocationNameSyntax::NameExpression(name)) => {
            format_expression_with_leading_comments(&name.into(), leading_comments, doc)
        }
        Err(error) => {
            doc.block_on_invariant(error.to_string());
            Doc::nil()
        }
    }
}

pub(crate) fn format_argument_list<'source>(
    arguments: ArgumentList<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let open = resolve_required_delimiter(arguments.open_paren(), doc);
    let close = resolve_required_delimiter(arguments.close_paren(), doc);
    let items = argument_list_items(&arguments, doc);
    delimited_comma_list(doc, open, close, items)
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
