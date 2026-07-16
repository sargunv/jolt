use super::{
    AnnotationInterfaceDeclaration, ClassDeclaration, CommaListItem, Doc, EnumDeclaration,
    ExtendsClause, ImplementsClause, InterfaceDeclaration, JavaSyntaxToken, LeadingTrivia,
    PermitsClause, RecordDeclaration, TrailingTrivia, comma_list, comment_forces_line,
    format_annotation_interface_body, format_class_body, format_construct_leading_comments,
    format_enum_body_contents, format_interface_body, format_leading_comment_list,
    format_modifier_prefix, format_name, format_record_body, format_record_component, format_token,
    format_token_with_comments, format_type_parameter_list, format_type_without_leading_comments,
    parenthesized_list, source_braced_body,
};
use crate::helpers::{
    comments::format_token_after_relocated_leading_comments,
    recovery::{
        JavaFormatDelimiter, JavaFormatField, JavaFormatListPart, resolve_list_part,
        resolve_optional_field, resolve_required_delimiter, resolve_required_field,
    },
};
use jolt_fmt_ir::DocBuilder;

pub(super) fn format_class_declaration<'source>(
    class: &ClassDeclaration<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let modifiers = optional_doc(class.modifiers(), doc, |modifiers, doc| {
        format_modifier_prefix(Some(modifiers), doc)
    });
    let keyword = required_doc(class.class_keyword(), doc, |keyword, doc| {
        keyword_without_space(keyword, doc)
    });
    let (name, name_is_structured) = required_doc_with_presence(class.name(), doc, |name, doc| {
        format_token_with_comments(doc, &name)
    });
    let name_separator = structured_separator(name_is_structured, doc);
    let parameters = optional_doc(class.type_parameters(), doc, |parameters, doc| {
        format_type_parameter_list(Some(parameters), doc)
    });
    let extends = optional_doc(class.extends(), doc, |extends, doc| {
        format_extends(&extends, doc)
    });
    let implements = optional_doc(class.implements(), doc, |implements, doc| {
        format_implements(&implements, doc)
    });
    let permits = optional_doc(class.permits(), doc, |permits, doc| {
        format_permits(&permits, doc)
    });
    let (body, body_is_structured) = required_doc_with_presence(class.body(), doc, |body, doc| {
        let open = resolve_required_delimiter(body.open_brace(), doc);
        let close = resolve_required_delimiter(body.close_brace(), doc);
        let contents = format_class_body(&body, doc);
        source_braced_body(doc, open, close, contents)
    });
    let (missing_body_semicolon, has_missing_body_semicolon) =
        optional_doc_with_presence(class.missing_body_semicolon(), doc, |semicolon, doc| {
            format_token_with_comments(doc, &semicolon)
        });
    type_with_body(
        class.first_token().as_ref(),
        modifiers,
        doc_concat!(
            doc,
            [
                keyword,
                name_separator,
                name,
                parameters,
                extends,
                implements,
                permits
            ]
        ),
        body,
        body_is_structured && !has_missing_body_semicolon,
        missing_body_semicolon,
        doc,
    )
}

pub(super) fn format_interface_declaration<'source>(
    interface: &InterfaceDeclaration<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let modifiers = optional_doc(interface.modifiers(), doc, |modifiers, doc| {
        format_modifier_prefix(Some(modifiers), doc)
    });
    let keyword = required_doc(interface.interface_keyword(), doc, |keyword, doc| {
        keyword_without_space(keyword, doc)
    });
    let (name, name_is_structured) =
        required_doc_with_presence(interface.name(), doc, |name, doc| {
            format_token_with_comments(doc, &name)
        });
    let name_separator = structured_separator(name_is_structured, doc);
    let parameters = optional_doc(interface.type_parameters(), doc, |parameters, doc| {
        format_type_parameter_list(Some(parameters), doc)
    });
    let extends = optional_doc(interface.extends(), doc, |extends, doc| {
        format_extends(&extends, doc)
    });
    let permits = optional_doc(interface.permits(), doc, |permits, doc| {
        format_permits(&permits, doc)
    });
    let (body, body_is_structured) =
        required_doc_with_presence(interface.body(), doc, |body, doc| {
            let open = resolve_required_delimiter(body.open_brace(), doc);
            let close = resolve_required_delimiter(body.close_brace(), doc);
            let contents = format_interface_body(&body, doc);
            source_braced_body(doc, open, close, contents)
        });
    let (missing_body_semicolon, has_missing_body_semicolon) =
        optional_doc_with_presence(interface.missing_body_semicolon(), doc, |semicolon, doc| {
            format_token_with_comments(doc, &semicolon)
        });
    type_with_body(
        interface.first_token().as_ref(),
        modifiers,
        doc_concat!(
            doc,
            [keyword, name_separator, name, parameters, extends, permits]
        ),
        body,
        body_is_structured && !has_missing_body_semicolon,
        missing_body_semicolon,
        doc,
    )
}

