use jolt_diagnostics::{Diagnostic, DiagnosticCode, DiagnosticStage, Severity};
use jolt_syntax::{CompletedMarker, Event, Marker};
use jolt_text::{TextRange, TextSize};
use std::sync::Arc;

use crate::{JavaSyntaxKind, Token, Trivia};

use super::JavaParseDiagnosticCode;

pub(super) struct ParseEvents {
    pub(super) events: Vec<Event>,
    pub(super) tokens: Vec<ParserToken>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct ParserToken {
    pub(super) kind: JavaSyntaxKind,
    pub(super) range: TextRange,
    pub(super) leading: Vec<Trivia>,
    pub(super) trailing: Vec<Trivia>,
}

pub(super) struct Parser<'source> {
    cursor: TokenCursor<'source>,
    /// Logical tokens consumed by the parser and passed to the tree builder.
    tree_tokens: Vec<ParserToken>,
    events: Vec<Event>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct CursorCheckpoint {
    pos: usize,
    pending_gt_split: Option<PendingGtSplit>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct PendingGtSplit {
    original_index: usize,
    next_part: u8,
    total_parts: u8,
}

#[derive(Clone, Debug)]
pub(super) struct TokenCursor<'source> {
    source: &'source str,
    tokens: Arc<[ParserToken]>,
    pos: usize,
    pending_gt_split: Option<PendingGtSplit>,
}

impl<'source> Parser<'source> {
    pub(super) fn new(source: &'source str, tokens: Vec<Token>) -> Self {
        let token_capacity = tokens.len();
        let event_capacity = token_capacity.saturating_mul(2);
        Self {
            cursor: TokenCursor::new(source, tokens.into_iter().map(ParserToken::from).collect()),
            tree_tokens: Vec::with_capacity(token_capacity),
            events: Vec::with_capacity(event_capacity),
        }
    }

    pub(super) fn finish(self) -> ParseEvents {
        ParseEvents {
            events: self.events,
            tokens: self.tree_tokens,
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

    pub(super) fn at(&self, kind: JavaSyntaxKind) -> bool {
        self.current_kind() == kind
    }

    pub(super) fn at_contextual(&self, text: &str) -> bool {
        self.current_kind() == JavaSyntaxKind::Identifier && self.current_text() == Some(text)
    }

    pub(super) fn at_eof(&self) -> bool {
        self.current_kind() == JavaSyntaxKind::Eof
    }

    pub(super) fn current_kind(&self) -> JavaSyntaxKind {
        self.cursor.kind()
    }

    pub(super) fn nth_kind(&self, n: usize) -> JavaSyntaxKind {
        self.cursor.nth_kind(n)
    }

    pub(super) fn kind_at(&self, index: usize) -> JavaSyntaxKind {
        self.cursor.kind_at(index)
    }

    pub(super) fn current_text(&self) -> Option<&'source str> {
        self.cursor.text()
    }

    pub(super) fn text_at(&self, index: usize) -> Option<&'source str> {
        self.cursor.text_at(index)
    }

    pub(super) fn bump(&mut self) {
        let token = self.cursor.bump();
        self.events.push(Event::Token);
        self.tree_tokens.push(token);
    }

    pub(super) fn bump_split_gt(&mut self) {
        if let Some(token) = self.cursor.bump_split_gt() {
            self.events.push(Event::Token);
            self.tree_tokens.push(token);
        } else {
            self.expected_here("expected `>`");
        }
    }

    pub(super) fn fork_cursor(&self) -> TokenCursor<'source> {
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
            .range()
            .or_else(|| self.cursor.last_token().map(|token| token.range))
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

impl<'source> TokenCursor<'source> {
    fn new(source: &'source str, tokens: Vec<ParserToken>) -> Self {
        Self {
            source,
            tokens: tokens.into(),
            pos: 0,
            pending_gt_split: None,
        }
    }

    pub(super) const fn position(&self) -> usize {
        self.pos
    }

    pub(super) fn kind(&self) -> JavaSyntaxKind {
        if self.pending_gt_split.is_some() {
            JavaSyntaxKind::Gt
        } else {
            self.kind_at(self.pos)
        }
    }

    pub(super) fn nth_kind(&self, n: usize) -> JavaSyntaxKind {
        let Some(split) = self.pending_gt_split else {
            return self.kind_at(self.pos + n);
        };

        let remaining_split_parts = usize::from(split.total_parts - split.next_part);
        if n < remaining_split_parts {
            JavaSyntaxKind::Gt
        } else {
            self.kind_at(self.pos + 1 + n - remaining_split_parts)
        }
    }

