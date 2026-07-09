use super::{
    AnnotationInterfaceDeclaration, ClassDeclaration, CommaListItem, Doc, EnumDeclaration,
    ExtendsClause, ImplementsClause, InterfaceDeclaration, JavaSyntaxToken, LeadingTrivia,
    ModifierList, PermitsClause, PermitsClauseEntry, RecordDeclaration, TrailingTrivia,
    TypeClauseEntry, comma_list, comment_forces_line, format_annotation_interface_body,
    format_class_body, format_construct_leading_comments, format_enum_body_contents,
    format_interface_body, format_leading_comment_list, format_modifier_prefix, format_name,
    format_record_body, format_record_component, format_token, format_token_with_comments,
    format_type_parameter_list, format_type_without_leading_comments, parenthesized_list,
    recovered_comma_list_items, source_braced_body,
};
use crate::helpers::comments::format_token_after_relocated_leading_comments;
use jolt_fmt_ir::DocBuilder;
use jolt_java_syntax::RecoveredSeparatedListEntry;

pub(super) fn format_class_declaration<'source>(
    class: &ClassDeclaration<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let body = class.body();
    let open = body
        .as_ref()
        .and_then(jolt_java_syntax::ClassBody::open_brace);
    let close = body
        .as_ref()
        .and_then(jolt_java_syntax::ClassBody::close_brace);
    let body_doc = body.as_ref().and_then(|body| format_class_body(body, doc));
    format_type_declaration_with_body(
        class.first_token().as_ref(),
        class.modifiers(),
        doc_concat!(
            doc,
            [
                format_keyword_with_space(class.keyword(), doc),
                class
                    .name()
                    .map_or_else(Doc::nil, |name| format_token_with_comments(doc, &name)),
                format_type_parameter_list(class.type_parameters(), doc),
                format_extends_clause(class.extends_clause(), doc),
                format_implements_clause(class.implements_clause(), doc),
                format_permits_clause(class.permits_clause(), doc),
            ]
        ),
        open,
        close,
        body_doc,
        doc,
    )
}

pub(super) fn format_interface_declaration<'source>(
    interface: &InterfaceDeclaration<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let body = interface.body();
    let open = body
        .as_ref()
        .and_then(jolt_java_syntax::InterfaceBody::open_brace);
    let close = body
        .as_ref()
        .and_then(jolt_java_syntax::InterfaceBody::close_brace);
    let body_doc = body
        .as_ref()
        .and_then(|body| format_interface_body(body, doc));
    format_type_declaration_with_body(
        interface.first_token().as_ref(),
        interface.modifiers(),
        doc_concat!(
            doc,
            [
                format_keyword_with_space(interface.keyword(), doc),
                interface
                    .name()
                    .map_or_else(Doc::nil, |name| format_token_with_comments(doc, &name)),
                format_type_parameter_list(interface.type_parameters(), doc),
                format_extends_clause(interface.extends_clause(), doc),
                format_permits_clause(interface.permits_clause(), doc),
            ]
        ),
        open,
        close,
        body_doc,
        doc,
    )
}

pub(super) fn format_record_declaration<'source>(
    record: &RecordDeclaration<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let body = record.body();
    let open = body
        .as_ref()
        .and_then(jolt_java_syntax::RecordBody::open_brace);
    let close = body
        .as_ref()
        .and_then(jolt_java_syntax::RecordBody::close_brace);
    let body_doc = body.as_ref().and_then(|body| format_record_body(body, doc));
    format_type_declaration_with_body(
        record.first_token().as_ref(),
        record.modifiers(),
        doc_group!(
            doc,
            doc_concat!(
                doc,
                [
                    format_keyword_with_space(record.keyword(), doc),
                    record
                        .name()
                        .map_or_else(Doc::nil, |name| format_token_with_comments(doc, &name)),
                    format_type_parameter_list(record.type_parameters(), doc),
                    format_record_components(record, doc),
                    format_implements_clause(record.implements_clause(), doc),
                ]
            ),
        ),
        open,
        close,
        body_doc,
        doc,
    )
}

