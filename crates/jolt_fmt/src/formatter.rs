//! The `Formatter` context: the chokepoints through which rules emit source,
//! comment printing, verbatim output, and item lists with formatter-ignore.

use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

use jolt_syntax::{SyntaxElement, SyntaxNode, SyntaxToken, TextRange, TextSize, TriviaKind};

use crate::comments::{self, CommentKind, CommentLayout, Comments, Side};
use crate::doc::{LINE_COMMENT, Reindent, VERBATIM_START, has_line_break};
use crate::{Doc, DocBuilder, FormatLanguage, cat};

/// One item of an [`Formatter::item_list`].
#[derive(Clone, Copy, Debug)]
pub enum ListItem<'a> {
    /// Formatted with [`Formatter::node`].
    Format(SyntaxNode<'a>),
    /// Removed by a normalization: only its comments are printed.
    Elide(SyntaxNode<'a>),
}

/// What a list separator callback sees: the items around the separator, and
/// their positions in the list given to [`Formatter::item_list`].
#[derive(Clone, Copy, Debug)]
pub struct Separator<'a> {
    pub prev: SyntaxNode<'a>,
    pub next: SyntaxNode<'a>,
    pub prev_index: usize,
    pub next_index: usize,
    pub blank_line_in_source: bool,
}

pub struct Formatter<'a, L: FormatLanguage> {
    builder: DocBuilder<'a>,
    comments: Comments<'a>,
    /// The formatter-ignore regions, sorted and disjoint.
    regions: Vec<TextRange>,
    emitted: Vec<u8>,
    by_verbatim: Vec<bool>,
    language: PhantomData<L>,
}

impl<'a, L: FormatLanguage> Deref for Formatter<'a, L> {
    type Target = DocBuilder<'a>;

    fn deref(&self) -> &Self::Target {
        &self.builder
    }
}

impl<L: FormatLanguage> DerefMut for Formatter<'_, L> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.builder
    }
}

impl<'a, L: FormatLanguage> Formatter<'a, L> {
    pub(crate) fn new(root: SyntaxNode<'a>) -> Self {
        let comments = comments::place::<L>(root);
        let regions = ignore_regions::<L>(root, &comments);
        let count = comments.list.len();
        Self {
            builder: DocBuilder::with_capacity(root.tree().token_count()),
            comments,
            regions,
            emitted: vec![0; count],
            by_verbatim: vec![false; count],
            language: PhantomData,
        }
    }

