use jolt_syntax::{CompletedMarker, UnresolvedDiagnosticOwner};

use crate::KotlinSyntaxKind as K;

use super::super::Parser;

impl Parser<'_> {
    pub(super) fn parse_string_template_expression(&mut self) -> CompletedMarker {
        let marker = self.start();
        let parts = self.start();
        while !matches!(
            self.current_kind(),
            K::ClosingQuote | K::Semicolon | K::DoubleSemicolon | K::RBrace | K::Eof
        ) {
            let entry = self.start();
            let content = self.start();
            match self.current_kind() {
                K::LongTemplateEntryStart => {
                    let long_entry = self.start();
                    self.bump();
                    self.parse_expression_until(&[K::LongTemplateEntryEnd]);
                    if !self.eat(K::LongTemplateEntryEnd) {
                        let diagnostic = self.expected_here("expected '}' after string template");
                        self.own_diagnostic(
                            diagnostic,
                            UnresolvedDiagnosticOwner::missing_slot(
                                long_entry.anchor(),
                                crate::shape::long_string_template_entry::Slot::close as u16,
                            ),
                        );
                    }
                    self.complete(long_entry, K::LongStringTemplateEntry);
                }
                _ => self.bump(),
            }
            self.complete(content, K::StringTemplateContent);
            self.complete(entry, K::StringTemplateEntry);
        }
        self.complete(parts, K::StringTemplateEntryList);
        if self.at(K::ClosingQuote) {
            self.bump();
        } else {
            let diagnostic = self.expected_here("expected closing quote");
            self.own_diagnostic(
                diagnostic,
                UnresolvedDiagnosticOwner::missing_slot(
                    marker.anchor(),
                    crate::shape::string_template_expression::Slot::close_quote as u16,
                ),
            );
        }
        self.complete(marker, K::StringTemplateExpression)
    }
}
