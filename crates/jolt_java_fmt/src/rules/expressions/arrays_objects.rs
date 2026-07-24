use super::calls::format_argument_list;
use super::{
    ArrayAccessExpression, ArrayCreationExpression, ArrayInitializer, CommaListItem, DimExpression,
    Doc, InlineLeadingTrivia, JavaSyntaxToken, LeadingTrivia, ObjectCreationExpression,
    TrailingTrivia, VariableInitializerValue, braced_comma_list_with_trailing_separator,
    comment_forces_line, format_anonymous_class_body, format_array_dimensions, format_expression,
    format_token, format_token_with_comments, format_token_with_inline_leading_comments,
    format_trailing_comments_before_line_break, format_type, format_type_argument_list,
    trailing_comments_force_line,
};
use crate::helpers::lists::syntax_comma_list_items;
use crate::helpers::recovery::{
    JavaFormatDelimiter, JavaFormatField, JavaFormatListPart, format_malformed,
    format_optional_field, format_required_field, resolve_list_part, resolve_required_delimiter,
    resolve_required_field,
};
use jolt_fmt_ir::DocBuilder;
use jolt_java_syntax::{ArrayCreationTypeSyntax, ObjectCreationTypeSyntax};

pub(super) fn format_array_access_expression<'source>(
    expression: &ArrayAccessExpression<'source>,
    array: Option<Doc<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let open_bracket = resolve_required_delimiter(expression.open_bracket(), doc);
    let close_bracket = resolve_required_delimiter(expression.close_bracket(), doc);
    let array = array.unwrap_or_else(|| {
        format_required_field(expression.array(), doc, |array, doc| {
            format_expression(&array, doc)
        })
    });
    let index = format_required_field(expression.index(), doc, |index, doc| {
        format_expression(&index, doc)
    });
    let index = format_bracketed_expression(doc, &open_bracket, index, &close_bracket);

    doc_group!(doc, doc_concat!(doc, [array, index]),)
}

pub(super) fn format_object_creation_expression<'source>(
    expression: &ObjectCreationExpression<'source>,
    qualifier: Option<Doc<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let qualifier = qualifier.unwrap_or_else(|| {
        format_optional_field(expression.qualifier(), doc, |qualifier, doc| {
            format_expression(&qualifier, doc)
        })
    });
    let dot = format_optional_field(expression.dot(), doc, |token, doc| {
        format_token_with_comments(doc, &token)
    });
    let new = format_required_field(expression.new_keyword(), doc, |keyword, doc| {
        format_creation_new_keyword(Some(&keyword), doc)
    });
    let constructor_type_arguments = format_optional_field(
        expression.constructor_type_arguments(),
        doc,
        |arguments, doc| {
            doc_concat!(
                doc,
                [format_type_argument_list(&arguments, doc), doc.space(),]
            )
        },
    );
    let ty = format_required_field(expression.r#type(), doc, |ty, doc| match ty {
        ObjectCreationTypeSyntax::ClassType(ty) => format_type(&ty.into(), doc),
        ObjectCreationTypeSyntax::BogusObjectCreationType(ty) => format_malformed(&ty, doc),
    });
    let arguments = format_required_field(expression.arguments(), doc, |arguments, doc| {
        format_argument_list(arguments, doc)
    });
    let body = format_optional_field(expression.body(), doc, |body, doc| {
        doc_concat!(doc, [doc.space(), format_anonymous_class_body(&body, doc)])
    });

    doc_group!(
        doc,
        doc_concat!(
            doc,
            [
                qualifier,
                dot,
                new,
                constructor_type_arguments,
                ty,
                arguments,
                body,
            ]
        ),
    )
}

