use jolt_fmt_ir::{Doc, concat, group, hard_line, indent, line, literal_text, soft_line, text};
use jolt_java_syntax::{
    AssertStatement, BasicForStatement, Block, BlockItem, BlockStatement, CatchClause,
    CatchParameter, CatchTypeList, DoStatement, EnhancedForStatement, Expression,
    ExpressionStatement, FinallyClause, ForInitializer, ForStatement, ForUpdate, IfStatement,
    JavaLexer, JavaSyntaxKind, JavaSyntaxToken, LabeledStatement, Resource, ReturnStatement,
    Statement, StatementBody, StatementExpressionEntry, StatementExpressionList, SwitchBlock,
    SwitchBlockEntry, SwitchBlockStatementGroup, SwitchLabel, SwitchLabelCaseItem, SwitchRule,
    SwitchStatement, SynchronizedStatement, ThrowStatement, TriviaKind, TryStatement,
    TryWithResourcesStatement, Type, WhileStatement, YieldStatement,
};
use std::ops::Range;

use crate::helpers::blocks::{
    BodyItem, braced_block, braced_body, empty_block, join_body_items, join_hard_lines,
};
use crate::helpers::comments::{
    comment_forces_line, format_comment, format_leading_comments, format_token_with_comments,
    format_trailing_comments_before_line_break, trailing_comments_force_line,
};
use crate::helpers::lists::{TrailingSeparator, comma_list, semicolon_list};
use crate::rules::annotations::format_annotation;
use crate::rules::declarations::format_type_declaration;
use crate::rules::expressions::format_expression;
use crate::rules::patterns::format_pattern;
use crate::rules::types::format_type;
use crate::rules::variables::format_local_variable_declaration;

pub(crate) fn format_block(block: &Block) -> Doc {
    braced_body(format_block_body(block))
}

pub(crate) fn format_block_body(block: &Block) -> Option<Doc> {
    format_block_statements_body(block)
}

pub(crate) fn format_block_items_body<'a>(
    items: impl Iterator<Item = BlockItem> + 'a,
) -> Option<Doc> {
    let items = items.filter_map(format_block_item).collect::<Vec<_>>();
    (!items.is_empty()).then(|| join_body_items(items))
}

fn format_block_statements_body(block: &Block) -> Option<Doc> {
    let statements = block.block_statements().collect::<Vec<_>>();
    let block_start = block.text_range().start().get();
    let ignored_ranges = formatter_ignore_ranges(block);
    let items = format_block_statement_items(&statements, block_start, &ignored_ranges);
    (!items.is_empty()).then(|| join_body_items(items))
}

fn format_block_statement_items(
    statements: &[BlockStatement],
    block_start: usize,
    ignored_ranges: &[FormatterIgnoreRange],
) -> Vec<BodyItem> {
    let statement_ranges = statements
        .iter()
        .map(|statement| block_statement_token_range(statement, block_start))
        .collect::<Vec<_>>();
    let ignored_runs = ignored_ranges
        .iter()
        .map(|range| ignored_run(range, &statement_ranges))
        .collect::<Vec<_>>();

    let mut items = Vec::new();
    let mut ignored_index = 0;
    let mut skip_index = 0;
    for (statement_index, statement) in statements.iter().enumerate() {
        while ignored_index < ignored_runs.len()
            && ignored_runs[ignored_index].insert_index == statement_index
        {
            let run = &ignored_runs[ignored_index];
            items.push(BodyItem::new(formatter_ignore_range_doc(&run.range), false));
            ignored_index += 1;
        }

        while skip_index < ignored_runs.len()
            && ignored_runs[skip_index].skip_end <= statement_index
        {
            skip_index += 1;
        }

        if skip_index < ignored_runs.len() && ignored_runs[skip_index].skips(statement_index) {
            continue;
        }

        if let Some(mut item) = statement.item().and_then(format_block_item) {
            if skip_index > 0 && ignored_runs[skip_index - 1].skip_end == statement_index {
                item = item.without_blank_line_before();
            }
            items.push(item);
        }
    }

    while ignored_index < ignored_runs.len() {
        let run = &ignored_runs[ignored_index];
        items.push(BodyItem::new(formatter_ignore_range_doc(&run.range), false));
        ignored_index += 1;
    }

    items
}

