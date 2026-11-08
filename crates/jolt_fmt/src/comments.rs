//! Comment classification and placement: every comment is attached exactly
//! once, as leading or trailing to an element or dangling in a node.

use std::ops::Range;

use jolt_syntax::{Language, SyntaxElement, SyntaxNode, SyntaxToken, TextRange, TriviaKind};

use crate::FormatLanguage;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommentKind {
    Line,
    /// A block comment, including an unterminated one.
    Block,
    Doc,
}

/// Where a comment sits relative to code on its lines.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommentLayout {
    /// Newline before and after (file start and end count as newlines).
    OwnLine,
    /// Newline before, code after on the same line.
    StartOfLine,
    /// Code before, newline after.
    EndOfLine,
    /// Code before and after on the same line.
    Inline,
}

#[derive(Clone, Copy, Debug)]
pub struct Comment<'a> {
    pub text: &'a str,
    pub range: TextRange,
    pub kind: CommentKind,
    /// Newlines since the previous token or comment, capped at 2.
    pub lines_before: u8,
    /// Newlines until the next comment or token, capped at 2.
    pub lines_after: u8,
    /// The layout the source gives the comment. Placement decides the
    /// layout the comment prints with.
    pub source_layout: CommentLayout,
    /// A block comment printed exactly as written, because its delimiters
    /// aren't the literal `/*` and `*/` that comment layout rewrites around:
    /// one that runs to the end of the file without `*/`, or one whose
    /// delimiters are written with Unicode escapes (`\u002a\u002f`).
    pub(crate) raw: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Placement<'a> {
    Leading(SyntaxElement<'a>, CommentLayout),
    Trailing(SyntaxElement<'a>, CommentLayout),
    Dangling(SyntaxNode<'a>),
}

/// What a language hook sees about one comment.
pub struct CommentContext<'a, 'p> {
    pub comment: Comment<'a>,
    pub prev: Option<SyntaxToken<'a>>,
    pub next: SyntaxToken<'a>,
    root: SyntaxNode<'a>,
    unclosed: &'p Unclosed,
}

impl<'a> CommentContext<'a, '_> {
    /// [`is_unclosed`], answered from a set computed once per tree.
    #[must_use]
    pub fn is_unclosed(&self, open: SyntaxToken<'_>) -> bool {
        self.unclosed.tokens.contains(open.id().index())
    }

    /// Whether one of `node`'s child tokens up to `token` [`is_unclosed`].
    #[must_use]
    pub fn has_unclosed_child_until(&self, node: SyntaxNode<'_>, token: SyntaxToken<'_>) -> bool {
        self.unclosed
            .first_in_node
            .get(node.id().index())
            .is_some_and(|&first| first <= token.id().index())
    }

    /// The deepest node containing both `prev` and `next` (the root when
    /// there is no `prev`).
    #[must_use]
    pub fn enclosing(&self) -> SyntaxNode<'a> {
        enclosing(self.root, self.prev, self.next)
    }
}

pub(crate) fn enclosing<'a>(
    root: SyntaxNode<'a>,
    prev: Option<SyntaxToken<'a>>,
    next: SyntaxToken<'a>,
) -> SyntaxNode<'a> {
    let Some(prev) = prev else {
        return root;
    };
    let mut node = next.parent();
    while node
        .first_token()
        .is_none_or(|first| first.id() > prev.id())
    {
        match node.parent() {
            Some(parent) => node = parent,
            None => break,
        }
    }
    node
}

/// `open` opens a region of its parent whose closer is missing (recovery):
/// no token of kind `L::closer_for(open)` follows it in its parent. Linear in
/// the parent's children; comment placement uses the per-tree set instead.
#[must_use]
pub fn is_unclosed<L: FormatLanguage>(open: SyntaxToken<'_>) -> bool {
    let mut unclosed = false;
    for_each_unclosed::<L>(open.parent(), |token| unclosed |= token == open);
    unclosed
}

/// The openers under `root` that are unclosed, and for each node the index
/// of its first unclosed child token (`usize::MAX` for none; empty when the
/// tree is recovery-free), found in one linear pass.
pub(crate) struct Unclosed {
    tokens: BitSet,
    first_in_node: Vec<usize>,
}

