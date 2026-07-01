use jolt_fmt_ir::{Doc, concat, text};
use jolt_java_syntax::{
    Annotation, AnnotationArgument, AnnotationArgumentList, AnnotationArrayInitializer,
    AnnotationElementValue, AnnotationElementValuePair,
};

use crate::helpers::lists::{braced_initializer_list, parenthesized_list};
use crate::rules::expressions::format_expression;

pub(crate) fn format_annotation(annotation: &Annotation) -> Doc {
    concat([
        text("@"),
        annotation
            .name()
            .map_or_else(jolt_fmt_ir::nil, |name| text(name.compact_text())),
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
    parenthesized_list(
        arguments
            .arguments()
            .map(format_annotation_argument)
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
            .map_or_else(jolt_fmt_ir::nil, |name| text(name.text().to_owned())),
        text(" = "),
        pair.value().map_or_else(jolt_fmt_ir::nil, |value| {
            format_annotation_element_value(&value)
        }),
    ])
}

fn format_annotation_array_initializer(initializer: &AnnotationArrayInitializer) -> Doc {
    braced_initializer_list(
        initializer
            .values()
            .map(|value| format_annotation_element_value(&value))
            .collect(),
    )
}