pub(super) fn format_record_declaration<'source>(
    record: &RecordDeclaration<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let modifiers = optional_doc(record.modifiers(), doc, |modifiers, doc| {
        format_modifier_prefix(Some(modifiers), doc)
    });
    let keyword = required_doc(record.record_keyword(), doc, |keyword, doc| {
        keyword_without_space(keyword, doc)
    });
    let (name, name_is_structured) = required_doc_with_presence(record.name(), doc, |name, doc| {
        format_token_with_comments(doc, &name)
    });
    let name_separator = structured_separator(name_is_structured, doc);
    let parameters = optional_doc(record.type_parameters(), doc, |parameters, doc| {
        format_type_parameter_list(Some(parameters), doc)
    });
    let component_open = resolve_required_delimiter(record.open_paren(), doc);
    let components = resolve_optional_field(record.components(), doc);
    let component_close = resolve_required_delimiter(record.close_paren(), doc);
    let component_doc = format_record_components(component_open, components, component_close, doc);
    let implements = optional_doc(record.implements(), doc, |implements, doc| {
        format_implements(&implements, doc)
    });
    let (body, body_is_structured) = required_doc_with_presence(record.body(), doc, |body, doc| {
        let open = resolve_required_delimiter(body.open_brace(), doc);
        let close = resolve_required_delimiter(body.close_brace(), doc);
        let contents = format_record_body(&body, doc);
        source_braced_body(doc, open, close, contents)
    });
    let (missing_body_semicolon, has_missing_body_semicolon) =
        optional_doc_with_presence(record.missing_body_semicolon(), doc, |semicolon, doc| {
            format_token_with_comments(doc, &semicolon)
        });
    type_with_body(
        record.first_token().as_ref(),
        modifiers,
        doc_group!(
            doc,
            doc_concat!(
                doc,
                [
                    keyword,
                    name_separator,
                    name,
                    parameters,
                    component_doc,
                    implements
                ]
            )
        ),
        body,
        body_is_structured && !has_missing_body_semicolon,
        missing_body_semicolon,
        doc,
    )
}

pub(super) fn format_enum_declaration<'source>(
    node: &EnumDeclaration<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let modifiers = optional_doc(node.modifiers(), doc, |modifiers, doc| {
        format_modifier_prefix(Some(modifiers), doc)
    });
    let keyword = required_doc(node.enum_keyword(), doc, |keyword, doc| {
        keyword_without_space(keyword, doc)
    });
    let (name, name_is_structured) = required_doc_with_presence(node.name(), doc, |name, doc| {
        format_token_with_comments(doc, &name)
    });
    let name_separator = structured_separator(name_is_structured, doc);
    let implements = optional_doc(node.implements(), doc, |implements, doc| {
        format_implements(&implements, doc)
    });
    let (body, body_is_structured) = required_doc_with_presence(node.body(), doc, |body, doc| {
        let open = resolve_required_delimiter(body.open_brace(), doc);
        let close = resolve_required_delimiter(body.close_brace(), doc);
        let contents = format_enum_body_contents(&body, doc);
        source_braced_body(doc, open, close, contents)
    });
    let (missing_body_semicolon, has_missing_body_semicolon) =
        optional_doc_with_presence(node.missing_body_semicolon(), doc, |semicolon, doc| {
            format_token_with_comments(doc, &semicolon)
        });
    type_with_body(
        node.first_token().as_ref(),
        modifiers,
        doc_concat!(doc, [keyword, name_separator, name, implements]),
        body,
        body_is_structured && !has_missing_body_semicolon,
        missing_body_semicolon,
        doc,
    )
}

