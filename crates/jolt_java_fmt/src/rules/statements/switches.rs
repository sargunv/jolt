use super::control_flow::{
    format_parenthesized_statement_expression, format_statement_header_body_separator,
};
use super::simple::format_statement_keyword;
use super::{
    BlockItem, BlockStatement, Doc, LeadingTrivia, SwitchBlock, SwitchBlockStatementGroup,
    SwitchLabel, SwitchRule, SwitchStatement, TrailingTrivia, comment_forces_line, format_block,
    format_block_statement_item, format_expression, format_pattern, format_separator_with_comments,
    format_statement_semicolon, format_throw_statement, format_token, format_token_with_comments,
    join_body_items,
};
use crate::helpers::blocks::BodyItem;
use crate::helpers::comments::token_has_comments;
use crate::helpers::recovery::{
    JavaFormatDelimiter, JavaFormatField, JavaFormatListPart, format_malformed, resolve_list_part,
    resolve_optional_field, resolve_required_delimiter, resolve_required_field,
};
use jolt_fmt_ir::DocBuilder;
use jolt_java_syntax::{
    JavaSyntaxListPart, JavaSyntaxView, Pattern, SwitchEntrySyntax, SwitchGuardSyntax,
    SwitchLabelItemSyntax, SwitchRuleBodySyntax,
};

pub(super) fn format_switch_statement<'source>(
    statement: &SwitchStatement<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let close = resolve_required_delimiter(statement.close_paren(), doc);
    let separator = format_statement_header_body_separator(close.source(), doc);
    let selector = match resolve_required_field(statement.selector(), doc) {
        JavaFormatField::Present(selector) => format_expression(&selector, doc),
        JavaFormatField::Malformed(malformed) => malformed,
    };
    let open = resolve_required_delimiter(statement.open_paren(), doc);
    let selector = format_parenthesized_statement_expression(doc, open, selector, close);
    let body = match resolve_required_field(statement.body(), doc) {
        JavaFormatField::Present(block) => format_switch_block(&block, doc),
        JavaFormatField::Malformed(malformed) => malformed,
    };
    doc_concat!(
        doc,
        [
            format_statement_keyword(statement.switch_keyword(), doc),
            doc.space(),
            selector,
            separator,
            body,
        ]
    )
}

pub(crate) fn format_switch_block<'source>(
    block: &SwitchBlock<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let entries = match resolve_required_field(block.entries(), doc) {
        JavaFormatField::Present(entries) => entries,
        JavaFormatField::Malformed(malformed) => {
            return braced_switch_block(block, Some(malformed), doc);
        }
    };
    let mut has_body = false;
    let body = doc.concat_list(|docs| {
        for part in entries.parts() {
            let entry = match resolve_list_part(part, docs) {
                JavaFormatListPart::Item(entry) => match entry {
                    SwitchEntrySyntax::SwitchBlockStatementGroup(group) => {
                        format_switch_statement_group(&group, docs)
                    }
                    SwitchEntrySyntax::SwitchRule(rule) => format_switch_rule(&rule, docs),
                    SwitchEntrySyntax::BogusSwitchEntry(bogus) => format_malformed(&bogus, docs),
                },
                JavaFormatListPart::Malformed(malformed) => malformed,
                JavaFormatListPart::Separator(separator) => {
                    docs.block_on_invariant("unseparated switch entry list had a separator");
                    format_token_with_comments(docs, &separator)
                }
            };
            if !docs.is_empty() {
                let hard_line = docs.hard_line();
                docs.push(hard_line);
            }
            docs.push(entry);
        }
        has_body = !docs.is_empty();
    });
    braced_switch_block(block, has_body.then_some(body), doc)
}

fn braced_switch_block<'source>(
    block: &SwitchBlock<'source>,
    body: Option<Doc<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let open = match resolve_required_field(block.open_brace(), doc) {
        JavaFormatField::Present(token) => format_token_with_comments(doc, &token),
        JavaFormatField::Malformed(malformed) => malformed,
    };
    let body = match body {
        Some(body) => {
            let body = doc_concat!(doc, [doc.hard_line(), body]);
            doc_concat!(doc, [doc_indent!(doc, body), doc.hard_line()])
        }
        None => doc.hard_line(),
    };
    let close = match resolve_required_field(block.close_brace(), doc) {
        JavaFormatField::Present(token) => format_token_with_comments(doc, &token),
        JavaFormatField::Malformed(malformed) => malformed,
    };
    doc_concat!(doc, [open, body, close])
}

