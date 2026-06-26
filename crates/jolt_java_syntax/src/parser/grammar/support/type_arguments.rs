// Handles generic type argument closes, including split shift tokens like `>>`.
use super::{JavaSyntaxKind, Parser};

impl Parser<'_> {
    pub(in crate::parser::grammar) fn at_type_argument_close(&self) -> bool {
        matches!(
            self.current_kind(),
            JavaSyntaxKind::Gt | JavaSyntaxKind::RShift | JavaSyntaxKind::UnsignedRShift
        )
    }

    pub(in crate::parser::grammar) fn eat_type_argument_close(&mut self) -> bool {
        if self.at_type_argument_close() {
            self.bump_split_gt();
            true
        } else {
            false
        }
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
