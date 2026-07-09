use super::control_flow::{format_condition_open_paren, format_statement_header_body_separator};
use super::simple::format_statement_keyword;
use super::{
    CatchClause, CatchParameter, CatchTypeList, Doc, FinallyClause, JavaFormatter, JavaSyntaxToken,
    LeadingTrivia, Resource, ResourceList, ResourceListEntry, TrailingTrivia, TryStatement,
    TryWithResourcesStatement, Type, concat, empty_block, format_annotation, format_block,
    format_dangling_comments, format_expression, format_local_variable_declaration,
    format_removed_comments, format_separator_with_comments, format_statement_semicolon,
    format_token, format_token_sequence, format_token_with_comments,
    format_trailing_comments_before_line_break, format_type, group, hard_line, indent, line,
    soft_line, trailing_comments_force_line,
};
use crate::helpers::modifiers::inline_modifier_prefix_from_docs;
use jolt_fmt_ir::space;

pub(super) fn format_try_statement<'source>(
    statement: &TryStatement<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    if let Some(resources_statement) = statement.resources_statement() {
        return format_try_with_resources_statement(&resources_statement, formatter);
    }

    concat([
        format_statement_keyword(statement.keyword(), "try"),
        space(),
        statement
            .body()
            .map_or_else(empty_block, |body| format_block(&body, formatter)),
        format_catch_clauses(statement.catch_clauses(), formatter),
        statement
            .finally_clause()
            .map_or_else(jolt_fmt_ir::nil, |finally_clause| {
                format_finally_clause(&finally_clause, formatter)
            }),
    ])
}

pub(super) fn format_try_with_resources_statement<'source>(
    statement: &TryWithResourcesStatement<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    let resources = statement.resources();
    let close_paren = resources
        .as_ref()
        .and_then(jolt_java_syntax::ResourceSpecification::close_paren);

    concat([
        format_statement_keyword(statement.keyword(), "try"),
        space(),
        format_resource_specification(statement, formatter),
        format_statement_header_body_separator(close_paren.as_ref()),
        statement
            .body()
            .map_or_else(empty_block, |body| format_block(&body, formatter)),
        format_catch_clauses(statement.catch_clauses(), formatter),
        statement
            .finally_clause()
            .map_or_else(jolt_fmt_ir::nil, |finally_clause| {
                format_finally_clause(&finally_clause, formatter)
            }),
    ])
}

fn format_resource_specification<'source>(
    statement: &TryWithResourcesStatement<'source>,
    formatter: &JavaFormatter<'_>,
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
        .and_then(format_removed_resource_separator_comments);
    let close_paren = specification
        .as_ref()
        .and_then(jolt_java_syntax::ResourceSpecification::close_paren);
    let resource_list = specification
        .as_ref()
        .and_then(jolt_java_syntax::ResourceSpecification::list);
    let Some(resource_list) = resource_list.as_ref() else {
        return concat([
            format_condition_open_paren(open_paren.as_ref()),
            format_resource_close_paren(close_paren.as_ref()),
        ]);
    };
    let mut resources = resource_list_items(resource_list, formatter).peekable();

    if resources.peek().is_none() {
        return concat([
            format_condition_open_paren(open_paren.as_ref()),
            format_resource_close_paren(close_paren.as_ref()),
        ]);
    }

    concat([
        format_condition_open_paren(open_paren.as_ref()),
        jolt_fmt_ir::indent(concat([
            format_resource_open_spacing(open_paren.as_ref()),
            join_resource_lines(resources, removed_trailing_separator_comments),
        ])),
        format_resource_close_paren(close_paren.as_ref()),
    ])
}

fn format_resource_open_spacing<'source>(open: Option<&JavaSyntaxToken<'source>>) -> Doc<'source> {
    open.map_or_else(hard_line, |open| {
        if open.trailing_comments().is_empty() {
            hard_line()
        } else {
            concat([
                format_trailing_comments_before_line_break(open),
                hard_line(),
            ])
        }
    })
}

