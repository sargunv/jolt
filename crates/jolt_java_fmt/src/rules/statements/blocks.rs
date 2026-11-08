//! Blocks, block statements, local declarations, and the simple statements.

use jolt_fmt::{Doc, cat};
use jolt_java_syntax::{
    Block, BlockStatement, EmptyStatement, ExpressionStatement, LabeledStatement,
    LocalClassOrInterfaceDeclaration, LocalVariableDeclaration, LocalVariableType,
};
use jolt_syntax::AstNode;

use super::statement_items;
use crate::JavaFormatter;
use crate::helpers::bodies::{braced_body, statement_separator};

/// `{`, the statements one per line (blank lines kept, at most one), `}`.
pub(crate) fn block<'a>(f: &mut JavaFormatter<'a>, node: Block<'a>) -> Doc {
    let statements = statement_items(f, node.syntax(), node.statements());
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

/// `modifiers Type declarators`, without the `;` (the enclosing block
/// statement or `for` header owns it).
pub(crate) fn local_variable_declaration<'a>(
    f: &mut JavaFormatter<'a>,
    node: LocalVariableDeclaration<'a>,
) -> Doc {
    let modifiers = f.fmt_opt(node.modifiers());
    let ty = f.element_opt(node.r#type().map(LocalVariableType::element));
    let declarators = f.fmt_opt(node.declarators());
    cat![f; modifiers, ty, f.space(), declarators]
}

pub(crate) fn local_class_or_interface_declaration<'a>(
    f: &mut JavaFormatter<'a>,
    node: LocalClassOrInterfaceDeclaration<'a>,
) -> Doc {
    f.fmt_opt(node.declaration())
}

/// A `;` statement that no normalization removes (a labeled empty
/// statement).
pub(crate) fn empty_statement<'a>(f: &mut JavaFormatter<'a>, node: EmptyStatement<'a>) -> Doc {
    f.token_opt(node.semicolon())
}

/// `label:` on its own line, then the statement.
pub(crate) fn labeled_statement<'a>(f: &mut JavaFormatter<'a>, node: LabeledStatement<'a>) -> Doc {
    let label = f.token_opt(node.label());
    let colon = f.token_opt(node.colon());
    let body = f.fmt_opt(node.body());
    cat![f; label, colon, f.hard_line(), body]
}

pub(crate) fn expression_statement<'a>(
    f: &mut JavaFormatter<'a>,
    node: ExpressionStatement<'a>,
) -> Doc {
    let expression = f.fmt_opt(node.expression());
    let semicolon = f.token_opt(node.semicolon());
    cat![f; expression, semicolon]
}
