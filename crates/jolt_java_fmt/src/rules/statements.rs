use jolt_fmt_ir::{Doc, concat, group, hard_line, indent, line, soft_line, text};
use jolt_java_syntax::{
    AssertStatement, BasicForStatement, Block, BlockItem, BlockStatement, CatchClause,
    CatchParameter, CatchTypeList, DoStatement, EnhancedForStatement, Expression,
    ExpressionStatement, FinallyClause, ForInitializer, ForStatement, ForUpdate, IfStatement,
    JavaComment, JavaSyntaxToken, LabeledStatement, Resource, ResourceListEntry, ReturnStatement,
    Statement, StatementBody, StatementExpressionEntry, StatementExpressionList, SwitchBlock,
    SwitchBlockEntry, SwitchBlockStatementGroup, SwitchLabel, SwitchLabelCaseEntry,
    SwitchLabelCaseItem, SwitchRule, SwitchStatement, SynchronizedStatement, ThrowStatement,
    TryStatement, TryWithResourcesStatement, Type, WhileStatement, YieldStatement,
};
use std::ops::Range;

use crate::context::JavaFormatter;
use crate::helpers::blocks::{
    BodyItem, braced_block, braced_body, empty_block, join_body_items, join_hard_lines,
};
use crate::helpers::comments::{
    comment_forces_line, comment_is_star_block, format_comment, format_dangling_comments,
    format_leading_comments, format_removed_token_comments, format_token_text,
    format_token_with_comments, format_trailing_comments,
    format_trailing_comments_before_line_break, non_formatter_control_comments,
    trailing_comments_force_line,
};
use crate::helpers::formatter_ignore::{
    FormatterIgnoreRange, formatter_ignore_ranges, formatter_ignore_run_doc, formatter_ignore_runs,
    relative_token_range,
};
use crate::helpers::lists::semicolon_list;
use crate::rules::annotations::format_annotation;
use crate::rules::declarations::format_type_declaration;
use crate::rules::expressions::format_expression;
use crate::rules::patterns::format_pattern;
use crate::rules::types::format_type;
use crate::rules::variables::format_local_variable_declaration;

pub(crate) fn format_block(block: &Block, formatter: &JavaFormatter<'_>) -> Doc {
    braced_body(format_block_body(block, formatter))
}

pub(crate) fn format_block_body(block: &Block, formatter: &JavaFormatter<'_>) -> Option<Doc> {
    format_block_statements_body(block, formatter)
}

fn format_block_statements_body(block: &Block, formatter: &JavaFormatter<'_>) -> Option<Doc> {
    let statements = block.block_statements().collect::<Vec<_>>();
    let block_start = block.text_range().start().get();
    let ignored_ranges = block_formatter_ignore_ranges(block);
    let mut items = Vec::new();
    items.extend(format_block_open_dangling_comments(block));
    items.extend(format_block_statement_items(
        &statements,
        block_start,
        &ignored_ranges,
        formatter,
    ));
    items.extend(format_block_close_dangling_comments(block));
    (!items.is_empty()).then(|| join_body_items(items))
}

fn format_block_open_dangling_comments(block: &Block) -> Option<BodyItem> {
    let comments = block.open_brace()?.trailing_comments();
    (!comments.is_empty()).then(|| BodyItem::new(format_dangling_comments(comments), false))
}

fn format_block_close_dangling_comments(block: &Block) -> Option<BodyItem> {
    let comments = block.close_brace()?.leading_comments();
    (!comments.is_empty()).then(|| BodyItem::new(format_dangling_comments(comments), false))
}