pub(super) fn format_enum_declaration<'source>(
    enum_: &EnumDeclaration<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let body = enum_.body();
    let open = body
        .as_ref()
        .and_then(jolt_java_syntax::EnumBody::open_brace);
    let close = body
        .as_ref()
        .and_then(jolt_java_syntax::EnumBody::close_brace);
    let body_doc = body
        .as_ref()
        .and_then(|body| format_enum_body_contents(body, doc));

    format_type_declaration_with_body(
        enum_.first_token().as_ref(),
        enum_.modifiers(),
        doc_concat!(
            doc,
            [
                format_keyword_with_space(enum_.keyword(), doc),
                enum_
                    .name()
                    .map_or_else(Doc::nil, |name| format_token_with_comments(doc, &name)),
                format_implements_clause(enum_.implements_clause(), doc),
            ]
        ),
        open,
        close,
        body_doc,
        doc,
    )
}

pub(super) fn format_annotation_interface_declaration<'source>(
    annotation: &AnnotationInterfaceDeclaration<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let body = annotation.body();
    let open = body
        .as_ref()
        .and_then(jolt_java_syntax::AnnotationInterfaceBody::open_brace);
    let close = body
        .as_ref()
        .and_then(jolt_java_syntax::AnnotationInterfaceBody::close_brace);
    let body_doc = body
        .as_ref()
        .and_then(|body| format_annotation_interface_body(body, doc));

    format_type_declaration_with_body(
        annotation.first_token().as_ref(),
        annotation.modifiers(),
        doc_concat!(
            doc,
            [
                annotation.at_token().map_or_else(Doc::nil, |token| {
                    format_token_after_relocated_leading_comments(
                        doc,
                        &token,
                        TrailingTrivia::Preserve,
                    )
                },),
                format_keyword_with_space(annotation.interface_token(), doc),
                annotation
                    .name()
                    .map_or_else(Doc::nil, |name| format_token_with_comments(doc, &name)),
            ]
        ),
        open,
        close,
        body_doc,
        doc,
    )
}

fn format_keyword_with_space<'source>(
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
                    TrailingTrivia::Preserve,
                ),
                doc.space(),
            ]
        )
    })
}

fn format_type_declaration_with_body<'source>(
    first_token: Option<&jolt_java_syntax::JavaSyntaxToken<'source>>,
    modifiers: Option<ModifierList<'source>>,
    header_tail: Doc<'source>,
    open_brace: Option<JavaSyntaxToken<'source>>,
    close_brace: Option<JavaSyntaxToken<'source>>,
    body: Option<Doc<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    doc_concat!(
        doc,
        [
            doc_concat!(
                doc,
                [
                    format_leading_comment_list(
                        doc,
                        first_token
                            .into_iter()
                            .flat_map(jolt_java_syntax::JavaSyntaxToken::leading_comments),
                    ),
                    format_modifier_prefix(modifiers, doc),
                ]
            ),
            doc_group!(doc, header_tail),
            doc.space(),
            source_braced_body(doc, open_brace.as_ref(), close_brace.as_ref(), body),
        ]
    )
}

fn format_record_components<'source>(
    record: &RecordDeclaration<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let Some(components) = record.components() else {
        let open = record.open_paren();
        let close = record.close_paren();
        return parenthesized_list(doc, open.as_ref(), close.as_ref(), std::iter::empty());
    };

    let open = components.open_paren();
    let close = components.close_paren();
    let items = record_component_list_items(&components, doc);
    parenthesized_list(doc, open.as_ref(), close.as_ref(), items)
}

fn record_component_list_items<'source, 'fmt>(
    components: &'fmt jolt_java_syntax::RecordComponentList<'source>,
    doc: &'fmt mut DocBuilder<'source>,
) -> Vec<CommaListItem<'source>> {
    recovered_comma_list_items(doc, components.entries_with_recovered(), |entry, doc| {
        CommaListItem {
            doc: format_record_component(&entry.component, doc),
            comma: entry.comma,
        }
    })
}

fn format_extends_clause<'source>(
    clause: Option<ExtendsClause<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let Some(clause) = clause else {
        return Doc::nil();
    };
    let keyword = clause.keyword();
    format_type_header_clause(
        keyword.as_ref(),
        "extends",
        clause.entries_with_recovered(),
        doc,
    )
}

fn format_implements_clause<'source>(
    clause: Option<ImplementsClause<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let Some(clause) = clause else {
        return Doc::nil();
    };
    let keyword = clause.keyword();
    format_type_header_clause(
        keyword.as_ref(),
        "implements",
        clause.entries_with_recovered(),
        doc,
    )
}

