use crate::KotlinSyntaxKind as K;

use super::super::Parser;
use super::token_sets::DECLARATION_RECOVERY;

impl Parser<'_> {
    pub(in crate::parser::grammar) fn recover_declaration(&mut self) {
        self.recover_until(DECLARATION_RECOVERY);
    }

    pub(in crate::parser::grammar) fn recover_class_member(&mut self) {
        self.recover_until(DECLARATION_RECOVERY);
    }

    pub(in crate::parser::grammar) fn recover_until(&mut self, stops: &[K]) {
        let marker = self.start();
        while !stops.contains(&self.current_kind()) && !self.at_eof() {
            self.bump();
        }
        self.complete(marker, K::ErrorNode);
    }
}