pub(super) fn format_annotation_interface_declaration<'source>(
    node: &AnnotationInterfaceDeclaration<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let modifiers = optional_doc(node.modifiers(), doc, |modifiers, doc| {
        format_modifier_prefix(Some(modifiers), doc)
    });
    let at = required_doc(node.at(), doc, |at, doc| {
        format_token_after_relocated_leading_comments(doc, &at, TrailingTrivia::Preserve)
    });
    let interface = required_doc(node.interface_keyword(), doc, |interface, doc| {
        keyword_without_space(interface, doc)
    });
    let (name, name_is_structured) = required_doc_with_presence(node.name(), doc, |name, doc| {
        format_token_with_comments(doc, &name)
    });
    let name_separator = structured_separator(name_is_structured, doc);
    let (body, body_is_structured) = required_doc_with_presence(node.body(), doc, |body, doc| {
        let open = resolve_required_delimiter(body.open_brace(), doc);
        let close = resolve_required_delimiter(body.close_brace(), doc);
        let contents = format_annotation_interface_body(&body, doc);
        source_braced_body(doc, open, close, contents)
    });
    let (missing_body_semicolon, has_missing_body_semicolon) =
        optional_doc_with_presence(node.missing_body_semicolon(), doc, |semicolon, doc| {
            format_token_with_comments(doc, &semicolon)
        });
    type_with_body(
        node.first_token().as_ref(),
        modifiers,
        doc_concat!(doc, [at, interface, name_separator, name]),
        body,
        body_is_structured && !has_missing_body_semicolon,
        missing_body_semicolon,
        doc,
    )
}

fn type_with_body<'source>(
    first: Option<&JavaSyntaxToken<'source>>,
    modifiers: Doc<'source>,
    header: Doc<'source>,
    body: Doc<'source>,
    body_is_structured: bool,
    missing_body_semicolon: Doc<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let body_separator = structured_separator(body_is_structured, doc);
    doc_concat!(
        doc,
        [
            doc_concat!(
                doc,
                [
                    format_leading_comment_list(
                        doc,
                        first
                            .into_iter()
                            .flat_map(JavaSyntaxToken::leading_comments)
                    ),
                    modifiers
                ]
            ),
            doc_group!(doc, header),
            body_separator,
            body,
            missing_body_semicolon,
        ]
    )
}

fn required_doc_with_presence<'source, T>(
    field: Result<
        jolt_java_syntax::JavaSyntaxField<'source, T>,
        jolt_java_syntax::JavaSyntaxInvariantError,
    >,
    doc: &mut DocBuilder<'source>,
    present: impl FnOnce(T, &mut DocBuilder<'source>) -> Doc<'source>,
) -> (Doc<'source>, bool) {
    match resolve_required_field(field, doc) {
        JavaFormatField::Present(value) => (present(value, doc), true),
        JavaFormatField::Malformed(malformed) => (malformed, false),
    }
}

fn structured_separator<'source>(present: bool, doc: &mut DocBuilder<'source>) -> Doc<'source> {
    if present { doc.space() } else { Doc::nil() }
}

fn required_doc<'source, T>(
    field: Result<
        jolt_java_syntax::JavaSyntaxField<'source, T>,
        jolt_java_syntax::JavaSyntaxInvariantError,
    >,
    doc: &mut DocBuilder<'source>,
    present: impl FnOnce(T, &mut DocBuilder<'source>) -> Doc<'source>,
) -> Doc<'source> {
    match resolve_required_field(field, doc) {
        JavaFormatField::Present(value) => present(value, doc),
        JavaFormatField::Malformed(malformed) => malformed,
    }
}

fn optional_doc<'source, T>(
    field: Result<
        jolt_java_syntax::JavaSyntaxField<'source, T>,
        jolt_java_syntax::JavaSyntaxInvariantError,
    >,
    doc: &mut DocBuilder<'source>,
    present: impl FnOnce(T, &mut DocBuilder<'source>) -> Doc<'source>,
) -> Doc<'source> {
    match resolve_optional_field(field, doc) {
        JavaFormatField::Present(Some(value)) => present(value, doc),
        JavaFormatField::Present(None) => Doc::nil(),
        JavaFormatField::Malformed(malformed) => malformed,
    }
}