fn unclosed_openers<L: FormatLanguage>(root: SyntaxNode<'_>) -> Unclosed {
    let tree = root.tree();
    let mut unclosed = Unclosed {
        tokens: BitSet::new(tree.token_count()),
        first_in_node: Vec::new(),
    };
    if !root.is_recovery_free() {
        unclosed.first_in_node = vec![usize::MAX; tree.node_count()];
        for element in root.descendants() {
            if let SyntaxElement::Node(node) = element {
                // Reported last to first, so the first one stays.
                for_each_unclosed::<L>(node, |token| {
                    unclosed.tokens.insert(token.id().index());
                    unclosed.first_in_node[node.id().index()] = token.id().index();
                });
            }
        }
    }
    unclosed
}

/// Calls `f` with each opener among `node`'s child tokens that no token of
/// its closer's kind follows. None in a recovery-free node.
fn for_each_unclosed<'a, L: FormatLanguage>(
    node: SyntaxNode<'a>,
    mut f: impl FnMut(SyntaxToken<'a>),
) {
    if node.is_recovery_free() {
        return;
    }
    // Kinds seen after the current token; at most one entry per kind.
    let mut seen = Vec::new();
    for token in node.children().rev().filter_map(SyntaxElement::as_token) {
        if L::closer_for(token).is_some_and(|closer| !seen.contains(&closer)) {
            f(token);
        }
        if !seen.contains(&token.kind()) {
            seen.push(token.kind());
        }
    }
}

/// The outermost element other than the root that begins at `token`, never a
/// list node (a list's item on the path to `token` is chosen instead).
#[must_use]
pub fn leading_target<L: Language>(token: SyntaxToken<'_>) -> SyntaxElement<'_> {
    target::<L>(token, SyntaxNode::first_token, |_, _| false)
}

/// The outermost element other than the root that ends at `token`, never a
/// list node, and never a recovered node that is missing a piece after
/// `token` (unless it also begins there): its layout may put a line break
/// for that piece, which would separate the comment from `token`.
#[must_use]
pub fn trailing_target<L: Language>(token: SyntaxToken<'_>) -> SyntaxElement<'_> {
    target::<L>(token, SyntaxNode::last_token, |node, token| {
        !node.is_recovery_free()
            && node.first_token() != Some(token)
            && node.slots().next_back().is_some_and(|slot| {
                slot.is_none_or(|last| {
                    last.as_node()
                        .is_some_and(|last| last.first_token().is_none())
                })
            })
    })
}

fn target<'a, L: Language>(
    token: SyntaxToken<'a>,
    edge: fn(SyntaxNode<'a>) -> Option<SyntaxToken<'a>>,
    stop: fn(SyntaxNode<'a>, SyntaxToken<'a>) -> bool,
) -> SyntaxElement<'a> {
    let mut best = SyntaxElement::Token(token);
    let mut node = token.parent();
    while let Some(parent) = node.parent() {
        if edge(node) != Some(token) || stop(node, token) {
            break;
        }
        if !L::is_list(node.kind()) {
            best = SyntaxElement::Node(node);
        }
        node = parent;
    }
    best
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Side {
    Leading,
    Trailing,
    Dangling,
}

#[derive(Clone, Copy, Debug)]
struct Attachment {
    key: u64,
    side: Side,
    comment: u32,
}

struct BitSet(Vec<u64>);

impl BitSet {
    fn new(len: usize) -> Self {
        Self(vec![0; len.div_ceil(64)])
    }

    fn insert(&mut self, index: usize) {
        self.0[index / 64] |= 1 << (index % 64);
    }

    fn contains(&self, index: usize) -> bool {
        self.0[index / 64] & (1 << (index % 64)) != 0
    }
}

fn key(element: SyntaxElement<'_>) -> u64 {
    match element {
        SyntaxElement::Node(node) => (node.id().index() as u64) << 1,
        SyntaxElement::Token(token) => ((token.id().index() as u64) << 1) | 1,
    }
}

/// One comment with the tokens around it and the attachment placement gave
/// it.
#[derive(Clone, Copy, Debug)]
pub(crate) struct PlacedComment<'a> {
    pub(crate) comment: Comment<'a>,
    /// The token before the comment.
    pub(crate) prev: Option<SyntaxToken<'a>>,
    /// The token after the comment's gap (the comments between two tokens).
    pub(crate) next: SyntaxToken<'a>,
    pub(crate) side: Side,
    /// The effective layout, which placement may change from the source's.
    pub(crate) layout: CommentLayout,
}

/// Whether no comment of the same gap precedes comment `index`.
pub(crate) fn first_in_gap(comments: &[PlacedComment<'_>], index: usize) -> bool {
    index == 0 || comments[index - 1].next != comments[index].next
}

/// Every comment of a tree with its attachment.
pub(crate) struct Comments<'a> {
    /// In source order.
    pub(crate) list: Vec<PlacedComment<'a>>,
    attachments: Vec<Attachment>,
    nodes: BitSet,
    tokens: BitSet,
}

impl Comments<'_> {
    pub(crate) fn has(&self, element: SyntaxElement<'_>) -> bool {
        match element {
            SyntaxElement::Node(node) => self.nodes.contains(node.id().index()),
            SyntaxElement::Token(token) => self.tokens.contains(token.id().index()),
        }
    }

    /// Indices of the comments attached to `element` on `side`, in source
    /// order.
    pub(crate) fn attached(
        &self,
        element: SyntaxElement<'_>,
        side: Side,
    ) -> impl Iterator<Item = usize> + '_ {
        self.positions(element, side)
            .map(|position| self.at(position))
    }

    /// The positions in the attachment table of the comments attached to
    /// `element` on `side` (see [`Self::at`]), which borrow nothing.
    pub(crate) fn positions(&self, element: SyntaxElement<'_>, side: Side) -> Range<usize> {
        if !self.has(element) {
            return 0..0;
        }
        let key = (key(element), side);
        let start = self.attachments.partition_point(|a| (a.key, a.side) < key);
        let end = self.attachments.partition_point(|a| (a.key, a.side) <= key);
        start..end
    }

    /// The index of the comment at `position` in the attachment table.
    pub(crate) fn at(&self, position: usize) -> usize {
        self.attachments[position].comment as usize
    }

    /// Indices of the comments whose text lies inside `range`.
    pub(crate) fn within(&self, range: TextRange) -> Range<usize> {
        let start = self
            .list
            .partition_point(|c| c.comment.range.start() < range.start());
        let end = self
            .list
            .partition_point(|c| c.comment.range.end() <= range.end());
        start..end.max(start)
    }
}

