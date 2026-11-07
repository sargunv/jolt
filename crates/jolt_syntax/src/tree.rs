//! The flat, lossless syntax tree arena and its builder.

use std::ops::Range;

use crate::{Diagnostic, Event, Language, ParseOutput, RawSyntaxKind, TextRange, TextSize};

/// A node's index in its tree.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NodeId(pub(crate) u32);

/// A token's index in its tree, in source order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TokenId(pub(crate) u32);

impl NodeId {
    #[must_use]
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

impl TokenId {
    #[must_use]
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TriviaKind {
    Whitespace,
    Newline,
    LineComment,
    BlockComment,
    DocComment,
    UnterminatedBlockComment,
    ByteOrderMark,
    TrailingSubstitute,
}

/// One trivia piece as stored: its kind and byte length.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Trivia {
    pub kind: TriviaKind,
    pub len: u32,
}

impl Trivia {
    #[must_use]
    pub const fn new(kind: TriviaKind, len: TextSize) -> Self {
        Self {
            kind,
            len: len.raw(),
        }
    }
}

/// A token as stored: its kind, text range, and trivia. A token's leading and
/// trailing trivia are adjacent in the trivia vector.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TokenData {
    kind: RawSyntaxKind,
    pub(crate) parent: u32,
    full_start: u32,
    start: u32,
    end: u32,
    trivia: [u32; 3],
}

const NO_PARENT: u32 = u32::MAX;

impl TokenData {
    /// # Panics
    ///
    /// Panics when the trailing trivia does not directly follow the leading
    /// trivia, or an index exceeds `u32`.
    #[must_use]
    pub fn new(
        kind: RawSyntaxKind,
        full_start: TextSize,
        range: TextRange,
        leading: Range<usize>,
        trailing: Range<usize>,
    ) -> Self {
        assert_eq!(leading.end, trailing.start, "token trivia must be adjacent");
        let index = |value: usize| u32::try_from(value).expect("trivia index fits in u32");
        Self {
            kind,
            parent: NO_PARENT,
            full_start: full_start.raw(),
            start: range.start().raw(),
            end: range.end().raw(),
            trivia: [
                index(leading.start),
                index(leading.end),
                index(trailing.end),
            ],
        }
    }

    #[must_use]
    pub const fn kind(&self) -> RawSyntaxKind {
        self.kind
    }

    /// The token text range, without trivia.
    #[must_use]
    pub fn range(&self) -> TextRange {
        TextRange::new(self.start.into(), self.end.into())
    }

    /// The start of the token's leading trivia.
    #[must_use]
    pub fn full_start(&self) -> TextSize {
        self.full_start.into()
    }

    #[must_use]
    pub const fn leading(&self) -> Range<usize> {
        self.trivia[0] as usize..self.trivia[1] as usize
    }

    #[must_use]
    pub const fn trailing(&self) -> Range<usize> {
        self.trivia[1] as usize..self.trivia[2] as usize
    }

    /// Moves the token, e.g. from translated to raw source coordinates.
    pub fn set_ranges(&mut self, full_start: TextSize, range: TextRange) {
        self.full_start = full_start.raw();
        self.start = range.start().raw();
        self.end = range.end().raw();
    }

    /// Splits off the first `len` bytes of the token as a token of kind
    /// `first`, which keeps the leading trivia. `self` becomes the remainder
    /// with kind `rest` and keeps the trailing trivia.
    pub(crate) fn split_off_first(
        &mut self,
        len: u32,
        first: RawSyntaxKind,
        rest: RawSyntaxKind,
    ) -> Self {
        let head = Self {
            kind: first,
            parent: NO_PARENT,
            full_start: self.full_start,
            start: self.start,
            end: self.start + len,
            trivia: [self.trivia[0], self.trivia[1], self.trivia[1]],
        };
        self.kind = rest;
        self.start += len;
        self.full_start = self.start;
        self.trivia[0] = self.trivia[1];
        head
    }
}

pub(crate) const MALFORMED: u16 = 1;
pub(crate) const HAS_RECOVERY: u16 = 2;

#[derive(Clone, Copy, Debug)]
pub(crate) struct NodeData {
    pub(crate) kind: RawSyntaxKind,
    pub(crate) flags: u16,
    pub(crate) slots: [u32; 2],
    pub(crate) tokens: [u32; 2],
    pub(crate) parent: u32,
    pub(crate) index: u32,
}

const _: () = {
    assert!(size_of::<NodeData>() == 28);
    assert!(size_of::<TokenData>() == 32);
    assert!(size_of::<Trivia>() == 8);
};

