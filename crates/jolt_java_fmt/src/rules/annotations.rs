use jolt_fmt_ir::{Doc, concat, text};
use jolt_java_syntax::{
    Annotation, AnnotationArgument, AnnotationArgumentList, AnnotationArrayInitializer,
    AnnotationElementValue, AnnotationElementValuePair,
};

use crate::helpers::comments::format_token_text;
use crate::helpers::lists::{
    CommaListItem, braced_comma_list_with_trailing_separator, parenthesized_list,
};
use crate::rules::expressions::format_expression;
use crate::rules::names::format_name;

pub(crate) fn format_annotation(annotation: &Annotation) -> Doc {
    concat([
        text("@"),
        annotation
            .name()
            .map_or_else(jolt_fmt_ir::nil, |name| format_name(&name)),
        annotation
            .arguments()
            .map_or_else(jolt_fmt_ir::nil, |arguments| {
                format_annotation_argument_list(&arguments)
            }),
    ])
}

pub(crate) fn format_annotation_element_value(value: &AnnotationElementValue) -> Doc {
    if let Some(expression) = value.expression() {
        return format_expression(&expression);
    }
    if let Some(annotation) = value.annotation() {
        return format_annotation(&annotation);
    }
    value
        .array_initializer()
        .map_or_else(jolt_fmt_ir::nil, |array| {
            format_annotation_array_initializer(&array)
        })
}

fn format_annotation_argument_list(arguments: &AnnotationArgumentList) -> Doc {
    let open = arguments.open_paren();
    let close = arguments.close_paren();
    parenthesized_list(
        open.as_ref(),
        close.as_ref(),
        arguments
            .entries()
            .map(|entry| CommaListItem {
                doc: format_annotation_argument(entry.argument),
                comma: entry.comma,
            })
            .collect(),
    )
}

fn format_annotation_argument(argument: AnnotationArgument) -> Doc {
    match argument {
        AnnotationArgument::Value(value) => format_annotation_element_value(&value),
        AnnotationArgument::Pair(pair) => format_annotation_element_value_pair(&pair),
    }
}

fn format_annotation_element_value_pair(pair: &AnnotationElementValuePair) -> Doc {
    concat([
        pair.name()
            .map_or_else(jolt_fmt_ir::nil, |name| format_token_text(name.text())),
        text(" = "),
        pair.value().map_or_else(jolt_fmt_ir::nil, |value| {
            format_annotation_element_value(&value)
        }),
    ])
}

fn format_annotation_array_initializer(initializer: &AnnotationArrayInitializer) -> Doc {
    let open = initializer.open_brace();
    let close = initializer.close_brace();
    braced_comma_list_with_trailing_separator(
        open.as_ref(),
        close.as_ref(),
        initializer
            .entries()
            .map(|entry| CommaListItem {
                doc: format_annotation_element_value(&entry.value),
                comma: entry.comma,
            })
            .collect(),
    )
}