fn format_switch_statement_group<'source>(
    group: &SwitchBlockStatementGroup<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let (labels_doc, label_count, single_label) = match resolve_required_field(group.labels(), doc)
    {
        JavaFormatField::Present(labels) => format_switch_group_labels(labels.parts(), doc),
        JavaFormatField::Malformed(malformed) => (malformed, 0, None),
    };
    let statements = match resolve_required_field(group.statements(), doc) {
        JavaFormatField::Present(statements) => statements,
        JavaFormatField::Malformed(malformed) => {
            return doc_concat!(doc, [labels_doc, doc.hard_line(), malformed]);
        }
    };
    let statement_parts = statements.parts().collect::<Vec<_>>();
    if let [Ok(JavaSyntaxListPart::Item(statement))] = statement_parts.as_slice()
        && let Some(single) = format_single_block_switch_statement_group(
            label_count,
            single_label,
            std::slice::from_ref(statement),
            doc,
        )
    {
        return single;
    }

    let items = statement_parts
        .iter()
        .filter_map(|part| match part {
            Ok(JavaSyntaxListPart::Item(statement)) => format_block_statement_item(statement, doc),
            Ok(JavaSyntaxListPart::Malformed(malformed)) => {
                Some(BodyItem::new(format_malformed(malformed, doc), false))
            }
            Ok(JavaSyntaxListPart::Missing(missing)) => Some(BodyItem::new(
                crate::helpers::recovery::format_missing(missing, doc),
                false,
            )),
            Ok(JavaSyntaxListPart::Separator(token)) => {
                doc.block_on_invariant("unseparated switch statement list had a separator");
                Some(BodyItem::new(format_token_with_comments(doc, token), false))
            }
            Err(error) => {
                doc.block_on_invariant(error.to_string());
                None
            }
        })
        .collect::<Vec<_>>();
    doc_concat!(
        doc,
        [
            labels_doc,
            if items.is_empty() {
                Doc::nil()
            } else {
                let body = doc_concat!(doc, [doc.hard_line(), join_body_items(doc, items)]);
                doc_indent!(doc, body)
            },
        ]
    )
}

fn format_switch_group_labels<'source>(
    parts: impl IntoIterator<
        Item = Result<
            JavaSyntaxListPart<'source, SwitchLabel<'source>>,
            jolt_java_syntax::JavaSyntaxInvariantError,
        >,
    >,
    doc: &mut DocBuilder<'source>,
) -> (Doc<'source>, usize, Option<Doc<'source>>) {
    let mut label_count = 0;
    let mut single_label = None;
    let labels_doc = doc.concat_list(|docs| {
        let mut pending: Option<Doc<'source>> = None;
        for part in parts {
            match resolve_list_part(part, docs) {
                JavaFormatListPart::Item(label) => {
                    if let Some(previous) = pending.take() {
                        if !docs.is_empty() {
                            let line = docs.hard_line();
                            docs.push(line);
                        }
                        docs.push(previous);
                    }
                    let formatted = format_switch_label(&label, docs);
                    label_count += 1;
                    single_label = Some(formatted);
                    pending = Some(formatted);
                }
                JavaFormatListPart::Separator(colon) => {
                    let label = pending.take().unwrap_or_else(Doc::nil);
                    let formatted =
                        doc_concat!(docs, [label, format_token_with_comments(docs, &colon)]);
                    single_label = Some(formatted);
                    if !docs.is_empty() {
                        let line = docs.hard_line();
                        docs.push(line);
                    }
                    docs.push(formatted);
                }
                JavaFormatListPart::Malformed(malformed) => {
                    if let Some(previous) = pending.take() {
                        docs.push(previous);
                    }
                    docs.push(malformed);
                }
            }
        }
        if let Some(pending) = pending {
            docs.push(pending);
        }
    });
    (labels_doc, label_count, single_label)
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
    let jolt_java_syntax::JavaSyntaxField::Present(item) = statements[0].item().ok()? else {
        return None;
    };
    let BlockItem::Block(block) = item else {
        return None;
    };
    Some(doc_concat!(
        doc,
        [label?, doc.space(), format_block(&block, doc)]
    ))
}