const fn cap(newlines: u32) -> u8 {
    match newlines {
        0 => 0,
        1 => 1,
        _ => 2,
    }
}

pub(crate) const fn is_comment(kind: TriviaKind) -> bool {
    matches!(
        kind,
        TriviaKind::LineComment
            | TriviaKind::BlockComment
            | TriviaKind::DocComment
            | TriviaKind::UnterminatedBlockComment
    )
}

/// A comment found in the trivia, with the tokens around its gap.
#[derive(Clone, Copy)]
struct Found<'a> {
    piece: jolt_syntax::TriviaPiece<'a>,
    lines_before: u8,
    lines_after: u8,
    prev: Option<SyntaxToken<'a>>,
    next: SyntaxToken<'a>,
}

impl<'a> Found<'a> {
    fn comment(self, source_layout: CommentLayout) -> Comment<'a> {
        let piece = self.piece;
        Comment {
            text: piece.text,
            range: piece.range,
            kind: match piece.kind {
                TriviaKind::LineComment => CommentKind::Line,
                TriviaKind::DocComment => CommentKind::Doc,
                _ => CommentKind::Block,
            },
            lines_before: self.lines_before,
            lines_after: self.lines_after,
            source_layout,
            raw: piece.kind == TriviaKind::UnterminatedBlockComment
                || (piece.kind != TriviaKind::LineComment
                    && !(piece.text.starts_with("/*") && piece.text.ends_with("*/"))),
        }
    }
}

/// Whether no comment of the same gap follows comment `index`.
fn last_in_gap(found: &[Found<'_>], index: usize) -> bool {
    found
        .get(index + 1)
        .is_none_or(|after| after.next != found[index].next)
}

struct Collector<'a> {
    list: Vec<Found<'a>>,
    newlines: u32,
    /// The comment whose gap to what follows is still being counted.
    open: Option<usize>,
}

impl<'a> Collector<'a> {
    fn trivia(
        &mut self,
        piece: jolt_syntax::TriviaPiece<'a>,
        prev: Option<SyntaxToken<'a>>,
        next: SyntaxToken<'a>,
    ) {
        if piece.kind == TriviaKind::Newline {
            self.newlines += 1;
            return;
        }
        if !is_comment(piece.kind) {
            return;
        }
        let lines_before = cap(self.newlines);
        self.close();
        self.open = Some(self.list.len());
        self.list.push(Found {
            piece,
            lines_before,
            lines_after: 0,
            prev,
            next,
        });
    }

    /// Ends the gap after the open comment (at a token or the next comment).
    fn close(&mut self) {
        if let Some(index) = self.open.take() {
            self.list[index].lines_after = cap(self.newlines);
        }
        self.newlines = 0;
    }
}

