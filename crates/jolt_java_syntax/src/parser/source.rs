use std::ops::{Deref, DerefMut};

use jolt_syntax::{CompletedMarker, DiagnosticMarker, NodeAnchor};

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

    pub(super) const fn position(&self) -> usize {
        self.0.position()
    }

    pub(super) fn kind_at(&mut self, index: usize) -> JavaSyntaxKind {
        self.0.kind_at(index)
    }

    pub(super) fn text_at(&mut self, index: usize) -> Option<&'source str> {
        self.0.text_at(index)
    }

    pub(super) fn tokens_are_adjacent(&mut self, index: usize, count: usize) -> bool {
        self.0.tokens_are_adjacent(index, count)
    }

    pub(super) fn completed_is_error_node(marker: &CompletedMarker) -> bool {
        jolt_syntax::Parser::<JavaLanguage>::completed_is_error_node(marker)
    }

    pub(super) fn expect_contextual_owned(
        &mut self,
        text: &str,
        message: &str,
        owner: NodeAnchor,
        slot: u16,
    ) {
        if !self.eat_contextual(text) {
            self.expected_owned_slot(message, owner, slot);
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
    fn expect_contextual(&mut self, text: &str, message: &str);
    fn eat_contextual(&mut self, text: &str) -> bool;
    fn at_contextual(&mut self, text: &str) -> bool;
    fn invalid_statement_expression_here(&mut self, message: &str);
    fn invalid_resource_variable_access_here(&mut self, message: &str);
    fn invalid_switch_guard_here(&mut self, message: &str);
    fn unqualified_yield_method_invocation_here(&mut self, message: &str);
    fn decimal_integer_boundary_literal_here(&mut self, message: &str);
    fn misplaced_receiver_parameter_here(&mut self, message: &str) -> DiagnosticMarker;
    fn misplaced_constructor_invocation_here(&mut self, message: &str) -> DiagnosticMarker;
    fn restricted_type_identifier_here(&mut self, message: &str) -> DiagnosticMarker;
}

impl JavaParserExt for Parser<'_> {
    fn expect_contextual(&mut self, text: &str, message: &str) {
        if !self.eat_contextual(text) {
            self.expected_here(message);
        }
    }

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

    fn invalid_statement_expression_here(&mut self, message: &str) {
        self.error_here(
            JavaParseDiagnosticCode::InvalidStatementExpression.id(),
            message,
        );
    }

    fn invalid_resource_variable_access_here(&mut self, message: &str) {
        self.error_here(
            JavaParseDiagnosticCode::InvalidResourceVariableAccess.id(),
            message,
        );
    }

    fn invalid_switch_guard_here(&mut self, message: &str) {
        self.error_here(JavaParseDiagnosticCode::InvalidSwitchGuard.id(), message);
    }

    fn unqualified_yield_method_invocation_here(&mut self, message: &str) {
        self.error_here(
            JavaParseDiagnosticCode::UnqualifiedYieldMethodInvocation.id(),
            message,
        );
    }

    fn decimal_integer_boundary_literal_here(&mut self, message: &str) {
        self.error_here(
            JavaParseDiagnosticCode::DecimalIntegerBoundaryLiteral.id(),
            message,
        );
    }

    fn misplaced_receiver_parameter_here(&mut self, message: &str) -> DiagnosticMarker {
        self.error_here(
            JavaParseDiagnosticCode::MisplacedReceiverParameter.id(),
            message,
        )
    }

    fn misplaced_constructor_invocation_here(&mut self, message: &str) -> DiagnosticMarker {
        self.error_here(
            JavaParseDiagnosticCode::MisplacedConstructorInvocation.id(),
            message,
        )
    }

    fn restricted_type_identifier_here(&mut self, message: &str) -> DiagnosticMarker {
        self.error_here(
            JavaParseDiagnosticCode::RestrictedTypeIdentifier.id(),
            message,
        )
    }
}
