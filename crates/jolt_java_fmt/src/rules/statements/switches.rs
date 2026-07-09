use super::control_flow::{
    format_parenthesized_statement_expression, format_statement_header_body_separator,
};
use super::simple::format_statement_keyword;
use super::{
    BlockItem, BlockStatement, Doc, LeadingTrivia, SwitchBlock, SwitchBlockEntry,
    SwitchBlockStatementGroup, SwitchLabel, SwitchLabelCaseEntry, SwitchLabelCaseItem, SwitchRule,
    SwitchStatement, TrailingTrivia, comment_forces_line, empty_block, format_block,
    format_block_statement_item_or_recovered, format_expression, format_pattern,
    format_separator_with_comments, format_statement_semicolon, format_throw_statement,
    format_token, format_token_sequence, format_token_with_comments, join_body_items,
};
use jolt_fmt_ir::DocBuilder;

pub(super) fn format_switch_statement<'source>(
    statement: &SwitchStatement<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let open = statement.open_paren();
    let close = statement.close_paren();
    let selector = match statement.selector() {
        Some(selector) => format_expression(&selector, doc),
        None => Doc::nil(),
    };
    let selector =
        format_parenthesized_statement_expression(doc, open.as_ref(), selector, close.as_ref());
    let block = match statement.block() {
        Some(block) => format_switch_block(&block, doc),
        None => empty_block(doc),
    };
    doc_concat!(
        doc,
        [
            format_statement_keyword(statement.keyword(), "switch", doc),
            doc.space(),
            selector,
            format_statement_header_body_separator(close.as_ref(), doc),
            block,
        ]
    )
}

pub(crate) fn format_switch_block<'source>(
    block: &SwitchBlock<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let mut docs = doc.list();
    for entry in block.entries_with_recovered() {
        if !docs.is_empty() {
            docs.push(doc.hard_line(), doc);
        }
        let entry = match entry {
            jolt_java_syntax::RecoveredSeparatedListEntry::Entry(entry) => match entry {
                SwitchBlockEntry::StatementGroup(group) => {
                    format_switch_statement_group(&group, doc)
                }
                SwitchBlockEntry::Rule(rule) => format_switch_rule(&rule, doc),
            },
            jolt_java_syntax::RecoveredSeparatedListEntry::Token(token) => {
                format_token_sequence(doc, std::iter::once(token), LeadingTrivia::Preserve)
            }
            jolt_java_syntax::RecoveredSeparatedListEntry::Error(error) => {
                format_token_sequence(doc, error.token_iter(), LeadingTrivia::Preserve)
            }
            jolt_java_syntax::RecoveredSeparatedListEntry::Node(node) => {
                format_token_sequence(doc, node.token_iter(), LeadingTrivia::Preserve)
            }
        };
        docs.push(entry, doc);
    }

    let body = (!docs.is_empty()).then(|| docs.finish(doc));
    braced_switch_block(block, body, doc)
}

fn braced_switch_block<'source>(
    block: &SwitchBlock<'source>,
    body: Option<Doc<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let open = match block.open_brace().as_ref() {
        Some(token) => format_token_with_comments(doc, token),
        None => Doc::nil(),
    };
    let body = match body {
        Some(body) => {
            let body = doc_concat!(doc, [doc.hard_line(), body]);
            doc_concat!(doc, [doc_indent!(doc, body), doc.hard_line()])
        }
        None => doc.hard_line(),
    };
    let close = match block.close_brace().as_ref() {
        Some(token) => format_token_with_comments(doc, token),
        None => Doc::nil(),
    };

    doc_concat!(doc, [open, body, close])
}

