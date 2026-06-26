// Answers expression-level grammar questions before the parser commits to a branch.
use super::{JavaSyntaxKind, Parser};

impl Parser<'_> {
    pub(in crate::parser::grammar) fn starts_parenthesized_lambda_expression(&self) -> bool {
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

    pub(in crate::parser::grammar) fn starts_lambda_expression(&self) -> bool {
        self.starts_parenthesized_lambda_expression()
            || ((self.current_kind() == JavaSyntaxKind::Identifier
                || self.current_kind() == JavaSyntaxKind::UnderscoreKw)
                && self.nth_kind(1) == JavaSyntaxKind::Arrow)
    }

    pub(in crate::parser::grammar) fn at_assignment_operator(&self) -> bool {
        matches!(
            self.current_kind(),
            JavaSyntaxKind::Assign
                | JavaSyntaxKind::PlusEq
                | JavaSyntaxKind::MinusEq
                | JavaSyntaxKind::StarEq
                | JavaSyntaxKind::SlashEq
                | JavaSyntaxKind::AmpEq
                | JavaSyntaxKind::BarEq
                | JavaSyntaxKind::CaretEq
                | JavaSyntaxKind::PercentEq
                | JavaSyntaxKind::LShiftEq
                | JavaSyntaxKind::RShiftEq
                | JavaSyntaxKind::UnsignedRShiftEq
        )
    }

    pub(in crate::parser::grammar) fn binary_operator_precedence(&self) -> Option<u8> {
        Some(match self.current_kind() {
            JavaSyntaxKind::OrOr => 1,
            JavaSyntaxKind::AndAnd => 2,
            JavaSyntaxKind::Bar => 3,
            JavaSyntaxKind::Caret => 4,
            JavaSyntaxKind::Amp => 5,
            JavaSyntaxKind::EqEq | JavaSyntaxKind::BangEq => 6,
            JavaSyntaxKind::Lt
            | JavaSyntaxKind::Gt
            | JavaSyntaxKind::LtEq
            | JavaSyntaxKind::GtEq
            | JavaSyntaxKind::InstanceofKw => 7,
            JavaSyntaxKind::LShift | JavaSyntaxKind::RShift | JavaSyntaxKind::UnsignedRShift => 8,
            JavaSyntaxKind::Plus | JavaSyntaxKind::Minus => 9,
            JavaSyntaxKind::Star | JavaSyntaxKind::Slash | JavaSyntaxKind::Percent => 10,
            _ => return None,
        })
    }

    pub(in crate::parser::grammar) fn starts_cast_expression(&self) -> bool {
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

    pub(in crate::parser::grammar) fn starts_primitive_or_void_class_literal(&self) -> bool {
        self.starts_primitive_or_void_class_literal_at(self.position())
    }

    pub(in crate::parser::grammar) fn starts_primitive_or_void_class_literal_at(
        &self,
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

        index += 1;
        while self.kind_at(index) == JavaSyntaxKind::LBracket
            && self.kind_at(index + 1) == JavaSyntaxKind::RBracket
        {
            index += 2;
        }

        self.kind_at(index) == JavaSyntaxKind::Dot
            && self.kind_at(index + 1) == JavaSyntaxKind::ClassKw
    }

    pub(in crate::parser::grammar) fn starts_typed_lambda_parameter(&self) -> bool {
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

    pub(in crate::parser::grammar) fn starts_literal_expression(&self) -> bool {
        self.starts_literal_expression_at(self.position())
    }

    pub(in crate::parser::grammar) fn starts_literal_expression_at(&self, index: usize) -> bool {
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

    pub(in crate::parser::grammar) fn new_expression_is_array_creation(&self) -> bool {
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
