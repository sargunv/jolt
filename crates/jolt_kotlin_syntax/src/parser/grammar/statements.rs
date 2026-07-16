use crate::KotlinSyntaxKind as K;

use super::Parser;

impl Parser<'_> {
    pub(super) fn parse_statement_tail(&mut self) {
        if self.at(K::ValKw) || self.at(K::VarKw) {
            let local = self.start();
            self.parse_property_tail();
            self.complete(local, K::LocalDeclaration);
        } else if matches!(self.current_kind(), K::Semicolon | K::DoubleSemicolon) {
            let empty = self.start();
            self.bump();
            self.complete(empty, K::EmptyStatement);
        } else {
            let expression = self.start();
            self.parse_expression_until(&[K::Semicolon, K::DoubleSemicolon, K::RBrace]);
            self.complete(expression, K::ExpressionStatement);
        }
        let tail = self.start();
        self.eat_semicolon_boundary();
        self.complete(tail, K::TerminatorList);
    }

    pub(super) fn parse_block(&mut self) {
        let marker = self.start();
        if !self.eat(K::LBrace) {
            let diagnostic = self.pending_expected("expected block");
            self.missing_required_slot(
                marker.anchor(),
                crate::shape::block::Slot::open_brace as u16,
                [diagnostic],
            );
        }
        let items = self.start();
        while !matches!(self.current_kind(), K::RBrace | K::Eof) {
            let before = self.position();
            self.parse_declaration_or_statement();
            debug_assert!(self.position() > before);
        }
        self.complete(items, K::BlockItemList);
        if !self.eat(K::RBrace) {
            let diagnostic = self.pending_expected("expected '}' after block");
            self.missing_required_slot(
                marker.anchor(),
                crate::shape::block::Slot::close_brace as u16,
                [diagnostic],
            );
        }
        self.complete(marker, K::Block);
    }

    pub(in crate::parser::grammar) fn complete_missing_block(&mut self, message: &'static str) {
        let block = self.start();
        let diagnostic = self.pending_expected(message);
        self.complete_recovery(block, K::Block, [diagnostic]);
    }
}