/// Every comment under `root` with the tokens around it, in source order.
fn collect(root: SyntaxNode<'_>) -> Vec<Found<'_>> {
    let mut collector = Collector {
        list: Vec::new(),
        newlines: 0,
        open: None,
    };
    let mut prev = None;
    for token in root.tokens() {
        for piece in token.leading() {
            collector.trivia(piece, prev, token);
        }
        collector.close();
        let next = token.next().unwrap_or(token);
        for piece in token.trailing() {
            collector.trivia(piece, Some(token), next);
        }
        prev = Some(token);
    }
    collector.close();
    collector.list
}

/// Classifies and places every comment under `root`.
pub(crate) fn place<'a, L: FormatLanguage>(root: SyntaxNode<'a>) -> Comments<'a> {
    let found = collect(root);
    let source_layouts = source_layouts::<L::Syntax>(&found);

    let mut list: Vec<PlacedComment<'a>> = Vec::with_capacity(found.len());
    let mut attachments = Vec::with_capacity(found.len());
    let tree = root.tree();
    let mut previous: Option<Placement<'a>> = None;
    let mut nodes = BitSet::new(tree.node_count());
    let mut tokens = BitSet::new(tree.token_count());
    let unclosed = unclosed_openers::<L>(root);
    for (index, (&found_comment, &source_layout)) in found.iter().zip(&source_layouts).enumerate() {
        let Found { prev, next, .. } = found_comment;
        let comment = found_comment.comment(source_layout);
        let context = CommentContext {
            comment,
            prev,
            next,
            root,
            unclosed: &unclosed,
        };
        let first_in_gap = index == 0 || found[index - 1].next != next;
        let last_in_gap = last_in_gap(&found, index);
        let mut placement = match L::place_comment(&context) {
            Some(placement) => {
                debug_assert_hook_placement(&context, placement);
                placement
            }
            None => generic_placement::<L>(&context, source_layout, first_in_gap, last_in_gap),
        };
        // The placement of the comment before this one in its gap.
        let earlier = previous.filter(|_| !first_in_gap);
        // A comment never moves before an earlier comment of its gap: once
        // one leads the next element (or dangles), the rest follow it.
        if let (Placement::Trailing(..), Some(earlier)) = (placement, earlier) {
            placement = match earlier {
                Placement::Leading(element, _) => Placement::Leading(element, source_layout),
                Placement::Dangling(node) => Placement::Dangling(node),
                Placement::Trailing(..) => placement,
            };
        }
        // Nothing can follow a trailing line comment on its line (S1 can put
        // a comment after it), so the next trailing comment of the same
        // element is placed as if it were on its own line: it leads what
        // follows, where the next pass puts it.
        if let (Placement::Trailing(element, _), Some(Placement::Trailing(earlier, _))) =
            (placement, previous)
            && earlier == element
            && list[index - 1].comment.kind == CommentKind::Line
        {
            placement =
                generic_placement::<L>(&context, CommentLayout::OwnLine, first_in_gap, last_in_gap);
        }
        previous = Some(placement);
        let (element, side, layout) = match placement {
            Placement::Leading(element, layout) => (element, Side::Leading, layout),
            Placement::Trailing(element, layout) => (element, Side::Trailing, layout),
            Placement::Dangling(node) => (SyntaxElement::Node(node), Side::Dangling, source_layout),
        };
        match element {
            SyntaxElement::Node(node) => nodes.insert(node.id().index()),
            SyntaxElement::Token(token) => tokens.insert(token.id().index()),
        }
        list.push(PlacedComment {
            comment,
            prev,
            next,
            side,
            layout,
        });
        attachments.push(Attachment {
            key: key(element),
            side,
            comment: u32::try_from(index).expect("comment count fits in u32"),
        });
    }
    attachments.sort_unstable_by_key(|a| (a.key, a.side, a.comment));
    Comments {
        list,
        attachments,
        nodes,
        tokens,
    }
}

