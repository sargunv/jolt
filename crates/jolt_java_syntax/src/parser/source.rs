use std::ops::{Deref, DerefMut};

use jolt_syntax::{NodeAnchor, PendingDiagnostic};

use crate::JavaSyntaxKind;
use crate::language::JavaLanguage;

use super::JavaParseDiagnosticCode;

pub(super) type ParseEvents = jolt_syntax::ParseEvents;
pub(super) type TokenBuffer<'source> = jolt_syntax::TokenBuffer<'source, JavaLanguage>;
pub(super) use jolt_syntax::TokenCursor;

pub(super) struct Parser<'source>(pub jolt_syntax::Parser<'source, JavaLanguage>);

impl<'source> Parser<'source> {
    pub(super) fn new(source: &'source str) -> Self {
        Self(jolt_syntax::Parser::new(source))
    }

    pub(super) fn finish(self) -> ParseEvents {
        self.0.finish()
    }

    pub(super) fn expect_required(
        &mut self,
        kind: JavaSyntaxKind,
        message: &str,
        owner: NodeAnchor,
        slot: u16,
    ) {
        if !self.eat(kind) {
            let diagnostic = self.pending_expected(message);
            self.missing_required_slot(owner, slot, [diagnostic]);
        }
    }

    pub(super) fn expect_contextual_required(
        &mut self,
        text: &str,
        message: &str,
        owner: NodeAnchor,
        slot: u16,
    ) {
        if !self.eat_contextual(text) {
            let diagnostic = self.pending_expected(message);
            self.missing_required_slot(owner, slot, [diagnostic]);
        }
    }
}

impl<'source> Deref for Parser<'source> {
    type Target = jolt_syntax::Parser<'source, JavaLanguage>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Parser<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub(super) trait JavaParserExt {
    fn eat_contextual(&mut self, text: &str) -> bool;
    fn at_contextual(&mut self, text: &str) -> bool;
    fn decimal_integer_boundary_literal_here(&mut self, message: &str);
    fn restricted_type_identifier_here(&mut self, message: &str) -> PendingDiagnostic;
}

impl JavaParserExt for Parser<'_> {
    fn eat_contextual(&mut self, text: &str) -> bool {
        if self.at_contextual(text) {
            self.bump();
            true
        } else {
            false
        }
    }

    fn at_contextual(&mut self, text: &str) -> bool {
        self.current_kind() == JavaSyntaxKind::Identifier && self.current_text() == Some(text)
    }

    fn decimal_integer_boundary_literal_here(&mut self, message: &str) {
        let diagnostic = self.pending_error(
            JavaParseDiagnosticCode::DecimalIntegerBoundaryLiteral.id(),
            message,
        );
        self.report_non_structural(diagnostic);
    }

    fn restricted_type_identifier_here(&mut self, message: &str) -> PendingDiagnostic {
        self.pending_error(
            JavaParseDiagnosticCode::RestrictedTypeIdentifier.id(),
            message,
        )
    }
}
