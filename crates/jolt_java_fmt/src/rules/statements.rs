use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_java_syntax::{
    AssertStatement, BasicForStatement, Block, BlockItem, BlockStatement, CatchClause,
    CatchParameter, DoStatement, EnhancedForStatement, ExpressionStatement, FinallyClause,
    ForStatement, IfStatement, JavaSyntaxToken, LabeledStatement, Resource, ResourceList,
    ReturnStatement, Statement, SwitchBlock, SwitchBlockStatementGroup, SwitchLabel, SwitchRule,
    SwitchStatement, SynchronizedStatement, ThrowStatement, TryStatement,
    TryWithResourcesStatement, WhileStatement, YieldStatement,
};
use std::ops::Range;

use crate::helpers::blocks::{BodyItem, inserted_braced_body, join_body_items};
use crate::helpers::comments::{
    LeadingTrivia, TrailingTrivia, comment_forces_line, comments_from_tokens,
    format_dangling_comments, format_removed_comments, format_separator_with_comments,
    format_token, format_token_before_relocated_trailing_comments, format_token_with_comments,
    format_trailing_comments_before_line_break, trailing_comments_force_line,
};
use crate::helpers::formatter_ignore::{
    FormatterIgnoreRange, FormatterIgnoreSplice, for_each_formatter_ignore_splice,
    formatter_ignore_ranges, formatter_ignore_run_doc, formatter_ignore_runs,
    relative_token_range_between,
};
use crate::helpers::recovery::{JavaFormatField, format_malformed, resolve_required_field};
use crate::rules::annotations::format_annotation;
use crate::rules::declarations::format_type_declaration;
use crate::rules::expressions::format_expression;
use crate::rules::patterns::format_pattern;
use crate::rules::types::format_type;
use crate::rules::variables::{
    format_enhanced_for_variable, format_local_variable_declaration,
    format_resource_variable_declaration,
};

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
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    match statement {
        Statement::Block(block) => format_block(block, doc),
        Statement::EmptyStatement(statement) => format_empty_statement(statement, doc),
        Statement::LabeledStatement(statement) => format_labeled_statement(statement, doc),
        Statement::ExpressionStatement(statement) => format_expression_statement(statement, doc),
        Statement::IfStatement(statement) => format_if_statement(statement, doc),
        Statement::AssertStatement(statement) => format_assert_statement(statement, doc),
        Statement::SwitchStatement(statement) => format_switch_statement(statement, doc),
        Statement::WhileStatement(statement) => format_while_statement(statement, doc),
        Statement::DoStatement(statement) => format_do_statement(statement, doc),
        Statement::ForStatement(statement) => format_for_statement(statement, doc),
        Statement::BreakStatement(statement) => format_jump_statement(
            statement.break_keyword(),
            statement.label(),
            statement.semicolon(),
            doc,
        ),
        Statement::YieldStatement(statement) => format_yield_statement(statement, doc),
        Statement::ContinueStatement(statement) => format_jump_statement(
            statement.continue_keyword(),
            statement.label(),
            statement.semicolon(),
            doc,
        ),
        Statement::ReturnStatement(statement) => format_return_statement(statement, doc),
        Statement::ThrowStatement(statement) => format_throw_statement(statement, doc),
        Statement::SynchronizedStatement(statement) => {
            format_synchronized_statement(statement, doc)
        }
        Statement::TryStatement(statement) => format_try_statement(statement, doc),
        Statement::TryWithResourcesStatement(statement) => {
            format_try_with_resources_statement(statement, doc)
        }
        Statement::BogusStatement(statement) => format_malformed(statement, doc),
    }
}

fn statement_body_as_block<'source>(
    body: Result<
        jolt_java_syntax::JavaSyntaxField<'source, Statement<'source>>,
        jolt_java_syntax::JavaSyntaxInvariantError,
    >,
    normalization: Option<jolt_java_syntax::JavaControlBodyNormalization<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    match resolve_required_field(body, doc) {
        JavaFormatField::Present(Statement::Block(block)) => format_block(&block, doc),
        JavaFormatField::Present(Statement::EmptyStatement(statement)) => match normalization {
            Some(normalization) => {
                let body = format_empty_statement_comments(&statement, doc);
                let removed = normalization
                    .empty_separator
                    .map_or_else(Doc::nil, |claim| doc.removed_source(claim));
                let block = inserted_braced_body(doc, body, normalization.braces);
                doc_concat!(doc, [removed, block])
            }
            None => format_empty_statement(&statement, doc),
        },
        JavaFormatField::Present(statement) => {
            let body = format_statement(&statement, doc);
            normalization.map_or_else(
                || body,
                |normalization| inserted_braced_body(doc, Some(body), normalization.braces),
            )
        }
        JavaFormatField::Malformed(recovery) => recovery,
    }
}

fn format_empty_statement<'source>(
    statement: &jolt_java_syntax::EmptyStatement<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    format_statement_semicolon(statement.semicolon(), doc)
}

fn format_empty_statement_comments<'source>(
    statement: &jolt_java_syntax::EmptyStatement<'source>,
    doc: &mut DocBuilder<'source>,
) -> Option<Doc<'source>> {
    format_removed_comments(doc, comments_from_tokens(statement.token_iter()))
}

fn statement_body_trailing_comments_force_line(body: &Statement<'_>) -> bool {
    let Statement::Block(block) = body else {
        return false;
    };
    matches!(block.close_brace(), Ok(jolt_java_syntax::JavaSyntaxField::Present(close)) if trailing_comments_force_line(&close))
}
