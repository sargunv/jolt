//! `if`, loops and `synchronized`, with brace promotion (N4) of their
//! bodies.

use jolt_fmt::{CommentContext, CommentLayout, Doc, ListItem, Placement, cat};
use jolt_java_syntax::JavaSyntaxKind as K;
use jolt_java_syntax::{
    BasicForStatement, DoStatement, EnhancedForStatement, EnhancedForVariable,
    EnhancedForVariableType, ForInitializer, ForInitializerValue, ForStatement, ForUpdate,
    IfStatement, Statement, StatementExpressionList, SynchronizedStatement, WhileStatement,
};
use jolt_syntax::{AstNode, SyntaxElement, SyntaxNode};

use super::{parenthesized, statement_separator};
use crate::JavaFormatter;
use crate::helpers::bodies::braced;
use crate::helpers::lists::joined;
use crate::rules::types::inline_modifiers;

/// `if (condition) body`, then ` else body`. An `else if` chain stays flat;
/// it is walked iteratively (its length is unbounded), each chained `if`'s
/// comments printed around it as `node` would.
pub(crate) fn if_statement<'a>(f: &mut JavaFormatter<'a>, node: IfStatement<'a>) -> Doc {
    let mut docs = Vec::new();
    let mut chained = Vec::new();
    let mut current = node;
    loop {
        let keyword = f.token_opt(current.if_keyword());
        let condition = f.fmt_opt(current.condition());
        let header = parenthesized(
            f,
            current.syntax(),
            current.open_paren(),
            condition,
            current.close_paren(),
        );
        let then_branch = body(f, current.syntax(), current.then_branch());
        docs.extend([keyword, f.space(), header, then_branch]);
        if let Some(keyword) = current.else_keyword() {
            let keyword = f.token(keyword);
            docs.extend([f.space(), keyword]);
        }
        match current.else_branch() {
            Some(Statement::IfStatement(next)) if !next.syntax().is_malformed() => {
                let leading = f.leading_comments(next.syntax());
                docs.extend([f.space(), leading]);
                chained.push(next.syntax());
                current = next;
            }
            branch => {
                let else_branch = body(f, current.syntax(), branch);
                docs.push(else_branch);
                break;
            }
        }
    }
    for &node in chained.iter().rev() {
        let trailing = f.trailing_comments(node);
        docs.push(trailing);
    }
    f.concat(docs)
}

pub(crate) fn while_statement<'a>(f: &mut JavaFormatter<'a>, node: WhileStatement<'a>) -> Doc {
    let keyword = f.token_opt(node.while_keyword());
    let condition = f.fmt_opt(node.condition());
    let header = parenthesized(
        f,
        node.syntax(),
        node.open_paren(),
        condition,
        node.close_paren(),
    );
    let body = body(f, node.syntax(), node.body());
    cat![f; keyword, f.space(), header, body]
}

/// `do body while (condition);`, with `while` on the line of the body's `}`.
pub(crate) fn do_statement<'a>(f: &mut JavaFormatter<'a>, node: DoStatement<'a>) -> Doc {
    let do_keyword = f.token_opt(node.do_keyword());
    let body = body(f, node.syntax(), node.body());
    let while_keyword = f.token_opt(node.while_keyword());
    let condition = f.fmt_opt(node.condition());
    let header = parenthesized(
        f,
        node.syntax(),
        node.open_paren(),
        condition,
        node.close_paren(),
    );
    let semicolon = f.token_opt(node.semicolon());
    cat![
        f;
        do_keyword,
        body,
        f.space(),
        while_keyword,
        f.space(),
        header,
        semicolon,
    ]
}

pub(crate) fn for_statement<'a>(f: &mut JavaFormatter<'a>, node: ForStatement<'a>) -> Doc {
    f.fmt_opt(node.form())
}

/// `for (init; condition; update) body`. A header that does not fit puts
/// each part on its own line; an empty header is `for (;;)`.
pub(crate) fn basic_for_statement<'a>(
    f: &mut JavaFormatter<'a>,
    node: BasicForStatement<'a>,
) -> Doc {
    let keyword = f.token_opt(node.for_keyword());
    let empty =
        node.initializer().is_none() && node.condition().is_none() && node.update().is_none();
    let initializer = f.fmt_opt(node.initializer());
    let first = f.token_opt(node.first_semicolon());
    let condition = f.fmt_opt(node.condition());
    let second = f.token_opt(node.second_semicolon());
    let update = f.fmt_opt(node.update());
    let parts = if empty {
        cat![f; first, f.soft_line(), second]
    } else {
        cat![f; initializer, first, f.line(), condition, second, f.line(), update]
    };
    let header = parenthesized(
        f,
        node.syntax(),
        node.open_paren(),
        parts,
        node.close_paren(),
    );
    let body = body(f, node.syntax(), node.body());
    cat![f; keyword, f.space(), header, body]
}