const TAG_SHIFT: u32 = 30;
const INDEX_MASK: u32 = (1 << TAG_SHIFT) - 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PackedSlot(u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Slot {
    Node(NodeId),
    Token(TokenId),
    Empty,
}

impl PackedSlot {
    const EMPTY: Self = Self(3 << TAG_SHIFT);

    fn node(id: NodeId) -> Self {
        assert!(id.0 <= INDEX_MASK, "syntax tree exceeds its node limit");
        Self(id.0)
    }

    fn token(id: TokenId) -> Self {
        assert!(id.0 <= INDEX_MASK, "syntax tree exceeds its token limit");
        Self((1 << TAG_SHIFT) | id.0)
    }

    pub(crate) const fn unpack(self) -> Slot {
        match self.0 >> TAG_SHIFT {
            0 => Slot::Node(NodeId(self.0 & INDEX_MASK)),
            1 => Slot::Token(TokenId(self.0 & INDEX_MASK)),
            _ => Slot::Empty,
        }
    }
}

/// One parse's tree: nodes, a shared slot vector, tokens, and trivia, all
/// indexed by `u32`.
pub struct SyntaxTree {
    pub(crate) root: NodeId,
    pub(crate) nodes: Vec<NodeData>,
    pub(crate) slots: Vec<PackedSlot>,
    pub(crate) tokens: Vec<TokenData>,
    pub(crate) trivia: Vec<Trivia>,
    pub(crate) kind_name: fn(RawSyntaxKind) -> &'static str,
}

impl SyntaxTree {
    /// The number of nodes, so per-node tables can be indexed by
    /// [`NodeId::index`].
    #[must_use]
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// The number of tokens (including `Eof`), so per-token tables can be
    /// indexed by [`TokenId::index`].
    #[must_use]
    pub fn token_count(&self) -> usize {
        self.tokens.len()
    }

    pub(crate) fn node(&self, id: NodeId) -> &NodeData {
        &self.nodes[id.index()]
    }

    pub(crate) fn token(&self, id: TokenId) -> &TokenData {
        &self.tokens[id.index()]
    }

    pub(crate) fn node_slots(&self, id: NodeId) -> &[PackedSlot] {
        let [start, end] = self.node(id).slots;
        &self.slots[start as usize..end as usize]
    }
}

impl std::fmt::Debug for SyntaxTree {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SyntaxTree")
            .field("nodes", &self.nodes.len())
            .field("tokens", &self.tokens.len())
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Copy)]
struct Child {
    slot: PackedSlot,
    kind: RawSyntaxKind,
}

/// The direct children the parser produced for one node, handed to the
/// language's factory.
#[doc(hidden)]
#[derive(Clone, Copy)]
pub struct ParsedChildren<'a> {
    children: &'a [Child],
    tokens: &'a [TokenData],
    source: &'a str,
}

impl<'a> ParsedChildren<'a> {
    #[must_use]
    pub const fn len(self) -> usize {
        self.children.len()
    }

    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.children.is_empty()
    }

    /// The kind of child `index`; `None` past the end.
    #[must_use]
    pub fn kind(self, index: usize) -> Option<RawSyntaxKind> {
        self.children.get(index).map(|child| child.kind)
    }

    #[must_use]
    pub fn is_node(self, index: usize) -> bool {
        self.children
            .get(index)
            .is_some_and(|child| matches!(child.slot.unpack(), Slot::Node(_)))
    }

    #[must_use]
    pub fn is_token(self, index: usize) -> bool {
        self.children
            .get(index)
            .is_some_and(|child| matches!(child.slot.unpack(), Slot::Token(_)))
    }

    /// The text of child `index` when it is a token.
    #[must_use]
    pub fn token_text(self, index: usize) -> Option<&'a str> {
        let Slot::Token(id) = self.children.get(index)?.slot.unpack() else {
            return None;
        };
        let range = self.tokens[id.index()].range();
        Some(&self.source[range.start().get()..range.end().get()])
    }
}

/// Where a factory puts one fixed slot.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FactorySlot {
    /// The parsed child at this index.
    Child(usize),
    /// A valid absent optional element.
    Absent,
    /// A missing required element.
    Missing,
}

/// Appends factory-built nodes to the arena.
#[doc(hidden)]
pub struct NodeSink<'a> {
    nodes: &'a mut Vec<NodeData>,
    slots: &'a mut Vec<PackedSlot>,
    input: &'a [Child],
    tokens: [u32; 2],
    token_data: &'a [TokenData],
    bogus: RawSyntaxKind,
    mismatches: &'a mut Mismatches,
}

