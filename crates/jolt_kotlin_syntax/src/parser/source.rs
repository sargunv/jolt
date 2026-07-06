#![allow(dead_code)]

use std::ops::Range;

use jolt_diagnostics::{Diagnostic, DiagnosticStage, Severity};
use jolt_syntax::{
    CompletedMarker, Event, Marker, SyntaxTokenData, SyntaxTrivia, TriviaKind as SyntaxTriviaKind,
};
use jolt_text::{TextRange, TextSize};

use crate::{KotlinLexer, KotlinSyntaxKind, lexer::LexedToken};

use super::KotlinParseDiagnosticCode;

pub(super) struct ParseEvents {
    pub(super) events: Vec<Event>,
    pub(super) tokens: Vec<SyntaxTokenData>,
    pub(super) trivia: Vec<SyntaxTrivia>,
    pub(super) diagnostics: Vec<Diagnostic>,
}

pub(super) struct Parser<'source> {
    pub(in crate::parser) source: &'source str,
    pub(in crate::parser) buffer: TokenBuffer<'source>,
    cursor: TokenCursor,
    events: Vec<Event>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct CursorCheckpoint {
    pos: usize,
}

#[derive(Clone, Copy)]
pub(super) struct TokenCursor {
    pos: usize,
}

impl<'source> Parser<'source> {
    pub(super) fn new(source: &'source str) -> Self {
        Self {
            source,
            buffer: TokenBuffer::new(source),
            cursor: TokenCursor::new(),
            events: Vec::new(),
        }
    }

    pub(super) fn finish(self) -> ParseEvents {
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

    pub(super) const fn position(&self) -> usize {
        self.cursor.position()
    }

    pub(super) fn expect(&mut self, kind: KotlinSyntaxKind, message: &str) {
        if !self.eat(kind) {
            self.expected_here(message);
        }
    }

    pub(super) fn eat(&mut self, kind: KotlinSyntaxKind) -> bool {
        if self.at(kind) {
            self.bump();
            true
        } else {
            false
        }
    }

    pub(super) fn at(&mut self, kind: KotlinSyntaxKind) -> bool {
        self.current_kind() == kind
    }

    pub(super) fn at_eof(&mut self) -> bool {
        self.current_kind() == KotlinSyntaxKind::Eof
    }

    pub(super) fn current_kind(&mut self) -> KotlinSyntaxKind {
        self.cursor.kind(&mut self.buffer)
    }

    pub(super) fn nth_kind(&mut self, n: usize) -> KotlinSyntaxKind {
        self.cursor.nth_kind(&mut self.buffer, n)
    }

    pub(super) fn kind_at(&mut self, index: usize) -> KotlinSyntaxKind {
        self.buffer.kind_at(index)
    }

    pub(super) fn current_text(&mut self) -> Option<&'source str> {
        self.cursor.text(self.source, &mut self.buffer)
    }

    pub(super) fn text_at(&mut self, index: usize) -> Option<&'source str> {
        self.buffer.text_at(self.source, index)
    }

    pub(super) fn tokens_are_adjacent(&mut self, index: usize, count: usize) -> bool {
        self.buffer.tokens_are_adjacent(index, count)
    }

    pub(super) fn newline_before_current(&mut self) -> bool {
        self.buffer.newline_before(self.cursor.position())
    }

    pub(super) fn newline_between(&mut self, left: usize, right: usize) -> bool {
        self.buffer.newline_between(left, right)
    }

    pub(super) fn at_semicolon_boundary(&mut self) -> bool {
        matches!(
            self.current_kind(),
            KotlinSyntaxKind::Semicolon
                | KotlinSyntaxKind::DoubleSemicolon
                | KotlinSyntaxKind::RBrace
                | KotlinSyntaxKind::Eof
        ) || self.newline_before_current()
    }

    pub(super) fn eat_semicolon_boundary(&mut self) -> bool {
        let mut ate = false;
        while matches!(
            self.current_kind(),
            KotlinSyntaxKind::Semicolon | KotlinSyntaxKind::DoubleSemicolon
        ) {
            self.bump();
            ate = true;
        }
        ate || self.at_semicolon_boundary()
    }

    pub(super) fn bump(&mut self) {
        self.cursor.bump(&mut self.buffer);
        self.events.push(Event::Token);
    }

    pub(super) fn fork_cursor(&self) -> TokenCursor {
        self.cursor.fork()
    }

    pub(super) fn expected_here(&mut self, message: &str) {
        self.error_here(KotlinParseDiagnosticCode::ExpectedSyntax, message);
    }

    pub(super) fn unexpected_here(&mut self, message: &str) {
        self.error_here(KotlinParseDiagnosticCode::UnexpectedSyntax, message);
    }

    pub(super) fn ensure_progress(&mut self, before: usize, message: &str) {
        if self.position() == before {
            self.unexpected_here(message);
            if !self.at_eof() {
                self.bump();
            }
        }
    }

    pub(super) fn invalid_assignment_target_here(&mut self, message: &str) {
        self.error_here(KotlinParseDiagnosticCode::InvalidAssignmentTarget, message);
    }

    pub(super) fn malformed_type_argument_list_here(&mut self, message: &str) {
        self.error_here(
            KotlinParseDiagnosticCode::MalformedTypeArgumentList,
            message,
        );
    }

    pub(super) fn invalid_when_guard_here(&mut self, message: &str) {
        self.error_here(KotlinParseDiagnosticCode::InvalidWhenGuard, message);
    }

    pub(super) fn reserved_callable_reference_call_here(&mut self, message: &str) {
        self.error_here(
            KotlinParseDiagnosticCode::ReservedCallableReferenceCall,
            message,
        );
    }

    fn error_here(&mut self, code: KotlinParseDiagnosticCode, message: &str) {
        let range = self
            .cursor
            .range(&mut self.buffer)
            .or_else(|| self.buffer.last_token_range())
            .expect("parser token stream must include EOF");
        self.events.push(Event::Error(Diagnostic {
            code: code.id(),
            severity: Severity::Error,
            stage: DiagnosticStage::Parser,
            message: message.to_owned(),
            range: Some(range),
        }));
    }

    pub(super) fn start(&mut self) -> Marker {
        Marker::new(&mut self.events)
    }

    pub(super) fn complete(&mut self, marker: Marker, kind: KotlinSyntaxKind) -> CompletedMarker {
        marker.complete(&mut self.events, kind.to_raw())
    }

    pub(super) fn precede(&mut self, marker: CompletedMarker) -> Marker {
        marker.precede(&mut self.events)
    }

    pub(super) fn completed_is_error_node(marker: &CompletedMarker) -> bool {
        marker.kind() == KotlinSyntaxKind::ErrorNode.to_raw()
    }

    pub(super) fn abandon(&mut self, marker: Marker) {
        marker.abandon(&mut self.events);
    }
}

