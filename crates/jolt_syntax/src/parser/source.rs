use std::ops::Range;

use jolt_diagnostics::{Diagnostic, DiagnosticCodeId, DiagnosticStage, Severity};
use jolt_text::{TextRange, TextSize};

use crate::{
    CompletedMarker, Event, Language, LanguageLexer, LexedToken, Marker, SyntaxTokenData,
    SyntaxTrivia,
};

pub struct ParseEvents {
    pub events: Vec<Event>,
    pub tokens: Vec<SyntaxTokenData>,
    pub trivia: Vec<SyntaxTrivia>,
    pub diagnostics: Vec<Diagnostic>,
}

pub struct Parser<'source, L: Language> {
    pub source: &'source str,
    pub buffer: TokenBuffer<'source, L>,
    cursor: TokenCursor,
    events: Vec<Event>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CursorCheckpoint {
    pos: usize,
}

#[derive(Clone, Copy)]
pub struct TokenCursor {
    pos: usize,
}

impl<'source, L: Language> Parser<'source, L> {
    #[must_use]
    pub fn new(source: &'source str) -> Self {
        Self {
            source,
            buffer: TokenBuffer::new(source),
            cursor: TokenCursor::new(),
            events: Vec::new(),
        }
    }

    pub fn finish(self) -> ParseEvents {
        let events = self.events;
        let committed_len = self.cursor.position();
        let (tokens, trivia, diagnostics) = self.buffer.finish(committed_len);
        ParseEvents {
            events,
            tokens,
            trivia,
            diagnostics,
        }
    }

    pub const fn position(&self) -> usize {
        self.cursor.position()
    }

    pub fn expect(&mut self, kind: L::Kind, message: &str) {
        if !self.eat(kind) {
            self.expected_here(message);
        }
    }

    pub fn eat(&mut self, kind: L::Kind) -> bool {
        if self.at(kind) {
            self.bump();
            true
        } else {
            false
        }
    }

    pub fn at(&mut self, kind: L::Kind) -> bool {
        self.current_kind() == kind
    }

    pub fn at_eof(&mut self) -> bool {
        self.current_kind() == L::eof_kind()
    }

    pub fn current_kind(&mut self) -> L::Kind {
        self.cursor.kind(&mut self.buffer)
    }

    pub fn nth_kind(&mut self, n: usize) -> L::Kind {
        self.cursor.nth_kind(&mut self.buffer, n)
    }

    pub fn kind_at(&mut self, index: usize) -> L::Kind {
        self.buffer.kind_at(index)
    }

    pub fn current_text(&mut self) -> Option<&'source str> {
        self.cursor.text(self.source, &mut self.buffer)
    }

    pub fn text_at(&mut self, index: usize) -> Option<&'source str> {
        self.buffer.text_at(self.source, index)
    }

    pub fn tokens_are_adjacent(&mut self, index: usize, count: usize) -> bool {
        self.buffer.tokens_are_adjacent(index, count)
    }

    pub fn bump(&mut self) {
        self.cursor.bump(&mut self.buffer);
        self.events.push(Event::Token);
    }

    pub fn fork_cursor(&self) -> TokenCursor {
        self.cursor.fork()
    }

    pub fn expected_here(&mut self, message: &str) {
        self.error_here(L::expected_diagnostic_code(), message);
    }

    pub fn unexpected_here(&mut self, message: &str) {
        self.error_here(L::unexpected_diagnostic_code(), message);
    }

    /// Adds a parser error at the current token, or at the last token if the cursor is past EOF.
    ///
    /// # Panics
    ///
    /// Panics if the parser token stream does not contain EOF.
    pub fn error_here(&mut self, code: DiagnosticCodeId, message: &str) {
        let range = self
            .cursor
            .range(&mut self.buffer)
            .or_else(|| self.buffer.last_token_range())
            .expect("parser token stream must include EOF");
        self.events.push(Event::Error(Diagnostic {
            code,
            severity: Severity::Error,
            stage: DiagnosticStage::Parser,
            message: message.to_owned(),
            range: Some(range),
        }));
    }

    pub fn start(&mut self) -> Marker {
        Marker::new(&mut self.events)
    }

    pub fn complete(&mut self, marker: Marker, kind: L::Kind) -> CompletedMarker {
        marker.complete(&mut self.events, L::kind_to_raw(kind))
    }

    pub fn precede(&mut self, marker: CompletedMarker) -> Marker {
        marker.precede(&mut self.events)
    }

    #[must_use]
    pub fn completed_is_error_node(marker: &CompletedMarker) -> bool {
        marker.kind() == L::kind_to_raw(L::error_node_kind())
    }

    pub fn abandon(&mut self, marker: Marker) {
        marker.abandon(&mut self.events);
    }
}

impl TokenCursor {
    const fn new() -> Self {
        Self { pos: 0 }
    }

    #[must_use]
    pub const fn position(self) -> usize {
        self.pos
    }

    pub fn kind<L: Language>(self, buffer: &mut TokenBuffer<'_, L>) -> L::Kind {
        buffer.kind_at(self.pos)
    }

    pub fn nth_kind<L: Language>(self, buffer: &mut TokenBuffer<'_, L>, n: usize) -> L::Kind {
        buffer.kind_at(self.pos + n)
    }

    pub fn text<'source, L: Language>(
        self,
        source: &'source str,
        buffer: &mut TokenBuffer<'source, L>,
    ) -> Option<&'source str> {
        let range = self.range(buffer)?;
        Some(source_text(source, range))
    }

    pub fn range<L: Language>(self, buffer: &mut TokenBuffer<'_, L>) -> Option<TextRange> {
        buffer.range_at(self.pos)
    }

    pub fn bump<L: Language>(&mut self, buffer: &mut TokenBuffer<'_, L>) {
        buffer.ensure(self.pos);
        self.pos += 1;
    }

    #[must_use]
    pub const fn checkpoint(self) -> CursorCheckpoint {
        CursorCheckpoint { pos: self.pos }
    }

    pub fn rewind(&mut self, checkpoint: CursorCheckpoint) {
        self.pos = checkpoint.pos;
    }

    #[must_use]
    pub const fn fork(self) -> Self {
        self
    }
}

