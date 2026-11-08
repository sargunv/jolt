//! The Java parser: the shared parser plus Java's lookahead state.

use std::ops::{Deref, DerefMut};

use jolt_syntax::{LoopGuard, ParseOutput};

use crate::lexer::JavaLexer;
use crate::{JavaLanguage, JavaSyntaxKind};

use super::codes;
use super::grammar::LookaheadTable;

pub(super) use jolt_syntax::MAX_DEPTH;

/// The Java parser: the shared parser, the lookahead table over its tokens,
/// and the number of type argument lists the consuming parser is inside.
pub(super) struct Parser<'source> {
    pub(super) inner: jolt_syntax::Parser<'source, JavaLanguage>,
    pub(super) table: LookaheadTable,
    pub(super) generic_depth: usize,
}

impl<'source> Parser<'source> {
    pub(super) fn new(source: &'source str) -> Self {
        let inner = jolt_syntax::Parser::new(source, JavaLexer::new(source));
        let table = LookaheadTable::new(&inner);
        Self {
            inner,
            table,
            generic_depth: 0,
        }
    }

    pub(super) fn finish(self) -> ParseOutput {
        debug_assert_eq!(self.generic_depth, 0, "generic depth must unwind at EOF");
        self.inner.finish()
    }

    pub(super) fn at_eof(&self) -> bool {
        self.at(JavaSyntaxKind::Eof)
    }

    pub(super) fn current_text(&self) -> &'source str {
        self.nth_text(0)
    }

    pub(super) fn error_expected(&mut self, message: &str) {
        self.error(codes::EXPECTED, message);
    }

    pub(super) fn error_expected_at(&mut self, n: usize, message: &str) {
        self.error_at(n, codes::EXPECTED, message);
    }

    pub(super) fn error_unexpected(&mut self, message: &str) {
        self.error(codes::UNEXPECTED, message);
    }

    pub(super) fn eat_contextual(&mut self, text: &str) -> bool {
        let at = self.at_contextual(text);
        if at {
            self.bump();
        }
        at
    }

    pub(super) fn at_contextual(&self, text: &str) -> bool {
        self.at(JavaSyntaxKind::Identifier) && self.current_text() == text
    }

    pub(super) fn expect_contextual(&mut self, text: &str, message: &str) {
        if !self.eat_contextual(text) {
            self.error_expected(message);
        }
    }
}

impl<'source> Deref for Parser<'source> {
    type Target = jolt_syntax::Parser<'source, JavaLanguage>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for Parser<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<'source> AsMut<jolt_syntax::Parser<'source, JavaLanguage>> for Parser<'source> {
    fn as_mut(&mut self) -> &mut jolt_syntax::Parser<'source, JavaLanguage> {
        &mut self.inner
    }
}

impl Parser<'_> {
    /// See [`jolt_syntax::Parser::nested`].
    pub(super) fn nested<T>(&mut self, parse: impl FnOnce(&mut Self) -> T) -> Option<T> {
        jolt_syntax::Parser::nested(self, parse)
    }

    /// Runs `guard` against the shared parser; see [`LoopGuard::check`].
    pub(super) fn check(&mut self, guard: &mut LoopGuard) -> bool {
        guard.check(&mut self.inner)
    }
}