fn optional_doc_with_presence<'source, T>(
    field: Result<
        jolt_java_syntax::JavaSyntaxField<'source, T>,
        jolt_java_syntax::JavaSyntaxInvariantError,
    >,
    doc: &mut DocBuilder<'source>,
    present: impl FnOnce(T, &mut DocBuilder<'source>) -> Doc<'source>,
) -> (Doc<'source>, bool) {
    match resolve_optional_field(field, doc) {
        JavaFormatField::Present(Some(value)) => (present(value, doc), true),
        JavaFormatField::Present(None) => (Doc::nil(), false),
        JavaFormatField::Malformed(malformed) => (malformed, true),
    }
}

fn keyword_with_space<'source>(
    keyword: Option<JavaSyntaxToken<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    keyword.map_or_else(Doc::nil, |keyword| {
        doc_concat!(
            doc,
            [
                format_token_after_relocated_leading_comments(
                    doc,
                    &keyword,
                    TrailingTrivia::Preserve
                ),
                doc.space()
            ]
        )
    })
}

fn keyword_without_space<'source>(
    keyword: JavaSyntaxToken<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    format_token_after_relocated_leading_comments(doc, &keyword, TrailingTrivia::Preserve)
}

fn format_record_components<'source>(
    open: JavaFormatDelimiter<'source>,
    components: JavaFormatField<'source, Option<jolt_java_syntax::RecordComponentList<'source>>>,
    close: JavaFormatDelimiter<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    match components {
        JavaFormatField::Present(Some(components)) => {
            let parts = components.parts();
            let (lower, _) = parts.size_hint();
            let mut items = Vec::with_capacity(lower);
            for part in parts {
                match resolve_list_part(part, doc) {
                    JavaFormatListPart::Item(component) => items.push(CommaListItem {
                        doc: format_record_component(&component, doc),
                        comma: None,
                    }),
                    JavaFormatListPart::Separator(comma) => {
                        if let Some(item) = items.last_mut() {
                            item.comma = Some(comma);
                        } else {
                            doc.block_on_invariant(
                                "record component separator had no preceding component",
                            );
                        }
                    }
                    JavaFormatListPart::Malformed(malformed) => items.push(CommaListItem {
                        doc: malformed,
                        comma: None,
                    }),
                }
            }
            parenthesized_list(doc, open, close, items)
        }
        JavaFormatField::Present(None) => parenthesized_list(doc, open, close, []),
        JavaFormatField::Malformed(malformed) => parenthesized_list(
            doc,
            open,
            close,
            [CommaListItem {
                doc: malformed,
                comma: None,
            }],
        ),
    }
}

fn format_extends<'source>(
    clause: &ExtendsClause<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let keyword = resolve_required_field(clause.extends_keyword(), doc);
    let types = resolve_required_field(clause.types(), doc);
    format_type_clause_fields(keyword, types, doc)
}

fn format_implements<'source>(
    clause: &ImplementsClause<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let keyword = resolve_required_field(clause.implements_keyword(), doc);
    let types = resolve_required_field(clause.types(), doc);
    format_type_clause_fields(keyword, types, doc)
}

fn format_permits<'source>(
    clause: &PermitsClause<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let keyword = resolve_required_field(clause.permits_keyword(), doc);
    let names = match resolve_required_field(clause.names(), doc) {
        JavaFormatField::Present(names) => names,
        JavaFormatField::Malformed(names) => {
            return format_missing_clause_target(keyword, names, doc);
        }
    };
    let parts = names.parts();
    let (lower, _) = parts.size_hint();
    let mut items = Vec::with_capacity(lower);
    for part in parts {
        match resolve_list_part(part, doc) {
            JavaFormatListPart::Item(name) => items.push(CommaListItem {
                doc: doc_concat!(
                    doc,
                    [
                        format_construct_leading_comments(doc, name.first_token().as_ref()),
                        format_name(&name, doc)
                    ]
                ),
                comma: None,
            }),
            JavaFormatListPart::Separator(comma) => {
                if let Some(item) = items.last_mut() {
                    item.comma = Some(comma);
                }
            }
            JavaFormatListPart::Malformed(malformed) => items.push(CommaListItem {
                doc: malformed,
                comma: None,
            }),
        }
    }
    match keyword {
        JavaFormatField::Present(keyword) => header_clause(Some(&keyword), items, doc),
        JavaFormatField::Malformed(keyword) => {
            doc_concat!(doc, [keyword, header_clause(None, items, doc)])
        }
    }
}

