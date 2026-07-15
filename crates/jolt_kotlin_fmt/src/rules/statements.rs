use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_kotlin_syntax::{
    BlockItem, Declaration, Expression, ExpressionStatement, KotlinRoleElement, Statement,
    StatementSyntax,
};

mod blocks;

use crate::helpers::comments::{LeadingTrivia, format_terminator_list};
use crate::helpers::recovery::{format_malformed, format_or_verbatim, format_required_field};
use crate::rules::expressions::{format_expression, format_expression_without_leading};
pub(crate) use blocks::format_block;

pub(crate) fn format_block_item<'source>(
    doc: &mut DocBuilder<'source>,
    item: &BlockItem<'source>,
) -> Doc<'source> {
    match item {
        BlockItem::ClassDeclaration(declaration) => {
            format_declaration_item(doc, Declaration::ClassDeclaration(*declaration))
        }
        BlockItem::InterfaceDeclaration(declaration) => {
            format_declaration_item(doc, Declaration::InterfaceDeclaration(*declaration))
        }
        BlockItem::ObjectDeclaration(declaration) => {
            format_declaration_item(doc, Declaration::ObjectDeclaration(*declaration))
        }
        BlockItem::FunctionDeclaration(declaration) => {
            format_declaration_item(doc, Declaration::FunctionDeclaration(*declaration))
        }
        BlockItem::PropertyDeclaration(declaration) => {
            format_declaration_item(doc, Declaration::PropertyDeclaration(*declaration))
        }
        BlockItem::TypeAliasDeclaration(declaration) => {
            format_declaration_item(doc, Declaration::TypeAliasDeclaration(*declaration))
        }
        BlockItem::SecondaryConstructor(constructor) => {
            format_declaration_item(doc, Declaration::SecondaryConstructor(*constructor))
        }
        BlockItem::InitializerBlock(block) => {
            format_declaration_item(doc, Declaration::InitializerBlock(*block))
        }
        BlockItem::Statement(statement) => {
            format_statement_syntax_with_leading(doc, &StatementSyntax::Statement(*statement))
        }
        BlockItem::ExpressionStatement(statement) => format_statement_syntax_with_leading(
            doc,
            &StatementSyntax::ExpressionStatement(*statement),
        ),
        BlockItem::LocalDeclaration(declaration) => {
            format_statement_syntax(doc, &StatementSyntax::LocalDeclaration(*declaration))
        }
        BlockItem::Block(block) => format_block(doc, block),
        BlockItem::BogusBlockItem(item) => format_malformed(item, doc),
    }
}

fn format_declaration_item<'source>(
    doc: &mut DocBuilder<'source>,
    declaration: Declaration<'source>,
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
            format_or_verbatim(declaration, doc, |doc| {
                format_required_field(declaration.declaration(), doc, |declaration, doc| {
                    crate::rules::declarations::format_declaration(
                        doc,
                        &Declaration::PropertyDeclaration(declaration),
                    )
                })
            })
        }
        StatementSyntax::Block(block) => format_block(doc, block),
        StatementSyntax::BogusStatement(statement) => format_malformed(statement, doc),
    }
}

fn format_statement_node<'source>(
    doc: &mut DocBuilder<'source>,
    statement: &Statement<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    format_or_verbatim(statement, doc, |doc| {
        let statement_doc = format_required_field(statement.statement(), doc, |inner, doc| {
            format_statement_role(doc, inner, leading)
        });
        let tail = format_required_field(statement.tail(), doc, |tail, doc| {
            format_terminator_list(doc, &tail, true)
        });
        doc.concat([statement_doc, tail])
    })
}

fn format_statement_role<'source>(
    doc: &mut DocBuilder<'source>,
    inner: KotlinRoleElement<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    if let Some(statement) = inner.cast_family::<StatementSyntax<'source>>() {
        return format_statement_owned(doc, &statement, leading);
    }
    if let Some(expression) = inner.cast_family::<Expression<'source>>() {
        return match leading {
            LeadingTrivia::Preserve => format_expression(doc, &expression),
            LeadingTrivia::SuppressAlreadyHandled => {
                format_expression_without_leading(doc, &expression)
            }
        };
    }
    if let Some(declaration) = inner.cast_family::<Declaration<'source>>() {
        return crate::rules::declarations::format_declaration(doc, &declaration);
    }
    doc.block_on_invariant("Kotlin statement role had an unsupported generated element");
    Doc::nil()
}

fn format_expression_statement<'source>(
    doc: &mut DocBuilder<'source>,
    statement: &ExpressionStatement<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    format_or_verbatim(statement, doc, |doc| {
        format_required_field(
            statement.expression(),
            doc,
            |expression, doc| match leading {
                LeadingTrivia::Preserve => format_expression(doc, &expression),
                LeadingTrivia::SuppressAlreadyHandled => {
                    format_expression_without_leading(doc, &expression)
                }
            },
        )
    })
}
