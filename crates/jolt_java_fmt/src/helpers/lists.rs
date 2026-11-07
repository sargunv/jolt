//! Delimited comma lists.

use jolt_fmt::{Doc, cat};
use jolt_syntax::{AstNode, ListElement, SyntaxElement, SyntaxNode, SyntaxToken};

use crate::JavaFormatter;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TrailingComma {
    /// Argument, parameter and type-argument lists (Java forbids one there).
    Never,
    /// Array and annotation-array initializers: a synthesized `,` when
    /// broken, except after a last element that carries a trailing line
    /// comment (`last // c` / `}`, as in the reference); a trailing `,` in the
    /// source forces the broken layout and is printed with `token`.
    IfBroken,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct ListLayout {
    /// `{ a, b }` when flat.
    pub(crate) padded: bool,
    pub(crate) trailing_comma: TrailingComma,
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
    elements: impl Iterator<Item = ListElement<'a, T>>,
    close: Option<SyntaxToken<'a>>,
    layout: ListLayout,
) -> Doc {
    let elements: Vec<_> = elements
        .filter(|element| !matches!(element, ListElement::Missing))
        .collect();
    let mut docs = Vec::with_capacity(elements.len() * 2 + 2);
    let mut after_item = false;
    let mut has_items = false;
    let mut last_commented = false;
    for (index, element) in elements.iter().enumerate() {
        let last = index + 1 == elements.len();
        match *element {
            ListElement::Item(item) => {
                if after_item {
                    docs.push(f.line());
                }
                last_commented = f.has_trailing_line_comment(SyntaxElement::Node(item.syntax()));
                docs.push(f.fmt(item));
                after_item = true;
                has_items = true;
            }
            ListElement::Bogus(node) => {
                if after_item {
                    docs.push(f.line());
                }
                last_commented = f.has_trailing_line_comment(SyntaxElement::Node(node));
                docs.push(f.node(node));
                after_item = true;
                has_items = true;
            }
            ListElement::Separator(token) => {
                docs.push(f.token(token));
                if !last {
                    docs.push(f.line());
                }
                after_item = false;
            }
            ListElement::Missing => {}
        }
    }
    let trailing_separator = matches!(elements.last(), Some(ListElement::Separator(_)));
    if has_items && layout.trailing_comma == TrailingComma::IfBroken {
        if trailing_separator {
            docs.push(f.break_parent());
        } else if !last_commented {
            let comma = f.sticky(",");
            docs.push(f.if_break(comma, Doc::NIL, None));
        }
    }
    let dangling = f.dangling(owner);
    if !dangling.is_nil() {
        if has_items || trailing_separator {
            let blank = f.blank_line_before_dangling(owner);
            docs.push(if blank { f.empty_line() } else { f.hard_line() });
        }
        docs.push(dangling);
    }
    let open = f.token_opt(open);
    let close = f.token_opt(close);
    if docs.is_empty() {
        return cat![f; open, close];
    }
    let line = if layout.padded {
        f.line()
    } else {
        f.soft_line()
    };
    let inner = f.concat(docs);
    let inner = cat![f; line, inner];
    let inner = f.indent(inner);
    let list = cat![f; open, inner, line, close];
    f.group(list)
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
    let dangling = f.dangling(owner);
    let (inner, before_close) = if dangling.is_nil() {
        let inner = if indent_inner { f.indent(inner) } else { inner };
        (inner, Doc::NIL)
    } else {
        let inner = cat![f; inner, f.hard_line(), dangling];
        (f.indent(inner), f.hard_line())
    };
    let close = f.token_opt(close);
    cat![f; open, inner, before_close, close]
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

/// A list kind that no rule formats as a unit: its children separated by
/// spaces. Reaching it is a rule bug.
pub(crate) fn unexpected_list<'a>(f: &mut JavaFormatter<'a>, node: SyntaxNode<'a>) -> Doc {
    debug_assert!(false, "list {node:?} has no rule of its own");
    let mut docs = Vec::new();
    for child in node.children() {
        if !docs.is_empty() {
            docs.push(f.space());
        }
        docs.push(match child {
            SyntaxElement::Node(child) => f.node(child),
            SyntaxElement::Token(token) => f.token(token),
        });
    }
    f.concat(docs)
}
