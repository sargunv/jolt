use super::{
    AnnotationElementDeclaration, CommaListItem, Doc, FormalParameterList, JavaSyntaxToken,
    LeadingTrivia, MethodDeclaration, ThrowsClause, TrailingTrivia, comment_forces_line,
    delimited_comma_list, format_annotation_element_value, format_array_dimensions, format_block,
    format_construct_leading_comments, format_constructor_body, format_formal_parameter,
    format_modifier_prefix, format_receiver_parameter, format_separator_with_comments,
    format_statement_semicolon, format_token, format_token_after_construct_leading_comments,
    format_token_with_comments, format_type, format_type_parameter_list,
    format_type_without_leading_comments, format_typed_modifier_prefix, source_braced_body,
};
use jolt_fmt_ir::DocBuilder;

use crate::helpers::recovery::{
    JavaFormatDelimiter, JavaFormatField, JavaFormatListPart, format_optional_field,
    format_required_field, resolve_list_part, resolve_optional_field, resolve_required_delimiter,
    resolve_required_field,
};
use crate::rules::annotations::format_annotation;

fn format_optional_modifier_prefix<'source>(
    modifiers: JavaFormatField<'source, Option<jolt_java_syntax::ModifierList<'source>>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    match modifiers {
        JavaFormatField::Present(modifiers) => format_modifier_prefix(modifiers, doc),
        JavaFormatField::Malformed(malformed) => malformed,
    }
}

fn format_optional_type_parameters<'source>(
    parameters: JavaFormatField<'source, Option<jolt_java_syntax::TypeParameterList<'source>>>,
    doc: &mut DocBuilder<'source>,
) -> (Doc<'source>, bool) {
    match parameters {
        JavaFormatField::Present(Some(parameters)) => {
            (format_type_parameter_list(parameters, doc), true)
        }
        JavaFormatField::Present(None) => (Doc::nil(), false),
        JavaFormatField::Malformed(malformed) => (malformed, true),
    }
}

fn format_optional_throws_clause<'source>(
    throws: JavaFormatField<'source, Option<ThrowsClause<'source>>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    match throws {
        JavaFormatField::Present(Some(throws)) => format_throws_clause(&throws, doc),
        JavaFormatField::Present(None) => Doc::nil(),
        JavaFormatField::Malformed(malformed) => malformed,
    }
}

fn format_constructor_body_field<'source>(
    body: jolt_java_syntax::JavaSyntaxField<'source, jolt_java_syntax::ConstructorBody<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    match resolve_required_field(body, doc) {
        JavaFormatField::Present(body) => {
            let open = resolve_required_delimiter(body.open_brace(), doc);
            let close = resolve_required_delimiter(body.close_brace(), doc);
            let contents = format_constructor_body(
                &body,
                open.source().copied(),
                close.source().copied(),
                doc,
            );
            source_braced_body(doc, open, close, contents)
        }
        JavaFormatField::Malformed(malformed) => malformed,
    }
}

pub(super) fn format_constructor_declaration<'source>(
    constructor: &jolt_java_syntax::ConstructorDeclaration<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let constructor_first_token = constructor.first_token();
    let modifiers = resolve_optional_field(constructor.modifiers(), doc);
    let throws = resolve_optional_field(constructor.throws(), doc);
    let type_parameters = resolve_optional_field(constructor.type_parameters(), doc);
    let name = format_required_field(constructor.name(), doc, |name, doc| {
        format_token_after_construct_leading_comments(doc, &name, constructor_first_token.as_ref())
    });
    let open_paren = resolve_required_delimiter(constructor.open_paren(), doc);
    let parameters = resolve_optional_field(constructor.parameters(), doc);
    let close_paren = resolve_required_delimiter(constructor.close_paren(), doc);
    let prefix = doc_concat!(
        doc,
        [
            format_construct_leading_comments(doc, constructor_first_token.as_ref()),
            format_optional_modifier_prefix(modifiers, doc),
        ]
    );
    let (type_parameters, has_type_parameters) =
        format_optional_type_parameters(type_parameters, doc);
    let header = doc_concat!(
        doc,
        [
            type_parameters,
            if has_type_parameters {
                doc.space()
            } else {
                Doc::nil()
            },
            name,
            format_parameters(open_paren, close_paren, parameters, doc,),
            format_optional_throws_clause(throws, doc),
        ]
    );
    callable_declaration_with_body_doc(
        prefix,
        header,
        format_constructor_body_field(constructor.body(), doc),
        doc,
    )
}

