use std::borrow::Cow;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

use crate::width::{TextWidth, display_width, literal_text_metrics};
use crate::{
    ExceptionalFragment, ExceptionalSeparator, FragmentBoundary, LexicalAtom, LexicalSafety,
    formatter_ignore::{
        FormatterIgnoreItemRange, FormatterIgnorePlan, FormatterIgnoreRun,
        formatter_ignore_may_apply, formatter_ignore_runs,
    },
    source_fragment::{
        ExceptionalSeparators, SourceClaim, exceptional_separators, normalized_lexical_kind,
    },
};
use jolt_syntax::{
    Language, RemovalClaim, ReorderClaim, ReplacementClaim, SourceIdentity, SourceRangeClaim,
    SourceTriviaPiece, SyntaxToken, SyntaxVerbatimCore, SynthesisClaim,
};
use jolt_text::TextRange;

pub(crate) const INLINE_CONCAT_CAPACITY: usize = 4;

/// Copyable formatter document handle.
///
/// Documents are allocated into a [`DocBuilder`] for one formatting run. This
/// handle indexes that builder's arena and does not own recursive child data.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Doc<'source> {
    id: DocId,
    source: PhantomData<&'source str>,
}

impl Doc<'_> {
    const NIL_ID: DocId = DocId(u32::MAX);

    #[must_use]
    pub const fn nil() -> Self {
        Self::new(Self::NIL_ID)
    }

    const fn new(id: DocId) -> Self {
        Self {
            id,
            source: PhantomData,
        }
    }

    const fn node_index(self) -> u32 {
        self.id.0
    }

    pub(crate) const fn is_nil(self) -> bool {
        self.id.0 == Self::NIL_ID.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DocId(u32);

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DocArena<'source> {
    nodes: Vec<DocNode<'source>>,
    children: Vec<Doc<'source>>,
    invariant_error: Option<String>,
}

/// Formatter document arena measurements exposed only to the benchmark driver.
#[cfg(feature = "bench")]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DocArenaMetrics {
    pub nodes: usize,
    pub children: usize,
    pub logical_bytes: usize,
    pub reserved_bytes: usize,
}

impl<'source> DocArena<'source> {
    pub(crate) fn invariant_error(&self) -> Option<&str> {
        self.invariant_error.as_deref()
    }

    /// Returns allocation-independent size and capacity measurements.
    #[cfg(feature = "bench")]
    #[must_use]
    pub fn benchmark_metrics(&self) -> DocArenaMetrics {
        use std::mem::size_of;

        DocArenaMetrics {
            nodes: self.nodes.len(),
            children: self.children.len(),
            logical_bytes: self.nodes.len() * size_of::<DocNode<'_>>()
                + self.children.len() * size_of::<Doc<'_>>(),
            reserved_bytes: self.nodes.capacity() * size_of::<DocNode<'_>>()
                + self.children.capacity() * size_of::<Doc<'_>>(),
        }
    }
    pub(crate) fn node(&self, doc: Doc<'source>) -> Option<&DocNode<'source>> {
        if doc.is_nil() {
            return None;
        }
        self.nodes.get(usize::try_from(doc.node_index()).ok()?)
    }

    pub(crate) fn child(&self, index: u32) -> Doc<'source> {
        self.children[usize::try_from(index).expect("doc child index fits usize")]
    }

    fn child_count(&self) -> u32 {
        self.children
            .len()
            .try_into()
            .expect("doc arena child count fits u32")
    }

