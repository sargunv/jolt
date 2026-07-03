use super::control_flow::{
    format_parenthesized_statement_expression, format_statement_header_body_separator,
};
use super::simple::format_statement_keyword;
use super::{
    BlockItem, BlockStatement, Doc, JavaFormatter, LeadingTrivia, SwitchBlock, SwitchBlockEntry,
    SwitchBlockStatementGroup, SwitchLabel, SwitchLabelCaseEntry, SwitchLabelCaseItem, SwitchRule,
    SwitchStatement, TrailingTrivia, comment_forces_line, concat, empty_block, format_block,
    format_block_statement_item, format_expression, format_pattern, format_separator_with_comments,
    format_statement_semicolon, format_throw_statement, format_token, format_token_with_comments,
    group, hard_line, indent, join_body_items, join_hard_lines, line, text,
};

pub(super) fn format_switch_statement(
    statement: &SwitchStatement,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    let open = statement.open_paren();
    let close = statement.close_paren();
    concat([
        format_statement_keyword(statement.keyword(), "switch"),
        text(" "),
        format_parenthesized_statement_expression(
            open.as_ref(),
            statement
                .selector()
                .map_or_else(jolt_fmt_ir::nil, |selector| {
                    format_expression(&selector, formatter)
                }),
            close.as_ref(),
        ),
        format_statement_header_body_separator(close.as_ref()),
        statement
            .block()
            .map_or_else(empty_block, |block| format_switch_block(&block, formatter)),
    ])
}

pub(crate) fn format_switch_block(block: &SwitchBlock, formatter: &JavaFormatter<'_>) -> Doc {
    let entries = block
        .entries()
        .map(|entry| match entry {
            SwitchBlockEntry::StatementGroup(group) => {
                format_switch_statement_group(&group, formatter)
            }
            SwitchBlockEntry::Rule(rule) => format_switch_rule(&rule, formatter),
        })
        .collect::<Vec<_>>();

    braced_switch_block(block, entries)
}

fn braced_switch_block(block: &SwitchBlock, entries: Vec<Doc>) -> Doc {
    let body = (!entries.is_empty()).then(|| join_hard_lines(entries));
    concat([
        block
            .open_brace()
            .as_ref()
            .map_or_else(jolt_fmt_ir::nil, format_token_with_comments),
        body.map_or_else(hard_line, |body| {
            concat([
                jolt_fmt_ir::indent(concat([hard_line(), body])),
                hard_line(),
            ])
        }),
        block
            .close_brace()
            .as_ref()
            .map_or_else(jolt_fmt_ir::nil, format_token_with_comments),
    ])
}

fn format_switch_statement_group(
    group: &SwitchBlockStatementGroup,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    let labels = group
        .label_entries()
        .map(|entry| {
            concat([
                format_switch_label(&entry.label, formatter),
                entry
                    .colon
                    .as_ref()
                    .map_or_else(jolt_fmt_ir::nil, format_token_with_comments),
            ])
        })
        .collect::<Vec<_>>();
    let statements = group.block_statements().collect::<Vec<_>>();

    if let Some(doc) = format_single_block_switch_statement_group(&labels, &statements, formatter) {
        return doc;
    }

    let items = statements
        .iter()
        .filter_map(|statement| format_block_statement_item(statement, formatter))
        .collect::<Vec<_>>();

    concat([
        join_hard_lines(labels),
        if items.is_empty() {
            jolt_fmt_ir::nil()
        } else {
            jolt_fmt_ir::indent(concat([hard_line(), join_body_items(items)]))
        },
    ])
}

fn format_single_block_switch_statement_group(
    labels: &[Doc],
    statements: &[BlockStatement],
    formatter: &JavaFormatter<'_>,
) -> Option<Doc> {
    if labels.len() != 1 || statements.len() != 1 || statements[0].starts_after_blank_line() {
        return None;
    }

    let BlockItem::Block(block) = statements[0].item()? else {
        return None;
    };

    Some(concat([
        labels.first()?.clone(),
        text(" "),
        format_block(&block, formatter),
    ]))
}

fn format_switch_rule(rule: &SwitchRule, formatter: &JavaFormatter<'_>) -> Doc {
    let label = rule.label().map_or_else(jolt_fmt_ir::nil, |label| {
        format_switch_label(&label, formatter)
    });

    if let Some(expression) = rule.expression() {
        return concat([
            label,
            group(concat([
                format_switch_rule_arrow_head(rule),
                indent(concat([
                    format_switch_rule_arrow_body_separator(rule),
                    format_expression(&expression, formatter),
                    format_statement_semicolon(rule.semicolon()),
                ])),
            ])),
        ]);
    }

    concat([
        label,
        format_switch_rule_arrow(rule),
        format_switch_rule_body(rule, formatter),
    ])
}

