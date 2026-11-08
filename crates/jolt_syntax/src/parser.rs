//! Parser infrastructure: tokens, events, markers, and loop and depth guards.

use std::num::NonZeroU32;
use std::ops::Range;

use crate::{Diagnostic, Language, RawSyntaxKind, TextRange, TextSize, TokenData, Trivia};

/// Active `nested` calls allowed at once. Calibrated for the 1 MiB WASM stack.
pub const MAX_DEPTH: u32 = 128;

/// One step of a parse, in source order. [`build_tree`](crate::build_tree)
/// turns the events into a tree.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    /// Starts a node. `forward_parent` is the offset to a later start event
    /// whose node wraps this one ([`Parser::precede`]).
    Start {
        kind: RawSyntaxKind,
        forward_parent: Option<NonZeroU32>,
    },
    /// Consumes the next token into the open node.
    Token,
    /// Finishes the innermost open node.
    Finish,
    /// A reserved, abandoned, or already consumed start.
    Tombstone,
}

/// A node start whose kind is not decided yet. Must be completed or
/// abandoned (checked in debug builds).
#[must_use = "a marker must be completed or abandoned"]
pub struct Marker {
    position: u32,
}

impl Drop for Marker {
    fn drop(&mut self) {
        assert!(
            !cfg!(debug_assertions) || std::thread::panicking(),
            "marker dropped without being completed or abandoned"
        );
    }
}

/// A completed node, which [`Parser::precede`] can wrap.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompletedMarker {
    position: u32,
    kind: RawSyntaxKind,
}

impl CompletedMarker {
    #[must_use]
    pub const fn kind(self) -> RawSyntaxKind {
        self.kind
    }
}

/// A set of token kinds. Token kinds must fit in `0..128`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TokenSet(u128);

impl TokenSet {
    pub const EMPTY: Self = Self(0);

    #[must_use]
    pub const fn new(kinds: &[RawSyntaxKind]) -> Self {
        let mut set = Self::EMPTY;
        let mut index = 0;
        while index < kinds.len() {
            set = set.with(kinds[index]);
            index += 1;
        }
        set
    }

    #[must_use]
    pub const fn with(self, kind: RawSyntaxKind) -> Self {
        Self(self.0 | 1 << kind.0)
    }

    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    #[must_use]
    pub const fn contains(self, kind: RawSyntaxKind) -> bool {
        kind.0 < 128 && self.0 & (1 << kind.0) != 0
    }
}

/// One lexed token; its trivia was appended to the trivia vector.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LexedToken<K> {
    pub kind: K,
    pub range: TextRange,
    pub leading: Range<usize>,
    pub trailing: Range<usize>,
}

/// A language's lexer. [`Parser::new`] calls `next_token` until it returns
/// `L::EOF`, then `finish`.
pub trait Lexer<'s, L: Language> {
    /// Lexes the next token, appending its leading and then its trailing
    /// trivia to `trivia`. At the end of input, returns `L::EOF` with the
    /// remaining trivia as its leading trivia.
    fn next_token(&mut self, trivia: &mut Vec<Trivia>) -> LexedToken<L::Kind>;
    /// The diagnostics of the lexed source.
    fn finish(self) -> Vec<Diagnostic>;
}

/// What a parse produced, for [`build_tree`](crate::build_tree).
pub struct ParseOutput {
    pub events: Vec<Event>,
    pub tokens: Vec<TokenData>,
    pub trivia: Vec<Trivia>,
    pub diagnostics: Vec<Diagnostic>,
}

/// The language-agnostic parser: the lexed tokens, a cursor over them, and
/// the events and diagnostics recorded so far. A language's grammar drives
/// it through markers ([`Parser::start`]) and token consumption.
pub struct Parser<'s, L: Language> {
    source: &'s str,
    /// Every lexed token, ending with `EOF`. Tokens before `pos` are dead.
    input: Vec<TokenData>,
    kinds: Vec<L::Kind>,
    pos: usize,
    /// Consumed tokens, in event order.
    tokens: Vec<TokenData>,
    trivia: Vec<Trivia>,
    events: Vec<Event>,
    diagnostics: Vec<Diagnostic>,
    depth: u32,
    consumed: u32,
}