    pub(super) fn kind_at(&self, index: usize) -> JavaSyntaxKind {
        self.tokens
            .get(index)
            .map_or(JavaSyntaxKind::Eof, |token| token.kind)
    }

    pub(super) fn text(&self) -> Option<&'source str> {
        self.range().map(|range| self.source_text(range))
    }

    pub(super) fn text_at(&self, index: usize) -> Option<&'source str> {
        let token = self.tokens.get(index)?;
        Some(self.token_text(token))
    }

    pub(super) fn range(&self) -> Option<TextRange> {
        if let Some(split) = self.pending_gt_split {
            Some(self.virtual_gt_range(split))
        } else {
            self.tokens.get(self.pos).map(|token| token.range)
        }
    }

    pub(super) fn last_token(&self) -> Option<&ParserToken> {
        self.tokens.last()
    }

    pub(super) fn bump(&mut self) -> ParserToken {
        if self.pending_gt_split.is_some() {
            return self.bump_pending_gt_split();
        }

        let token = self
            .tokens
            .get(self.pos)
            .expect("parser attempted to consume beyond EOF token")
            .clone();
        self.pos += 1;
        token
    }

    pub(super) fn advance(&mut self) {
        if self.pending_gt_split.is_some() {
            self.advance_pending_gt_split();
            return;
        }

        self.tokens
            .get(self.pos)
            .expect("parser attempted to advance beyond EOF token");
        self.pos += 1;
    }

    pub(super) fn bump_split_gt(&mut self) -> Option<ParserToken> {
        match self.kind() {
            JavaSyntaxKind::Gt if self.pending_gt_split.is_some() => {
                Some(self.bump_pending_gt_split())
            }
            JavaSyntaxKind::Gt => Some(self.bump()),
            JavaSyntaxKind::RShift => Some(self.start_pending_gt_split(2)),
            JavaSyntaxKind::UnsignedRShift => Some(self.start_pending_gt_split(3)),
            _ => None,
        }
    }

    pub(super) const fn checkpoint(&self) -> CursorCheckpoint {
        CursorCheckpoint {
            pos: self.pos,
            pending_gt_split: self.pending_gt_split,
        }
    }

    pub(super) fn rewind(&mut self, checkpoint: CursorCheckpoint) {
        self.pos = checkpoint.pos;
        self.pending_gt_split = checkpoint.pending_gt_split;
    }

    pub(super) fn fork(&self) -> Self {
        self.clone()
    }

    fn start_pending_gt_split(&mut self, total_parts: u8) -> ParserToken {
        self.pending_gt_split = Some(PendingGtSplit {
            original_index: self.pos,
            next_part: 0,
            total_parts,
        });
        self.bump_pending_gt_split()
    }

    fn bump_pending_gt_split(&mut self) -> ParserToken {
        let split = self
            .pending_gt_split
            .expect("pending split must exist before bumping virtual `>`");
        let token = self.virtual_gt_token(split);
        self.finish_pending_gt_split(split);
        token
    }

    fn advance_pending_gt_split(&mut self) {
        let split = self
            .pending_gt_split
            .expect("pending split must exist before advancing virtual `>`");
        self.finish_pending_gt_split(split);
    }

    fn finish_pending_gt_split(&mut self, split: PendingGtSplit) {
        let next_part = split.next_part + 1;
        if next_part == split.total_parts {
            self.pending_gt_split = None;
            self.pos += 1;
        } else {
            self.pending_gt_split = Some(PendingGtSplit { next_part, ..split });
        }
    }

    fn virtual_gt_token(&self, split: PendingGtSplit) -> ParserToken {
        let token = &self.tokens[split.original_index];
        ParserToken {
            kind: JavaSyntaxKind::Gt,
            range: self.virtual_gt_range(split),
            leading: if split.next_part == 0 {
                token.leading.clone()
            } else {
                Vec::new()
            },
            trailing: if split.next_part + 1 == split.total_parts {
                token.trailing.clone()
            } else {
                Vec::new()
            },
        }
    }

    fn virtual_gt_range(&self, split: PendingGtSplit) -> TextRange {
        let token = &self.tokens[split.original_index];
        let token_start = token.range.start() + TextSize::new(usize::from(split.next_part));
        let token_end = token_start + TextSize::new(1);
        TextRange::new(token_start, token_end)
    }

    fn token_text(&self, token: &ParserToken) -> &'source str {
        self.source_text(token.range)
    }

    fn source_text(&self, range: TextRange) -> &'source str {
        let start = range.start().get();
        let end = range.end().get();
        &self.source[start..end]
    }
}

impl From<Token> for ParserToken {
    fn from(token: Token) -> Self {
        Self {
            kind: token.kind,
            range: token.range,
            leading: token.leading,
            trailing: token.trailing,
        }
    }
}
