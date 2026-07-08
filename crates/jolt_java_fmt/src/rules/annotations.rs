use jolt_fmt_ir::space;
use jolt_fmt_ir::{Doc, concat};
use jolt_java_syntax::{
    Annotation, AnnotationArgument, AnnotationArgumentList, AnnotationArrayInitializer,
    AnnotationElementValue, AnnotationElementValuePair,
};

use crate::context::JavaFormatter;
use crate::helpers::comments::{
    LeadingTrivia, TrailingTrivia, format_token_after_relocated_leading_comments,
    format_token_sequence, format_token_with_comments,
};
use crate::helpers::lists::{
    CommaListItem, braced_comma_list_with_trailing_separator, parenthesized_list,
    recovered_comma_list_items,
};
use crate::rules::expressions::format_expression;
use crate::rules::names::format_name;

pub(crate) fn format_annotation<'source>(
    annotation: &Annotation<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    format_annotation_with_at_token(annotation, formatter, format_token_with_comments)
}

pub(crate) fn format_annotation_without_leading_comments<'source>(
    annotation: &Annotation<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    format_annotation_with_at_token(annotation, formatter, |token| {
        format_token_after_relocated_leading_comments(token, TrailingTrivia::Preserve)
    })
}

fn format_annotation_with_at_token<'source>(
    annotation: &Annotation<'source>,
    formatter: &JavaFormatter<'_>,
    at_token: impl Fn(&jolt_java_syntax::JavaSyntaxToken<'source>) -> Doc<'source>,
) -> Doc<'source> {
    concat([
        annotation
            .at_token()
            .map_or_else(jolt_fmt_ir::nil, |token| at_token(&token)),
        annotation
            .name()
            .map_or_else(jolt_fmt_ir::nil, |name| format_name(&name)),
        annotation
            .arguments()
            .map_or_else(jolt_fmt_ir::nil, |arguments| {
                format_annotation_argument_list(&arguments, formatter)
            }),
    ])
}

pub(crate) fn format_annotation_element_value<'source>(
    value: &AnnotationElementValue<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    if let Some(expression) = value.expression() {
        return format_expression(&expression, formatter);
    }
    if let Some(annotation) = value.annotation() {
        return format_annotation(&annotation, formatter);
    }
    if let Some(array) = value.array_initializer() {
        return format_annotation_array_initializer(&array, formatter);
    }

    format_token_sequence(value.token_iter(), LeadingTrivia::Preserve)
}

fn format_annotation_argument_list<'source>(
    arguments: &AnnotationArgumentList<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    let open = arguments.open_paren();
    let close = arguments.close_paren();
    let items = annotation_argument_list_items(arguments, formatter);
    parenthesized_list(open.as_ref(), close.as_ref(), items)
}

fn annotation_argument_list_items<'source, 'fmt>(
    arguments: &'fmt AnnotationArgumentList<'source>,
    formatter: &'fmt JavaFormatter<'_>,
) -> impl Iterator<Item = CommaListItem<'source>> + use<'source, 'fmt> {
    recovered_comma_list_items(arguments.entries_with_recovered(), |entry| CommaListItem {
        doc: format_annotation_argument(&entry.argument, formatter),
        comma: entry.comma,
    })
}

fn format_annotation_argument<'source>(
    argument: &AnnotationArgument<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    match argument {
        AnnotationArgument::Value(value) => format_annotation_element_value(value, formatter),
        AnnotationArgument::Pair(pair) => format_annotation_element_value_pair(pair, formatter),
    }
}

fn format_annotation_element_value_pair<'source>(
    pair: &AnnotationElementValuePair<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    concat([
        pair.name()
            .map_or_else(jolt_fmt_ir::nil, |name| format_token_with_comments(&name)),
        space(),
        pair.equals_token()
            .map_or_else(jolt_fmt_ir::nil, |token| format_token_with_comments(&token)),
        space(),
        pair.value().map_or_else(jolt_fmt_ir::nil, |value| {
            format_annotation_element_value(&value, formatter)
        }),
    ])
}

fn format_annotation_array_initializer<'source>(
    initializer: &AnnotationArrayInitializer<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    let open = initializer.open_brace();
    let close = initializer.close_brace();
    braced_comma_list_with_trailing_separator(
        open.as_ref(),
        close.as_ref(),
        annotation_array_initializer_items(initializer, formatter),
    )
}

fn annotation_array_initializer_items<'source, 'fmt>(
    initializer: &'fmt AnnotationArrayInitializer<'source>,
    formatter: &'fmt JavaFormatter<'_>,
) -> impl Iterator<Item = CommaListItem<'source>> + use<'source, 'fmt> {
    recovered_comma_list_items(initializer.entries_with_recovered(), |entry| {
        CommaListItem {
            doc: format_annotation_element_value(&entry.value, formatter),
            comma: entry.comma,
        }
    })
}