/// Children that fit no slot of the node the factory built.
#[derive(Default)]
struct Mismatches {
    /// Each one, wrapped in a bogus node or kept raw in a malformed node.
    unexpected: Vec<Unexpected>,
    /// Tokens whose parent is a bogus wrapper rather than the new node.
    wrapped_tokens: Vec<(TokenId, NodeId)>,
}

/// A factory mismatch the parser may not have reported.
struct Unexpected {
    range: TextRange,
    kind: RawSyntaxKind,
    parent: RawSyntaxKind,
}

impl NodeSink<'_> {
    /// A node whose children are placed into fixed slots.
    pub fn fixed(
        &mut self,
        kind: RawSyntaxKind,
        slots: impl IntoIterator<Item = FactorySlot>,
    ) -> NodeId {
        let input = self.input;
        self.push(
            kind,
            slots.into_iter().map(|slot| match slot {
                FactorySlot::Child(index) => Some(input[index].slot),
                FactorySlot::Absent => Some(PackedSlot::EMPTY),
                FactorySlot::Missing => None,
            }),
            false,
        )
    }

    /// A node that keeps every child in order, in one slot each. A malformed
    /// node other than the bogus kind is a factory mismatch.
    pub fn raw(&mut self, kind: RawSyntaxKind, malformed: bool) -> NodeId {
        if malformed && kind != self.bogus {
            self.record_malformed(kind);
        }
        let input = self.input;
        self.push(kind, input.iter().map(|child| Some(child.slot)), malformed)
    }

    /// A list node. A child that does not match `item` is wrapped in a bogus
    /// node, which every list accepts at an item position, so a list is never
    /// malformed. Fewer than `minimum` items leave missing slots.
    pub fn list(
        &mut self,
        kind: RawSyntaxKind,
        len: usize,
        minimum: usize,
        item: impl Fn(usize) -> bool,
    ) -> NodeId {
        let input = self.input;
        if len >= minimum && (0..len).all(&item) {
            return self.push(kind, input.iter().map(|child| Some(child.slot)), false);
        }
        let mut slots: Vec<Option<PackedSlot>> = (0..len)
            .map(|index| {
                Some(if item(index) {
                    input[index].slot
                } else {
                    PackedSlot::node(self.wrap(kind, index))
                })
            })
            .collect();
        slots.resize(len.max(minimum), None);
        self.push(kind, slots.into_iter(), false)
    }

    /// A separated list node. Items and separators alternate; a missing item
    /// or separator gets an empty slot, as does a forbidden trailing or
    /// required absent final separator. A child that fits neither is wrapped
    /// in a bogus node at an item position.
    pub fn separated_list(
        &mut self,
        kind: RawSyntaxKind,
        len: usize,
        minimum: usize,
        trailing: Trailing,
        item: impl Fn(usize) -> bool,
        separator: impl Fn(usize) -> bool,
    ) -> NodeId {
        let input = self.input;
        let mut slots: Vec<Option<PackedSlot>> = Vec::with_capacity(len.max(minimum * 2));
        let mut cursor = 0;
        let mut count = 0;
        let mut expect_item = true;
        while cursor < len {
            let (fits, other) = if expect_item {
                (item(cursor), separator(cursor))
            } else {
                (separator(cursor), item(cursor))
            };
            if fits {
                slots.push(Some(input[cursor].slot));
                count += usize::from(expect_item);
                cursor += 1;
            } else if other || !expect_item {
                // A misfit after an item takes the next item position.
                slots.push(None);
            } else {
                slots.push(Some(PackedSlot::node(self.wrap(kind, cursor))));
                count += 1;
                cursor += 1;
            }
            expect_item = !expect_item;
        }
        match trailing {
            Trailing::Forbidden if expect_item && !slots.is_empty() => slots.push(None),
            Trailing::Required if !expect_item && count > 0 => slots.push(None),
            _ => {}
        }
        while count < minimum {
            if count > 0 {
                slots.push(None);
            }
            slots.push(None);
            count += 1;
        }
        self.push(kind, slots.into_iter(), false)
    }

    /// Wraps input child `index` of a `parent` list in a bogus node.
    #[cold]
    #[inline(never)]
    fn wrap(&mut self, parent: RawSyntaxKind, index: usize) -> NodeId {
        let child = self.input[index];
        let tokens = match child.slot.unpack() {
            Slot::Node(node) => self.nodes[node.index()].tokens,
            Slot::Token(token) => [token.0, token.0 + 1],
            Slot::Empty => unreachable!("parsed children are never empty"),
        };
        self.mismatches.unexpected.push(Unexpected {
            range: self.token_span(tokens),
            kind: child.kind,
            parent,
        });
        let outer = std::mem::replace(&mut self.tokens, tokens);
        let id = self.push(self.bogus, std::iter::once(Some(child.slot)), true);
        self.tokens = outer;
        if let Slot::Token(token) = child.slot.unpack() {
            self.mismatches.wrapped_tokens.push((token, id));
        }
        id
    }

    #[cold]
    #[inline(never)]
    fn record_malformed(&mut self, kind: RawSyntaxKind) {
        self.mismatches.unexpected.push(Unexpected {
            range: self.token_span(self.tokens),
            kind,
            parent: kind,
        });
    }

    /// From the first token's text start to the last token's text end.
    fn token_span(&self, [first, end]: [u32; 2]) -> TextRange {
        let first = first as usize;
        let end = end as usize;
        if first == end {
            let at = self
                .token_data
                .get(first)
                .map_or(TextSize::from(0), |token| token.range().start());
            return TextRange::empty(at);
        }
        TextRange::new(
            self.token_data[first].range().start(),
            self.token_data[end - 1].range().end(),
        )
    }

    /// `None` is a missing required slot.
    fn push(
        &mut self,
        kind: RawSyntaxKind,
        slots: impl Iterator<Item = Option<PackedSlot>>,
        malformed: bool,
    ) -> NodeId {
        let id = NodeId(u32::try_from(self.nodes.len()).expect("node count fits in u32"));
        let start = u32::try_from(self.slots.len()).expect("slot count fits in u32");
        let mut recovery = malformed;
        for (index, slot) in slots.enumerate() {
            let slot = slot.unwrap_or_else(|| {
                recovery = true;
                PackedSlot::EMPTY
            });
            if let Slot::Node(child) = slot.unpack() {
                let child = &mut self.nodes[child.index()];
                recovery |= child.flags & HAS_RECOVERY != 0;
                child.parent = id.0;
                child.index = u32::try_from(index).expect("slot index fits in u32");
            }
            self.slots.push(slot);
        }
        let end = u32::try_from(self.slots.len()).expect("slot count fits in u32");
        let mut flags = 0;
        if malformed {
            flags |= MALFORMED;
        }
        if recovery {
            flags |= HAS_RECOVERY;
        }
        self.nodes.push(NodeData {
            kind,
            flags,
            slots: [start, end],
            tokens: self.tokens,
            parent: NO_PARENT,
            index: 0,
        });
        id
    }
}