    pub(crate) fn into_builder(self) -> DocBuilder<'a> {
        self.builder
    }

    /// Panics listing every comment not emitted exactly once.
    pub(crate) fn assert_emitted(&self) {
        let wrong: Vec<String> = self
            .emitted
            .iter()
            .enumerate()
            .filter(|&(_, &count)| count != 1)
            .map(|(index, count)| {
                let comment = &self.comments.list[index].comment;
                format!(
                    "  {:?} at {} emitted {count} times",
                    comment.text, comment.range
                )
            })
            .collect();
        assert!(
            wrong.is_empty(),
            "comments not emitted exactly once:\n{}",
            wrong.join("\n")
        );
    }

    // ── The chokepoints ──

    /// Leading comments, then the node (verbatim when malformed), then
    /// trailing comments. The only caller of `L::format_node`.
    pub fn node(&mut self, node: SyntaxNode<'a>) -> Doc {
        let body = if node.is_malformed() {
            self.verbatim(node)
        } else {
            L::format_node(self, node)
        };
        self.with_comments(SyntaxElement::Node(node), body)
    }

    pub fn fmt<N: jolt_syntax::AstNode<'a>>(&mut self, node: N) -> Doc {
        self.node(node.syntax())
    }

    pub fn fmt_opt<N: jolt_syntax::AstNode<'a>>(&mut self, node: Option<N>) -> Doc {
        node.map_or(Doc::NIL, |node| self.fmt(node))
    }

    /// Leading comments, the token text, trailing comments.
    pub fn token(&mut self, token: SyntaxToken<'a>) -> Doc {
        let text = self.builder.text(token.text());
        self.with_comments(SyntaxElement::Token(token), text)
    }

    pub fn token_opt(&mut self, token: Option<SyntaxToken<'a>>) -> Doc {
        token.map_or(Doc::NIL, |token| self.token(token))
    }

    /// [`node`](Self::node) or [`token`](Self::token), whichever `element`
    /// is.
    pub fn element(&mut self, element: SyntaxElement<'a>) -> Doc {
        match element {
            SyntaxElement::Node(node) => self.node(node),
            SyntaxElement::Token(token) => self.token(token),
        }
    }

    pub fn element_opt(&mut self, element: Option<SyntaxElement<'a>>) -> Doc {
        element.map_or(Doc::NIL, |element| self.element(element))
    }

    /// The element's leading and trailing comments around `body`.
    fn with_comments(&mut self, element: SyntaxElement<'a>, body: Doc) -> Doc {
        if !self.comments.has(element) {
            return body;
        }
        let leading = self.comments_on(element, Side::Leading);
        let trailing = self.comments_on(element, Side::Trailing);
        cat![self; leading, body, trailing]
    }

    pub fn leading_comments(&mut self, node: SyntaxNode<'a>) -> Doc {
        self.comments_on(SyntaxElement::Node(node), Side::Leading)
    }

    pub fn trailing_comments(&mut self, node: SyntaxNode<'a>) -> Doc {
        self.comments_on(SyntaxElement::Node(node), Side::Trailing)
    }

    /// Every comment attached to or inside `element`, in source order, without
    /// the element's text. NIL when there are none.
    pub fn removed(&mut self, element: SyntaxElement<'a>) -> Doc {
        let (before, after, _) = self.removed_parts(element);
        cat![self; before, after]
    }

    /// `removed` for several elements, for comments a normalization moves to
    /// lines of their own (those of an enum's moved `;`): in source order,
    /// printed like dangling comments, which is how the next pass reads them.
    pub fn removed_on_own_lines(&mut self, elements: &[SyntaxElement<'a>]) -> Doc {
        let mut indices: Vec<usize> = elements
            .iter()
            .flat_map(|&element| self.removed_indices(element))
            .collect();
        indices.sort_unstable();
        self.own_lines(&indices)
    }

    /// The comments attached to or inside `element`, in source order.
    fn removed_indices(&self, element: SyntaxElement<'a>) -> Vec<usize> {
        let mut indices: Vec<usize> = self
            .comments
            .attached(element, Side::Leading)
            .chain(self.comments.attached(element, Side::Trailing))
            .collect();
        if let SyntaxElement::Node(node) = element {
            indices.extend(self.comments.within(node.text_range()));
        }
        indices.sort_unstable();
        indices
    }

    /// `removed`, split into the trailing comments that sit before the
    /// element in the source (moved onto it by S1, so they end the previous
    /// line) and the rest, and whether the source has a blank line after the
    /// last comment (which, with the element gone, separates the comments
    /// from what follows).
    fn removed_parts(&mut self, element: SyntaxElement<'a>) -> (Doc, Doc, bool) {
        let indices = self.removed_indices(element);
        let start = element.text_range().start();
        let mut before = Vec::new();
        let mut after = Vec::new();
        let last = indices.last().copied();
        let blank_after =
            last.is_some_and(|last| self.comments.list[last].comment.lines_after >= 2);
        for index in indices {
            let placed = self.comments.list[index];
            // The last comment has no element left to separate it from.
            let doc = if Some(index) == last
                && (placed.side, placed.layout) == (Side::Leading, CommentLayout::OwnLine)
            {
                let text = self.comment_doc(index);
                cat![self; Doc::HARD_LINE, text]
            } else {
                self.attached_comment(index)
            };
            if placed.side == Side::Trailing && placed.comment.range.end() <= start {
                before.push(doc);
            } else {
                after.push(doc);
            }
        }
        (
            self.builder.concat(before),
            self.builder.concat(after),
            blank_after,
        )
    }

    /// The node's source from its first to its last token, interior comments
    /// included, re-indented relative to the output line it starts on. Text
    /// blocks (multi-line tokens) are written raw. Comments attached to
    /// elements inside the node but lying outside its text (trailing its last
    /// token, or moved onto its first by S1) print around it, as the node's
    /// own formatting would have printed them.
    pub fn verbatim(&mut self, node: SyntaxNode<'a>) -> Doc {
        let range = node.text_range();
        if node.last_token().is_none() || range.is_empty() {
            return Doc::NIL;
        }
        let inside = self.comments.within(range);
        for index in inside.clone() {
            self.emitted[index] += 1;
            self.by_verbatim[index] = true;
        }
        let source = node.source();
        let prefix = line_indentation(source, range.start());
        // Text blocks are written raw. A multi-line comment with a line that
        // does not start with the node's indentation, which stripping it
        // would shift relative to the comment's other lines, is re-indented
        // as a comment.
        let mut apart: Vec<(TextRange, Option<Reindent<'a>>)> = node
            .tokens()
            .filter(|token| has_line_break(token.text()))
            .map(|token| (token.text_range(), None))
            .collect();
        apart.extend(
            inside
                .map(|index| self.comments.list[index].comment)
                .filter(|comment| !comment.unterminated && lines_outside(comment.text, prefix))
                .map(|comment| (comment.range, Some(Reindent::Comment))),
        );
        apart.sort_unstable_by_key(|(range, _)| range.start());
        let mut docs = Vec::new();
        let mut cursor = range.start();
        let mut started = false;
        let mut piece = |f: &mut Self, text: &'a str, reindent: Option<Reindent<'a>>| {
            let flags = if started { 0 } else { VERBATIM_START };
            started |= !text.is_empty();
            f.builder.text_with(text, reindent, flags)
        };
        for (span, reindent) in apart {
            let text = &source[cursor.get()..span.start().get()];
            docs.push(piece(self, text, Some(Reindent::Strip(prefix))));
            let text = &source[span.start().get()..span.end().get()];
            docs.push(piece(self, text, reindent));
            cursor = span.end();
        }
        let text = &source[cursor.get()..range.end().get()];
        docs.push(piece(self, text, Some(Reindent::Strip(prefix))));
        // Comments attached inside the node but lying outside its text: the
        // node's own formatting would have printed them, before or after.
        let (mut before, mut after) = (Vec::new(), Vec::new());
        for element in node.descendants().skip(1) {
            for side in [Side::Leading, Side::Dangling, Side::Trailing] {
                for index in self.comments.attached(element, side) {
                    if self.by_verbatim[index] {
                        continue;
                    }
                    if self.comments.list[index].comment.range.start() < range.start() {
                        before.push(index);
                    } else {
                        after.push(index);
                    }
                }
            }
        }
        before.sort_unstable();
        after.sort_unstable();
        let mut all: Vec<Doc> = before
            .into_iter()
            .map(|index| self.attached_comment(index))
            .collect();
        all.append(&mut docs);
        all.extend(after.into_iter().map(|index| self.attached_comment(index)));
        self.builder.concat(all)
    }

    /// The node's dangling comments, each on its own line (comments that
    /// shared a source line share it), with no line before the first or after
    /// the last. NIL when none.
    pub fn dangling(&mut self, node: SyntaxNode<'a>) -> Doc {
        let indices: Vec<usize> = self
            .comments
            .attached(SyntaxElement::Node(node), Side::Dangling)
            .filter(|&index| !self.by_verbatim[index])
            .collect();
        self.own_lines(&indices)
    }

    /// Comments each on its own line (comments that shared a source line
    /// share it), with no line before the first or after the last.
    fn own_lines(&mut self, indices: &[usize]) -> Doc {
        let mut docs = Vec::with_capacity(indices.len() * 2 + 1);
        for (position, &index) in indices.iter().enumerate() {
            docs.push(if position == 0 {
                Doc::BREAK_PARENT
            } else {
                match self.comments.list[index].comment.lines_before {
                    0 => Doc::SPACE,
                    1 => Doc::HARD_LINE,
                    _ => Doc::EMPTY_LINE,
                }
            });
            // Dangling comments always print on their own lines.
            docs.push(self.comment_doc(index));
        }
        self.builder.concat(docs)
    }

    /// `inner`, then `owner`'s dangling comments on their own lines after it,
    /// separated by a blank line when the source had one there. `inner`
    /// alone when there are none, the comments alone when `inner` is NIL.
    pub fn with_dangling(&mut self, owner: SyntaxNode<'a>, inner: Doc) -> Doc {
        if !self.comments.has(SyntaxElement::Node(owner)) {
            return inner;
        }
        let dangling = self.dangling(owner);
        if dangling.is_nil() {
            return inner;
        }
        if inner.is_nil() {
            return dangling;
        }
        let blank = self
            .comments
            .attached(SyntaxElement::Node(owner), Side::Dangling)
            .next()
            .is_some_and(|index| self.comments.list[index].comment.lines_before >= 2);
        let line = self.line_break(blank);
        cat![self; inner, line, dangling]
    }

    // ── Queries ──

    /// Any comment attached to the element: leading, trailing or dangling.
    #[must_use]
    pub fn has_comments(&self, element: SyntaxElement<'a>) -> bool {
        self.comments.has(element)
    }

    #[must_use]
    pub fn has_leading_comments(&self, element: SyntaxElement<'a>) -> bool {
        self.comments
            .attached(element, Side::Leading)
            .next()
            .is_some()
    }

    #[must_use]
    pub fn has_trailing_comments(&self, element: SyntaxElement<'a>) -> bool {
        self.comments
            .attached(element, Side::Trailing)
            .next()
            .is_some()
    }

    /// The element has a trailing line comment, which ends its line.
    #[must_use]
    pub fn has_trailing_line_comment(&self, element: SyntaxElement<'a>) -> bool {
        self.comments
            .attached(element, Side::Trailing)
            .any(|index| self.comments.list[index].comment.kind == CommentKind::Line)
    }

    /// Some leading comment of the element prints a line break between the
    /// code before it and the element: one on its own line, starting its
    /// line, or ending its line.
    #[must_use]
    pub fn leading_comments_break(&self, element: SyntaxElement<'a>) -> bool {
        self.comments
            .attached(element, Side::Leading)
            .any(|index| self.comments.list[index].layout != CommentLayout::Inline)
    }

    /// Some trailing comment of the element ends its line.
    #[must_use]
    pub fn trailing_comments_break(&self, element: SyntaxElement<'a>) -> bool {
        self.comments
            .attached(element, Side::Trailing)
            .any(|index| self.comments.list[index].layout != CommentLayout::Inline)
    }

    #[must_use]
    pub fn has_dangling_comments(&self, node: SyntaxNode<'a>) -> bool {
        self.comments
            .attached(SyntaxElement::Node(node), Side::Dangling)
            .next()
            .is_some()
    }

    /// Any comment attached to the node or located inside its range.
    #[must_use]
    pub fn has_comments_within(&self, node: SyntaxNode<'a>) -> bool {
        self.comments.has(SyntaxElement::Node(node))
            || !self.comments.within(node.text_range()).is_empty()
    }

    /// The source has a blank line before the element's first leading comment,
    /// or before its first token when it has none.
    #[must_use]
    pub fn blank_line_before(&self, element: SyntaxElement<'a>) -> bool {
        if let Some(first) = self.comments.attached(element, Side::Leading).next() {
            return self.comments.list[first].comment.lines_before >= 2;
        }
        let Some(token) = element.first_token() else {
            return false;
        };
        let mut newlines = 0;
        for piece in token.leading() {
            if piece.kind == TriviaKind::Newline {
                newlines += 1;
            } else if comments::is_comment(piece.kind) {
                newlines = 0;
            }
        }
        newlines >= 2
    }

    /// Whether `node` is an item inside a formatter-ignore region, so a
    /// normalization must leave it in place.
    #[must_use]
    pub fn in_ignore_region(&self, node: SyntaxNode<'a>) -> bool {
        let Some(start) = node.first_token().map(|token| token.text_range().start()) else {
            return false;
        };
        let index = self.regions.partition_point(|region| region.end() <= start);
        self.regions
            .get(index)
            .is_some_and(|region| region.start() <= start)
    }

    // ── Item lists ──

    /// Items separated by `separator`, with elision and formatter-ignore.
    pub fn item_list(
        &mut self,
        items: impl IntoIterator<Item = ListItem<'a>>,
        mut separator: impl FnMut(&mut Self, Separator<'a>) -> Doc,
    ) -> Doc {
        let mut docs = Vec::new();
        // The last emitted item, its index, and whether it is ignored.
        let mut prev: Option<(SyntaxNode<'a>, usize, bool)> = None;
        // An elided item vanished since the last emitted one.
        let mut vanished = false;
        // The previous item was elided and the source has a blank line after
        // its last comment.
        let mut blank_after_elided = false;
        for (index, item) in items.into_iter().enumerate() {
            let (node, elide) = match item {
                ListItem::Format(node) => (node, false),
                ListItem::Elide(node) => (node, true),
            };
            let ignored = self.in_ignore_region(node);
            let mut blank_after = false;
            let doc = if ignored {
                let body = self.verbatim(node);
                self.with_comments(SyntaxElement::Node(node), body)
            } else if elide {
                let (before, after, blank) = self.removed_parts(SyntaxElement::Node(node));
                blank_after = blank;
                let doc = if vanished || prev.is_none() {
                    cat![self; before, after]
                } else {
                    docs.push(before);
                    after
                };
                if doc.is_nil() {
                    vanished = true;
                    continue;
                }
                doc
            } else {
                self.node(node)
            };
            if let Some((prev_node, prev_index, prev_ignored)) = prev {
                let blank = blank_after_elided || self.blank_line_before(SyntaxElement::Node(node));
                // Items in an ignore region, and the item after one, keep the
                // source's separation, so the region prints as written.
                let doc = if prev_ignored {
                    self.line_break(blank)
                } else {
                    separator(
                        self,
                        Separator {
                            prev: prev_node,
                            next: node,
                            prev_index,
                            next_index: index,
                            blank_line_in_source: blank,
                        },
                    )
                };
                docs.push(doc);
            }
            docs.push(doc);
            prev = Some((node, index, ignored));
            vanished = false;
            blank_after_elided = blank_after;
        }
        self.builder.concat(docs)
    }

    // ── Comment printing ──

    /// The comments attached to `element` on `side`, each in its effective
    /// layout.
    fn comments_on(&mut self, element: SyntaxElement<'a>, side: Side) -> Doc {
        let positions = self.comments.positions(element, side);
        if positions.is_empty() {
            return Doc::NIL;
        }
        let docs: Vec<Doc> = positions
            .map(|position| {
                let index = self.comments.at(position);
                self.attached_comment(index)
            })
            .collect();
        self.builder.concat(docs)
    }

    /// One attached comment in its effective layout. A comment that verbatim
    /// output already printed is skipped (see the formatter design's
    /// "Verbatim edges").
    fn attached_comment(&mut self, index: usize) -> Doc {
        if self.by_verbatim[index] {
            return Doc::NIL;
        }
        let placed = self.comments.list[index];
        let blank_after = placed.comment.lines_after >= 2;
        let text = self.comment_doc(index);
        match (placed.side, placed.layout) {
            (Side::Leading, CommentLayout::OwnLine) => {
                let after = self.line_break(blank_after);
                cat![self; Doc::HARD_LINE, text, after]
            }
            // Spaced from what follows like an inline comment, which is what
            // the comment is when the line does not break before it.
            (Side::Leading, CommentLayout::StartOfLine) => {
                let after = self.space_after(index);
                cat![self; Doc::HARD_LINE, text, after]
            }
            (Side::Leading, CommentLayout::EndOfLine) => {
                let before = self.space_before(index);
                let after = self.line_break(blank_after);
                cat![self; before, text, after]
            }
            // A block comment at the end of a line trails its code like an
            // inline one: it does not force the line to break.
            (Side::Trailing | Side::Dangling, CommentLayout::EndOfLine)
                if placed.comment.kind == CommentKind::Line =>
            {
                let suffix = cat![self; Doc::SPACE, text];
                let suffix = self.builder.line_suffix(suffix);
                cat![self; suffix, Doc::BREAK_PARENT]
            }
            (_, CommentLayout::OwnLine) => cat![self; Doc::HARD_LINE, text, Doc::HARD_LINE],
            // Code may follow on the line (after a block comment that ended
            // its line in the source), spaced as after a leading inline one.
            (_, CommentLayout::Inline | CommentLayout::StartOfLine | CommentLayout::EndOfLine) => {
                let before = self.space_before(index);
                let after = self.space_after(index);
                cat![self; before, text, after]
            }
        }
    }

    /// No space after a token that hugs what follows, unless another comment
    /// of the same gap precedes this one.
    fn space_before(&self, index: usize) -> Doc {
        if comments::first_in_gap(&self.comments.list, index)
            && self.comments.list[index].prev.is_some_and(L::hugs_next)
        {
            Doc::NIL
        } else {
            Doc::SPACE
        }
    }

    /// No space before a closer or separator. (A comment after this one
    /// brings its own space.)
    fn space_after(&self, index: usize) -> Doc {
        let next = self.comments.list[index].next;
        if L::is_closer(next) || L::is_separator(next) {
            Doc::NIL
        } else {
            Doc::SPACE
        }
    }

    /// The comment's text, counted as emitted.
    fn comment_doc(&mut self, index: usize) -> Doc {
        self.emitted[index] += 1;
        let comment = self.comments.list[index].comment;
        if comment.kind == CommentKind::Line {
            return self
                .builder
                .comment_text(comment.text.trim_end(), LINE_COMMENT);
        }
        if comment.unterminated {
            return self.builder.text(comment.text.trim_end());
        }
        if has_line_break(comment.text) {
            return self.builder.reindented(comment.text, Reindent::Comment);
        }
        self.builder.comment_text(comment.text, 0)
    }
}

/// Whether a continuation line of a multi-line `text` does not start with
/// `prefix` (blank lines aside).
fn lines_outside(text: &str, prefix: &str) -> bool {
    text.lines()
        .skip(1)
        .any(|line| !line.trim().is_empty() && !line.starts_with(prefix))
}

/// The indentation of the source line containing `offset`.
fn line_indentation(source: &str, offset: TextSize) -> &str {
    let before = &source[..offset.get()];
    let start = before.rfind(['\n', '\r']).map_or(0, |index| index + 1);
    let line = &source[start..];
    let end = line
        .find(|ch: char| ch != ' ' && ch != '\t')
        .unwrap_or(line.len());
    &line[..end]
}

/// A formatter-ignore marker comment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Marker {
    /// `@formatter:off`, which starts a region.
    Off,
    /// `@formatter:on`, which ends it.
    On,
}

fn marker(text: &str) -> Option<Marker> {
    let body = text
        .strip_prefix("//")
        .or_else(|| text.strip_prefix("/*")?.strip_suffix("*/"))?
        .trim();
    match body {
        "@formatter:off" => Some(Marker::Off),
        "@formatter:on" => Some(Marker::On),
        _ => None,
    }
}

/// Pairs `@formatter:off`/`on` markers that sit between the items of one
/// list into sorted, disjoint regions. A region runs from the end of its
/// `off` marker to the start of its `on` marker, or, without one, to the end
/// of the list's owner (before its closer).
fn ignore_regions<'a, L: FormatLanguage>(
    root: SyntaxNode<'a>,
    comments: &Comments<'a>,
) -> Vec<TextRange> {
    let mut regions = Vec::new();
    // The end of the open region's `off` marker and the list owner.
    let mut open: Option<(TextSize, SyntaxNode<'a>)> = None;
    for placed in &comments.list {
        let Some(marker) = marker(placed.comment.text) else {
            continue;
        };
        let scope = || L::ignore_scope(comments::enclosing(root, placed.prev, placed.next));
        match (marker, open) {
            (Marker::Off, None) => {
                open = scope().map(|owner| (placed.comment.range.end(), owner));
            }
            (Marker::On, Some((start, owner))) => {
                if scope() == Some(owner) {
                    regions.push(TextRange::new(start, placed.comment.range.start()));
                }
                open = None;
            }
            _ => {}
        }
    }
    if let Some((start, owner)) = open {
        let end = owner.last_token().map_or_else(
            || owner.text_range().end(),
            |last| {
                if last.text().is_empty() || L::is_closer(last) {
                    last.text_range().start()
                } else {
                    owner.text_range().end()
                }
            },
        );
        if start < end {
            regions.push(TextRange::new(start, end));
        }
    }
    regions
}
