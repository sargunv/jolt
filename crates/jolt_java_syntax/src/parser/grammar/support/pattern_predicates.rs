// Answers pattern grammar questions before the parser commits to a branch.
use super::{JavaSyntaxKind, Parser};

impl Parser<'_> {
    pub(in crate::parser::grammar) fn starts_case_type_pattern(&mut self) -> bool {
        let mut lookahead = self.lookahead();
        lookahead.skip_variable_modifiers();
        if !lookahead.at_non_void_type_start() || lookahead.starts_literal_expression() {
            return false;
        }

        lookahead.skip_type();
        lookahead.at_variable_identifier()
    }

    pub(in crate::parser::grammar) fn starts_pattern(&mut self) -> bool {
        self.starts_case_type_pattern() || self.starts_record_pattern()
    }

    pub(in crate::parser::grammar) fn starts_record_pattern(&mut self) -> bool {
        let mut lookahead = self.lookahead();
        lookahead.skip_variable_modifiers();
        if !lookahead.at_non_void_type_start() || lookahead.starts_literal_expression() {
            return false;
        }

        lookahead.skip_type();
        lookahead.at(JavaSyntaxKind::LParen)
    }
}