fn format_switch_statement_group<'source>(
    group: &SwitchBlockStatementGroup<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let mut labels = doc.list();
    let mut label_count = 0;
    let mut single_label = None;
    for entry in group.label_entries() {
        let label = doc_concat!(
            doc,
            [
                format_switch_label(&entry.label, doc),
                entry
                    .colon
                    .as_ref()
                    .map_or_else(Doc::nil, |token| format_token_with_comments(doc, token)),
            ]
        );
        if !labels.is_empty() {
            labels.push(doc.hard_line(), doc);
        }
        labels.push(label, doc);
        label_count += 1;
        single_label = Some(label);
    }
    let statements = group.block_statements().collect::<Vec<_>>();

    if let Some(doc) =
        format_single_block_switch_statement_group(label_count, single_label, &statements, doc)
    {
        return doc;
    }

    let mut items = Vec::with_capacity(statements.len());
    items.extend(
        group
            .block_statements_with_recovered()
            .filter_map(|entry| match entry {
                jolt_java_syntax::RecoveredSeparatedListEntry::Entry(statement) => {
                    format_block_statement_item_or_recovered(&statement, doc)
                }
                jolt_java_syntax::RecoveredSeparatedListEntry::Token(token) => {
                    Some(crate::helpers::blocks::BodyItem::new(
                        format_token_sequence(doc, std::iter::once(token), LeadingTrivia::Preserve),
                        false,
                    ))
                }
                jolt_java_syntax::RecoveredSeparatedListEntry::Error(error) => {
                    Some(crate::helpers::blocks::BodyItem::new(
                        format_token_sequence(doc, error.token_iter(), LeadingTrivia::Preserve),
                        false,
                    ))
                }
                jolt_java_syntax::RecoveredSeparatedListEntry::Node(node) => {
                    Some(crate::helpers::blocks::BodyItem::new(
                        format_token_sequence(doc, node.token_iter(), LeadingTrivia::Preserve),
                        false,
                    ))
                }
            }),
    );

    doc_concat!(
        doc,
        [
            labels.finish(doc),
            if items.is_empty() {
                Doc::nil()
            } else {
                doc_indent!(
                    doc,
                    doc_concat!(doc, [doc.hard_line(), join_body_items(doc, items)])
                )
            },
        ]
    )
}

fn format_single_block_switch_statement_group<'source>(
    label_count: usize,
    label: Option<Doc<'source>>,
    statements: &[BlockStatement<'source>],
    doc: &mut DocBuilder<'source>,
) -> Option<Doc<'source>> {
    if label_count != 1 || statements.len() != 1 || statements[0].starts_after_blank_line() {
        return None;
    }

    let BlockItem::Block(block) = statements[0].item()? else {
        return None;
    };

    Some(doc_concat!(
        doc,
        [label?, doc.space(), format_block(&block, doc),]
    ))
}

fn format_switch_rule<'source>(
    rule: &SwitchRule<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let label = rule
        .label()
        .map_or_else(Doc::nil, |label| format_switch_label(&label, doc));

    if let Some(expression) = rule.expression() {
        return doc_concat!(
            doc,
            [
                label,
                doc_group!(
                    doc,
                    doc_concat!(
                        doc,
                        [
                            format_switch_rule_arrow_head(rule, doc),
                            doc_indent!(
                                doc,
                                doc_concat!(
                                    doc,
                                    [
                                        format_switch_rule_arrow_body_separator(rule, doc),
                                        format_expression(&expression, doc),
                                        format_statement_semicolon(rule.semicolon(), doc),
                                    ]
                                )
                            ),
                        ]
                    )
                ),
            ]
        );
    }

    doc_concat!(
        doc,
        [
            label,
            format_switch_rule_arrow(rule, doc),
            format_switch_rule_body(rule, doc),
        ]
    )
}

fn format_switch_rule_arrow_head<'source>(
    rule: &SwitchRule<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let Some(arrow) = rule.arrow() else {
        return Doc::nil();
    };

    let trailing_comments = arrow.trailing_comments();
    if trailing_comments.is_empty() {
        return doc_concat!(doc, [doc.space(), format_token_with_comments(doc, &arrow)]);
    }

    doc_concat!(
        doc,
        [
            doc.space(),
            format_token(
                doc,
                &arrow,
                LeadingTrivia::Preserve,
                TrailingTrivia::BeforeLineBreak,
            ),
        ]
    )
}

fn format_switch_rule_arrow_body_separator<'source>(
    rule: &SwitchRule<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    if rule.arrow().is_some_and(|arrow| {
        arrow
            .trailing_comments()
            .any(|comment| comment_forces_line(&comment))
    }) {
        doc.hard_line()
    } else {
        doc.line()
    }
}

fn format_switch_rule_arrow<'source>(
    rule: &SwitchRule<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let Some(arrow) = rule.arrow() else {
        return Doc::nil();
    };

    let trailing_comments = arrow.trailing_comments();
    if trailing_comments.is_empty() {
        return doc_concat!(
            doc,
            [
                doc.space(),
                format_token_with_comments(doc, &arrow),
                doc.space(),
            ]
        );
    }

    let forced_line = arrow
        .trailing_comments()
        .any(|comment| comment_forces_line(&comment));
    let mut docs = doc.list();
    docs.push(doc.space(), doc);
    let arrow = format_token(
        doc,
        &arrow,
        LeadingTrivia::Preserve,
        TrailingTrivia::BeforeLineBreak,
    );
    docs.push(arrow, doc);
    let separator = if forced_line {
        doc.hard_line()
    } else {
        doc.space()
    };
    docs.push(separator, doc);
    docs.finish(doc)
}

