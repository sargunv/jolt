use jolt_diagnostics::{Diagnostic, DiagnosticCode, DiagnosticStage, Severity};
use jolt_syntax::{CompletedMarker, Event, Marker};
use jolt_text::{TextRange, TextSize};

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
    source: &'source str,
    /// Immutable lexer token stream used for parser lookahead.
    tokens: Vec<ParserToken>,
    /// Logical tokens consumed by the parser and passed to the tree builder.
    tree_tokens: Vec<ParserToken>,
    pos: usize,
    pending_gt_split: Option<PendingGtSplit>,
    events: Vec<Event>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PendingGtSplit {
    original_index: usize,
    next_part: u8,
    total_parts: u8,
}

impl<'source> Parser<'source> {
    pub(super) fn new(source: &'source str, tokens: Vec<Token>) -> Self {
        let token_capacity = tokens.len();
        let event_capacity = token_capacity.saturating_mul(2);
        Self {
            source,
            tokens: tokens.into_iter().map(ParserToken::from).collect(),
            tree_tokens: Vec::with_capacity(token_capacity),
            pos: 0,
            pending_gt_split: None,
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
        self.pos
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
        self.logical_token_at(self.pos)
            .map_or(JavaSyntaxKind::Eof, |token| token.kind)
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

    pub(super) fn current_text(&self) -> Option<&'source str> {
        self.logical_token_at(self.pos)
            .map(|token| self.token_text(&token))
    }

    pub(super) fn text_at(&self, index: usize) -> Option<&'source str> {
        let token = self.tokens.get(index)?;
        Some(self.token_text(token))
    }

    pub(super) fn bump(&mut self) {
        if self.pending_gt_split.is_some() {
            self.bump_pending_gt_split();
            return;
        }

        let token = self
            .tokens
            .get(self.pos)
            .expect("parser attempted to consume beyond EOF token")
            .clone();
        self.events.push(Event::Token);
        self.tree_tokens.push(token);
        self.pos += 1;
    }

    pub(super) fn bump_split_gt(&mut self) {
        match self.current_kind() {
            JavaSyntaxKind::Gt if self.pending_gt_split.is_some() => self.bump_pending_gt_split(),
            JavaSyntaxKind::Gt => self.bump(),
            JavaSyntaxKind::RShift => self.start_pending_gt_split(2),
            JavaSyntaxKind::UnsignedRShift => self.start_pending_gt_split(3),
            _ => self.expected_here("expected `>`"),
        }
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
            .logical_token_at(self.pos)
            .or_else(|| self.tokens.last().cloned())
            .expect("parser token stream must include EOF")
            .range;
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

    fn start_pending_gt_split(&mut self, total_parts: u8) {
        self.pending_gt_split = Some(PendingGtSplit {
            original_index: self.pos,
            next_part: 0,
            total_parts,
        });
        self.bump_pending_gt_split();
    }

    fn bump_pending_gt_split(&mut self) {
        let split = self
            .pending_gt_split
            .expect("pending split must exist before bumping virtual `>`");
        let token = self.virtual_gt_token(split);
        self.events.push(Event::Token);
        self.tree_tokens.push(token);

        let next_part = split.next_part + 1;
        if next_part == split.total_parts {
            self.pending_gt_split = None;
            self.pos += 1;
        } else {
            self.pending_gt_split = Some(PendingGtSplit { next_part, ..split });
        }
    }

    fn logical_token_at(&self, index: usize) -> Option<ParserToken> {
        if index == self.pos
            && let Some(split) = self.pending_gt_split
        {
            return Some(self.virtual_gt_token(split));
        }

        self.tokens.get(index).cloned()
    }

    fn virtual_gt_token(&self, split: PendingGtSplit) -> ParserToken {
        let token = &self.tokens[split.original_index];
        let token_start = token.range.start() + TextSize::new(usize::from(split.next_part));
        let token_end = token_start + TextSize::new(1);
        ParserToken {
            kind: JavaSyntaxKind::Gt,
            range: TextRange::new(token_start, token_end),
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

    fn token_text(&self, token: &ParserToken) -> &'source str {
        let start = token.range.start().get();
        let end = token.range.end().get();
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
