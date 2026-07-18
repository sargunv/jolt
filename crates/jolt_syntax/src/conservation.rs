use std::fmt;

use jolt_text::{TextRange, TextSize};

use crate::{
    Language, SyntaxNode, SyntaxToken, SyntaxTrivia, TriviaKind,
    syntax_tree::{NodeId, SyntaxTree, TokenId},
};

/// The parse-local identity of a represented syntax node.
#[derive(Clone, Copy)]
pub struct SourceNodeId<'tree> {
    pub(crate) tree: &'tree SyntaxTree,
    pub(crate) id: NodeId,
}

impl SourceNodeId<'_> {
    fn belongs_to(self, tree: &SyntaxTree) -> bool {
        std::ptr::eq(self.tree, tree)
    }

    pub(crate) fn contains_token(self, token: SourceTokenId<'_>) -> bool {
        token.belongs_to(self.tree) && self.tree.token_range(self.id).contains(&token.id.index())
    }

    pub(crate) fn immediately_precedes(self, token: SourceTokenId<'_>) -> bool {
        token.belongs_to(self.tree) && self.tree.token_range(self.id).end == token.id.index()
    }
}

impl fmt::Debug for SourceNodeId<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("SourceNodeId")
            .field(&self.id.index())
            .finish()
    }
}

impl PartialEq for SourceNodeId<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.belongs_to(other.tree) && self.id == other.id
    }
}

impl Eq for SourceNodeId<'_> {}

impl std::hash::Hash for SourceNodeId<'_> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::ptr::from_ref(self.tree).hash(state);
        self.id.hash(state);
    }
}

/// Identifies which side of a source token owns a trivia piece.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SourceTriviaSide {
    Leading,
    Trailing,
}

/// The parse-local identity of a represented source token.
#[derive(Clone, Copy)]
pub struct SourceTokenId<'tree> {
    pub(crate) tree: &'tree SyntaxTree,
    pub(crate) id: TokenId,
}

impl SourceTokenId<'_> {
    pub(crate) fn belongs_to(self, tree: &SyntaxTree) -> bool {
        std::ptr::eq(self.tree, tree)
    }
}

impl fmt::Debug for SourceTokenId<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("SourceTokenId")
            .field(&self.id.index())
            .finish()
    }
}

impl PartialEq for SourceTokenId<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.belongs_to(other.tree) && self.id == other.id
    }
}

impl Eq for SourceTokenId<'_> {}

impl std::hash::Hash for SourceTokenId<'_> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::ptr::from_ref(self.tree).hash(state);
        self.id.hash(state);
    }
}

/// The parse-local identity of a represented source trivia piece.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SourceTriviaId<'tree> {
    token: SourceTokenId<'tree>,
    side: SourceTriviaSide,
    ordinal: usize,
}

impl<'tree> SourceTriviaId<'tree> {
    #[cfg(debug_assertions)]
    pub(crate) const fn new(
        token: SourceTokenId<'tree>,
        side: SourceTriviaSide,
        ordinal: usize,
    ) -> Self {
        Self {
            token,
            side,
            ordinal,
        }
    }
}

/// A represented source identity handled by formatter output.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SourceIdentity<'tree> {
    Token(SourceTokenId<'tree>),
    Trivia(SourceTriviaId<'tree>),
}

impl<'tree> SourceIdentity<'tree> {
    pub(crate) const fn token_id(self) -> SourceTokenId<'tree> {
        match self {
            Self::Token(token) => token,
            Self::Trivia(trivia) => trivia.token,
        }
    }
}

/// A trivia piece paired with its parse-local identity and exact source range.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SourceTriviaPiece<'tree> {
    id: SourceTriviaId<'tree>,
    trivia: SyntaxTrivia,
    text_range: TextRange,
}

impl<'tree> SourceTriviaPiece<'tree> {
    #[cfg(debug_assertions)]
    pub(crate) const fn new(
        id: SourceTriviaId<'tree>,
        trivia: SyntaxTrivia,
        text_range: TextRange,
    ) -> Self {
        Self {
            id,
            trivia,
            text_range,
        }
    }
    #[must_use]
    pub const fn id(self) -> SourceTriviaId<'tree> {
        self.id
    }

    #[must_use]
    pub const fn trivia(self) -> SyntaxTrivia {
        self.trivia
    }

    #[must_use]
    pub const fn text_range(self) -> TextRange {
        self.text_range
    }
}

pub(crate) struct SourceTriviaPieces<'tree> {
    token: SourceTokenId<'tree>,
    side: SourceTriviaSide,
    trivia: &'tree [SyntaxTrivia],
    next: usize,
    offset: TextSize,
}

