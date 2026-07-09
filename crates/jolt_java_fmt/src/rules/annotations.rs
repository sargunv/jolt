use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_java_syntax::{
    Annotation, AnnotationArgument, AnnotationArgumentList, AnnotationArrayInitializer,
    AnnotationElementValue, AnnotationElementValuePair,
};

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
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    format_annotation_with_at_token(annotation, doc, |doc, token| {
        format_token_with_comments(doc, token)
    })
}

pub(crate) fn format_annotation_without_leading_comments<'source>(
    annotation: &Annotation<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    format_annotation_with_at_token(annotation, doc, |doc, token| {
        format_token_after_relocated_leading_comments(doc, token, TrailingTrivia::Preserve)
    })
}

fn format_annotation_with_at_token<'source>(
    annotation: &Annotation<'source>,
    doc: &mut DocBuilder<'source>,
    at_token: impl Fn(
        &mut jolt_fmt_ir::DocBuilder<'source>,
        &jolt_java_syntax::JavaSyntaxToken<'source>,
    ) -> Doc<'source>,
) -> Doc<'source> {
    doc_concat!(
        doc,
        [
            annotation
                .at_token()
                .map_or_else(Doc::nil, |token| at_token(doc, &token)),
            annotation
                .name()
                .map_or_else(Doc::nil, |name| format_name(&name, doc)),
            annotation.arguments().map_or_else(Doc::nil, |arguments| {
                format_annotation_argument_list(&arguments, doc)
            },),
        ]
    )
}

pub(crate) fn format_annotation_element_value<'source>(
    value: &AnnotationElementValue<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    if let Some(expression) = value.expression() {
        return format_expression(&expression, doc);
    }
    if let Some(annotation) = value.annotation() {
        return format_annotation(&annotation, doc);
    }
    if let Some(array) = value.array_initializer() {
        return format_annotation_array_initializer(&array, doc);
    }

    format_token_sequence(doc, value.token_iter(), LeadingTrivia::Preserve)
}

fn format_annotation_argument_list<'source>(
    arguments: &AnnotationArgumentList<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let open = arguments.open_paren();
    let close = arguments.close_paren();
    let items = annotation_argument_list_items(arguments, doc);
    parenthesized_list(doc, open.as_ref(), close.as_ref(), items)
}

fn annotation_argument_list_items<'source, 'fmt>(
    arguments: &'fmt AnnotationArgumentList<'source>,
    doc: &'fmt mut DocBuilder<'source>,
) -> Vec<CommaListItem<'source>> {
    recovered_comma_list_items(doc, arguments.entries_with_recovered(), |entry, doc| {
        CommaListItem {
            doc: format_annotation_argument(&entry.argument, doc),
            comma: entry.comma,
        }
    })
}

fn format_annotation_argument<'source>(
    argument: &AnnotationArgument<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    match argument {
        AnnotationArgument::Value(value) => format_annotation_element_value(value, doc),
        AnnotationArgument::Pair(pair) => format_annotation_element_value_pair(pair, doc),
    }
}

fn format_annotation_element_value_pair<'source>(
    pair: &AnnotationElementValuePair<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    doc_concat!(
        doc,
        [
            pair.name()
                .map_or_else(Doc::nil, |name| format_token_with_comments(doc, &name)),
            doc.space(),
            pair.equals_token()
                .map_or_else(Doc::nil, |token| format_token_with_comments(doc, &token),),
            doc.space(),
            pair.value()
                .map_or_else(Doc::nil, |value| format_annotation_element_value(
                    &value, doc
                ),),
        ]
    )
}

fn format_annotation_array_initializer<'source>(
    initializer: &AnnotationArrayInitializer<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let open = initializer.open_brace();
    let close = initializer.close_brace();
    let items = annotation_array_initializer_items(initializer, doc);
    braced_comma_list_with_trailing_separator(doc, open.as_ref(), close.as_ref(), items)
}

fn annotation_array_initializer_items<'source, 'fmt>(
    initializer: &'fmt AnnotationArrayInitializer<'source>,
    doc: &'fmt mut DocBuilder<'source>,
) -> Vec<CommaListItem<'source>> {
    recovered_comma_list_items(doc, initializer.entries_with_recovered(), |entry, doc| {
        CommaListItem {
            doc: format_annotation_element_value(&entry.value, doc),
            comma: entry.comma,
        }
    })
}
