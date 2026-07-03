// Provides a markerless grammar scanner over the same logical tokens as the parser.
use super::{JavaSyntaxKind, Parser};
use crate::parser::source::TokenCursor;

impl Parser<'_> {
    pub(in crate::parser::grammar) fn lookahead(&self) -> JavaLookahead<'_> {
        JavaLookahead::new(self.fork_cursor())
    }
}

pub(in crate::parser::grammar) struct JavaLookahead<'source> {
    cursor: TokenCursor<'source>,
}

impl<'source> JavaLookahead<'source> {
    fn new(cursor: TokenCursor<'source>) -> Self {
        Self { cursor }
    }

    pub(in crate::parser::grammar) fn kind(&self) -> JavaSyntaxKind {
        self.cursor.kind()
    }

    pub(in crate::parser::grammar) fn nth_kind(&self, n: usize) -> JavaSyntaxKind {
        self.cursor.nth_kind(n)
    }

    pub(in crate::parser::grammar) fn text(&self) -> Option<&'source str> {
        self.cursor.text()
    }

    pub(in crate::parser::grammar) fn at(&self, kind: JavaSyntaxKind) -> bool {
        self.kind() == kind
    }

    pub(in crate::parser::grammar) fn at_contextual(&self, text: &str) -> bool {
        self.at(JavaSyntaxKind::Identifier) && self.text() == Some(text)
    }

    pub(in crate::parser::grammar) fn at_eof(&self) -> bool {
        self.at(JavaSyntaxKind::Eof)
    }

    pub(in crate::parser::grammar) fn bump(&mut self) {
        self.cursor.advance();
    }

    pub(in crate::parser::grammar) fn eat(&mut self, kind: JavaSyntaxKind) -> bool {
        if self.at(kind) {
            self.bump();
            true
        } else {
            false
        }
    }

    pub(in crate::parser::grammar) fn skip_annotation(&mut self) -> bool {
        if !self.at(JavaSyntaxKind::At) || self.nth_kind(1) == JavaSyntaxKind::InterfaceKw {
            return false;
        }

        self.bump();
        if !self.at_name_segment() {
            return true;
        }

        self.bump();
        while self.at(JavaSyntaxKind::Dot) && self.nth_kind(1) == JavaSyntaxKind::Identifier {
            self.bump();
            self.bump();
        }

        if self.at(JavaSyntaxKind::LParen) {
            self.skip_balanced(JavaSyntaxKind::LParen, JavaSyntaxKind::RParen);
        }

        true
    }

    pub(in crate::parser::grammar) fn skip_annotations(&mut self) -> bool {
        let start = self.cursor.checkpoint();
        while self.skip_annotation() {}
        self.cursor.checkpoint() != start
    }

    pub(in crate::parser::grammar) fn skip_type_modifiers(&mut self) {
        loop {
            if self.skip_annotation() {
                continue;
            }

            if self.at_type_modifier() {
                self.skip_type_modifier();
                continue;
            }

            return;
        }
    }

    pub(in crate::parser::grammar) fn skip_variable_modifiers(&mut self) {
        loop {
            if self.skip_annotation() {
                continue;
            }

            if self.eat(JavaSyntaxKind::FinalKw) {
                continue;
            }

            return;
        }
    }

    pub(in crate::parser::grammar) fn skip_balanced(
        &mut self,
        open: JavaSyntaxKind,
        close: JavaSyntaxKind,
    ) {
        let mut depth = 0usize;
        while !self.at_eof() {
            if self.at(open) {
                depth += 1;
            } else if self.at(close) {
                depth = depth.saturating_sub(1);
                self.bump();
                if depth == 0 {
                    return;
                }
                continue;
            }
            self.bump();
        }
    }

    pub(in crate::parser::grammar) fn skip_type(&mut self) -> bool {
        if !self.skip_type_base() {
            return false;
        }

        loop {
            self.skip_annotations();
            if self.at(JavaSyntaxKind::LBracket) && self.nth_kind(1) == JavaSyntaxKind::RBracket {
                self.bump();
                self.bump();
            } else {
                return true;
            }
        }
    }

    pub(in crate::parser::grammar) fn skip_type_base(&mut self) -> bool {
        self.skip_annotations();

        if self.at_primitive_type_start() {
            self.bump();
            return true;
        }

        if !self.at_name_segment() {
            return false;
        }

        self.bump();
        self.skip_type_arguments();

        while self.at(JavaSyntaxKind::Dot) {
            let checkpoint = self.cursor.checkpoint();
            self.bump();
            self.skip_annotations();
            if !self.at_name_segment() {
                self.cursor.rewind(checkpoint);
                break;
            }

            self.bump();
            self.skip_type_arguments();
        }

        true
    }

    pub(in crate::parser::grammar) fn skip_cast_type(&mut self) -> bool {
        self.skip_annotations();
        if !self.at_type_start() {
            return false;
        }

        if !self.skip_type() {
            return false;
        }

        loop {
            let checkpoint = self.cursor.checkpoint();
            if !self.eat(JavaSyntaxKind::Amp) {
                return true;
            }

            if !self.at_type_start() || !self.skip_type() {
                self.cursor.rewind(checkpoint);
                return true;
            }
        }
    }

    pub(in crate::parser::grammar) fn skip_type_arguments(&mut self) -> bool {
        if !self.eat(JavaSyntaxKind::Lt) {
            return false;
        }

        while !self.at_eof() && !self.at_type_argument_close() {
            if !self.skip_type_argument() {
                self.bump();
            }

            if !self.eat(JavaSyntaxKind::Comma) {
                break;
            }
        }

        self.eat_type_argument_close();
        true
    }

    pub(in crate::parser::grammar) fn skip_type_parameters(&mut self) -> bool {
        if !self.eat(JavaSyntaxKind::Lt) {
            return false;
        }

        while !self.at_eof() && !self.at_type_argument_close() {
            self.skip_annotations();
            self.bump();

            if self.eat(JavaSyntaxKind::ExtendsKw) {
                self.skip_type();
                while self.eat(JavaSyntaxKind::Amp) {
                    self.skip_type();
                }
            }

            if !self.eat(JavaSyntaxKind::Comma) {
                break;
            }
        }

        self.eat_type_argument_close();
        true
    }

    fn skip_type_argument(&mut self) -> bool {
        self.skip_annotations();
        if self.eat(JavaSyntaxKind::Question) {
            if self.eat(JavaSyntaxKind::ExtendsKw) || self.eat(JavaSyntaxKind::SuperKw) {
                self.skip_type();
            }
            true
        } else {
            self.skip_type()
        }
    }

    pub(in crate::parser::grammar) fn at_type_argument_close(&self) -> bool {
        matches!(
            self.kind(),
            JavaSyntaxKind::Gt | JavaSyntaxKind::RShift | JavaSyntaxKind::UnsignedRShift
        )
    }

    pub(in crate::parser::grammar) fn eat_type_argument_close(&mut self) -> bool {
        self.cursor.bump_split_gt().is_some()
    }

    pub(in crate::parser::grammar) fn at_name_segment(&self) -> bool {
        self.kind() == JavaSyntaxKind::Identifier
    }

    pub(in crate::parser::grammar) fn at_variable_identifier(&self) -> bool {
        matches!(
            self.kind(),
            JavaSyntaxKind::Identifier | JavaSyntaxKind::UnderscoreKw
        )
    }

    pub(in crate::parser::grammar) fn at_type_start(&self) -> bool {
        self.at_non_void_type_start() || self.at(JavaSyntaxKind::VoidKw)
    }

    pub(in crate::parser::grammar) fn at_non_void_type_start(&self) -> bool {
        self.at_name_segment() || self.at_primitive_type_start()
    }

    pub(in crate::parser::grammar) fn at_primitive_type_start(&self) -> bool {
        matches!(
            self.kind(),
            JavaSyntaxKind::BooleanKw
                | JavaSyntaxKind::ByteKw
                | JavaSyntaxKind::CharKw
                | JavaSyntaxKind::DoubleKw
                | JavaSyntaxKind::FloatKw
                | JavaSyntaxKind::IntKw
                | JavaSyntaxKind::LongKw
                | JavaSyntaxKind::ShortKw
        )
    }

    pub(in crate::parser::grammar) fn at_type_modifier(&self) -> bool {
        matches!(
            self.kind(),
            JavaSyntaxKind::PublicKw
                | JavaSyntaxKind::ProtectedKw
                | JavaSyntaxKind::PrivateKw
                | JavaSyntaxKind::AbstractKw
                | JavaSyntaxKind::StaticKw
                | JavaSyntaxKind::FinalKw
                | JavaSyntaxKind::TransientKw
                | JavaSyntaxKind::VolatileKw
                | JavaSyntaxKind::SynchronizedKw
                | JavaSyntaxKind::NativeKw
                | JavaSyntaxKind::StrictfpKw
                | JavaSyntaxKind::DefaultKw
        ) || self.at_contextual("sealed")
            || (self.at_contextual("non")
                && self.nth_kind(1) == JavaSyntaxKind::Minus
                && self.cursor.text_at(self.cursor.position() + 2) == Some("sealed"))
    }

    fn skip_type_modifier(&mut self) {
        if self.at_contextual("non")
            && self.nth_kind(1) == JavaSyntaxKind::Minus
            && self.cursor.text_at(self.cursor.position() + 2) == Some("sealed")
        {
            self.bump();
            self.bump();
            self.bump();
        } else {
            self.bump();
        }
    }

    pub(in crate::parser::grammar) fn starts_expression(&self) -> bool {
        matches!(
            self.kind(),
            JavaSyntaxKind::Identifier
                | JavaSyntaxKind::UnderscoreKw
                | JavaSyntaxKind::IntegerLiteral
                | JavaSyntaxKind::FloatingPointLiteral
                | JavaSyntaxKind::BooleanLiteral
                | JavaSyntaxKind::CharacterLiteral
                | JavaSyntaxKind::StringLiteral
                | JavaSyntaxKind::TextBlockLiteral
                | JavaSyntaxKind::NullLiteral
                | JavaSyntaxKind::ThisKw
                | JavaSyntaxKind::SuperKw
                | JavaSyntaxKind::SwitchKw
                | JavaSyntaxKind::NewKw
                | JavaSyntaxKind::LParen
                | JavaSyntaxKind::PlusPlus
                | JavaSyntaxKind::MinusMinus
                | JavaSyntaxKind::Plus
                | JavaSyntaxKind::Minus
                | JavaSyntaxKind::Bang
                | JavaSyntaxKind::Tilde
        ) || self.starts_primitive_or_void_class_literal()
    }

    pub(in crate::parser::grammar) fn starts_expression_not_plus_minus(&self) -> bool {
        self.starts_expression()
            && !matches!(self.kind(), JavaSyntaxKind::Plus | JavaSyntaxKind::Minus)
    }

    pub(in crate::parser::grammar) fn starts_literal_expression(&self) -> bool {
        matches!(
            self.kind(),
            JavaSyntaxKind::IntegerLiteral
                | JavaSyntaxKind::FloatingPointLiteral
                | JavaSyntaxKind::BooleanLiteral
                | JavaSyntaxKind::CharacterLiteral
                | JavaSyntaxKind::StringLiteral
                | JavaSyntaxKind::TextBlockLiteral
                | JavaSyntaxKind::NullLiteral
        )
    }

    fn starts_primitive_or_void_class_literal(&self) -> bool {
        if !matches!(
            self.kind(),
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

        let mut cursor = self.cursor.fork();
        cursor.bump();
        while cursor.kind() == JavaSyntaxKind::LBracket
            && cursor.nth_kind(1) == JavaSyntaxKind::RBracket
        {
            cursor.bump();
            cursor.bump();
        }

        cursor.kind() == JavaSyntaxKind::Dot && cursor.nth_kind(1) == JavaSyntaxKind::ClassKw
    }
}
