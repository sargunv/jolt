//! Blocks, block statements, local variables, expression and return
//! statements.

use jolt_fmt::{Doc, ListItem, cat, indent};
use jolt_java_syntax::{
    Block, BlockItem, BlockStatement, ExpressionStatement, LocalVariableDeclaration,
    LocalVariableType, ReturnStatement,
};
use jolt_syntax::{AstNode, SyntaxElement};

use crate::JavaFormatter;
use crate::helpers::bodies::{braced_body, statement_separator};

pub(crate) fn block<'a>(f: &mut JavaFormatter<'a>, node: Block<'a>) -> Doc {
    let statements: Vec<_> = node
        .statements()
        .into_iter()
        .flat_map(jolt_java_syntax::BlockStatementList::items)
        .map(|statement| {
            if let Some(BlockItem::EmptyStatement(_)) = statement.item() {
                ListItem::Elide(statement.syntax())
            } else {
                ListItem::Format(statement.syntax())
            }
        })
        .collect();
    braced_body(
        f,
        node.syntax(),
        node.open_brace(),
        statements,
        node.close_brace(),
        statement_separator,
    )
}

pub(crate) fn block_statement<'a>(f: &mut JavaFormatter<'a>, node: BlockStatement<'a>) -> Doc {
    let item = f.fmt_opt(node.item());
    let semicolon = f.token_opt(node.local_declaration_semicolon());
    cat![f; item, semicolon]
}

pub(crate) fn local_variable_declaration<'a>(
    f: &mut JavaFormatter<'a>,
    node: LocalVariableDeclaration<'a>,
) -> Doc {
    let modifiers = f.fmt_opt(node.modifiers());
    let ty = match node.r#type() {
        Some(LocalVariableType::Type(ty)) => f.fmt(ty),
        Some(LocalVariableType::Token(token)) => f.token(token),
        None => Doc::NIL,
    };
    let declarators = f.fmt_opt(node.declarators());
    cat![f; modifiers, ty, f.space(), declarators]
}

pub(crate) fn expression_statement<'a>(
    f: &mut JavaFormatter<'a>,
    node: ExpressionStatement<'a>,
) -> Doc {
    let expression = f.fmt_opt(node.expression());
    let semicolon = f.token_opt(node.semicolon());
    cat![f; expression, semicolon]
}

/// `return value;`. When the keyword carries a trailing comment, the value
/// continues on the next line one indent deeper.
pub(crate) fn return_statement<'a>(f: &mut JavaFormatter<'a>, node: ReturnStatement<'a>) -> Doc {
    let keyword_comments = node
        .return_keyword()
        .is_some_and(|keyword| f.has_trailing_comments(SyntaxElement::Token(keyword)));
    let keyword = f.token_opt(node.return_keyword());
    let value = match node.expression() {
        Some(value) if keyword_comments => {
            let value = f.fmt(value);
            let value = indent![f; f.line(), value];
            f.group(value)
        }
        Some(value) => {
            let value = f.fmt(value);
            cat![f; f.space(), value]
        }
        None => Doc::NIL,
    };
    let semicolon = f.token_opt(node.semicolon());
    cat![f; keyword, value, semicolon]
}
