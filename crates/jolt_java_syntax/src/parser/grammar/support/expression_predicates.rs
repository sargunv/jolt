// Answers expression-level grammar questions before the parser commits to a branch.
use super::{JavaSyntaxKind, Parser};
use crate::nodes::{
    COMPOSITE_BINARY_OPERATORS, assignment_operator_kind, binary_operator_kind,
    binary_operator_precedence as java_binary_operator_precedence,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::parser::grammar) struct ParserOperator {
    pub(in crate::parser::grammar) precedence: u8,
    pub(in crate::parser::grammar) len: usize,
}

impl Parser<'_> {
    pub(in crate::parser::grammar) fn starts_parenthesized_lambda_expression(&mut self) -> bool {
        if self.current_kind() != JavaSyntaxKind::LParen {
            return false;
        }

        let after_parameters = self.skip_balanced_from(
            self.position(),
            JavaSyntaxKind::LParen,
            JavaSyntaxKind::RParen,
        );
        self.kind_at(after_parameters) == JavaSyntaxKind::Arrow
    }

    pub(in crate::parser::grammar) fn starts_lambda_expression(&mut self) -> bool {
        self.starts_parenthesized_lambda_expression()
            || ((self.current_kind() == JavaSyntaxKind::Identifier
                || self.current_kind() == JavaSyntaxKind::UnderscoreKw)
                && self.nth_kind(1) == JavaSyntaxKind::Arrow)
    }

    pub(in crate::parser::grammar) fn assignment_operator_len(&mut self) -> Option<usize> {
        self.assignment_operator_len_at(self.position())
    }

    pub(in crate::parser::grammar) fn assignment_operator_len_at(
        &mut self,
        index: usize,
    ) -> Option<usize> {
        for pattern in crate::nodes::COMPOSITE_ASSIGNMENT_OPERATORS {
            if self.matches_adjacent_kinds(index, pattern.tokens) {
                return Some(pattern.tokens.len());
            }
        }

        assignment_operator_kind(self.kind_at(index)).map(|_| 1)
    }

    pub(in crate::parser::grammar) fn binary_operator(&mut self) -> Option<ParserOperator> {
        self.binary_operator_at(self.position())
    }

    fn binary_operator_at(&mut self, index: usize) -> Option<ParserOperator> {
        if self.matches_adjacent_kinds(index, COMPOSITE_BINARY_OPERATORS[0].tokens) {
            return Some(ParserOperator {
                precedence: 7,
                len: COMPOSITE_BINARY_OPERATORS[0].tokens.len(),
            });
        }

        if self.assignment_operator_len_at(index).is_some() {
            return None;
        }

        for pattern in &COMPOSITE_BINARY_OPERATORS[1..] {
            if self.matches_adjacent_kinds(index, pattern.tokens) {
                return Some(ParserOperator {
                    precedence: java_binary_operator_precedence(pattern.kind)
                        .expect("composite binary operator must have precedence"),
                    len: pattern.tokens.len(),
                });
            }
        }

        let operator = binary_operator_kind(self.kind_at(index))?;
        Some(ParserOperator {
            precedence: java_binary_operator_precedence(operator)?,
            len: 1,
        })
    }

    fn matches_adjacent_kinds(&mut self, index: usize, kinds: &[JavaSyntaxKind]) -> bool {
        kinds
            .iter()
            .enumerate()
            .all(|(offset, kind)| self.kind_at(index + offset) == *kind)
            && self.tokens_are_adjacent(index, kinds.len())
    }

    pub(in crate::parser::grammar) fn starts_cast_expression(&mut self) -> bool {
        if self.current_kind() != JavaSyntaxKind::LParen
            || self.starts_parenthesized_lambda_expression()
        {
            return false;
        }

        let mut lookahead = self.lookahead();
        lookahead.eat(JavaSyntaxKind::LParen);
        lookahead.skip_annotations();
        let is_primitive_cast =
            lookahead.at_primitive_type_start() && lookahead.nth_kind(1) == JavaSyntaxKind::RParen;
        if !lookahead.skip_cast_type()
            || !lookahead.at(JavaSyntaxKind::RParen)
            || lookahead.nth_kind(1) == JavaSyntaxKind::Arrow
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

    pub(in crate::parser::grammar) fn starts_primitive_or_void_class_literal(&mut self) -> bool {
        self.starts_primitive_or_void_class_literal_at(self.position())
    }

    pub(in crate::parser::grammar) fn starts_primitive_or_void_class_literal_at(
        &mut self,
        mut index: usize,
    ) -> bool {
        if !matches!(
            self.kind_at(index),
            JavaSyntaxKind::BooleanKw
                | JavaSyntaxKind::ByteKw
                | JavaSyntaxKind::CharKw
                | JavaSyntaxKind::DoubleKw
                | JavaSyntaxKind::FloatKw
                | JavaSyntaxKind::IntKw
                | JavaSyntaxKind::LongKw
                | JavaSyntaxKind::ShortKw
                | JavaSyntaxKind::VoidKw
        ) {
            return false;
        }

        let is_void = self.kind_at(index) == JavaSyntaxKind::VoidKw;
        index += 1;
        if is_void {
            return self.kind_at(index) == JavaSyntaxKind::Dot
                && self.kind_at(index + 1) == JavaSyntaxKind::ClassKw;
        }
        while self.kind_at(index) == JavaSyntaxKind::LBracket
            && self.kind_at(index + 1) == JavaSyntaxKind::RBracket
        {
            index += 2;
        }

        self.kind_at(index) == JavaSyntaxKind::Dot
            && self.kind_at(index + 1) == JavaSyntaxKind::ClassKw
    }

    pub(in crate::parser::grammar) fn starts_typed_lambda_parameter(&mut self) -> bool {
        if self.text_at(self.position()) == Some("var") && self.nth_kind(1) != JavaSyntaxKind::Dot {
            return self.is_variable_identifier_at_offset(self.position() + 1);
        }

        let mut lookahead = self.lookahead();
        if !lookahead.at_type_start() {
            return false;
        }

        lookahead.skip_type();
        lookahead.skip_annotations();
        lookahead.eat(JavaSyntaxKind::Ellipsis);

        lookahead.at_variable_identifier()
    }

    pub(in crate::parser::grammar) fn starts_literal_expression(&mut self) -> bool {
        self.starts_literal_expression_at(self.position())
    }

    pub(in crate::parser::grammar) fn starts_literal_expression_at(
        &mut self,
        index: usize,
    ) -> bool {
        matches!(
            self.kind_at(index),
            JavaSyntaxKind::IntegerLiteral
                | JavaSyntaxKind::FloatingPointLiteral
                | JavaSyntaxKind::BooleanLiteral
                | JavaSyntaxKind::CharacterLiteral
                | JavaSyntaxKind::StringLiteral
                | JavaSyntaxKind::TextBlockLiteral
                | JavaSyntaxKind::NullLiteral
        )
    }

    pub(in crate::parser::grammar) fn new_expression_is_array_creation(&mut self) -> bool {
        if self.current_kind() != JavaSyntaxKind::NewKw {
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