/// Whether a separated list may or must end with a separator.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Trailing {
    Forbidden,
    Optional,
    Required,
}

/// A language's generated node constructor.
#[doc(hidden)]
pub trait SyntaxFactory {
    type Language: Language;

    /// Places `children` into the fixed slots of `kind`, or stores them raw
    /// and marks the node `MALFORMED` when they do not fit. Infallible.
    fn make(
        &self,
        kind: RawSyntaxKind,
        children: ParsedChildren<'_>,
        sink: &mut NodeSink<'_>,
    ) -> NodeId;
}

/// Builds the tree from a parse.
///
/// # Panics
///
/// Panics when the event stream is unbalanced or does not consume every
/// token. The marker API cannot produce such a stream, so this is a parser
/// bug.
pub fn build_tree<F: SyntaxFactory>(
    source: &str,
    output: ParseOutput,
    factory: &F,
) -> (SyntaxTree, Vec<Diagnostic>) {
    let ParseOutput {
        mut events,
        mut tokens,
        trivia,
        diagnostics,
    } = output;
    let token_count = tokens.len();
    let mut nodes = Vec::with_capacity(events.len().saturating_sub(token_count) / 2);
    let mut slots = Vec::with_capacity(events.len());
    let mut pending: Vec<Child> = Vec::with_capacity(64);
    // (kind, first pending child, first token)
    let mut stack: Vec<(RawSyntaxKind, usize, usize)> = Vec::with_capacity(64);
    let mut next_token = 0usize;
    let mut root = None;
    let mut mismatches = Mismatches::default();

    for position in 0..events.len() {
        match events[position] {
            Event::Start { .. } => {
                open_nodes(
                    &mut events,
                    position,
                    &mut stack,
                    (pending.len(), next_token),
                );
            }
            Event::Token => {
                assert!(!stack.is_empty(), "token outside of any node");
                let id = TokenId(u32::try_from(next_token).expect("token count fits in u32"));
                pending.push(Child {
                    slot: PackedSlot::token(id),
                    kind: tokens[next_token].kind,
                });
                next_token += 1;
            }
            Event::Finish => {
                let (kind, children_start, tokens_start) =
                    stack.pop().expect("finish event without a start");
                let input = &pending[children_start..];
                let children = ParsedChildren {
                    children: input,
                    tokens: &tokens,
                    source,
                };
                let mut sink = NodeSink {
                    nodes: &mut nodes,
                    slots: &mut slots,
                    input,
                    tokens: [
                        u32::try_from(tokens_start).expect("token count fits in u32"),
                        u32::try_from(next_token).expect("token count fits in u32"),
                    ],
                    token_data: &tokens,
                    bogus: <F::Language as Language>::to_raw(<F::Language as Language>::BOGUS),
                    mismatches: &mut mismatches,
                };
                let id = factory.make(kind, children, &mut sink);
                for child in input {
                    if let Slot::Token(token) = child.slot.unpack() {
                        tokens[token.index()].parent = id.0;
                    }
                }
                if !mismatches.wrapped_tokens.is_empty() {
                    for (token, wrapper) in mismatches.wrapped_tokens.drain(..) {
                        tokens[token.index()].parent = wrapper.0;
                    }
                }
                pending.truncate(children_start);
                if stack.is_empty() {
                    assert!(root.replace(id).is_none(), "event stream has two roots");
                } else {
                    pending.push(Child {
                        slot: PackedSlot::node(id),
                        kind,
                    });
                }
            }
            Event::Tombstone => {}
        }
    }
    assert!(stack.is_empty(), "event stream leaves nodes open");
    assert_eq!(
        next_token, token_count,
        "event stream leaves tokens unconsumed"
    );
    let tree = SyntaxTree {
        root: root.expect("event stream has no root"),
        nodes,
        slots,
        tokens,
        trivia,
        kind_name: <F::Language as Language>::kind_name,
    };
    let diagnostics = report_unexpected::<F::Language>(diagnostics, mismatches.unexpected);
    (tree, diagnostics)
}

