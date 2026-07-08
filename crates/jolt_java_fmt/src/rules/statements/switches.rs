use super::control_flow::{
    format_parenthesized_statement_expression, format_statement_header_body_separator,
};
use super::simple::format_statement_keyword;
use super::{
    BlockItem, BlockStatement, Doc, JavaFormatter, LeadingTrivia, SwitchBlock, SwitchBlockEntry,
    SwitchBlockStatementGroup, SwitchLabel, SwitchLabelCaseEntry, SwitchLabelCaseItem, SwitchRule,
    SwitchStatement, TrailingTrivia, comment_forces_line, concat, empty_block, format_block,
    format_block_statement_item_or_recovered, format_expression, format_pattern,
    format_separator_with_comments, format_statement_semicolon, format_throw_statement,
    format_token, format_token_sequence, format_token_with_comments, group, hard_line, indent,
    join_body_items, join_hard_lines, line,
};
use jolt_fmt_ir::space;

pub(super) fn format_switch_statement<'source>(
    statement: &SwitchStatement<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    let open = statement.open_paren();
    let close = statement.close_paren();
    concat([
        format_statement_keyword(statement.keyword(), "switch"),
        space(),
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

pub(crate) fn format_switch_block<'source>(
    block: &SwitchBlock<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    let entries = block
        .entries_with_recovered()
        .map(|entry| match entry {
            jolt_java_syntax::RecoveredSeparatedListEntry::Entry(entry) => match entry {
                SwitchBlockEntry::StatementGroup(group) => {
                    format_switch_statement_group(&group, formatter)
                }
                SwitchBlockEntry::Rule(rule) => format_switch_rule(&rule, formatter),
            },
            jolt_java_syntax::RecoveredSeparatedListEntry::Token(token) => {
                format_token_sequence(std::iter::once(token), LeadingTrivia::Preserve)
            }
            jolt_java_syntax::RecoveredSeparatedListEntry::Error(error) => {
                format_token_sequence(error.token_iter(), LeadingTrivia::Preserve)
            }
            jolt_java_syntax::RecoveredSeparatedListEntry::Node(node) => {
                format_token_sequence(node.token_iter(), LeadingTrivia::Preserve)
            }
        })
        .collect::<Vec<_>>();

    braced_switch_block(block, entries)
}

fn braced_switch_block<'source>(
    block: &SwitchBlock<'source>,
    entries: Vec<Doc<'source>>,
) -> Doc<'source> {
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

fn format_switch_statement_group<'source>(
    group: &SwitchBlockStatementGroup<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    let mut labels = group
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

    if let Some(doc) =
        format_single_block_switch_statement_group(&mut labels, &statements, formatter)
    {
        return doc;
    }

    let items = group
        .block_statements_with_recovered()
        .filter_map(|entry| match entry {
            jolt_java_syntax::RecoveredSeparatedListEntry::Entry(statement) => {
                format_block_statement_item_or_recovered(&statement, formatter)
            }
            jolt_java_syntax::RecoveredSeparatedListEntry::Token(token) => {
                Some(crate::helpers::blocks::BodyItem::new(
                    format_token_sequence(std::iter::once(token), LeadingTrivia::Preserve),
                    false,
                ))
            }
            jolt_java_syntax::RecoveredSeparatedListEntry::Error(error) => {
                Some(crate::helpers::blocks::BodyItem::new(
                    format_token_sequence(error.token_iter(), LeadingTrivia::Preserve),
                    false,
                ))
            }
            jolt_java_syntax::RecoveredSeparatedListEntry::Node(node) => {
                Some(crate::helpers::blocks::BodyItem::new(
                    format_token_sequence(node.token_iter(), LeadingTrivia::Preserve),
                    false,
                ))
            }
        })
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

fn format_single_block_switch_statement_group<'source>(
    labels: &mut Vec<Doc<'source>>,
    statements: &[BlockStatement<'source>],
    formatter: &JavaFormatter<'_>,
) -> Option<Doc<'source>> {
    if labels.len() != 1 || statements.len() != 1 || statements[0].starts_after_blank_line() {
        return None;
    }

    let BlockItem::Block(block) = statements[0].item()? else {
        return None;
    };

    Some(concat([
        labels.pop()?,
        space(),
        format_block(&block, formatter),
    ]))
}

fn format_switch_rule<'source>(
    rule: &SwitchRule<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
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

fn format_switch_rule_arrow_head<'source>(rule: &SwitchRule<'source>) -> Doc<'source> {
    let Some(arrow) = rule.arrow() else {
        return jolt_fmt_ir::nil();
    };

    let trailing_comments = arrow.trailing_comments();
    if trailing_comments.is_empty() {
        return concat([space(), format_token_with_comments(&arrow)]);
    }

    concat([
        space(),
        format_token(
            &arrow,
            LeadingTrivia::Preserve,
            TrailingTrivia::BeforeLineBreak,
        ),
    ])
}

