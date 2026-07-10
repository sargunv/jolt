use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_kotlin_syntax::{
    BlockItem, Declaration, Expression, ExpressionStatement, KotlinSyntaxToken,
    RecoveredSeparatedListEntry, Statement, StatementSyntax,
};

mod blocks;

use crate::helpers::comments::{LeadingTrivia, format_token_gap, format_token_sequence};
use crate::rules::expressions::{format_expression, format_expression_without_leading};
pub(crate) use blocks::format_block;

pub(crate) fn format_block_item<'source>(
    doc: &mut DocBuilder<'source>,
    item: &BlockItem<'source>,
) -> Doc<'source> {
    match item {
        BlockItem::ClassDeclaration(declaration) => format_declaration_item(
            doc,
            jolt_kotlin_syntax::Declaration::ClassDeclaration(*declaration),
        ),
        BlockItem::InterfaceDeclaration(declaration) => format_declaration_item(
            doc,
            jolt_kotlin_syntax::Declaration::InterfaceDeclaration(*declaration),
        ),
        BlockItem::ObjectDeclaration(declaration) => format_declaration_item(
            doc,
            jolt_kotlin_syntax::Declaration::ObjectDeclaration(*declaration),
        ),
        BlockItem::FunctionDeclaration(declaration) => format_declaration_item(
            doc,
            jolt_kotlin_syntax::Declaration::FunctionDeclaration(*declaration),
        ),
        BlockItem::PropertyDeclaration(declaration) => format_declaration_item(
            doc,
            jolt_kotlin_syntax::Declaration::PropertyDeclaration(*declaration),
        ),
        BlockItem::TypeAliasDeclaration(declaration) => format_declaration_item(
            doc,
            jolt_kotlin_syntax::Declaration::TypeAliasDeclaration(*declaration),
        ),
        BlockItem::SecondaryConstructor(constructor) => format_declaration_item(
            doc,
            jolt_kotlin_syntax::Declaration::SecondaryConstructor(*constructor),
        ),
        BlockItem::InitializerBlock(block) => format_declaration_item(
            doc,
            jolt_kotlin_syntax::Declaration::InitializerBlock(*block),
        ),
        BlockItem::Statement(statement) => {
            format_statement_syntax(doc, &StatementSyntax::Statement(*statement))
        }
        BlockItem::ExpressionStatement(statement) => {
            format_statement_syntax(doc, &StatementSyntax::ExpressionStatement(*statement))
        }
        BlockItem::LocalDeclaration(declaration) => {
            format_statement_syntax(doc, &StatementSyntax::LocalDeclaration(*declaration))
        }
        BlockItem::Block(block) => format_block(doc, block),
    }
}

fn format_declaration_item<'source>(
    doc: &mut DocBuilder<'source>,
    declaration: jolt_kotlin_syntax::Declaration<'source>,
) -> Doc<'source> {
    crate::rules::declarations::format_declaration(doc, &declaration)
}

pub(crate) fn format_statement_syntax<'source>(
    doc: &mut DocBuilder<'source>,
    statement: &StatementSyntax<'source>,
) -> Doc<'source> {
    format_statement_owned(doc, statement, LeadingTrivia::SuppressAlreadyHandled)
}

pub(crate) fn format_statement_syntax_with_leading<'source>(
    doc: &mut DocBuilder<'source>,
    statement: &StatementSyntax<'source>,
) -> Doc<'source> {
    format_statement_owned(doc, statement, LeadingTrivia::Preserve)
}

fn format_statement_owned<'source>(
    doc: &mut DocBuilder<'source>,
    statement: &StatementSyntax<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    match statement {
        StatementSyntax::Statement(statement) => format_statement_node(doc, statement, leading),
        StatementSyntax::ExpressionStatement(statement) => {
            format_expression_statement(doc, statement, leading)
        }
        StatementSyntax::LocalDeclaration(declaration) => {
            if let Some(declaration) = declaration.property_declaration() {
                crate::rules::declarations::format_declaration(
                    doc,
                    &Declaration::PropertyDeclaration(declaration),
                )
            } else {
                format_token_sequence(doc, declaration.token_iter(), leading)
            }
        }
        StatementSyntax::Block(block) => format_block(doc, block),
    }
}

fn format_statement_node<'source>(
    doc: &mut DocBuilder<'source>,
    statement: &Statement<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let Some(inner) = statement.statement() else {
        return format_token_sequence(doc, statement.token_iter(), leading);
    };
    let statement_doc = format_statement_owned(doc, &inner, leading);
    let mut tail = statement.tail_tokens_after_statement().peekable();
    if tail.peek().is_none() {
        return statement_doc;
    }

    let tail = format_token_sequence(doc, tail, LeadingTrivia::Preserve);
    doc.concat([statement_doc, tail])
}

fn format_expression_statement<'source>(
    doc: &mut DocBuilder<'source>,
    statement: &ExpressionStatement<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let entries = statement.entries_with_recovered();
    let mut has_output = false;
    let mut previous_last_token = None;
    let mut is_empty = true;
    let result = doc.concat_list(|docs| {
        for entry in entries {
            if let (Some(left), Some(right)) = (
                previous_last_token.as_ref(),
                entry_first_token(docs, &entry),
            ) {
                let gap = format_token_gap(docs, left, &right);
                docs.push(gap);
            }
            let entry_leading = if has_output {
                LeadingTrivia::Preserve
            } else {
                leading
            };
            match &entry {
                RecoveredSeparatedListEntry::Entry(expression) => {
                    let expression = match entry_leading {
                        LeadingTrivia::Preserve => format_expression(docs, expression),
                        LeadingTrivia::SuppressAlreadyHandled => {
                            format_expression_without_leading(docs, expression)
                        }
                    };
                    docs.push(expression);
                }
                RecoveredSeparatedListEntry::Token(token) => {
                    let token = format_token_sequence(docs, std::iter::once(*token), entry_leading);
                    docs.push(token);
                }
                RecoveredSeparatedListEntry::Error(error) => {
                    let error = format_token_sequence(docs, error.token_iter(), entry_leading);
                    docs.push(error);
                }
                RecoveredSeparatedListEntry::Node(node) => {
                    let node = format_token_sequence(docs, node.token_iter(), entry_leading);
                    docs.push(node);
                }
            }
            previous_last_token = entry_last_token(docs, &entry);
            has_output = true;
        }
        is_empty = docs.is_empty();
    });

    if is_empty {
        format_token_sequence(doc, statement.token_iter(), leading)
    } else {
        result
    }
}

fn entry_first_token<'source>(
    _doc: &mut DocBuilder<'source>,
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
    _doc: &mut DocBuilder<'source>,
    entry: &RecoveredSeparatedListEntry<'source, Expression<'source>>,
) -> Option<KotlinSyntaxToken<'source>> {
    match entry {
        RecoveredSeparatedListEntry::Entry(expression) => expression.last_token(),
        RecoveredSeparatedListEntry::Token(token) => Some(*token),
        RecoveredSeparatedListEntry::Error(error) => error.last_token(),
        RecoveredSeparatedListEntry::Node(node) => node.last_token(),
    }
}