impl<'tree> SourceTriviaPieces<'tree> {
    pub(crate) const fn new(
        token: SourceTokenId<'tree>,
        side: SourceTriviaSide,
        trivia: &'tree [SyntaxTrivia],
        offset: TextSize,
    ) -> Self {
        Self {
            token,
            side,
            trivia,
            next: 0,
            offset,
        }
    }
}

impl<'tree> Iterator for SourceTriviaPieces<'tree> {
    type Item = SourceTriviaPiece<'tree>;

    fn next(&mut self) -> Option<Self::Item> {
        let ordinal = self.next;
        let trivia = *self.trivia.get(ordinal)?;
        self.next += 1;
        let text_range = TextRange::new(self.offset, self.offset + trivia.text_len());
        self.offset = text_range.end();
        Some(SourceTriviaPiece {
            id: SourceTriviaId {
                token: self.token,
                side: self.side,
                ordinal,
            },
            trivia,
            text_range,
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.trivia.len() - self.next;
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for SourceTriviaPieces<'_> {}

/// A deterministic debug/test conservation failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConservationError {
    ForeignToken,
    ForeignTrivia,
    ForeignSourceRange,
    UnownedTrivia {
        trivia: usize,
    },
    UnauthorizedToken {
        token: usize,
        range: TextRange,
    },
    DuplicateToken {
        token: usize,
    },
    DuplicateTrivia {
        token: usize,
        side: SourceTriviaSide,
        ordinal: usize,
    },
    UnauthorizedTrivia {
        token: usize,
        side: SourceTriviaSide,
        ordinal: usize,
        kind: TriviaKind,
        range: TextRange,
    },
    MissingToken {
        token: usize,
        range: TextRange,
    },
    MissingTrivia {
        token: usize,
        side: SourceTriviaSide,
        ordinal: usize,
        kind: TriviaKind,
        range: TextRange,
    },
}

/// A syntax-tree-branded source range used by formatter-ignore output.
///
/// The formatter can select a parser-backed range, but it cannot enumerate or
/// fabricate the identities that range conserves.
#[derive(Clone, Copy)]
pub struct SourceRangeClaim<'tree> {
    #[cfg(debug_assertions)]
    tree: &'tree SyntaxTree,
    #[cfg(debug_assertions)]
    range: TextRange,
    #[cfg(debug_assertions)]
    include_line_ending_at_end: bool,
    #[cfg(debug_assertions)]
    token_start: usize,
    #[cfg(debug_assertions)]
    token_end: usize,
    #[cfg(not(debug_assertions))]
    marker: std::marker::PhantomData<&'tree SyntaxTree>,
}

impl fmt::Debug for SourceRangeClaim<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        #[cfg(debug_assertions)]
        return formatter
            .debug_tuple("SourceRangeClaim")
            .field(&self.range)
            .finish();
        #[cfg(not(debug_assertions))]
        formatter.write_str("SourceRangeClaim")
    }
}

impl PartialEq for SourceRangeClaim<'_> {
    fn eq(&self, other: &Self) -> bool {
        #[cfg(debug_assertions)]
        return std::ptr::eq(self.tree, other.tree)
            && self.range == other.range
            && self.include_line_ending_at_end == other.include_line_ending_at_end
            && self.token_start == other.token_start
            && self.token_end == other.token_end;
        #[cfg(not(debug_assertions))]
        {
            let _ = other;
            true
        }
    }
}

impl Eq for SourceRangeClaim<'_> {}

impl<'tree> SourceRangeClaim<'tree> {
    pub(crate) fn new<L: Language>(
        token: &SyntaxToken<'tree, L>,
        range: TextRange,
        include_line_ending_at_end: bool,
    ) -> Self {
        assert!(
            range.end().get() <= token.source().len(),
            "source claim range belongs to its syntax source"
        );
        Self::from_tree(token.source_id().tree, range, include_line_ending_at_end)
    }

    fn from_tree(
        tree: &'tree SyntaxTree,
        range: TextRange,
        include_line_ending_at_end: bool,
    ) -> Self {
        #[cfg(debug_assertions)]
        {
            let tokens = tree.token_data();
            let token_start =
                tokens.partition_point(|token| token.full_text_range().end() <= range.start());
            let token_end = tokens.partition_point(|token| {
                if include_line_ending_at_end {
                    token.full_text_range().start() <= range.end()
                } else {
                    token.full_text_range().start() < range.end()
                }
            });
            Self {
                tree,
                range,
                include_line_ending_at_end,
                token_start,
                token_end,
            }
        }
        #[cfg(not(debug_assertions))]
        {
            let _ = (tree, range, include_line_ending_at_end);
            Self {
                marker: std::marker::PhantomData,
            }
        }
    }
}

