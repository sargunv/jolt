//! Braced bodies and their item separators.

use jolt_fmt::{Doc, ListItem, Separator, cat};
use jolt_java_syntax::{BlockItem, BlockStatement, JavaSyntaxKind as K};
use jolt_syntax::{AstNode, SyntaxElement, SyntaxNode, SyntaxToken};

use crate::JavaFormatter;

/// `{`, the items one indent deeper joined by `separator`, the owner's
/// dangling comments, `}` (see [`braced`]).
pub(crate) fn braced_body<'a>(
    f: &mut JavaFormatter<'a>,
    owner: SyntaxNode<'a>,
    open: Option<SyntaxToken<'a>>,
    items: impl IntoIterator<Item = ListItem<'a>>,
    close: Option<SyntaxToken<'a>>,
    separator: impl FnMut(&mut JavaFormatter<'a>, Separator<'a>) -> Doc,
) -> Doc {
    let open = f.token_opt(open);
    let items = f.item_list(items, separator);
    let close = f.token_opt(close);
    braced(f, Some(owner), open, items, close)
}

/// `open`, then `content` and `owner`'s dangling comments one indent deeper
/// on their own lines (a blank line between the two when the source had
/// one), then `close` on its own line. A body with neither prints `{}`.
pub(crate) fn braced<'a>(
    f: &mut JavaFormatter<'a>,
    owner: Option<SyntaxNode<'a>>,
    open: Doc,
    content: Doc,
    close: Doc,
) -> Doc {
    let inner = match owner {
        Some(owner) => f.with_dangling(owner, content),
        None => content,
    };
    if inner.is_nil() {
        return cat![f; open, close];
    }
    let inner = cat![f; f.hard_line(), inner];
    let inner = f.indent(inner);
    // An unclosed body (recovery) ends after its content, so what trails
    // the content stays on its line.
    if close.is_nil() {
        return cat![f; open, inner];
    }
    cat![f; open, inner, f.hard_line(), close]
}

fn kind(node: SyntaxNode<'_>) -> K {
    K::from_raw(node.kind())
}

/// A stray `;`: an empty declaration or empty statement.
fn is_stray_semicolon(node: SyntaxNode<'_>) -> bool {
    match kind(node) {
        K::EmptyDeclaration | K::EmptyStatement => true,
        K::BlockStatement => BlockStatement::cast(node)
            .and_then(BlockStatement::item)
            .is_some_and(|item| matches!(item, BlockItem::EmptyStatement(_))),
        _ => false,
    }
}

/// The items of a body as item-list items, each stray `;` without comments
/// elided (N5) when `owner` is recovery-free. A comment after the last `;`
/// of a list without a closer (a switch group) trails `owner`, and the `;`
/// stays so the next pass attaches the comment the same way.
pub(crate) fn body_items<'a>(
    f: &JavaFormatter<'a>,
    owner: SyntaxNode<'a>,
    items: impl IntoIterator<Item = SyntaxNode<'a>>,
) -> Vec<ListItem<'a>> {
    let normalize = owner.is_recovery_free();
    let owner_trailing = f.has_trailing_comments(SyntaxElement::Node(owner));
    items
        .into_iter()
        .map(|item| {
            let elide = normalize
                && is_stray_semicolon(item)
                && !f.has_comments_within(item)
                && !(owner_trailing && item.last_token() == owner.last_token());
            if elide {
                ListItem::Elide(item)
            } else {
                ListItem::Format(item)
            }
        })
        .collect()
}

/// Members: a blank line between two members, except between two fields,
/// and around a stray `;` kept for its comments, which keep the source's
/// choice.
pub(crate) fn member_separator<'a>(f: &mut JavaFormatter<'a>, separator: Separator<'a>) -> Doc {
    let (prev, next) = (separator.prev, separator.next);
    let fields = kind(prev) == K::FieldDeclaration && kind(next) == K::FieldDeclaration;
    let stray = is_stray_semicolon(prev) || is_stray_semicolon(next);
    f.line_break(separator.blank_line_in_source || !(fields || stray))
}

/// Statements: a blank line iff the source had one.
pub(crate) fn statement_separator<'a>(f: &mut JavaFormatter<'a>, separator: Separator<'a>) -> Doc {
    f.line_break(separator.blank_line_in_source)
}
