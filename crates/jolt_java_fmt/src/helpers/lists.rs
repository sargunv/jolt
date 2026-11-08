//! Delimited comma lists.

use jolt_fmt::{Doc, cat, group, indent, is_unclosed};
use jolt_syntax::{AstNode, ListElement, SyntaxNode, SyntaxToken};

use crate::{JavaFormat, JavaFormatter};

/// The two layouts of a delimited list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ListLayout {
    /// Argument, parameter, type-argument and pattern lists: `(a, b)` when
    /// flat, and no trailing comma (Java forbids one there).
    Plain,
    /// Array and annotation-array initializers: `{ a, b }` when flat; when
    /// broken, a synthesized `,` after the last element (before a trailing
    /// line comment of the last element: `last, // c`). A trailing `,` in
    /// the source forces the broken layout and is printed with `token`.
    Initializer,
}

/// `open item, item close` with the break-all layout: flat if it fits, else
/// every element on its own line one indent deeper and `close` on its own
/// line. `owner` is the node that owns `open` (or would); its dangling
/// comments print after the last element, preceded by a blank line when the
/// source had one there.
pub(crate) fn delimited<'a, T: AstNode<'a>>(
    f: &mut JavaFormatter<'a>,
    owner: SyntaxNode<'a>,
    open: Option<SyntaxToken<'a>>,
    elements: impl IntoIterator<Item = ListElement<'a, T>>,
    close: Option<SyntaxToken<'a>>,
    layout: ListLayout,
) -> Doc {
    let mut elements = elements
        .into_iter()
        .filter(|element| !matches!(element, ListElement::Missing))
        .peekable();
    let mut docs = Vec::new();
    let mut has_items = false;
    // The last element is an item, or a separator (recovery: a trailing one).
    let mut after_item = false;
    while let Some(element) = elements.next() {
        let separator = matches!(element, ListElement::Separator(_));
        // Two items without a separator between them (recovery).
        if !separator && after_item {
            docs.push(f.line());
        }
        docs.push(list_item(f, element));
        if separator && elements.peek().is_some() {
            docs.push(f.line());
        }
        after_item = !separator;
        has_items |= !separator;
    }
    if has_items && layout == ListLayout::Initializer {
        if !after_item {
            docs.push(f.break_parent());
        } else if owner.is_recovery_free() {
            // N8 normalizes only recovery-free lists.
            let comma = f.sticky(",");
            docs.push(f.if_break(comma, Doc::NIL));
        }
    }
    let items = f.concat(docs);
    let inner = f.with_dangling(owner, items);
    let line = match layout {
        ListLayout::Plain => f.soft_line(),
        ListLayout::Initializer => f.line(),
    };
    let before_close = before_closer(open, line);
    let open = f.token_opt(open);
    let close = f.token_opt(close);
    if inner.is_nil() {
        return cat![f; open, close];
    }
    let inner = indent![f; line, inner];
    group![f; open, inner, before_close, close]
}

/// `line`, which puts the closer on its own line when the list breaks, or
/// nothing when the closer is missing (recovery): what follows then
/// continues after the last element, where the source has it.
fn before_closer(open: Option<SyntaxToken<'_>>, line: Doc) -> Doc {
    if open.is_some_and(is_unclosed::<JavaFormat>) {
        Doc::NIL
    } else {
        line
    }
}

/// `open inner close` for a node with one inner element (brackets, casts):
/// no line breaks of its own; `inner` is one indent deeper when
/// `indent_inner`, or when `owner`'s dangling comments (own-line comments
/// before `close`) print on their own lines after it.
pub(crate) fn enclosed<'a>(
    f: &mut JavaFormatter<'a>,
    owner: SyntaxNode<'a>,
    (open, close): (Option<SyntaxToken<'a>>, Option<SyntaxToken<'a>>),
    inner: Doc,
    indent_inner: bool,
) -> Doc {
    let open = f.token_opt(open);
    let (inner, before_close) = if f.has_dangling_comments(owner) {
        let inner = f.with_dangling(owner, inner);
        (f.indent(inner), f.hard_line())
    } else {
        let inner = if indent_inner { f.indent(inner) } else { inner };
        (inner, Doc::NIL)
    };
    let close = f.token_opt(close);
    cat![f; open, inner, before_close, close]
}

/// `(inner)` that breaks like a call: flat if it fits, otherwise `(` ends the
/// line, `inner` and `owner`'s dangling comments continue one indent deeper,
/// and `)` returns to the outer indent. Statement headers and parenthesized
/// expressions.
pub(crate) fn parenthesized<'a>(
    f: &mut JavaFormatter<'a>,
    owner: SyntaxNode<'a>,
    open: Option<SyntaxToken<'a>>,
    inner: Doc,
    close: Option<SyntaxToken<'a>>,
) -> Doc {
    let before_close = before_closer(open, f.soft_line());
    let open = f.token_opt(open);
    let inner = f.with_dangling(owner, inner);
    let inner = indent![f; f.soft_line(), inner];
    let close = f.token_opt(close);
    group![f; open, inner, before_close, close]
}

/// One item of a list: the item, or a bogus node in its place.
pub(crate) fn list_item<'a, T: AstNode<'a>>(
    f: &mut JavaFormatter<'a>,
    element: ListElement<'a, T>,
) -> Doc {
    match element {
        ListElement::Item(item) => f.fmt(item),
        ListElement::Bogus(bogus) => f.node(bogus),
        ListElement::Separator(token) => f.token(token),
        ListElement::Missing => Doc::NIL,
    }
}

/// A list's elements in order, each separator that an element follows
/// followed by `after` (`space`, `line`, `hard_line`, or NIL to join
/// directly). A trailing separator (recovery) has nothing to separate.
pub(crate) fn joined<'a, T: AstNode<'a>>(
    f: &mut JavaFormatter<'a>,
    elements: impl IntoIterator<Item = ListElement<'a, T>>,
    after: Doc,
) -> Doc {
    let mut elements = elements
        .into_iter()
        .filter(|element| !matches!(element, ListElement::Missing))
        .peekable();
    let mut docs = Vec::new();
    while let Some(element) = elements.next() {
        let separator = matches!(element, ListElement::Separator(_));
        docs.push(list_item(f, element));
        if separator && elements.peek().is_some() {
            docs.push(after);
        }
    }
    f.concat(docs)
}

/// A list kind that no rule formats as a unit: its children separated by
/// spaces. Reaching it is a rule bug.
pub(crate) fn unexpected_list<'a>(f: &mut JavaFormatter<'a>, node: SyntaxNode<'a>) -> Doc {
    debug_assert!(false, "list {node:?} has no rule of its own");
    let mut docs = Vec::new();
    for child in node.children() {
        if !docs.is_empty() {
            docs.push(f.space());
        }
        docs.push(f.element(child));
    }
    f.concat(docs)
}
