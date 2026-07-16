use std::{num::NonZeroU32, ops::Range};

use jolt_text::{TextRange, TextSize};

use crate::{Event, RawSyntaxKind, UnresolvedDiagnosticOwner, event::NO_FORWARD_PARENT};

/// Stable identity of a node in one parse-owned syntax tree.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SyntaxNodeId(pub(crate) u32);

/// Exact resolved syntax owner of one structural parser diagnostic.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SyntaxDiagnosticOwner {
    node: SyntaxNodeId,
    slot: Option<u16>,
}

impl SyntaxDiagnosticOwner {
    /// Returns the owning node identity.
    #[must_use]
    pub const fn node(self) -> SyntaxNodeId {
        self.node
    }

    /// Returns the generated slot index when this diagnostic owns an empty slot.
    #[must_use]
    pub const fn slot(self) -> Option<u16> {
        self.slot
    }
}

const MAX_PACKED_INDEX: u32 = (1 << 30) - 1;
const DIRECT_MALFORMED: u16 = 1 << 0;
const CONTAINS_RECOVERY: u16 = 1 << 1;
const PARSED_DIRECT_MALFORMED: u16 = 1 << 0;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct NodeId(NonZeroU32);

impl NodeId {
    fn new(index: usize) -> Self {
        let value = u32::try_from(index + 1).expect("syntax node index must fit in u32");
        Self(NonZeroU32::new(value).expect("syntax node ids are one-based"))
    }

    pub(crate) fn index(self) -> usize {
        self.0.get() as usize - 1
    }