impl<'s, L: Language> Parser<'s, L> {
    /// Lexes all of `source`.
    pub fn new(source: &'s str, mut lexer: impl Lexer<'s, L>) -> Self {
        let mut input = Vec::with_capacity(source.len() / 6 + 1);
        let mut kinds = Vec::with_capacity(source.len() / 6 + 1);
        let mut trivia = Vec::with_capacity(source.len() / 10 + 1);
        loop {
            let token = lexer.next_token(&mut trivia);
            let leading_len: u32 = trivia[token.leading.clone()]
                .iter()
                .map(|piece| piece.len)
                .sum();
            let full_start = token.range.start() - TextSize::from(leading_len);
            input.push(TokenData::new(
                L::to_raw(token.kind),
                full_start,
                token.range,
                token.leading,
                token.trailing,
            ));
            kinds.push(token.kind);
            if token.kind == L::EOF {
                break;
            }
        }
        let tokens = Vec::with_capacity(input.len());
        let events = Vec::with_capacity(input.len() * 3);
        Self {
            source,
            input,
            kinds,
            pos: 0,
            tokens,
            trivia,
            events,
            diagnostics: lexer.finish(),
            depth: 0,
            consumed: 0,
        }
    }

    #[must_use]
    pub const fn source(&self) -> &'s str {
        self.source
    }

    /// The index of the current token.
    #[must_use]
    pub const fn position(&self) -> usize {
        self.pos
    }

    #[must_use]
    pub fn current(&self) -> L::Kind {
        self.kind_at(self.pos)
    }

    #[must_use]
    pub fn nth(&self, n: usize) -> L::Kind {
        self.kind_at(self.pos + n)
    }

    /// The kind of the token at `index`; `EOF` past the end.
    #[must_use]
    pub fn kind_at(&self, index: usize) -> L::Kind {
        self.kinds.get(index).copied().unwrap_or(L::EOF)
    }

    #[must_use]
    pub fn nth_text(&self, n: usize) -> &'s str {
        self.text_at(self.pos + n)
    }

    /// The text of the token at `index`; empty past the end.
    #[must_use]
    pub fn text_at(&self, index: usize) -> &'s str {
        self.input.get(index).map_or("", |token| {
            let range = token.range();
            &self.source[range.start().get()..range.end().get()]
        })
    }

    /// The text range of the token at `index`; the end of input past the end.
    #[must_use]
    pub fn range_at(&self, index: usize) -> TextRange {
        self.input.get(index).map_or_else(
            || TextRange::empty(TextSize::new(self.source.len())),
            TokenData::range,
        )
    }

    #[must_use]
    pub fn at(&self, kind: L::Kind) -> bool {
        self.current() == kind
    }

    /// Whether the current token is in `set`.
    #[must_use]
    pub fn at_any(&self, set: TokenSet) -> bool {
        set.contains(L::to_raw(self.current()))
    }

    /// Consumes the current token. A no-op at the end of input.
    pub fn bump(&mut self) {
        let Some(token) = self.input.get(self.pos) else {
            debug_assert!(false, "bump past the end of input");
            return;
        };
        self.tokens.push(*token);
        self.events.push(Event::Token);
        self.pos += 1;
        self.consumed += 1;
    }

    pub fn eat(&mut self, kind: L::Kind) -> bool {
        let at = self.at(kind);
        if at {
            self.bump();
        }
        at
    }

    /// Eats `kind` or records `message` as an `expected` diagnostic at the
    /// current token. The factory turns the absent token into a missing slot.
    pub fn expect(&mut self, kind: L::Kind, message: &str) -> bool {
        let eaten = self.eat(kind);
        if !eaten {
            self.error(L::EXPECTED_CODE, message);
        }
        eaten
    }

    /// Records a diagnostic at the current token.
    pub fn error(&mut self, code: &'static str, message: impl Into<String>) {
        self.error_at(0, code, message);
    }

    /// Records a diagnostic at the `n`th token from the current one.
    pub fn error_at(&mut self, n: usize, code: &'static str, message: impl Into<String>) {
        let range = self.range_at(self.pos + n);
        self.diagnostics.push(Diagnostic {
            code,
            message: message.into(),
            range,
        });
    }

    /// # Panics
    ///
    /// Panics when the event count exceeds `u32`.
    pub fn start(&mut self) -> Marker {
        let position = u32::try_from(self.events.len()).expect("event count fits in u32");
        self.events.push(Event::Tombstone);
        Marker { position }
    }

    pub fn complete(&mut self, marker: Marker, kind: L::Kind) -> CompletedMarker {
        let position = marker.position;
        std::mem::forget(marker);
        let kind = L::to_raw(kind);
        self.events[position as usize] = Event::Start {
            kind,
            forward_parent: None,
        };
        self.events.push(Event::Finish);
        CompletedMarker { position, kind }
    }

    pub fn abandon(&mut self, marker: Marker) {
        let position = marker.position;
        std::mem::forget(marker);
        if position as usize + 1 == self.events.len() {
            self.events.pop();
        }
    }

    /// Starts a node that will wrap the completed one.
    pub fn precede(&mut self, completed: CompletedMarker) -> Marker {
        let marker = self.start();
        let Event::Start { forward_parent, .. } = &mut self.events[completed.position as usize]
        else {
            unreachable!("a completed marker points at its start event");
        };
        debug_assert!(
            forward_parent.is_none(),
            "a node can have one forward parent"
        );
        *forward_parent = NonZeroU32::new(marker.position - completed.position);
        marker
    }

    /// Replaces the current token by a one-byte `first` token followed by
    /// `rest` covering the remaining bytes. Leading trivia stays on `first`,
    /// trailing trivia moves to `rest`. Used only by consuming grammar (never
    /// by lookahead) for Java's `>` family, which always follows a consumed
    /// token. The head takes that consumed token's slot, so token indices are
    /// unchanged.
    pub fn split_current(&mut self, first: L::Kind, rest: L::Kind) {
        debug_assert!(self.pos > 0, "a split token follows a consumed token");
        let mut remainder = self.input[self.pos];
        let head = remainder.split_off_first(1, L::to_raw(first), L::to_raw(rest));
        self.pos -= 1;
        self.input[self.pos] = head;
        self.kinds[self.pos] = first;
        self.input[self.pos + 1] = remainder;
        self.kinds[self.pos + 1] = rest;
    }

    #[must_use]
    pub const fn loop_guard(&self) -> LoopGuard {
        LoopGuard {
            last: self.consumed.wrapping_sub(1),
        }
    }

    /// Runs `parse` one recursion level deeper, or returns `None` without
    /// running it when `MAX_DEPTH` levels are active. `parser` is this parser
    /// or a language's wrapper around it.
    pub fn nested<P: AsMut<Self>, T>(parser: &mut P, parse: impl FnOnce(&mut P) -> T) -> Option<T> {
        let this = parser.as_mut();
        if this.depth >= MAX_DEPTH {
            return None;
        }
        this.depth += 1;
        let result = parse(parser);
        parser.as_mut().depth -= 1;
        Some(result)
    }

    /// The one deep-nesting recovery. Records a "too deeply nested"
    /// diagnostic and wraps tokens into a single `BOGUS` node until, at
    /// bracket depth zero, a token in `stops` or an unmatched closing
    /// bracket, or EOF. The brackets are [`Language::BRACKETS`]; it never
    /// splits a token.
    pub fn recover_too_deep(&mut self, stops: TokenSet) -> CompletedMarker {
        let marker = self.start();
        self.error(
            L::TOO_DEEP_CODE,
            "syntax is too deeply nested to parse safely",
        );
        self.skip_balanced(|kind| {
            stops.contains(L::to_raw(kind)) || L::BRACKETS.iter().any(|&(_, close)| close == kind)
        });
        self.complete(marker, L::BOGUS)
    }

    /// Bumps tokens up to EOF or, outside the brackets the skipped tokens
    /// open, a token `stop` accepts.
    pub fn skip_balanced(&mut self, stop: impl Fn(L::Kind) -> bool) {
        let mut depth = 0usize;
        loop {
            let kind = self.current();
            if kind == L::EOF || (depth == 0 && stop(kind)) {
                break;
            }
            if L::BRACKETS.iter().any(|&(open, _)| open == kind) {
                depth += 1;
            } else if L::BRACKETS.iter().any(|&(_, close)| close == kind) {
                depth = depth.saturating_sub(1);
            }
            self.bump();
        }
    }

    /// A read-only cursor for speculative lookahead.
    #[must_use]
    pub const fn lookahead(&self) -> Lookahead<'_, 's, L> {
        Lookahead {
            parser: self,
            pos: self.pos,
        }
    }

    /// # Panics
    ///
    /// Panics when the grammar left tokens unconsumed.
    #[must_use]
    pub fn finish(self) -> ParseOutput {
        assert_eq!(
            self.pos,
            self.input.len(),
            "the parser must consume every token"
        );
        ParseOutput {
            events: self.events,
            tokens: self.tokens,
            trivia: self.trivia,
            diagnostics: self.diagnostics,
        }
    }
}

