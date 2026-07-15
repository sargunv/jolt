use crate::KotlinSyntaxKind as K;

use super::Parser;

impl Parser<'_> {
    pub(super) fn parse_statement_tail(&mut self) {
        if self.at(K::ValKw) || self.at(K::VarKw) {
            let local = self.start();
            self.parse_property_tail();
            self.complete(local, K::LocalDeclaration);
        } else {
            let expression = self.start();
            if matches!(self.current_kind(), K::Semicolon | K::DoubleSemicolon) {
                self.expected_here("expected expression");
                let malformed = self.start();
                self.bump();
                self.complete(malformed, K::ErrorNode);
            } else {
                self.parse_expression_until(&[K::Semicolon, K::DoubleSemicolon, K::RBrace]);
            }
            self.complete(expression, K::ExpressionStatement);
        }
        let tail = self.start();
        self.eat_semicolon_boundary();
        self.complete(tail, K::TerminatorList);
    }

    pub(super) fn parse_block(&mut self) {
        let marker = self.start();
        self.expect(K::LBrace, "expected block");
        let items = self.start();
        while !matches!(self.current_kind(), K::RBrace | K::Eof) {
            let before = self.position();
            self.parse_declaration_or_statement();
            self.ensure_progress(before, "expected statement");
        }
        self.complete(items, K::BlockItemList);
        self.expect(K::RBrace, "expected '}' after block");
        self.complete(marker, K::Block);
    }
}