impl TokenCursor {
    const fn new() -> Self {
        Self { pos: 0 }
    }

    pub(super) const fn position(self) -> usize {
        self.pos
    }

    pub(super) fn kind(self, buffer: &mut TokenBuffer<'_>) -> KotlinSyntaxKind {
        buffer.kind_at(self.pos)
    }

    pub(super) fn nth_kind(self, buffer: &mut TokenBuffer<'_>, n: usize) -> KotlinSyntaxKind {
        buffer.kind_at(self.pos + n)
    }

    pub(super) fn text<'source>(
        self,
        source: &'source str,
        buffer: &mut TokenBuffer<'source>,
    ) -> Option<&'source str> {
        let range = self.range(buffer)?;
        Some(source_text(source, range))
    }

    pub(super) fn range(self, buffer: &mut TokenBuffer<'_>) -> Option<TextRange> {
        buffer.range_at(self.pos)
    }

    pub(super) fn bump(&mut self, buffer: &mut TokenBuffer<'_>) {
        buffer.ensure(self.pos);
        self.pos += 1;
    }

    pub(super) const fn checkpoint(self) -> CursorCheckpoint {
        CursorCheckpoint { pos: self.pos }
    }

    pub(super) fn rewind(&mut self, checkpoint: CursorCheckpoint) {
        self.pos = checkpoint.pos;
    }

    pub(super) const fn fork(self) -> Self {
        self
    }
}

fn source_text(source: &str, range: TextRange) -> &str {
    let start = range.start().get();
    let end = range.end().get();
    &source[start..end]
}

pub(super) struct TokenBuffer<'source> {
    lexer: KotlinLexer<'source>,
    tokens: Vec<SyntaxTokenData>,
    trivia: Vec<SyntaxTrivia>,
}

impl<'source> TokenBuffer<'source> {
    fn new(source: &'source str) -> Self {
        Self {
            lexer: KotlinLexer::new(source),
            tokens: Vec::new(),
            trivia: Vec::new(),
        }
    }

    fn kind_at(&mut self, index: usize) -> KotlinSyntaxKind {
        self.ensure(index);
        self.tokens
            .get(index)
            .map_or(KotlinSyntaxKind::Eof, |token| {
                KotlinSyntaxKind::from_raw(token.raw_kind()).unwrap_or(KotlinSyntaxKind::Eof)
            })
    }

    fn range_at(&mut self, index: usize) -> Option<TextRange> {
        self.ensure(index);
        self.tokens
            .get(index)
            .map(SyntaxTokenData::token_text_range)
    }

    pub(in crate::parser) fn text_at(
        &mut self,
        source: &'source str,
        index: usize,
    ) -> Option<&'source str> {
        let range = self.range_at(index)?;
        Some(source_text(source, range))
    }

    fn last_token_range(&mut self) -> Option<TextRange> {
        self.ensure(0);
        self.tokens.last().map(SyntaxTokenData::token_text_range)
    }

    fn tokens_are_adjacent(&mut self, index: usize, count: usize) -> bool {
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

    fn newline_before(&mut self, index: usize) -> bool {
        self.ensure(index);
        self.tokens
            .get(index)
            .is_some_and(|token| self.trivia_has_newline(token.leading()))
    }

    fn newline_between(&mut self, left: usize, right: usize) -> bool {
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

    fn trivia_has_newline(&self, range: &Range<usize>) -> bool {
        self.trivia[range.start..range.end]
            .iter()
            .any(|trivia| trivia.kind() == SyntaxTriviaKind::Newline)
    }

    fn ensure(&mut self, index: usize) {
        while self.tokens.len() <= index
            && self
                .tokens
                .last()
                .is_none_or(|token| token.raw_kind() != KotlinSyntaxKind::Eof.to_raw())
        {
            let token = self.lexer.next_token_into(&mut self.trivia);
            self.push_token(token);
        }
    }

    fn push_token(&mut self, token: LexedToken) {
        self.push_buffered_token(token.kind, token.range, token.leading, token.trailing);
    }

    fn push_buffered_token(
        &mut self,
        kind: KotlinSyntaxKind,
        range: TextRange,
        leading: Range<usize>,
        trailing: Range<usize>,
    ) {
        let text_len =
            self.trivia_text_len(&leading) + range.len() + self.trivia_text_len(&trailing);
        self.tokens.push(SyntaxTokenData::new(
            kind.to_raw(),
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