fn format_permits_clause<'source>(
    clause: Option<PermitsClause<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let Some(clause) = clause else {
        return Doc::nil();
    };
    let keyword = clause.keyword();
    format_permits_header_clause(
        keyword.as_ref(),
        "permits",
        clause.entries_with_recovered(),
        doc,
    )
}

fn format_type_header_clause<'source>(
    keyword: Option<&JavaSyntaxToken<'source>>,
    fallback: &'static str,
    entries: impl IntoIterator<Item = RecoveredSeparatedListEntry<'source, TypeClauseEntry<'source>>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let mut entries = entries.into_iter().peekable();
    if entries.peek().is_none() {
        return doc_indent!(
            doc,
            doc_concat!(
                doc,
                [
                    doc.line(),
                    format_header_clause_keyword(keyword, fallback, doc),
                ]
            )
        );
    }

    doc_indent!(
        doc,
        doc_concat!(
            doc,
            [
                doc.line(),
                format_header_clause_keyword(keyword, fallback, doc),
                doc_indent!(
                    doc,
                    doc_group!(
                        doc,
                        doc_concat!(
                            doc,
                            [
                                format_header_clause_keyword_break(keyword, doc),
                                format_type_clause_entries_broken(entries, doc),
                            ]
                        )
                    )
                ),
            ]
        )
    )
}

fn format_permits_header_clause<'source>(
    keyword: Option<&JavaSyntaxToken<'source>>,
    fallback: &'static str,
    entries: impl IntoIterator<Item = RecoveredSeparatedListEntry<'source, PermitsClauseEntry<'source>>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let mut entries = entries.into_iter().peekable();
    if entries.peek().is_none() {
        return doc_indent!(
            doc,
            doc_concat!(
                doc,
                [
                    doc.line(),
                    format_header_clause_keyword(keyword, fallback, doc),
                ]
            )
        );
    }

    doc_indent!(
        doc,
        doc_concat!(
            doc,
            [
                doc.line(),
                format_header_clause_keyword(keyword, fallback, doc),
                doc_indent!(
                    doc,
                    doc_group!(
                        doc,
                        doc_concat!(
                            doc,
                            [
                                format_header_clause_keyword_break(keyword, doc),
                                format_permits_clause_entries_broken(entries, doc),
                            ]
                        )
                    )
                ),
            ]
        )
    )
}

fn format_header_clause_keyword<'source>(
    keyword: Option<&JavaSyntaxToken<'source>>,
    _fallback: &'static str,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    keyword.map_or_else(Doc::nil, |keyword| {
        format_token(
            doc,
            keyword,
            LeadingTrivia::Preserve,
            TrailingTrivia::BeforeLineBreak,
        )
    })
}

fn format_header_clause_keyword_break<'source>(
    keyword: Option<&JavaSyntaxToken<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    if header_keyword_forces_line(keyword) {
        doc.hard_line()
    } else {
        doc.line()
    }
}

fn header_keyword_forces_line(keyword: Option<&JavaSyntaxToken<'_>>) -> bool {
    keyword.is_some_and(|keyword| {
        keyword
            .trailing_comments()
            .any(|comment| comment_forces_line(&comment))
    })
}

fn format_type_clause_entries_broken<'source>(
    entries: impl IntoIterator<Item = RecoveredSeparatedListEntry<'source, TypeClauseEntry<'source>>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let items = recovered_comma_list_items(doc, entries, |entry, doc| CommaListItem {
        doc: doc_concat!(
            doc,
            [
                format_construct_leading_comments(doc, entry.ty.first_token().as_ref()),
                format_type_without_leading_comments(&entry.ty, doc),
            ]
        ),
        comma: entry.comma,
    });
    comma_list(doc, items)
}

fn format_permits_clause_entries_broken<'source>(
    entries: impl IntoIterator<Item = RecoveredSeparatedListEntry<'source, PermitsClauseEntry<'source>>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let items = recovered_comma_list_items(doc, entries, |entry, doc| CommaListItem {
        doc: doc_concat!(
            doc,
            [
                format_construct_leading_comments(doc, entry.name.first_token().as_ref()),
                format_name(&entry.name, doc),
            ]
        ),
        comma: entry.comma,
    });
    comma_list(doc, items)
}
