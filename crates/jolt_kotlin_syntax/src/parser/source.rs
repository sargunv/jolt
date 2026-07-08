use std::ops::{Deref, DerefMut};

use crate::KotlinSyntaxKind as K;
use crate::language::KotlinLanguage;
use crate::parser::KotlinParseDiagnosticCode;

pub(super) type ParseEvents = jolt_syntax::ParseEvents;
pub(super) type TokenBuffer<'source> = jolt_syntax::TokenBuffer<'source, KotlinLanguage>;
pub(super) use jolt_syntax::TokenCursor;

pub(super) struct Parser<'source>(pub jolt_syntax::Parser<'source, KotlinLanguage>);

impl<'source> Parser<'source> {
    pub(super) fn new(source: &'source str) -> Self {
        Self(jolt_syntax::Parser::new(source))
    }

    pub(super) fn finish(self) -> ParseEvents {
        self.0.finish()
    }

    pub(super) const fn position(&self) -> usize {
        self.0.position()
    }

    pub(super) fn current_text(&mut self) -> Option<&'source str> {
        self.0.current_text()
    }

    pub(super) fn kind_at(&mut self, index: usize) -> K {
        self.0.kind_at(index)
    }

    pub(super) fn tokens_are_adjacent(&mut self, index: usize, count: usize) -> bool {
        self.0.tokens_are_adjacent(index, count)
    }

    pub(super) fn newline_before_current(&mut self) -> bool {
        self.0.buffer.newline_before(self.0.position())
    }

    pub(super) fn newline_between(&mut self, left: usize, right: usize) -> bool {
        self.0.buffer.newline_between(left, right)
    }

    pub(super) fn at_semicolon_boundary(&mut self) -> bool {
        matches!(
            self.current_kind(),
            K::Semicolon | K::DoubleSemicolon | K::RBrace | K::Eof,
        ) || self.newline_before_current()
    }

    pub(super) fn eat_semicolon_boundary(&mut self) -> bool {
        let mut ate = false;
        while matches!(self.current_kind(), K::Semicolon | K::DoubleSemicolon) {
            self.bump();
            ate = true;
        }
        ate || self.at_semicolon_boundary()
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
        self.error_here(
            KotlinParseDiagnosticCode::InvalidAssignmentTarget.id(),
            message,
        );
    }

    pub(super) fn malformed_type_argument_list_here(&mut self, message: &str) {
        self.error_here(
            KotlinParseDiagnosticCode::MalformedTypeArgumentList.id(),
            message,
        );
    }

    pub(super) fn invalid_when_guard_here(&mut self, message: &str) {
        self.error_here(KotlinParseDiagnosticCode::InvalidWhenGuard.id(), message);
    }

    pub(super) fn reserved_callable_reference_call_here(&mut self, message: &str) {
        self.error_here(
            KotlinParseDiagnosticCode::ReservedCallableReferenceCall.id(),
            message,
        );
    }
}

impl<'source> Deref for Parser<'source> {
    type Target = jolt_syntax::Parser<'source, KotlinLanguage>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Parser<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