    pub(crate) const fn index_u32(self) -> u32 {
        self.0.get() - 1
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct TokenId(NonZeroU32);

impl TokenId {
    pub(crate) fn new(index: usize) -> Self {
        let value = u32::try_from(index + 1).expect("syntax token index must fit in u32");
        Self(NonZeroU32::new(value).expect("syntax token ids are one-based"))
    }

    pub(crate) fn index(self) -> usize {
        self.0.get() as usize - 1
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum TreeElement {
    Node(NodeId),
    Token(TokenId),
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum TreeSlot {
    Node(NodeId),
    Token(TokenId),
    Empty,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct PackedSlot(u32);

impl PackedSlot {
    const EMPTY: Self = Self(3 << 30);

    fn node(id: NodeId) -> Self {
        let index = u32::try_from(id.index()).expect("syntax node index fits u32");
        assert!(
            index <= MAX_PACKED_INDEX,
            "syntax node index exceeds packed slot"
        );
        Self(index)
    }

    fn token(id: TokenId) -> Self {
        let index = u32::try_from(id.index()).expect("syntax token index fits u32");
        assert!(
            index <= MAX_PACKED_INDEX,
            "syntax token index exceeds packed slot"
        );
        Self((1 << 30) | index)
    }

    const fn is_node(self) -> bool {
        self.0 >> 30 == 0
    }

    const fn is_token(self) -> bool {
        self.0 >> 30 == 1
    }

    fn element(self) -> Option<TreeElement> {
        let index = usize::try_from(self.0 & MAX_PACKED_INDEX).expect("packed index fits usize");
        match self.0 >> 30 {
            0 => Some(TreeElement::Node(NodeId::new(index))),
            1 => Some(TreeElement::Token(TokenId::new(index))),
            3 => None,
            _ => unreachable!("reserved packed syntax slot tag"),
        }
    }

    fn slot(self) -> TreeSlot {
        match self.element() {
            Some(TreeElement::Node(id)) => TreeSlot::Node(id),
            Some(TreeElement::Token(id)) => TreeSlot::Token(id),
            None => TreeSlot::Empty,
        }
    }
}

/// A token trivia kind.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TriviaKind {
    Whitespace,
    Newline,
    LineComment,
    ShebangComment,
    BlockComment,
    DocComment,
    Ignored,
}

/// Trivia attached to a token.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SyntaxTrivia {
    kind: TriviaKind,
    text_len: u32,
}

impl SyntaxTrivia {
    /// Creates one compact trivia record.
    ///
    /// # Panics
    ///
    /// Panics when one trivia piece exceeds the tree's four-gibibyte source
    /// range limit.
    #[must_use]
    pub fn new(kind: TriviaKind, text_len: TextSize) -> Self {
        Self {
            kind,
            text_len: u32::try_from(text_len.get()).expect("trivia text length fits u32"),
        }
    }

    #[must_use]
    pub const fn kind(self) -> TriviaKind {
        self.kind
    }

    #[must_use]
    pub fn text_len(self) -> TextSize {
        TextSize::new(self.text_len as usize)
    }
}

/// Token metadata stored once in the parse-owned syntax arena.
#[derive(Debug, Eq, Hash, PartialEq)]
pub struct SyntaxTokenData {
    pub(crate) kind: RawSyntaxKind,
    full_text_range: CompactRange,
    token_text_range: CompactRange,
    leading: CompactRange,
    trailing: CompactRange,
}

impl SyntaxTokenData {
    /// Creates compact metadata for one represented source token.
    ///
    /// # Panics
    ///
    /// Panics when either source range exceeds the tree's four-gibibyte source
    /// range limit or either trivia range exceeds its `u32` index limit.
    #[must_use]
    pub fn new(
        kind: RawSyntaxKind,
        full_text_range: TextRange,
        token_text_range: TextRange,
        leading: Range<usize>,
        trailing: Range<usize>,
    ) -> Self {
        Self {
            kind,
            full_text_range: CompactRange::from_text_range(full_text_range, "full token range"),
            token_text_range: CompactRange::from_text_range(token_text_range, "token text range"),
            leading: CompactRange::from_usize(leading, "leading trivia range"),
            trailing: CompactRange::from_usize(trailing, "trailing trivia range"),
        }
    }

    #[must_use]
    pub const fn raw_kind(&self) -> RawSyntaxKind {
        self.kind
    }

    #[must_use]
    pub fn token_text_range(&self) -> TextRange {
        self.token_text_range.as_text_range()
    }

    #[must_use]
    pub fn full_text_range(&self) -> TextRange {
        self.full_text_range.as_text_range()
    }

    #[must_use]
    pub fn leading(&self) -> Range<usize> {
        self.leading.as_usize()
    }

    #[must_use]
    pub fn trailing(&self) -> Range<usize> {
        self.trailing.as_usize()
    }
}

#[derive(Debug, Eq, Hash, PartialEq)]
pub(crate) struct TreeNode {
    pub(crate) kind: RawSyntaxKind,
    flags: u16,
    children: CompactRange,
    tokens: CompactRange,
    parent: Option<NodeId>,
    index: u32,
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
struct CompactRange {
    start: u32,
    end: u32,
}

impl CompactRange {
    fn from_usize(range: Range<usize>, label: &str) -> Self {
        Self {
            start: u32::try_from(range.start).unwrap_or_else(|_| panic!("{label} start fits u32")),
            end: u32::try_from(range.end).unwrap_or_else(|_| panic!("{label} end fits u32")),
        }
    }

    fn as_usize(self) -> Range<usize> {
        self.start as usize..self.end as usize
    }

    fn from_text_range(range: TextRange, label: &str) -> Self {
        Self {
            start: u32::try_from(range.start().get())
                .unwrap_or_else(|_| panic!("{label} start fits u32")),
            end: u32::try_from(range.end().get())
                .unwrap_or_else(|_| panic!("{label} end fits u32")),
        }
    }

    fn as_text_range(self) -> TextRange {
        TextRange::new(
            TextSize::new(self.start as usize),
            TextSize::new(self.end as usize),
        )
    }
}

const _: () = {
    assert!(std::mem::size_of::<PackedSlot>() == 4);
    assert!(std::mem::size_of::<CompactRange>() == 8);
    assert!(std::mem::size_of::<TreeNode>() <= 28);
    assert!(std::mem::size_of::<SyntaxTokenData>() == 36);
    assert!(std::mem::size_of::<SyntaxTrivia>() == 8);
};

/// A flat, lossless syntax tree containing one uniform physical node model.
#[derive(Debug, Eq, Hash, PartialEq)]
pub struct SyntaxTree {
    root: NodeId,
    nodes: Vec<TreeNode>,
    slots: Vec<PackedSlot>,
    tokens: Vec<SyntaxTokenData>,
    trivia: Vec<SyntaxTrivia>,
}

#[cfg(feature = "bench")]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SyntaxTreeMetrics {
    pub nodes: usize,
    pub children: usize,
    pub tokens: usize,
    pub trivia: usize,
    pub logical_bytes: usize,
    pub reserved_bytes: usize,
}

impl SyntaxTree {
    #[must_use]
    pub(crate) fn token_count(&self) -> usize {
        self.tokens.len()
    }

    #[cfg(feature = "bench")]
    #[must_use]
    pub fn benchmark_metrics(&self) -> SyntaxTreeMetrics {
        use std::mem::size_of;
        let logical_bytes = self.nodes.len() * size_of::<TreeNode>()
            + self.slots.len() * size_of::<PackedSlot>()
            + self.tokens.len() * size_of::<SyntaxTokenData>()
            + self.trivia.len() * size_of::<SyntaxTrivia>();
        let reserved_bytes = self.nodes.capacity() * size_of::<TreeNode>()
            + self.slots.capacity() * size_of::<PackedSlot>()
            + self.tokens.capacity() * size_of::<SyntaxTokenData>()
            + self.trivia.capacity() * size_of::<SyntaxTrivia>();
        SyntaxTreeMetrics {
            nodes: self.nodes.len(),
            children: self.slots.len(),
            tokens: self.tokens.len(),
            trivia: self.trivia.len(),
            logical_bytes,
            reserved_bytes,
        }
    }

    pub(crate) const fn root(&self) -> NodeId {
        self.root
    }

    pub(crate) fn node(&self, id: NodeId) -> &TreeNode {
        &self.nodes[id.index()]
    }

    pub(crate) fn token(&self, id: TokenId) -> &SyntaxTokenData {
        &self.tokens[id.index()]
    }

    pub(crate) fn parent(&self, id: NodeId) -> Option<NodeId> {
        self.node(id).parent
    }

    pub(crate) fn index(&self, id: NodeId) -> u32 {
        self.node(id).index
    }

    pub(crate) fn is_directly_malformed(&self, id: NodeId) -> bool {
        self.node(id).flags & DIRECT_MALFORMED != 0
    }

    pub(crate) fn is_recovery_free(&self, id: NodeId) -> bool {
        self.node(id).flags & CONTAINS_RECOVERY == 0
    }

    pub(crate) fn children(&self, id: NodeId) -> impl Iterator<Item = TreeElement> + '_ {
        let node = self.node(id);
        self.slots[node.children.start as usize..node.children.end as usize]
            .iter()
            .filter_map(|slot| slot.element())
    }

    pub(crate) fn slot_count(&self, id: NodeId) -> usize {
        let node = self.node(id);
        (node.children.end - node.children.start) as usize
    }

    pub(crate) fn slot_at(&self, id: NodeId, index: usize) -> Option<TreeSlot> {
        let node = self.node(id);
        self.slots
            .get(node.children.start as usize + index)
            .filter(|_| index < self.slot_count(id))
            .map(|slot| slot.slot())
    }

    pub(crate) fn child_at(&self, id: NodeId, index: usize) -> Option<TreeElement> {
        let node = self.node(id);
        self.slots
            .get(node.children.start as usize + index)
            .filter(|_| index < self.slot_count(id))
            .and_then(|slot| slot.element())
    }

    pub(crate) fn token_range(&self, id: NodeId) -> Range<usize> {
        let node = self.node(id);
        node.tokens.start as usize..node.tokens.end as usize
    }

    pub(crate) fn token_offset(&self, id: TokenId) -> TextSize {
        self.token(id).full_text_range().start()
    }

    pub(crate) fn node_offset(&self, id: NodeId) -> TextSize {
        let range = self.token_range(id);
        self.token_anchor(range.start)
    }

    pub(crate) fn node_text_len(&self, id: NodeId) -> TextSize {
        let range = self.token_range(id);
        if range.start == range.end {
            return TextSize::new(0);
        }
        self.tokens[range.end - 1].full_text_range().end() - self.token_anchor(range.start)
    }

    fn token_anchor(&self, index: usize) -> TextSize {
        self.tokens.get(index).map_or_else(
            || {
                self.tokens
                    .last()
                    .map_or(TextSize::new(0), |token| token.full_text_range().end())
            },
            |token| token.full_text_range().start(),
        )
    }

    pub(crate) fn trivia(&self, range: &Range<usize>) -> &[SyntaxTrivia] {
        &self.trivia[range.start..range.end]
    }

    #[cfg(debug_assertions)]
    pub(crate) fn token_data(&self) -> &[SyntaxTokenData] {
        &self.tokens
    }

    #[cfg(debug_assertions)]
    pub(crate) const fn trivia_len(&self) -> usize {
        self.trivia.len()
    }

    pub(crate) fn trivia_at(&self, index: usize) -> SyntaxTrivia {
        self.trivia[index]
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum BuildSyntaxTreeError {
    TokenOutsideNode { token_index: usize },
    MissingToken { token_index: usize },
    UnexpectedFinishNode,
    MultipleRoots,
    UnclosedNodes { count: usize },
    MissingRoot,
    UnconsumedTokens { first_unconsumed: usize },
    UnresolvedMarker { position: usize },
    UnexpectedConsumedEvent { position: usize },
    InvalidForwardParent { position: usize, target: usize },
    UnresolvedDiagnosticOwner { diagnostic: usize, anchor: usize },
    FactoryMismatch { kind: RawSyntaxKind },
}

#[derive(Clone, Copy)]
struct ParsedChild {
    element: PackedSlot,
    kind: RawSyntaxKind,
    flags: u16,
}

impl ParsedChild {
    fn node(id: NodeId, kind: RawSyntaxKind, directly_malformed: bool) -> Self {
        Self {
            element: PackedSlot::node(id),
            kind,
            flags: u16::from(directly_malformed) * PARSED_DIRECT_MALFORMED,
        }
    }

    fn token(id: TokenId, kind: RawSyntaxKind) -> Self {
        Self {
            element: PackedSlot::token(id),
            kind,
            flags: 0,
        }
    }

    fn element(self) -> TreeElement {
        self.element
            .element()
            .expect("pending parser children cannot contain empty slots")
    }

    const fn is_node(self) -> bool {
        self.element.is_node()
    }

    const fn is_token(self) -> bool {
        self.element.is_token()
    }

    const fn is_directly_malformed(self) -> bool {
        self.flags & PARSED_DIRECT_MALFORMED != 0
    }
}

const _: () = assert!(std::mem::size_of::<ParsedChild>() == 8);

/// Borrowed direct parser children supplied to a language syntax factory.
#[doc(hidden)]
#[derive(Clone, Copy)]
pub struct ParsedChildren<'a> {
    children: &'a [ParsedChild],
    tokens: &'a [SyntaxTokenData],
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

    #[must_use]
    pub fn kind(self, index: usize) -> Option<RawSyntaxKind> {
        self.children.get(index).map(|child| child.kind)
    }

    #[must_use]
    pub fn is_node(self, index: usize) -> bool {
        self.children
            .get(index)
            .is_some_and(|child| child.is_node())
    }

    #[must_use]
    pub fn is_token(self, index: usize) -> bool {
        self.children
            .get(index)
            .is_some_and(|child| child.is_token())
    }

    /// Returns whether the parser/syntax layer assigned malformed ownership
    /// directly to this child node.
    #[must_use]
    pub fn is_directly_malformed(self, index: usize) -> bool {
        self.children
            .get(index)
            .is_some_and(|child| child.is_directly_malformed())
    }

    #[must_use]
    pub fn token_text_is(self, index: usize, expected: &str) -> bool {
        self.token_text(index) == Some(expected)
    }

    #[must_use]
    pub fn token_text(self, index: usize) -> Option<&'a str> {
        let child = self.children.get(index)?;
        let TreeElement::Token(id) = child.element() else {
            return None;
        };
        let range = self.tokens[id.index()].token_text_range();
        Some(&self.source[range.start().get()..range.end().get()])
    }
}

/// A node constructed by a syntax factory.
#[doc(hidden)]
#[derive(Clone, Copy)]
pub struct FactoryNode(NodeId);

/// One final physical node slot selected by a syntax factory.
#[doc(hidden)]
#[derive(Clone, Copy)]
pub enum FactorySlot {
    Input(usize),
    /// A valid unoccupied optional grammar slot.
    Absent,
    /// A required grammar slot missing through parser recovery.
    Missing,
}

/// Language-owned node placement over parser-produced direct children.
#[doc(hidden)]
pub trait SyntaxFactory {
    fn make_syntax(
        &self,
        kind: RawSyntaxKind,
        children: ParsedChildren<'_>,
        sink: &mut SyntaxTreeSink<'_>,
    ) -> Result<FactoryNode, BuildSyntaxTreeError>;
}

/// Append-only tree sink used by generated syntax factories.
#[doc(hidden)]
pub struct SyntaxTreeSink<'a> {
    nodes: &'a mut Vec<TreeNode>,
    slots: &'a mut Vec<PackedSlot>,
    input: &'a [ParsedChild],
    parser_tokens: Range<usize>,
}

impl SyntaxTreeSink<'_> {
    #[must_use]
    pub fn raw(&mut self, kind: RawSyntaxKind) -> FactoryNode {
        self.push_node(
            kind,
            self.parser_tokens.clone(),
            (0..self.input.len()).map(FactorySlot::Input),
            false,
        )
    }

    /// Preserves parser-owned malformed children in their raw represented order.
    #[must_use]
    pub fn raw_malformed(&mut self, kind: RawSyntaxKind) -> FactoryNode {
        self.push_node(
            kind,
            self.parser_tokens.clone(),
            (0..self.input.len()).map(FactorySlot::Input),
            true,
        )
    }

    #[must_use]
    pub fn fixed(
        &mut self,
        kind: RawSyntaxKind,
        slots: impl IntoIterator<Item = FactorySlot>,
    ) -> FactoryNode {
        self.push_node(kind, self.parser_tokens.clone(), slots, false)
    }

    fn push_node(
        &mut self,
        kind: RawSyntaxKind,
        tokens: Range<usize>,
        final_slots: impl IntoIterator<Item = FactorySlot>,
        directly_malformed: bool,
    ) -> FactoryNode {
        let children_start = u32::try_from(self.slots.len()).expect("syntax slot offset fits u32");
        let node_id = NodeId::new(self.nodes.len());
        let mut contains_recovery = directly_malformed;
        for (index, slot) in final_slots.into_iter().enumerate() {
            let packed = match slot {
                FactorySlot::Input(input) => match self.input[input].element() {
                    TreeElement::Node(child) => {
                        contains_recovery |=
                            self.nodes[child.index()].flags & CONTAINS_RECOVERY != 0;
                        let child_node = &mut self.nodes[child.index()];
                        child_node.parent = Some(node_id);
                        child_node.index = u32::try_from(index).expect("child index fits u32");
                        PackedSlot::node(child)
                    }
                    TreeElement::Token(token) => PackedSlot::token(token),
                },
                FactorySlot::Absent => PackedSlot::EMPTY,
                FactorySlot::Missing => {
                    contains_recovery = true;
                    PackedSlot::EMPTY
                }
            };
            self.slots.push(packed);
        }
        let children_end = u32::try_from(self.slots.len()).expect("syntax slot offset fits u32");
        let mut flags = 0;
        if directly_malformed {
            flags |= DIRECT_MALFORMED;
        }
        if contains_recovery {
            flags |= CONTAINS_RECOVERY;
        }
        self.nodes.push(TreeNode {
            kind,
            flags,
            children: CompactRange {
                start: children_start,
                end: children_end,
            },
            tokens: CompactRange {
                start: u32::try_from(tokens.start).expect("token offset fits u32"),
                end: u32::try_from(tokens.end).expect("token offset fits u32"),
            },
            parent: None,
            index: 0,
        });
        FactoryNode(node_id)
    }
}

#[doc(hidden)]
pub fn build_syntax_tree_with_factory(
    source: &str,
    events: Vec<Event>,
    tokens: Vec<SyntaxTokenData>,
    trivia: Vec<SyntaxTrivia>,
    factory: &impl SyntaxFactory,
) -> Result<SyntaxTree, BuildSyntaxTreeError> {
    SyntaxTreeBuilder::new(tokens, trivia, events.len(), false)
        .build(source, events, factory)
        .map(|(tree, _)| tree)
}

#[doc(hidden)]
pub fn build_syntax_tree_with_factory_and_diagnostic_owners(
    source: &str,
    events: Vec<Event>,
    tokens: Vec<SyntaxTokenData>,
    trivia: Vec<SyntaxTrivia>,
    owners: &[Option<UnresolvedDiagnosticOwner>],
    factory: &impl SyntaxFactory,
) -> Result<(SyntaxTree, Vec<Option<SyntaxDiagnosticOwner>>), BuildSyntaxTreeError> {
    let resolve = owners.iter().any(Option::is_some);
    let (tree, event_nodes) = SyntaxTreeBuilder::new(tokens, trivia, events.len(), resolve)
        .build(source, events, factory)?;
    let mut resolved = Vec::with_capacity(owners.len());
    for (diagnostic, owner) in owners.iter().enumerate() {
        let Some(owner) = *owner else {
            resolved.push(None);
            continue;
        };
        let node = event_nodes
            .as_ref()
            .and_then(|nodes| nodes.get(owner.node.0))
            .copied()
            .flatten()
            .ok_or(BuildSyntaxTreeError::UnresolvedDiagnosticOwner {
                diagnostic,
                anchor: owner.node.0,
            })?;
        resolved.push(Some(SyntaxDiagnosticOwner {
            node: SyntaxNodeId(u32::try_from(node.index()).expect("syntax node index fits u32")),
            slot: owner.slot,
        }));
    }
    Ok((tree, resolved))
}

struct SyntaxTreeBuilder {
    nodes: Vec<TreeNode>,
    slots: Vec<PackedSlot>,
    pending: Vec<ParsedChild>,
    tokens: Vec<SyntaxTokenData>,
    trivia: Vec<SyntaxTrivia>,
    stack: Vec<PartialNode>,
    root: Option<NodeId>,
    token_index: usize,
    event_nodes: Option<Vec<Option<NodeId>>>,
}

type BuiltSyntaxTree = (SyntaxTree, Option<Vec<Option<NodeId>>>);

impl SyntaxTreeBuilder {
    fn new(
        tokens: Vec<SyntaxTokenData>,
        trivia: Vec<SyntaxTrivia>,
        event_count: usize,
        resolve_diagnostic_owners: bool,
    ) -> Self {
        let token_count = tokens.len();
        // Every completed physical node contributes one start and one finish
        // event, while every represented token contributes one token event.
        // Reserve the exact valid-stream node count without scanning events.
        let node_count = event_count.saturating_sub(token_count) / 2;
        Self {
            nodes: Vec::with_capacity(node_count),
            slots: Vec::with_capacity(event_count),
            pending: Vec::with_capacity(64),
            tokens,
            trivia,
            stack: Vec::with_capacity(64),
            root: None,
            token_index: 0,
            event_nodes: resolve_diagnostic_owners.then(|| vec![None; event_count]),
        }
    }

