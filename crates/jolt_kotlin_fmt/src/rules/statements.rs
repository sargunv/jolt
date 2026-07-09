use jolt_fmt_ir::{Doc, concat};
use jolt_kotlin_syntax::{
    BlockItem, Declaration, Expression, ExpressionStatement, KotlinSyntaxToken,
    RecoveredSeparatedListEntry, Statement, StatementSyntax,
};

mod blocks;

use crate::helpers::comments::{LeadingTrivia, format_token_gap, format_token_sequence};
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
        StatementSyntax::Statement(statement) => format_statement_node(statement, leading),
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

fn format_statement_node<'source>(
    statement: &Statement<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let Some(inner) = statement.statement() else {
        return format_token_sequence(statement.token_iter(), leading);
    };
    let doc = format_statement_owned(&inner, leading);
    let tail = statement.tail_tokens_after_statement().collect::<Vec<_>>();
    if tail.is_empty() {
        return doc;
    }

    concat([doc, format_token_sequence(tail, LeadingTrivia::Preserve)])
}

fn format_expression_statement<'source>(
    statement: &ExpressionStatement<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let mut docs = Vec::new();
    let mut has_output = false;
    let mut previous_last_token = None;

    for entry in statement.entries_with_recovered() {
        if let (Some(left), Some(right)) = (previous_last_token.as_ref(), entry_first_token(&entry))
        {
            docs.push(format_token_gap(left, &right));
        }
        let entry_leading = if has_output {
            LeadingTrivia::Preserve
        } else {
            leading
        };
        match &entry {
            RecoveredSeparatedListEntry::Entry(expression) => {
                docs.push(match entry_leading {
                    LeadingTrivia::Preserve => format_expression(expression),
                    LeadingTrivia::SuppressAlreadyHandled => {
                        format_expression_without_leading(expression)
                    }
                });
            }
            RecoveredSeparatedListEntry::Token(token) => {
                docs.push(format_token_sequence(
                    std::iter::once(*token),
                    entry_leading,
                ));
            }
            RecoveredSeparatedListEntry::Error(error) => {
                docs.push(format_token_sequence(error.token_iter(), entry_leading));
            }
            RecoveredSeparatedListEntry::Node(node) => {
                docs.push(format_token_sequence(node.token_iter(), entry_leading));
            }
        }
        previous_last_token = entry_last_token(&entry);
        has_output = true;
    }

    if docs.is_empty() {
        return format_token_sequence(statement.token_iter(), leading);
    }

    concat(docs)
}

fn entry_first_token<'source>(
    entry: &RecoveredSeparatedListEntry<'source, Expression<'source>>,
) -> Option<KotlinSyntaxToken<'source>> {
    match entry {
        RecoveredSeparatedListEntry::Entry(expression) => expression.first_token(),
        RecoveredSeparatedListEntry::Token(token) => Some(*token),
        RecoveredSeparatedListEntry::Error(error) => error.first_token(),
        RecoveredSeparatedListEntry::Node(node) => node.first_token(),
    }
}

fn entry_last_token<'source>(
    entry: &RecoveredSeparatedListEntry<'source, Expression<'source>>,
) -> Option<KotlinSyntaxToken<'source>> {
    match entry {
        RecoveredSeparatedListEntry::Entry(expression) => expression.last_token(),
        RecoveredSeparatedListEntry::Token(token) => Some(*token),
        RecoveredSeparatedListEntry::Error(error) => error.last_token(),
        RecoveredSeparatedListEntry::Node(node) => node.last_token(),
    }
}
