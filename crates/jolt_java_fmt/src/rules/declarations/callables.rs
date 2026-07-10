use super::{
    AnnotationElementDeclaration, CommaListItem, Doc, FormalParameterList, JavaSyntaxToken,
    LeadingTrivia, MethodDeclaration, ThrowsClause, ThrowsClauseEntry, TrailingTrivia,
    comment_forces_line, format_annotation_element_value, format_array_dimensions, format_block,
    format_construct_leading_comments, format_constructor_body, format_formal_parameter,
    format_inline_annotations, format_modifier_prefix, format_receiver_parameter,
    format_separator_with_comments, format_statement_semicolon, format_token,
    format_token_after_construct_leading_comments, format_token_sequence,
    format_token_with_comments, format_type, format_type_parameter_list,
    format_type_without_leading_comments, format_typed_modifier_prefix, parenthesized_list,
    recovered_comma_list_items, source_braced_body,
};
use jolt_fmt_ir::DocBuilder;
use jolt_java_syntax::RecoveredSeparatedListEntry;

pub(super) fn format_constructor_declaration<'source>(
    constructor: &jolt_java_syntax::ConstructorDeclaration<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let constructor_first_token = constructor.first_token();
    let prefix = doc_concat!(
        doc,
        [
            format_construct_leading_comments(doc, constructor_first_token.as_ref()),
            format_modifier_prefix(constructor.modifiers(), doc),
        ]
    );
    let throws = constructor.throws_clause();
    let type_parameters = constructor.type_parameters();
    let has_type_parameters = type_parameters.is_some();
    let header = doc_concat!(
        doc,
        [
            format_type_parameter_list(type_parameters, doc),
            if has_type_parameters {
                doc.space()
            } else {
                Doc::nil()
            },
            constructor.name().map_or_else(Doc::nil, |name| {
                format_token_after_construct_leading_comments(
                    doc,
                    &name,
                    constructor_first_token.as_ref(),
                )
            },),
            format_parameters(
                constructor.open_paren(),
                constructor.close_paren(),
                constructor.parameters(),
                doc,
            ),
            format_throws_clause(throws, doc),
        ]
    );

    if let Some(body) = constructor.body() {
        let open = body.open_brace();
        let close = body.close_brace();
        callable_declaration_with_body(
            prefix,
            header,
            open.as_ref(),
            close.as_ref(),
            format_constructor_body(&body, doc),
            doc,
        )
    } else {
        doc_concat!(doc, [prefix, doc_group!(doc, header)])
    }
}

pub(super) fn format_compact_constructor_declaration<'source>(
    constructor: &jolt_java_syntax::CompactConstructorDeclaration<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let prefix = format_modifier_prefix(constructor.modifiers(), doc);
    let header = constructor
        .name()
        .map_or_else(Doc::nil, |name| format_token_with_comments(doc, &name));

    if let Some(body) = constructor.body() {
        let open = body.open_brace();
        let close = body.close_brace();
        callable_declaration_with_body(
            prefix,
            header,
            open.as_ref(),
            close.as_ref(),
            format_constructor_body(&body, doc),
            doc,
        )
    } else {
        doc_concat!(doc, [prefix, doc_group!(doc, header)])
    }
}

