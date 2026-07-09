use super::control_flow::{format_condition_open_paren, format_statement_header_body_separator};
use super::simple::format_statement_keyword;
use super::{
    CatchClause, CatchParameter, CatchTypeList, Doc, FinallyClause, JavaSyntaxToken, LeadingTrivia,
    Resource, ResourceList, TrailingTrivia, TryStatement, TryWithResourcesStatement, Type,
    empty_block, format_annotation, format_block, format_dangling_comments, format_expression,
    format_local_variable_declaration, format_removed_comments, format_separator_with_comments,
    format_statement_semicolon, format_token, format_token_sequence, format_token_with_comments,
    format_trailing_comments_before_line_break, format_type, trailing_comments_force_line,
};
use crate::helpers::modifiers::inline_modifier_prefix_from_docs;
use jolt_fmt_ir::DocBuilder;

pub(super) fn format_try_statement<'source>(
    statement: &TryStatement<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    if let Some(resources_statement) = statement.resources_statement() {
        return format_try_with_resources_statement(&resources_statement, doc);
    }

    doc_concat!(
        doc,
        [
            format_statement_keyword(statement.keyword(), "try", doc),
            doc.space(),
            match statement.body() {
                Some(body) => format_block(&body, doc),
                None => empty_block(doc),
            },
            format_catch_clauses(statement.catch_clauses(), doc),
            statement
                .finally_clause()
                .map_or_else(Doc::nil, |finally_clause| format_finally_clause(
                    &finally_clause,
                    doc
                ),),
        ]
    )
}

pub(super) fn format_try_with_resources_statement<'source>(
    statement: &TryWithResourcesStatement<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let resources = statement.resources();
    let close_paren = resources
        .as_ref()
        .and_then(jolt_java_syntax::ResourceSpecification::close_paren);

    doc_concat!(
        doc,
        [
            format_statement_keyword(statement.keyword(), "try", doc),
            doc.space(),
            format_resource_specification(statement, doc),
            format_statement_header_body_separator(close_paren.as_ref(), doc),
            match statement.body() {
                Some(body) => format_block(&body, doc),
                None => empty_block(doc),
            },
            format_catch_clauses(statement.catch_clauses(), doc),
            statement
                .finally_clause()
                .map_or_else(Doc::nil, |finally_clause| format_finally_clause(
                    &finally_clause,
                    doc
                ),),
        ]
    )
}

fn format_resource_specification<'source>(
    statement: &TryWithResourcesStatement<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let specification = statement.resources();
    let open_paren = specification
        .as_ref()
        .and_then(jolt_java_syntax::ResourceSpecification::open_paren);
    let trailing_separator = specification
        .as_ref()
        .and_then(jolt_java_syntax::ResourceSpecification::trailing_semicolon);
    let removed_trailing_separator_comments = trailing_separator
        .as_ref()
        .and_then(|separator| format_removed_resource_separator_comments(separator, doc));
    let close_paren = specification
        .as_ref()
        .and_then(jolt_java_syntax::ResourceSpecification::close_paren);
    let resource_list = specification
        .as_ref()
        .and_then(jolt_java_syntax::ResourceSpecification::list);
    let Some(resource_list) = resource_list.as_ref() else {
        return doc_concat!(
            doc,
            [
                format_condition_open_paren(open_paren.as_ref(), doc),
                format_resource_close_paren(close_paren.as_ref(), doc),
            ]
        );
    };
    let Some(resources) =
        format_resource_lines(resource_list, removed_trailing_separator_comments, doc)
    else {
        return doc_concat!(
            doc,
            [
                format_condition_open_paren(open_paren.as_ref(), doc),
                format_resource_close_paren(close_paren.as_ref(), doc),
            ]
        );
    };

    doc_concat!(
        doc,
        [
            format_condition_open_paren(open_paren.as_ref(), doc),
            doc_indent!(
                doc,
                doc_concat!(
                    doc,
                    [
                        format_resource_open_spacing(open_paren.as_ref(), doc),
                        resources,
                    ]
                )
            ),
            format_resource_close_paren(close_paren.as_ref(), doc),
        ]
    )
}

fn format_resource_open_spacing<'source>(
    open: Option<&JavaSyntaxToken<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let Some(open) = open else {
        return doc.hard_line();
    };

    if open.trailing_comments().is_empty() {
        doc.hard_line()
    } else {
        let comments = format_trailing_comments_before_line_break(doc, open);
        doc_concat!(doc, [comments, doc.hard_line()])
    }
}

