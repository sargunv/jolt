use super::{
    AnnotationElementDeclaration, CommaListItem, Doc, FormalParameterList, JavaFormatter,
    JavaSyntaxToken, MethodDeclaration, ThrowsClause, ThrowsClauseEntry, braced_body,
    comment_forces_line, concat, declaration_with_body, declaration_without_body,
    format_annotation_element_value, format_array_dimensions, format_block_body,
    format_construct_leading_comments, format_constructor_body, format_formal_parameter,
    format_inline_annotations, format_leading_comments, format_modifier_prefix,
    format_receiver_parameter, format_statement_semicolon, format_token_sequence,
    format_token_text, format_trailing_comments_before_line_break, format_type,
    format_type_parameter_list, format_type_without_leading_comments, format_typed_modifier_prefix,
    group, hard_line, line, parenthesized_list, text,
};

pub(super) fn format_constructor_declaration(
    constructor: &jolt_java_syntax::ConstructorDeclaration,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    let Some(name) = constructor.name() else {
        return format_token_sequence(&constructor.tokens());
    };
    let prefix = concat([
        format_construct_leading_comments(formatter.comments(), &constructor.tokens()),
        format_modifier_prefix(constructor.modifiers(), formatter),
    ]);
    let throws = constructor.throws_clause();
    let has_throws = throws
        .as_ref()
        .is_some_and(|throws| throws.exceptions().next().is_some());
    let header = concat([
        format_type_parameter_list(constructor.type_parameters(), formatter),
        format_token_text(name.text()),
        format_parameters(constructor.parameters(), formatter),
        format_throws_clause(throws, formatter),
    ]);

    match constructor.body() {
        Some(body) if has_throws => {
            declaration_with_body(prefix, header, format_constructor_body(&body, formatter))
        }
        Some(body) => callable_declaration_with_body(
            prefix,
            header,
            format_constructor_body(&body, formatter),
        ),
        None => declaration_without_body(prefix, header),
    }
}

pub(super) fn format_compact_constructor_declaration(
    constructor: &jolt_java_syntax::CompactConstructorDeclaration,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    let prefix = format_modifier_prefix(constructor.modifiers(), formatter);
    let header = constructor
        .name()
        .map_or_else(jolt_fmt_ir::nil, |name| format_token_text(name.text()));

    match constructor.body() {
        Some(body) => {
            declaration_with_body(prefix, header, format_constructor_body(&body, formatter))
        }
        None => declaration_without_body(prefix, header),
    }
}

pub(super) fn format_method_declaration(
    method: &MethodDeclaration,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    let Some(name) = method.name() else {
        return format_token_sequence(&method.tokens());
    };
    let modifiers = format_typed_modifier_prefix(method.modifiers(), formatter);
    let prefix = concat([
        format_construct_leading_comments(formatter.comments(), &method.tokens()),
        modifiers.declaration_prefix,
    ]);
    let throws = method.throws_clause();
    let has_throws = throws
        .as_ref()
        .is_some_and(|throws| throws.exceptions().next().is_some());
    let header = concat([
        format_type_parameter_list(method.type_parameters(), formatter),
        modifiers.type_use_prefix,
        format_inline_annotations(method.return_type_annotations().collect(), formatter),
        method
            .return_type()
            .map_or_else(jolt_fmt_ir::nil, |return_type| {
                concat([
                    format_type_without_leading_comments(&return_type, formatter),
                    text(" "),
                ])
            }),
        format_token_text(name.text()),
        format_parameters(method.parameters(), formatter),
        format_throws_clause(throws, formatter),
    ]);

    match method.body() {
        Some(body) if has_throws => {
            declaration_with_body(prefix, header, format_block_body(&body, formatter))
        }
        Some(body) => {
            callable_declaration_with_body(prefix, header, format_block_body(&body, formatter))
        }
        None => concat([
            prefix,
            group(header),
            format_statement_semicolon(method.semicolon()),
        ]),
    }
}