fn format_block_statement_items(
    statements: &[BlockStatement],
    block_start: usize,
    ignored_ranges: &[FormatterIgnoreRange],
    formatter: &JavaFormatter<'_>,
) -> Vec<BodyItem> {
    let statement_ranges = statements
        .iter()
        .map(|statement| block_statement_token_range(statement, block_start))
        .collect::<Vec<_>>();
    let ignored_runs = formatter_ignore_runs(ignored_ranges, &statement_ranges);

    let mut items = Vec::new();
    let mut ignored_index = 0;
    let mut skip_index = 0;
    for (statement_index, statement) in statements.iter().enumerate() {
        while ignored_index < ignored_runs.len()
            && ignored_runs[ignored_index].insert_index == statement_index
        {
            let run = &ignored_runs[ignored_index];
            items.push(BodyItem::new(formatter_ignore_run_doc(run), false));
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

        if let Some(mut item) = format_block_statement_item(statement, formatter) {
            if skip_index > 0 && ignored_runs[skip_index - 1].skip_end == statement_index {
                item = item.without_blank_line_before();
            }
            items.push(item);
        }
    }

    while ignored_index < ignored_runs.len() {
        let run = &ignored_runs[ignored_index];
        items.push(BodyItem::new(formatter_ignore_run_doc(run), false));
        ignored_index += 1;
    }

    items
}

pub(crate) fn format_block_statement_item(
    statement: &BlockStatement,
    formatter: &JavaFormatter<'_>,
) -> Option<BodyItem> {
    let starts_after_blank_line = statement.starts_after_blank_line();
    let doc = format_block_item_doc(statement.item()?, statement.semicolon(), formatter)?;
    Some(BodyItem::new(doc, starts_after_blank_line))
}

fn format_block_item_doc(
    item: BlockItem,
    semicolon: Option<JavaSyntaxToken>,
    formatter: &JavaFormatter<'_>,
) -> Option<Doc> {
    let doc = match item {
        BlockItem::EmptyStatement(statement) => format_removed_empty_statement(&statement),
        BlockItem::LocalVariableDeclaration(declaration) => Some(concat([
            format_local_variable_declaration(&declaration, formatter),
            format_statement_semicolon(semicolon),
        ])),
        BlockItem::LocalClassOrInterfaceDeclaration(declaration) => declaration
            .declaration()
            .map(|declaration| format_type_declaration(&declaration, formatter)),
        BlockItem::Block(block) => Some(format_block(&block, formatter)),
        BlockItem::LabeledStatement(statement) => {
            Some(format_statement(&statement.into(), formatter))
        }
        BlockItem::ExpressionStatement(statement) => {
            Some(format_statement(&statement.into(), formatter))
        }
        BlockItem::IfStatement(statement) => Some(format_statement(&statement.into(), formatter)),
        BlockItem::AssertStatement(statement) => {
            Some(format_statement(&statement.into(), formatter))
        }
        BlockItem::SwitchStatement(statement) => {
            Some(format_statement(&statement.into(), formatter))
        }
        BlockItem::WhileStatement(statement) => {
            Some(format_statement(&statement.into(), formatter))
        }
        BlockItem::DoStatement(statement) => Some(format_statement(&statement.into(), formatter)),
        BlockItem::ForStatement(statement) => Some(format_statement(&statement.into(), formatter)),
        BlockItem::BreakStatement(statement) => {
            Some(format_statement(&statement.into(), formatter))
        }
        BlockItem::YieldStatement(statement) => {
            Some(format_statement(&statement.into(), formatter))
        }
        BlockItem::ContinueStatement(statement) => {
            Some(format_statement(&statement.into(), formatter))
        }
        BlockItem::ReturnStatement(statement) => {
            Some(format_statement(&statement.into(), formatter))
        }
        BlockItem::ThrowStatement(statement) => {
            Some(format_statement(&statement.into(), formatter))
        }
        BlockItem::SynchronizedStatement(statement) => {
            Some(format_statement(&statement.into(), formatter))
        }
        BlockItem::TryStatement(statement) => Some(format_statement(&statement.into(), formatter)),
        BlockItem::TryWithResourcesStatement(statement) => {
            Some(format_statement(&statement.into(), formatter))
        }
    }?;
    Some(doc)
}

fn format_removed_empty_statement(statement: &jolt_java_syntax::EmptyStatement) -> Option<Doc> {
    format_removed_token_comments(&statement.tokens())
}

fn block_formatter_ignore_ranges(block: &Block) -> Vec<FormatterIgnoreRange> {
    formatter_ignore_ranges(&block.source_text())
}

fn block_statement_token_range(
    statement: &BlockStatement,
    block_start: usize,
) -> Option<Range<usize>> {
    let tokens = statement.tokens();
    relative_token_range(&tokens, block_start)
}

fn format_statement(statement: &Statement, formatter: &JavaFormatter<'_>) -> Doc {
    match statement {
        Statement::Block(block) => format_block(block, formatter),
        Statement::EmptyStatement(_) => empty_block(),
        Statement::LabeledStatement(statement) => format_labeled_statement(statement, formatter),
        Statement::ExpressionStatement(statement) => {
            format_expression_statement(statement, formatter)
        }
        Statement::IfStatement(statement) => format_if_statement(statement, formatter),
        Statement::AssertStatement(statement) => format_assert_statement(statement, formatter),
        Statement::SwitchStatement(statement) => format_switch_statement(statement, formatter),
        Statement::WhileStatement(statement) => format_while_statement(statement, formatter),
        Statement::DoStatement(statement) => format_do_statement(statement, formatter),
        Statement::ForStatement(statement) => format_for_statement(statement, formatter),
        Statement::BreakStatement(statement) => format_jump_statement(
            statement.keyword(),
            "break",
            statement.label(),
            statement.semicolon(),
        ),
        Statement::YieldStatement(statement) => format_yield_statement(statement, formatter),
        Statement::ContinueStatement(statement) => format_jump_statement(
            statement.keyword(),
            "continue",
            statement.label(),
            statement.semicolon(),
        ),
        Statement::ReturnStatement(statement) => format_return_statement(statement, formatter),
        Statement::ThrowStatement(statement) => format_throw_statement(statement, formatter),
        Statement::SynchronizedStatement(statement) => {
            format_synchronized_statement(statement, formatter)
        }
        Statement::TryStatement(statement) => format_try_statement(statement, formatter),
        Statement::TryWithResourcesStatement(statement) => {
            format_try_with_resources_statement(statement, formatter)
        }
    }
}

fn format_labeled_statement(statement: &LabeledStatement, formatter: &JavaFormatter<'_>) -> Doc {
    let label = statement
        .label()
        .map_or_else(jolt_fmt_ir::nil, |label| format_token_with_comments(&label));

    concat([
        label,
        text(":"),
        hard_line(),
        statement
            .body()
            .map_or_else(jolt_fmt_ir::nil, |body| format_statement(&body, formatter)),
    ])
}

fn format_expression_statement(
    statement: &ExpressionStatement,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    concat([
        statement
            .expression()
            .map_or_else(jolt_fmt_ir::nil, |expression| {
                format_expression(&expression, formatter)
            }),
        format_statement_semicolon(statement.semicolon()),
    ])
}

fn format_if_statement(statement: &IfStatement, formatter: &JavaFormatter<'_>) -> Doc {
    let else_body = statement.else_body();
    let then_body = statement.then_body();
    let then_body_trailing_comments_force_line =
        else_body.is_some() && statement_body_trailing_comments_force_line(then_body.as_ref());
    let open = statement.open_paren();
    let close = statement.close_paren();

    concat([
        format_statement_keyword(statement.keyword(), "if"),
        text(" "),
        format_parenthesized_statement_expression(
            open.as_ref(),
            statement
                .condition()
                .map_or_else(jolt_fmt_ir::nil, |condition| {
                    format_expression(&condition, formatter)
                }),
            close.as_ref(),
        ),
        format_statement_header_body_separator(close.as_ref()),
        statement_body_as_block_with_trailing_comments(then_body, formatter),
        else_body.map_or_else(jolt_fmt_ir::nil, |else_body| {
            concat([
                if then_body_trailing_comments_force_line {
                    jolt_fmt_ir::nil()
                } else {
                    text(" ")
                },
                format_statement_keyword(statement.else_keyword(), "else"),
                text(" "),
                match else_body {
                    StatementBody::Unbraced(Statement::IfStatement(else_if)) => {
                        format_if_statement(&else_if, formatter)
                    }
                    body => statement_body_as_block(Some(body), formatter),
                },
            ])
        }),
    ])
}

fn format_parenthesized_statement_expression(
    open: Option<&JavaSyntaxToken>,
    expression: Doc,
    close: Option<&JavaSyntaxToken>,
) -> Doc {
    group(concat([
        format_condition_open_paren(open),
        indent(concat([format_condition_open_spacing(open), expression])),
        format_condition_close_paren(close),
    ]))
}

fn format_condition_open_paren(open: Option<&JavaSyntaxToken>) -> Doc {
    open.map_or_else(
        || text("("),
        |open| concat([format_leading_comments(open), text("(")]),
    )
}

fn format_condition_open_spacing(open: Option<&JavaSyntaxToken>) -> Doc {
    let Some(open) = open else {
        return jolt_fmt_ir::nil();
    };

    if open.trailing_comments().is_empty() {
        return soft_line();
    }

    concat([
        format_trailing_comments_before_line_break(open),
        if trailing_comments_force_line(open) {
            hard_line()
        } else {
            text(" ")
        },
    ])
}

fn format_condition_close_paren(close: Option<&JavaSyntaxToken>) -> Doc {
    let close_has_leading_comments =
        close.is_some_and(|token| !token.leading_comments().is_empty());

    concat([
        if close_has_leading_comments {
            line()
        } else {
            soft_line()
        },
        close.map_or_else(
            || text(")"),
            |close| {
                concat([
                    if close_has_leading_comments {
                        format_leading_comments(close)
                    } else {
                        jolt_fmt_ir::nil()
                    },
                    text(")"),
                    format_trailing_comments_before_line_break(close),
                    if trailing_comments_force_line(close) {
                        hard_line()
                    } else {
                        jolt_fmt_ir::nil()
                    },
                ])
            },
        ),
    ])
}

fn format_statement_header_body_separator(close: Option<&JavaSyntaxToken>) -> Doc {
    if close.is_some_and(trailing_comments_force_line) {
        jolt_fmt_ir::nil()
    } else {
        text(" ")
    }
}

fn format_assert_statement(statement: &AssertStatement, formatter: &JavaFormatter<'_>) -> Doc {
    concat([
        format_statement_keyword(statement.keyword(), "assert"),
        text(" "),
        statement
            .condition()
            .map_or_else(jolt_fmt_ir::nil, |condition| {
                format_expression(&condition, formatter)
            }),
        statement.detail().map_or_else(jolt_fmt_ir::nil, |detail| {
            concat([text(" : "), format_expression(&detail, formatter)])
        }),
        format_statement_semicolon(statement.semicolon()),
    ])
}

fn format_while_statement(statement: &WhileStatement, formatter: &JavaFormatter<'_>) -> Doc {
    let open = statement.open_paren();
    let close = statement.close_paren();
    concat([
        format_statement_keyword(statement.keyword(), "while"),
        text(" "),
        format_parenthesized_statement_expression(
            open.as_ref(),
            statement
                .condition()
                .map_or_else(jolt_fmt_ir::nil, |condition| {
                    format_expression(&condition, formatter)
                }),
            close.as_ref(),
        ),
        format_statement_header_body_separator(close.as_ref()),
        statement_body_as_block(statement.statement_body(), formatter),
    ])
}

fn format_do_statement(statement: &DoStatement, formatter: &JavaFormatter<'_>) -> Doc {
    let open = statement.open_paren();
    let close = statement.close_paren();
    concat([
        format_statement_keyword(statement.keyword(), "do"),
        text(" "),
        statement_body_as_block(statement.statement_body(), formatter),
        text(" "),
        format_statement_keyword(statement.while_keyword(), "while"),
        text(" "),
        format_parenthesized_statement_expression(
            open.as_ref(),
            statement
                .condition()
                .map_or_else(jolt_fmt_ir::nil, |condition| {
                    format_expression(&condition, formatter)
                }),
            close.as_ref(),
        ),
        format_statement_semicolon(statement.semicolon()),
    ])
}

fn format_for_statement(statement: &ForStatement, formatter: &JavaFormatter<'_>) -> Doc {
    if let Some(basic) = statement.basic() {
        return format_basic_for_statement(&basic, formatter);
    }
    if let Some(enhanced) = statement.enhanced() {
        return format_enhanced_for_statement(&enhanced, formatter);
    }

    jolt_fmt_ir::nil()
}

fn format_basic_for_statement(statement: &BasicForStatement, formatter: &JavaFormatter<'_>) -> Doc {
    let open = statement.open_paren();
    let close = statement.close_paren();
    let initializer = statement
        .initializer()
        .map(|initializer| format_for_initializer(&initializer, formatter));
    let condition = statement
        .condition()
        .map(|condition| format_expression(&condition, formatter));
    let update = statement
        .update()
        .map(|update| format_for_update(&update, formatter));
    let is_empty_header = initializer.is_none() && condition.is_none() && update.is_none();
    let header = if is_empty_header {
        concat([
            format_statement_keyword(statement.keyword(), "for"),
            text(" "),
            format_condition_open_paren(open.as_ref()),
            format_condition_open_spacing(open.as_ref()),
            text(";;"),
            format_inline_close_paren(close.as_ref()),
        ])
    } else {
        group(concat([
            format_statement_keyword(statement.keyword(), "for"),
            text(" "),
            format_condition_open_paren(open.as_ref()),
            indent(concat([
                format_for_header_open_spacing(open.as_ref()),
                semicolon_list(vec![
                    initializer.unwrap_or_else(jolt_fmt_ir::nil),
                    condition.unwrap_or_else(jolt_fmt_ir::nil),
                    update.unwrap_or_else(jolt_fmt_ir::nil),
                ]),
            ])),
            format_for_header_close_paren(close.as_ref()),
        ]))
    };

    concat([
        header,
        format_statement_header_body_separator(close.as_ref()),
        statement_body_as_block(statement.statement_body(), formatter),
    ])
}

fn format_enhanced_for_statement(
    statement: &EnhancedForStatement,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    let open = statement.open_paren();
    let close = statement.close_paren();
    concat([
        format_statement_keyword(statement.keyword(), "for"),
        text(" "),
        group(concat([
            format_condition_open_paren(open.as_ref()),
            indent(concat([
                format_for_header_open_spacing(open.as_ref()),
                statement
                    .variable()
                    .map_or_else(jolt_fmt_ir::nil, |variable| {
                        format_local_variable_declaration(&variable, formatter)
                    }),
                text(" : "),
                statement
                    .iterable()
                    .map_or_else(jolt_fmt_ir::nil, |iterable| {
                        format_expression(&iterable, formatter)
                    }),
            ])),
            format_for_header_close_paren(close.as_ref()),
        ])),
        format_statement_header_body_separator(close.as_ref()),
        statement_body_as_block(statement.statement_body(), formatter),
    ])
}

fn format_for_header_open_spacing(open: Option<&JavaSyntaxToken>) -> Doc {
    if open.is_some_and(|open| !open.trailing_comments().is_empty()) {
        format_condition_open_spacing(open)
    } else {
        soft_line()
    }
}

fn format_inline_close_paren(close: Option<&JavaSyntaxToken>) -> Doc {
    let close_has_leading_comments =
        close.is_some_and(|token| !token.leading_comments().is_empty());

    concat([
        if close_has_leading_comments {
            line()
        } else {
            jolt_fmt_ir::nil()
        },
        close.map_or_else(
            || text(")"),
            |close| {
                concat([
                    if close_has_leading_comments {
                        format_leading_comments(close)
                    } else {
                        jolt_fmt_ir::nil()
                    },
                    text(")"),
                    format_trailing_comments_before_line_break(close),
                    if trailing_comments_force_line(close) {
                        hard_line()
                    } else {
                        jolt_fmt_ir::nil()
                    },
                ])
            },
        ),
    ])
}

fn format_for_header_close_paren(close: Option<&JavaSyntaxToken>) -> Doc {
    let close_has_leading_comments =
        close.is_some_and(|token| !token.leading_comments().is_empty());

    concat([
        if close_has_leading_comments {
            line()
        } else {
            soft_line()
        },
        close.map_or_else(
            || text(")"),
            |close| {
                concat([
                    if close_has_leading_comments {
                        format_leading_comments(close)
                    } else {
                        jolt_fmt_ir::nil()
                    },
                    text(")"),
                    format_trailing_comments_before_line_break(close),
                    if trailing_comments_force_line(close) {
                        hard_line()
                    } else {
                        jolt_fmt_ir::nil()
                    },
                ])
            },
        ),
    ])
}

fn format_for_initializer(initializer: &ForInitializer, formatter: &JavaFormatter<'_>) -> Doc {
    if let Some(declaration) = initializer.local_variable_declaration() {
        return format_local_variable_declaration(&declaration, formatter);
    }
    initializer
        .expressions()
        .map_or_else(jolt_fmt_ir::nil, |expressions| {
            format_statement_expression_list(&expressions, formatter)
        })
}

fn format_for_update(update: &ForUpdate, formatter: &JavaFormatter<'_>) -> Doc {
    update
        .expressions()
        .map_or_else(jolt_fmt_ir::nil, |expressions| {
            format_statement_expression_list(&expressions, formatter)
        })
}

fn format_statement_expression_list(
    expressions: &StatementExpressionList,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    format_statement_expression_entries(expressions.entries().collect(), formatter)
}

fn format_statement_expression_entries(
    entries: Vec<StatementExpressionEntry>,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    let mut docs = Vec::new();
    let entries_len = entries.len();

    for (index, entry) in entries.into_iter().enumerate() {
        docs.push(format_expression(&entry.expression, formatter));
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

fn format_return_statement(statement: &ReturnStatement, formatter: &JavaFormatter<'_>) -> Doc {
    format_keyword_expression_statement(
        statement.keyword().as_ref(),
        "return",
        statement.expression(),
        statement.semicolon(),
        formatter,
    )
}

fn format_throw_statement(statement: &ThrowStatement, formatter: &JavaFormatter<'_>) -> Doc {
    format_keyword_expression_statement(
        statement.keyword().as_ref(),
        "throw",
        statement.expression(),
        statement.semicolon(),
        formatter,
    )
}

fn format_yield_statement(statement: &YieldStatement, formatter: &JavaFormatter<'_>) -> Doc {
    format_keyword_expression_statement(
        statement.keyword().as_ref(),
        "yield",
        statement.expression(),
        statement.semicolon(),
        formatter,
    )
}

fn format_keyword_expression_statement(
    keyword: Option<&JavaSyntaxToken>,
    fallback: &'static str,
    expression: Option<Expression>,
    semicolon: Option<JavaSyntaxToken>,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    concat([
        format_statement_keyword_head(keyword, fallback),
        expression.map_or_else(jolt_fmt_ir::nil, |expression| {
            let expression_doc = concat([
                format_keyword_expression_separator(keyword),
                format_expression(&expression, formatter),
            ]);
            if matches!(expression, Expression::SwitchExpression(_)) {
                expression_doc
            } else {
                indent(expression_doc)
            }
        }),
        format_statement_semicolon(semicolon),
    ])
}

fn format_keyword_expression_separator(keyword: Option<&JavaSyntaxToken>) -> Doc {
    let Some(keyword) = keyword else {
        return text(" ");
    };

    if keyword.trailing_comments().is_empty() {
        return text(" ");
    }

    concat([
        format_trailing_comments_before_line_break(keyword),
        if trailing_comments_force_line(keyword) {
            hard_line()
        } else {
            text(" ")
        },
    ])
}

fn format_jump_statement(
    keyword: Option<JavaSyntaxToken>,
    fallback: &'static str,
    label: Option<JavaSyntaxToken>,
    semicolon: Option<JavaSyntaxToken>,
) -> Doc {
    concat([
        format_statement_keyword(keyword, fallback),
        label.map_or_else(jolt_fmt_ir::nil, |label| {
            concat([text(" "), format_token_with_comments(&label)])
        }),
        format_statement_semicolon(semicolon),
    ])
}

fn format_synchronized_statement(
    statement: &SynchronizedStatement,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    let open = statement.open_paren();
    let close = statement.close_paren();
    concat([
        format_statement_keyword(statement.keyword(), "synchronized"),
        text(" "),
        format_parenthesized_statement_expression(
            open.as_ref(),
            statement
                .expression()
                .map_or_else(jolt_fmt_ir::nil, |expression| {
                    format_expression(&expression, formatter)
                }),
            close.as_ref(),
        ),
        format_statement_header_body_separator(close.as_ref()),
        statement
            .body()
            .map_or_else(empty_block, |body| format_block(&body, formatter)),
    ])
}

fn format_switch_statement(statement: &SwitchStatement, formatter: &JavaFormatter<'_>) -> Doc {
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
    let items = group
        .block_statements()
        .filter_map(|statement| format_block_statement_item(&statement, formatter))
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

pub(crate) fn format_statement_semicolon(semicolon: Option<JavaSyntaxToken>) -> Doc {
    let Some(semicolon) = semicolon else {
        return text(";");
    };

    concat([
        format_semicolon_leading_comments(&semicolon),
        text(";"),
        format_terminator_trailing_comments(&semicolon),
    ])
}

fn format_semicolon_leading_comments(semicolon: &JavaSyntaxToken) -> Doc {
    let mut docs = Vec::new();
    for comment in semicolon.leading_comments() {
        docs.push(text(" "));
        docs.push(format_comment(&comment));
        if comment_forces_line(&comment) {
            docs.push(hard_line());
        }
    }
    concat(docs)
}

fn format_terminator_trailing_comments(token: &JavaSyntaxToken) -> Doc {
    let mut docs = Vec::new();
    for comment in token.trailing_comments() {
        if terminator_comment_starts_next_line(&comment) {
            docs.push(hard_line());
        } else {
            docs.push(text(" "));
        }
        docs.push(format_comment(&comment));
    }
    concat(docs)
}

fn terminator_comment_starts_next_line(comment: &JavaComment) -> bool {
    comment_is_star_block(comment)
}

fn format_try_statement(statement: &TryStatement, formatter: &JavaFormatter<'_>) -> Doc {
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

fn format_try_with_resources_statement(
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

fn statement_body_as_block(body: Option<StatementBody>, formatter: &JavaFormatter<'_>) -> Doc {
    match body {
        Some(StatementBody::Block(block)) => format_block(&block, formatter),
        Some(StatementBody::Empty(_)) | None => empty_block(),
        Some(StatementBody::Unbraced(statement)) => {
            braced_body(Some(format_statement(&statement, formatter)))
        }
    }
}

fn statement_body_as_block_with_trailing_comments(
    body: Option<StatementBody>,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    match body {
        Some(StatementBody::Block(block)) => concat([
            format_block(&block, formatter),
            block
                .close_brace()
                .map_or_else(jolt_fmt_ir::nil, |close| format_trailing_comments(&close)),
        ]),
        body => statement_body_as_block(body, formatter),
    }
}

fn statement_body_trailing_comments_force_line(body: Option<&StatementBody>) -> bool {
    let Some(StatementBody::Block(block)) = body else {
        return false;
    };
    block
        .close_brace()
        .is_some_and(|close| trailing_comments_force_line(&close))
}

fn format_statement_keyword(keyword: Option<JavaSyntaxToken>, fallback: &'static str) -> Doc {
    keyword.map_or_else(
        || text(fallback),
        |keyword| format_token_with_comments(&keyword),
    )
}

fn format_statement_keyword_head(keyword: Option<&JavaSyntaxToken>, fallback: &'static str) -> Doc {
    keyword.map_or_else(
        || text(fallback),
        |keyword| {
            concat([
                format_leading_comments(keyword),
                format_token_text(keyword.text()),
            ])
        },
    )
}