pub(crate) fn format_method_declaration<'source>(
    method: &MethodDeclaration<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let modifiers = format_typed_modifier_prefix(method.modifiers(), doc);
    let prefix = doc_concat!(
        doc,
        [
            format_construct_leading_comments(doc, method.first_token().as_ref()),
            modifiers.declaration_prefix,
        ]
    );
    let throws = method.throws_clause();
    let type_parameters = method.type_parameters();
    let has_type_parameters = type_parameters.is_some();
    let parameters = method.parameters();
    let name_and_parameters = doc_concat!(
        doc,
        [
            method
                .name()
                .map_or_else(Doc::nil, |name| format_token_with_comments(doc, &name)),
            format_parameters(method.open_paren(), method.close_paren(), parameters, doc,),
        ]
    );
    let header = doc_concat!(
        doc,
        [
            format_type_parameter_list(type_parameters, doc),
            if has_type_parameters {
                doc.space()
            } else {
                Doc::nil()
            },
            modifiers.type_use_prefix,
            format_inline_annotations(method.return_type_annotations(), doc),
            method.return_type().map_or_else(Doc::nil, |return_type| {
                format_type_without_leading_comments(&return_type, doc)
            },),
            doc.space(),
            name_and_parameters,
            format_throws_clause(throws, doc),
        ]
    );

    if let Some(body) = method.body() {
        callable_declaration_with_body_doc(prefix, header, format_block(&body, doc), doc)
    } else {
        doc_concat!(
            doc,
            [
                prefix,
                doc_group!(doc, header),
                format_statement_semicolon(method.semicolon(), doc),
            ]
        )
    }
}

pub(super) fn format_annotation_element_declaration<'source>(
    element: &AnnotationElementDeclaration<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    doc_concat!(
        doc,
        [
            doc_group!(
                doc,
                doc_concat!(
                    doc,
                    [
                        format_modifier_prefix(element.modifiers(), doc),
                        element
                            .ty()
                            .map_or_else(Doc::nil, |ty| format_type(&ty, doc)),
                        doc.space(),
                        element
                            .name()
                            .map_or_else(Doc::nil, |name| format_token_with_comments(doc, &name)),
                        format_empty_parameters(doc, element.open_paren(), element.close_paren()),
                        element.dimensions().map_or_else(Doc::nil, |dimensions| {
                            format_array_dimensions(&dimensions, doc)
                        },),
                        format_annotation_element_default(element.default_value(), doc),
                    ]
                ),
            ),
            format_statement_semicolon(element.semicolon(), doc),
        ]
    )
}

fn format_annotation_element_default<'source>(
    default: Option<jolt_java_syntax::DefaultValue<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    default.map_or_else(Doc::nil, |default| {
        doc_concat!(
            doc,
            [
                doc.space(),
                default
                    .default_token()
                    .map_or_else(Doc::nil, |token| doc_concat!(
                        doc,
                        [format_token_with_comments(doc, &token), doc.space()]
                    ),),
                default
                    .value()
                    .map_or_else(Doc::nil, |value| format_annotation_element_value(
                        &value, doc
                    ),),
            ]
        )
    })
}

fn format_parameters<'source>(
    open: Option<JavaSyntaxToken<'source>>,
    close: Option<JavaSyntaxToken<'source>>,
    parameters: Option<FormalParameterList<'source>>,
    doc: &mut DocBuilder<'source>,
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
        return format_empty_parameters(doc, open, close);
    };

    let items = parameter_list_items(&parameters, doc);
    parenthesized_list(doc, open.as_ref(), close.as_ref(), items)
}

fn parameter_list_items<'source, 'fmt>(
    parameters: &'fmt FormalParameterList<'source>,
    doc: &'fmt mut DocBuilder<'source>,
) -> Vec<CommaListItem<'source>> {
    recovered_comma_list_items(doc, parameters.entries_with_recovered(), |entry, doc| {
        CommaListItem {
            doc: match entry.item {
                jolt_java_syntax::FormalParameterListItem::ReceiverParameter(parameter) => {
                    format_receiver_parameter(&parameter, doc)
                }
                jolt_java_syntax::FormalParameterListItem::FormalParameter(parameter) => {
                    format_formal_parameter(&parameter, doc)
                }
            },
            comma: entry.comma,
        }
    })
}

fn format_empty_parameters<'source>(
    doc: &mut jolt_fmt_ir::DocBuilder<'source>,
    open: Option<JavaSyntaxToken<'source>>,
    close: Option<JavaSyntaxToken<'source>>,
) -> Doc<'source> {
    parenthesized_list(
        doc,
        open.as_ref(),
        close.as_ref(),
        std::iter::empty::<CommaListItem<'source>>(),
    )
}

