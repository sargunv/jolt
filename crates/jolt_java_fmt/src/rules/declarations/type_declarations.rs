use super::{
    AnnotationInterfaceDeclaration, ClassDeclaration, CommaListItem, Doc, EnumDeclaration,
    ExtendsClause, ImplementsClause, InterfaceDeclaration, JavaFormatter, JavaSyntaxToken,
    LeadingTrivia, ModifierList, PermitsClause, PermitsClauseEntry, RecordDeclaration,
    TrailingTrivia, TypeClauseEntry, comment_forces_line, concat, declaration_with_body,
    format_annotation_interface_body, format_class_body, format_construct_leading_comments,
    format_enum_body_contents, format_enum_constant_entry, format_interface_body,
    format_leading_comment_list, format_modifier_prefix, format_name, format_record_body,
    format_record_component, format_separator_with_comments, format_token,
    format_token_with_comments, format_type_parameter_list, format_type_without_leading_comments,
    group, hard_line, line, parenthesized_list, text,
};
use crate::helpers::comments::format_token_after_relocated_leading_comments;

pub(super) fn format_class_declaration<'source>(
    class: &ClassDeclaration<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    format_type_declaration_with_body(
        class.first_token().as_ref(),
        class.modifiers(),
        concat([
            format_keyword_with_space(class.keyword()),
            class
                .name()
                .map_or_else(jolt_fmt_ir::nil, |name| format_token_with_comments(&name)),
            format_type_parameter_list(class.type_parameters(), formatter),
            format_extends_clause(class.extends_clause(), formatter),
            format_implements_clause(class.implements_clause(), formatter),
            format_permits_clause(class.permits_clause()),
        ]),
        class
            .body()
            .and_then(|body| format_class_body(&body, formatter)),
        formatter,
    )
}

pub(super) fn format_interface_declaration<'source>(
    interface: &InterfaceDeclaration<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    format_type_declaration_with_body(
        interface.first_token().as_ref(),
        interface.modifiers(),
        concat([
            format_keyword_with_space(interface.keyword()),
            interface
                .name()
                .map_or_else(jolt_fmt_ir::nil, |name| format_token_with_comments(&name)),
            format_type_parameter_list(interface.type_parameters(), formatter),
            format_extends_clause(interface.extends_clause(), formatter),
            format_permits_clause(interface.permits_clause()),
        ]),
        interface
            .body()
            .and_then(|body| format_interface_body(&body, formatter)),
        formatter,
    )
}

pub(super) fn format_record_declaration<'source>(
    record: &RecordDeclaration<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    format_type_declaration_with_body(
        record.first_token().as_ref(),
        record.modifiers(),
        group(concat([
            format_keyword_with_space(record.keyword()),
            record
                .name()
                .map_or_else(jolt_fmt_ir::nil, |name| format_token_with_comments(&name)),
            format_type_parameter_list(record.type_parameters(), formatter),
            format_record_components(record, formatter),
            format_implements_clause(record.implements_clause(), formatter),
        ])),
        record
            .body()
            .and_then(|body| format_record_body(&body, formatter)),
        formatter,
    )
}

pub(super) fn format_enum_declaration<'source>(
    enum_: &EnumDeclaration<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    let body = enum_.body();
    let constants = body
        .as_ref()
        .and_then(jolt_java_syntax::EnumBody::constants)
        .map(|constants| {
            constants
                .entries()
                .map(|entry| format_enum_constant_entry(&entry, formatter))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let body_doc = enum_
        .body()
        .and_then(|body| format_enum_body_contents(constants, &body, formatter));

    format_type_declaration_with_body(
        enum_.first_token().as_ref(),
        enum_.modifiers(),
        concat([
            format_keyword_with_space(enum_.keyword()),
            enum_
                .name()
                .map_or_else(jolt_fmt_ir::nil, |name| format_token_with_comments(&name)),
            format_implements_clause(enum_.implements_clause(), formatter),
        ]),
        body_doc,
        formatter,
    )
}

pub(super) fn format_annotation_interface_declaration<'source>(
    annotation: &AnnotationInterfaceDeclaration<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    format_type_declaration_with_body(
        annotation.first_token().as_ref(),
        annotation.modifiers(),
        concat([
            annotation
                .at_token()
                .map_or_else(jolt_fmt_ir::nil, |token| {
                    format_token_after_relocated_leading_comments(&token, TrailingTrivia::Preserve)
                }),
            format_keyword_with_space(annotation.interface_token()),
            annotation
                .name()
                .map_or_else(jolt_fmt_ir::nil, |name| format_token_with_comments(&name)),
        ]),
        annotation
            .body()
            .and_then(|body| format_annotation_interface_body(&body, formatter)),
        formatter,
    )
}