impl fmt::Display for ConservationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for ConservationError {}

#[cfg(debug_assertions)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ClaimState {
    NotConserved,
    Unclaimed,
    Claimed,
}

/// Root-level dense source identity accounting for debug and test formatters.
///
/// Optimized builds retain this API as a zero-sized no-op and allocate no
/// tracking state. The syntax tree itself never stores formatter state.
pub struct SyntaxConservationTracker<'tree> {
    #[cfg(debug_assertions)]
    tree: &'tree SyntaxTree,
    #[cfg(debug_assertions)]
    tokens: Vec<ClaimState>,
    #[cfg(debug_assertions)]
    trivia: Vec<ClaimState>,
    #[cfg(not(debug_assertions))]
    marker: std::marker::PhantomData<&'tree SyntaxTree>,
}

#[cfg(not(debug_assertions))]
const _: () = assert!(std::mem::size_of::<SyntaxConservationTracker<'static>>() == 0);

impl<'tree> SyntaxConservationTracker<'tree> {
    pub(crate) fn new<L: Language>(root: &SyntaxNode<'tree, L>) -> Self {
        #[cfg(debug_assertions)]
        {
            let tree = root.tree();
            let eof = L::kind_to_raw(L::eof_kind());
            let tokens = tree
                .token_data()
                .iter()
                .map(|token| {
                    if token.raw_kind() == eof {
                        ClaimState::NotConserved
                    } else {
                        ClaimState::Unclaimed
                    }
                })
                .collect();
            let mut trivia = vec![ClaimState::NotConserved; tree.trivia_len()];
            mark_conserved_trivia(tree, &mut trivia);
            Self {
                tree,
                tokens,
                trivia,
            }
        }
        #[cfg(not(debug_assertions))]
        {
            let _ = root;
            Self {
                marker: std::marker::PhantomData,
            }
        }
    }

    /// Claims one represented source identity.
    ///
    /// # Errors
    ///
    /// Returns an error for foreign, unauthorized, or duplicate identities.
    pub fn claim(&mut self, identity: SourceIdentity<'tree>) -> Result<(), ConservationError> {
        match identity {
            SourceIdentity::Token(token) => self.claim_token(token),
            SourceIdentity::Trivia(trivia) => self.claim_trivia(trivia),
        }
    }

    /// Claims one represented source token.
    ///
    /// # Errors
    ///
    /// Returns an error for a foreign, unauthorized, or duplicate token.
    pub fn claim_token(&mut self, identity: SourceTokenId<'tree>) -> Result<(), ConservationError> {
        #[cfg(debug_assertions)]
        {
            if !identity.belongs_to(self.tree) {
                return Err(ConservationError::ForeignToken);
            }
            match self.tokens[identity.id.index()] {
                ClaimState::NotConserved => {
                    return Err(ConservationError::UnauthorizedToken {
                        token: identity.id.index(),
                        range: self.tree.token(identity.id).token_text_range(),
                    });
                }
                ClaimState::Unclaimed => {
                    self.tokens[identity.id.index()] = ClaimState::Claimed;
                }
                ClaimState::Claimed => {
                    return Err(ConservationError::DuplicateToken {
                        token: identity.id.index(),
                    });
                }
            }
        }
        #[cfg(not(debug_assertions))]
        let _ = identity;
        Ok(())
    }

    /// Validates a token identity without consuming it.
    ///
    /// # Errors
    ///
    /// Returns an error for a foreign or unauthorized token.
    pub fn validate_token(&self, identity: SourceTokenId<'tree>) -> Result<(), ConservationError> {
        #[cfg(debug_assertions)]
        {
            if !identity.belongs_to(self.tree) {
                return Err(ConservationError::ForeignToken);
            }
            if self.tokens[identity.id.index()] == ClaimState::NotConserved {
                return Err(ConservationError::UnauthorizedToken {
                    token: identity.id.index(),
                    range: self.tree.token(identity.id).token_text_range(),
                });
            }
        }
        #[cfg(not(debug_assertions))]
        let _ = identity;
        Ok(())
    }

    /// Claims one represented conserved trivia piece.
    ///
    /// # Errors
    ///
    /// Returns an error for foreign, unowned, unauthorized, or duplicate trivia.
    pub fn claim_trivia(
        &mut self,
        identity: SourceTriviaId<'tree>,
    ) -> Result<(), ConservationError> {
        #[cfg(debug_assertions)]
        {
            let Some(index) = self.trivia_index(identity) else {
                return Err(ConservationError::ForeignTrivia);
            };
            match self.trivia[index] {
                ClaimState::NotConserved => {
                    let Some((_, _, _, range)) = self.trivia_identity_for_index(index) else {
                        return Err(ConservationError::UnownedTrivia { trivia: index });
                    };
                    return Err(ConservationError::UnauthorizedTrivia {
                        token: identity.token.id.index(),
                        side: identity.side,
                        ordinal: identity.ordinal,
                        kind: self.tree.trivia_at(index).kind(),
                        range,
                    });
                }
                ClaimState::Unclaimed => self.trivia[index] = ClaimState::Claimed,
                ClaimState::Claimed => {
                    return Err(ConservationError::DuplicateTrivia {
                        token: identity.token.id.index(),
                        side: identity.side,
                        ordinal: identity.ordinal,
                    });
                }
            }
        }
        #[cfg(not(debug_assertions))]
        let _ = identity;
        Ok(())
    }

    /// Claims identities selected by one parser-backed formatter-ignore range.
    ///
    /// # Errors
    ///
    /// Returns an error when the range belongs to a different syntax tree or
    /// claims an unauthorized or already-consumed identity.
    pub fn claim_source_range(
        &mut self,
        claim: SourceRangeClaim<'tree>,
    ) -> Result<(), ConservationError> {
        #[cfg(debug_assertions)]
        {
            if !std::ptr::eq(claim.tree, self.tree) {
                return Err(ConservationError::ForeignSourceRange);
            }
            for index in claim.token_start..claim.token_end {
                let token_id = SourceTokenId {
                    tree: self.tree,
                    id: TokenId::new(index),
                };
                let token = self.tree.token(token_id.id);
                for piece in
                    source_trivia_pieces(token_id, SourceTriviaSide::Leading, token.leading())
                        .chain(source_trivia_pieces(
                            token_id,
                            SourceTriviaSide::Trailing,
                            token.trailing(),
                        ))
                {
                    let in_range = range_contains(claim.range, piece.text_range())
                        || (claim.include_line_ending_at_end
                            && piece.trivia().kind() == TriviaKind::Newline
                            && piece.text_range().start() == claim.range.end());
                    if in_range && is_conserved_trivia(piece.id()) {
                        self.claim_trivia(piece.id())?;
                    }
                }
                let token_range = token.token_text_range();
                if token_range.start() != token_range.end()
                    && range_contains(claim.range, token_range)
                {
                    self.claim_token(token_id)?;
                }
            }
        }
        #[cfg(not(debug_assertions))]
        let _ = claim;
        Ok(())
    }

    /// Completes this root conservation proof.
    ///
    /// # Errors
    ///
    /// Returns the first deterministic missing or unowned source identity.
    pub fn finish(self) -> Result<(), ConservationError> {
        #[cfg(debug_assertions)]
        {
            for (index, state) in self.tokens.iter().enumerate() {
                if *state == ClaimState::Unclaimed {
                    return Err(ConservationError::MissingToken {
                        token: index,
                        range: self.tree.token(TokenId::new(index)).token_text_range(),
                    });
                }
            }
            for (index, state) in self.trivia.iter().enumerate() {
                if *state != ClaimState::Unclaimed {
                    continue;
                }
                let Some((token, side, ordinal, range)) = self.trivia_identity_for_index(index)
                else {
                    return Err(ConservationError::UnownedTrivia { trivia: index });
                };
                return Err(ConservationError::MissingTrivia {
                    token,
                    side,
                    ordinal,
                    kind: self.tree.trivia_at(index).kind(),
                    range,
                });
            }
        }
        Ok(())
    }

    #[cfg(debug_assertions)]
    fn trivia_index(&self, identity: SourceTriviaId<'tree>) -> Option<usize> {
        if !identity.token.belongs_to(self.tree) {
            return None;
        }
        let token = self.tree.token(identity.token.id);
        let range = match identity.side {
            SourceTriviaSide::Leading => token.leading(),
            SourceTriviaSide::Trailing => token.trailing(),
        };
        (identity.ordinal < range.len()).then(|| range.start + identity.ordinal)
    }

    #[cfg(debug_assertions)]
    fn trivia_identity_for_index(
        &self,
        index: usize,
    ) -> Option<(usize, SourceTriviaSide, usize, TextRange)> {
        for (token_index, token) in self.tree.token_data().iter().enumerate() {
            let token_id = SourceTokenId {
                tree: self.tree,
                id: TokenId::new(token_index),
            };
            for piece in source_trivia_pieces(token_id, SourceTriviaSide::Leading, token.leading())
            {
                if self.trivia_index(piece.id()) == Some(index) {
                    return Some((
                        token_index,
                        SourceTriviaSide::Leading,
                        piece.id.ordinal,
                        piece.text_range,
                    ));
                }
            }
            for piece in
                source_trivia_pieces(token_id, SourceTriviaSide::Trailing, token.trailing())
            {
                if self.trivia_index(piece.id()) == Some(index) {
                    return Some((
                        token_index,
                        SourceTriviaSide::Trailing,
                        piece.id.ordinal,
                        piece.text_range,
                    ));
                }
            }
        }
        None
    }
}

#[cfg(debug_assertions)]
fn mark_conserved_trivia(tree: &SyntaxTree, states: &mut [ClaimState]) {
    let mut line_comment_needs_terminator = false;
    for (index, state) in states.iter_mut().enumerate() {
        let kind = tree.trivia_at(index).kind();
        let conserved = matches!(
            kind,
            TriviaKind::LineComment
                | TriviaKind::ShebangComment
                | TriviaKind::BlockComment
                | TriviaKind::DocComment
                | TriviaKind::Ignored
        ) || (kind == TriviaKind::Newline && line_comment_needs_terminator);
        if conserved {
            *state = ClaimState::Unclaimed;
        }
        line_comment_needs_terminator = match kind {
            TriviaKind::LineComment | TriviaKind::ShebangComment => true,
            TriviaKind::Whitespace
            | TriviaKind::Newline
            | TriviaKind::BlockComment
            | TriviaKind::DocComment
            | TriviaKind::Ignored => false,
        };
    }
}

/// The exact syntax-owned source core eligible for malformed verbatim output.
#[derive(Clone, Copy)]
pub struct SyntaxVerbatimCore<'tree, L: Language> {
    node: SyntaxNode<'tree, L>,
    range: TextRange,
    previous: Option<TokenId>,
    next: Option<TokenId>,
}

