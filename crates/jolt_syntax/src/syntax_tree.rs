use std::ops::Range;

use jolt_text::{TextRange, TextSize};

use jolt_diagnostics::Diagnostic;

use crate::{Event, RawSyntaxKind};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct NodeId(usize);

impl NodeId {
    const fn new(index: usize) -> Self {
        Self(index)
    }

    pub(crate) const fn index(self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct TokenId(usize);

impl TokenId {
    const fn new(index: usize) -> Self {
        Self(index)
    }

    pub(crate) const fn index(self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum TreeElement {
    Node(NodeId),
    Token(TokenId),
}

/// A token trivia kind.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TriviaKind {
    /// Spaces or tabs.
    Whitespace,
    /// A line break.
    Newline,
    /// A `//` comment.
    LineComment,
    /// A `/* */` comment that is not documentation.
    BlockComment,
    /// A documentation comment.
    DocComment,
    /// Text ignored by the lexer.
    Ignored,
}

/// Trivia attached to a token.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SyntaxTrivia {
    kind: TriviaKind,
    text_len: TextSize,
}

impl SyntaxTrivia {
    #[must_use]
    pub const fn new(kind: TriviaKind, text_len: TextSize) -> Self {
        Self { kind, text_len }
    }

    #[must_use]
    pub const fn kind(self) -> TriviaKind {
        self.kind
    }

    #[must_use]
    pub const fn text_len(self) -> TextSize {
        self.text_len
    }
}

/// Token metadata stored once in the parse-owned syntax arena.
#[derive(Debug, Eq, Hash, PartialEq)]
pub struct SyntaxTokenData {
    pub(crate) kind: RawSyntaxKind,
    pub(crate) token_text_range: TextRange,
    pub(crate) text_len: TextSize,
    pub(crate) leading: Range<usize>,
    pub(crate) trailing: Range<usize>,
    pub(crate) parent: Option<NodeId>,
    pub(crate) offset: TextSize,
    pub(crate) index: usize,
}

impl SyntaxTokenData {
    #[must_use]
    pub fn new(
        kind: RawSyntaxKind,
        token_text_range: TextRange,
        leading: Range<usize>,
        trailing: Range<usize>,
        text_len: TextSize,
    ) -> Self {
        Self {
            kind,
            token_text_range,
            text_len,
            leading,
            trailing,
            parent: None,
            offset: TextSize::new(0),
            index: 0,
        }
    }

    #[must_use]
    pub const fn raw_kind(&self) -> RawSyntaxKind {
        self.kind
    }

    #[must_use]
    pub const fn token_text_range(&self) -> TextRange {
        self.token_text_range
    }

    #[must_use]
    pub fn leading(&self) -> &Range<usize> {
        &self.leading
    }

    #[must_use]
    pub fn trailing(&self) -> &Range<usize> {
        &self.trailing
    }
}

#[derive(Debug, Eq, Hash, PartialEq)]
pub(crate) struct TreeNode {
    pub(crate) kind: RawSyntaxKind,
    pub(crate) children: Range<usize>,
    pub(crate) parent: Option<NodeId>,
    pub(crate) offset: TextSize,
    pub(crate) text_len: TextSize,
    pub(crate) index: usize,
}

/// A flat syntax tree arena.
#[derive(Debug, Eq, Hash, PartialEq)]
pub struct SyntaxTree {
    root: NodeId,
    nodes: Vec<TreeNode>,
    children: Vec<TreeElement>,
    tokens: Vec<SyntaxTokenData>,
    trivia: Vec<SyntaxTrivia>,
}

impl SyntaxTree {
    #[must_use]
    pub(crate) const fn root(&self) -> NodeId {
        self.root
    }

    #[must_use]
    pub(crate) fn node(&self, id: NodeId) -> &TreeNode {
        &self.nodes[id.index()]
    }

    #[must_use]
    pub(crate) fn token(&self, id: TokenId) -> &SyntaxTokenData {
        &self.tokens[id.index()]
    }

    pub(crate) fn token_mut(&mut self, id: TokenId) -> &mut SyntaxTokenData {
        &mut self.tokens[id.index()]
    }

    pub(crate) fn node_mut(&mut self, id: NodeId) -> &mut TreeNode {
        &mut self.nodes[id.index()]
    }

    #[must_use]
    pub(crate) fn children(&self, id: NodeId) -> &[TreeElement] {
        let children = &self.node(id).children;
        &self.children[children.start..children.end]
    }

    #[must_use]
    pub(crate) fn trivia(&self, range: &Range<usize>) -> &[SyntaxTrivia] {
        &self.trivia[range.start..range.end]
    }
}

/// An event-to-tree construction error.
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
    InvalidForwardParent { position: usize, target: usize },
}

/// Builds a flat syntax tree from parser events and committed tokens.
///
/// Error events are collected as diagnostics and do not affect tree shape.
///
/// # Errors
///
/// Returns an error when the event stream is structurally invalid or does not
/// consume the supplied tokens exactly once.
pub fn build_syntax_tree(
    events: Vec<Event>,
    tokens: Vec<SyntaxTokenData>,
    trivia: Vec<SyntaxTrivia>,
) -> Result<(SyntaxTree, Vec<Diagnostic>), BuildSyntaxTreeError> {
    let builder = SyntaxTreeBuilder {
        nodes: Vec::new(),
        children: Vec::new(),
        pending_children: Vec::new(),
        tokens,
        trivia,
        stack: Vec::new(),
        root: None,
        token_index: 0,
        diagnostics: Vec::new(),
        skip_events: vec![false; events.len()],
    };

    builder.build(events)
}

struct SyntaxTreeBuilder {
    nodes: Vec<TreeNode>,
    children: Vec<TreeElement>,
    pending_children: Vec<TreeElement>,
    tokens: Vec<SyntaxTokenData>,
    trivia: Vec<SyntaxTrivia>,
    stack: Vec<PartialNode>,
    root: Option<NodeId>,
    token_index: usize,
    diagnostics: Vec<Diagnostic>,
    skip_events: Vec<bool>,
}

impl SyntaxTreeBuilder {
    fn build(
        mut self,
        mut events: Vec<Event>,
    ) -> Result<(SyntaxTree, Vec<Diagnostic>), BuildSyntaxTreeError> {
        let mut event_index = 0;

        while event_index < events.len() {
            if self.skip_events[event_index] {
                event_index += 1;
                continue;
            }

            match &events[event_index] {
                Event::StartNode {
                    kind,
                    forward_parent,
                } => {
                    if forward_parent.is_none() {
                        self.start_node(*kind);
                    } else {
                        self.start_forward_parent_nodes(&events, event_index)?;
                    }
                }
                Event::Token => self.push_token()?,
                Event::FinishNode => self.finish_node()?,
                Event::Error(_) => {
                    let Event::Error(diagnostic) =
                        std::mem::replace(&mut events[event_index], Event::Tombstone)
                    else {
                        unreachable!("event kind was checked before moving diagnostic")
                    };
                    self.diagnostics.push(diagnostic);
                }
                Event::Tombstone => {
                    return Err(BuildSyntaxTreeError::UnresolvedMarker {
                        position: event_index,
                    });
                }
            }

            event_index += 1;
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

        let root = self.root.ok_or(BuildSyntaxTreeError::MissingRoot)?;
        let mut tree = SyntaxTree {
            root,
            nodes: self.nodes,
            children: self.children,
            tokens: self.tokens,
            trivia: self.trivia,
        };
        assign_layout(&mut tree, root, None, TextSize::new(0), 0);

        Ok((tree, self.diagnostics))
    }

    fn start_node(&mut self, kind: RawSyntaxKind) {
        self.stack.push(PartialNode {
            kind,
            children_start: self.pending_children.len(),
            text_len: TextSize::new(0),
        });
    }

    fn start_forward_parent_nodes(
        &mut self,
        events: &[Event],
        position: usize,
    ) -> Result<(), BuildSyntaxTreeError> {
        let Event::StartNode {
            kind,
            forward_parent,
        } = events
            .get(position)
            .ok_or(BuildSyntaxTreeError::InvalidForwardParent {
                position,
                target: position,
            })?
        else {
            return Err(BuildSyntaxTreeError::InvalidForwardParent {
                position,
                target: position,
            });
        };

        if let Some(forward_parent) = forward_parent {
            let target = position.checked_add(*forward_parent).ok_or(
                BuildSyntaxTreeError::InvalidForwardParent {
                    position,
                    target: usize::MAX,
                },
            )?;

            if target <= position || target >= events.len() || self.skip_events[target] {
                return Err(BuildSyntaxTreeError::InvalidForwardParent { position, target });
            }

            self.skip_events[target] = true;
            self.start_forward_parent_nodes(events, target)?;
        }

        self.start_node(*kind);
        Ok(())
    }

    fn push_token(&mut self) -> Result<(), BuildSyntaxTreeError> {
        let parent = self
            .stack
            .last_mut()
            .ok_or(BuildSyntaxTreeError::TokenOutsideNode {
                token_index: self.token_index,
            })?;

        if self.token_index == self.tokens.len() {
            return Err(BuildSyntaxTreeError::MissingToken {
                token_index: self.token_index,
            });
        }

        let token = TokenId::new(self.token_index);
        parent.text_len += self.tokens[token.index()].text_len;
        self.pending_children.push(TreeElement::Token(token));
        self.token_index += 1;

        Ok(())
    }

    fn finish_node(&mut self) -> Result<(), BuildSyntaxTreeError> {
        let node = self
            .stack
            .pop()
            .ok_or(BuildSyntaxTreeError::UnexpectedFinishNode)?;
        let children_start = self.children.len();
        self.children
            .extend_from_slice(&self.pending_children[node.children_start..]);
        let children = children_start..self.children.len();
        self.pending_children.truncate(node.children_start);
        let node_id = NodeId::new(self.nodes.len());
        self.nodes.push(TreeNode {
            kind: node.kind,
            children,
            parent: None,
            offset: TextSize::new(0),
            text_len: node.text_len,
            index: 0,
        });

        if let Some(parent) = self.stack.last_mut() {
            parent.text_len += node.text_len;
            self.pending_children.push(TreeElement::Node(node_id));
        } else if self.root.replace(node_id).is_some() {
            return Err(BuildSyntaxTreeError::MultipleRoots);
        }

        Ok(())
    }
}

#[derive(Debug)]
struct PartialNode {
    kind: RawSyntaxKind,
    children_start: usize,
    text_len: TextSize,
}

fn assign_layout(
    tree: &mut SyntaxTree,
    node: NodeId,
    parent: Option<NodeId>,
    offset: TextSize,
    index: usize,
) {
    {
        let node_data = tree.node_mut(node);
        node_data.parent = parent;
        node_data.offset = offset;
        node_data.index = index;
    }

    let children = {
        let range = &tree.node(node).children;
        range.start..range.end
    };
    let mut child_offset = offset;
    for (child_index, child_position) in children.enumerate() {
        let child = tree.children[child_position];
        match child {
            TreeElement::Node(child_node) => {
                assign_layout(tree, child_node, Some(node), child_offset, child_index);
                child_offset += tree.node(child_node).text_len;
            }
            TreeElement::Token(token) => {
                let text_len = tree.token(token).text_len;
                let token_data = tree.token_mut(token);
                token_data.parent = Some(node);
                token_data.offset = child_offset;
                token_data.index = child_index;
                child_offset += text_len;
            }
        }
    }
}