fn format_resource_close_paren<'source>(
    close: Option<&JavaSyntaxToken<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let Some(close) = close else {
        return doc.hard_line();
    };

    let leading_comments = close.leading_comments();
    doc_concat!(
        doc,
        [
            if leading_comments.is_empty() {
                doc.hard_line()
            } else {
                doc_concat!(
                    doc,
                    [
                        doc_indent!(
                            doc,
                            doc_concat!(
                                doc,
                                [
                                    doc.hard_line(),
                                    format_dangling_comments(doc, leading_comments),
                                ]
                            )
                        ),
                        doc.hard_line(),
                    ]
                )
            },
            format_token(
                doc,
                close,
                LeadingTrivia::SuppressAlreadyHandled,
                TrailingTrivia::BeforeLineBreak,
            ),
            if trailing_comments_force_line(close) {
                doc.hard_line()
            } else {
                Doc::nil()
            },
        ]
    )
}

fn format_resource_lines<'source>(
    list: &ResourceList<'source>,
    trailing_comments: Option<Doc<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Option<Doc<'source>> {
    let mut entries = list.entries_with_recovered().peekable();
    let mut joined = doc.list();
    let mut trailing_comments = trailing_comments;

    while let Some(entry) = entries.next() {
        let has_next = entries.peek().is_some();
        match entry {
            jolt_java_syntax::RecoveredSeparatedListEntry::Entry(entry) => {
                let resource = format_resource(&entry.resource, doc);
                joined.push(resource, doc);
                if has_next {
                    joined.push(format_statement_semicolon(entry.separator, doc), doc);
                    joined.push(doc.hard_line(), doc);
                } else if let Some(comments) = trailing_comments.take() {
                    joined.push(doc.hard_line(), doc);
                    joined.push(comments, doc);
                }
            }
            jolt_java_syntax::RecoveredSeparatedListEntry::Token(token) => {
                let token = format_token(
                    doc,
                    &token,
                    LeadingTrivia::Preserve,
                    TrailingTrivia::Preserve,
                );
                joined.push(token, doc);
                if has_next {
                    joined.push(doc.hard_line(), doc);
                }
            }
            jolt_java_syntax::RecoveredSeparatedListEntry::Error(error) => {
                let error = format_token_sequence(doc, error.token_iter(), LeadingTrivia::Preserve);
                joined.push(error, doc);
                if has_next {
                    joined.push(doc.hard_line(), doc);
                }
            }
            jolt_java_syntax::RecoveredSeparatedListEntry::Node(node) => {
                let node = format_token_sequence(doc, node.token_iter(), LeadingTrivia::Preserve);
                joined.push(node, doc);
                if has_next {
                    joined.push(doc.hard_line(), doc);
                }
            }
        }
    }

    (!joined.is_empty()).then(|| joined.finish(doc))
}

fn format_resource<'source>(
    resource: &Resource<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    if let Some(declaration) = resource.declaration() {
        return format_local_variable_declaration(&declaration, doc);
    }
    if let Some(access) = resource.variable_access() {
        return match access.expression() {
            Some(expression) => format_expression(&expression, doc),
            None => format_token_sequence(doc, access.token_iter(), LeadingTrivia::Preserve),
        };
    }

    format_token_sequence(doc, resource.token_iter(), LeadingTrivia::Preserve)
}

fn format_catch_clauses<'source>(
    clauses: impl Iterator<Item = CatchClause<'source>> + 'source,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let mut docs = doc.list();
    for clause in clauses {
        docs.push(format_catch_clause(&clause, doc), doc);
    }
    docs.finish(doc)
}

fn format_catch_clause<'source>(
    clause: &CatchClause<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let open = clause.open_paren();
    let close = clause.close_paren();
    doc_concat!(
        doc,
        [
            doc.space(),
            format_statement_keyword(clause.keyword(), "catch", doc),
            doc.space(),
            clause.parameter().map_or_else(Doc::nil, |parameter| {
                format_parenthesized_catch_parameter(open.as_ref(), &parameter, close.as_ref(), doc)
            },),
            doc.space(),
            match clause.body() {
                Some(body) => format_block(&body, doc),
                None => empty_block(doc),
            },
        ]
    )
}

fn format_parenthesized_catch_parameter<'source>(
    open: Option<&JavaSyntaxToken<'source>>,
    parameter: &CatchParameter<'source>,
    close: Option<&JavaSyntaxToken<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    doc_group!(
        doc,
        doc_concat!(
            doc,
            [
                open.map_or_else(Doc::nil, |open| format_token_with_comments(doc, open)),
                doc_indent!(
                    doc,
                    doc_concat!(
                        doc,
                        [doc.soft_line(), format_catch_parameter(parameter, doc),]
                    )
                ),
                doc.soft_line(),
                close.map_or_else(Doc::nil, |close| format_token_with_comments(doc, close)),
            ]
        )
    )
}