    fn push_node(&mut self, node: DocNode<'source>) -> Doc<'source> {
        let id = DocId(
            self.nodes
                .len()
                .try_into()
                .expect("doc arena node count fits u32"),
        );
        assert_ne!(id, Doc::NIL_ID, "doc arena node count fits u32");
        self.nodes.push(node);
        Doc::new(id)
    }

    fn push_child(&mut self, doc: Doc<'source>) {
        self.children.push(doc);
    }

    fn extend_children(&mut self, docs: &[Doc<'source>]) {
        self.children.extend_from_slice(docs);
    }
}

#[derive(Default)]
pub struct DocBuilder<'source> {
    arena: DocArena<'source>,
    list_scratch: Vec<Doc<'source>>,
    formatter_ignore: Option<FormatterIgnorePlan<'source>>,
}

impl<'source> DocBuilder<'source> {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates the root document builder with its immutable formatter-ignore
    /// plan and a linear arena reservation derived from the source.
    #[must_use]
    pub(crate) fn for_root(
        source_len: usize,
        formatter_ignore: FormatterIgnorePlan<'source>,
    ) -> Self {
        let mut builder = Self::with_source_capacity(source_len);
        builder.formatter_ignore = Some(formatter_ignore);
        builder
    }

    /// Creates one document arena with a linear reservation derived from the
    /// source it will format.
    ///
    /// Realistic Java and Kotlin both produce about two document nodes per
    /// five source bytes. Child ranges are less dense; one slot per eight
    /// bytes is a conservative estimate. These fixed ratios avoid geometric
    /// arena growth without inspecting syntax twice or introducing per-rule
    /// allocation pools.
    fn with_source_capacity(source_len: usize) -> Self {
        // Allocator size-class rounding dominates these estimates for small
        // files. Let the ordinary arena grow inline there; reserve once only
        // when the source is large enough for the estimate to be meaningful.
        if source_len < 2 * 1024 {
            return Self::new();
        }
        let node_capacity = source_len.saturating_mul(2).div_ceil(5);
        let child_capacity = source_len.div_ceil(8);
        Self {
            arena: DocArena {
                nodes: Vec::with_capacity(node_capacity),
                children: Vec::with_capacity(child_capacity),
                invariant_error: None,
            },
            list_scratch: Vec::new(),
            formatter_ignore: None,
        }
    }

    /// Returns whether a root-discovered ignore range can belong to this
    /// syntax-owned content interval.
    pub fn formatter_ignore_may_apply(&mut self, container: TextRange) -> bool {
        let Some(plan) = self.formatter_ignore.as_ref() else {
            self.block_on_invariant("formatter-ignore plan was not installed at the syntax root");
            return false;
        };
        formatter_ignore_may_apply(plan, container)
    }

    /// Lazily derives exact formatter-ignore runs for one syntax-owned item
    /// list when an ignore range belongs to its content interval.
    #[must_use]
    pub fn formatter_ignore_runs(
        &mut self,
        container: TextRange,
        items: impl IntoIterator<Item = Option<FormatterIgnoreItemRange>>,
    ) -> Vec<FormatterIgnoreRun<'source>> {
        let Some(plan) = self.formatter_ignore.as_ref() else {
            self.block_on_invariant("formatter-ignore plan was not installed at the syntax root");
            return Vec::new();
        };
        if !formatter_ignore_may_apply(plan, container) {
            return Vec::new();
        }
        let items = items.into_iter().collect::<Vec<_>>();
        formatter_ignore_runs(plan, container, &items)
    }

