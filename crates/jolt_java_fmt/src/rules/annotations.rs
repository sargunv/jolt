use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_java_syntax::{
    Annotation, AnnotationArgumentList, AnnotationArgumentSyntax, AnnotationArrayInitializer,
    AnnotationElementValue, AnnotationElementValueContentItem, AnnotationElementValuePair,
};

use crate::helpers::comments::{
    TrailingTrivia, format_token_after_relocated_leading_comments, format_token_with_comments,
};
use crate::helpers::lists::{
    CommaListItem, braced_comma_list_with_trailing_separator, parenthesized_list,
    syntax_comma_list_items,
};
use crate::helpers::recovery::{
    JavaFormatField, format_optional_field, format_required_field, resolve_required_delimiter,
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
    let at = format_required_field(annotation.at(), doc, |token, doc| at_token(doc, &token));
    let name = format_required_field(annotation.name(), doc, |name, doc| format_name(&name, doc));
    let arguments = format_optional_field(annotation.arguments(), doc, |arguments, doc| {
        format_annotation_argument_list(&arguments, doc)
    });
    doc_concat!(doc, [at, name, arguments])
}

pub(crate) fn format_annotation_element_value<'source>(
    value: &AnnotationElementValue<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    format_required_field(value.value(), doc, |content, doc| {
        match content.classify() {
            Some(AnnotationElementValueContentItem::Expression(expression)) => {
                format_expression(&expression, doc)
            }
            Some(AnnotationElementValueContentItem::Annotation(annotation)) => {
                format_annotation(&annotation, doc)
            }
            Some(AnnotationElementValueContentItem::ArrayInitializer(array)) => {
                format_annotation_array_initializer(&array, doc)
            }
            None => {
                doc.block_on_invariant("invalid annotation element value role");
                Doc::nil()
            }
        }
    })
}

fn format_annotation_argument_list<'source>(
    arguments: &AnnotationArgumentList<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let open = resolve_required_delimiter(arguments.open_paren(), doc);
    let close = resolve_required_delimiter(arguments.close_paren(), doc);
    let items = annotation_argument_list_items(arguments, doc);
    parenthesized_list(doc, open, close, items)
}

fn annotation_argument_list_items<'source, 'fmt>(
    arguments: &'fmt AnnotationArgumentList<'source>,
    doc: &'fmt mut DocBuilder<'source>,
) -> Vec<CommaListItem<'source>> {
    #[allow(clippy::single_match_else)]
    match crate::helpers::recovery::resolve_optional_field(arguments.elements(), doc) {
        JavaFormatField::Present(Some(elements)) => {
            match crate::helpers::recovery::resolve_required_field(elements.arguments(), doc) {
                JavaFormatField::Present(arguments) => {
                    syntax_comma_list_items(doc, arguments.parts(), |argument, doc| {
                        format_annotation_argument(&argument, doc)
                    })
                }
                JavaFormatField::Malformed(malformed) => vec![CommaListItem {
                    doc: malformed,
                    comma: None,
                }],
            }
        }
        JavaFormatField::Present(None) => Vec::new(),
        JavaFormatField::Malformed(malformed) => vec![CommaListItem {
            doc: malformed,
            comma: None,
        }],
    }
}

fn format_annotation_argument<'source>(
    argument: &AnnotationArgumentSyntax<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    match argument {
        AnnotationArgumentSyntax::AnnotationElementValue(value) => {
            format_annotation_element_value(value, doc)
        }
        AnnotationArgumentSyntax::AnnotationElementValuePair(pair) => {
            format_annotation_element_value_pair(pair, doc)
        }
        AnnotationArgumentSyntax::BogusAnnotationArgument(bogus) => {
            crate::helpers::recovery::format_malformed(bogus, doc)
        }
    }
}

fn format_annotation_element_value_pair<'source>(
    pair: &AnnotationElementValuePair<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let name = format_required_field(pair.name(), doc, |name, doc| {
        format_token_with_comments(doc, &name)
    });
    let assign = format_required_field(pair.assign(), doc, |assign, doc| {
        format_token_with_comments(doc, &assign)
    });
    let value = format_required_field(pair.value(), doc, |value, doc| {
        format_annotation_element_value(&value, doc)
    });
    doc_concat!(doc, [name, doc.space(), assign, doc.space(), value])
}

fn format_annotation_array_initializer<'source>(
    initializer: &AnnotationArrayInitializer<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let open = resolve_required_delimiter(initializer.open_brace(), doc);
    let close = resolve_required_delimiter(initializer.close_brace(), doc);
    let items = annotation_array_initializer_items(initializer, doc);
    braced_comma_list_with_trailing_separator(
        doc,
        open,
        close,
        items,
        initializer.trailing_comma_claim(),
    )
}

fn annotation_array_initializer_items<'source, 'fmt>(
    initializer: &'fmt AnnotationArrayInitializer<'source>,
    doc: &'fmt mut DocBuilder<'source>,
) -> Vec<CommaListItem<'source>> {
    match crate::helpers::recovery::resolve_required_field(initializer.values(), doc) {
        JavaFormatField::Present(values) => {
            syntax_comma_list_items(doc, values.parts(), |value, doc| {
                format_annotation_element_value(&value, doc)
            })
        }
        JavaFormatField::Malformed(malformed) => vec![CommaListItem {
            doc: malformed,
            comma: None,
        }],
    }
}
