use std::ops::Range;

use jolt_diagnostics::{Diagnostic, DiagnosticCode, DiagnosticStage, Severity};
use jolt_syntax::{
    CompletedMarker, Event, Marker, SyntaxTokenData, SyntaxTrivia, TriviaKind as SyntaxTriviaKind,
};
use jolt_text::{TextRange, TextSize};

use crate::{JavaLexer, JavaSyntaxKind, Trivia, lexer::LexedToken};

use super::JavaParseDiagnosticCode;

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

    pub(super) fn expect(&mut self, kind: JavaSyntaxKind, message: &str) {
        if !self.eat(kind) {
            self.expected_here(message);
        }
    }

    pub(super) fn expect_contextual(&mut self, text: &str, message: &str) {
        if !self.eat_contextual(text) {
            self.expected_here(message);
        }
    }

    pub(super) fn eat(&mut self, kind: JavaSyntaxKind) -> bool {
        if self.at(kind) {
            self.bump();
            true
        } else {
            false
        }
    }

    pub(super) fn eat_contextual(&mut self, text: &str) -> bool {
        if self.at_contextual(text) {
            self.bump();
            true
        } else {
            false
        }
    }

    pub(super) fn at(&mut self, kind: JavaSyntaxKind) -> bool {
        self.current_kind() == kind
    }

    pub(super) fn at_contextual(&mut self, text: &str) -> bool {
        self.current_kind() == JavaSyntaxKind::Identifier && self.current_text() == Some(text)
    }

    pub(super) fn at_eof(&mut self) -> bool {
        self.current_kind() == JavaSyntaxKind::Eof
    }

    pub(super) fn current_kind(&mut self) -> JavaSyntaxKind {
        self.cursor.kind(&mut self.buffer)
    }

    pub(super) fn nth_kind(&mut self, n: usize) -> JavaSyntaxKind {
        self.cursor.nth_kind(&mut self.buffer, n)
    }

    pub(super) fn kind_at(&mut self, index: usize) -> JavaSyntaxKind {
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

    pub(super) fn bump(&mut self) {
        self.cursor.bump(&mut self.buffer);
        self.events.push(Event::Token);
    }

    pub(super) fn fork_cursor(&self) -> TokenCursor {
        self.cursor.fork()
    }

    pub(super) fn expected_here(&mut self, message: &str) {
        self.error_here(JavaParseDiagnosticCode::ExpectedSyntax, message);
    }

    pub(super) fn unexpected_here(&mut self, message: &str) {
        self.error_here(JavaParseDiagnosticCode::UnexpectedSyntax, message);
    }

    pub(super) fn invalid_statement_expression_here(&mut self, message: &str) {
        self.error_here(JavaParseDiagnosticCode::InvalidStatementExpression, message);
    }

    pub(super) fn invalid_resource_variable_access_here(&mut self, message: &str) {
        self.error_here(
            JavaParseDiagnosticCode::InvalidResourceVariableAccess,
            message,
        );
    }

    pub(super) fn invalid_switch_guard_here(&mut self, message: &str) {
        self.error_here(JavaParseDiagnosticCode::InvalidSwitchGuard, message);
    }

    pub(super) fn unqualified_yield_method_invocation_here(&mut self, message: &str) {
        self.error_here(
            JavaParseDiagnosticCode::UnqualifiedYieldMethodInvocation,
            message,
        );
    }

    pub(super) fn decimal_integer_boundary_literal_here(&mut self, message: &str) {
        self.error_here(
            JavaParseDiagnosticCode::DecimalIntegerBoundaryLiteral,
            message,
        );
    }

    pub(super) fn misplaced_receiver_parameter_here(&mut self, message: &str) {
        self.error_here(JavaParseDiagnosticCode::MisplacedReceiverParameter, message);
    }

    pub(super) fn misplaced_constructor_invocation_here(&mut self, message: &str) {
        self.error_here(
            JavaParseDiagnosticCode::MisplacedConstructorInvocation,
            message,
        );
    }

    pub(super) fn restricted_type_identifier_here(&mut self, message: &str) {
        self.error_here(JavaParseDiagnosticCode::RestrictedTypeIdentifier, message);
    }

    fn error_here(&mut self, code: JavaParseDiagnosticCode, message: &str) {
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

    pub(super) fn complete(&mut self, marker: Marker, kind: JavaSyntaxKind) -> CompletedMarker {
        marker.complete(&mut self.events, kind.to_raw())
    }

    pub(super) fn precede(&mut self, marker: CompletedMarker) -> Marker {
        marker.precede(&mut self.events)
    }

    pub(super) fn completed_is_error_node(marker: &CompletedMarker) -> bool {
        marker.kind() == JavaSyntaxKind::ErrorNode.to_raw()
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

    pub(super) fn kind(self, buffer: &mut TokenBuffer<'_>) -> JavaSyntaxKind {
        buffer.kind_at(self.pos)
    }

    pub(super) fn nth_kind(self, buffer: &mut TokenBuffer<'_>, n: usize) -> JavaSyntaxKind {
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
    lexer: JavaLexer<'source>,
    tokens: Vec<SyntaxTokenData>,
    trivia: Vec<Trivia>,
}

impl<'source> TokenBuffer<'source> {
    fn new(source: &'source str) -> Self {
        Self {
            lexer: JavaLexer::new(source),
            tokens: Vec::new(),
            trivia: Vec::new(),
        }
    }

    fn kind_at(&mut self, index: usize) -> JavaSyntaxKind {
        self.ensure(index);
        self.tokens.get(index).map_or(JavaSyntaxKind::Eof, |token| {
            JavaSyntaxKind::from_raw(token.raw_kind()).unwrap_or(JavaSyntaxKind::Eof)
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

    fn ensure(&mut self, index: usize) {
        while self.tokens.len() <= index
            && self
                .tokens
                .last()
                .is_none_or(|token| token.raw_kind() != JavaSyntaxKind::Eof.to_raw())
        {
            let token = self.lexer.next_token_into(&mut self.trivia);
            self.push_token(token);
        }
    }

    fn push_token(&mut self, token: LexedToken) {
        match token.kind {
            JavaSyntaxKind::GtEq => {
                self.push_split_token(&token, &[JavaSyntaxKind::Gt, JavaSyntaxKind::Assign]);
            }
            JavaSyntaxKind::RShift => {
                self.push_split_token(&token, &[JavaSyntaxKind::Gt, JavaSyntaxKind::Gt]);
            }
            JavaSyntaxKind::UnsignedRShift => {
                self.push_split_token(
                    &token,
                    &[JavaSyntaxKind::Gt, JavaSyntaxKind::Gt, JavaSyntaxKind::Gt],
                );
            }
            JavaSyntaxKind::RShiftEq => {
                self.push_split_token(
                    &token,
                    &[
                        JavaSyntaxKind::Gt,
                        JavaSyntaxKind::Gt,
                        JavaSyntaxKind::Assign,
                    ],
                );
            }
            JavaSyntaxKind::UnsignedRShiftEq => {
                self.push_split_token(
                    &token,
                    &[
                        JavaSyntaxKind::Gt,
                        JavaSyntaxKind::Gt,
                        JavaSyntaxKind::Gt,
                        JavaSyntaxKind::Assign,
                    ],
                );
            }
            _ => {
                self.push_syntax_token(token.kind, token.range, token.leading, token.trailing);
            }
        }
    }

    fn push_split_token(&mut self, token: &LexedToken, kinds: &[JavaSyntaxKind]) {
        let start = token.range.start();
        let last_index = kinds.len().saturating_sub(1);
        for (index, kind) in kinds.iter().copied().enumerate() {
            let token_start = start + TextSize::new(index);
            self.push_syntax_token(
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

    fn push_syntax_token(
        &mut self,
        kind: JavaSyntaxKind,
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
            .fold(TextSize::new(0), |len, trivia| len + trivia.range.len())
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
        let trivia = self
            .trivia
            .into_iter()
            .map(|trivia| SyntaxTrivia::new(to_syntax_trivia_kind(trivia.kind), trivia.range.len()))
            .collect();
        let diagnostics = self.lexer.finish();
        (self.tokens, trivia, diagnostics)
    }
}

fn to_syntax_trivia_kind(kind: crate::TriviaKind) -> SyntaxTriviaKind {
    match kind {
        crate::TriviaKind::Whitespace => SyntaxTriviaKind::Whitespace,
        crate::TriviaKind::Newline => SyntaxTriviaKind::Newline,
        crate::TriviaKind::LineComment => SyntaxTriviaKind::LineComment,
        crate::TriviaKind::BlockComment => SyntaxTriviaKind::BlockComment,
        crate::TriviaKind::JavadocComment => SyntaxTriviaKind::DocComment,
        crate::TriviaKind::Ignored => SyntaxTriviaKind::Ignored,
    }
}
