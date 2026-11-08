//! Expression-level grammar questions, answered before the parser commits to
//! a branch.

use super::{JavaSyntaxKind, Parser};
use crate::{binary_precedence, is_assignment_operator};

impl Parser<'_> {
    pub(in crate::parser::grammar) fn starts_parenthesized_lambda_expression(&self) -> bool {
        self.at(JavaSyntaxKind::LParen)
            && self.kind_at(self.table.bracket_end(self.position())) == JavaSyntaxKind::Arrow
    }

    pub(in crate::parser::grammar) fn starts_lambda_expression(&self) -> bool {
        self.starts_parenthesized_lambda_expression()
            || (self.lookahead().at_variable_identifier() && self.nth(1) == JavaSyntaxKind::Arrow)
    }

    pub(in crate::parser::grammar) fn at_assignment_operator(&self) -> bool {
        is_assignment_operator(self.current())
    }

    /// The precedence of the binary operator at the current token.
    pub(in crate::parser::grammar) fn binary_operator(&self) -> Option<u8> {
        binary_precedence(self.current())
    }

    /// Whether the tokens at `index` and `index + 1` touch, with no trivia
    /// between them.
    pub(in crate::parser::grammar) fn tokens_touch(&self, index: usize) -> bool {
        self.range_at(index).end() == self.range_at(index + 1).start()
    }

    pub(in crate::parser::grammar) fn starts_cast_expression(&self) -> bool {
        if self.current() != JavaSyntaxKind::LParen {
            return false;
        }

        let mut lookahead = self.lookahead();
        lookahead.eat(JavaSyntaxKind::LParen);
        lookahead.skip_annotations();
        let is_primitive_cast =
            lookahead.at_primitive_type_start() && lookahead.nth(1) == JavaSyntaxKind::RParen;
        if !lookahead.skip_cast_type()
            || !lookahead.at(JavaSyntaxKind::RParen)
            || lookahead.nth(1) == JavaSyntaxKind::Arrow
        {
            return false;
        }
        lookahead.bump();

        if is_primitive_cast {
            lookahead.starts_expression()
        } else {
            lookahead.starts_expression_not_plus_minus()
        }
    }

    pub(in crate::parser::grammar) fn starts_typed_lambda_parameter(&self) -> bool {
        let mut lookahead = self.lookahead();
        if lookahead.at_contextual("var") && lookahead.nth(1) != JavaSyntaxKind::Dot {
            lookahead.bump();
            return lookahead.at_variable_identifier();
        }

        if !lookahead.at_type_start() {
            return false;
        }

        lookahead.skip_type();
        lookahead.skip_annotations();
        lookahead.eat(JavaSyntaxKind::Ellipsis);

        lookahead.at_variable_identifier()
    }

    pub(in crate::parser::grammar) fn new_expression_is_array_creation(&self) -> bool {
        if self.current() != JavaSyntaxKind::NewKw {
            return false;
        }

        let mut lookahead = self.lookahead();
        lookahead.bump();
        if lookahead.at(JavaSyntaxKind::Lt) {
            lookahead.skip_type_arguments();
        }

        lookahead.skip_type_base();
        lookahead.skip_annotations();
        lookahead.at(JavaSyntaxKind::LBracket)
    }
}