    fn build(
        mut self,
        source: &str,
        mut events: Vec<Event>,
        factory: &impl SyntaxFactory,
    ) -> Result<BuiltSyntaxTree, BuildSyntaxTreeError> {
        for (position, event) in events.iter().enumerate() {
            match event {
                Event::Tombstone => {
                    return Err(BuildSyntaxTreeError::UnresolvedMarker { position });
                }
                Event::Consumed => {
                    return Err(BuildSyntaxTreeError::UnexpectedConsumedEvent { position });
                }
                Event::Start { .. } | Event::Token | Event::Finish => {}
            }
        }
        for position in 0..events.len() {
            match events[position] {
                Event::Start {
                    kind,
                    forward_parent,
                } => {
                    if forward_parent == NO_FORWARD_PARENT {
                        self.start_node(kind, position);
                    } else {
                        self.start_forward_parents(&mut events, position)?;
                    }
                }
                Event::Token => self.push_token()?,
                Event::Finish => self.finish_node(source, factory)?,
                Event::Tombstone => {
                    return Err(BuildSyntaxTreeError::UnresolvedMarker { position });
                }
                Event::Consumed => {}
            }
        }
        if !self.stack.is_empty() {
            return Err(BuildSyntaxTreeError::UnclosedNodes {
                count: self.stack.len(),
            });
        }
        if self.token_index < self.tokens.len() {
            return Err(BuildSyntaxTreeError::UnconsumedTokens {
                first_unconsumed: self.token_index,
            });
        }
        let tree = SyntaxTree {
            root: self.root.ok_or(BuildSyntaxTreeError::MissingRoot)?,
            nodes: self.nodes,
            slots: self.slots,
            tokens: self.tokens,
            trivia: self.trivia,
        };
        Ok((tree, self.event_nodes))
    }