fn format_catch_parameter<'source>(
    parameter: &CatchParameter<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    doc_concat!(
        doc,
        [
            format_catch_modifier_prefix(parameter, doc),
            parameter
                .types()
                .map_or_else(Doc::nil, |types| format_catch_type_list(
                    &types,
                    parameter.name(),
                    doc
                ),),
        ]
    )
}

fn format_catch_modifier_prefix<'source>(
    parameter: &CatchParameter<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let modifiers = parameter.modifier_entries().collect::<Vec<_>>();
    let mut annotations = parameter.annotations().peekable();
    if annotations.peek().is_none() {
        return inline_modifier_prefix_from_docs(
            doc,
            std::iter::empty::<Doc<'source>>(),
            modifiers,
        );
    }

    let mut prefix = doc.list();
    for annotation in annotations {
        if !prefix.is_empty() {
            prefix.push(doc.space(), doc);
        }
        prefix.push(format_annotation(&annotation, doc), doc);
    }

    if modifiers.is_empty() {
        prefix.push(doc.space(), doc);
    } else {
        let modifiers =
            inline_modifier_prefix_from_docs(doc, std::iter::empty::<Doc<'source>>(), modifiers);
        prefix.push(doc.space(), doc);
        prefix.push(modifiers, doc);
    }

    prefix.finish(doc)
}

fn format_catch_type_list<'source>(
    types: &CatchTypeList<'source>,
    name: Option<jolt_java_syntax::JavaSyntaxToken<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let name = name.map_or_else(Doc::nil, |name| format_token_with_comments(doc, &name));
    let mut entries = (*types).entries_with_recovered();

    let Some(mut current) = entries
        .next()
        .map(|entry| format_catch_type_part(entry, doc))
    else {
        return name;
    };

    let mut docs = doc.list();
    for entry in entries {
        let next = format_catch_type_part(entry, doc);
        docs.push(current.doc, doc);
        if let Some(separator) = current.separator {
            let separator = format_catch_type_separator(Some(&separator), doc);
            docs.push(separator, doc);
        }
        current = next;
    }

    let last = doc_concat!(doc, [current.doc, doc.space(), name]);
    if docs.is_empty() {
        return last;
    }

    docs.push(last, doc);
    doc_group!(doc, docs.finish(doc))
}

struct CatchTypePart<'source> {
    doc: Doc<'source>,
    separator: Option<JavaSyntaxToken<'source>>,
}

fn format_catch_type_part<'source>(
    entry: jolt_java_syntax::RecoveredSeparatedListEntry<
        'source,
        jolt_java_syntax::UnionTypeEntry<'source>,
    >,
    doc: &mut DocBuilder<'source>,
) -> CatchTypePart<'source> {
    match entry {
        jolt_java_syntax::RecoveredSeparatedListEntry::Entry(entry) => CatchTypePart {
            doc: format_catch_type(&entry.ty, doc),
            separator: entry.separator,
        },
        jolt_java_syntax::RecoveredSeparatedListEntry::Token(token) => CatchTypePart {
            doc: format_token(
                doc,
                &token,
                LeadingTrivia::Preserve,
                TrailingTrivia::Preserve,
            ),
            separator: None,
        },
        jolt_java_syntax::RecoveredSeparatedListEntry::Error(error) => CatchTypePart {
            doc: format_token_sequence(doc, error.token_iter(), LeadingTrivia::Preserve),
            separator: None,
        },
        jolt_java_syntax::RecoveredSeparatedListEntry::Node(node) => CatchTypePart {
            doc: format_token_sequence(doc, node.token_iter(), LeadingTrivia::Preserve),
            separator: None,
        },
    }
}

fn format_catch_type_separator<'source>(
    separator: Option<&JavaSyntaxToken<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    doc_concat!(
        doc,
        [
            doc.line(),
            match separator {
                Some(separator) => {
                    let space = doc.space();
                    format_separator_with_comments(doc, separator, space)
                }
                None => Doc::nil(),
            },
        ]
    )
}

fn format_catch_type<'source>(ty: &Type<'source>, doc: &mut DocBuilder<'source>) -> Doc<'source> {
    format_type(ty, doc)
}

fn format_finally_clause<'source>(
    clause: &FinallyClause<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    doc_concat!(
        doc,
        [
            doc.space(),
            format_statement_keyword(clause.keyword(), "finally", doc),
            doc.space(),
            match clause.body() {
                Some(body) => format_block(&body, doc),
                None => empty_block(doc),
            },
        ]
    )
}

fn format_removed_resource_separator_comments<'source>(
    separator: &JavaSyntaxToken<'source>,
    doc: &mut DocBuilder<'source>,
) -> Option<Doc<'source>> {
    format_removed_comments(
        doc,
        separator
            .leading_comments()
            .chain(separator.trailing_comments()),
    )
}
