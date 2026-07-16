use super::{
    Doc, LambdaExpression, LambdaParameter, LeadingTrivia, TrailingTrivia, comment_forces_line,
    format_annotation, format_block, format_expression, format_separator_with_comments,
    format_token, format_token_with_comments, format_type, token_iter_has_comments,
};
use crate::helpers::comments::token_has_comments;
use crate::helpers::lists::{CommaListItem, comma_list, syntax_comma_list_items};
use crate::helpers::recovery::{
    JavaFormatField, JavaFormatListPart, format_malformed, format_optional_field,
    format_required_field, resolve_list_part, resolve_required_field,
};
use jolt_fmt_ir::DocBuilder;
use jolt_java_syntax::{
    JavaSyntaxField, JavaSyntaxListPart, JavaSyntaxView, LambdaBodySyntax, LambdaBodyValue,
    LambdaModifier, LambdaModifierSyntax,
};

pub(super) fn format_lambda_expression<'source>(
    expression: &LambdaExpression<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let parameters = format_lambda_parameters(expression, doc);
    let arrow = format_lambda_arrow(expression, doc);
    let body = format_required_field(expression.body(), doc, format_lambda_body);

    doc_concat!(doc, [parameters, arrow, body])
}

fn format_lambda_body<'source>(
    body: LambdaBodySyntax<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    match body.classify() {
        Ok(LambdaBodyValue::Expression(expression)) => format_expression(&expression, doc),
        Ok(LambdaBodyValue::Block(block)) => format_block(&block, doc),
        Ok(LambdaBodyValue::Bogus(bogus)) => format_malformed(&bogus, doc),
        Err(error) => {
            doc.block_on_invariant(error.to_string());
            Doc::nil()
        }
    }
}

fn format_lambda_arrow<'source>(
    expression: &LambdaExpression<'source>,
    doc: &mut jolt_fmt_ir::DocBuilder<'source>,
) -> Doc<'source> {
    format_required_field(expression.arrow(), doc, |arrow, doc| {
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
    })
}

fn format_lambda_parameters<'source>(
    expression: &LambdaExpression<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let parameters = resolve_required_field(expression.parameters(), doc);
    if let JavaFormatField::Present(parameters) = &parameters
        && expression.is_recovery_free()
        && optional_delimiter_is_comment_free(expression.open_paren())
        && optional_delimiter_is_comment_free(expression.close_paren())
        && let Some(parameter) = single_lambda_parameter(parameters)
        && is_simple_untyped_lambda_parameter(&parameter)
    {
        let parameter = if token_iter_has_comments(parameter.token_iter()) {
            format_lambda_parameter(&parameter, doc)
        } else {
            format_required_field(parameter.name(), doc, |name, doc| {
                format_token_with_comments(doc, &name)
            })
        };
        let removals = expression.parameter_parenthesis_removal_claims();
        let open = removals
            .open
            .map_or_else(Doc::nil, |claim| doc.removed_source(claim));
        let close = removals
            .close
            .map_or_else(Doc::nil, |claim| doc.removed_source(claim));
        return doc_concat!(doc, [open, parameter, close]);
    }
    let open = format_optional_field(expression.open_paren(), doc, |token, doc| {
        format_token_with_comments(doc, &token)
    });
    let parameters = match parameters {
        JavaFormatField::Present(parameters) => format_lambda_parameter_entries(&parameters, doc),
        JavaFormatField::Malformed(recovery) => recovery,
    };
    let close = format_optional_field(expression.close_paren(), doc, |token, doc| {
        let token_doc = format_token_with_comments(doc, &token);
        if token.leading_comments().is_empty() {
            token_doc
        } else {
            doc_concat!(doc, [doc.line(), token_doc])
        }
    });

    doc_group!(doc, doc_concat!(doc, [open, parameters, close]),)
}

fn optional_delimiter_is_comment_free(
    field: Result<
        JavaSyntaxField<'_, jolt_java_syntax::JavaSyntaxToken<'_>>,
        jolt_java_syntax::JavaSyntaxInvariantError,
    >,
) -> bool {
    match field {
        Ok(JavaSyntaxField::Missing(_)) => true,
        Ok(JavaSyntaxField::Present(token)) => !token_has_comments(&token),
        Ok(JavaSyntaxField::Malformed(_)) | Err(_) => false,
    }
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
    syntax_comma_list_items(doc, parameters.parts(), |parameter, doc| {
        format_lambda_parameter(&parameter, doc)
    })
}