    fn start_node(&mut self, kind: RawSyntaxKind, anchor: usize) {
        self.stack.push(PartialNode {
            kind,
            children_start: self.pending.len(),
            tokens_start: self.token_index,
            anchor,
        });
    }

    fn start_forward_parents(
        &mut self,
        events: &mut [Event],
        position: usize,
    ) -> Result<(), BuildSyntaxTreeError> {
        let stack_start = self.stack.len();
        let mut current = position;
        loop {
            let Event::Start {
                kind,
                forward_parent,
            } = events[current]
            else {
                return Err(BuildSyntaxTreeError::InvalidForwardParent {
                    position: current,
                    target: current,
                });
            };
            if current != position {
                events[current] = Event::Consumed;
            }
            self.start_node(kind, current);
            if forward_parent == NO_FORWARD_PARENT {
                break;
            }
            let target = current.checked_add(forward_parent as usize).ok_or(
                BuildSyntaxTreeError::InvalidForwardParent {
                    position: current,
                    target: usize::MAX,
                },
            )?;
            if target <= current || target >= events.len() {
                return Err(BuildSyntaxTreeError::InvalidForwardParent {
                    position: current,
                    target,
                });
            }
            current = target;
        }
        // Forward parents are encountered from the innermost node outwards,
        // while the construction stack must hold the outermost node first.
        // Reuse the construction stack itself as the bounded scratch space.
        self.stack[stack_start..].reverse();
        Ok(())
    }

