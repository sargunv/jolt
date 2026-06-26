use crate::{
    Diagnostic, Event, GreenElement, GreenNode, GreenToken, GreenTrivia, RawSyntaxKind, TriviaKind,
};

/// A borrowed trivia piece supplied by a token source.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GreenTriviaPiece<'a> {
    kind: TriviaKind,
    text: &'a str,
}

impl<'a> GreenTriviaPiece<'a> {
    /// Creates a borrowed trivia piece.
    #[must_use]
    pub const fn new(kind: TriviaKind, text: &'a str) -> Self {
        Self { kind, text }
    }

    /// Returns the trivia kind.
    #[must_use]
    pub const fn kind(self) -> TriviaKind {
        self.kind
    }

    /// Returns the trivia text.
    #[must_use]
    pub const fn text(self) -> &'a str {
        self.text
    }
}

/// A language-specific token source consumed by the shared green tree sink.
pub trait GreenTokenSource {
    /// Returns the number of tokens in the source.
    fn token_count(&self) -> usize;

    /// Returns the raw kind for the token at `index`.
    fn token_kind(&self, index: usize) -> RawSyntaxKind;

    /// Returns the token text without attached trivia for the token at `index`.
    fn token_text(&self, index: usize) -> &str;

    /// Returns trivia attached before the token at `index`.
    fn leading_trivia(&self, index: usize) -> impl Iterator<Item = GreenTriviaPiece<'_>>;

    /// Returns trivia attached after the token at `index`.
    fn trailing_trivia(&self, index: usize) -> impl Iterator<Item = GreenTriviaPiece<'_>>;
}

/// A green tree and the parser diagnostics collected while building it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GreenTree {
    root: GreenNode,
    diagnostics: Vec<Diagnostic>,
}

impl GreenTree {
    /// Creates a green tree parse result.
    #[must_use]
    pub fn new(root: GreenNode, diagnostics: Vec<Diagnostic>) -> Self {
        Self { root, diagnostics }
    }

    /// Returns the root green node.
    #[must_use]
    pub const fn root(&self) -> &GreenNode {
        &self.root
    }

    /// Returns parser diagnostics collected during tree construction.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Splits this result into its root node and diagnostics.
    #[must_use]
    pub fn into_parts(self) -> (GreenNode, Vec<Diagnostic>) {
        (self.root, self.diagnostics)
    }
}

/// An event-to-green construction error.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BuildGreenTreeError {
    /// A token event appeared when no node was open.
    TokenOutsideNode { token_index: usize },
    /// A token event had no corresponding token in the token source.
    MissingToken { token_index: usize },
    /// A finish-node event appeared when no node was open.
    UnexpectedFinishNode,
    /// More than one root node was completed.
    MultipleRoots,
    /// The event stream ended with unclosed nodes.
    UnclosedNodes { count: usize },
    /// The event stream ended without producing a root node.
    MissingRoot,
    /// The event stream did not consume every token in the source.
    UnconsumedTokens { first_unconsumed: usize },
    /// A parser marker placeholder remained in the finished event stream.
    UnresolvedMarker { position: usize },
    /// A forward-parent link pointed at an invalid event.
    InvalidForwardParent { position: usize, target: usize },
}

/// Builds a green tree from parser events and a token source.
///
/// Error events are collected as diagnostics and do not affect tree shape.
///
/// # Errors
///
/// Returns an error when the event stream is structurally invalid or does not
/// consume the supplied token source exactly once.
pub fn build_green_tree(
    events: &[Event],
    token_source: &impl GreenTokenSource,
) -> Result<GreenTree, BuildGreenTreeError> {
    let mut stack = Vec::<PartialGreenNode>::new();
    let mut root = None;
    let mut token_index = 0;
    let mut diagnostics = Vec::new();
    let mut skip_events = vec![false; events.len()];
    let mut event_index = 0;

    while event_index < events.len() {
        if skip_events[event_index] {
            event_index += 1;
            continue;
        }

        match &events[event_index] {
            Event::StartNode { .. } => {
                for kind in start_node_kinds(events, &mut skip_events, event_index)?
                    .into_iter()
                    .rev()
                {
                    stack.push(PartialGreenNode {
                        kind,
                        children: Vec::new(),
                    });
                }
            }
            Event::Token => {
                if stack.is_empty() {
                    return Err(BuildGreenTreeError::TokenOutsideNode { token_index });
                }

                if token_index == token_source.token_count() {
                    return Err(BuildGreenTreeError::MissingToken { token_index });
                }

                let token = GreenToken::with_trivia(
                    token_source.token_kind(token_index),
                    token_source.token_text(token_index),
                    token_source
                        .leading_trivia(token_index)
                        .map(|trivia| GreenTrivia::new(trivia.kind(), trivia.text())),
                    token_source
                        .trailing_trivia(token_index)
                        .map(|trivia| GreenTrivia::new(trivia.kind(), trivia.text())),
                );

                push_child(&mut stack, token.into());
                token_index += 1;
            }
            Event::FinishNode => {
                let node = stack
                    .pop()
                    .ok_or(BuildGreenTreeError::UnexpectedFinishNode)?;
                let node = GreenNode::new(node.kind, node.children);

                if stack.is_empty() {
                    if root.replace(node).is_some() {
                        return Err(BuildGreenTreeError::MultipleRoots);
                    }
                } else {
                    push_child(&mut stack, node.into());
                }
            }
            Event::Error(diagnostic) => diagnostics.push(diagnostic.clone()),
            Event::Tombstone => {
                return Err(BuildGreenTreeError::UnresolvedMarker {
                    position: event_index,
                });
            }
        }

        event_index += 1;
    }

    if !stack.is_empty() {
        return Err(BuildGreenTreeError::UnclosedNodes { count: stack.len() });
    }

    if token_index < token_source.token_count() {
        return Err(BuildGreenTreeError::UnconsumedTokens {
            first_unconsumed: token_index,
        });
    }

    let root = root.ok_or(BuildGreenTreeError::MissingRoot)?;

    Ok(GreenTree::new(root, diagnostics))
}

#[derive(Debug)]
struct PartialGreenNode {
    kind: RawSyntaxKind,
    children: Vec<GreenElement>,
}

fn start_node_kinds(
    events: &[Event],
    skip_events: &mut [bool],
    position: usize,
) -> Result<Vec<RawSyntaxKind>, BuildGreenTreeError> {
    let mut position = position;
    let mut kinds = Vec::new();

    loop {
        let Event::StartNode {
            kind,
            forward_parent,
        } = events
            .get(position)
            .ok_or(BuildGreenTreeError::InvalidForwardParent {
                position,
                target: position,
            })?
        else {
            return Err(BuildGreenTreeError::InvalidForwardParent {
                position,
                target: position,
            });
        };

        kinds.push(*kind);

        let Some(forward_parent) = forward_parent else {
            break;
        };

        let target = position.checked_add(*forward_parent).ok_or(
            BuildGreenTreeError::InvalidForwardParent {
                position,
                target: usize::MAX,
            },
        )?;

        if target <= position || target >= events.len() {
            return Err(BuildGreenTreeError::InvalidForwardParent { position, target });
        }

        if skip_events[target] {
            return Err(BuildGreenTreeError::InvalidForwardParent { position, target });
        }

        skip_events[target] = true;
        position = target;
    }

    Ok(kinds)
}

fn push_child(stack: &mut [PartialGreenNode], child: GreenElement) {
    stack
        .last_mut()
        .expect("checked non-empty stack before pushing child")
        .children
        .push(child);
}