fn format_switch_rule_arrow_head(rule: &SwitchRule) -> Doc {
    let Some(arrow) = rule.arrow() else {
        return text(" ->");
    };

    let trailing_comments = arrow.trailing_comments();
    if trailing_comments.is_empty() {
        return concat([text(" "), format_token_with_comments(&arrow)]);
    }

    concat([
        text(" "),
        format_token(
            &arrow,
            LeadingTrivia::Preserve,
            TrailingTrivia::BeforeLineBreak,
        ),
    ])
}

fn format_switch_rule_arrow_body_separator(rule: &SwitchRule) -> Doc {
    if rule
        .arrow()
        .is_some_and(|arrow| arrow.trailing_comments().iter().any(comment_forces_line))
    {
        hard_line()
    } else {
        line()
    }
}

fn format_switch_rule_arrow(rule: &SwitchRule) -> Doc {
    let Some(arrow) = rule.arrow() else {
        return text(" -> ");
    };

    let trailing_comments = arrow.trailing_comments();
    if trailing_comments.is_empty() {
        return concat([text(" "), format_token_with_comments(&arrow), text(" ")]);
    }

    let forced_line = trailing_comments.iter().any(comment_forces_line);
    let mut docs = vec![
        text(" "),
        format_token(
            &arrow,
            LeadingTrivia::Preserve,
            TrailingTrivia::BeforeLineBreak,
        ),
    ];
    docs.push(if forced_line { hard_line() } else { text(" ") });
    concat(docs)
}

fn format_switch_label(label: &SwitchLabel, formatter: &JavaFormatter<'_>) -> Doc {
    if label.is_default_label() {
        return label
            .default_token()
            .as_ref()
            .map_or_else(jolt_fmt_ir::nil, format_token_with_comments);
    }

    let entries = label.case_entries().collect::<Vec<_>>();

    concat([
        label
            .case_token()
            .as_ref()
            .map_or_else(jolt_fmt_ir::nil, |token| {
                concat([format_token_with_comments(token), text(" ")])
            }),
        group(indent(format_switch_label_case_entries(entries, formatter))),
        label.guard().map_or_else(jolt_fmt_ir::nil, |guard| {
            concat([
                text(" "),
                guard
                    .when_token()
                    .as_ref()
                    .map_or_else(jolt_fmt_ir::nil, format_token_with_comments),
                text(" "),
                guard
                    .expression()
                    .map_or_else(jolt_fmt_ir::nil, |expression| {
                        format_expression(&expression, formatter)
                    }),
            ])
        }),
    ])
}

fn format_switch_label_case_entries(
    entries: Vec<SwitchLabelCaseEntry>,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    let mut docs = Vec::new();

    for entry in entries {
        docs.push(format_switch_label_case_item(&entry.item, formatter));
        if let Some(comma) = entry.comma {
            docs.push(format_separator_with_comments(&comma, line()));
        }
    }

    concat(docs)
}

fn format_switch_label_case_item(item: &SwitchLabelCaseItem, formatter: &JavaFormatter<'_>) -> Doc {
    match item {
        SwitchLabelCaseItem::Constant(constant) => constant
            .expression()
            .map_or_else(jolt_fmt_ir::nil, |expression| {
                format_expression(&expression, formatter)
            }),
        SwitchLabelCaseItem::Pattern(pattern) => {
            pattern.pattern().map_or_else(jolt_fmt_ir::nil, |pattern| {
                format_pattern(&pattern, formatter)
            })
        }
        SwitchLabelCaseItem::Default(default) => format_token(
            default,
            LeadingTrivia::Preserve,
            TrailingTrivia::BeforeLineBreak,
        ),
    }
}

fn format_switch_rule_body(rule: &SwitchRule, formatter: &JavaFormatter<'_>) -> Doc {
    if let Some(block) = rule.block() {
        return format_block(&block, formatter);
    }
    if let Some(statement) = rule.throw_statement() {
        return format_throw_statement(&statement, formatter);
    }
    if let Some(expression) = rule.expression() {
        return concat([
            format_expression(&expression, formatter),
            format_statement_semicolon(rule.semicolon()),
        ]);
    }

    jolt_fmt_ir::nil()
}
