//! Comment classification and placement: every comment is attached exactly
//! once, as leading or trailing to an element or dangling in a node.

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
    /// The layout the source gives the comment.
    pub layout: CommentLayout,
}

impl Comment<'_> {
    pub(crate) fn is_unterminated(&self) -> bool {
        self.kind != CommentKind::Line && !(self.text.len() >= 4 && self.text.ends_with("*/"))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Placement<'a> {
    Leading(SyntaxElement<'a>, CommentLayout),
    Trailing(SyntaxElement<'a>, CommentLayout),
    Dangling(SyntaxNode<'a>),
}

/// What a language hook sees about one comment.
pub struct CommentContext<'a> {
    pub comment: Comment<'a>,
    pub prev: Option<SyntaxToken<'a>>,
    pub next: SyntaxToken<'a>,
    root: SyntaxNode<'a>,
}

impl<'a> CommentContext<'a> {
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
/// no token of kind `L::closer_for(open)` follows it in its parent.
#[must_use]
pub fn is_unclosed<L: FormatLanguage>(open: SyntaxToken<'_>) -> bool {
    let parent = open.parent();
    L::closer_for(open).is_some_and(|closer| {
        !parent.is_recovery_free()
            && !parent
                .children()
                .filter_map(SyntaxElement::as_token)
                .any(|token| token.kind() == closer && token.id() > open.id())
    })
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

pub(crate) struct BitSet(Vec<u64>);

impl BitSet {
    fn new(len: usize) -> Self {
        Self(vec![0; len.div_ceil(64)])
    }

    fn insert(&mut self, index: usize) {
        self.0[index / 64] |= 1 << (index % 64);
    }

    pub(crate) fn contains(&self, index: usize) -> bool {
        self.0[index / 64] & (1 << (index % 64)) != 0
    }
}

pub(crate) fn key(element: SyntaxElement<'_>) -> u64 {
    match element {
        SyntaxElement::Node(node) => (node.id().index() as u64) << 1,
        SyntaxElement::Token(token) => ((token.id().index() as u64) << 1) | 1,
    }
}

/// Every comment of a tree with its attachment.
pub(crate) struct Comments<'a> {
    pub(crate) list: Vec<Comment<'a>>,
    /// The token before each comment.
    pub(crate) prev: Vec<Option<SyntaxToken<'a>>>,
    pub(crate) next: Vec<SyntaxToken<'a>>,
    /// Each comment's attachment side and effective layout.
    pub(crate) attached: Vec<(Side, CommentLayout)>,
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
        let range = if self.has(element) {
            let key = key(element);
            let start = self
                .attachments
                .partition_point(|a| (a.key, a.side) < (key, side));
            let end = self
                .attachments
                .partition_point(|a| (a.key, a.side) <= (key, side));
            start..end
        } else {
            0..0
        };
        self.attachments[range].iter().map(|a| a.comment as usize)
    }

    /// Indices of the comments whose text lies inside `range`.
    pub(crate) fn within(&self, range: TextRange) -> std::ops::Range<usize> {
        let start = self
            .list
            .partition_point(|c| c.range.start() < range.start());
        let end = self.list.partition_point(|c| c.range.end() <= range.end());
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

const fn is_comment(kind: TriviaKind) -> bool {
    matches!(
        kind,
        TriviaKind::LineComment
            | TriviaKind::BlockComment
            | TriviaKind::DocComment
            | TriviaKind::UnterminatedBlockComment
    )
}

struct Collector<'a> {
    list: Vec<Comment<'a>>,
    prev: Vec<Option<SyntaxToken<'a>>>,
    next: Vec<SyntaxToken<'a>>,
    newlines: u32,
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
        self.list.push(Comment {
            text: piece.text,
            range: piece.range,
            kind: match piece.kind {
                TriviaKind::LineComment => CommentKind::Line,
                TriviaKind::DocComment => CommentKind::Doc,
                _ => CommentKind::Block,
            },
            lines_before,
            lines_after: 0,
            layout: CommentLayout::Inline,
        });
        self.prev.push(prev);
        self.next.push(next);
        self.newlines = 0;
    }

    /// Ends the gap after the open comment (at a token or the next comment).
    fn close(&mut self) {
        if let Some(index) = self.open.take() {
            self.list[index].lines_after = cap(self.newlines);
        }
        self.newlines = 0;
    }
}

/// Classifies and places every comment under `root`.
pub(crate) fn place<'a, L: FormatLanguage>(root: SyntaxNode<'a>) -> Comments<'a> {
    let mut collector = Collector {
        list: Vec::new(),
        prev: Vec::new(),
        next: Vec::new(),
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
    let Collector {
        mut list,
        prev,
        next,
        ..
    } = collector;

    classify_layouts::<L::Syntax>(&mut list, &prev, &next);

    let mut attached = Vec::with_capacity(list.len());
    let mut attachments = Vec::with_capacity(list.len());
    let tree = root.tree();
    let mut previous: Option<(Placement<'a>, SyntaxToken<'a>)> = None;
    let mut nodes = BitSet::new(tree.node_count());
    let mut tokens = BitSet::new(tree.token_count());
    for index in 0..list.len() {
        let context = CommentContext {
            comment: list[index],
            prev: prev[index],
            next: next[index],
            root,
        };
        let last_before_next = next.get(index + 1) != Some(&next[index]);
        let first_in_gap = index == 0 || next[index - 1] != next[index];
        let mut placement = match L::place_comment(&context) {
            Some(placement) => {
                debug_assert_hook_placement(&context, placement);
                placement
            }
            None => generic_placement::<L>(&context, first_in_gap, last_before_next),
        };
        // A comment never moves before an earlier comment of its gap: once
        // one leads the next element (or dangles), the rest follow it.
        if let (Placement::Trailing(..), Some(&(earlier, earlier_next))) =
            (placement, previous.as_ref())
            && earlier_next == next[index]
        {
            placement = match earlier {
                Placement::Leading(element, _) => Placement::Leading(element, list[index].layout),
                Placement::Dangling(node) => Placement::Dangling(node),
                Placement::Trailing(..) => placement,
            };
        }
        // Nothing can follow a trailing line comment on its line (S1 can put
        // a comment after it), so the next trailing comment of the same
        // element leads what follows instead, where the next pass puts it.
        if let (Placement::Trailing(element, _), Some(&(Placement::Trailing(earlier, _), _))) =
            (placement, previous.as_ref())
            && earlier == element
            && index > 0
            && list[index - 1].kind == CommentKind::Line
        {
            let mut own_line = context;
            own_line.comment.layout = CommentLayout::OwnLine;
            placement = generic_placement::<L>(&own_line, first_in_gap, last_before_next);
        }
        previous = Some((placement, next[index]));
        let (element, side, layout) = match placement {
            Placement::Leading(element, layout) => (element, Side::Leading, layout),
            Placement::Trailing(element, layout) => (element, Side::Trailing, layout),
            Placement::Dangling(node) => (
                SyntaxElement::Node(node),
                Side::Dangling,
                list[index].layout,
            ),
        };
        match element {
            SyntaxElement::Node(node) => nodes.insert(node.id().index()),
            SyntaxElement::Token(token) => tokens.insert(token.id().index()),
        }
        attached.push((side, layout));
        attachments.push(Attachment {
            key: key(element),
            side,
            comment: u32::try_from(index).expect("comment count fits in u32"),
        });
    }
    attachments.sort_unstable_by_key(|a| (a.key, a.side, a.comment));
    Comments {
        list,
        prev,
        next,
        attached,
        attachments,
        nodes,
        tokens,
    }
}

/// Decides each comment's layout from the line breaks around it. A comment
/// with code before it on its line that is followed only by other comments
/// up to the line's end ends its line too: `/* a */` in `x; /* a */ // b`
/// trails `x;` like `// b`.
fn classify_layouts<L: Language>(
    list: &mut [Comment<'_>],
    prev: &[Option<SyntaxToken<'_>>],
    next: &[SyntaxToken<'_>],
) {
    let eof = L::to_raw(L::EOF);
    // Whether the line ends after the comment, past any comments after it.
    let mut line_ends = vec![false; list.len()];
    for index in (0..list.len()).rev() {
        let last_before_next = next.get(index + 1) != Some(&next[index]);
        line_ends[index] = list[index].lines_after > 0
            || if last_before_next {
                next[index].kind() == eof
            } else {
                line_ends[index + 1]
            };
    }
    for index in 0..list.len() {
        let comment = &mut list[index];
        let last_before_next = next.get(index + 1) != Some(&next[index]);
        // The file start counts as a line break before the first comment.
        let newline_before = (index == 0 && prev[index].is_none()) || comment.lines_before > 0;
        let newline_after = comment.lines_after > 0
            || (last_before_next && next[index].kind() == eof)
            || (!newline_before && line_ends[index]);
        comment.layout = match (newline_before, newline_after) {
            (true, true) => CommentLayout::OwnLine,
            (true, false) => CommentLayout::StartOfLine,
            (false, true) => CommentLayout::EndOfLine,
            (false, false) => CommentLayout::Inline,
        };
    }
}

fn generic_placement<'a, L: FormatLanguage>(
    context: &CommentContext<'a>,
    first_in_gap: bool,
    last_before_next: bool,
) -> Placement<'a> {
    let CommentContext {
        comment,
        prev,
        next,
        ..
    } = *context;
    let layout = comment.layout;
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
        opens_region = !is_unclosed::<L>(prev);
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
        && last_before_next
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

fn debug_assert_hook_placement(context: &CommentContext<'_>, placement: Placement<'_>) {
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