fn format_resource_close_paren<'source>(close: Option<&JavaSyntaxToken<'source>>) -> Doc<'source> {
    let Some(close) = close else {
        return hard_line();
    };

    let leading_comments = close.leading_comments();
    concat([
        if leading_comments.is_empty() {
            hard_line()
        } else {
            concat([
                jolt_fmt_ir::indent(concat([
                    hard_line(),
                    format_dangling_comments(leading_comments),
                ])),
                hard_line(),
            ])
        },
        format_token(
            close,
            LeadingTrivia::SuppressAlreadyHandled,
            TrailingTrivia::BeforeLineBreak,
        ),
        if trailing_comments_force_line(close) {
            hard_line()
        } else {
            jolt_fmt_ir::nil()
        },
    ])
}

enum ResourceLineItem<'source> {
    Resource {
        resource: Doc<'source>,
        separator: Option<JavaSyntaxToken<'source>>,
    },
    Recovered(Doc<'source>),
}

fn format_resource_entry<'source>(
    entry: &ResourceListEntry<'source>,
    formatter: &JavaFormatter<'_>,
) -> ResourceLineItem<'source> {
    ResourceLineItem::Resource {
        resource: format_resource(&entry.resource, formatter),
        separator: entry.separator,
    }
}

fn resource_list_items<'source, 'fmt>(
    list: &'fmt ResourceList<'source>,
    formatter: &'fmt JavaFormatter<'_>,
) -> impl Iterator<Item = ResourceLineItem<'source>> + use<'source, 'fmt> {
    list.entries_with_recovered().map(move |entry| match entry {
        jolt_java_syntax::RecoveredSeparatedListEntry::Entry(entry) => {
            format_resource_entry(&entry, formatter)
        }
        jolt_java_syntax::RecoveredSeparatedListEntry::Token(token) => ResourceLineItem::Recovered(
            format_token(&token, LeadingTrivia::Preserve, TrailingTrivia::Preserve),
        ),
        jolt_java_syntax::RecoveredSeparatedListEntry::Error(error) => ResourceLineItem::Recovered(
            format_token_sequence(error.token_iter(), LeadingTrivia::Preserve),
        ),
        jolt_java_syntax::RecoveredSeparatedListEntry::Node(node) => ResourceLineItem::Recovered(
            format_token_sequence(node.token_iter(), LeadingTrivia::Preserve),
        ),
    })
}

fn format_resource<'source>(
    resource: &Resource<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    if let Some(declaration) = resource.declaration() {
        return format_local_variable_declaration(&declaration, formatter);
    }
    if let Some(access) = resource.variable_access() {
        return access.expression().map_or_else(
            || format_token_sequence(access.token_iter(), LeadingTrivia::Preserve),
            |expression| format_expression(&expression, formatter),
        );
    }

    format_token_sequence(resource.token_iter(), LeadingTrivia::Preserve)
}

fn format_catch_clauses<'source>(
    clauses: impl Iterator<Item = CatchClause<'source>> + 'source,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    concat(clauses.map(|clause| format_catch_clause(&clause, formatter)))
}

fn format_catch_clause<'source>(
    clause: &CatchClause<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    let open = clause.open_paren();
    let close = clause.close_paren();
    concat([
        space(),
        format_statement_keyword(clause.keyword(), "catch"),
        space(),
        clause
            .parameter()
            .map_or_else(jolt_fmt_ir::nil, |parameter| {
                format_parenthesized_catch_parameter(
                    open.as_ref(),
                    &parameter,
                    close.as_ref(),
                    formatter,
                )
            }),
        space(),
        clause
            .body()
            .map_or_else(empty_block, |body| format_block(&body, formatter)),
    ])
}

fn format_parenthesized_catch_parameter<'source>(
    open: Option<&JavaSyntaxToken<'source>>,
    parameter: &CatchParameter<'source>,
    close: Option<&JavaSyntaxToken<'source>>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    group(concat([
        open.map_or_else(jolt_fmt_ir::nil, format_token_with_comments),
        indent(concat([
            soft_line(),
            format_catch_parameter(parameter, formatter),
        ])),
        soft_line(),
        close.map_or_else(jolt_fmt_ir::nil, format_token_with_comments),
    ]))
}

fn format_catch_parameter<'source>(
    parameter: &CatchParameter<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    concat([
        format_catch_modifier_prefix(parameter, formatter),
        parameter.types().map_or_else(jolt_fmt_ir::nil, |types| {
            format_catch_type_list(&types, parameter.name(), formatter)
        }),
    ])
}

fn format_catch_modifier_prefix<'source>(
    parameter: &CatchParameter<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    inline_modifier_prefix_from_docs(
        parameter
            .annotations()
            .map(|annotation| format_annotation(&annotation, formatter))
            .collect(),
        parameter.modifier_entries().collect(),
    )
}