fn format_switch_rule<'source>(
    rule: &SwitchRule<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let label = match resolve_required_field(rule.label(), doc) {
        JavaFormatField::Present(label) => format_switch_label(&label, doc),
        JavaFormatField::Malformed(malformed) => malformed,
    };
    let body = match resolve_required_field(rule.body(), doc) {
        JavaFormatField::Present(body) => body,
        JavaFormatField::Malformed(malformed) => {
            return doc_concat!(
                doc,
                [
                    label,
                    format_switch_rule_arrow(rule, doc),
                    malformed,
                    format_switch_rule_semicolon(rule, doc),
                ]
            );
        }
    };
    match body.classify() {
        Ok(SwitchRuleBodySyntax::Expression(expression)) => doc_concat!(
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
        ),
        Ok(body) => doc_concat!(
            doc,
            [
                label,
                format_switch_rule_arrow(rule, doc),
                format_switch_rule_body(body, doc),
                format_switch_rule_semicolon(rule, doc),
            ]
        ),
        Err(error) => {
            doc.block_on_invariant(error.to_string());
            label
        }
    }
}

fn format_switch_rule_semicolon<'source>(
    rule: &SwitchRule<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    crate::helpers::recovery::format_optional_field(rule.semicolon(), doc, |semicolon, doc| {
        format_statement_semicolon(
            Ok(jolt_java_syntax::JavaSyntaxField::Present(semicolon)),
            doc,
        )
    })
}

fn switch_arrow<'source>(
    rule: &SwitchRule<'source>,
) -> Option<jolt_java_syntax::JavaSyntaxToken<'source>> {
    match rule.arrow().ok()? {
        jolt_java_syntax::JavaSyntaxField::Present(arrow) => Some(arrow),
        _ => None,
    }
}

fn format_switch_rule_arrow_head<'source>(
    rule: &SwitchRule<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let Some(arrow) = switch_arrow(rule) else {
        return match resolve_required_field(rule.arrow(), doc) {
            JavaFormatField::Malformed(doc) => doc,
            JavaFormatField::Present(_) => Doc::nil(),
        };
    };
    if arrow.trailing_comments().next().is_none() {
        doc_concat!(doc, [doc.space(), format_token_with_comments(doc, &arrow)])
    } else {
        doc_concat!(
            doc,
            [
                doc.space(),
                format_token(
                    doc,
                    &arrow,
                    LeadingTrivia::Preserve,
                    TrailingTrivia::BeforeLineBreak
                )
            ]
        )
    }
}

fn format_switch_rule_arrow_body_separator<'source>(
    rule: &SwitchRule<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    if switch_arrow(rule).is_some_and(|arrow| {
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
    let Some(arrow) = switch_arrow(rule) else {
        return match resolve_required_field(rule.arrow(), doc) {
            JavaFormatField::Malformed(doc) => doc,
            JavaFormatField::Present(_) => Doc::nil(),
        };
    };
    let forced = arrow
        .trailing_comments()
        .any(|comment| comment_forces_line(&comment));
    doc_concat!(
        doc,
        [
            doc.space(),
            format_token(
                doc,
                &arrow,
                LeadingTrivia::Preserve,
                TrailingTrivia::BeforeLineBreak
            ),
            if forced { doc.hard_line() } else { doc.space() }
        ]
    )
}

fn format_switch_label<'source>(
    label: &SwitchLabel<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let (keyword_doc, is_bare_default) = match resolve_required_field(label.keyword(), doc) {
        JavaFormatField::Present(keyword) => (
            format_token_with_comments(doc, &keyword),
            keyword.kind() == jolt_java_syntax::JavaSyntaxKind::DefaultKw,
        ),
        JavaFormatField::Malformed(malformed) => (malformed, false),
    };
    let (items_doc, items_are_empty) = match resolve_required_field(label.items(), doc) {
        JavaFormatField::Present(items) => {
            let items_are_empty = items.parts().next().is_none();
            (
                format_switch_label_items(items.parts(), doc),
                items_are_empty,
            )
        }
        JavaFormatField::Malformed(malformed) => (malformed, false),
    };
    let guard = match resolve_optional_field(label.guard(), doc) {
        JavaFormatField::Present(Some(SwitchGuardSyntax::Guard(guard))) => {
            format_guard(&guard, doc)
        }
        JavaFormatField::Present(Some(SwitchGuardSyntax::BogusSwitchGuard(bogus))) => {
            format_malformed(&bogus, doc)
        }
        JavaFormatField::Present(None) => Doc::nil(),
        JavaFormatField::Malformed(malformed) => malformed,
    };
    if is_bare_default && items_are_empty {
        doc_concat!(doc, [keyword_doc, guard])
    } else {
        doc_concat!(
            doc,
            [
                keyword_doc,
                doc.space(),
                doc_group!(doc, doc_indent!(doc, items_doc)),
                guard
            ]
        )
    }
}