    fn push_token(&mut self) -> Result<(), BuildSyntaxTreeError> {
        if self.stack.is_empty() {
            return Err(BuildSyntaxTreeError::TokenOutsideNode {
                token_index: self.token_index,
            });
        }
        if self.token_index == self.tokens.len() {
            return Err(BuildSyntaxTreeError::MissingToken {
                token_index: self.token_index,
            });
        }
        self.pending.push(ParsedChild::token(
            TokenId::new(self.token_index),
            self.tokens[self.token_index].kind,
        ));
        self.token_index += 1;
        Ok(())
    }

    fn finish_node(
        &mut self,
        source: &str,
        factory: &impl SyntaxFactory,
    ) -> Result<(), BuildSyntaxTreeError> {
        let partial = self
            .stack
            .pop()
            .ok_or(BuildSyntaxTreeError::UnexpectedFinishNode)?;
        let factory_children = &self.pending[partial.children_start..];
        let input = ParsedChildren {
            children: factory_children,
            tokens: &self.tokens,
            source,
        };
        let mut sink = SyntaxTreeSink {
            nodes: &mut self.nodes,
            slots: &mut self.slots,
            input: factory_children,
            parser_tokens: partial.tokens_start..self.token_index,
        };
        let FactoryNode(node) = factory.make_syntax(partial.kind, input, &mut sink)?;
        if let Some(event_nodes) = &mut self.event_nodes {
            event_nodes[partial.anchor] = Some(node);
        }
        self.pending.truncate(partial.children_start);
        if self.stack.is_empty() {
            if self.root.replace(node).is_some() {
                return Err(BuildSyntaxTreeError::MultipleRoots);
            }
        } else {
            self.pending.push(ParsedChild::node(
                node,
                self.nodes[node.index()].kind,
                self.nodes[node.index()].flags & DIRECT_MALFORMED != 0,
            ));
        }
        Ok(())
    }
}

struct PartialNode {
    kind: RawSyntaxKind,
    children_start: usize,
    tokens_start: usize,
    anchor: usize,
}

#[cfg(test)]
mod tests {
    use crate::{BuildSyntaxTreeError, Event, RawSyntaxKind};