/// The layout the source gives each comment, from the line breaks around it.
/// A comment with code before it on its line that is followed only by other
/// comments up to the line's end ends its line too: `/* a */` in
/// `x; /* a */ // b` trails `x;` like `// b`.
fn source_layouts<L: Language>(found: &[Found<'_>]) -> Vec<CommentLayout> {
    let eof = L::to_raw(L::EOF);
    let mut layouts = Vec::with_capacity(found.len());
    // Whether the line ends after the next comment of the gap, past any
    // comments after that one.
    let mut next_line_ends = false;
    for (index, comment) in found.iter().enumerate().rev() {
        let last_in_gap = last_in_gap(found, index);
        let ends_file = last_in_gap && comment.next.kind() == eof;
        // Whether the line ends after this comment, past any comments after it.
        let line_ends = comment.lines_after > 0
            || if last_in_gap {
                ends_file
            } else {
                next_line_ends
            };
        // The file start counts as a line break before the first comment.
        let newline_before = (index == 0 && comment.prev.is_none()) || comment.lines_before > 0;
        let newline_after = comment.lines_after > 0 || ends_file || (!newline_before && line_ends);
        layouts.push(match (newline_before, newline_after) {
            (true, true) => CommentLayout::OwnLine,
            (true, false) => CommentLayout::StartOfLine,
            (false, true) => CommentLayout::EndOfLine,
            (false, false) => CommentLayout::Inline,
        });
        next_line_ends = line_ends;
    }
    layouts.reverse();
    layouts
}

/// The generic placement of the comment in `context`, as if the source gave
/// it `layout`.
fn generic_placement<'a, L: FormatLanguage>(
    context: &CommentContext<'a, '_>,
    layout: CommentLayout,
    first_in_gap: bool,
    last_in_gap: bool,
) -> Placement<'a> {
    let CommentContext {
        comment,
        prev,
        next,
        ..
    } = *context;
    let syntax_lead = |token| leading_target::<L::Syntax>(token);
    let syntax_trail = |token| trailing_target::<L::Syntax>(token);
    // `prev` opens a region whose closer is present. After an unclosed
    // opener the next token may lie past the missing closer (a method body
    // after `void m(`), so the opener rules below do not apply.
    let mut opens_region = false;
    if let Some(prev) = prev
        && let Some(closer) = L::closer_for(prev)
    {
        // G1: a comment alone in an empty delimited region.
        if next.kind() == closer && next.parent() == prev.parent() {
            return Placement::Dangling(prev.parent());
        }
        opens_region = !context.is_unclosed(prev);
        // G2: an end-of-line comment right after an opener starts the inside
        // (also when only the end of the file follows an unclosed opener).
        let eof = next.kind() == <L::Syntax as Language>::to_raw(<L::Syntax as Language>::EOF);
        if layout == CommentLayout::EndOfLine && first_in_gap && (opens_region || eof) {
            return Placement::Leading(syntax_lead(next), CommentLayout::OwnLine);
        }
    }
    let own_line = matches!(layout, CommentLayout::OwnLine | CommentLayout::StartOfLine);
    // G3: a comment on its own line before a closer, inside the closer's
    // node (a closer that begins its node, as in a stray `}`, leads it).
    if own_line && L::is_closer(next) && next.parent().first_token() != Some(next) {
        return Placement::Dangling(next.parent());
    }
    // S1: an end-of-line line comment moves past the separator that follows
    // it (unless it already follows one).
    if layout == CommentLayout::EndOfLine
        && comment.kind == CommentKind::Line
        && last_in_gap
        && L::is_separator(next)
        && !prev.is_some_and(L::is_separator)
    {
        return Placement::Trailing(syntax_trail(next), CommentLayout::EndOfLine);
    }
    let prev = prev.filter(|_| !own_line);
    match (layout, prev) {
        (CommentLayout::EndOfLine, Some(prev)) => {
            Placement::Trailing(syntax_trail(prev), CommentLayout::EndOfLine)
        }
        // An inline comment before a closer or separator trails the code
        // before it, unless that is an opener: a separator after an opener
        // may be removed, which would leave the comment alone in the region.
        (CommentLayout::Inline, Some(prev))
            if (L::is_closer(next) || L::is_separator(next)) && !opens_region =>
        {
            Placement::Trailing(syntax_trail(prev), CommentLayout::Inline)
        }
        _ => Placement::Leading(syntax_lead(next), layout),
    }
}

fn debug_assert_hook_placement(context: &CommentContext<'_, '_>, placement: Placement<'_>) {
    if !cfg!(debug_assertions) {
        return;
    }
    let (element, valid) = match placement {
        Placement::Leading(element, layout) => (element, layout != CommentLayout::EndOfLine),
        Placement::Trailing(element, layout) => (
            element,
            matches!(layout, CommentLayout::EndOfLine | CommentLayout::Inline),
        ),
        Placement::Dangling(node) => (SyntaxElement::Node(node), true),
    };
    assert!(valid, "comment hook chose an invalid layout: {placement:?}");
    let enclosing = context.enclosing();
    let range = enclosing.text_range();
    let target = element.text_range();
    assert!(
        range.start() <= target.start() && target.end() <= range.end(),
        "comment hook targeted {element:?} outside the enclosing {enclosing:?}"
    );
}