impl<L: Language> AsMut<Self> for Parser<'_, L> {
    fn as_mut(&mut self) -> &mut Self {
        self
    }
}

/// Release-active progress guard. Every grammar loop that calls back into
/// grammar functions holds one and calls `check` at the top of each
/// iteration.
#[derive(Clone, Copy, Debug)]
pub struct LoopGuard {
    last: u32,
}

impl LoopGuard {
    /// Returns `false` at EOF (the loop must exit). If no token was consumed
    /// since the previous `check`, wraps the current token in a `BOGUS` node
    /// with an "unexpected token" diagnostic so the loop always advances.
    pub fn check<L: Language>(&mut self, p: &mut Parser<'_, L>) -> bool {
        if p.at(L::EOF) {
            return false;
        }
        if p.consumed == self.last {
            let marker = p.start();
            p.error(L::UNEXPECTED_CODE, "unexpected token");
            p.bump();
            p.complete(marker, L::BOGUS);
            if p.at(L::EOF) {
                return false;
            }
        }
        self.last = p.consumed;
        true
    }
}

/// A read-only cursor over the parser's tokens.
#[derive(Clone, Copy)]
pub struct Lookahead<'p, 's, L: Language> {
    parser: &'p Parser<'s, L>,
    pos: usize,
}

impl<'s, L: Language> Lookahead<'_, 's, L> {
    #[must_use]
    pub const fn position(&self) -> usize {
        self.pos
    }

    /// Moves to an absolute token index.
    pub const fn set_position(&mut self, pos: usize) {
        self.pos = pos;
    }

    #[must_use]
    pub fn current(&self) -> L::Kind {
        self.parser.kind_at(self.pos)
    }

    #[must_use]
    pub fn nth(&self, n: usize) -> L::Kind {
        self.parser.kind_at(self.pos + n)
    }

    #[must_use]
    pub fn nth_text(&self, n: usize) -> &'s str {
        self.parser.text_at(self.pos + n)
    }

    #[must_use]
    pub fn at(&self, kind: L::Kind) -> bool {
        self.current() == kind
    }

    /// Moves to the next token; stays on `EOF`.
    pub fn bump(&mut self) {
        if self.pos + 1 < self.parser.input.len() {
            self.pos += 1;
        }
    }

    pub fn eat(&mut self, kind: L::Kind) -> bool {
        let at = self.at(kind);
        if at {
            self.bump();
        }
        at
    }
}
