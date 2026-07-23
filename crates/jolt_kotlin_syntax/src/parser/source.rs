use std::ops::{Deref, DerefMut};

use crate::KotlinSyntaxKind as K;
use crate::language::KotlinLanguage;
use jolt_syntax::PendingDiagnostic;

use crate::parser::KotlinParseDiagnosticCode;

pub(super) type ParseEvents = jolt_syntax::ParseEvents;
pub(super) type TokenBuffer<'source> = jolt_syntax::TokenBuffer<'source, KotlinLanguage>;
pub(super) use jolt_syntax::TokenCursor;

// Resource budget for simultaneously active recursive grammar owners. This is
// not source depth: one construct may activate more than one owner. The value
// leaves conservative headroom on the optimized plugin's 1 MiB WASM stack.
pub(super) const MAX_RECURSIVE_PARSE_OWNERS: usize = 128;

pub(super) struct Parser<'source> {
    inner: jolt_syntax::Parser<'source, KotlinLanguage>,
    syntax_nesting_depth: usize,
}

impl<'source> Parser<'source> {
    pub(super) fn new(source: &'source str) -> Self {
        Self {
            inner: jolt_syntax::Parser::new(source),
            syntax_nesting_depth: 0,
        }
    }

    pub(super) fn finish(self) -> ParseEvents {
        debug_assert_eq!(
            self.syntax_nesting_depth, 0,
            "syntax nesting depth must unwind at EOF"
        );
        self.inner.finish()
    }

    pub(super) fn with_syntax_nesting<T>(
        &mut self,
        parse: impl FnOnce(&mut Self) -> T,
    ) -> Option<T> {
        if self.syntax_nesting_depth >= MAX_RECURSIVE_PARSE_OWNERS {
            return None;
        }

        self.syntax_nesting_depth += 1;
        let parsed = parse(self);
        self.syntax_nesting_depth -= 1;
        Some(parsed)
    }

    pub(super) fn pending_excessive_syntax_nesting(&mut self) -> PendingDiagnostic {
        self.pending_error(
            KotlinParseDiagnosticCode::ExcessiveSyntaxNesting.id(),
            "syntax is too deeply nested to parse safely",
        )
    }

    pub(super) fn newline_before_current(&mut self) -> bool {
        self.inner.buffer.newline_before(self.inner.position())
    }

    pub(super) fn newline_between(&mut self, left: usize, right: usize) -> bool {
        self.inner.buffer.newline_between(left, right)
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

    pub(super) fn invalid_assignment_target_here(&mut self, message: &str) -> PendingDiagnostic {
        self.pending_error(
            KotlinParseDiagnosticCode::InvalidAssignmentTarget.id(),
            message,
        )
    }

    pub(super) fn malformed_type_argument_list_here(&mut self, message: &str) -> PendingDiagnostic {
        self.pending_error(
            KotlinParseDiagnosticCode::MalformedTypeArgumentList.id(),
            message,
        )
    }

    pub(super) fn invalid_when_guard_here(&mut self, message: &str) {
        let diagnostic =
            self.pending_error(KotlinParseDiagnosticCode::InvalidWhenGuard.id(), message);
        self.report_non_structural(diagnostic);
    }

    pub(super) fn reserved_callable_reference_call_here(&mut self, message: &str) {
        let diagnostic = self.pending_error(
            KotlinParseDiagnosticCode::ReservedCallableReferenceCall.id(),
            message,
        );
        self.report_non_structural(diagnostic);
    }
}

impl<'source> Deref for Parser<'source> {
    type Target = jolt_syntax::Parser<'source, KotlinLanguage>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for Parser<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