fn single_lambda_parameter<'source>(
    parameters: &jolt_java_syntax::LambdaParameterList<'source>,
) -> Option<LambdaParameter<'source>> {
    let mut parts = parameters.parts();
    let parameter = match parts.next()?.ok()? {
        JavaSyntaxListPart::Item(parameter) => parameter,
        JavaSyntaxListPart::Separator(_)
        | JavaSyntaxListPart::Missing(_)
        | JavaSyntaxListPart::Malformed(_) => return None,
    };
    parts.next().is_none().then_some(parameter)
}

#[allow(clippy::needless_pass_by_value)]
fn optional_is_absent<T>(
    field: Result<JavaSyntaxField<'_, T>, jolt_java_syntax::JavaSyntaxInvariantError>,
) -> bool {
    matches!(field, Ok(JavaSyntaxField::Missing(_)))
}

fn required_list_is_empty<'source, T>(
    field: Result<JavaSyntaxField<'source, T>, jolt_java_syntax::JavaSyntaxInvariantError>,
) -> bool
where
    T: JavaSyntaxView<'source>,
{
    matches!(field, Ok(JavaSyntaxField::Present(list)) if list.is_recovery_free() && list.first_token().is_none())
}

fn is_simple_untyped_lambda_parameter(parameter: &LambdaParameter<'_>) -> bool {
    optional_is_absent(parameter.r#type())
        && optional_is_absent(parameter.ellipsis())
        && required_list_is_empty(parameter.modifiers())
        && required_list_is_empty(parameter.varargs_annotations())
}

fn format_lambda_parameter<'source>(
    parameter: &LambdaParameter<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let modifiers = format_required_field(parameter.modifiers(), doc, |modifiers, doc| {
        format_lambda_modifiers(modifiers.parts(), doc)
    });
    let ty = format_optional_field(parameter.r#type(), doc, |ty, doc| format_type(&ty, doc));
    let varargs_annotations =
        format_required_field(parameter.varargs_annotations(), doc, |annotations, doc| {
            format_annotation_parts(annotations.parts(), doc)
        });
    let ellipsis = format_optional_field(parameter.ellipsis(), doc, |ellipsis, doc| {
        format_token_with_comments(doc, &ellipsis)
    });
    let name = format_required_field(parameter.name(), doc, |name, doc| {
        format_token_with_comments(doc, &name)
    });
    doc_concat!(
        doc,
        [
            modifiers,
            ty,
            varargs_annotations,
            ellipsis,
            if optional_is_absent(parameter.r#type()) && optional_is_absent(parameter.ellipsis()) {
                Doc::nil()
            } else {
                doc.space()
            },
            name
        ]
    )
}

fn format_lambda_modifiers<'source>(
    parts: impl IntoIterator<
        Item = Result<
            JavaSyntaxListPart<'source, LambdaModifier<'source>>,
            jolt_java_syntax::JavaSyntaxInvariantError,
        >,
    >,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let mut has_parts = false;
    let docs = doc.concat_list(|docs| {
        for part in parts {
            if has_parts {
                let space = docs.space();
                docs.push(space);
            }
            has_parts = true;
            let part = match resolve_list_part(part, docs) {
                JavaFormatListPart::Item(modifier) => match modifier.classify() {
                    Ok(LambdaModifierSyntax::Annotation(annotation)) => {
                        format_annotation(&annotation, docs)
                    }
                    Ok(LambdaModifierSyntax::Final(token) | LambdaModifierSyntax::Var(token)) => {
                        format_token_with_comments(docs, &token)
                    }
                    Err(error) => {
                        docs.block_on_invariant(error.to_string());
                        Doc::nil()
                    }
                },
                JavaFormatListPart::Separator(separator) => {
                    format_token_with_comments(docs, &separator)
                }
                JavaFormatListPart::Malformed(recovery) => recovery,
            };
            docs.push(part);
        }
    });
    if has_parts {
        doc_concat!(doc, [docs, doc.space()])
    } else {
        Doc::nil()
    }
}

fn format_annotation_parts<'source>(
    parts: impl IntoIterator<
        Item = Result<
            JavaSyntaxListPart<'source, jolt_java_syntax::Annotation<'source>>,
            jolt_java_syntax::JavaSyntaxInvariantError,
        >,
    >,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let mut has_parts = false;
    let annotations = doc.concat_list(|docs| {
        for part in parts {
            if has_parts {
                let space = docs.space();
                docs.push(space);
            }
            has_parts = true;
            let part = match resolve_list_part(part, docs) {
                JavaFormatListPart::Item(annotation) => format_annotation(&annotation, docs),
                JavaFormatListPart::Separator(separator) => {
                    format_token_with_comments(docs, &separator)
                }
                JavaFormatListPart::Malformed(recovery) => recovery,
            };
            docs.push(part);
        }
    });
    if has_parts {
        doc_concat!(doc, [doc.space(), annotations, doc.space()])
    } else {
        Doc::nil()
    }
}