pub(super) fn format_array_creation_expression<'source>(
    expression: &ArrayCreationExpression<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let new = format_required_field(expression.new_keyword(), doc, |keyword, doc| {
        format_creation_new_keyword(Some(&keyword), doc)
    });
    let ty = format_required_field(expression.r#type(), doc, |ty, doc| match ty {
        ArrayCreationTypeSyntax::PrimitiveType(ty) => format_type(&ty.into(), doc),
        ArrayCreationTypeSyntax::ClassType(ty) => format_type(&ty.into(), doc),
        ArrayCreationTypeSyntax::ArrayType(ty) => format_type(&ty.into(), doc),
        ArrayCreationTypeSyntax::BogusArrayCreationType(ty) => format_malformed(&ty, doc),
    });
    let dimensions = format_required_field(
        expression.dimension_expressions(),
        doc,
        |dimensions, doc| {
            doc.concat_list(|docs| {
                for part in dimensions.parts() {
                    let part = match resolve_list_part(part, docs) {
                        JavaFormatListPart::Item(dimension) => {
                            format_dim_expression(&dimension, docs)
                        }
                        JavaFormatListPart::Separator(separator) => {
                            format_token_with_comments(docs, &separator)
                        }
                        JavaFormatListPart::Recovery(recovery) => recovery.doc(),
                    };
                    docs.push(part);
                }
            })
        },
    );
    let trailing_dimensions =
        format_optional_field(expression.dimensions(), doc, |dimensions, doc| {
            format_array_dimensions(&dimensions, doc)
        });
    let initializer = format_optional_field(expression.initializer(), doc, |initializer, doc| {
        doc_concat!(
            doc,
            [doc.space(), format_array_initializer(&initializer, doc),]
        )
    });

    doc_group!(
        doc,
        doc_concat!(doc, [new, ty, dimensions, trailing_dimensions, initializer]),
    )
}

fn format_creation_new_keyword<'source>(
    keyword: Option<&JavaSyntaxToken<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    keyword.map_or_else(Doc::nil, |keyword| {
        doc_concat!(
            doc,
            [
                format_token(
                    doc,
                    keyword,
                    LeadingTrivia::Preserve,
                    TrailingTrivia::BeforeLineBreak,
                ),
                if trailing_comments_force_line(keyword) {
                    doc.hard_line()
                } else {
                    doc.space()
                },
            ]
        )
    })
}

fn format_dim_expression<'source>(
    dimension: &DimExpression<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let annotations = format_required_field(dimension.annotations(), doc, |annotations, doc| {
        format_dimension_annotations(annotations.parts(), doc)
    });
    let open_bracket = resolve_required_delimiter(dimension.open_bracket(), doc);
    let close_bracket = resolve_required_delimiter(dimension.close_bracket(), doc);
    let expression = format_required_field(dimension.expression(), doc, |expression, doc| {
        format_expression(&expression, doc)
    });

    doc_concat!(
        doc,
        [
            annotations,
            format_bracketed_expression(doc, &open_bracket, expression, &close_bracket),
        ]
    )
}

fn format_dimension_annotations<'source>(
    parts: impl IntoIterator<
        Item = jolt_java_syntax::JavaSyntaxListPart<'source, jolt_java_syntax::Annotation<'source>>,
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
                JavaFormatListPart::Item(annotation) => {
                    crate::rules::annotations::format_annotation(&annotation, docs)
                }
                JavaFormatListPart::Separator(separator) => {
                    format_token_with_comments(docs, &separator)
                }
                JavaFormatListPart::Recovery(recovery) => recovery.doc(),
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

fn format_bracketed_expression<'source>(
    doc: &mut DocBuilder<'source>,
    open: &JavaFormatDelimiter<'source>,
    expression: Doc<'source>,
    close: &JavaFormatDelimiter<'source>,
) -> Doc<'source> {
    doc_group!(
        doc,
        doc_concat!(
            doc,
            [
                format_open_bracket(open, doc),
                doc_indent!(
                    doc,
                    doc_concat!(doc, [format_open_bracket_spacing(open, doc), expression])
                ),
                format_close_bracket_with_spacing(close, doc),
            ]
        )
    )
}

fn format_open_bracket<'source>(
    open: &JavaFormatDelimiter<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    match open {
        JavaFormatDelimiter::Source(open) => format_token(
            doc,
            open,
            LeadingTrivia::Preserve,
            TrailingTrivia::RelocatedToEnclosingContext,
        ),
        JavaFormatDelimiter::Recovery(recovery) => recovery.doc(),
    }
}

fn format_open_bracket_spacing<'source>(
    open: &JavaFormatDelimiter<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let Some(open) = open.source() else {
        return Doc::nil();
    };

    if open.trailing_comments().is_empty() {
        return Doc::nil();
    }

    doc_concat!(
        doc,
        [
            format_trailing_comments_before_line_break(doc, open),
            if open
                .trailing_comments()
                .any(|comment| comment_forces_line(&comment))
            {
                doc.hard_line()
            } else {
                doc.space()
            },
        ]
    )
}

