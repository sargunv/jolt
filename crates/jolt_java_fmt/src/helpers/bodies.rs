//! Braced bodies and their item separators.

use jolt_fmt::{Doc, ListItem, Separator, cat};
use jolt_java_syntax::JavaSyntaxKind as K;
use jolt_syntax::{SyntaxNode, SyntaxToken};

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
/// one), then `close` on its own line. A body with neither prints `{}`
/// (D1).
pub(crate) fn braced<'a>(
    f: &mut JavaFormatter<'a>,
    owner: Option<SyntaxNode<'a>>,
    open: Doc,
    content: Doc,
    close: Doc,
) -> Doc {
    let dangling = owner.map_or(Doc::NIL, |owner| f.dangling(owner));
    let mut inner = Vec::with_capacity(4);
    if !content.is_nil() {
        inner.extend([f.hard_line(), content]);
    }
    if !dangling.is_nil() {
        let blank =
            !content.is_nil() && owner.is_some_and(|owner| f.blank_line_before_dangling(owner));
        inner.extend([if blank { f.empty_line() } else { f.hard_line() }, dangling]);
    }
    if inner.is_empty() {
        return cat![f; open, close];
    }
    let inner = f.concat(inner);
    let inner = f.indent(inner);
    cat![f; open, inner, f.hard_line(), close]
}

fn kind(node: SyntaxNode<'_>) -> K {
    K::from_raw(node.kind())
}

/// A removed `;` whose comments stand in its place.
fn is_carrier(node: SyntaxNode<'_>) -> bool {
    matches!(kind(node), K::EmptyDeclaration | K::EmptyStatement)
        || (kind(node) == K::BlockStatement
            && node
                .children()
                .next()
                .and_then(jolt_syntax::SyntaxElement::as_node)
                .is_some_and(|child| kind(child) == K::EmptyStatement))
}

fn line_or_blank(f: &JavaFormatter<'_>, blank: bool) -> Doc {
    if blank { f.empty_line() } else { f.hard_line() }
}

/// Members: a blank line between two members, except between two fields,
/// which keep the source's choice. The comments of a removed `;` keep the
/// source's choice too.
pub(crate) fn member_separator<'a>(f: &mut JavaFormatter<'a>, separator: Separator<'a>) -> Doc {
    if is_carrier(separator.prev) {
        return line_or_blank(f, separator.blank_line_in_source);
    }
    let fields =
        kind(separator.prev) == K::FieldDeclaration && kind(separator.next) == K::FieldDeclaration;
    line_or_blank(f, !fields || separator.blank_line_in_source)
}

/// Statements: a blank line iff the source had one.
pub(crate) fn statement_separator<'a>(f: &mut JavaFormatter<'a>, separator: Separator<'a>) -> Doc {
    line_or_blank(f, separator.blank_line_in_source)
}
