use super::control_flow::{format_condition_open_paren, format_statement_header_body_separator};
use super::simple::format_statement_keyword;
use super::{
    CatchClause, CatchParameter, CatchTypeList, Doc, FinallyClause, JavaFormatter, JavaSyntaxToken,
    Resource, ResourceListEntry, TryStatement, TryWithResourcesStatement, Type, concat,
    empty_block, format_annotation, format_block, format_dangling_comments, format_expression,
    format_leading_comments, format_local_variable_declaration, format_statement_semicolon,
    format_token_with_comments, format_trailing_comments_before_line_break, format_type, group,
    hard_line, indent, line, non_formatter_control_comments, soft_line, text,
    trailing_comments_force_line,
};

pub(super) fn format_try_statement(statement: &TryStatement, formatter: &JavaFormatter<'_>) -> Doc {
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

pub(super) fn format_try_with_resources_statement(
    statement: &TryWithResourcesStatement,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    let close_paren = statement
        .resources()
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

fn format_resource_specification(
    statement: &TryWithResourcesStatement,
    formatter: &JavaFormatter<'_>,
) -> Doc {
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
            join_resource_lines(resources, &trailing_comments),
        ])),
        format_resource_close_paren(close_paren.as_ref()),
    ])
}

fn format_resource_open_spacing(open: Option<&JavaSyntaxToken>) -> Doc {
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

fn format_resource_close_paren(close: Option<&JavaSyntaxToken>) -> Doc {
    let Some(close) = close else {
        return concat([hard_line(), text(")")]);
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
        text(")"),
        format_trailing_comments_before_line_break(close),
        if trailing_comments_force_line(close) {
            hard_line()
        } else {
            jolt_fmt_ir::nil()
        },
    ])
}

struct FormattedResource {
    resource: Doc,
    separator: Option<JavaSyntaxToken>,
}

fn format_resource_entry(
    entry: &ResourceListEntry,
    formatter: &JavaFormatter<'_>,
) -> FormattedResource {
    FormattedResource {
        resource: format_resource(&entry.resource, formatter),
        separator: entry.separator.clone(),
    }
}

fn format_resource(resource: &Resource, formatter: &JavaFormatter<'_>) -> Doc {
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

fn format_catch_clauses<'a>(
    clauses: impl Iterator<Item = CatchClause> + 'a,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    concat(clauses.map(|clause| format_catch_clause(&clause, formatter)))
}

fn format_catch_clause(clause: &CatchClause, formatter: &JavaFormatter<'_>) -> Doc {
    concat([
        text(" "),
        format_statement_keyword(clause.keyword(), "catch"),
        text(" "),
        clause
            .parameter()
            .map_or_else(jolt_fmt_ir::nil, |parameter| {
                format_parenthesized_catch_parameter(&parameter, formatter)
            }),
        text(" "),
        clause
            .body()
            .map_or_else(empty_block, |body| format_block(&body, formatter)),
    ])
}

fn format_parenthesized_catch_parameter(
    parameter: &CatchParameter,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    group(concat([
        text("("),
        indent(concat([
            soft_line(),
            format_catch_parameter(parameter, formatter),
        ])),
        soft_line(),
        text(")"),
    ]))
}

fn format_catch_parameter(parameter: &CatchParameter, formatter: &JavaFormatter<'_>) -> Doc {
    concat([
        format_catch_modifier_prefix(parameter, formatter),
        parameter.types().map_or_else(jolt_fmt_ir::nil, |types| {
            format_catch_type_list(&types, parameter.name(), formatter)
        }),
    ])
}

fn format_catch_modifier_prefix(parameter: &CatchParameter, formatter: &JavaFormatter<'_>) -> Doc {
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
        concat([jolt_fmt_ir::join(text(" "), docs), text(" ")])
    }
}

fn format_catch_type_list(
    types: &CatchTypeList,
    name: Option<jolt_java_syntax::JavaSyntaxToken>,
    formatter: &JavaFormatter<'_>,
) -> Doc {
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

fn format_catch_type_separator(separator: Option<&JavaSyntaxToken>) -> Doc {
    concat([
        line(),
        separator.map_or_else(
            || text("| "),
            |separator| {
                concat([
                    format_leading_comments(separator),
                    text("|"),
                    format_trailing_comments_before_line_break(separator),
                    if trailing_comments_force_line(separator) {
                        hard_line()
                    } else {
                        text(" ")
                    },
                ])
            },
        ),
    ])
}

fn format_catch_type(ty: &Type, formatter: &JavaFormatter<'_>) -> Doc {
    format_type(ty, formatter)
}

fn format_finally_clause(clause: &FinallyClause, formatter: &JavaFormatter<'_>) -> Doc {
    concat([
        text(" "),
        format_statement_keyword(clause.keyword(), "finally"),
        text(" "),
        clause
            .body()
            .map_or_else(empty_block, |body| format_block(&body, formatter)),
    ])
}

fn join_resource_lines(resources: Vec<FormattedResource>, trailing_comments: &[Doc]) -> Doc {
    let mut joined = Vec::new();
    let resource_count = resources.len();
    for (index, resource) in resources.into_iter().enumerate() {
        let is_last = index + 1 == resource_count;

        joined.push(resource.resource);
        if is_last {
            for comments in trailing_comments {
                joined.push(hard_line());
                joined.push(comments.clone());
            }
        } else {
            joined.push(format_statement_semicolon(resource.separator));
            joined.push(hard_line());
        }
    }
    concat(joined)
}

fn format_removed_resource_separator_comments(separator: &JavaSyntaxToken) -> Option<Doc> {
    let comments = non_formatter_control_comments(
        separator
            .leading_comments()
            .into_iter()
            .chain(separator.trailing_comments())
            .collect(),
    );
    (!comments.is_empty()).then(|| format_dangling_comments(comments))
}