#[inline]
/// Pushes the node started at `position` and its forward parents onto
/// `stack`, outermost first, each starting at `at` (first pending child,
/// first token).
fn open_nodes(
    events: &mut [Event],
    position: usize,
    stack: &mut Vec<(RawSyntaxKind, usize, usize)>,
    at: (usize, usize),
) {
    let stack_start = stack.len();
    let mut current = position;
    loop {
        let Event::Start {
            kind,
            forward_parent,
        } = events[current]
        else {
            panic!("forward parent must point at a start event");
        };
        if current != position {
            events[current] = Event::Tombstone;
        }
        stack.push((kind, at.0, at.1));
        if forward_parent == 0 {
            break;
        }
        current += forward_parent as usize;
    }
    // Forward parents are found innermost first; the stack needs the
    // outermost first.
    stack[stack_start..].reverse();
}

/// Adds an "unexpected" diagnostic for each factory mismatch that no parser
/// diagnostic (or earlier mismatch) overlaps, so a tree with a malformed node
/// always has a diagnostic. `O((d + u) log (d + u))`.
fn report_unexpected<L: Language>(
    mut diagnostics: Vec<Diagnostic>,
    mut unexpected: Vec<Unexpected>,
) -> Vec<Diagnostic> {
    if unexpected.is_empty() {
        return diagnostics;
    }
    let mut reported: Vec<TextRange> = diagnostics.iter().map(|d| d.range).collect();
    reported.sort_unstable_by_key(|range| range.start());
    // The furthest end among the diagnostics up to each index.
    let mut furthest = Vec::with_capacity(reported.len());
    let mut end = TextSize::from(0);
    for range in &reported {
        end = end.max(range.end());
        furthest.push(end);
    }
    let overlaps_parser = |range: TextRange| {
        let before = reported.partition_point(|reported| reported.start() <= range.end());
        before > 0 && furthest[before - 1] >= range.start()
    };
    unexpected.sort_by_key(|item| (item.range.start(), std::cmp::Reverse(item.range.end())));
    let mut accepted_end: Option<TextSize> = None;
    for item in unexpected {
        if overlaps_parser(item.range) || accepted_end.is_some_and(|end| end >= item.range.start())
        {
            continue;
        }
        accepted_end = Some(accepted_end.map_or(item.range.end(), |end| end.max(item.range.end())));
        let message = if item.kind == item.parent {
            format!("unexpected syntax in {}", L::kind_name(item.parent))
        } else {
            format!(
                "unexpected {} in {}",
                L::kind_name(item.kind),
                L::kind_name(item.parent)
            )
        };
        diagnostics.push(Diagnostic {
            code: L::UNEXPECTED_CODE,
            message,
            range: item.range,
        });
    }
    diagnostics.sort_by_key(|diagnostic| diagnostic.range.start());
    diagnostics
}
