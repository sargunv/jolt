// Handles generic type argument closes, consuming one `>` atom at a time.
use super::lookahead::{gt_rest, is_gt_family};
use super::{JavaSyntaxKind, Parser};

impl Parser<'_> {
    pub(in crate::parser::grammar) fn at_type_argument_close(&mut self) -> bool {
        is_gt_family(self.current())
    }

    /// Consumes one `>`, splitting it off a longer `>`-family token.
    pub(in crate::parser::grammar) fn eat_type_argument_close(&mut self) -> bool {
        let kind = self.current();
        if let Some(rest) = gt_rest(kind) {
            self.split_current(JavaSyntaxKind::Gt, rest);
        }
        self.eat(JavaSyntaxKind::Gt)
    }

    pub(in crate::parser::grammar) fn type_arguments_are_followed_by_double_colon(
        &mut self,
    ) -> bool {
        let mut lookahead = self.lookahead();
        lookahead.skip_type_arguments();
        lookahead.at(JavaSyntaxKind::DoubleColon)
    }

    pub(in crate::parser::grammar) fn type_arguments_are_followed_by_dot(&mut self) -> bool {
        let mut lookahead = self.lookahead();
        lookahead.skip_type_arguments();
        lookahead.at(JavaSyntaxKind::Dot)
    }

    pub(in crate::parser::grammar) fn dot_is_followed_by_annotated_name(&mut self) -> bool {
        if !self.at(JavaSyntaxKind::Dot) {
            return false;
        }

        let mut lookahead = self.lookahead();
        lookahead.bump();
        lookahead.skip_annotations();
        lookahead.at_name_segment()
    }
}