fn format_switch_rule_arrow_body_separator<'source>(rule: &SwitchRule<'source>) -> Doc<'source> {
    if rule.arrow().is_some_and(|arrow| {
        arrow
            .trailing_comments()
            .any(|comment| comment_forces_line(&comment))
    }) {
        hard_line()
    } else {
        line()
    }
}

fn format_switch_rule_arrow<'source>(rule: &SwitchRule<'source>) -> Doc<'source> {
    let Some(arrow) = rule.arrow() else {
        return jolt_fmt_ir::nil();
    };

    let trailing_comments = arrow.trailing_comments();
    if trailing_comments.is_empty() {
        return concat([space(), format_token_with_comments(&arrow), space()]);
    }

    let forced_line = arrow
        .trailing_comments()
        .any(|comment| comment_forces_line(&comment));
    let mut docs = vec![
        space(),
        format_token(
            &arrow,
            LeadingTrivia::Preserve,
            TrailingTrivia::BeforeLineBreak,
        ),
    ];
    docs.push(if forced_line { hard_line() } else { space() });
    concat(docs)
}

fn format_switch_label<'source>(
    label: &SwitchLabel<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    if label.is_default_label() {
        return label
            .default_token()
            .as_ref()
            .map_or_else(jolt_fmt_ir::nil, format_token_with_comments);
    }

    concat([
        label
            .case_token()
            .as_ref()
            .map_or_else(jolt_fmt_ir::nil, |token| {
                concat([format_token_with_comments(token), space()])
            }),
        group(indent(format_switch_label_case_entries(
            label.case_entries_with_recovered(),
            formatter,
        ))),
        label.guard().map_or_else(jolt_fmt_ir::nil, |guard| {
            concat([
                space(),
                guard
                    .when_token()
                    .as_ref()
                    .map_or_else(jolt_fmt_ir::nil, format_token_with_comments),
                space(),
                guard
                    .expression()
                    .map_or_else(jolt_fmt_ir::nil, |expression| {
                        format_expression(&expression, formatter)
                    }),
            ])
        }),
    ])
}

fn format_switch_label_case_entries<'source>(
    entries: impl IntoIterator<
        Item = jolt_java_syntax::RecoveredSeparatedListEntry<
            'source,
            SwitchLabelCaseEntry<'source>,
        >,
    >,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    let mut docs = Vec::new();
    let mut entries = entries.into_iter().peekable();

    while let Some(entry) = entries.next() {
        let has_next = entries.peek().is_some();
        match entry {
            jolt_java_syntax::RecoveredSeparatedListEntry::Entry(entry) => {
                docs.push(format_switch_label_case_item(&entry.item, formatter));
                if let Some(comma) = entry.comma {
                    docs.push(format_separator_with_comments(&comma, line()));
                } else if has_next {
                    docs.push(line());
                }
            }
            jolt_java_syntax::RecoveredSeparatedListEntry::Token(token) => {
                docs.push(format_token(
                    &token,
                    LeadingTrivia::Preserve,
                    TrailingTrivia::Preserve,
                ));
                if has_next {
                    docs.push(line());
                }
            }
            jolt_java_syntax::RecoveredSeparatedListEntry::Error(error) => {
                docs.push(format_token_sequence(
                    error.token_iter(),
                    LeadingTrivia::Preserve,
                ));
                if has_next {
                    docs.push(line());
                }
            }
            jolt_java_syntax::RecoveredSeparatedListEntry::Node(node) => {
                docs.push(format_token_sequence(
                    node.token_iter(),
                    LeadingTrivia::Preserve,
                ));
                if has_next {
                    docs.push(line());
                }
            }
        }
    }

    concat(docs)
}

fn format_switch_label_case_item<'source>(
    item: &SwitchLabelCaseItem<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    match item {
        SwitchLabelCaseItem::Constant(constant) => constant.expression().map_or_else(
            || format_token_sequence(constant.token_iter(), LeadingTrivia::Preserve),
            |expression| format_expression(&expression, formatter),
        ),
        SwitchLabelCaseItem::Pattern(pattern) => pattern.pattern().map_or_else(
            || format_token_sequence(pattern.token_iter(), LeadingTrivia::Preserve),
            |pattern| format_pattern(&pattern, formatter),
        ),
        SwitchLabelCaseItem::Default(default) => format_token(
            default,
            LeadingTrivia::Preserve,
            TrailingTrivia::BeforeLineBreak,
        ),
    }
}

fn format_switch_rule_body<'source>(
    rule: &SwitchRule<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
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
    if rule.semicolon().is_some() {
        return format_statement_semicolon(rule.semicolon());
    }

    jolt_fmt_ir::nil()
}