impl<'tree, L: Language> SyntaxVerbatimCore<'tree, L> {
    pub(crate) fn new(node: SyntaxNode<'tree, L>) -> Self {
        let range = if node.parent().is_none() {
            node.text_range()
        } else if let (Some(first), Some(last)) = (node.first_token(), node.last_token()) {
            let mut start = first.token_text_range().start();
            let mut ignored_run_start = None;
            for piece in first.leading_trivia_with_ids() {
                if piece.trivia().kind() == TriviaKind::Ignored {
                    ignored_run_start.get_or_insert(piece.text_range().start());
                } else {
                    ignored_run_start = None;
                }
            }
            if let Some(ignored_start) = ignored_run_start {
                start = ignored_start;
            }

            let mut end = last.token_text_range().end();
            for piece in last.trailing_trivia_with_ids() {
                if piece.trivia().kind() != TriviaKind::Ignored {
                    break;
                }
                end = piece.text_range().end();
            }
            TextRange::new(start, end)
        } else {
            TextRange::empty(node.text_range().start())
        };
        let previous = node
            .first_token()
            .and_then(|token| token.source_id().id.index().checked_sub(1))
            .map(TokenId::new);
        let next = node.last_token().and_then(|token| {
            let index = token.source_id().id.index() + 1;
            (index < node.tree().token_count()).then(|| TokenId::new(index))
        });
        Self {
            node,
            range,
            previous,
            next,
        }
    }