fn format_switch_label_items<'source>(
    parts: impl IntoIterator<
        Item = Result<
            JavaSyntaxListPart<'source, jolt_java_syntax::SwitchLabelItem<'source>>,
            jolt_java_syntax::JavaSyntaxInvariantError,
        >,
    >,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let mut need_line = false;
    doc.concat_list(|docs| {
        for part in parts {
            match resolve_list_part(part, docs) {
                JavaFormatListPart::Item(item) => {
                    if need_line {
                        let line = docs.line();
                        docs.push(line);
                    }
                    let formatted = match item.classify() {
                        Ok(SwitchLabelItemSyntax::CaseConstant(constant)) => {
                            match resolve_required_field(constant.expression(), docs) {
                                JavaFormatField::Present(value) => format_expression(&value, docs),
                                JavaFormatField::Malformed(value) => value,
                            }
                        }
                        Ok(SwitchLabelItemSyntax::CasePattern(pattern)) => {
                            match resolve_required_field(pattern.pattern(), docs) {
                                JavaFormatField::Present(role) => match role {
                                    Pattern::TypePattern(value) => {
                                        format_pattern(&value.into(), docs)
                                    }
                                    Pattern::RecordPattern(value) => {
                                        format_pattern(&value.into(), docs)
                                    }
                                    Pattern::MatchAllPattern(value) => {
                                        format_pattern(&value.into(), docs)
                                    }
                                    Pattern::BogusPattern(value) => format_malformed(&value, docs),
                                },
                                JavaFormatField::Malformed(value) => value,
                            }
                        }
                        Ok(SwitchLabelItemSyntax::BogusSwitchLabelItem(value)) => {
                            format_malformed(&value, docs)
                        }
                        Ok(SwitchLabelItemSyntax::Default(token)) => format_token(
                            docs,
                            &token,
                            LeadingTrivia::Preserve,
                            TrailingTrivia::BeforeLineBreak,
                        ),
                        Err(error) => {
                            docs.block_on_invariant(error.to_string());
                            Doc::nil()
                        }
                    };
                    docs.push(formatted);
                    need_line = true;
                }
                JavaFormatListPart::Separator(comma) => {
                    let line = docs.line();
                    let comma = format_separator_with_comments(docs, &comma, line);
                    docs.push(comma);
                    need_line = false;
                }
                JavaFormatListPart::Malformed(value) => {
                    if need_line {
                        let line = docs.line();
                        docs.push(line);
                    }
                    docs.push(value);
                    need_line = true;
                }
            }
        }
    })
}

fn format_guard<'source>(
    guard: &jolt_java_syntax::Guard<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let when = match resolve_required_field(guard.when_keyword(), doc) {
        JavaFormatField::Present(value) => format_token_with_comments(doc, &value),
        JavaFormatField::Malformed(value) => value,
    };
    let open = resolve_optional_field(guard.open_paren(), doc);
    let close = resolve_optional_field(guard.close_paren(), doc);
    let condition = match resolve_required_field(guard.condition(), doc) {
        JavaFormatField::Present(value) => format_expression(&value, doc),
        JavaFormatField::Malformed(value) => value,
    };
    let condition = match (open, close) {
        (JavaFormatField::Present(Some(open)), JavaFormatField::Present(Some(close)))
            if !token_has_comments(&open) && !token_has_comments(&close) =>
        {
            let removals = guard.redundant_parenthesis_removal_claims();
            let open = removals
                .open
                .map_or_else(Doc::nil, |claim| doc.removed_source(claim));
            let close = removals
                .close
                .map_or_else(Doc::nil, |claim| doc.removed_source(claim));
            doc_concat!(doc, [open, condition, close])
        }
        (JavaFormatField::Present(Some(open)), JavaFormatField::Present(Some(close))) => {
            format_parenthesized_statement_expression(
                doc,
                JavaFormatDelimiter::Source(open),
                condition,
                JavaFormatDelimiter::Source(close),
            )
        }
        (JavaFormatField::Malformed(open), JavaFormatField::Malformed(close)) => {
            doc_concat!(doc, [open, condition, close])
        }
        (JavaFormatField::Malformed(open), _) => doc_concat!(doc, [open, condition]),
        (_, JavaFormatField::Malformed(close)) => doc_concat!(doc, [condition, close]),
        _ => condition,
    };
    doc_concat!(doc, [doc.space(), when, doc.space(), condition])
}

fn format_switch_rule_body<'source>(
    body: SwitchRuleBodySyntax<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    match body {
        SwitchRuleBodySyntax::Block(block) => format_block(&block, doc),
        SwitchRuleBodySyntax::ThrowStatement(statement) => format_throw_statement(&statement, doc),
        SwitchRuleBodySyntax::Expression(expression) => format_expression(&expression, doc),
    }
}
