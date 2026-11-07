//! Area B, statements: blocks, control flow with brace promotion (N4), jump
//! statements, switches, and try statements.

mod blocks;
mod control;
mod jumps;
mod switches;
mod tries;

use jolt_fmt::{Doc, ListItem, cat, group, indent};
use jolt_java_syntax::{BlockItem, BlockStatement, BlockStatementList};
use jolt_syntax::{AstNode, SyntaxElement, SyntaxNode, SyntaxToken, TriviaKind};

use crate::JavaFormatter;
use crate::helpers::bodies;

pub(crate) use blocks::{
    block, block_statement, empty_statement, expression_statement, labeled_statement,
    local_class_or_interface_declaration, local_variable_declaration,
};
#[allow(
    unused_imports,
    reason = "REQUEST(shared): hook H4, wired at integration"
)]
pub(crate) use control::promoted_body_comment;
pub(crate) use control::{
    basic_for_statement, do_statement, enhanced_for_statement, enhanced_for_variable,
    for_initializer, for_statement, for_update, if_statement, synchronized_statement,
    while_statement,
};
pub(crate) use jumps::{
    assert_statement, break_statement, continue_statement, return_statement, throw_statement,
    yield_statement,
};
pub(crate) use switches::{
    case_constant, case_pattern, guard, switch_block, switch_block_statement_group,
    switch_expression, switch_label, switch_rule, switch_statement,
};
pub(crate) use tries::{
    catch_clause, catch_parameter, catch_type_list, finally_clause, resource,
    resource_specification, resource_variable_declaration, try_statement,
    try_with_resources_statement, variable_access,
};

/// `(inner)` of a statement header: flat if it fits, otherwise `(` ends the
/// line, `inner` continues one indent deeper, and `)` returns to the
/// statement's indent. `owner`'s dangling comments print before `)`, each on
/// its own line.
fn parenthesized<'a>(
    f: &mut JavaFormatter<'a>,
    owner: SyntaxNode<'a>,
    open: Option<SyntaxToken<'a>>,
    inner: Doc,
    close: Option<SyntaxToken<'a>>,
) -> Doc {
    let open = f.token_opt(open);
    let inner = with_dangling(f, owner, inner);
    let inner = indent![f; f.soft_line(), inner];
    let close = f.token_opt(close);
    group![f; open, inner, f.soft_line(), close]
}

/// `inner` followed by `owner`'s dangling comments on their own lines, with a
/// blank line before them when the source had one.
fn with_dangling<'a>(f: &mut JavaFormatter<'a>, owner: SyntaxNode<'a>, inner: Doc) -> Doc {
    let dangling = f.dangling(owner);
    if dangling.is_nil() {
        return inner;
    }
    if inner.is_nil() {
        return dangling;
    }
    let line = if f.blank_line_before_dangling(owner) {
        f.empty_line()
    } else {
        f.hard_line()
    };
    cat![f; inner, line, dangling]
}

/// The element has leading comments and the source breaks the line between
/// them and the element.
fn comments_break_before<'a>(f: &JavaFormatter<'a>, element: SyntaxElement<'a>) -> bool {
    f.has_leading_comments(element)
        && element.first_token().is_some_and(|token| {
            token
                .leading()
                .any(|piece| piece.kind == TriviaKind::Newline)
        })
}

/// The statements of a block or switch group as item-list items: every
/// child of the list, including bogus ones, with empty statements elided
/// (N5).
fn statement_items(list: Option<BlockStatementList<'_>>) -> Vec<ListItem<'_>> {
    let Some(list) = list else {
        return Vec::new();
    };
    list.syntax()
        .children()
        .filter_map(SyntaxElement::as_node)
        .map(|node| {
            let empty = BlockStatement::cast(node)
                .and_then(BlockStatement::item)
                .is_some_and(|item| matches!(item, BlockItem::EmptyStatement(_)));
            if empty {
                ListItem::Elide(node)
            } else {
                ListItem::Format(node)
            }
        })
        .collect()
}

/// Statements: a blank line iff the source had one. After a removed `;`
/// whose leading comments stand in its place, a blank line between the last
/// of those comments and the `;` also counts, since nothing else is left to
/// carry it.
fn statement_separator<'a>(f: &mut JavaFormatter<'a>, separator: jolt_fmt::Separator<'a>) -> Doc {
    if blank_after_removed_comments(separator.prev) {
        f.empty_line()
    } else {
        bodies::statement_separator(f, separator)
    }
}

/// Whether `node` is a statement holding only an empty statement whose `;`
/// has a leading comment followed by a blank line.
fn blank_after_removed_comments(node: SyntaxNode<'_>) -> bool {
    let semicolon = BlockStatement::cast(node)
        .and_then(BlockStatement::item)
        .and_then(|item| match item {
            BlockItem::EmptyStatement(empty) => empty.semicolon(),
            _ => None,
        });
    let Some(semicolon) = semicolon else {
        return false;
    };
    let mut newlines = None;
    for piece in semicolon.leading() {
        match piece.kind {
            TriviaKind::Newline => newlines = newlines.map(|count| count + 1),
            TriviaKind::LineComment
            | TriviaKind::BlockComment
            | TriviaKind::DocComment
            | TriviaKind::UnterminatedBlockComment => newlines = Some(0),
            _ => {}
        }
    }
    newlines.is_some_and(|count| count >= 2)
}
