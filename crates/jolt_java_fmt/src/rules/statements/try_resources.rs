use super::control_flow::{format_condition_open_paren, format_statement_header_body_separator};
use super::simple::format_statement_keyword;
use super::{
    CatchClause, CatchParameter, CatchTypeList, Doc, FinallyClause, JavaFormatter, JavaSyntaxToken,
    LeadingTrivia, Resource, ResourceListEntry, TrailingTrivia, TryStatement,
    TryWithResourcesStatement, Type, concat, empty_block, format_annotation, format_block,
    format_dangling_comments, format_expression, format_local_variable_declaration,
    format_removed_comments, format_separator_with_comments, format_statement_semicolon,
    format_token, format_token_with_comments, format_trailing_comments_before_line_break,
    format_type, group, hard_line, indent, line, soft_line, text, trailing_comments_force_line,
};

pub(super) fn format_try_statement<'source>(
    statement: &TryStatement<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    if let Some(resources_statement) = statement.resources_statement() {
        return format_try_with_resources_statement(&resources_statement, formatter);
    }

    concat([
        format_statement_keyword(statement.keyword(), "try"),
        text(" "),
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
        text(" "),
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
    let resources = specification
        .as_ref()
        .and_then(jolt_java_syntax::ResourceSpecification::list)
        .map(|list| {
            list.entries()
                .map(|entry| format_resource_entry(&entry, formatter))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    if resources.is_empty() {
        return concat([
            format_condition_open_paren(open_paren.as_ref()),
            format_resource_close_paren(close_paren.as_ref()),
        ]);
    }

    let trailing_comments = [removed_trailing_separator_comments]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

    concat([
        format_condition_open_paren(open_paren.as_ref()),
        jolt_fmt_ir::indent(concat([
            format_resource_open_spacing(open_paren.as_ref()),
            join_resource_lines(resources, trailing_comments),
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

struct FormattedResource<'source> {
    resource: Doc<'source>,
    separator: Option<JavaSyntaxToken<'source>>,
}

fn format_resource_entry<'source>(
    entry: &ResourceListEntry<'source>,
    formatter: &JavaFormatter<'_>,
) -> FormattedResource<'source> {
    FormattedResource {
        resource: format_resource(&entry.resource, formatter),
        separator: entry.separator,
    }
}

fn format_resource<'source>(
    resource: &Resource<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    if let Some(declaration) = resource.declaration() {
        return format_local_variable_declaration(&declaration, formatter);
    }
    if let Some(access) = resource.variable_access() {
        return access
            .expression()
            .map_or_else(jolt_fmt_ir::nil, |expression| {
                format_expression(&expression, formatter)
            });
    }

    jolt_fmt_ir::nil()
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
        text(" "),
        format_statement_keyword(clause.keyword(), "catch"),
        text(" "),
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
        text(" "),
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
    let mut docs = parameter
        .annotations()
        .map(|annotation| format_annotation(&annotation, formatter))
        .collect::<Vec<_>>();
    docs.extend(
        parameter
            .modifier_tokens()
            .map(|token| format_token_with_comments(&token)),
    );

    if docs.is_empty() {
        jolt_fmt_ir::nil()
    } else {
        concat([jolt_fmt_ir::join(&text(" "), docs), text(" ")])
    }
}

fn format_catch_type_list<'source>(
    types: &CatchTypeList<'source>,
    name: Option<jolt_java_syntax::JavaSyntaxToken<'source>>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    let mut entries = types.entries().collect::<Vec<_>>();
    let name = name.map_or_else(jolt_fmt_ir::nil, |name| format_token_with_comments(&name));

    let Some(last_entry) = entries.pop() else {
        return name;
    };

    let last = concat([
        format_catch_type(&last_entry.ty, formatter),
        text(" "),
        name,
    ]);
    if entries.is_empty() {
        return last;
    }

    let first = entries.remove(0);
    group(concat([
        format_catch_type(&first.ty, formatter),
        format_catch_type_separator(first.separator.as_ref()),
        concat(entries.into_iter().map(|entry| {
            concat([
                format_catch_type(&entry.ty, formatter),
                format_catch_type_separator(entry.separator.as_ref()),
            ])
        })),
        last,
    ]))
}

fn format_catch_type_separator<'source>(
    separator: Option<&JavaSyntaxToken<'source>>,
) -> Doc<'source> {
    concat([
        line(),
        separator.map_or_else(jolt_fmt_ir::nil, |separator| {
            format_separator_with_comments(separator, text(" "))
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
        text(" "),
        format_statement_keyword(clause.keyword(), "finally"),
        text(" "),
        clause
            .body()
            .map_or_else(empty_block, |body| format_block(&body, formatter)),
    ])
}

fn join_resource_lines<'source>(
    resources: Vec<FormattedResource<'source>>,
    trailing_comments: Vec<Doc<'source>>,
) -> Doc<'source> {
    let mut joined = Vec::new();
    let resource_count = resources.len();
    let mut trailing_comments = trailing_comments.into_iter();
    for (index, resource) in resources.into_iter().enumerate() {
        let is_last = index + 1 == resource_count;

        joined.push(resource.resource);
        if is_last {
            for comments in trailing_comments.by_ref() {
                joined.push(hard_line());
                joined.push(comments);
            }
        } else {
            joined.push(format_statement_semicolon(resource.separator));
            joined.push(hard_line());
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
