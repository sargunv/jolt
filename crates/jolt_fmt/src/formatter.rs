//! The `Formatter` context: the chokepoints through which rules emit source,
//! comment printing, verbatim output, and item lists with formatter-ignore.

use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

use jolt_syntax::{
    NodeId, SyntaxElement, SyntaxNode, SyntaxToken, TextRange, TextSize, TriviaKind,
};

use crate::comments::{self, CommentKind, CommentLayout, Comments, Side};
use crate::doc::{LINE_COMMENT, VERBATIM_START, has_line_break};
use crate::{Doc, DocBuilder, FormatLanguage, Reindent, cat};

/// One item of an [`Formatter::item_list`].
#[derive(Clone, Copy, Debug)]
pub enum ListItem<'a> {
    /// Formatted with [`Formatter::node`].
    Format(SyntaxNode<'a>),
    /// Removed by a normalization: only its comments are printed.
    Elide(SyntaxNode<'a>),
}

/// What a list separator callback sees.
#[derive(Clone, Copy, Debug)]
pub struct Separator<'a> {
    pub prev: SyntaxNode<'a>,
    pub next: SyntaxNode<'a>,
    pub blank_line_in_source: bool,
}

/// A formatter-ignore region and the item-list owner it belongs to.
#[derive(Clone, Copy, Debug)]
struct Region {
    range: TextRange,
}