    pub(crate) const fn empty(
        node: SyntaxNode<'tree, L>,
        range: TextRange,
        previous: Option<TokenId>,
        next: Option<TokenId>,
    ) -> Self {
        Self {
            node,
            range,
            previous,
            next,
        }
    }

    #[must_use]
    pub const fn text_range(&self) -> TextRange {
        self.range
    }

    #[must_use]
    pub fn contains(&self, range: TextRange) -> bool {
        range_contains(self.range, range)
    }

    #[must_use]
    pub fn first_token(&self) -> Option<SyntaxToken<'tree, L>> {
        self.node.first_token()
    }

    #[must_use]
    pub fn last_token(&self) -> Option<SyntaxToken<'tree, L>> {
        self.node.last_token()
    }

    #[must_use]
    pub fn raw_kind(&self) -> crate::RawSyntaxKind {
        self.node.raw_kind()
    }

    /// Returns the nearest represented source token before this core.
    #[must_use]
    pub fn previous_token(&self) -> Option<SyntaxToken<'tree, L>> {
        self.previous
            .map(|id| SyntaxToken::new(self.node.source(), self.node.tree(), id))
    }

    /// Returns the nearest represented non-EOF source token after this core.
    #[must_use]
    pub fn next_token(&self) -> Option<SyntaxToken<'tree, L>> {
        let id = self.next?;
        let token = self.node.tree().token(id);
        (token.raw_kind() != L::kind_to_raw(L::eof_kind()))
            .then(|| SyntaxToken::new(self.node.source(), self.node.tree(), id))
    }

    #[must_use]
    pub fn ends_with_line_comment(&self) -> bool {
        let mut ends_with_line_comment = false;
        for token in self.node.tokens() {
            for piece in token.leading_trivia_with_ids() {
                if !range_contains(self.range, piece.text_range()) {
                    continue;
                }
                ends_with_line_comment = match piece.trivia().kind() {
                    TriviaKind::LineComment | TriviaKind::ShebangComment => true,
                    TriviaKind::Whitespace => ends_with_line_comment,
                    TriviaKind::Newline
                    | TriviaKind::BlockComment
                    | TriviaKind::DocComment
                    | TriviaKind::Ignored => false,
                };
            }
            if range_contains(self.range, token.token_text_range()) && !token.text().is_empty() {
                ends_with_line_comment = false;
            }
            for piece in token.trailing_trivia_with_ids() {
                if !range_contains(self.range, piece.text_range()) {
                    continue;
                }
                ends_with_line_comment = match piece.trivia().kind() {
                    TriviaKind::LineComment | TriviaKind::ShebangComment => true,
                    TriviaKind::Whitespace => ends_with_line_comment,
                    TriviaKind::Newline
                    | TriviaKind::BlockComment
                    | TriviaKind::DocComment
                    | TriviaKind::Ignored => false,
                };
            }
        }
        ends_with_line_comment
    }

    #[must_use]
    pub fn text(&self) -> &'tree str {
        &self.node.source()[self.range.start().get()..self.range.end().get()]
    }

    pub fn tokens(&self) -> impl Iterator<Item = SyntaxToken<'tree, L>> + use<'tree, L> {
        let range = self.range;
        self.node.tokens().filter(move |token| {
            token.token_text_range().start() != token.token_text_range().end()
                && range_contains(range, token.token_text_range())
        })
    }

    /// Returns the compact syntax-owned claim for every conserved identity in
    /// this malformed core. The formatter cannot enumerate or widen it.
    #[must_use]
    pub fn source_claim(&self) -> SourceRangeClaim<'tree> {
        SourceRangeClaim::from_tree(self.node.tree(), self.range, false)
    }
}