    #[must_use]
    pub const fn nil(&self) -> Doc<'source> {
        Doc::new(Doc::NIL_ID)
    }

    #[must_use]
    pub fn text(&mut self, value: impl Into<Cow<'source, str>>) -> Doc<'source> {
        let text = value.into();
        let final_width = display_width(&text);
        self.push_node(DocNode::Text(DocumentText {
            text,
            final_width,
            first_width: final_width,
            is_multiline: false,
            #[cfg(debug_assertions)]
            claim: None,
        }))
    }

    #[must_use]
    pub fn space(&mut self) -> Doc<'source> {
        self.text(" ")
    }

    #[must_use]
    pub fn literal_text(&mut self, value: impl Into<Cow<'source, str>>) -> Doc<'source> {
        let text = value.into();
        let metrics = literal_text_metrics(&text);
        self.push_node(DocNode::Text(DocumentText {
            text,
            final_width: metrics.final_width,
            first_width: metrics.first_width,
            is_multiline: metrics.line_count > 1,
            #[cfg(debug_assertions)]
            claim: None,
        }))
    }

    /// Emits an ordinary structured source token with its conservation claim.
    #[must_use]
    pub fn source_token<L: Language>(&mut self, token: &SyntaxToken<'source, L>) -> Doc<'source> {
        if token.text().is_empty() {
            return self.nil();
        }
        self.source_fragment(
            Cow::Borrowed(token.text()),
            SourceClaim::Identity(SourceIdentity::Token(token.source_id())),
        )
    }

    /// Builds structured trivia output together with its represented source.
    ///
    /// The closure keeps construction and ownership attachment atomic: callers
    /// cannot build a source-backed trivia document and later forget its claim.
    #[must_use]
    pub fn source_trivia(
        &mut self,
        pieces: impl IntoIterator<Item = SourceTriviaPiece<'source>>,
        build: impl FnOnce(&mut Self) -> Doc<'source>,
    ) -> Doc<'source> {
        let contents = build(self);
        let claim = self.concat_list(|docs| {
            for piece in pieces {
                let claim = docs.source_fragment(
                    Cow::Borrowed(""),
                    SourceClaim::Identity(SourceIdentity::Trivia(piece.id())),
                );
                docs.push(claim);
            }
        });
        self.concat([claim, contents])
    }

    /// Emits one parser-backed formatter-ignore document.
    #[must_use]
    pub(crate) fn formatter_ignore_source(
        &mut self,
        contents: Doc<'source>,
        range: SourceRangeClaim<'source>,
        separators: ExceptionalSeparators,
    ) -> Doc<'source> {
        let claim = self.source_fragment(Cow::Borrowed(""), SourceClaim::FormatterIgnore { range });
        let before = self.exceptional_separator(separators.before);
        let after = self.exceptional_separator(separators.after);
        self.concat([claim, before, contents, after])
    }

    /// Records a syntax/schema contradiction that must block the entire render.
    pub fn block_on_invariant(&mut self, error: impl Into<String>) {
        if self.arena.invariant_error.is_none() {
            self.arena.invariant_error = Some(error.into());
        }
    }

    /// Emits one borrowed malformed source core and records every identity it
    /// covers. An empty core and empty claim set still records malformed
    /// dispatch.
    #[must_use]
    pub fn malformed_verbatim<L: Language>(
        &mut self,
        core: &SyntaxVerbatimCore<'source, L>,
        boundary: FragmentBoundary<'source>,
    ) -> ExceptionalFragment<'source> {
        let contents = self.source_fragment(
            Cow::Borrowed(core.text()),
            SourceClaim::MalformedVerbatim {
                claim: core.source_claim(),
                kind: core.raw_kind(),
                range: core.text_range(),
            },
        );
        ExceptionalFragment::exact_source(contents, boundary, core.text_range())
    }

    /// Emits malformed source and derives lexical boundaries from syntax.
    #[must_use]
    pub fn malformed_verbatim_with_safety<L: Language>(
        &mut self,
        core: &SyntaxVerbatimCore<'source, L>,
        safety: &mut impl LexicalSafety<L>,
    ) -> ExceptionalFragment<'source> {
        let mut tokens = core.tokens().filter(|token| !token.text().is_empty());
        let first = tokens.next();
        let last = tokens.last().or(first);
        let boundary = FragmentBoundary {
            first: first.map(|token| LexicalAtom::new(safety.classify(&token), token.text())),
            last: last.map(|token| LexicalAtom::new(safety.classify(&token), token.text())),
            ends_with_line_comment: core.ends_with_line_comment(),
        };
        self.malformed_verbatim(core, boundary)
    }

    /// Emits normalized spelling while consuming the replaced source identity.
    #[must_use]
    pub fn replaced_source(
        &mut self,
        claim: ReplacementClaim<'source>,
    ) -> ExceptionalFragment<'source> {
        let (source, token) = claim.into_parts();
        let text = token.text();
        let contents =
            self.source_fragment(Cow::Borrowed(text), SourceClaim::Replaced { source, token });
        let atom = LexicalAtom::new(normalized_lexical_kind(token), text);
        ExceptionalFragment::new(
            contents,
            FragmentBoundary {
                first: Some(atom),
                last: Some(atom),
                ends_with_line_comment: false,
            },
        )
    }

    /// Consumes a source identity without emitting text.
    #[must_use]
    pub fn removed_source(&mut self, claim: RemovalClaim<'source>) -> Doc<'source> {
        let (source, reason) = claim.into_parts();
        self.source_fragment(Cow::Borrowed(""), SourceClaim::Removed { source, reason })
    }

    /// Emits an authorized source-free token anchored near represented syntax.
    #[must_use]
    pub fn synthesized_source(
        &mut self,
        claim: SynthesisClaim<'source>,
    ) -> ExceptionalFragment<'source> {
        let (anchor, token) = claim.into_parts();
        let text = token.text();
        let contents = self.source_fragment(
            Cow::Borrowed(text),
            SourceClaim::Synthesized { token, anchor },
        );
        let atom = LexicalAtom::new(normalized_lexical_kind(token), text);
        ExceptionalFragment::new(
            contents,
            FragmentBoundary {
                first: Some(atom),
                last: Some(atom),
                ends_with_line_comment: false,
            },
        )
    }

    /// Marks a selected structured document as an authorized source reorder.
    #[must_use]
    pub fn reordered_source(
        &mut self,
        contents: Doc<'source>,
        claim: ReorderClaim<'source>,
    ) -> Doc<'source> {
        let (anchor, reason) = claim.into_parts();
        let marker =
            self.source_fragment(Cow::Borrowed(""), SourceClaim::Reordered { reason, anchor });
        self.concat([marker, contents])
    }

    /// Resolves the only permitted lexical joins around an exceptional fragment.
    #[must_use]
    pub fn resolve_exceptional<L: Language>(
        &mut self,
        fragment: ExceptionalFragment<'source>,
        left: Option<&SyntaxToken<'source, L>>,
        right: Option<&SyntaxToken<'source, L>>,
        safety: &mut impl LexicalSafety<L>,
    ) -> Doc<'source> {
        let source_range = fragment.source_range();
        let left = left
            .filter(|token| {
                source_range.is_none_or(|range| token.token_text_range().end() != range.start())
            })
            .map(|token| LexicalAtom::new(safety.classify(token), token.text()));
        let right = right
            .filter(|token| {
                source_range.is_none_or(|range| range.end() != token.token_text_range().start())
            })
            .map(|token| LexicalAtom::new(safety.classify(token), token.text()));
        let separators = exceptional_separators(left, fragment, right, safety);
        let before = self.exceptional_separator(separators.before);
        let after = self.exceptional_separator(separators.after);
        self.concat([before, fragment.doc(), after])
    }

    /// Joins two exceptional fragments while retaining their outer boundary.
    ///
    /// This is the only exceptional-to-exceptional composition path. It makes
    /// exactly one bounded lexical-safety decision at their shared edge.
    #[must_use]
    pub fn join_exceptional<L: Language>(
        &mut self,
        left: ExceptionalFragment<'source>,
        right: ExceptionalFragment<'source>,
        safety: &mut impl LexicalSafety<L>,
    ) -> ExceptionalFragment<'source> {
        let left_boundary = left.boundary();
        let right_boundary = right.boundary();
        let separator = if left_boundary.ends_with_line_comment && right_boundary.first.is_some() {
            ExceptionalSeparator::HardLine
        } else {
            match (left_boundary.last, right_boundary.first) {
                (Some(left), Some(right)) => safety.separator(left, right),
                _ => ExceptionalSeparator::None,
            }
        };
        let separator = self.exceptional_separator(separator);
        let doc = self.concat([left.doc(), separator, right.doc()]);
        let right_has_boundary = right_boundary.first.is_some() || right_boundary.last.is_some();
        ExceptionalFragment::new(
            doc,
            FragmentBoundary {
                first: left_boundary.first.or(right_boundary.first),
                last: right_boundary.last.or(left_boundary.last),
                ends_with_line_comment: if right_has_boundary {
                    right_boundary.ends_with_line_comment
                } else {
                    left_boundary.ends_with_line_comment
                },
            },
        )
    }

    #[must_use]
    pub fn concat(&mut self, docs: impl IntoIterator<Item = Doc<'source>>) -> Doc<'source> {
        let mut concat = ConcatAppender::new();
        for doc in docs {
            concat.push(doc, self);
        }
        concat.finish(self)
    }

    #[must_use]
    pub fn join(
        &mut self,
        separator: Doc<'source>,
        docs: impl IntoIterator<Item = Doc<'source>>,
    ) -> Doc<'source> {
        let mut concat = ConcatAppender::new();
        let mut needs_separator = false;
        for doc in docs {
            if needs_separator {
                concat.push(separator, self);
            } else {
                needs_separator = true;
            }
            concat.push(doc, self);
        }
        concat.finish(self)
    }

    /// Builds a concatenation using reusable builder scratch storage.
    ///
    /// # Panics
    ///
    /// Panics if the list exceeds the supported document size.
    #[must_use]
    pub fn concat_list(
        &mut self,
        build: impl FnOnce(&mut ConcatBuilder<'_, 'source>),
    ) -> Doc<'source> {
        let start = self.list_scratch.len();
        let mut list = ConcatBuilder {
            builder: self,
            start,
            active: true,
        };
        build(&mut list);
        list.finish()
    }

    #[must_use]
    pub fn group(&mut self, contents: Doc<'source>) -> Doc<'source> {
        self.group_with_break(contents, false)
    }

    #[must_use]
    pub fn force_group(&mut self, contents: Doc<'source>) -> Doc<'source> {
        self.group_with_break(contents, true)
    }

    #[must_use]
    pub fn indent(&mut self, contents: Doc<'source>) -> Doc<'source> {
        if contents.is_nil() {
            return contents;
        }

        self.push_node(DocNode::Indent {
            contents,
            levels: 1,
        })
    }

    #[must_use]
    pub fn line(&mut self) -> Doc<'source> {
        self.push_node(DocNode::Line(Line {
            mode: LineMode::SoftOrSpace,
            flat: FlatLine::Space,
            indent_delta: 0,
        }))
    }

    #[must_use]
    pub fn soft_line(&mut self) -> Doc<'source> {
        self.push_node(DocNode::Line(Line {
            mode: LineMode::Soft,
            flat: FlatLine::Empty,
            indent_delta: 0,
        }))
    }

    #[must_use]
    pub fn hard_line(&mut self) -> Doc<'source> {
        self.push_node(DocNode::Line(Line {
            mode: LineMode::Hard,
            flat: FlatLine::Empty,
            indent_delta: 0,
        }))
    }

    #[must_use]
    pub fn empty_line(&mut self) -> Doc<'source> {
        self.push_node(DocNode::Line(Line {
            mode: LineMode::Empty,
            flat: FlatLine::Empty,
            indent_delta: 0,
        }))
    }

    #[must_use]
    pub fn if_break(&mut self, breaks: Doc<'source>, flat: Doc<'source>) -> Doc<'source> {
        self.push_node(DocNode::IfBreak { breaks, flat })
    }

    #[must_use]
    pub fn into_arena(self) -> DocArena<'source> {
        self.arena
    }

    fn group_with_break(&mut self, contents: Doc<'source>, should_break: bool) -> Doc<'source> {
        if contents.is_nil() {
            return contents;
        }

        self.push_node(DocNode::Group {
            contents,
            should_break,
        })
    }

    fn push_node(&mut self, node: DocNode<'source>) -> Doc<'source> {
        self.arena.push_node(node)
    }

    fn source_fragment(
        &mut self,
        text: Cow<'source, str>,
        claim: SourceClaim<'source>,
    ) -> Doc<'source> {
        #[cfg(not(debug_assertions))]
        let _ = claim;
        let metrics = literal_text_metrics(&text);
        self.push_node(DocNode::Text(DocumentText {
            text,
            final_width: metrics.final_width,
            first_width: metrics.first_width,
            is_multiline: metrics.line_count > 1,
            #[cfg(debug_assertions)]
            claim: Some(claim),
        }))
    }

    fn exceptional_separator(&mut self, separator: ExceptionalSeparator) -> Doc<'source> {
        match separator {
            ExceptionalSeparator::None => self.nil(),
            ExceptionalSeparator::Space => self.space(),
            ExceptionalSeparator::HardLine => self.hard_line(),
        }
    }

    fn child_count(&self) -> u32 {
        self.arena.child_count()
    }

    fn push_child(&mut self, doc: Doc<'source>) {
        self.arena.push_child(doc);
    }
}