fn callable_declaration_with_body<'source>(
    prefix: Doc<'source>,
    header: Doc<'source>,
    open: Option<&JavaSyntaxToken<'source>>,
    close: Option<&JavaSyntaxToken<'source>>,
    body: Option<Doc<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    doc_concat!(
        doc,
        [
            prefix,
            doc_group!(doc, header),
            doc.space(),
            source_braced_body(doc, open, close, body),
        ]
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
    throws: Option<ThrowsClause<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let Some(throws) = throws else {
        return Doc::nil();
    };
    let mut entries = throws.entries_with_recovered().peekable();
    if entries.peek().is_none() {
        return doc_indent!(
            doc,
            doc_concat!(doc, [doc.line(), format_throws_keyword(doc, &throws)])
        );
    }

    doc_indent!(
        doc,
        doc_concat!(
            doc,
            [
                doc.line(),
                format_throws_keyword(doc, &throws),
                format_throws_keyword_spacing(doc, &throws),
                format_throws_entries(entries, doc),
            ]
        )
    )
}

fn format_throws_keyword<'source>(
    doc: &mut jolt_fmt_ir::DocBuilder<'source>,
    throws: &ThrowsClause<'source>,
) -> Doc<'source> {
    throws.keyword().map_or_else(Doc::nil, |keyword| {
        format_token(
            doc,
            &keyword,
            LeadingTrivia::Preserve,
            TrailingTrivia::BeforeLineBreak,
        )
    })
}

fn format_throws_keyword_spacing<'source>(
    doc: &mut jolt_fmt_ir::DocBuilder<'source>,
    throws: &ThrowsClause<'source>,
) -> Doc<'source> {
    if throws.keyword().is_some_and(|keyword| {
        keyword
            .trailing_comments()
            .any(|comment| comment_forces_line(&comment))
    }) {
        doc.hard_line()
    } else {
        doc.space()
    }
}

fn format_throws_entries<'source>(
    entries: impl IntoIterator<Item = RecoveredSeparatedListEntry<'source, ThrowsClauseEntry<'source>>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let mut entries = entries.into_iter().peekable();
    let Some(entry) = entries.next() else {
        return Doc::nil();
    };

    let first = format_throws_entry(entry, entries.peek().is_some(), doc);
    let contents = doc.concat_list(|docs| {
        docs.push(first);
        while let Some(entry) = entries.next() {
            let has_next = entries.peek().is_some();
            let entry_doc = format_throws_entry(entry, has_next, docs);
            docs.push(entry_doc);
        }
    });

    doc_indent!(doc, contents)
}

fn format_throws_entry<'source>(
    entry: RecoveredSeparatedListEntry<'source, ThrowsClauseEntry<'source>>,
    has_next: bool,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    match entry {
        RecoveredSeparatedListEntry::Entry(entry) => doc_concat!(
            doc,
            [
                format_type(&entry.exception, doc),
                format_throws_entry_separator(doc, entry.comma, has_next),
            ]
        ),
        RecoveredSeparatedListEntry::Token(token) => doc_concat!(
            doc,
            [
                format_token(
                    doc,
                    &token,
                    LeadingTrivia::Preserve,
                    TrailingTrivia::Preserve,
                ),
                format_throws_entry_separator(doc, None, has_next),
            ]
        ),
        RecoveredSeparatedListEntry::Error(error) => doc_concat!(
            doc,
            [
                format_token_sequence(doc, error.token_iter(), LeadingTrivia::Preserve),
                format_throws_entry_separator(doc, None, has_next),
            ]
        ),
        RecoveredSeparatedListEntry::Node(node) => doc_concat!(
            doc,
            [
                format_token_sequence(doc, node.token_iter(), LeadingTrivia::Preserve),
                format_throws_entry_separator(doc, None, has_next),
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