/// `for (Type name : iterable) body`.
pub(crate) fn enhanced_for_statement<'a>(
    f: &mut JavaFormatter<'a>,
    node: EnhancedForStatement<'a>,
) -> Doc {
    let keyword = f.token_opt(node.for_keyword());
    let variable = f.fmt_opt(node.variable());
    let colon = f.token_opt(node.colon());
    let iterable = f.fmt_opt(node.iterable());
    let parts = cat![f; variable, f.space(), colon, f.space(), iterable];
    let header = parenthesized(
        f,
        node.syntax(),
        node.open_paren(),
        parts,
        node.close_paren(),
    );
    let body = body(f, node.syntax(), node.body());
    cat![f; keyword, f.space(), header, body]
}

pub(crate) fn enhanced_for_variable<'a>(
    f: &mut JavaFormatter<'a>,
    node: EnhancedForVariable<'a>,
) -> Doc {
    let modifiers = node
        .modifiers()
        .map_or(Doc::NIL, |modifiers| inline_modifiers(f, modifiers));
    let ty = match node.r#type() {
        Some(EnhancedForVariableType::Type(ty)) => f.fmt(ty),
        Some(EnhancedForVariableType::Token(token)) => f.token(token),
        None => Doc::NIL,
    };
    let name = f.token_opt(node.name());
    let dimensions = f.fmt_opt(node.dimensions());
    cat![f; modifiers, ty, f.space(), name, dimensions]
}

pub(crate) fn for_initializer<'a>(f: &mut JavaFormatter<'a>, node: ForInitializer<'a>) -> Doc {
    match node.value() {
        Some(ForInitializerValue::LocalVariableDeclaration(declaration)) => f.fmt(declaration),
        Some(ForInitializerValue::StatementExpressionList(list)) => expression_list(f, list),
        None => Doc::NIL,
    }
}

pub(crate) fn for_update<'a>(f: &mut JavaFormatter<'a>, node: ForUpdate<'a>) -> Doc {
    node.expressions()
        .map_or(Doc::NIL, |list| expression_list(f, list))
}

/// `a, b` on one line; an end-of-line comment after a `,` ends the line.
fn expression_list<'a>(f: &mut JavaFormatter<'a>, list: StatementExpressionList<'a>) -> Doc {
    joined(f, list.elements(), |f| f.space())
}

pub(crate) fn synchronized_statement<'a>(
    f: &mut JavaFormatter<'a>,
    node: SynchronizedStatement<'a>,
) -> Doc {
    let keyword = f.token_opt(node.synchronized_keyword());
    let expression = f.fmt_opt(node.expression());
    let header = parenthesized(
        f,
        node.syntax(),
        node.open_paren(),
        expression,
        node.close_paren(),
    );
    let block = f.fmt_opt(node.body());
    cat![f; keyword, f.space(), header, f.space(), block]
}

/// A control statement's body, preceded by a space. N4: when `owner` is
/// recovery-free, a body that is not a block is wrapped in a synthesized
/// block, and an empty statement becomes an empty block (its `;` elided
/// like an empty statement inside a block).
fn body<'a>(f: &mut JavaFormatter<'a>, owner: SyntaxNode<'a>, body: Option<Statement<'a>>) -> Doc {
    let Some(body) = body else {
        return Doc::NIL;
    };
    let promoted = match body {
        Statement::Block(_) => None,
        _ if !owner.is_recovery_free() => None,
        Statement::EmptyStatement(empty) => {
            let items = [ListItem::Elide(empty.syntax())];
            let content = f.item_list(items, statement_separator);
            Some(promoted_block(f, content))
        }
        _ => {
            let content = f.fmt(body);
            Some(promoted_block(f, content))
        }
    };
    let body = promoted.unwrap_or_else(|| f.fmt(body));
    cat![f; f.space(), body]
}

/// A synthesized block around `content`.
fn promoted_block(f: &mut JavaFormatter<'_>, content: Doc) -> Doc {
    let open = f.text("{");
    let close = f.text("}");
    braced(f, None, open, content, close)
}

/// Comment hook H4, tried after H1. An end-of-line comment after the last
/// token of a control statement's final non-block body (`if (a) return; //
/// c`) would trail the whole statement by the generic rules, so it would
/// print after the synthesized `}` of brace promotion. It trails the body
/// instead and stays inside the block, as in the reference.
pub(crate) fn promoted_body_comment<'a>(context: &CommentContext<'a, '_>) -> Option<Placement<'a>> {
    if context.comment.layout != CommentLayout::EndOfLine {
        return None;
    }
    let prev = context.prev?;
    let mut node = prev.parent();
    while node.last_token() == Some(prev) {
        let parent = node.parent()?;
        let kind = K::from_raw(node.kind());
        let body = match K::from_raw(parent.kind()) {
            K::IfStatement => IfStatement::cast(parent).is_some_and(|statement| {
                let is_else = statement.else_branch().map(AstNode::syntax) == Some(node);
                !(is_else && kind == K::IfStatement)
            }),
            K::WhileStatement | K::BasicForStatement | K::EnhancedForStatement => true,
            _ => false,
        };
        if body && kind != K::Block && Statement::can_cast(node.kind()) {
            return Some(Placement::Trailing(
                SyntaxElement::Node(node),
                CommentLayout::EndOfLine,
            ));
        }
        node = parent;
    }
    None
}