fn format_block_item(item: BlockItem) -> Option<BodyItem> {
    let starts_after_blank_line = item.starts_after_blank_line();
    let doc = match item {
        BlockItem::EmptyStatement(_) => None,
        BlockItem::LocalVariableDeclaration(declaration) => Some(concat([
            format_local_variable_declaration(&declaration),
            text(";"),
        ])),
        BlockItem::LocalClassOrInterfaceDeclaration(declaration) => declaration
            .declaration()
            .map(|declaration| format_type_declaration(&declaration)),
        BlockItem::Block(block) => Some(format_block(&block)),
        BlockItem::LabeledStatement(statement) => Some(format_statement(&statement.into())),
        BlockItem::ExpressionStatement(statement) => Some(format_statement(&statement.into())),
        BlockItem::IfStatement(statement) => Some(format_statement(&statement.into())),
        BlockItem::AssertStatement(statement) => Some(format_statement(&statement.into())),
        BlockItem::SwitchStatement(statement) => Some(format_statement(&statement.into())),
        BlockItem::WhileStatement(statement) => Some(format_statement(&statement.into())),
        BlockItem::DoStatement(statement) => Some(format_statement(&statement.into())),
        BlockItem::ForStatement(statement) => Some(format_statement(&statement.into())),
        BlockItem::BreakStatement(statement) => Some(format_statement(&statement.into())),
        BlockItem::YieldStatement(statement) => Some(format_statement(&statement.into())),
        BlockItem::ContinueStatement(statement) => Some(format_statement(&statement.into())),
        BlockItem::ReturnStatement(statement) => Some(format_statement(&statement.into())),
        BlockItem::ThrowStatement(statement) => Some(format_statement(&statement.into())),
        BlockItem::SynchronizedStatement(statement) => Some(format_statement(&statement.into())),
        BlockItem::TryStatement(statement) => Some(format_statement(&statement.into())),
        BlockItem::TryWithResourcesStatement(statement) => {
            Some(format_statement(&statement.into()))
        }
    }?;
    Some(BodyItem::new(doc, starts_after_blank_line))
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct FormatterIgnoreRange {
    raw_text: String,
    interior: Range<usize>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct FormatterIgnoreRun {
    range: FormatterIgnoreRange,
    insert_index: usize,
    skip_start: usize,
    skip_end: usize,
}

impl FormatterIgnoreRun {
    fn skips(&self, statement_index: usize) -> bool {
        (self.skip_start..self.skip_end).contains(&statement_index)
    }
}

fn formatter_ignore_ranges(block: &Block) -> Vec<FormatterIgnoreRange> {
    let source = block.source_text();
    let mut lexer = JavaLexer::new(&source);
    let mut off_comment_start = None;
    let mut ranges = Vec::new();

    loop {
        let token = lexer.next_token();
        for trivia in token.leading.iter().chain(token.trailing.iter()) {
            if !matches!(
                trivia.kind,
                TriviaKind::LineComment | TriviaKind::BlockComment | TriviaKind::JavadocComment
            ) {
                continue;
            }

            let comment_text = &source[trivia.range.start().get()..trivia.range.end().get()];
            if is_formatter_off_marker(comment_text) {
                off_comment_start = Some(line_start(&source, trivia.range.start().get()));
            } else if is_formatter_on_marker(comment_text)
                && let Some(start) = off_comment_start.take()
            {
                let end = line_start(&source, trivia.range.start().get());
                if start < end {
                    ranges.push(FormatterIgnoreRange {
                        raw_text: strip_trailing_line_ending(&source[start..end]).to_owned(),
                        interior: start..end,
                    });
                }
            }
        }

        if token.kind == JavaSyntaxKind::Eof {
            break;
        }
    }

    ranges
}

fn ignored_run(
    range: &FormatterIgnoreRange,
    statement_ranges: &[Option<Range<usize>>],
) -> FormatterIgnoreRun {
    let skipped = statement_ranges
        .iter()
        .enumerate()
        .filter_map(|(index, statement_range)| {
            let statement_range = statement_range.as_ref()?;
            range
                .interior
                .contains(&statement_range.start)
                .then_some(index)
        })
        .collect::<Vec<_>>();

    let insert_index = skipped.first().copied().unwrap_or_else(|| {
        statement_ranges
            .iter()
            .position(|statement_range| {
                statement_range
                    .as_ref()
                    .is_some_and(|statement_range| range.interior.start < statement_range.start)
            })
            .unwrap_or(statement_ranges.len())
    });
    let skip_start = skipped.first().copied().unwrap_or(insert_index);
    let skip_end = skipped.last().map_or(skip_start, |last| last + 1);

    FormatterIgnoreRun {
        range: range.clone(),
        insert_index,
        skip_start,
        skip_end,
    }
}

fn block_statement_token_range(
    statement: &BlockStatement,
    block_start: usize,
) -> Option<Range<usize>> {
    let tokens = statement.tokens();
    let first = tokens.first()?;
    let last = tokens.last()?;
    Some(
        first.token_text_range().start().get() - block_start
            ..last.token_text_range().end().get() - block_start,
    )
}

fn formatter_ignore_range_doc(range: &FormatterIgnoreRange) -> Doc {
    let lines = strip_first_line_indent(&range.raw_text)
        .split('\n')
        .map(|line| literal_text(line.to_owned()))
        .collect::<Vec<_>>();
    join_hard_lines(lines)
}

fn strip_first_line_indent(text: &str) -> String {
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    let Some(first_line) = normalized.lines().find(|line| !line.trim().is_empty()) else {
        return normalized;
    };
    let indent = leading_indent(first_line);
    if indent.is_empty() {
        return normalized;
    }

    normalized
        .split('\n')
        .map(|line| line.strip_prefix(indent).unwrap_or(line))
        .collect::<Vec<_>>()
        .join("\n")
}

fn leading_indent(line: &str) -> &str {
    let indent_end = line
        .char_indices()
        .find_map(|(index, character)| (!matches!(character, ' ' | '\t')).then_some(index))
        .unwrap_or(line.len());
    &line[..indent_end]
}

fn strip_trailing_line_ending(text: &str) -> &str {
    text.strip_suffix("\r\n")
        .or_else(|| text.strip_suffix('\n'))
        .or_else(|| text.strip_suffix('\r'))
        .unwrap_or(text)
}

fn line_start(source: &str, offset: usize) -> usize {
    source[..offset]
        .rfind(['\n', '\r'])
        .map_or(0, |newline| newline + 1)
}

fn is_formatter_off_marker(comment: &str) -> bool {
    comment.contains("@formatter:off")
}

fn is_formatter_on_marker(comment: &str) -> bool {
    comment.contains("@formatter:on")
}

fn format_statement(statement: &Statement) -> Doc {
    match statement {
        Statement::Block(block) => format_block(block),
        Statement::EmptyStatement(_) => empty_block(),
        Statement::LabeledStatement(statement) => format_labeled_statement(statement),
        Statement::ExpressionStatement(statement) => format_expression_statement(statement),
        Statement::IfStatement(statement) => format_if_statement(statement),
        Statement::AssertStatement(statement) => format_assert_statement(statement),
        Statement::SwitchStatement(statement) => format_switch_statement(statement),
        Statement::WhileStatement(statement) => format_while_statement(statement),
        Statement::DoStatement(statement) => format_do_statement(statement),
        Statement::ForStatement(statement) => format_for_statement(statement),
        Statement::BreakStatement(statement) => format_jump_statement("break", statement.label()),
        Statement::YieldStatement(statement) => format_yield_statement(statement),
        Statement::ContinueStatement(statement) => {
            format_jump_statement("continue", statement.label())
        }
        Statement::ReturnStatement(statement) => format_return_statement(statement),
        Statement::ThrowStatement(statement) => format_throw_statement(statement),
        Statement::SynchronizedStatement(statement) => format_synchronized_statement(statement),
        Statement::TryStatement(statement) => format_try_statement(statement),
        Statement::TryWithResourcesStatement(statement) => {
            format_try_with_resources_statement(statement)
        }
    }
}

fn format_labeled_statement(statement: &LabeledStatement) -> Doc {
    let label = statement
        .label()
        .map_or_else(String::new, |label| label.text().to_owned());

    concat([
        text(label),
        text(":"),
        hard_line(),
        statement
            .body()
            .map_or_else(jolt_fmt_ir::nil, |body| format_statement(&body)),
    ])
}

fn format_expression_statement(statement: &ExpressionStatement) -> Doc {
    concat([
        statement
            .expression()
            .map_or_else(jolt_fmt_ir::nil, |expression| {
                format_expression(&expression)
            }),
        text(";"),
    ])
}

fn format_if_statement(statement: &IfStatement) -> Doc {
    let condition = statement
        .condition()
        .map_or_else(jolt_fmt_ir::nil, |condition| format_expression(&condition));
    let then_body = statement_body_as_block(statement.then_body());

    concat([
        text("if ("),
        condition,
        text(") "),
        then_body,
        statement
            .else_body()
            .map_or_else(jolt_fmt_ir::nil, |else_body| {
                concat([
                    text(" else "),
                    match else_body {
                        StatementBody::Unbraced(Statement::IfStatement(else_if)) => {
                            format_if_statement(&else_if)
                        }
                        body => statement_body_as_block(Some(body)),
                    },
                ])
            }),
    ])
}

fn format_assert_statement(statement: &AssertStatement) -> Doc {
    concat([
        text("assert "),
        statement
            .condition()
            .map_or_else(jolt_fmt_ir::nil, |condition| format_expression(&condition)),
        statement.detail().map_or_else(jolt_fmt_ir::nil, |detail| {
            concat([text(" : "), format_expression(&detail)])
        }),
        text(";"),
    ])
}

fn format_while_statement(statement: &WhileStatement) -> Doc {
    let condition = statement
        .condition()
        .map_or_else(jolt_fmt_ir::nil, |condition| format_expression(&condition));
    concat([
        text("while ("),
        condition,
        text(") "),
        statement_body_as_block(statement.statement_body()),
    ])
}

fn format_do_statement(statement: &DoStatement) -> Doc {
    concat([
        text("do "),
        statement_body_as_block(statement.statement_body()),
        text(" while ("),
        statement
            .condition()
            .map_or_else(jolt_fmt_ir::nil, |condition| format_expression(&condition)),
        text(");"),
    ])
}

fn format_for_statement(statement: &ForStatement) -> Doc {
    if let Some(basic) = statement.basic() {
        return format_basic_for_statement(&basic);
    }
    if let Some(enhanced) = statement.enhanced() {
        return format_enhanced_for_statement(&enhanced);
    }

    jolt_fmt_ir::nil()
}

fn format_basic_for_statement(statement: &BasicForStatement) -> Doc {
    let initializer = statement
        .initializer()
        .map(|initializer| format_for_initializer(&initializer));
    let condition = statement
        .condition()
        .map(|condition| format_expression(&condition));
    let update = statement.update().map(|update| format_for_update(&update));
    let is_empty_header = initializer.is_none() && condition.is_none() && update.is_none();
    let header = if is_empty_header {
        concat([text("for ("), text(";;"), text(")")])
    } else {
        group(concat([
            text("for ("),
            indent(concat([
                soft_line(),
                semicolon_list(vec![
                    initializer.unwrap_or_else(jolt_fmt_ir::nil),
                    condition.unwrap_or_else(jolt_fmt_ir::nil),
                    update.unwrap_or_else(jolt_fmt_ir::nil),
                ]),
            ])),
            soft_line(),
            text(")"),
        ]))
    };

    concat([
        header,
        text(" "),
        statement_body_as_block(statement.statement_body()),
    ])
}

fn format_enhanced_for_statement(statement: &EnhancedForStatement) -> Doc {
    concat([
        text("for ("),
        statement
            .variable()
            .map_or_else(jolt_fmt_ir::nil, |variable| {
                format_local_variable_declaration(&variable)
            }),
        text(" : "),
        statement
            .iterable()
            .map_or_else(jolt_fmt_ir::nil, |iterable| format_expression(&iterable)),
        text(") "),
        statement_body_as_block(statement.statement_body()),
    ])
}

fn format_for_initializer(initializer: &ForInitializer) -> Doc {
    if let Some(declaration) = initializer.local_variable_declaration() {
        return format_local_variable_declaration(&declaration);
    }
    initializer
        .expressions()
        .map_or_else(jolt_fmt_ir::nil, |expressions| {
            format_statement_expression_list(&expressions)
        })
}

fn format_for_update(update: &ForUpdate) -> Doc {
    update
        .expressions()
        .map_or_else(jolt_fmt_ir::nil, |expressions| {
            format_statement_expression_list(&expressions)
        })
}

fn format_statement_expression_list(expressions: &StatementExpressionList) -> Doc {
    format_statement_expression_entries(expressions.entries().collect())
}

fn format_statement_expression_entries(entries: Vec<StatementExpressionEntry>) -> Doc {
    let mut docs = Vec::new();
    let entries_len = entries.len();

    for (index, entry) in entries.into_iter().enumerate() {
        docs.push(format_expression(&entry.expression));
        if let Some(comma) = entry.comma {
            docs.push(format_statement_expression_separator(&comma));
        } else if index + 1 < entries_len {
            docs.push(line());
        }
    }

    concat(docs)
}

fn format_statement_expression_separator(comma: &JavaSyntaxToken) -> Doc {
    concat([
        format_leading_comments(comma),
        text(","),
        format_trailing_comments_before_line_break(comma),
        if trailing_comments_force_line(comma) {
            hard_line()
        } else {
            text(" ")
        },
    ])
}

fn format_return_statement(statement: &ReturnStatement) -> Doc {
    format_keyword_expression_statement("return", statement.expression())
}

fn format_throw_statement(statement: &ThrowStatement) -> Doc {
    format_keyword_expression_statement("throw", statement.expression())
}

fn format_yield_statement(statement: &YieldStatement) -> Doc {
    format_keyword_expression_statement("yield", statement.expression())
}

fn format_keyword_expression_statement(keyword: &str, expression: Option<Expression>) -> Doc {
    concat([
        text(keyword.to_owned()),
        expression.map_or_else(jolt_fmt_ir::nil, |expression| {
            let expression_doc = concat([text(" "), format_expression(&expression)]);
            if matches!(expression, Expression::SwitchExpression(_)) {
                expression_doc
            } else {
                indent(expression_doc)
            }
        }),
        text(";"),
    ])
}

fn format_jump_statement(keyword: &str, label: Option<jolt_java_syntax::JavaSyntaxToken>) -> Doc {
    concat([
        text(keyword.to_owned()),
        label.map_or_else(jolt_fmt_ir::nil, |label| {
            concat([text(" "), text(label.text().to_owned())])
        }),
        text(";"),
    ])
}

fn format_synchronized_statement(statement: &SynchronizedStatement) -> Doc {
    concat([
        text("synchronized ("),
        statement
            .expression()
            .map_or_else(jolt_fmt_ir::nil, |expression| {
                format_expression(&expression)
            }),
        text(") "),
        statement
            .body()
            .map_or_else(empty_block, |body| format_block(&body)),
    ])
}

fn format_switch_statement(statement: &SwitchStatement) -> Doc {
    concat([
        text("switch ("),
        statement
            .selector()
            .map_or_else(jolt_fmt_ir::nil, |selector| format_expression(&selector)),
        text(") "),
        statement
            .block()
            .map_or_else(empty_block, |block| format_switch_block(&block)),
    ])
}

pub(crate) fn format_switch_block(block: &SwitchBlock) -> Doc {
    let entries = block
        .entries()
        .map(|entry| match entry {
            SwitchBlockEntry::StatementGroup(group) => format_switch_statement_group(&group),
            SwitchBlockEntry::Rule(rule) => format_switch_rule(&rule),
        })
        .collect::<Vec<_>>();

    braced_block(entries)
}

fn format_switch_statement_group(group: &SwitchBlockStatementGroup) -> Doc {
    let labels = group
        .labels()
        .map(|label| concat([format_switch_label(&label), text(":")]))
        .collect::<Vec<_>>();
    let items = group
        .items()
        .filter_map(format_block_item)
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

fn format_switch_rule(rule: &SwitchRule) -> Doc {
    let label = rule
        .label()
        .map_or_else(jolt_fmt_ir::nil, |label| format_switch_label(&label));

    concat([
        label,
        format_switch_rule_arrow(rule),
        format_switch_rule_body(rule),
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

fn format_switch_label(label: &SwitchLabel) -> Doc {
    if label.is_default_label() {
        return text("default");
    }

    let items = label
        .case_items()
        .map(format_switch_label_case_item)
        .collect::<Vec<_>>();

    concat([
        text("case "),
        group(indent(comma_list(items, TrailingSeparator::Never))),
        label.guard().map_or_else(jolt_fmt_ir::nil, |guard| {
            concat([
                text(" when "),
                guard
                    .expression()
                    .map_or_else(jolt_fmt_ir::nil, |expression| {
                        format_expression(&expression)
                    }),
            ])
        }),
    ])
}

fn format_switch_label_case_item(item: SwitchLabelCaseItem) -> Doc {
    match item {
        SwitchLabelCaseItem::Constant(constant) => constant
            .expression()
            .map_or_else(jolt_fmt_ir::nil, |expression| {
                format_expression(&expression)
            }),
        SwitchLabelCaseItem::Pattern(pattern) => pattern
            .pattern()
            .map_or_else(jolt_fmt_ir::nil, |pattern| format_pattern(&pattern)),
        SwitchLabelCaseItem::Default(_) => text("default"),
    }
}

fn format_switch_rule_body(rule: &SwitchRule) -> Doc {
    if let Some(block) = rule.block() {
        return format_block(&block);
    }
    if let Some(statement) = rule.throw_statement() {
        return format_throw_statement(&statement);
    }
    if let Some(expression) = rule.expression() {
        return concat([format_expression(&expression), text(";")]);
    }

    jolt_fmt_ir::nil()
}

fn format_try_statement(statement: &TryStatement) -> Doc {
    if let Some(resources_statement) = statement.resources_statement() {
        return format_try_with_resources_statement(&resources_statement);
    }

    concat([
        text("try "),
        statement
            .body()
            .map_or_else(empty_block, |body| format_block(&body)),
        format_catch_clauses(statement.catch_clauses()),
        statement
            .finally_clause()
            .map_or_else(jolt_fmt_ir::nil, |finally_clause| {
                format_finally_clause(&finally_clause)
            }),
    ])
}

fn format_try_with_resources_statement(statement: &TryWithResourcesStatement) -> Doc {
    concat([
        text("try ("),
        format_resource_specification(statement),
        text(") "),
        statement
            .body()
            .map_or_else(empty_block, |body| format_block(&body)),
        format_catch_clauses(statement.catch_clauses()),
        statement
            .finally_clause()
            .map_or_else(jolt_fmt_ir::nil, |finally_clause| {
                format_finally_clause(&finally_clause)
            }),
    ])
}

fn format_resource_specification(statement: &TryWithResourcesStatement) -> Doc {
    let resources = statement
        .resources()
        .and_then(|specification| specification.list())
        .map(|list| {
            list.resources()
                .map(|resource| format_resource(&resource))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    if resources.is_empty() {
        return jolt_fmt_ir::nil();
    }

    concat([
        jolt_fmt_ir::indent(concat([hard_line(), join_resource_lines(resources)])),
        hard_line(),
    ])
}

fn format_resource(resource: &Resource) -> Doc {
    if let Some(declaration) = resource.declaration() {
        return format_local_variable_declaration(&declaration);
    }
    if let Some(access) = resource.variable_access() {
        return access
            .expression()
            .map_or_else(jolt_fmt_ir::nil, |expression| {
                format_expression(&expression)
            });
    }

    jolt_fmt_ir::nil()
}

fn format_catch_clauses<'a>(clauses: impl Iterator<Item = CatchClause> + 'a) -> Doc {
    concat(clauses.map(|clause| format_catch_clause(&clause)))
}

fn format_catch_clause(clause: &CatchClause) -> Doc {
    concat([
        text(" catch "),
        clause
            .parameter()
            .map_or_else(jolt_fmt_ir::nil, |parameter| {
                format_parenthesized_catch_parameter(&parameter)
            }),
        text(" "),
        clause
            .body()
            .map_or_else(empty_block, |body| format_block(&body)),
    ])
}

fn format_parenthesized_catch_parameter(parameter: &CatchParameter) -> Doc {
    group(concat([
        text("("),
        indent(concat([soft_line(), format_catch_parameter(parameter)])),
        soft_line(),
        text(")"),
    ]))
}

fn format_catch_parameter(parameter: &CatchParameter) -> Doc {
    concat([
        format_catch_modifier_prefix(parameter),
        parameter.types().map_or_else(jolt_fmt_ir::nil, |types| {
            format_catch_type_list(&types, parameter.name())
        }),
    ])
}

fn format_catch_modifier_prefix(parameter: &CatchParameter) -> Doc {
    let mut docs = parameter
        .annotations()
        .map(|annotation| format_annotation(&annotation))
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
) -> Doc {
    let mut entries = types.entries().collect::<Vec<_>>();
    let name = name.map_or_else(jolt_fmt_ir::nil, |name| format_token_with_comments(&name));

    let Some(last_entry) = entries.pop() else {
        return name;
    };

    let last = concat([format_catch_type(&last_entry.ty), text(" "), name]);
    if entries.is_empty() {
        return last;
    }

    let first = entries.remove(0);
    group(concat([
        format_catch_type(&first.ty),
        format_catch_type_separator(first.separator.as_ref()),
        concat(entries.into_iter().map(|entry| {
            concat([
                format_catch_type(&entry.ty),
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

fn format_catch_type(ty: &Type) -> Doc {
    format_type(ty)
}

fn format_finally_clause(clause: &FinallyClause) -> Doc {
    concat([
        text(" finally "),
        clause
            .body()
            .map_or_else(empty_block, |body| format_block(&body)),
    ])
}

fn join_resource_lines(docs: Vec<Doc>) -> Doc {
    let mut joined = Vec::new();
    for (index, doc) in docs.into_iter().enumerate() {
        if index > 0 {
            joined.push(text(";"));
            joined.push(hard_line());
        }
        joined.push(doc);
    }
    concat(joined)
}

fn statement_body_as_block(body: Option<StatementBody>) -> Doc {
    match body {
        Some(StatementBody::Block(block)) => format_block(&block),
        Some(StatementBody::Empty(_)) | None => empty_block(),
        Some(StatementBody::Unbraced(statement)) => braced_body(Some(format_statement(&statement))),
    }
}