    use super::{
        FactoryNode, ParsedChildren, SyntaxFactory, SyntaxTreeSink, build_syntax_tree_with_factory,
    };

    struct TestFactory;

    impl SyntaxFactory for TestFactory {
        fn make_syntax(
            &self,
            kind: RawSyntaxKind,
            _children: ParsedChildren<'_>,
            sink: &mut SyntaxTreeSink<'_>,
        ) -> Result<FactoryNode, BuildSyntaxTreeError> {
            Ok(sink.raw(kind))
        }
    }

    fn build(events: Vec<Event>) -> Result<super::SyntaxTree, BuildSyntaxTreeError> {
        build_syntax_tree_with_factory("", events, Vec::new(), Vec::new(), &TestFactory)
    }

    #[test]
    fn construction_only_consumed_event_is_rejected_from_input() {
        let events = vec![
            Event::Start {
                kind: RawSyntaxKind::new(1),
                forward_parent: 0,
            },
            Event::Consumed,
            Event::Finish,
        ];

        let error = build(events).expect_err("caller-provided consumed event must be rejected");
        assert_eq!(
            error,
            BuildSyntaxTreeError::UnexpectedConsumedEvent { position: 1 }
        );
    }

    #[test]
    fn deep_forward_parent_chain_builds_iteratively() {
        const DEPTH: usize = 16_384;
        let mut events = Vec::with_capacity(DEPTH * 2);
        for index in 0..DEPTH {
            events.push(Event::Start {
                kind: RawSyntaxKind::new(1),
                forward_parent: u32::from(index + 1 < DEPTH),
            });
        }
        events.extend(std::iter::repeat_n(Event::Finish, DEPTH));

        let tree = build(events).expect("deep forward-parent chain is a valid tree");

        assert_eq!(tree.nodes.len(), DEPTH);
    }
}