pub(super) fn format_compact_constructor_declaration<'source>(
    constructor: &jolt_java_syntax::CompactConstructorDeclaration<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let modifiers = resolve_optional_field(constructor.modifiers(), doc);
    let prefix = format_optional_modifier_prefix(modifiers, doc);
    let header = format_required_field(constructor.name(), doc, |name, doc| {
        format_token_with_comments(doc, &name)
    });

    callable_declaration_with_body_doc(
        prefix,
        header,
        format_constructor_body_field(constructor.body(), doc),
        doc,
    )
}

pub(crate) fn format_method_declaration<'source>(
    method: &MethodDeclaration<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let method_modifiers = resolve_optional_field(method.modifiers(), doc);
    let throws = resolve_optional_field(method.throws(), doc);
    let type_parameters = resolve_optional_field(method.type_parameters(), doc);
    let return_annotations = resolve_optional_field(method.return_annotations(), doc);
    let parameters = resolve_optional_field(method.parameters(), doc);
    let name = format_required_field(method.name(), doc, |name, doc| {
        format_token_with_comments(doc, &name)
    });
    let open_paren = resolve_required_delimiter(method.open_paren(), doc);
    let close_paren = resolve_required_delimiter(method.close_paren(), doc);
    let dimensions = format_optional_field(method.dimensions(), doc, |dimensions, doc| {
        format_array_dimensions(&dimensions, doc)
    });
    let return_type = format_required_field(method.return_type(), doc, |return_type, doc| {
        format_type_without_leading_comments(&return_type, doc)
    });
    let body = resolve_required_field(method.body(), doc);
    let modifiers = match method_modifiers {
        JavaFormatField::Present(modifiers) => format_typed_modifier_prefix(modifiers, doc),
        JavaFormatField::Malformed(malformed) => crate::rules::modifiers::TypedModifierPrefix {
            declaration_prefix: malformed,
            type_use_prefix: Doc::nil(),
        },
    };
    let prefix = doc_concat!(
        doc,
        [
            format_construct_leading_comments(doc, method.first_token().as_ref()),
            modifiers.declaration_prefix,
        ]
    );
    let (type_parameters, has_type_parameters) =
        format_optional_type_parameters(type_parameters, doc);
    let return_annotations = format_optional_annotation_list(return_annotations, doc);
    let has_return_annotations = return_annotations.is_some();
    let name_and_parameters = doc_concat!(
        doc,
        [
            name,
            format_parameters(open_paren, close_paren, parameters, doc,),
            dimensions,
        ]
    );
    let header = doc_concat!(
        doc,
        [
            type_parameters,
            if has_type_parameters {
                doc.space()
            } else {
                Doc::nil()
            },
            modifiers.type_use_prefix,
            return_annotations.unwrap_or_else(Doc::nil),
            if has_return_annotations {
                doc.space()
            } else {
                Doc::nil()
            },
            return_type,
            doc.space(),
            name_and_parameters,
            format_optional_throws_clause(throws, doc),
        ]
    );

    match body {
        JavaFormatField::Present(body) => {
            if let Some(block) = body.cast_node::<jolt_java_syntax::Block<'source>>() {
                callable_declaration_with_body_doc(prefix, header, format_block(&block, doc), doc)
            } else if let Some(semicolon) = body.token() {
                doc_concat!(
                    doc,
                    [
                        prefix,
                        doc_group!(doc, header),
                        format_statement_semicolon(
                            jolt_java_syntax::JavaSyntaxField::Present(semicolon),
                            doc,
                        )
                    ]
                )
            } else {
                doc.block_on_invariant("method body had an undeclared kind");
                doc_concat!(doc, [prefix, doc_group!(doc, header)])
            }
        }
        JavaFormatField::Malformed(malformed) => doc_concat!(
            doc,
            [prefix, doc_group!(doc, header), doc.space(), malformed,]
        ),
    }
}