pub(super) fn format_annotation_element_declaration(
    element: &AnnotationElementDeclaration,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    let Some(name) = element.name() else {
        return format_token_sequence(&element.tokens());
    };

    concat([
        group(concat([
            format_modifier_prefix(element.modifiers(), formatter),
            element
                .ty()
                .map_or_else(jolt_fmt_ir::nil, |ty| format_type(&ty, formatter)),
            text(" "),
            format_token_text(name.text()),
            text("()"),
            element
                .dimensions()
                .map_or_else(jolt_fmt_ir::nil, |dimensions| {
                    format_array_dimensions(&dimensions, formatter)
                }),
            format_annotation_element_default(element.default_value(), formatter),
        ])),
        format_statement_semicolon(element.semicolon()),
    ])
}

fn format_annotation_element_default(
    default: Option<jolt_java_syntax::DefaultValue>,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    default.map_or_else(jolt_fmt_ir::nil, |default| {
        concat([
            text(" "),
            text("default "),
            default.value().map_or_else(jolt_fmt_ir::nil, |value| {
                format_annotation_element_value(&value, formatter)
            }),
        ])
    })
}

fn format_parameters(
    parameters: Option<FormalParameterList>,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    let Some(parameters) = parameters else {
        return text("()");
    };
    let open = parameters.open_paren();
    let close = parameters.close_paren();
    parenthesized_list(
        open.as_ref(),
        close.as_ref(),
        parameters
            .entries()
            .map(|entry| CommaListItem {
                doc: match entry.item {
                    jolt_java_syntax::FormalParameterListItem::ReceiverParameter(parameter) => {
                        format_receiver_parameter(&parameter, formatter)
                    }
                    jolt_java_syntax::FormalParameterListItem::FormalParameter(parameter) => {
                        format_formal_parameter(&parameter, formatter)
                    }
                },
                comma: entry.comma,
            })
            .collect(),
    )
}

fn callable_declaration_with_body(prefix: Doc, header: Doc, body: Option<Doc>) -> Doc {
    concat([prefix, group(header), text(" "), braced_body(body)])
}

fn format_throws_clause(throws: Option<ThrowsClause>, formatter: &JavaFormatter<'_>) -> Doc {
    let Some(throws) = throws else {
        return jolt_fmt_ir::nil();
    };
    let entries = throws.entries().collect::<Vec<_>>();
    if entries.is_empty() {
        return jolt_fmt_ir::nil();
    }

    jolt_fmt_ir::indent(concat([
        line(),
        format_throws_keyword(&throws),
        format_throws_keyword_spacing(&throws),
        format_throws_entries(entries, formatter),
    ]))
}

fn format_throws_keyword(throws: &ThrowsClause) -> Doc {
    throws.keyword().map_or_else(
        || text("throws"),
        |keyword| {
            concat([
                format_leading_comments(&keyword),
                text("throws"),
                format_trailing_comments_before_line_break(&keyword),
            ])
        },
    )
}

fn format_throws_keyword_spacing(throws: &ThrowsClause) -> Doc {
    if throws
        .keyword()
        .is_some_and(|keyword| keyword.trailing_comments().iter().any(comment_forces_line))
    {
        hard_line()
    } else {
        text(" ")
    }
}

fn format_throws_entries(entries: Vec<ThrowsClauseEntry>, formatter: &JavaFormatter<'_>) -> Doc {
    let mut docs = Vec::new();
    let entries_len = entries.len();

    for (index, entry) in entries.into_iter().enumerate() {
        docs.push(format_type(&entry.exception, formatter));
        if let Some(comma) = entry.comma {
            docs.push(format_throws_separator(&comma));
        } else if index + 1 < entries_len {
            docs.push(line());
        }
    }

    concat(docs)
}

fn format_throws_separator(comma: &JavaSyntaxToken) -> Doc {
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
