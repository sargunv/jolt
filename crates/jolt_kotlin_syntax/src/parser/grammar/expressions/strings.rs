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
                    self.expect(
                        K::LongTemplateEntryEnd,
                        "expected '}' after string template",
                    );
                    self.complete(long_entry, K::LongStringTemplateEntry);
                }
                _ => self.bump(),
            }
            self.complete(entry, K::StringTemplateEntry);
        }
        self.complete(parts, K::StringTemplateEntryList);
        if self.at(K::ClosingQuote) {
            self.bump();
        }
        self.complete(marker, K::StringTemplateExpression)
    }
}
