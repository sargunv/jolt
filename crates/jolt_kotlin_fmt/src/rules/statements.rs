use jolt_fmt_ir::{Doc, concat};
use jolt_kotlin_syntax::{
    BlockItem, Declaration, Expression, ExpressionStatement, KotlinSyntaxToken, StatementSyntax,
};

mod blocks;

use crate::helpers::comments::{LeadingTrivia, format_token_sequence};
use crate::helpers::source::source_gap_is_trivia;
use crate::rules::expressions::{format_expression, format_expression_without_leading};
pub(crate) use blocks::format_block;

pub(crate) fn format_block_item<'source>(item: &BlockItem<'source>) -> Doc<'source> {
    match item {
        BlockItem::ClassDeclaration(declaration) => format_declaration_item(
            jolt_kotlin_syntax::Declaration::ClassDeclaration(*declaration),
        ),
        BlockItem::InterfaceDeclaration(declaration) => format_declaration_item(
            jolt_kotlin_syntax::Declaration::InterfaceDeclaration(*declaration),
        ),
        BlockItem::ObjectDeclaration(declaration) => format_declaration_item(
            jolt_kotlin_syntax::Declaration::ObjectDeclaration(*declaration),
        ),
        BlockItem::FunctionDeclaration(declaration) => format_declaration_item(
            jolt_kotlin_syntax::Declaration::FunctionDeclaration(*declaration),
        ),
        BlockItem::PropertyDeclaration(declaration) => format_declaration_item(
            jolt_kotlin_syntax::Declaration::PropertyDeclaration(*declaration),
        ),
        BlockItem::TypeAliasDeclaration(declaration) => format_declaration_item(
            jolt_kotlin_syntax::Declaration::TypeAliasDeclaration(*declaration),
        ),
        BlockItem::SecondaryConstructor(constructor) => format_declaration_item(
            jolt_kotlin_syntax::Declaration::SecondaryConstructor(*constructor),
        ),
        BlockItem::InitializerBlock(block) => {
            format_declaration_item(jolt_kotlin_syntax::Declaration::InitializerBlock(*block))
        }
        BlockItem::Statement(statement) => {
            format_statement_syntax(&StatementSyntax::Statement(*statement))
        }
        BlockItem::ExpressionStatement(statement) => {
            format_statement_syntax(&StatementSyntax::ExpressionStatement(*statement))
        }
        BlockItem::LocalDeclaration(declaration) => {
            format_statement_syntax(&StatementSyntax::LocalDeclaration(*declaration))
        }
        BlockItem::Block(block) => format_block(block),
    }
}

fn format_declaration_item(declaration: jolt_kotlin_syntax::Declaration<'_>) -> Doc<'_> {
    crate::rules::declarations::format_declaration(&declaration)
}

pub(crate) fn format_statement_syntax<'source>(
    statement: &StatementSyntax<'source>,
) -> Doc<'source> {
    format_statement_owned(statement, LeadingTrivia::SuppressAlreadyHandled)
}

pub(crate) fn format_statement_syntax_with_leading<'source>(
    statement: &StatementSyntax<'source>,
) -> Doc<'source> {
    format_statement_owned(statement, LeadingTrivia::Preserve)
}

fn format_statement_owned<'source>(
    statement: &StatementSyntax<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    match statement {
        StatementSyntax::Statement(statement) => statement.statement().map_or_else(
            || format_token_sequence(statement.token_iter(), leading),
            |statement| format_statement_owned(&statement, leading),
        ),
        StatementSyntax::ExpressionStatement(statement) => {
            format_expression_statement(statement, leading)
        }
        StatementSyntax::LocalDeclaration(declaration) => {
            declaration.property_declaration().map_or_else(
                || format_token_sequence(declaration.token_iter(), leading),
                |declaration| {
                    crate::rules::declarations::format_declaration(
                        &Declaration::PropertyDeclaration(declaration),
                    )
                },
            )
        }
        StatementSyntax::Block(block) => format_block(block),
    }
}

fn format_expression_statement<'source>(
    statement: &ExpressionStatement<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let Some(expression) = statement.expression() else {
        return format_token_sequence(statement.token_iter(), leading);
    };
    if expression_statement_has_recovered_tokens(statement, &expression) {
        return format_expression_statement_with_recovered_tokens(statement, &expression, leading);
    }

    match leading {
        LeadingTrivia::Preserve => format_expression(&expression),
        LeadingTrivia::SuppressAlreadyHandled => format_expression_without_leading(&expression),
    }
}

fn expression_statement_has_recovered_tokens(
    statement: &ExpressionStatement<'_>,
    expression: &Expression<'_>,
) -> bool {
    let source = statement.source_text();
    let statement_start = statement.text_range().start().get();
    let expression_range = expression.text_range();
    !source_gap_is_trivia(
        source,
        statement_start,
        statement.token_iter(),
        statement.text_range().start().get(),
        expression_range.start().get(),
    ) || !source_gap_is_trivia(
        source,
        statement_start,
        statement.token_iter(),
        expression_range.end().get(),
        statement.text_range().end().get(),
    )
}

fn format_expression_statement_with_recovered_tokens<'source>(
    statement: &ExpressionStatement<'source>,
    expression: &Expression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let source = statement.source_text();
    let statement_start = statement.text_range().start().get();
    let tokens = statement.token_iter().collect::<Vec<_>>();
    let mut token_cursor = 0;
    let mut docs = Vec::new();

    push_recovered_statement_gap(
        &mut docs,
        source,
        statement_start,
        &tokens,
        &mut token_cursor,
        statement.text_range().start().get(),
        expression.text_range().start().get(),
        leading,
    );

    docs.push(match leading {
        LeadingTrivia::Preserve => format_expression(expression),
        LeadingTrivia::SuppressAlreadyHandled => format_expression_without_leading(expression),
    });

    push_recovered_statement_gap(
        &mut docs,
        source,
        statement_start,
        &tokens,
        &mut token_cursor,
        expression.text_range().end().get(),
        statement.text_range().end().get(),
        LeadingTrivia::Preserve,
    );

    concat(docs)
}

fn push_recovered_statement_gap<'source>(
    docs: &mut Vec<Doc<'source>>,
    source: &'source str,
    source_start: usize,
    tokens: &[KotlinSyntaxToken<'source>],
    token_cursor: &mut usize,
    start: usize,
    end: usize,
    leading: LeadingTrivia,
) {
    if source_gap_is_trivia(source, source_start, tokens.iter().copied(), start, end) {
        return;
    }

    let mut gap_tokens = Vec::new();
    while *token_cursor < tokens.len() {
        let range = tokens[*token_cursor].token_text_range();
        if range.end().get() <= start {
            *token_cursor += 1;
            continue;
        }
        if range.start().get() >= end {
            break;
        }
        if range.start().get() >= start && range.end().get() <= end {
            gap_tokens.push(tokens[*token_cursor]);
            *token_cursor += 1;
            continue;
        }
        break;
    }

    if !gap_tokens.is_empty() {
        docs.push(format_token_sequence(gap_tokens, leading));
    }
}
