use super::{
    Doc, LambdaExpression, LambdaParameter, LeadingTrivia, TrailingTrivia, comment_forces_line,
    format_annotation, format_block, format_expression, format_separator_with_comments,
    format_token, format_token_with_comments, format_type, inline_modifier_prefix_from_docs,
    token_iter_has_comments,
};
use crate::helpers::lists::{CommaListItem, comma_list, recovered_comma_list_items};
use jolt_fmt_ir::DocBuilder;

pub(super) fn format_lambda_expression<'source>(
    expression: &LambdaExpression<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let parameters = format_lambda_parameters(expression, doc);
    let arrow = format_lambda_arrow(expression, doc);
    let body = match expression.expression_body() {
        Some(body) => format_expression(&body, doc),
        None => expression
            .block_body()
            .map_or_else(Doc::nil, |block| format_block(&block, doc)),
    };

    doc_concat!(doc, [parameters, arrow, body])
}

fn format_lambda_arrow<'source>(
    expression: &LambdaExpression<'source>,
    doc: &mut jolt_fmt_ir::DocBuilder<'source>,
) -> Doc<'source> {
    let Some(arrow) = expression.arrow() else {
        return Doc::nil();
    };

    if arrow.leading_comments().is_empty() && arrow.trailing_comments().is_empty() {
        let space = doc.space();
        let arrow = format_separator_with_comments(doc, &arrow, space);
        return doc_concat!(doc, [doc.space(), arrow]);
    }

    let forced_line = arrow
        .trailing_comments()
        .any(|comment| comment_forces_line(&comment));

    doc_concat!(
        doc,
        [
            doc.space(),
            format_token(
                doc,
                &arrow,
                LeadingTrivia::Preserve,
                TrailingTrivia::BeforeLineBreak,
            ),
            if forced_line {
                doc.hard_line()
            } else {
                doc.space()
            },
        ]
    )
}

fn format_lambda_parameters<'source>(
    expression: &LambdaExpression<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    if let Some(parameter) = expression.concise_parameter()
        && is_simple_untyped_lambda_parameter(&parameter)
    {
        if token_iter_has_comments(parameter.token_iter()) {
            return format_lambda_parameter(&parameter, doc);
        }
        return parameter
            .name()
            .map_or_else(Doc::nil, |name| format_token_with_comments(doc, &name));
    }

    let parameter_list = expression.parameters();
    if let Some(parameters) = parameter_list.as_ref() {
        let mut entries = parameters.entries_with_recovered();
        if let Some(jolt_java_syntax::RecoveredSeparatedListEntry::Entry(entry)) = entries.next()
            && entries.next().is_none()
            && entry.comma.is_none()
            && is_simple_untyped_lambda_parameter(&entry.parameter)
        {
            if token_iter_has_comments(entry.parameter.token_iter()) {
                return format_lambda_parameter(&entry.parameter, doc);
            }
            return entry
                .parameter
                .name()
                .map_or_else(Doc::nil, |name| format_token_with_comments(doc, &name));
        }
    }

    let open = parameter_list
        .as_ref()
        .and_then(jolt_java_syntax::LambdaParameterList::open_paren)
        .or_else(|| expression.open_paren());
    let close = parameter_list
        .as_ref()
        .and_then(jolt_java_syntax::LambdaParameterList::close_paren)
        .or_else(|| expression.close_paren());
    let has_parameters = parameter_list
        .as_ref()
        .is_some_and(|parameters| parameters.parameters().next().is_some());

    if open.is_none() && close.is_none() && !has_parameters {
        return Doc::nil();
    }

    let open = open
        .as_ref()
        .map_or_else(Doc::nil, |token| format_token_with_comments(doc, token));
    let parameters = parameter_list.as_ref().map_or_else(Doc::nil, |parameters| {
        format_lambda_parameter_entries(parameters, doc)
    });
    let close = close
        .as_ref()
        .map_or_else(Doc::nil, |token| format_token_with_comments(doc, token));

    doc_group!(doc, doc_concat!(doc, [open, parameters, close]),)
}

fn format_lambda_parameter_entries<'source>(
    parameters: &jolt_java_syntax::LambdaParameterList<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let items = lambda_parameter_items(parameters, doc);
    comma_list(doc, items)
}

fn lambda_parameter_items<'source, 'fmt>(
    parameters: &'fmt jolt_java_syntax::LambdaParameterList<'source>,
    doc: &'fmt mut DocBuilder<'source>,
) -> Vec<CommaListItem<'source>> {
    recovered_comma_list_items(doc, parameters.entries_with_recovered(), |entry, doc| {
        CommaListItem {
            doc: format_lambda_parameter(&entry.parameter, doc),
            comma: entry.comma,
        }
    })
}

fn is_simple_untyped_lambda_parameter(parameter: &LambdaParameter<'_>) -> bool {
    parameter.ty().is_none()
        && parameter.var_token().is_none()
        && !parameter.is_variable_arity()
        && parameter.prefix_annotations().next().is_none()
        && parameter.varargs_annotations().next().is_none()
        && parameter.modifier_entries().next().is_none()
}

fn format_lambda_parameter<'source>(
    parameter: &LambdaParameter<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let prefix_annotations = format_annotation_run(parameter.prefix_annotations(), doc);
    let modifier_entries = parameter.modifier_entries().collect::<Vec<_>>();
    let has_inline_prefix = prefix_annotations.is_some() || !modifier_entries.is_empty();
    let prefix = inline_modifier_prefix_from_docs(doc, prefix_annotations, modifier_entries);
    let ty = parameter.ty();
    let var_token = parameter.var_token();
    let has_type_prefix = ty.is_some() || var_token.is_some();
    let varargs_annotations = format_annotation_run(parameter.varargs_annotations(), doc);
    let has_varargs_annotations = varargs_annotations.is_some();
    let ty = match ty {
        Some(ty) => format_type(&ty, doc),
        None => var_token.map_or_else(Doc::nil, |token| format_token_with_comments(doc, &token)),
    };
    let name = parameter
        .name()
        .map_or_else(Doc::nil, |name| format_token_with_comments(doc, &name));

    if !has_inline_prefix && !has_type_prefix {
        return name;
    }
    if !has_type_prefix {
        return doc_concat!(doc, [prefix, name]);
    }

    doc_concat!(
        doc,
        [
            prefix,
            ty,
            if parameter.is_variable_arity() {
                if let Some(ellipsis) = parameter.ellipsis_token() {
                    if has_varargs_annotations {
                        let annotations =
                            inline_modifier_prefix_from_docs(doc, varargs_annotations, Vec::new());
                        doc_concat!(
                            doc,
                            [
                                doc.space(),
                                annotations,
                                format_token_with_comments(doc, &ellipsis),
                                doc.space(),
                            ]
                        )
                    } else {
                        doc_concat!(
                            doc,
                            [format_token_with_comments(doc, &ellipsis), doc.space()]
                        )
                    }
                } else {
                    Doc::nil()
                }
            } else {
                doc.space()
            },
            name,
        ]
    )
}

fn format_annotation_run<'source>(
    annotations: impl IntoIterator<Item = jolt_java_syntax::Annotation<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Option<Doc<'source>> {
    let mut has_annotations = false;
    let docs = doc.concat_list(|docs| {
        for annotation in annotations {
            if !docs.is_empty() {
                let space = docs.space();
                docs.push(space);
            }
            let annotation = format_annotation(&annotation, docs);
            docs.push(annotation);
        }
        has_annotations = !docs.is_empty();
    });

    has_annotations.then_some(docs)
}
