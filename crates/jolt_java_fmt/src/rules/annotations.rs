use jolt_fmt_ir::{Doc, concat, text};
use jolt_java_syntax::{
    Annotation, AnnotationArgument, AnnotationArgumentList, AnnotationArrayInitializer,
    AnnotationElementValue, AnnotationElementValuePair,
};

use crate::context::JavaFormatter;
use crate::helpers::comments::format_token_text;
use crate::helpers::lists::{
    CommaListItem, braced_comma_list_with_trailing_separator, parenthesized_list,
};
use crate::rules::expressions::format_expression;
use crate::rules::names::format_name;

pub(crate) fn format_annotation(annotation: &Annotation, formatter: &JavaFormatter<'_>) -> Doc {
    concat([
        text("@"),
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

pub(crate) fn format_annotation_element_value(
    value: &AnnotationElementValue,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    if let Some(expression) = value.expression() {
        return format_expression(&expression, formatter);
    }
    if let Some(annotation) = value.annotation() {
        return format_annotation(&annotation, formatter);
    }
    value
        .array_initializer()
        .map_or_else(jolt_fmt_ir::nil, |array| {
            format_annotation_array_initializer(&array, formatter)
        })
}

fn format_annotation_argument_list(
    arguments: &AnnotationArgumentList,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    let open = arguments.open_paren();
    let close = arguments.close_paren();
    parenthesized_list(
        open.as_ref(),
        close.as_ref(),
        arguments
            .entries()
            .map(|entry| CommaListItem {
                doc: format_annotation_argument(entry.argument, formatter),
                comma: entry.comma,
            })
            .collect(),
    )
}

fn format_annotation_argument(argument: AnnotationArgument, formatter: &JavaFormatter<'_>) -> Doc {
    match argument {
        AnnotationArgument::Value(value) => format_annotation_element_value(&value, formatter),
        AnnotationArgument::Pair(pair) => format_annotation_element_value_pair(&pair, formatter),
    }
}

fn format_annotation_element_value_pair(
    pair: &AnnotationElementValuePair,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    concat([
        pair.name()
            .map_or_else(jolt_fmt_ir::nil, |name| format_token_text(name.text())),
        text(" = "),
        pair.value().map_or_else(jolt_fmt_ir::nil, |value| {
            format_annotation_element_value(&value, formatter)
        }),
    ])
}

fn format_annotation_array_initializer(
    initializer: &AnnotationArrayInitializer,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    let open = initializer.open_brace();
    let close = initializer.close_brace();
    braced_comma_list_with_trailing_separator(
        open.as_ref(),
        close.as_ref(),
        initializer
            .entries()
            .map(|entry| CommaListItem {
                doc: format_annotation_element_value(&entry.value, formatter),
                comma: entry.comma,
            })
            .collect(),
    )
}