fn format_type_clause_fields<'source>(
    keyword: JavaFormatField<'source, JavaSyntaxToken<'source>>,
    types: JavaFormatField<'source, jolt_java_syntax::TypeList<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    match (keyword, types) {
        (JavaFormatField::Present(keyword), JavaFormatField::Present(types)) => {
            format_type_clause(Some(&keyword), types.parts(), doc)
        }
        (keyword, JavaFormatField::Present(types)) => doc_concat!(
            doc,
            [
                field_token_with_space(keyword, doc),
                format_type_clause(None, types.parts(), doc)
            ]
        ),
        (keyword, JavaFormatField::Malformed(types)) => {
            format_missing_clause_target(keyword, types, doc)
        }
    }
}

fn format_missing_clause_target<'source>(
    keyword: JavaFormatField<'source, JavaSyntaxToken<'source>>,
    missing: Doc<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let keyword = match keyword {
        JavaFormatField::Present(keyword) => format_token(
            doc,
            &keyword,
            LeadingTrivia::Preserve,
            TrailingTrivia::BeforeLineBreak,
        ),
        JavaFormatField::Malformed(malformed) => malformed,
    };
    doc_indent!(doc, doc_concat!(doc, [doc.line(), keyword, missing]))
}

fn field_token_with_space<'source>(
    field: JavaFormatField<'source, JavaSyntaxToken<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    match field {
        JavaFormatField::Present(token) => keyword_with_space(Some(token), doc),
        JavaFormatField::Malformed(malformed) => malformed,
    }
}

fn format_type_clause<'source>(
    keyword: Option<&JavaSyntaxToken<'source>>,
    parts: impl IntoIterator<
        Item = Result<
            jolt_java_syntax::JavaSyntaxListPart<'source, jolt_java_syntax::Type<'source>>,
            jolt_java_syntax::JavaSyntaxInvariantError,
        >,
    >,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let parts = parts.into_iter();
    let (lower, _) = parts.size_hint();
    let mut items = Vec::with_capacity(lower);
    for part in parts {
        match resolve_list_part(part, doc) {
            JavaFormatListPart::Item(ty) => items.push(CommaListItem {
                doc: doc_concat!(
                    doc,
                    [
                        format_construct_leading_comments(doc, ty.first_token().as_ref()),
                        format_type_without_leading_comments(&ty, doc)
                    ]
                ),
                comma: None,
            }),
            JavaFormatListPart::Separator(comma) => {
                if let Some(item) = items.last_mut() {
                    item.comma = Some(comma);
                }
            }
            JavaFormatListPart::Malformed(malformed) => items.push(CommaListItem {
                doc: malformed,
                comma: None,
            }),
        }
    }
    header_clause(keyword, items, doc)
}

fn header_clause<'source>(
    keyword: Option<&JavaSyntaxToken<'source>>,
    items: Vec<CommaListItem<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let keyword_doc = keyword.map_or_else(Doc::nil, |keyword| {
        format_token(
            doc,
            keyword,
            LeadingTrivia::Preserve,
            TrailingTrivia::BeforeLineBreak,
        )
    });
    if items.is_empty() {
        return doc_indent!(doc, doc_concat!(doc, [doc.line(), keyword_doc]));
    }
    let break_doc = if keyword.is_some_and(|token| {
        token
            .trailing_comments()
            .any(|comment| comment_forces_line(&comment))
    }) {
        doc.hard_line()
    } else {
        doc.line()
    };
    doc_indent!(
        doc,
        doc_concat!(
            doc,
            [
                doc.line(),
                keyword_doc,
                doc_indent!(
                    doc,
                    doc_group!(doc, doc_concat!(doc, [break_doc, comma_list(doc, items)]))
                )
            ]
        )
    )
}
