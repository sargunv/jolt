use super::{
    AnnotationInterfaceDeclaration, ClassDeclaration, CommaListItem, Doc, EnumDeclaration,
    ExtendsClause, ImplementsClause, InterfaceDeclaration, JavaSyntaxToken, LeadingTrivia,
    PermitsClause, RecordDeclaration, TrailingTrivia, TypeLeadingComments, attach_comma_separator,
    comma_list, comment_forces_line, delimited_comma_list, format_annotation_interface_body,
    format_class_body, format_construct_leading_comments, format_enum_body_contents,
    format_interface_body, format_modifier_prefix, format_name, format_record_body,
    format_record_component, format_token, format_token_after_construct_leading_comments,
    format_token_with_comments, format_type_parameter_list, format_type_without_leading_comments,
    source_braced_body,
};
use crate::helpers::recovery::{
    JavaFormatDelimiter, JavaFormatField, JavaFormatListPart, format_optional_field,
    format_required_field, resolve_list_part, resolve_optional_field, resolve_required_delimiter,
    resolve_required_field,
};
use jolt_fmt_ir::DocBuilder;
use jolt_java_syntax::JavaSyntaxField;

pub(super) fn format_class_declaration<'source>(
    class: &ClassDeclaration<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let hoisted = hoisted_header_keyword(&class.modifiers(), class.class_keyword());
    let modifiers = format_optional_field(class.modifiers(), doc, |modifiers, doc| {
        format_modifier_prefix(Some(modifiers), doc)
    });
    let modifiers = {
        let leading = format_construct_leading_comments(doc, hoisted.as_ref());
        doc_concat!(doc, [leading, modifiers])
    };
    let keyword = format_required_field(class.class_keyword(), doc, |keyword, doc| {
        format_token_after_construct_leading_comments(doc, &keyword, hoisted.as_ref())
    });
    let (name, name_is_structured) = required_doc_with_presence(class.name(), doc, |name, doc| {
        format_token_with_comments(doc, &name)
    });
    let name_separator = structured_separator(name_is_structured, doc);
    let parameters = format_optional_field(class.type_parameters(), doc, |parameters, doc| {
        format_type_parameter_list(parameters, TypeLeadingComments::Preserve, doc)
    });
    let extends = format_optional_field(class.extends(), doc, |extends, doc| {
        format_extends(&extends, doc)
    });
    let implements = format_optional_field(class.implements(), doc, |implements, doc| {
        format_implements(&implements, doc)
    });
    let permits = format_optional_field(class.permits(), doc, |permits, doc| {
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
    let hoisted = hoisted_header_keyword(&interface.modifiers(), interface.interface_keyword());
    let modifiers = format_optional_field(interface.modifiers(), doc, |modifiers, doc| {
        format_modifier_prefix(Some(modifiers), doc)
    });
    let modifiers = {
        let leading = format_construct_leading_comments(doc, hoisted.as_ref());
        doc_concat!(doc, [leading, modifiers])
    };
    let keyword = format_required_field(interface.interface_keyword(), doc, |keyword, doc| {
        format_token_after_construct_leading_comments(doc, &keyword, hoisted.as_ref())
    });
    let (name, name_is_structured) =
        required_doc_with_presence(interface.name(), doc, |name, doc| {
            format_token_with_comments(doc, &name)
        });
    let name_separator = structured_separator(name_is_structured, doc);
    let parameters = format_optional_field(interface.type_parameters(), doc, |parameters, doc| {
        format_type_parameter_list(parameters, TypeLeadingComments::Preserve, doc)
    });
    let extends = format_optional_field(interface.extends(), doc, |extends, doc| {
        format_extends(&extends, doc)
    });
    let permits = format_optional_field(interface.permits(), doc, |permits, doc| {
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
    let hoisted = hoisted_header_keyword(&record.modifiers(), record.record_keyword());
    let modifiers = format_optional_field(record.modifiers(), doc, |modifiers, doc| {
        format_modifier_prefix(Some(modifiers), doc)
    });
    let modifiers = {
        let leading = format_construct_leading_comments(doc, hoisted.as_ref());
        doc_concat!(doc, [leading, modifiers])
    };
    let keyword = format_required_field(record.record_keyword(), doc, |keyword, doc| {
        format_token_after_construct_leading_comments(doc, &keyword, hoisted.as_ref())
    });
    let (name, name_is_structured) = required_doc_with_presence(record.name(), doc, |name, doc| {
        format_token_with_comments(doc, &name)
    });
    let name_separator = structured_separator(name_is_structured, doc);
    let parameters = format_optional_field(record.type_parameters(), doc, |parameters, doc| {
        format_type_parameter_list(parameters, TypeLeadingComments::Preserve, doc)
    });
    let component_open = resolve_required_delimiter(record.open_paren(), doc);
    let components = resolve_optional_field(record.components(), doc);
    let component_close = resolve_required_delimiter(record.close_paren(), doc);
    let component_doc = format_record_components(component_open, components, component_close, doc);
    let implements = format_optional_field(record.implements(), doc, |implements, doc| {
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
    let hoisted = hoisted_header_keyword(&node.modifiers(), node.enum_keyword());
    let modifiers = format_optional_field(node.modifiers(), doc, |modifiers, doc| {
        format_modifier_prefix(Some(modifiers), doc)
    });
    let modifiers = {
        let leading = format_construct_leading_comments(doc, hoisted.as_ref());
        doc_concat!(doc, [leading, modifiers])
    };
    let keyword = format_required_field(node.enum_keyword(), doc, |keyword, doc| {
        format_token_after_construct_leading_comments(doc, &keyword, hoisted.as_ref())
    });
    let (name, name_is_structured) = required_doc_with_presence(node.name(), doc, |name, doc| {
        format_token_with_comments(doc, &name)
    });
    let name_separator = structured_separator(name_is_structured, doc);
    let implements = format_optional_field(node.implements(), doc, |implements, doc| {
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
    let modifiers = format_optional_field(node.modifiers(), doc, |modifiers, doc| {
        format_modifier_prefix(Some(modifiers), doc)
    });
    let at = format_required_field(node.at(), doc, |at, doc| {
        format_token_with_comments(doc, &at)
    });
    let interface = format_required_field(node.interface_keyword(), doc, |interface, doc| {
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
        modifiers,
        doc_concat!(doc, [at, interface, name_separator, name]),
        body,
        body_is_structured && !has_missing_body_semicolon,
        missing_body_semicolon,
        doc,
    )
}

fn type_with_body<'source>(
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
            modifiers,
            doc_group!(doc, header),
            body_separator,
            body,
            missing_body_semicolon,
        ]
    )
}

fn required_doc_with_presence<'source, T>(
    field: jolt_java_syntax::JavaSyntaxField<'source, T>,
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

fn optional_doc_with_presence<'source, T>(
    field: jolt_java_syntax::JavaSyntaxField<'source, T>,
    doc: &mut DocBuilder<'source>,
    present: impl FnOnce(T, &mut DocBuilder<'source>) -> Doc<'source>,
) -> (Doc<'source>, bool) {
    match resolve_optional_field(field, doc) {
        JavaFormatField::Present(Some(value)) => (present(value, doc), true),
        JavaFormatField::Present(None) => (Doc::nil(), false),
        JavaFormatField::Malformed(malformed) => (malformed, true),
    }
}

/// Returns the header keyword whose leading comments must be hoisted out of the
/// header group.
///
/// A leading comment ends its own line. Left inside the header group, that hard
/// line forces every clause in the header -- `extends`, `implements`, `throws`
/// -- to break with it, so a comment above a declaration silently changes its
/// layout. A modifier list already hoists its own leading comments, so the
/// keyword only needs to when no modifier list precedes it.
fn hoisted_header_keyword<'source, T>(
    modifiers: &JavaSyntaxField<'source, T>,
    keyword: JavaSyntaxField<'source, JavaSyntaxToken<'source>>,
) -> Option<JavaSyntaxToken<'source>> {
    if matches!(modifiers, JavaSyntaxField::Present(_)) {
        return None;
    }
    match keyword {
        JavaSyntaxField::Present(keyword) => Some(keyword),
        JavaSyntaxField::Missing(_) | JavaSyntaxField::Malformed(_) => None,
    }
}

fn keyword_without_space<'source>(
    keyword: JavaSyntaxToken<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    format_token_with_comments(doc, &keyword)
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
                    JavaFormatListPart::Item(component) => items.push(CommaListItem::visible(
                        format_record_component(&component, doc),
                    )),
                    JavaFormatListPart::Separator(comma) => {
                        attach_comma_separator(doc, &mut items, comma);
                    }
                    JavaFormatListPart::Recovery(malformed) => {
                        items.push(CommaListItem::recovery(malformed));
                    }
                }
            }
            delimited_comma_list(doc, open, close, items)
        }
        JavaFormatField::Present(None) => delimited_comma_list(doc, open, close, []),
        JavaFormatField::Malformed(malformed) => {
            delimited_comma_list(doc, open, close, [CommaListItem::visible(malformed)])
        }
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
            JavaFormatListPart::Item(name) => items.push(CommaListItem::visible(doc_concat!(
                doc,
                [
                    format_construct_leading_comments(doc, name.first_token().as_ref()),
                    format_name(&name, doc)
                ]
            ))),
            JavaFormatListPart::Separator(comma) => {
                attach_comma_separator(doc, &mut items, comma);
            }
            JavaFormatListPart::Recovery(malformed) => {
                items.push(CommaListItem::recovery(malformed));
            }
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
        (JavaFormatField::Malformed(keyword), JavaFormatField::Present(types)) => {
            doc_concat!(doc, [keyword, format_type_clause(None, types.parts(), doc)])
        }
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

fn format_type_clause<'source>(
    keyword: Option<&JavaSyntaxToken<'source>>,
    parts: impl IntoIterator<
        Item = jolt_java_syntax::JavaSyntaxListPart<'source, jolt_java_syntax::Type<'source>>,
    >,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let parts = parts.into_iter();
    let (lower, _) = parts.size_hint();
    let mut items = Vec::with_capacity(lower);
    for part in parts {
        match resolve_list_part(part, doc) {
            JavaFormatListPart::Item(ty) => items.push(CommaListItem::visible(doc_concat!(
                doc,
                [
                    format_construct_leading_comments(doc, ty.first_token().as_ref()),
                    format_type_without_leading_comments(&ty, doc)
                ]
            ))),
            JavaFormatListPart::Separator(comma) => {
                attach_comma_separator(doc, &mut items, comma);
            }
            JavaFormatListPart::Recovery(malformed) => {
                items.push(CommaListItem::recovery(malformed));
            }
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