#[cfg(debug_assertions)]
fn is_conserved_trivia(identity: SourceTriviaId<'_>) -> bool {
    let token = identity.token.tree.token(identity.token.id);
    let range = match identity.side {
        SourceTriviaSide::Leading => token.leading(),
        SourceTriviaSide::Trailing => token.trailing(),
    };
    let index = range.start + identity.ordinal;
    let kind = identity.token.tree.trivia_at(index).kind();
    matches!(
        kind,
        TriviaKind::LineComment
            | TriviaKind::ShebangComment
            | TriviaKind::BlockComment
            | TriviaKind::DocComment
            | TriviaKind::Ignored
    ) || (kind == TriviaKind::Newline
        && index.checked_sub(1).is_some_and(|previous| {
            matches!(
                identity.token.tree.trivia_at(previous).kind(),
                TriviaKind::LineComment | TriviaKind::ShebangComment
            )
        }))
}

fn range_contains(outer: TextRange, inner: TextRange) -> bool {
    outer.start() <= inner.start() && inner.end() <= outer.end()
}

pub(crate) fn source_trivia_pieces(
    token: SourceTokenId<'_>,
    side: SourceTriviaSide,
    range: std::ops::Range<usize>,
) -> SourceTriviaPieces<'_> {
    let data = token.tree.token(token.id);
    let offset = match side {
        SourceTriviaSide::Leading => data.full_text_range().start(),
        SourceTriviaSide::Trailing => data.token_text_range().end(),
    };
    SourceTriviaPieces::new(token, side, token.tree.trivia(&range), offset)
}

#[cfg(test)]
mod tests {
    use jolt_diagnostics::{Diagnostic, DiagnosticCodeId};
    use jolt_text::{TextRange, TextSize};

    use crate::{
        BuildSyntaxTreeError, Event, FactoryNode, Language, LanguageLexer, LexedToken,
        ParsedChildren, RawSyntaxKind, SyntaxFactory, SyntaxNode, SyntaxTokenData, SyntaxTreeSink,
        SyntaxTrivia, TriviaKind, build_syntax_tree_with_factory,
    };

    use super::{ConservationError, SourceIdentity, SourceTriviaSide, SyntaxVerbatimCore};

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum TestKind {
        Root,
        Token,
        Eof,
        Error,
    }

    struct TestLanguage;
    struct UnusedLexer;
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

