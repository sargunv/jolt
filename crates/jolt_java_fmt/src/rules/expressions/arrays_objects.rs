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
use crate::helpers::lists::recovered_comma_list_items;
use jolt_fmt_ir::DocBuilder;

pub(super) fn format_array_access_expression<'source>(
    expression: &ArrayAccessExpression<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let open_bracket = expression.open_bracket();
    let close_bracket = expression.close_bracket();
    let array = expression
        .array()
        .map_or_else(Doc::nil, |array| format_expression(&array, doc));
    let index = expression
        .index()
        .map_or_else(Doc::nil, |index| format_expression(&index, doc));
    let index =
        format_bracketed_expression(doc, open_bracket.as_ref(), index, close_bracket.as_ref());

    doc_group!(doc, doc_concat!(doc, [array, index]),)
}

pub(super) fn format_object_creation_expression<'source>(
    expression: &ObjectCreationExpression<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let qualifier = expression.qualifier();
    let qualifier = qualifier
        .as_ref()
        .map_or_else(Doc::nil, |qualifier| format_expression(qualifier, doc));
    let dot = expression
        .dot_token()
        .as_ref()
        .map_or_else(Doc::nil, |token| format_token_with_comments(doc, token));
    let new = format_creation_new_keyword(expression.new_token().as_ref(), doc);
    let constructor_type_arguments =
        expression
            .constructor_type_arguments()
            .map_or_else(Doc::nil, |arguments| {
                doc_concat!(
                    doc,
                    [format_type_argument_list(&arguments, doc), doc.space(),]
                )
            });
    let ty = match expression.ty() {
        Some(ty) => format_type(&ty, doc),
        None => expression
            .recovered_primitive_type_token()
            .map_or_else(Doc::nil, |token| format_token_with_comments(doc, &token)),
    };
    let arguments = format_argument_list(expression.arguments(), doc);
    let body = expression.body().map_or_else(Doc::nil, |body| {
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
    let new = format_creation_new_keyword(expression.new_token().as_ref(), doc);
    let ty = expression
        .ty()
        .map_or_else(Doc::nil, |ty| format_type(&ty, doc));
    let dimensions = doc.concat_list(|dimensions| {
        for dimension in expression.dimensions() {
            let dimension = format_dim_expression(&dimension, dimensions);
            dimensions.push(dimension);
        }
    });
    let trailing_dimensions = expression
        .trailing_dimensions()
        .map_or_else(Doc::nil, |dimensions| {
            format_array_dimensions(&dimensions, doc)
        });
    let initializer = expression
        .initializer()
        .map_or_else(Doc::nil, |initializer| {
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
    let mut annotations = dimension.annotations().peekable();
    let annotations = if annotations.peek().is_some() {
        doc_concat!(
            doc,
            [
                doc.space(),
                crate::rules::types::format_inline_annotations(annotations, doc),
            ]
        )
    } else {
        Doc::nil()
    };
    let open_bracket = dimension.open_bracket();
    let close_bracket = dimension.close_bracket();
    let expression = match dimension.expression() {
        Some(expression) => format_expression(&expression, doc),
        None => Doc::nil(),
    };

    doc_concat!(
        doc,
        [
            annotations,
            format_bracketed_expression(
                doc,
                open_bracket.as_ref(),
                expression,
                close_bracket.as_ref()
            ),
        ]
    )
}

fn format_bracketed_expression<'source>(
    doc: &mut DocBuilder<'source>,
    open: Option<&JavaSyntaxToken<'source>>,
    expression: Doc<'source>,
    close: Option<&JavaSyntaxToken<'source>>,
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
    open: Option<&JavaSyntaxToken<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    open.map_or_else(Doc::nil, |open| {
        format_token(
            doc,
            open,
            LeadingTrivia::Preserve,
            TrailingTrivia::RelocatedToEnclosingContext,
        )
    })
}

fn format_open_bracket_spacing<'source>(
    open: Option<&JavaSyntaxToken<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let Some(open) = open else {
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
    close: Option<&JavaSyntaxToken<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    close.map_or_else(Doc::nil, |close| {
        format_token_with_inline_leading_comments(
            doc,
            close,
            InlineLeadingTrivia::AfterPreviousToken,
            TrailingTrivia::Preserve,
        )
    })
}

fn format_array_initializer<'source>(
    initializer: &ArrayInitializer<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let open = initializer.open_brace();
    let close = initializer.close_brace();
    let items = array_initializer_items(initializer, doc);
    braced_comma_list_with_trailing_separator(doc, open.as_ref(), close.as_ref(), items)
}

fn array_initializer_items<'source, 'fmt>(
    initializer: &'fmt ArrayInitializer<'source>,
    doc: &'fmt mut DocBuilder<'source>,
) -> Vec<CommaListItem<'source>> {
    recovered_comma_list_items(doc, initializer.entries_with_recovered(), |entry, doc| {
        CommaListItem {
            doc: format_variable_initializer_value(entry.value, doc),
            comma: entry.comma,
        }
    })
}

pub(crate) fn format_variable_initializer_value<'source>(
    value: VariableInitializerValue<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    match value {
        VariableInitializerValue::LiteralExpression(expression) => {
            format_expression(&expression.into(), doc)
        }
        VariableInitializerValue::TemplateExpression(expression) => {
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
    }
}
