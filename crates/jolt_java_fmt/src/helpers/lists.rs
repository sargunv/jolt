//! Delimited comma lists.

use jolt_fmt::{Doc, cat, group, indent, is_unclosed};
use jolt_java_syntax::{Expression, JavaSyntaxKind as K};
use jolt_syntax::{AstNode, ListElement, SyntaxNode, SyntaxToken};

use crate::{JavaFormat, JavaFormatter};

/// The two layouts of a delimited list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ListLayout {
    /// Parameter, type-argument and pattern lists: `(a, b)` when flat, and
    /// no trailing comma (Java forbids one there).
    Plain,
    /// Argument lists: `Plain`, and a last argument with a body (see
    /// `has_body`) hugs the parentheses when it has a forced break and the
    /// arguments before it have none, in a recovery-free list without
    /// dangling comments: `run(a, () -> {` … `});` (see
    /// [`DocBuilder::hug`](jolt_fmt::DocBuilder::hug)). So two lambda bodies
    /// keep the broken list.
    Arguments,
    /// Array and annotation-array initializers: `{ a, b }` when flat; when
    /// broken, a synthesized `,` after the last element (before a trailing
    /// line comment of the last element: `last, // c`). A trailing `,` in
    /// the source forces the broken layout and is printed with `token`.
    Initializer,
}

/// `open item, item close` with the break-all layout: flat if it fits, else
/// every element on its own line one indent deeper and `close` on its own
/// line. `owner` is the node that owns `open` (or would, when `open` is
/// missing); its dangling comments print after the last element, preceded
/// by a blank line when the source had one there. Every list node with a
/// delimiter passes its node as `owner`, so its dangling comments print.
///
/// For comma-separated lists only: a source separator prints with `token`,
/// a synthesized one (N8) with `sticky`, and a `Missing` element prints
/// nothing. A resource specification (`;` separators and a removable
/// trailing `;`) has its own rule, which uses [`parenthesized`].
///
/// When `open` has no closer ([`jolt_fmt::is_unclosed`], recovery), no line
/// is put where the closer would go, so what follows continues after the
/// last element, where the source has it. [`parenthesized`], and so the
/// resource specification, does the same.
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
    // Whether the last item has a forced break, and whether an item before it
    // has one.
    let (mut last_breaks, mut earlier_breaks) = (false, false);
    let mut last_body = false;
    while let Some(element) = elements.next() {
        let separator = matches!(element, ListElement::Separator(_));
        // Two items without a separator between them (recovery).
        if !separator && after_item {
            docs.push(f.line());
        }
        last_body = matches!(element, ListElement::Item(item) if has_body(item.syntax()));
        let doc = list_item(f, element);
        if !separator {
            earlier_breaks |= last_breaks;
            last_breaks = f.will_break(doc);
        }
        docs.push(doc);
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
    let hugs = layout == ListLayout::Arguments
        && last_body
        && last_breaks
        && !earlier_breaks
        && after_item
        && owner.is_recovery_free()
        && !f.has_dangling_comments(owner);
    // The items with a space after each separator instead of a line.
    let hugged = hugs.then(|| {
        let (line, space) = (f.line(), f.space());
        let docs: Vec<Doc> = docs
            .iter()
            .map(|&doc| if doc == line { space } else { doc })
            .collect();
        f.concat(docs)
    });
    let items = f.concat(docs);
    let inner = f.with_dangling(owner, items);
    let line = match layout {
        ListLayout::Plain | ListLayout::Arguments => f.soft_line(),
        ListLayout::Initializer => f.line(),
    };
    let before_close = before_closer(open, line);
    let open = f.token_opt(open);
    let close = f.token_opt(close);
    if inner.is_nil() {
        return cat![f; open, close];
    }
    let inner = indent![f; line, inner];
    let broken = group![f; open, inner, before_close, close];
    match hugged {
        Some(hugged) => {
            let hugged = cat![f; open, hugged, close];
            f.hug(hugged, broken)
        }
        None => broken,
    }
}

/// A lambda with a block body, an anonymous class, a switch expression or a
/// text block: an argument whose body breaks wherever it starts.
fn has_body(node: SyntaxNode<'_>) -> bool {
    match Expression::cast(node) {
        Some(Expression::LambdaExpression(lambda)) => lambda
            .body()
            .is_some_and(|body| K::from_raw(body.syntax().kind()) == K::Block),
        Some(Expression::ObjectCreationExpression(creation)) => creation.body().is_some(),
        Some(Expression::SwitchExpression(_)) => true,
        Some(Expression::LiteralExpression(literal)) => literal
            .literal()
            .is_some_and(|token| K::from_raw(token.kind()) == K::TextBlockLiteral),
        _ => false,
    }
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
