//! Token checks and modifier consumption that need no grammar scan.

use super::{JavaSyntaxKind, Parser, TypeModifier};

impl Parser<'_> {
    /// Consumes the type modifier at the current token, wrapping `non-sealed`
    /// in a node.
    pub(in crate::parser::grammar) fn bump_type_modifier(&mut self) {
        if let Some(TypeModifier::NonSealed) = self.lookahead().type_modifier() {
            let modifier = self.start();
            for _ in 0..TypeModifier::NonSealed.len() {
                self.bump();
            }
            self.complete(modifier, JavaSyntaxKind::NonSealedModifier);
        } else {
            self.bump();
        }
    }

    pub(in crate::parser::grammar) fn starts_array_dimensions(&self) -> bool {
        let mut lookahead = self.lookahead();
        lookahead.skip_annotations();
        lookahead.at(JavaSyntaxKind::LBracket) && lookahead.nth(1) == JavaSyntaxKind::RBracket
    }

    pub(in crate::parser::grammar) fn starts_dim_expression(&self) -> bool {
        let mut lookahead = self.lookahead();
        lookahead.skip_annotations();
        lookahead.at(JavaSyntaxKind::LBracket) && lookahead.nth(1) != JavaSyntaxKind::RBracket
    }
}
