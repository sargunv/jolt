use super::{
    AnnotationElementDeclaration, CommaListItem, Doc, FormalParameterList, JavaFormatter,
    JavaSyntaxToken, LeadingTrivia, MethodDeclaration, ThrowsClause, ThrowsClauseEntry,
    TrailingTrivia, braced_body, comment_forces_line, concat, format_annotation_element_value,
    format_array_dimensions, format_block, format_construct_leading_comments,
    format_constructor_body, format_formal_parameter, format_inline_annotations,
    format_modifier_prefix, format_receiver_parameter, format_separator_with_comments,
    format_statement_semicolon, format_token, format_token_after_construct_leading_comments,
    format_token_with_comments, format_type, format_type_parameter_list,
    format_type_without_leading_comments, format_typed_modifier_prefix, group, hard_line, line,
    parenthesized_list, text,
};

pub(super) fn format_constructor_declaration<'source>(
    constructor: &jolt_java_syntax::ConstructorDeclaration<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    let constructor_first_token = constructor.first_token();
    let prefix = concat([
        format_construct_leading_comments(constructor_first_token.as_ref()),
        format_modifier_prefix(constructor.modifiers(), formatter),
    ]);
    let throws = constructor.throws_clause();
    let type_parameters = constructor.type_parameters();
    let has_type_parameters = type_parameters.is_some();
    let header = concat([
        format_type_parameter_list(type_parameters, formatter),
        if has_type_parameters {
            text(" ")
        } else {
            jolt_fmt_ir::nil()
        },
        constructor.name().map_or_else(jolt_fmt_ir::nil, |name| {
            format_token_after_construct_leading_comments(&name, constructor_first_token.as_ref())
        }),
        format_parameters(
            constructor.open_paren(),
            constructor.close_paren(),
            constructor.parameters(),
            formatter,
        ),
        format_throws_clause(throws, formatter),
    ]);

    match constructor.body() {
        Some(body) => callable_declaration_with_body(
            prefix,
            header,
            format_constructor_body(&body, formatter),
        ),
        None => concat([prefix, group(header)]),
    }
}

pub(super) fn format_compact_constructor_declaration<'source>(
    constructor: &jolt_java_syntax::CompactConstructorDeclaration<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    let prefix = format_modifier_prefix(constructor.modifiers(), formatter);
    let header = constructor
        .name()
        .map_or_else(jolt_fmt_ir::nil, |name| format_token_with_comments(&name));

    match constructor.body() {
        Some(body) => callable_declaration_with_body(
            prefix,
            header,
            format_constructor_body(&body, formatter),
        ),
        None => concat([prefix, group(header)]),
    }
}

pub(super) fn format_method_declaration<'source>(
    method: &MethodDeclaration<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    let modifiers = format_typed_modifier_prefix(method.modifiers(), formatter);
    let prefix = concat([
        format_construct_leading_comments(method.first_token().as_ref()),
        modifiers.declaration_prefix,
    ]);
    let throws = method.throws_clause();
    let type_parameters = method.type_parameters();
    let has_type_parameters = type_parameters.is_some();
    let parameters = method.parameters();
    let name_and_parameters = concat([
        method
            .name()
            .map_or_else(jolt_fmt_ir::nil, |name| format_token_with_comments(&name)),
        format_parameters(
            method.open_paren(),
            method.close_paren(),
            parameters,
            formatter,
        ),
    ]);
    let header = concat([
        format_type_parameter_list(type_parameters, formatter),
        if has_type_parameters {
            text(" ")
        } else {
            jolt_fmt_ir::nil()
        },
        modifiers.type_use_prefix,
        format_inline_annotations(method.return_type_annotations(), formatter),
        method
            .return_type()
            .map_or_else(jolt_fmt_ir::nil, |return_type| {
                format_type_without_leading_comments(&return_type, formatter)
            }),
        text(" "),
        name_and_parameters,
        format_throws_clause(throws, formatter),
    ]);

    match method.body() {
        Some(body) => {
            callable_declaration_with_body_doc(prefix, header, format_block(&body, formatter))
        }
        None => concat([
            prefix,
            group(header),
            format_statement_semicolon(method.semicolon()),
        ]),
    }
}