fn format_keyword_with_space(keyword: Option<JavaSyntaxToken<'_>>) -> Doc<'_> {
    keyword.map_or_else(jolt_fmt_ir::nil, |keyword| {
        concat([
            format_token_after_relocated_leading_comments(&keyword, TrailingTrivia::Preserve),
            text(" "),
        ])
    })
}

fn format_type_declaration_with_body<'source>(
    first_token: Option<&jolt_java_syntax::JavaSyntaxToken<'source>>,
    modifiers: Option<ModifierList<'source>>,
    header_tail: Doc<'source>,
    body: Option<Doc<'source>>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    declaration_with_body(
        concat([
            format_leading_comment_list(
                first_token
                    .into_iter()
                    .flat_map(jolt_java_syntax::JavaSyntaxToken::leading_comments),
            ),
            format_modifier_prefix(modifiers, formatter),
        ]),
        header_tail,
        body,
    )
}

fn format_record_components<'source>(
    record: &RecordDeclaration<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    let Some(components) = record.components() else {
        let open = record.open_paren();
        let close = record.close_paren();
        return parenthesized_list(open.as_ref(), close.as_ref(), Vec::new());
    };

    let open = components.open_paren();
    let close = components.close_paren();
    parenthesized_list(
        open.as_ref(),
        close.as_ref(),
        components
            .entries()
            .map(|entry| CommaListItem {
                doc: format_record_component(&entry.component, formatter),
                comma: entry.comma,
            })
            .collect(),
    )
}

fn format_extends_clause<'source>(
    clause: Option<ExtendsClause<'source>>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    let Some(clause) = clause else {
        return jolt_fmt_ir::nil();
    };
    let keyword = clause.keyword();
    format_type_header_clause(
        keyword.as_ref(),
        "extends",
        clause.entries().collect::<Vec<_>>(),
        formatter,
    )
}

fn format_implements_clause<'source>(
    clause: Option<ImplementsClause<'source>>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    let Some(clause) = clause else {
        return jolt_fmt_ir::nil();
    };
    let keyword = clause.keyword();
    format_type_header_clause(
        keyword.as_ref(),
        "implements",
        clause.entries().collect::<Vec<_>>(),
        formatter,
    )
}

fn format_permits_clause(clause: Option<PermitsClause<'_>>) -> Doc<'_> {
    let Some(clause) = clause else {
        return jolt_fmt_ir::nil();
    };
    let keyword = clause.keyword();
    format_permits_header_clause(
        keyword.as_ref(),
        "permits",
        clause.entries().collect::<Vec<_>>(),
    )
}

fn format_type_header_clause<'source>(
    keyword: Option<&JavaSyntaxToken<'source>>,
    fallback: &'static str,
    entries: Vec<TypeClauseEntry<'source>>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    if entries.is_empty() {
        return jolt_fmt_ir::nil();
    }

    jolt_fmt_ir::indent(concat([
        line(),
        format_header_clause_keyword(keyword, fallback),
        jolt_fmt_ir::indent(group(concat([
            format_header_clause_keyword_break(keyword),
            format_type_clause_entries_broken(entries, formatter),
        ]))),
    ]))
}

fn format_permits_header_clause<'source>(
    keyword: Option<&JavaSyntaxToken<'source>>,
    fallback: &'static str,
    entries: Vec<PermitsClauseEntry<'source>>,
) -> Doc<'source> {
    if entries.is_empty() {
        return jolt_fmt_ir::nil();
    }

    jolt_fmt_ir::indent(concat([
        line(),
        format_header_clause_keyword(keyword, fallback),
        jolt_fmt_ir::indent(group(concat([
            format_header_clause_keyword_break(keyword),
            format_permits_clause_entries_broken(entries),
        ]))),
    ]))
}

fn format_header_clause_keyword<'source>(
    keyword: Option<&JavaSyntaxToken<'source>>,
    fallback: &'static str,
) -> Doc<'source> {
    keyword.map_or_else(
        || text(fallback),
        |keyword| {
            format_token(
                keyword,
                LeadingTrivia::Preserve,
                TrailingTrivia::BeforeLineBreak,
            )
        },
    )
}

fn format_header_clause_keyword_break<'source>(
    keyword: Option<&JavaSyntaxToken<'source>>,
) -> Doc<'source> {
    if header_keyword_forces_line(keyword) {
        hard_line()
    } else {
        line()
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
    entries: Vec<TypeClauseEntry<'source>>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    let mut docs = Vec::new();
    let entries_len = entries.len();

    for (index, entry) in entries.into_iter().enumerate() {
        docs.push(concat([
            format_construct_leading_comments(entry.ty.first_token().as_ref()),
            format_type_without_leading_comments(&entry.ty, formatter),
        ]));
        if let Some(comma) = entry.comma {
            docs.push(format_separator_with_comments(&comma, line()));
        } else if index + 1 < entries_len {
            docs.push(line());
        }
    }

    concat(docs)
}

fn format_permits_clause_entries_broken(entries: Vec<PermitsClauseEntry<'_>>) -> Doc<'_> {
    let mut docs = Vec::new();
    let entries_len = entries.len();

    for (index, entry) in entries.into_iter().enumerate() {
        docs.push(concat([
            format_construct_leading_comments(entry.name.first_token().as_ref()),
            format_name(&entry.name),
        ]));
        if let Some(comma) = entry.comma {
            docs.push(format_separator_with_comments(&comma, line()));
        } else if index + 1 < entries_len {
            docs.push(line());
        }
    }

    concat(docs)
}
