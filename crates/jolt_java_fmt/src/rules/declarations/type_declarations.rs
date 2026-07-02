use super::{
    AnnotationInterfaceDeclaration, ClassDeclaration, CommaListItem, Doc, EnumDeclaration,
    ExtendsClause, ImplementsClause, InterfaceDeclaration, JavaFormatter, JavaSyntaxToken,
    ModifierList, PermitsClause, PermitsClauseEntry, RecordComponentList, RecordDeclaration,
    TypeClauseEntry, comment_forces_line, concat, declaration_with_body,
    format_annotation_interface_body, format_class_body, format_construct_leading_comments,
    format_enum_body_contents, format_enum_constant_entry, format_interface_body,
    format_leading_comment_list, format_leading_comments, format_modifier_prefix, format_name,
    format_record_body, format_record_component, format_token_text,
    format_trailing_comments_before_line_break, format_type_parameter_list,
    format_type_without_leading_comments, group, hard_line, line, parenthesized_list, text,
};

pub(super) fn format_class_declaration(
    class: &ClassDeclaration,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    format_type_declaration_with_body(
        &class.tokens(),
        class.modifiers(),
        concat([
            text("class "),
            class
                .name()
                .map_or_else(jolt_fmt_ir::nil, |name| format_token_text(name.text())),
            format_type_parameter_list(class.type_parameters(), formatter),
            format_extends_clause(class.extends_clause(), formatter),
            format_implements_clause(class.implements_clause(), formatter),
            format_permits_clause(class.permits_clause(), formatter),
        ]),
        class
            .body()
            .and_then(|body| format_class_body(&body, formatter)),
        formatter,
    )
}

pub(super) fn format_interface_declaration(
    interface: &InterfaceDeclaration,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    format_type_declaration_with_body(
        &interface.tokens(),
        interface.modifiers(),
        concat([
            text("interface "),
            interface
                .name()
                .map_or_else(jolt_fmt_ir::nil, |name| format_token_text(name.text())),
            format_type_parameter_list(interface.type_parameters(), formatter),
            format_extends_clause(interface.extends_clause(), formatter),
            format_permits_clause(interface.permits_clause(), formatter),
        ]),
        interface
            .body()
            .and_then(|body| format_interface_body(&body, formatter)),
        formatter,
    )
}

pub(super) fn format_record_declaration(
    record: &RecordDeclaration,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    format_type_declaration_with_body(
        &record.tokens(),
        record.modifiers(),
        group(concat([
            text("record "),
            record
                .name()
                .map_or_else(jolt_fmt_ir::nil, |name| format_token_text(name.text())),
            format_type_parameter_list(record.type_parameters(), formatter),
            format_record_components(record.components(), formatter),
            format_implements_clause(record.implements_clause(), formatter),
        ])),
        record
            .body()
            .and_then(|body| format_record_body(&body, formatter)),
        formatter,
    )
}

