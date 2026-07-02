use super::control_flow::{
    format_parenthesized_statement_expression, format_statement_header_body_separator,
};
use super::simple::format_statement_keyword;
use super::{
    BlockItem, BlockStatement, Doc, JavaFormatter, JavaSyntaxToken, SwitchBlock, SwitchBlockEntry,
    SwitchBlockStatementGroup, SwitchLabel, SwitchLabelCaseEntry, SwitchLabelCaseItem, SwitchRule,
    SwitchStatement, braced_block, comment_forces_line, concat, empty_block, format_block,
    format_block_statement_item, format_comment, format_expression, format_leading_comments,
    format_pattern, format_throw_statement, format_trailing_comments_before_line_break, group,
    hard_line, indent, join_body_items, join_hard_lines, line, text,
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

    braced_block(entries)
}

fn format_switch_statement_group(
    group: &SwitchBlockStatementGroup,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    let labels = group
        .labels()
        .map(|label| concat([format_switch_label(&label, formatter), text(":")]))
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

    concat([
        label,
        format_switch_rule_arrow(rule),
        format_switch_rule_body(rule, formatter),
    ])
}

fn format_switch_rule_arrow(rule: &SwitchRule) -> Doc {
    let Some(arrow) = rule.arrow() else {
        return text(" -> ");
    };

    let trailing_comments = arrow.trailing_comments();
    if trailing_comments.is_empty() {
        return text(" -> ");
    }

    let mut docs = vec![text(" ->")];
    let mut forced_line = false;
    for comment in trailing_comments {
        docs.push(text(" "));
        forced_line |= comment_forces_line(&comment);
        docs.push(format_comment(&comment));
    }
    docs.push(if forced_line { hard_line() } else { text(" ") });
    concat(docs)
}

fn format_switch_label(label: &SwitchLabel, formatter: &JavaFormatter<'_>) -> Doc {
    if label.is_default_label() {
        return text("default");
    }

    let entries = label.case_entries().collect::<Vec<_>>();

    concat([
        text("case "),
        group(indent(format_switch_label_case_entries(entries, formatter))),
        label.guard().map_or_else(jolt_fmt_ir::nil, |guard| {
            concat([
                text(" when "),
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
            docs.push(format_switch_label_case_separator(&comma));
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
        SwitchLabelCaseItem::Default(default) => concat([
            format_leading_comments(default),
            text("default"),
            format_trailing_comments_before_line_break(default),
        ]),
    }
}

fn format_switch_label_case_separator(comma: &JavaSyntaxToken) -> Doc {
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

fn format_switch_rule_body(rule: &SwitchRule, formatter: &JavaFormatter<'_>) -> Doc {
    if let Some(block) = rule.block() {
        return format_block(&block, formatter);
    }
    if let Some(statement) = rule.throw_statement() {
        return format_throw_statement(&statement, formatter);
    }
    if let Some(expression) = rule.expression() {
        return concat([format_expression(&expression, formatter), text(";")]);
    }

    jolt_fmt_ir::nil()
}