fn format_optional_annotation_list<'source>(
    annotations: JavaFormatField<'source, Option<jolt_java_syntax::AnnotationList<'source>>>,
    doc: &mut DocBuilder<'source>,
) -> Option<Doc<'source>> {
    match annotations {
        JavaFormatField::Present(None) => None,
        JavaFormatField::Malformed(recovery) => Some(recovery),
        JavaFormatField::Present(Some(annotations)) => Some(doc.concat_list(|docs| {
            let mut first = true;
            for part in annotations.parts() {
                if !first {
                    let space = docs.space();
                    docs.push(space);
                }
                first = false;
                let part = match resolve_list_part(part, docs) {
                    JavaFormatListPart::Item(annotation) => format_annotation(&annotation, docs),
                    JavaFormatListPart::Separator(separator) => {
                        docs.block_on_invariant("annotation list had a separator");
                        format_token_with_comments(docs, &separator)
                    }
                    JavaFormatListPart::Malformed(recovery) => recovery,
                };
                docs.push(part);
            }
        })),
    }
}

pub(super) fn format_annotation_element_declaration<'source>(
    element: &AnnotationElementDeclaration<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let modifiers = resolve_optional_field(element.modifiers(), doc);
    let ty = format_required_field(element.r#type(), doc, |ty, doc| format_type(&ty, doc));
    let name = format_required_field(element.name(), doc, |name, doc| {
        format_token_with_comments(doc, &name)
    });
    let open = resolve_required_delimiter(element.open_paren(), doc);
    let close = resolve_required_delimiter(element.close_paren(), doc);
    let dimensions = format_optional_field(element.dimensions(), doc, |dimensions, doc| {
        format_array_dimensions(&dimensions, doc)
    });
    let default = format_optional_field(element.default(), doc, |default, doc| {
        format_annotation_element_default(&default, doc)
    });
    let semicolon = element.semicolon();
    doc_concat!(
        doc,
        [
            doc_group!(
                doc,
                doc_concat!(
                    doc,
                    [
                        format_optional_modifier_prefix(modifiers, doc),
                        ty,
                        doc.space(),
                        name,
                        format_empty_parameters(doc, open, close),
                        dimensions,
                        default,
                    ]
                ),
            ),
            format_statement_semicolon(semicolon, doc),
        ]
    )
}

fn format_annotation_element_default<'source>(
    default: &jolt_java_syntax::DefaultValue<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let keyword = format_required_field(default.default_keyword(), doc, |keyword, doc| {
        format_token_with_comments(doc, &keyword)
    });
    let value = format_required_field(default.value(), doc, |value, doc| {
        format_annotation_element_value(&value, doc)
    });
    doc_concat!(doc, [doc.space(), keyword, doc.space(), value])
}

fn format_parameters<'source>(
    open: JavaFormatDelimiter<'source>,
    close: JavaFormatDelimiter<'source>,
    parameters: JavaFormatField<'source, Option<FormalParameterList<'source>>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let parameters = match parameters {
        JavaFormatField::Present(Some(parameters)) => parameter_list_items(&parameters, doc),
        JavaFormatField::Present(None) => Vec::new(),
        JavaFormatField::Malformed(malformed) => vec![CommaListItem {
            doc: malformed,
            comma: None,
        }],
    };
    delimited_comma_list(doc, open, close, parameters)
}

fn parameter_list_items<'source, 'fmt>(
    parameters: &'fmt FormalParameterList<'source>,
    doc: &'fmt mut DocBuilder<'source>,
) -> Vec<CommaListItem<'source>> {
    let parts = parameters.parts();
    let (lower, _) = parts.size_hint();
    let mut items = Vec::with_capacity(lower);
    for part in parts {
        match resolve_list_part(part, doc) {
            JavaFormatListPart::Item(item) => {
                let item_doc = match item {
                    jolt_java_syntax::FormalParameterSyntax::FormalParameter(parameter) => {
                        format_formal_parameter(&parameter, doc)
                    }
                    jolt_java_syntax::FormalParameterSyntax::ReceiverParameter(parameter) => {
                        format_receiver_parameter(&parameter, doc)
                    }
                    jolt_java_syntax::FormalParameterSyntax::BogusFormalParameter(bogus) => {
                        crate::helpers::recovery::format_malformed(&bogus, doc)
                    }
                };
                items.push(CommaListItem {
                    doc: item_doc,
                    comma: None,
                });
            }
            JavaFormatListPart::Separator(comma) => {
                if let Some(item) = items.last_mut() {
                    item.comma = Some(comma);
                } else {
                    doc.block_on_invariant("formal parameter separator had no preceding item");
                }
            }
            JavaFormatListPart::Malformed(malformed) => items.push(CommaListItem {
                doc: malformed,
                comma: None,
            }),
        }
    }
    items
}

