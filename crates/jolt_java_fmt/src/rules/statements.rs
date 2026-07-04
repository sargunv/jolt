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
    BodyItem, braced_body, empty_block, join_body_items, join_hard_lines,
};
use crate::helpers::comments::{
    LeadingTrivia, TrailingTrivia, comment_forces_line, comment_is_star_block,
    comments_from_tokens, format_comment, format_dangling_comments, format_leading_comments,
    format_removed_comments, format_separator_with_comments, format_token,
    format_token_before_relocated_trailing_comments, format_token_with_comments,
    format_trailing_comments_before_line_break, trailing_comments_force_line,
};
use crate::helpers::formatter_ignore::{
    FormatterIgnoreRange, formatter_ignore_ranges, formatter_ignore_run_doc, formatter_ignore_runs,
    relative_token_range_between,
};
use crate::helpers::lists::semicolon_list;
use crate::rules::annotations::format_annotation;
use crate::rules::declarations::format_type_declaration;
use crate::rules::expressions::format_expression;
use crate::rules::patterns::format_pattern;
use crate::rules::types::format_type;
use crate::rules::variables::format_local_variable_declaration;

mod blocks;
mod control_flow;
mod simple;
mod switches;
mod try_resources;

pub(crate) use blocks::{format_block, format_block_statement_item};
use control_flow::{
    format_do_statement, format_for_statement, format_if_statement, format_synchronized_statement,
    format_while_statement,
};
pub(crate) use simple::format_statement_semicolon;
use simple::{
    format_assert_statement, format_expression_statement, format_jump_statement,
    format_labeled_statement, format_return_statement, format_throw_statement,
    format_yield_statement,
};
pub(crate) use switches::format_switch_block;
use switches::format_switch_statement;
use try_resources::{format_try_statement, format_try_with_resources_statement};

fn format_statement<'source>(
    statement: &Statement<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
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

fn statement_body_as_block<'source>(
    body: Option<&StatementBody<'source>>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    match body {
        Some(StatementBody::Block(block)) => format_block(block, formatter),
        Some(StatementBody::Empty(_)) | None => empty_block(),
        Some(StatementBody::Unbraced(statement)) => {
            braced_body(Some(format_statement(statement, formatter)))
        }
    }
}

fn statement_body_trailing_comments_force_line(body: Option<&StatementBody<'_>>) -> bool {
    let Some(StatementBody::Block(block)) = body else {
        return false;
    };
    block
        .close_brace()
        .is_some_and(|close| trailing_comments_force_line(&close))
}
