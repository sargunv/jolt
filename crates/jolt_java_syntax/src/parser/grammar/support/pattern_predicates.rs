// Answers pattern grammar questions before the parser commits to a branch.
use super::{JavaSyntaxKind, Parser};
use crate::parser::grammar::patterns::PatternStart;

impl Parser<'_> {
    pub(in crate::parser::grammar) fn pattern_start(&mut self) -> Option<PatternStart> {
        let mut lookahead = self.lookahead();
        lookahead.skip_variable_modifiers();
        if !lookahead.at_non_void_type_start() || lookahead.starts_literal_expression() {
            return None;
        }

        lookahead.skip_type();
        if lookahead.at_variable_identifier() {
            Some(PatternStart::Type)
        } else if lookahead.at(JavaSyntaxKind::LParen) {
            Some(PatternStart::Record {
                open_paren: lookahead.position(),
            })
        } else {
            None
        }
    }
}