pub(super) fn format_enum_declaration(
    enum_: &EnumDeclaration,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    let constants = enum_
        .body()
        .and_then(|body| body.constants())
        .map(|constants| {
            constants
                .entries()
                .map(|entry| format_enum_constant_entry(entry, formatter))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let body_doc = enum_
        .body()
        .and_then(|body| format_enum_body_contents(constants, &body, formatter));

    format_type_declaration_with_body(
        &enum_.tokens(),
        enum_.modifiers(),
        concat([
            text("enum "),
            enum_
                .name()
                .map_or_else(jolt_fmt_ir::nil, |name| format_token_text(name.text())),
            format_implements_clause(enum_.implements_clause(), formatter),
        ]),
        body_doc,
        formatter,
    )
}

pub(super) fn format_annotation_interface_declaration(
    annotation: &AnnotationInterfaceDeclaration,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    format_type_declaration_with_body(
        &annotation.tokens(),
        annotation.modifiers(),
        concat([
            text("@interface "),
            annotation
                .name()
                .map_or_else(jolt_fmt_ir::nil, |name| format_token_text(name.text())),
        ]),
        annotation
            .body()
            .and_then(|body| format_annotation_interface_body(&body, formatter)),
        formatter,
    )
}

fn format_type_declaration_with_body(
    tokens: &[jolt_java_syntax::JavaSyntaxToken],
    modifiers: Option<ModifierList>,
    header_tail: Doc,
    body: Option<Doc>,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    declaration_with_body(
        concat([
            format_leading_comment_list(formatter.comments().leading_comments_for_tokens(tokens)),
            format_modifier_prefix(modifiers, formatter),
        ]),
        header_tail,
        body,
    )
}

fn format_record_components(
    components: Option<RecordComponentList>,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    let Some(components) = components else {
        return text("()");
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

fn format_extends_clause(clause: Option<ExtendsClause>, formatter: &JavaFormatter<'_>) -> Doc {
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

fn format_implements_clause(
    clause: Option<ImplementsClause>,
    formatter: &JavaFormatter<'_>,
) -> Doc {
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

fn format_permits_clause(clause: Option<PermitsClause>, formatter: &JavaFormatter<'_>) -> Doc {
    let Some(clause) = clause else {
        return jolt_fmt_ir::nil();
    };
    let keyword = clause.keyword();
    format_permits_header_clause(
        keyword.as_ref(),
        "permits",
        clause.entries().collect::<Vec<_>>(),
        formatter,
    )
}

fn format_type_header_clause(
    keyword: Option<&JavaSyntaxToken>,
    fallback: &'static str,
    entries: Vec<TypeClauseEntry>,
    formatter: &JavaFormatter<'_>,
) -> Doc {
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

fn format_permits_header_clause(
    keyword: Option<&JavaSyntaxToken>,
    fallback: &'static str,
    entries: Vec<PermitsClauseEntry>,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    if entries.is_empty() {
        return jolt_fmt_ir::nil();
    }

    jolt_fmt_ir::indent(concat([
        line(),
        format_header_clause_keyword(keyword, fallback),
        jolt_fmt_ir::indent(group(concat([
            format_header_clause_keyword_break(keyword),
            format_permits_clause_entries_broken(entries, formatter),
        ]))),
    ]))
}

fn format_header_clause_keyword(keyword: Option<&JavaSyntaxToken>, fallback: &'static str) -> Doc {
    keyword.map_or_else(
        || text(fallback),
        |keyword| {
            concat([
                format_leading_comments(keyword),
                format_token_text(keyword.text()),
                format_trailing_comments_before_line_break(keyword),
            ])
        },
    )
}

fn format_header_clause_keyword_break(keyword: Option<&JavaSyntaxToken>) -> Doc {
    if header_keyword_forces_line(keyword) {
        hard_line()
    } else {
        line()
    }
}

fn header_keyword_forces_line(keyword: Option<&JavaSyntaxToken>) -> bool {
    keyword.is_some_and(|keyword| keyword.trailing_comments().iter().any(comment_forces_line))
}

fn format_type_clause_entries_broken(
    entries: Vec<TypeClauseEntry>,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    let mut docs = Vec::new();
    let entries_len = entries.len();

    for (index, entry) in entries.into_iter().enumerate() {
        docs.push(concat([
            format_construct_leading_comments(formatter.comments(), &entry.ty.tokens()),
            format_type_without_leading_comments(&entry.ty, formatter),
        ]));
        if let Some(comma) = entry.comma {
            docs.push(format_header_clause_separator_broken(&comma));
        } else if index + 1 < entries_len {
            docs.push(line());
        }
    }

    concat(docs)
}

fn format_permits_clause_entries_broken(
    entries: Vec<PermitsClauseEntry>,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    let mut docs = Vec::new();
    let entries_len = entries.len();

    for (index, entry) in entries.into_iter().enumerate() {
        docs.push(concat([
            format_construct_leading_comments(formatter.comments(), &entry.name.tokens()),
            format_name(&entry.name),
        ]));
        if let Some(comma) = entry.comma {
            docs.push(format_header_clause_separator_broken(&comma));
        } else if index + 1 < entries_len {
            docs.push(line());
        }
    }

    concat(docs)
}

fn format_header_clause_separator_broken(comma: &JavaSyntaxToken) -> Doc {
    concat([
        format_leading_comments(comma),
        text(","),
        format_trailing_comments_before_line_break(comma),
        if comma.trailing_comments().iter().any(comment_forces_line) {
            hard_line()
        } else {
            line()
        },
    ])
}