/// Scoped dynamic concatenation backed by reusable [`DocBuilder`] scratch.
pub struct ConcatBuilder<'builder, 'source> {
    builder: &'builder mut DocBuilder<'source>,
    start: usize,
    active: bool,
}

impl<'source> ConcatBuilder<'_, 'source> {
    /// Appends a document to this concatenation.
    pub fn push(&mut self, doc: Doc<'source>) {
        if !doc.is_nil() {
            self.builder.list_scratch.push(doc);
        }
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.builder.list_scratch.len() == self.start
    }

    fn finish(mut self) -> Doc<'source> {
        let len = self.builder.list_scratch.len() - self.start;
        let doc = match len {
            0 => self.builder.nil(),
            1 => self
                .builder
                .list_scratch
                .pop()
                .expect("concat list item exists"),
            _ => {
                let len = u32::try_from(len).expect("concat list length fits u32");
                let doc = if len
                    <= u32::try_from(INLINE_CONCAT_CAPACITY)
                        .expect("inline concat capacity fits u32")
                {
                    let mut docs = [Doc::nil(); INLINE_CONCAT_CAPACITY];
                    let source = &self.builder.list_scratch[self.start..];
                    docs[..source.len()].copy_from_slice(source);
                    self.builder.push_node(DocNode::InlineConcat {
                        docs,
                        len: u8::try_from(len).expect("inline concat length fits u8"),
                    })
                } else {
                    let child_start = self.builder.child_count();
                    child_start
                        .checked_add(len)
                        .expect("doc arena child count fits u32");
                    self.builder
                        .arena
                        .extend_children(&self.builder.list_scratch[self.start..]);
                    self.builder.push_node(DocNode::ConcatRange {
                        start: child_start,
                        len,
                    })
                };
                self.builder.list_scratch.truncate(self.start);
                doc
            }
        };
        self.active = false;
        doc
    }
}