fn format_catch_type_list<'source>(
    types: &CatchTypeList<'source>,
    name: Option<jolt_java_syntax::JavaSyntaxToken<'source>>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    let name = name.map_or_else(jolt_fmt_ir::nil, |name| format_token_with_comments(&name));
    let mut entries = catch_type_parts(*types, formatter);

    let Some(mut current) = entries.next() else {
        return name;
    };

    let (lower, _) = entries.size_hint();
    let mut docs = Vec::with_capacity(lower.saturating_mul(2).saturating_add(1));
    for next in entries {
        docs.push(current.doc);
        if let Some(separator) = current.separator {
            docs.push(format_catch_type_separator(Some(&separator)));
        }
        current = next;
    }

    let last = concat([current.doc, space(), name]);
    if docs.is_empty() {
        return last;
    }

    docs.push(last);
    group(concat(docs))
}

struct CatchTypePart<'source> {
    doc: Doc<'source>,
    separator: Option<JavaSyntaxToken<'source>>,
}

fn catch_type_parts<'source, 'fmt>(
    types: CatchTypeList<'source>,
    formatter: &'fmt JavaFormatter<'_>,
) -> impl Iterator<Item = CatchTypePart<'source>> + use<'source, 'fmt> {
    types
        .entries_with_recovered()
        .map(move |entry| match entry {
            jolt_java_syntax::RecoveredSeparatedListEntry::Entry(entry) => CatchTypePart {
                doc: format_catch_type(&entry.ty, formatter),
                separator: entry.separator,
            },
            jolt_java_syntax::RecoveredSeparatedListEntry::Token(token) => CatchTypePart {
                doc: format_token(&token, LeadingTrivia::Preserve, TrailingTrivia::Preserve),
                separator: None,
            },
            jolt_java_syntax::RecoveredSeparatedListEntry::Error(error) => CatchTypePart {
                doc: format_token_sequence(error.token_iter(), LeadingTrivia::Preserve),
                separator: None,
            },
            jolt_java_syntax::RecoveredSeparatedListEntry::Node(node) => CatchTypePart {
                doc: format_token_sequence(node.token_iter(), LeadingTrivia::Preserve),
                separator: None,
            },
        })
}

fn format_catch_type_separator<'source>(
    separator: Option<&JavaSyntaxToken<'source>>,
) -> Doc<'source> {
    concat([
        line(),
        separator.map_or_else(jolt_fmt_ir::nil, |separator| {
            format_separator_with_comments(separator, space())
        }),
    ])
}

fn format_catch_type<'source>(ty: &Type<'source>, formatter: &JavaFormatter<'_>) -> Doc<'source> {
    format_type(ty, formatter)
}

fn format_finally_clause<'source>(
    clause: &FinallyClause<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    concat([
        space(),
        format_statement_keyword(clause.keyword(), "finally"),
        space(),
        clause
            .body()
            .map_or_else(empty_block, |body| format_block(&body, formatter)),
    ])
}

fn join_resource_lines<'source>(
    mut resources: std::iter::Peekable<impl Iterator<Item = ResourceLineItem<'source>>>,
    trailing_comments: Option<Doc<'source>>,
) -> Doc<'source> {
    let (lower, _) = resources.size_hint();
    let mut joined = Vec::with_capacity(lower.saturating_mul(3));
    let mut trailing_comments = trailing_comments;
    while let Some(resource) = resources.next() {
        match resource {
            ResourceLineItem::Resource {
                resource,
                separator,
            } => {
                joined.push(resource);
                if resources.peek().is_none() {
                    if let Some(comments) = trailing_comments.take() {
                        joined.push(hard_line());
                        joined.push(comments);
                    }
                } else {
                    joined.push(format_statement_semicolon(separator));
                    joined.push(hard_line());
                }
            }
            ResourceLineItem::Recovered(doc) => {
                joined.push(doc);
                if resources.peek().is_some() {
                    joined.push(hard_line());
                }
            }
        }
    }
    concat(joined)
}

fn format_removed_resource_separator_comments<'source>(
    separator: &JavaSyntaxToken<'source>,
) -> Option<Doc<'source>> {
    format_removed_comments(
        separator
            .leading_comments()
            .chain(separator.trailing_comments()),
    )
}