fn source_text(source: &str, range: TextRange) -> &str {
    let start = range.start().get();
    let end = range.end().get();
    &source[start..end]
}

pub struct TokenBuffer<'source, L: Language> {
    lexer: L::Lexer<'source>,
    tokens: Vec<SyntaxTokenData>,
    trivia: Vec<SyntaxTrivia>,
}

impl<'source, L: Language> TokenBuffer<'source, L> {
    fn new(source: &'source str) -> Self {
        Self {
            lexer: L::Lexer::new(source),
            tokens: Vec::new(),
            trivia: Vec::new(),
        }
    }

    fn kind_at(&mut self, index: usize) -> L::Kind {
        self.ensure(index);
        self.tokens
            .get(index)
            .map_or(L::eof_kind(), |token| L::kind_from_raw(token.raw_kind()))
    }

    fn range_at(&mut self, index: usize) -> Option<TextRange> {
        self.ensure(index);
        self.tokens
            .get(index)
            .map(SyntaxTokenData::token_text_range)
    }

    pub fn text_at(&mut self, source: &'source str, index: usize) -> Option<&'source str> {
        let range = self.range_at(index)?;
        Some(source_text(source, range))
    }

    fn last_token_range(&mut self) -> Option<TextRange> {
        self.ensure(0);
        self.tokens.last().map(SyntaxTokenData::token_text_range)
    }

    pub fn tokens_are_adjacent(&mut self, index: usize, count: usize) -> bool {
        if count <= 1 {
            return true;
        }

        self.ensure(index + count - 1);
        let Some(tokens) = self.tokens.get(index..index + count) else {
            return false;
        };

        tokens.windows(2).all(|window| {
            let [left, right] = window else {
                return true;
            };
            left.token_text_range().end() == right.token_text_range().start()
                && left.trailing().is_empty()
                && right.leading().is_empty()
        })
    }

    pub fn trivia_has_newline(&self, range: &Range<usize>) -> bool {
        self.trivia[range.start..range.end]
            .iter()
            .any(|trivia| trivia.kind() == crate::TriviaKind::Newline)
    }

    pub fn newline_before(&mut self, index: usize) -> bool {
        self.ensure(index);
        self.tokens
            .get(index)
            .is_some_and(|token| self.trivia_has_newline(token.leading()))
    }

    pub fn newline_between(&mut self, left: usize, right: usize) -> bool {
        self.ensure(right);
        let left_has_newline = self
            .tokens
            .get(left)
            .is_some_and(|token| self.trivia_has_newline(token.trailing()));
        let right_has_newline = self
            .tokens
            .get(right)
            .is_some_and(|token| self.trivia_has_newline(token.leading()));
        left_has_newline || right_has_newline
    }

    fn ensure(&mut self, index: usize) {
        while self.tokens.len() <= index
            && self
                .tokens
                .last()
                .is_none_or(|token| token.raw_kind() != L::kind_to_raw(L::eof_kind()))
        {
            let token = self.lexer.next_token_into(&mut self.trivia);
            self.push_token(token);
        }
    }

    fn push_token(&mut self, token: LexedToken<L>) {
        if let Some(kinds) = L::split_token(&token) {
            self.push_split_token(&token, kinds);
        } else {
            self.push_buffered_token(token.kind, token.range, token.leading, token.trailing);
        }
    }

    fn push_split_token(&mut self, token: &LexedToken<L>, kinds: &[L::Kind]) {
        let start = token.range.start();
        let last_index = kinds.len().saturating_sub(1);
        for (index, kind) in kinds.iter().copied().enumerate() {
            let token_start = start + TextSize::new(index);
            self.push_buffered_token(
                kind,
                TextRange::new(token_start, token_start + TextSize::new(1)),
                if index == 0 {
                    token.leading.start..token.leading.end
                } else {
                    self.empty_trivia()
                },
                if index == last_index {
                    token.trailing.start..token.trailing.end
                } else {
                    self.empty_trivia()
                },
            );
        }
    }

    fn push_buffered_token(
        &mut self,
        kind: L::Kind,
        range: TextRange,
        leading: Range<usize>,
        trailing: Range<usize>,
    ) {
        let text_len =
            self.trivia_text_len(&leading) + range.len() + self.trivia_text_len(&trailing);
        self.tokens.push(SyntaxTokenData::new(
            L::kind_to_raw(kind),
            range,
            leading,
            trailing,
            text_len,
        ));
    }

    fn trivia_text_len(&self, range: &Range<usize>) -> TextSize {
        self.trivia[range.start..range.end]
            .iter()
            .fold(TextSize::new(0), |len, trivia| len + trivia.text_len())
    }

    fn empty_trivia(&self) -> Range<usize> {
        self.trivia.len()..self.trivia.len()
    }

    fn finish(
        mut self,
        committed_len: usize,
    ) -> (Vec<SyntaxTokenData>, Vec<SyntaxTrivia>, Vec<Diagnostic>) {
        self.tokens.truncate(committed_len.min(self.tokens.len()));
        let trivia_len = self
            .tokens
            .iter()
            .flat_map(|token| [token.leading().end, token.trailing().end])
            .max()
            .unwrap_or(0);
        self.trivia.truncate(trivia_len);
        let diagnostics = self.lexer.finish();
        (self.tokens, self.trivia, diagnostics)
    }
}
