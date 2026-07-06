use jolt_syntax::CompletedMarker;

use crate::KotlinSyntaxKind as K;

use super::super::Parser;

impl Parser<'_> {
    pub(super) fn parse_string_template_expression(&mut self) -> CompletedMarker {
        let marker = self.start();
        while !matches!(
            self.current_kind(),
            K::ClosingQuote | K::Semicolon | K::DoubleSemicolon | K::RBrace | K::Eof
        ) {
            let entry = self.start();
            match self.current_kind() {
                K::LongTemplateEntryStart => {
                    self.bump();
                    self.parse_expression_until(&[K::LongTemplateEntryEnd]);
                    self.expect(
                        K::LongTemplateEntryEnd,
                        "expected '}' after string template",
                    );
                }
                _ => self.bump(),
            }
            self.complete(entry, K::StringTemplateEntry);
        }
        if self.at(K::ClosingQuote) {
            self.bump();
        }
        self.complete(marker, K::StringTemplateExpression)
    }
}