    impl Language for TestLanguage {
        type Kind = TestKind;
        type Lexer<'source> = UnusedLexer;
        type NormalizationAuthority = ();

        fn kind_from_raw(raw: RawSyntaxKind) -> Self::Kind {
            match raw.get() {
                0 => TestKind::Root,
                1 => TestKind::Token,
                2 => TestKind::Eof,
                _ => TestKind::Error,
            }
        }

        fn kind_to_raw(kind: Self::Kind) -> RawSyntaxKind {
            RawSyntaxKind::new(match kind {
                TestKind::Root => 0,
                TestKind::Token => 1,
                TestKind::Eof => 2,
                TestKind::Error => 3,
            })
        }

        fn eof_kind() -> Self::Kind {
            TestKind::Eof
        }

        fn expected_diagnostic_code() -> DiagnosticCodeId {
            DiagnosticCodeId::new("test.expected")
        }

        fn unexpected_diagnostic_code() -> DiagnosticCodeId {
            DiagnosticCodeId::new("test.unexpected")
        }

        fn split_token(_token: &LexedToken<Self>) -> Option<&'static [Self::Kind]> {
            None
        }
    }

    impl<'source> LanguageLexer<'source> for UnusedLexer {
        type Language = TestLanguage;

        fn new(_source: &'source str) -> Self {
            Self
        }

        fn next_token_into(
            &mut self,
            _trivia: &mut Vec<SyntaxTrivia>,
        ) -> LexedToken<Self::Language> {
            panic!("test constructs tokens directly")
        }

        fn finish(self) -> Vec<Diagnostic> {
            Vec::new()
        }
    }

    fn test_tree() -> (&'static str, crate::SyntaxTree) {
        let source = "x// c\n !\u{1a}";
        let trivia = vec![
            SyntaxTrivia::new(TriviaKind::LineComment, TextSize::new(4)),
            SyntaxTrivia::new(TriviaKind::Newline, TextSize::new(1)),
            SyntaxTrivia::new(TriviaKind::Whitespace, TextSize::new(1)),
            SyntaxTrivia::new(TriviaKind::Ignored, TextSize::new(1)),
        ];
        let tokens = vec![
            SyntaxTokenData::new(
                RawSyntaxKind::new(1),
                TextRange::new(TextSize::new(0), TextSize::new(5)),
                TextRange::new(TextSize::new(0), TextSize::new(1)),
                0..0,
                0..1,
            ),
            SyntaxTokenData::new(
                RawSyntaxKind::new(1),
                TextRange::new(TextSize::new(5), TextSize::new(9)),
                TextRange::new(TextSize::new(7), TextSize::new(8)),
                1..3,
                3..4,
            ),
            SyntaxTokenData::new(
                RawSyntaxKind::new(2),
                TextRange::empty(TextSize::new(9)),
                TextRange::empty(TextSize::new(9)),
                4..4,
                4..4,
            ),
        ];
        let events = vec![
            Event::Start {
                kind: RawSyntaxKind::new(0),
                forward_parent: 0,
            },
            Event::Token,
            Event::Start {
                kind: RawSyntaxKind::new(0),
                forward_parent: 0,
            },
            Event::Token,
            Event::Finish,
            Event::Token,
            Event::Finish,
        ];
        let tree = build_syntax_tree_with_factory("", events, tokens, trivia, &TestFactory)
            .expect("valid tree");
        (source, tree)
    }

    fn claim_all_individually(root: SyntaxNode<'_, TestLanguage>) {
        let mut tracker = root.conservation_tracker();
        for token in root.tokens() {
            if token.token_text_range().start() != token.token_text_range().end() {
                tracker
                    .claim_token(token.source_id())
                    .expect("unique source token");
            }
            for piece in token
                .leading_trivia_with_ids()
                .chain(token.trailing_trivia_with_ids())
            {
                if piece.trivia().kind() != TriviaKind::Whitespace {
                    tracker
                        .claim(SourceIdentity::Trivia(piece.id()))
                        .expect("unique conserved source trivia");
                }
            }
        }
        tracker.finish().expect("all conserved identities claimed");
    }

    #[test]
    fn dense_accounting_conserves_comments_ignored_and_line_terminators() {
        let (source, tree) = test_tree();
        let root = SyntaxNode::<TestLanguage>::new_root(source, &tree);
        claim_all_individually(root);

        let mut tracker = root.conservation_tracker();
        for token in root.tokens() {
            if token.token_text_range().start() != token.token_text_range().end() {
                tracker
                    .claim_token(token.source_id())
                    .expect("source token");
            }
            for piece in token
                .leading_trivia_with_ids()
                .chain(token.trailing_trivia_with_ids())
            {
                if !matches!(
                    piece.trivia().kind(),
                    TriviaKind::Newline | TriviaKind::Whitespace
                ) {
                    tracker.claim_trivia(piece.id()).expect("source trivia");
                }
            }
        }
        assert_eq!(
            tracker.finish(),
            Err(ConservationError::MissingTrivia {
                token: 1,
                side: SourceTriviaSide::Leading,
                ordinal: 0,
                kind: TriviaKind::Newline,
                range: TextRange::new(TextSize::new(5), TextSize::new(6)),
            })
        );
    }

    #[test]
    fn duplicate_missing_and_foreign_fail_deterministically() {
        let (source, tree) = test_tree();
        let root = SyntaxNode::<TestLanguage>::new_root(source, &tree);
        let first = root.first_token().expect("first token");
        let mut tracker = root.conservation_tracker();
        tracker.claim_token(first.source_id()).expect("first claim");
        assert_eq!(
            tracker.claim_token(first.source_id()),
            Err(ConservationError::DuplicateToken { token: 0 })
        );
        let comment = first
            .trailing_trivia_with_ids()
            .next()
            .expect("trailing comment");
        tracker
            .claim_trivia(comment.id())
            .expect("first trivia claim");
        assert_eq!(
            tracker.claim_trivia(comment.id()),
            Err(ConservationError::DuplicateTrivia {
                token: 0,
                side: SourceTriviaSide::Trailing,
                ordinal: 0,
            })
        );

        let tracker = root.conservation_tracker();
        assert_eq!(
            tracker.finish(),
            Err(ConservationError::MissingToken {
                token: 0,
                range: TextRange::new(TextSize::new(0), TextSize::new(1)),
            })
        );

        let (other_source, other_tree) = test_tree();
        let other_root = SyntaxNode::<TestLanguage>::new_root(other_source, &other_tree);
        let mut tracker = root.conservation_tracker();
        assert_eq!(
            tracker.claim_token(other_root.first_token().expect("foreign token").source_id()),
            Err(ConservationError::ForeignToken)
        );
        let foreign_comment = other_root
            .first_token()
            .expect("foreign token")
            .trailing_trivia_with_ids()
            .next()
            .expect("foreign comment");
        assert_eq!(
            tracker.claim_trivia(foreign_comment.id()),
            Err(ConservationError::ForeignTrivia)
        );

        let whitespace = root
            .tokens()
            .nth(1)
            .expect("second token")
            .leading_trivia_with_ids()
            .nth(1)
            .expect("leading whitespace");
        let mut tracker = root.conservation_tracker();
        assert_eq!(
            tracker.claim_trivia(whitespace.id()),
            Err(ConservationError::UnauthorizedTrivia {
                token: 1,
                side: SourceTriviaSide::Leading,
                ordinal: 1,
                kind: TriviaKind::Whitespace,
                range: TextRange::new(TextSize::new(6), TextSize::new(7)),
            })
        );

        let eof = root.last_token().expect("EOF token");
        let mut tracker = root.conservation_tracker();
        assert_eq!(
            tracker.claim_token(eof.source_id()),
            Err(ConservationError::UnauthorizedToken {
                token: 2,
                range: TextRange::empty(TextSize::new(9)),
            })
        );
        assert_eq!(
            tracker.validate_token(eof.source_id()),
            Err(ConservationError::UnauthorizedToken {
                token: 2,
                range: TextRange::empty(TextSize::new(9)),
            })
        );
    }

    #[test]
    fn verbatim_range_claims_exact_contained_tokens_and_trivia() {
        let (source, tree) = test_tree();
        let root = SyntaxNode::<TestLanguage>::new_root(source, &tree);
        let mut tracker = root.conservation_tracker();
        let core = SyntaxVerbatimCore::new(root);
        tracker
            .claim_source_range(core.source_claim())
            .expect("whole source is an exact verbatim range");
        tracker.finish().expect("verbatim claims all identities");
    }

    #[test]
    fn non_root_verbatim_core_excludes_boundary_layout_but_keeps_ignored_suffix() {
        let (source, tree) = test_tree();
        let root = SyntaxNode::<TestLanguage>::new_root(source, &tree);
        let child = root.children().next().expect("nested child");
        let core = SyntaxVerbatimCore::new(child);
        assert_eq!(
            core.text_range(),
            TextRange::new(TextSize::new(7), TextSize::new(9))
        );
        assert_eq!(core.text(), "!\u{1a}");
        assert_eq!(core.tokens().count(), 1);
        assert_eq!(core.previous_token().expect("left boundary").text(), "x");
        assert!(core.next_token().is_none(), "EOF is not a lexical boundary");
    }
}
