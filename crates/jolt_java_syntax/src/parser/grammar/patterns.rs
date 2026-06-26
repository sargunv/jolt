use super::{JavaSyntaxKind, Parser};

impl Parser<'_> {
    pub(super) fn parse_pattern_until(&mut self, stops: &[JavaSyntaxKind]) {
        if self.starts_record_pattern() {
            self.parse_record_pattern();
        } else {
            self.parse_type_pattern_until(stops);
        }
    }

    pub(super) fn parse_type_pattern_until(&mut self, stops: &[JavaSyntaxKind]) {
        let pattern = self.start();
        self.parse_local_variable_declaration_until(stops);
        self.complete(pattern, JavaSyntaxKind::TypePattern);
    }

    pub(super) fn parse_record_pattern(&mut self) {
        let pattern = self.start();
        self.parse_type();
        self.expect(JavaSyntaxKind::LParen, "expected `(` in record pattern");
        while !self.at_eof() && !self.at(JavaSyntaxKind::RParen) {
            self.parse_component_pattern();
            if !self.eat(JavaSyntaxKind::Comma) {
                break;
            }
        }
        self.expect(JavaSyntaxKind::RParen, "expected `)` after record pattern");
        self.complete(pattern, JavaSyntaxKind::RecordPattern);
    }

    pub(super) fn parse_component_pattern(&mut self) {
        let component = self.start();
        if self.at(JavaSyntaxKind::UnderscoreKw) {
            let match_all = self.start();
            self.bump();
            self.complete(match_all, JavaSyntaxKind::MatchAllPattern);
        } else if self.starts_record_pattern() {
            self.parse_record_pattern();
        } else {
            self.parse_type_pattern_until(&[JavaSyntaxKind::Comma, JavaSyntaxKind::RParen]);
        }
        self.complete(component, JavaSyntaxKind::ComponentPattern);
    }
}