pub struct Formatter<'a, L: FormatLanguage> {
    builder: DocBuilder<'a>,
    comments: Comments<'a>,
    regions: Vec<Region>,
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
                let comment = &self.comments.list[index];
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
        self.node_with(node, body)
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
        let element = SyntaxElement::Token(token);
        if !self.comments.has(element) {
            return text;
        }
        let leading = self.leading_comments_of(element);
        let trailing = self.trailing_comments_of(element);
        cat![self; leading, text, trailing]
    }

    pub fn token_opt(&mut self, token: Option<SyntaxToken<'a>>) -> Doc {
        token.map_or(Doc::NIL, |token| self.token(token))
    }

    /// `node`'s leading and trailing comments around `body`, which the caller
    /// built from the node's children.
    pub fn node_with(&mut self, node: SyntaxNode<'a>, body: Doc) -> Doc {
        let element = SyntaxElement::Node(node);
        if !self.comments.has(element) {
            return body;
        }
        let leading = self.leading_comments_of(element);
        let trailing = self.trailing_comments_of(element);
        cat![self; leading, body, trailing]
    }

    pub fn leading_comments(&mut self, node: SyntaxNode<'a>) -> Doc {
        self.leading_comments_of(SyntaxElement::Node(node))
    }

    pub fn trailing_comments(&mut self, node: SyntaxNode<'a>) -> Doc {
        self.trailing_comments_of(SyntaxElement::Node(node))
    }

    /// Every comment attached to or inside `element`, in source order, without
    /// the element's text. NIL when there are none.
    pub fn removed(&mut self, element: SyntaxElement<'a>) -> Doc {
        let (before, after, _) = self.removed_parts(element);
        cat![self; before, after]
    }

    /// `removed`, split into the trailing comments that sit before the
    /// element in the source (moved onto it by S1, so they end the previous
    /// line) and the rest, and whether the source has a blank line after the
    /// last comment (which, with the element gone, separates the comments
    /// from what follows).
    fn removed_parts(&mut self, element: SyntaxElement<'a>) -> (Doc, Doc, bool) {
        let mut indices: Vec<usize> = self
            .comments
            .attached(element, Side::Leading)
            .chain(self.comments.attached(element, Side::Trailing))
            .collect();
        if let SyntaxElement::Node(node) = element {
            indices.extend(self.comments.within(node.text_range()));
        }
        indices.sort_unstable();
        let start = element.text_range().start();
        let mut before = Vec::new();
        let mut after = Vec::new();
        let last = indices.last().copied();
        let blank_after = last.is_some_and(|last| self.comments.list[last].lines_after >= 2);
        for index in indices {
            let (side, layout) = self.comments.attached[index];
            // The last comment has no element left to separate it from.
            let doc = if Some(index) == last
                && (side, layout) == (Side::Leading, CommentLayout::OwnLine)
            {
                let text = self.comment_doc(index, true);
                cat![self; self.builder.hard_line(), text]
            } else {
                self.attached_comment(index, side, layout)
            };
            if side == Side::Trailing && self.comments.list[index].range.end() <= start {
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
    /// blocks (multi-line tokens) are written raw. Comments attached inside
    /// the node but outside its text (to an empty descendant, to an item that
    /// begins or ends it, or dangling in it) print before or after it.
    pub fn verbatim(&mut self, node: SyntaxNode<'a>) -> Doc {
        let range = node.text_range();
        let inside = self.comments.within(range);
        for index in inside.clone() {
            self.emitted[index] += 1;
            self.by_verbatim[index] = true;
        }
        let (leading, trailing) = self.outside_comments(node, range, inside);
        if range.is_empty() {
            return cat![self; leading, trailing];
        }
        let source = node.source();
        let prefix = line_indentation(source, range.start());
        let mut docs = vec![leading];
        let mut cursor = range.start();
        let mut started = false;
        let mut piece = |f: &mut Self, text: &'a str, prefix: Option<&'a str>| {
            let doc = f.verbatim_text(text, prefix, !started);
            started |= !doc.is_nil();
            doc
        };
        for token in node.tokens().filter(|token| has_line_break(token.text())) {
            let token_range = token.text_range();
            let text = &source[cursor.get()..token_range.start().get()];
            docs.push(piece(self, text, Some(prefix)));
            docs.push(piece(self, token.text(), None));
            cursor = token_range.end();
        }
        let text = &source[cursor.get()..range.end().get()];
        docs.push(piece(self, text, Some(prefix)));
        docs.push(trailing);
        self.builder.concat(docs)
    }

    /// The comments attached to `node`'s descendants (and dangling in
    /// `node`) that lie outside its text `range`, split into those before and
    /// after it. `node`'s own leading and trailing comments are left to
    /// `node_with`.
    fn outside_comments(
        &mut self,
        node: SyntaxNode<'a>,
        range: TextRange,
        inside: std::ops::Range<usize>,
    ) -> (Doc, Doc) {
        let mut indices = Vec::new();
        for element in node.descendants() {
            if !self.comments.has(element) {
                continue;
            }
            let own = element == SyntaxElement::Node(node);
            for side in [Side::Leading, Side::Trailing, Side::Dangling] {
                if own && side != Side::Dangling {
                    continue;
                }
                indices.extend(
                    self.comments
                        .attached(element, side)
                        .filter(|index| !inside.contains(index)),
                );
            }
        }
        indices.sort_unstable();
        let mut before = Vec::new();
        let mut after = Vec::new();
        for index in indices {
            let (side, layout) = self.comments.attached[index];
            let is_before = self.comments.list[index].range.end() <= range.start();
            let side = match side {
                Side::Dangling if is_before => Side::Leading,
                Side::Dangling => Side::Trailing,
                side => side,
            };
            let doc = self.attached_comment(index, side, layout);
            if is_before {
                before.push(doc);
            } else {
                after.push(doc);
            }
        }
        (self.builder.concat(before), self.builder.concat(after))
    }

    /// A piece of verbatim output: source re-indented by `prefix`, or a raw
    /// multi-line token. The first non-empty piece records the base
    /// indentation every piece of the node re-indents against.
    fn verbatim_text(&mut self, text: &'a str, prefix: Option<&'a str>, first: bool) -> Doc {
        let flags = if first { VERBATIM_START } else { 0 };
        let reindent = prefix.map(Reindent::Strip);
        self.builder.text_with(text, reindent, flags)
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
        let mut docs = Vec::with_capacity(indices.len() * 2 + 1);
        for (position, &index) in indices.iter().enumerate() {
            if position == 0 {
                docs.push(self.builder.break_parent());
            } else {
                let lines_before = self.comments.list[index].lines_before;
                docs.push(match lines_before {
                    0 => Doc::SPACE,
                    1 => Doc::HARD_LINE,
                    _ => self.builder.empty_line(),
                });
            }
            // Dangling comments always print on their own lines.
            docs.push(self.comment_doc(index, true));
        }
        self.builder.concat(docs)
    }

    // ── Queries ──

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
            .any(|index| self.comments.list[index].kind == CommentKind::Line)
    }

    /// Some leading comment of the element prints a line break between the
    /// code before it and the element: one on its own line, starting its
    /// line, or ending its line.
    #[must_use]
    pub fn leading_comments_break(&self, element: SyntaxElement<'a>) -> bool {
        self.comments
            .attached(element, Side::Leading)
            .any(|index| self.comments.attached[index].1 != CommentLayout::Inline)
    }

    /// Some trailing comment of the element ends its line.
    #[must_use]
    pub fn trailing_comments_break(&self, element: SyntaxElement<'a>) -> bool {
        self.comments
            .attached(element, Side::Trailing)
            .any(|index| self.comments.attached[index].1 != CommentLayout::Inline)
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
            return self.comments.list[first].lines_before >= 2;
        }
        let Some(token) = element.first_token() else {
            return false;
        };
        let mut newlines = 0;
        for piece in token.leading() {
            match piece.kind {
                TriviaKind::Newline => newlines += 1,
                TriviaKind::LineComment
                | TriviaKind::BlockComment
                | TriviaKind::DocComment
                | TriviaKind::UnterminatedBlockComment => newlines = 0,
                _ => {}
            }
        }
        newlines >= 2
    }

    /// Whether `node` is an item inside a formatter-ignore region, so a
    /// normalization must leave it in place.
    #[must_use]
    pub fn in_ignore_region(&self, node: SyntaxNode<'a>) -> bool {
        self.region_of(node).is_some()
    }

    /// The source has a blank line before the node's first dangling comment.
    #[must_use]
    pub fn blank_line_before_dangling(&self, node: SyntaxNode<'a>) -> bool {
        self.comments
            .attached(SyntaxElement::Node(node), Side::Dangling)
            .next()
            .is_some_and(|index| self.comments.list[index].lines_before >= 2)
    }

    // ── Item lists ──

    /// Items separated by `separator`, with elision and formatter-ignore.
    pub fn item_list(
        &mut self,
        items: impl IntoIterator<Item = ListItem<'a>>,
        mut separator: impl FnMut(&mut Self, Separator<'a>) -> Doc,
    ) -> Doc {
        let mut docs = Vec::new();
        let mut prev: Option<(SyntaxNode<'a>, Option<usize>)> = None;
        // An elided item vanished since the last emitted one.
        let mut vanished = false;
        // The previous item was elided and the source has a blank line after
        // its last comment.
        let mut blank_after_elided = false;
        for item in items {
            let (node, elide) = match item {
                ListItem::Format(node) => (node, false),
                ListItem::Elide(node) => (node, true),
            };
            let region = self.region_of(node);
            let mut blank_after = false;
            let doc = if region.is_some() {
                let body = self.verbatim(node);
                self.node_with(node, body)
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
            if let Some((prev_node, prev_region)) = prev {
                let blank = blank_after_elided || self.blank_line_before(SyntaxElement::Node(node));
                // Items in an ignore region, and the item after one, keep the
                // source's separation, so the region prints as written.
                let doc = if prev_region.is_some() {
                    self.line_or_blank(blank)
                } else {
                    separator(
                        self,
                        Separator {
                            prev: prev_node,
                            next: node,
                            blank_line_in_source: blank,
                        },
                    )
                };
                docs.push(doc);
            }
            docs.push(doc);
            prev = Some((node, region));
            vanished = false;
            blank_after_elided = blank_after;
        }
        self.builder.concat(docs)
    }

    fn region_of(&self, node: SyntaxNode<'a>) -> Option<usize> {
        let start = node.first_token()?.text_range().start();
        let index = self
            .regions
            .partition_point(|region| region.range.end() <= start);
        self.regions
            .get(index)
            .filter(|region| region.range.start() <= start && start < region.range.end())
            .map(|_| index)
    }

    // ── Comment printing ──

    fn line_or_blank(&self, blank: bool) -> Doc {
        if blank {
            self.builder.empty_line()
        } else {
            self.builder.hard_line()
        }
    }

    fn leading_comments_of(&mut self, element: SyntaxElement<'a>) -> Doc {
        let indices: Vec<usize> = self.comments.attached(element, Side::Leading).collect();
        let docs: Vec<Doc> = indices
            .into_iter()
            .map(|index| {
                let (_, layout) = self.comments.attached[index];
                self.attached_comment(index, Side::Leading, layout)
            })
            .collect();
        self.builder.concat(docs)
    }

    fn trailing_comments_of(&mut self, element: SyntaxElement<'a>) -> Doc {
        let indices: Vec<usize> = self.comments.attached(element, Side::Trailing).collect();
        let docs: Vec<Doc> = indices
            .into_iter()
            .map(|index| {
                let (_, layout) = self.comments.attached[index];
                self.attached_comment(index, Side::Trailing, layout)
            })
            .collect();
        self.builder.concat(docs)
    }

    /// One attached comment in its effective layout. A comment that verbatim
    /// output already printed is skipped (see the formatter design's
    /// "Verbatim and S1").
    fn attached_comment(&mut self, index: usize, side: Side, layout: CommentLayout) -> Doc {
        if self.by_verbatim[index] {
            return Doc::NIL;
        }
        let comment = self.comments.list[index];
        match (side, layout) {
            (Side::Leading, CommentLayout::OwnLine) => {
                let after = self.line_or_blank(comment.lines_after >= 2);
                let text = self.comment_doc(index, true);
                cat![self; self.builder.hard_line(), text, after]
            }
            // Spaced from what follows like an inline comment, which is what
            // the comment is when the line does not break before it.
            (Side::Leading, CommentLayout::StartOfLine) => {
                let after = self.space_after(index);
                let text = self.comment_doc(index, false);
                cat![self; Doc::HARD_LINE, text, after]
            }
            (Side::Leading, CommentLayout::EndOfLine) => {
                let before = self.space_before(index);
                let after = self.line_or_blank(comment.lines_after >= 2);
                let text = self.comment_doc(index, false);
                cat![self; before, text, after]
            }
            // A block comment at the end of a line trails its code like an
            // inline one: it does not force the line to break.
            (Side::Trailing | Side::Dangling, CommentLayout::EndOfLine)
                if comment.kind == CommentKind::Line =>
            {
                let (space, break_parent) = (Doc::SPACE, Doc::BREAK_PARENT);
                let text = self.comment_doc(index, false);
                let suffix = cat![self; space, text];
                let suffix = self.builder.line_suffix(suffix);
                cat![self; suffix, break_parent]
            }
            (_, CommentLayout::OwnLine) => {
                let (hard_line, after) = (Doc::HARD_LINE, Doc::HARD_LINE);
                let text = self.comment_doc(index, true);
                cat![self; hard_line, text, after]
            }
            // Code may follow on the line (after a block comment that ended
            // its line in the source), spaced as after a leading inline one.
            (_, CommentLayout::Inline | CommentLayout::StartOfLine | CommentLayout::EndOfLine) => {
                let before = self.space_before(index);
                let after = self.space_after(index);
                let text = self.comment_doc(index, false);
                cat![self; before, text, after]
            }
        }
    }

    /// No space after a token that hugs what follows, unless another comment
    /// of the same gap precedes this one.
    fn space_before(&self, index: usize) -> Doc {
        let after_comment = index > 0 && self.comments.next[index - 1] == self.comments.next[index];
        if !after_comment && self.comments.prev[index].is_some_and(L::hugs_next) {
            Doc::NIL
        } else {
            Doc::SPACE
        }
    }

    /// No space before a closer or separator. (A comment after this one
    /// brings its own space.)
    fn space_after(&self, index: usize) -> Doc {
        let next = self.comments.next[index];
        if L::is_closer(next) || L::is_separator(next) {
            Doc::NIL
        } else {
            Doc::SPACE
        }
    }

    /// The comment's text, counted as emitted. `own_line` allows doc comment
    /// expansion.
    fn comment_doc(&mut self, index: usize, own_line: bool) -> Doc {
        self.emitted[index] += 1;
        let comment = self.comments.list[index];
        if comment.kind == CommentKind::Line {
            return self
                .builder
                .comment_text(comment.text.trim_end(), LINE_COMMENT);
        }
        if comment.is_unterminated() {
            return self.builder.text(comment.text.trim_end());
        }
        if has_line_break(comment.text) {
            return self.builder.reindented(comment.text, Reindent::Comment);
        }
        let body = comment
            .text
            .strip_prefix("/**")
            .and_then(|text| text.strip_suffix("*/"))
            .unwrap_or("");
        if own_line
            && L::EXPAND_DOC_COMMENTS
            && comment.kind == CommentKind::Doc
            && !body.trim().is_empty()
        {
            return self.builder.reindented(comment.text, Reindent::ExpandedDoc);
        }
        self.builder.comment_text(comment.text, 0)
    }
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

fn marker(text: &str) -> Option<bool> {
    let body = text
        .strip_prefix("//")
        .or_else(|| text.strip_prefix("/*")?.strip_suffix("*/"))?
        .trim();
    match body {
        "@formatter:off" => Some(false),
        "@formatter:on" => Some(true),
        _ => None,
    }
}

/// Pairs `@formatter:off`/`on` markers that sit between the items of one
/// list into sorted, disjoint regions.
fn ignore_regions<'a, L: FormatLanguage>(
    root: SyntaxNode<'a>,
    comments: &Comments<'a>,
) -> Vec<Region> {
    let mut regions = Vec::new();
    let mut open: Option<(TextSize, Option<NodeId>, SyntaxNode<'a>)> = None;
    for (index, comment) in comments.list.iter().enumerate() {
        let Some(on) = marker(comment.text) else {
            continue;
        };
        let scope = || {
            L::ignore_scope(comments::enclosing(
                root,
                comments.prev[index],
                comments.next[index],
            ))
        };
        match (on, open) {
            (false, None) => {
                if let Some(owner) = scope() {
                    open = Some((comment.range.end(), Some(owner.id()), owner));
                }
            }
            (true, Some((start, owner, _))) => {
                if scope().map(SyntaxNode::id) == owner {
                    regions.push(Region {
                        range: TextRange::new(start, comment.range.start()),
                    });
                }
                open = None;
            }
            _ => {}
        }
    }
    if let Some((start, _, owner)) = open {
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
            regions.push(Region {
                range: TextRange::new(start, end),
            });
        }
    }
    regions
}
