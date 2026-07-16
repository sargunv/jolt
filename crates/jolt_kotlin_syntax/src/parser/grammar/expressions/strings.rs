use jolt_syntax::CompletedMarker;

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
            match self.current_kind() {
                K::LongTemplateEntryStart => {
                    let long_entry = self.start();
                    self.bump();
                    self.parse_expression_until(&[K::LongTemplateEntryEnd]);
                    if !self.eat(K::LongTemplateEntryEnd) {
                        let diagnostic =
                            self.pending_expected("expected '}' after string template");
                        self.missing_required_slot(
                            long_entry.anchor(),
                            crate::shape::long_string_template_entry::Slot::close as u16,
                            [diagnostic],
                        );
                    }
                    self.complete(long_entry, K::LongStringTemplateEntry);
                    self.complete(entry, K::StringTemplateEntry);
                }
                K::InterpolationPrefix
                | K::OpenQuote
                | K::RegularStringPart
                | K::EscapeSequence
                | K::ShortTemplateEntryStart
                | K::LongTemplateEntryEnd
                | K::DanglingNewline
                | K::Identifier
                | K::ThisKw => {
                    self.bump();
                    self.complete(entry, K::StringTemplateEntry);
                }
                _ => {
                    let diagnostic = self.pending_unexpected("unexpected token in string template");

                    self.bump();
                    self.complete_recovery(entry, K::BogusStringTemplatePart, [diagnostic]);
                }
            }
        }
        self.complete(parts, K::StringTemplateEntryList);
        if self.at(K::ClosingQuote) {
            self.bump();
        } else {
            let diagnostic = self.pending_expected("expected closing quote");
            self.missing_required_slot(
                marker.anchor(),
                crate::shape::string_template_expression::Slot::close_quote as u16,
                [diagnostic],
            );
        }
        self.complete(marker, K::StringTemplateExpression)
    }
}
