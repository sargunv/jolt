use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_kotlin_syntax::{
    BlockItem, Declaration, Expression, ExpressionStatement, KotlinSyntaxField, KotlinSyntaxView,
    Statement, StatementContentSyntax, StatementContentValue, StatementSyntax, ThrowExpression,
};

mod blocks;

use crate::helpers::comments::{
    LeadingTrivia, TrailingTrivia, comment_forces_line, format_line_start_construct,
    format_terminator_list, format_token, format_trailing_comment_list_before_line_break,
};
use crate::helpers::recovery::{format_malformed, format_required_field};
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
        BlockItem::EmptyStatement(statement) => format_empty_statement(doc, statement),
        BlockItem::BogusBlockItem(item) => format_malformed(item, doc),
    }
}

/// A body item together with the layout state owned by its following boundary.
pub(crate) struct BodyBoundaryDoc<'source> {
    pub(crate) doc: Doc<'source>,
    pub(crate) forces_line_after: bool,
}

/// Formats an item at the enclosing body's boundary with its source successor.
///
/// A final token can be formatted by a nested rule while its trailing comments
/// semantically belong to the surrounding body. The syntax tree may also expose
/// the same physical comment as leading trivia on the successor. The body join
/// therefore relocates the complete trailing run out of the nested layout, then
/// emits only comments not already owned by the successor.
pub(crate) fn format_block_item_at_body_boundary<'source>(
    doc: &mut DocBuilder<'source>,
    item: &BlockItem<'source>,
    successor: Option<&jolt_kotlin_syntax::KotlinSyntaxToken<'source>>,
) -> BodyBoundaryDoc<'source> {
    // A block item in a body begins its own line, so its first token's
    // leading comments keep lines of their own.
    format_line_start_construct(doc, item.first_token(), |doc| {
        format_block_item_at_body_boundary_at_line_start(doc, item, successor)
    })
}

fn format_block_item_at_body_boundary_at_line_start<'source>(
    doc: &mut DocBuilder<'source>,
    item: &BlockItem<'source>,
    successor: Option<&jolt_kotlin_syntax::KotlinSyntaxToken<'source>>,
) -> BodyBoundaryDoc<'source> {
    let Some(last) = item.last_token() else {
        return BodyBoundaryDoc {
            doc: format_block_item(doc, item),
            forces_line_after: false,
        };
    };
    let trailing = last.trailing_comments().collect::<Vec<_>>();
    if trailing.is_empty() {
        return BodyBoundaryDoc {
            doc: format_block_item(doc, item),
            forces_line_after: false,
        };
    }

    if item.last_token_is_malformed_owned() {
        return BodyBoundaryDoc {
            doc: format_block_item(doc, item),
            forces_line_after: false,
        };
    }

    // An ancestor body boundary for the same physical final token is the
    // source-level join that owns this trivia. Keep formatting structurally,
    // but do not create a second relocation/emission at a nested body view.
    if doc.relocates_trailing_trivia(&last) {
        return BodyBoundaryDoc {
            doc: format_block_item(doc, item),
            forces_line_after: false,
        };
    }

    let item = doc.with_relocated_trailing_trivia(&last, |doc| format_block_item(doc, item));
    let mut successor_comments = successor
        .into_iter()
        .flat_map(jolt_syntax::SyntaxToken::leading_comments)
        .map(|comment| comment.text_range())
        .peekable();
    let comments = trailing
        .into_iter()
        .filter(|comment| {
            let range = comment.text_range();
            while successor_comments
                .peek()
                .is_some_and(|successor| successor.start() < range.start())
            {
                successor_comments.next();
            }
            successor_comments
                .peek()
                .is_none_or(|successor| *successor != range)
        })
        .collect::<Vec<_>>();
    let forces_line_after = comments.iter().any(comment_forces_line);
    let comments = format_trailing_comment_list_before_line_break(doc, comments);
    BodyBoundaryDoc {
        doc: doc.concat([item, comments]),
        forces_line_after,
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
            format_required_field(declaration.declaration(), doc, |declaration, doc| {
                crate::rules::declarations::format_declaration(
                    doc,
                    &Declaration::PropertyDeclaration(declaration),
                )
            })
        }
        StatementSyntax::Block(block) => format_block(doc, block),
        StatementSyntax::EmptyStatement(statement) => format_empty_statement(doc, statement),
        StatementSyntax::BogusStatement(statement) => format_malformed(statement, doc),
    }
}

fn format_empty_statement<'source>(
    doc: &mut DocBuilder<'source>,
    statement: &jolt_kotlin_syntax::EmptyStatement<'source>,
) -> Doc<'source> {
    format_required_field(statement.terminator(), doc, |terminator, doc| {
        format_token(
            doc,
            &terminator,
            LeadingTrivia::Preserve,
            TrailingTrivia::Preserve,
        )
    })
}

fn format_statement_node<'source>(
    doc: &mut DocBuilder<'source>,
    statement: &Statement<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let tail = format_required_field(statement.tail(), doc, |tail, doc| {
        format_terminator_list(doc, &tail)
    });
    format_required_field(statement.statement(), doc, |inner, doc| {
        format_statement_role_with_tail(doc, inner, leading, tail)
    })
}

fn format_statement_role_with_tail<'source>(
    doc: &mut DocBuilder<'source>,
    inner: StatementContentValue<'source>,
    leading: LeadingTrivia,
    tail: Doc<'source>,
) -> Doc<'source> {
    if let Some(expression) = statement_throw_expression(inner) {
        return crate::rules::expressions::format_throw_expression_with_suffix(
            doc,
            &expression,
            leading,
            tail,
        );
    }
    let statement = format_statement_role(doc, inner, leading);
    doc.concat([statement, tail])
}

fn statement_throw_expression(inner: StatementContentValue<'_>) -> Option<ThrowExpression<'_>> {
    let expression = match inner.classify().ok()? {
        StatementContentSyntax::Expression(expression) => expression,
        StatementContentSyntax::Statement(StatementSyntax::ExpressionStatement(statement)) => {
            match statement.expression() {
                KotlinSyntaxField::Present(expression) => expression,
                KotlinSyntaxField::Missing(_) | KotlinSyntaxField::Malformed(_) => return None,
            }
        }
        StatementContentSyntax::Statement(_) | StatementContentSyntax::Declaration(_) => {
            return None;
        }
    };
    match expression {
        Expression::ThrowExpression(expression) => Some(expression),
        _ => None,
    }
}

fn format_statement_role<'source>(
    doc: &mut DocBuilder<'source>,
    inner: StatementContentValue<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    match inner.classify() {
        Ok(StatementContentSyntax::Statement(statement)) => {
            format_statement_owned(doc, &statement, leading)
        }
        Ok(StatementContentSyntax::Expression(expression)) => match leading {
            LeadingTrivia::Preserve => format_expression(doc, &expression),
            LeadingTrivia::SuppressAlreadyHandled => {
                format_expression_without_leading(doc, &expression)
            }
        },
        Ok(StatementContentSyntax::Declaration(declaration)) => {
            crate::rules::declarations::format_declaration(doc, &declaration)
        }
        Err(error) => {
            doc.block_on_invariant(error.to_string());
            Doc::nil()
        }
    }
}

fn format_expression_statement<'source>(
    doc: &mut DocBuilder<'source>,
    statement: &ExpressionStatement<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
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
}