impl Drop for ConcatBuilder<'_, '_> {
    fn drop(&mut self) {
        if self.active && self.builder.list_scratch.len() >= self.start {
            self.builder.list_scratch.truncate(self.start);
        }
    }
}

impl<'source> Deref for ConcatBuilder<'_, 'source> {
    type Target = DocBuilder<'source>;

    fn deref(&self) -> &Self::Target {
        self.builder
    }
}

impl DerefMut for ConcatBuilder<'_, '_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.builder
    }
}

struct ConcatAppender<'source> {
    inline: [Doc<'source>; INLINE_CONCAT_CAPACITY],
    start: Option<u32>,
    len: u32,
}

impl<'source> ConcatAppender<'source> {
    const fn new() -> Self {
        Self {
            inline: [Doc::nil(); INLINE_CONCAT_CAPACITY],
            start: None,
            len: 0,
        }
    }

    #[allow(
        clippy::inline_always,
        reason = "release profiles show concat append remains a hot out-of-line leaf"
    )]
    #[inline(always)]
    fn push(&mut self, doc: Doc<'source>, builder: &mut DocBuilder<'source>) {
        if doc.is_nil() {
            return;
        }

        match self.start {
            Some(_) => {
                builder.push_child(doc);
                self.len = self
                    .len
                    .checked_add(1)
                    .expect("concat child count fits u32");
            }
            None => {
                if self.len
                    < u32::try_from(INLINE_CONCAT_CAPACITY)
                        .expect("inline concat capacity fits u32")
                {
                    self.inline[usize::try_from(self.len).expect("concat length fits usize")] = doc;
                    self.len += 1;
                } else {
                    let start = builder.child_count();
                    builder.arena.extend_children(&self.inline);
                    builder.push_child(doc);
                    self.start = Some(start);
                    self.len += 1;
                }
            }
        }
    }

    fn finish(self, builder: &mut DocBuilder<'source>) -> Doc<'source> {
        match self.start {
            Some(start) => {
                start
                    .checked_add(self.len)
                    .expect("doc arena child count fits u32");
                builder.push_node(DocNode::ConcatRange {
                    start,
                    len: self.len,
                })
            }
            None => match self.len {
                0 => builder.nil(),
                1 => self.inline[0],
                len => builder.push_node(DocNode::InlineConcat {
                    docs: self.inline,
                    len: u8::try_from(len).expect("inline concat length fits u8"),
                }),
            },
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum DocNode<'source> {
    Text(DocumentText<'source>),
    InlineConcat {
        docs: [Doc<'source>; INLINE_CONCAT_CAPACITY],
        len: u8,
    },
    ConcatRange {
        start: u32,
        len: u32,
    },
    Group {
        contents: Doc<'source>,
        should_break: bool,
    },
    Indent {
        contents: Doc<'source>,
        levels: i16,
    },
    Line(Line),
    IfBreak {
        breaks: Doc<'source>,
        flat: Doc<'source>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct DocumentText<'source> {
    pub(crate) text: Cow<'source, str>,
    final_width: TextWidth,
    first_width: TextWidth,
    is_multiline: bool,
    #[cfg(debug_assertions)]
    pub(crate) claim: Option<SourceClaim<'source>>,
}

#[cfg(not(debug_assertions))]
const _: () = assert!(std::mem::size_of::<DocNode<'static>>() <= 40);

impl DocumentText<'_> {
    pub(crate) const fn final_width(&self) -> TextWidth {
        self.final_width
    }

    pub(crate) const fn first_width(&self) -> TextWidth {
        self.first_width
    }

    pub(crate) const fn is_multiline(&self) -> bool {
        self.is_multiline
    }
}

#[cfg(test)]
mod tests {
    use jolt_text::{TextRange, TextSize};

    use super::DocBuilder;
    use crate::formatter_ignore::FormatterIgnorePlan;

    fn range() -> TextRange {
        TextRange::new(TextSize::new(0), TextSize::new(1))
    }

    #[test]
    fn missing_formatter_ignore_plan_records_an_invariant() {
        let mut builder = DocBuilder::new();
        assert!(!builder.formatter_ignore_may_apply(range()));
        assert!(builder.into_arena().invariant_error().is_some());
    }

    #[test]
    fn empty_formatter_ignore_plan_never_applies() {
        let mut builder = DocBuilder::for_root(0, FormatterIgnorePlan::default());
        assert!(!builder.formatter_ignore_may_apply(range()));
        let mut called = false;
        assert!(
            builder
                .formatter_ignore_runs(
                    range(),
                    std::iter::once(()).map(|()| {
                        called = true;
                        None
                    })
                )
                .is_empty()
        );
        assert!(!called);
        assert!(builder.into_arena().invariant_error().is_none());
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Line {
    pub(crate) mode: LineMode,
    pub(crate) flat: FlatLine,
    pub(crate) indent_delta: i16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum LineMode {
    Soft,
    SoftOrSpace,
    Hard,
    Empty,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum FlatLine {
    Empty,
    Space,
}