fn format_switch_label<'source>(
    label: &SwitchLabel<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    if label.is_default_label() {
        return label
            .default_token()
            .as_ref()
            .map_or_else(Doc::nil, |token| format_token_with_comments(doc, token));
    }

    doc_concat!(
        doc,
        [
            label
                .case_token()
                .as_ref()
                .map_or_else(Doc::nil, |token| doc_concat!(
                    doc,
                    [format_token_with_comments(doc, token), doc.space()]
                ),),
            doc_group!(
                doc,
                doc_indent!(
                    doc,
                    format_switch_label_case_entries(label.case_entries_with_recovered(), doc,)
                )
            ),
            label.guard().map_or_else(Doc::nil, |guard| {
                doc_concat!(
                    doc,
                    [
                        doc.space(),
                        guard
                            .when_token()
                            .as_ref()
                            .map_or_else(Doc::nil, |token| format_token_with_comments(doc, token)),
                        doc.space(),
                        guard
                            .expression()
                            .map_or_else(Doc::nil, |expression| format_expression(
                                &expression,
                                doc
                            ),),
                    ]
                )
            },),
        ]
    )
}

fn format_switch_label_case_entries<'source>(
    entries: impl IntoIterator<
        Item = jolt_java_syntax::RecoveredSeparatedListEntry<
            'source,
            SwitchLabelCaseEntry<'source>,
        >,
    >,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let mut entries = entries.into_iter().peekable();
    let mut docs = doc.list();

    while let Some(entry) = entries.next() {
        let has_next = entries.peek().is_some();
        match entry {
            jolt_java_syntax::RecoveredSeparatedListEntry::Entry(entry) => {
                docs.push(format_switch_label_case_item(&entry.item, doc), doc);
                if let Some(comma) = entry.comma {
                    let line = doc.line();
                    let comma = format_separator_with_comments(doc, &comma, line);
                    docs.push(comma, doc);
                } else if has_next {
                    docs.push(doc.line(), doc);
                }
            }
            jolt_java_syntax::RecoveredSeparatedListEntry::Token(token) => {
                let token = format_token(
                    doc,
                    &token,
                    LeadingTrivia::Preserve,
                    TrailingTrivia::Preserve,
                );
                docs.push(token, doc);
                if has_next {
                    docs.push(doc.line(), doc);
                }
            }
            jolt_java_syntax::RecoveredSeparatedListEntry::Error(error) => {
                let error = format_token_sequence(doc, error.token_iter(), LeadingTrivia::Preserve);
                docs.push(error, doc);
                if has_next {
                    docs.push(doc.line(), doc);
                }
            }
            jolt_java_syntax::RecoveredSeparatedListEntry::Node(node) => {
                let node = format_token_sequence(doc, node.token_iter(), LeadingTrivia::Preserve);
                docs.push(node, doc);
                if has_next {
                    docs.push(doc.line(), doc);
                }
            }
        }
    }

    docs.finish(doc)
}

fn format_switch_label_case_item<'source>(
    item: &SwitchLabelCaseItem<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    match item {
        SwitchLabelCaseItem::Constant(constant) => match constant.expression() {
            Some(expression) => format_expression(&expression, doc),
            None => format_token_sequence(doc, constant.token_iter(), LeadingTrivia::Preserve),
        },
        SwitchLabelCaseItem::Pattern(pattern) => match pattern.pattern() {
            Some(pattern) => format_pattern(&pattern, doc),
            None => format_token_sequence(doc, pattern.token_iter(), LeadingTrivia::Preserve),
        },
        SwitchLabelCaseItem::Default(default) => format_token(
            doc,
            default,
            LeadingTrivia::Preserve,
            TrailingTrivia::BeforeLineBreak,
        ),
    }
}

fn format_switch_rule_body<'source>(
    rule: &SwitchRule<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    if let Some(block) = rule.block() {
        return format_block(&block, doc);
    }
    if let Some(statement) = rule.throw_statement() {
        return format_throw_statement(&statement, doc);
    }
    if let Some(expression) = rule.expression() {
        return doc_concat!(
            doc,
            [
                format_expression(&expression, doc),
                format_statement_semicolon(rule.semicolon(), doc),
            ]
        );
    }
    if rule.semicolon().is_some() {
        return format_statement_semicolon(rule.semicolon(), doc);
    }

    Doc::nil()
}