fn format_close_bracket_with_spacing<'source>(
    close: &JavaFormatDelimiter<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    match close {
        JavaFormatDelimiter::Source(close) => format_token_with_inline_leading_comments(
            doc,
            close,
            InlineLeadingTrivia::AfterPreviousToken,
            TrailingTrivia::Preserve,
        ),
        JavaFormatDelimiter::Recovery(recovery) => recovery.doc(),
    }
}

fn format_array_initializer<'source>(
    initializer: &ArrayInitializer<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let open = resolve_required_delimiter(initializer.open_brace(), doc);
    let close = resolve_required_delimiter(initializer.close_brace(), doc);
    let items = array_initializer_items(initializer, doc);
    braced_comma_list_with_trailing_separator(
        doc,
        open,
        close,
        items,
        initializer.trailing_comma_claim(),
    )
}

fn array_initializer_items<'source, 'fmt>(
    initializer: &'fmt ArrayInitializer<'source>,
    doc: &'fmt mut DocBuilder<'source>,
) -> Vec<CommaListItem<'source>> {
    match resolve_required_field(initializer.values(), doc) {
        JavaFormatField::Present(values) => {
            syntax_comma_list_items(doc, values.parts(), |value, doc| {
                format_variable_initializer_value(value, doc)
            })
        }
        JavaFormatField::Malformed(recovery) => vec![CommaListItem {
            doc: recovery,
            comma: None,
        }],
    }
}

pub(crate) fn format_variable_initializer_value<'source>(
    value: VariableInitializerValue<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    match value {
        VariableInitializerValue::LiteralExpression(expression) => {
            format_expression(&expression.into(), doc)
        }
        VariableInitializerValue::NameExpression(expression) => {
            format_expression(&expression.into(), doc)
        }
        VariableInitializerValue::ThisExpression(expression) => {
            format_expression(&expression.into(), doc)
        }
        VariableInitializerValue::SuperExpression(expression) => {
            format_expression(&expression.into(), doc)
        }
        VariableInitializerValue::ParenthesizedExpression(expression) => {
            format_expression(&expression.into(), doc)
        }
        VariableInitializerValue::ClassLiteralExpression(expression) => {
            format_expression(&expression.into(), doc)
        }
        VariableInitializerValue::FieldAccessExpression(expression) => {
            format_expression(&expression.into(), doc)
        }
        VariableInitializerValue::ArrayAccessExpression(expression) => {
            format_expression(&expression.into(), doc)
        }
        VariableInitializerValue::MethodInvocationExpression(expression) => {
            format_expression(&expression.into(), doc)
        }
        VariableInitializerValue::MethodReferenceExpression(expression) => {
            format_expression(&expression.into(), doc)
        }
        VariableInitializerValue::ObjectCreationExpression(expression) => {
            format_expression(&expression.into(), doc)
        }
        VariableInitializerValue::ArrayCreationExpression(expression) => {
            format_expression(&expression.into(), doc)
        }
        VariableInitializerValue::AssignmentExpression(expression) => {
            format_expression(&expression.into(), doc)
        }
        VariableInitializerValue::ConditionalExpression(expression) => {
            format_expression(&expression.into(), doc)
        }
        VariableInitializerValue::InstanceofExpression(expression) => {
            format_expression(&expression.into(), doc)
        }
        VariableInitializerValue::BinaryExpression(expression) => {
            format_expression(&expression.into(), doc)
        }
        VariableInitializerValue::UnaryExpression(expression) => {
            format_expression(&expression.into(), doc)
        }
        VariableInitializerValue::PostfixExpression(expression) => {
            format_expression(&expression.into(), doc)
        }
        VariableInitializerValue::CastExpression(expression) => {
            format_expression(&expression.into(), doc)
        }
        VariableInitializerValue::LambdaExpression(expression) => {
            format_expression(&expression.into(), doc)
        }
        VariableInitializerValue::SwitchExpression(expression) => {
            format_expression(&expression.into(), doc)
        }
        VariableInitializerValue::ArrayInitializer(initializer) => {
            format_array_initializer(&initializer, doc)
        }
        VariableInitializerValue::BogusExpression(value) => format_malformed(&value, doc),
        VariableInitializerValue::BogusVariableInitializer(value) => format_malformed(&value, doc),
    }
}