fn format_empty_parameters<'source>(
    doc: &mut jolt_fmt_ir::DocBuilder<'source>,
    open: JavaFormatDelimiter<'source>,
    close: JavaFormatDelimiter<'source>,
) -> Doc<'source> {
    delimited_comma_list(
        doc,
        open,
        close,
        std::iter::empty::<CommaListItem<'source>>(),
    )
}

fn callable_declaration_with_body_doc<'source>(
    prefix: Doc<'source>,
    header: Doc<'source>,
    body: Doc<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    doc_concat!(doc, [prefix, doc_group!(doc, header), doc.space(), body])
}

fn format_throws_clause<'source>(
    throws: &ThrowsClause<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let entries = match resolve_required_field(throws.exceptions(), doc) {
        JavaFormatField::Present(exceptions) => Ok(exceptions
            .parts()
            .map(|part| resolve_list_part(part, doc))
            .collect::<Vec<_>>()),
        JavaFormatField::Malformed(malformed) => Err(malformed),
    };
    let keyword = resolve_required_field(throws.throws_keyword(), doc);
    let keyword_forces_line = matches!(&keyword, JavaFormatField::Present(keyword) if {
        keyword
            .trailing_comments()
            .any(|comment| comment_forces_line(&comment))
    });
    let keyword = match keyword {
        JavaFormatField::Present(keyword) => format_token(
            doc,
            &keyword,
            LeadingTrivia::Preserve,
            TrailingTrivia::BeforeLineBreak,
        ),
        JavaFormatField::Malformed(malformed) => malformed,
    };
    let entries = match entries {
        Ok(entries) => entries,
        Err(malformed) => {
            return doc_indent!(doc, doc_concat!(doc, [doc.line(), keyword, malformed]));
        }
    };
    if entries.is_empty() {
        return doc_indent!(doc, doc_concat!(doc, [doc.line(), keyword]));
    }

    doc_indent!(
        doc,
        doc_concat!(
            doc,
            [
                doc.line(),
                keyword,
                if keyword_forces_line {
                    doc.hard_line()
                } else {
                    doc.space()
                },
                format_throws_entries(&entries, doc),
            ]
        )
    )
}

fn format_throws_entries<'source>(
    entries: &[JavaFormatListPart<'source, jolt_java_syntax::Type<'source>>],
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let mut entries = entries.iter().peekable();
    let Some(entry) = entries.next() else {
        return Doc::nil();
    };

    let first = format_throws_entry(entry, entries.peek().copied(), doc);
    let contents = doc.concat_list(|docs| {
        docs.push(first);
        while let Some(entry) = entries.next() {
            let entry_doc = format_throws_entry(entry, entries.peek().copied(), docs);
            docs.push(entry_doc);
        }
    });

    doc_indent!(doc, contents)
}

fn format_throws_entry<'source>(
    entry: &JavaFormatListPart<'source, jolt_java_syntax::Type<'source>>,
    next: Option<&JavaFormatListPart<'source, jolt_java_syntax::Type<'source>>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let has_next = next.is_some();
    let next_is_separator = matches!(next, Some(JavaFormatListPart::Separator(_)));
    match entry {
        JavaFormatListPart::Item(exception) => doc_concat!(
            doc,
            [
                format_type(exception, doc),
                format_throws_entry_separator(doc, None, has_next && !next_is_separator),
            ]
        ),
        JavaFormatListPart::Separator(token) => {
            format_throws_entry_separator(doc, Some(*token), has_next)
        }
        JavaFormatListPart::Malformed(malformed) => doc_concat!(
            doc,
            [
                *malformed,
                format_throws_entry_separator(doc, None, has_next && !next_is_separator),
            ]
        ),
    }
}

fn format_throws_entry_separator<'source>(
    doc: &mut jolt_fmt_ir::DocBuilder<'source>,
    comma: Option<JavaSyntaxToken<'source>>,
    has_next: bool,
) -> Doc<'source> {
    if let Some(comma) = comma {
        let separator = doc.line();
        format_separator_with_comments(doc, &comma, separator)
    } else if has_next {
        doc.line()
    } else {
        Doc::nil()
    }
}
