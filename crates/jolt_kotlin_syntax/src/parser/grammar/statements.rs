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
            self.parse_expression_until(&[K::Semicolon, K::DoubleSemicolon, K::RBrace]);
            self.complete(expression, K::ExpressionStatement);
        }
        self.eat_semicolon_boundary();
    }

    pub(super) fn parse_block(&mut self) {
        let marker = self.start();
        self.expect(K::LBrace, "expected block");
        while !matches!(self.current_kind(), K::RBrace | K::Eof) {
            self.parse_declaration_or_statement();
        }
        self.expect(K::RBrace, "expected '}' after block");
        self.complete(marker, K::Block);
    }
}
