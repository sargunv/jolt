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
    tokens: Vec<ParserToken>,
    pos: usize,
    events: Vec<Event>,
}

impl<'source> Parser<'source> {
    pub(super) fn new(source: &'source str, tokens: Vec<Token>) -> Self {
        let capacity = tokens.len().saturating_mul(2);
        Self {
            source,
            tokens: tokens.into_iter().map(ParserToken::from).collect(),
            pos: 0,
            events: Vec::with_capacity(capacity),
        }
    }

    pub(super) fn finish(self) -> ParseEvents {
        ParseEvents {
            events: self.events,
            tokens: self.tokens,
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
        self.kind_at(self.pos)
    }

    pub(super) fn nth_kind(&self, n: usize) -> JavaSyntaxKind {
        self.kind_at(self.pos + n)
    }

    pub(super) fn kind_at(&self, index: usize) -> JavaSyntaxKind {
        self.tokens
            .get(index)
            .map_or(JavaSyntaxKind::Eof, |token| token.kind)
    }

    pub(super) fn current_text(&self) -> Option<&'source str> {
        self.text_at(self.pos)
    }

    pub(super) fn text_at(&self, index: usize) -> Option<&'source str> {
        let token = self.tokens.get(index)?;
        let start = token.range.start().get();
        let end = token.range.end().get();
        Some(&self.source[start..end])
    }

    pub(super) fn bump(&mut self) {
        assert!(
            self.pos < self.tokens.len(),
            "parser attempted to consume beyond EOF token"
        );
        self.events.push(Event::Token);
        self.pos += 1;
    }

    pub(super) fn bump_split_gt(&mut self) {
        self.split_current_gt_token();
        self.expect(JavaSyntaxKind::Gt, "expected `>`");
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
            .tokens
            .get(self.pos)
            .or_else(|| self.tokens.last())
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

    fn split_current_gt_token(&mut self) {
        let Some(token) = self.tokens.get(self.pos) else {
            return;
        };

        let split_count = match token.kind {
            JavaSyntaxKind::RShift => 2,
            JavaSyntaxKind::UnsignedRShift => 3,
            _ => return,
        };

        let token = self.tokens.remove(self.pos);
        let start = token.range.start();
        let mut split_tokens = Vec::with_capacity(split_count);

        for index in 0..split_count {
            let token_start = start + TextSize::new(index);
            let token_end = token_start + TextSize::new(1);
            split_tokens.push(ParserToken {
                kind: JavaSyntaxKind::Gt,
                range: TextRange::new(token_start, token_end),
                leading: if index == 0 {
                    token.leading.clone()
                } else {
                    Vec::new()
                },
                trailing: if index + 1 == split_count {
                    token.trailing.clone()
                } else {
                    Vec::new()
                },
            });
        }

        self.tokens.splice(self.pos..self.pos, split_tokens);
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