pub(super) fn format_annotation_element_declaration<'source>(
    element: &AnnotationElementDeclaration<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    concat([
        group(concat([
            format_modifier_prefix(element.modifiers(), formatter),
            element
                .ty()
                .map_or_else(jolt_fmt_ir::nil, |ty| format_type(&ty, formatter)),
            text(" "),
            element
                .name()
                .map_or_else(jolt_fmt_ir::nil, |name| format_token_with_comments(&name)),
            format_empty_parameters(element.open_paren(), element.close_paren()),
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

fn format_annotation_element_default<'source>(
    default: Option<jolt_java_syntax::DefaultValue<'source>>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    default.map_or_else(jolt_fmt_ir::nil, |default| {
        concat([
            text(" "),
            default
                .default_token()
                .map_or_else(jolt_fmt_ir::nil, |token| {
                    concat([format_token_with_comments(&token), text(" ")])
                }),
            default.value().map_or_else(jolt_fmt_ir::nil, |value| {
                format_annotation_element_value(&value, formatter)
            }),
        ])
    })
}

fn format_parameters<'source>(
    open: Option<JavaSyntaxToken<'source>>,
    close: Option<JavaSyntaxToken<'source>>,
    parameters: Option<FormalParameterList<'source>>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    let open = parameters
        .as_ref()
        .and_then(FormalParameterList::open_paren)
        .or(open);
    let close = parameters
        .as_ref()
        .and_then(FormalParameterList::close_paren)
        .or(close);
    let Some(parameters) = parameters else {
        return format_empty_parameters(open, close);
    };

    parenthesized_list(
        open.as_ref(),
        close.as_ref(),
        parameters.entries().map(|entry| CommaListItem {
            doc: match entry.item {
                jolt_java_syntax::FormalParameterListItem::ReceiverParameter(parameter) => {
                    format_receiver_parameter(&parameter, formatter)
                }
                jolt_java_syntax::FormalParameterListItem::FormalParameter(parameter) => {
                    format_formal_parameter(&parameter, formatter)
                }
            },
            comma: entry.comma,
        }),
    )
}

fn format_empty_parameters<'source>(
    open: Option<JavaSyntaxToken<'source>>,
    close: Option<JavaSyntaxToken<'source>>,
) -> Doc<'source> {
    parenthesized_list(
        open.as_ref(),
        close.as_ref(),
        std::iter::empty::<CommaListItem<'source>>(),
    )
}

fn callable_declaration_with_body<'source>(
    prefix: Doc<'source>,
    header: Doc<'source>,
    body: Option<Doc<'source>>,
) -> Doc<'source> {
    concat([prefix, group(header), text(" "), braced_body(body)])
}

fn callable_declaration_with_body_doc<'source>(
    prefix: Doc<'source>,
    header: Doc<'source>,
    body: Doc<'source>,
) -> Doc<'source> {
    concat([prefix, group(header), text(" "), body])
}

fn format_throws_clause<'source>(
    throws: Option<ThrowsClause<'source>>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    let Some(throws) = throws else {
        return jolt_fmt_ir::nil();
    };
    let mut entries = throws.entries().peekable();
    if entries.peek().is_none() {
        return jolt_fmt_ir::nil();
    }

    jolt_fmt_ir::indent(concat([
        line(),
        format_throws_keyword(&throws),
        format_throws_keyword_spacing(&throws),
        format_throws_entries(entries, formatter),
    ]))
}

fn format_throws_keyword<'source>(throws: &ThrowsClause<'source>) -> Doc<'source> {
    throws.keyword().map_or_else(jolt_fmt_ir::nil, |keyword| {
        format_token(
            &keyword,
            LeadingTrivia::Preserve,
            TrailingTrivia::BeforeLineBreak,
        )
    })
}

fn format_throws_keyword_spacing<'source>(throws: &ThrowsClause<'source>) -> Doc<'source> {
    if throws.keyword().is_some_and(|keyword| {
        keyword
            .trailing_comments()
            .any(|comment| comment_forces_line(&comment))
    }) {
        hard_line()
    } else {
        text(" ")
    }
}

fn format_throws_entries<'source>(
    entries: impl IntoIterator<Item = ThrowsClauseEntry<'source>>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    let mut entries = entries.into_iter().peekable();
    let Some(entry) = entries.next() else {
        return jolt_fmt_ir::nil();
    };

    let first = format_type(&entry.exception, formatter);
    let rest = concat([
        format_throws_entry_separator(entry.comma, entries.peek().is_some()),
        concat(std::iter::from_fn(move || {
            let entry = entries.next()?;
            let has_next = entries.peek().is_some();
            Some(concat([
                format_type(&entry.exception, formatter),
                format_throws_entry_separator(entry.comma, has_next),
            ]))
        })),
    ]);

    concat([first, jolt_fmt_ir::indent(rest)])
}

fn format_throws_entry_separator(comma: Option<JavaSyntaxToken<'_>>, has_next: bool) -> Doc<'_> {
    if let Some(comma) = comma {
        format_separator_with_comments(&comma, line())
    } else if has_next {
        line()
    } else {
        jolt_fmt_ir::nil()
    }
}
