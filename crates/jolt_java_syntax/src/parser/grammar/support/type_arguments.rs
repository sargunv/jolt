//! Generic type argument closes, consumed one `>` at a time.

use super::{JavaSyntaxKind, Parser, gt_rest};

impl Parser<'_> {
    pub(in crate::parser::grammar) fn at_type_argument_close(&self) -> bool {
        self.lookahead().at_type_argument_close()
    }

    /// Consumes one `>`, splitting it off a longer `>`-family token.
    pub(in crate::parser::grammar) fn eat_type_argument_close(&mut self) -> bool {
        if let Some(rest) = gt_rest(self.current()) {
            self.split_current(JavaSyntaxKind::Gt, rest);
        }
        self.eat(JavaSyntaxKind::Gt)
    }

    pub(in crate::parser::grammar) fn type_arguments_are_followed_by_double_colon(&self) -> bool {
        let mut lookahead = self.lookahead();
        lookahead.skip_type_arguments();
        lookahead.at(JavaSyntaxKind::DoubleColon)
    }

    pub(in crate::parser::grammar) fn type_arguments_are_followed_by_dot(&self) -> bool {
        let mut lookahead = self.lookahead();
        lookahead.skip_type_arguments();
        lookahead.at(JavaSyntaxKind::Dot)
    }

    pub(in crate::parser::grammar) fn dot_is_followed_by_annotated_name(&self) -> bool {
        if !self.at(JavaSyntaxKind::Dot) {
            return false;
        }

        let mut lookahead = self.lookahead();
        lookahead.bump();
        lookahead.skip_annotations();
        lookahead.at_name_segment()
    }
}
