// Provides a markerless grammar scanner over the same logical tokens as the parser.
use super::{
    JavaSyntaxKind, MissingConstructorHeaderAction, Parser, is_literal_expression_start,
    is_primitive_type_start, is_type_argument_recovery_boundary, is_type_argument_value_start,
    missing_constructor_header_action, over_depth_type_end, type_modifier_len,
};
use crate::parser::source::{
    MAX_RECURSIVE_PARSE_OWNERS, ParenthesisSummary, TokenBuffer, TokenCursor,
};

impl<'source> Parser<'source> {
    pub(in crate::parser::grammar) fn lookahead(&mut self) -> JavaLookahead<'_, 'source> {
        let source = self.inner.source;
        let cursor = self.inner.fork_cursor();
        JavaLookahead::new(
            source,
            &mut self.inner.buffer,
            cursor,
            &mut self.parentheses,
            self.generic_depth,
        )
    }
}

pub(in crate::parser::grammar) struct JavaLookahead<'buffer, 'source> {
    source: &'source str,
    buffer: &'buffer mut TokenBuffer<'source>,
    cursor: TokenCursor,
    base: TokenCursor,
    parentheses: &'buffer mut ParenthesisSummary,
    generic_depth: usize,
}

impl<'buffer, 'source> JavaLookahead<'buffer, 'source> {
    fn new(
        source: &'source str,
        buffer: &'buffer mut TokenBuffer<'source>,
        cursor: TokenCursor,
        parentheses: &'buffer mut ParenthesisSummary,
        generic_depth: usize,
    ) -> Self {
        Self {
            source,
            buffer,
            cursor,
            base: cursor,
            parentheses,
            generic_depth,
        }
    }

    pub(in crate::parser::grammar) fn kind(&mut self) -> JavaSyntaxKind {
        self.cursor.kind(self.buffer)
    }

    pub(in crate::parser::grammar) fn nth_kind(&mut self, n: usize) -> JavaSyntaxKind {
        self.cursor.nth_kind(self.buffer, n)
    }

    pub(in crate::parser::grammar) fn text(&mut self) -> Option<&'source str> {
        self.cursor.text(self.source, self.buffer)
    }

    fn nth_text(&mut self, n: usize) -> Option<&'source str> {
        self.buffer.text_at(self.source, self.cursor.position() + n)
    }

    pub(in crate::parser::grammar) fn at(&mut self, kind: JavaSyntaxKind) -> bool {
        self.kind() == kind
    }

    pub(in crate::parser::grammar) fn at_contextual(&mut self, text: &str) -> bool {
        self.at(JavaSyntaxKind::Identifier) && self.text() == Some(text)
    }

    pub(in crate::parser::grammar) fn at_eof(&mut self) -> bool {
        self.at(JavaSyntaxKind::Eof)
    }

    pub(in crate::parser::grammar) fn bump(&mut self) {
        self.cursor.bump(self.buffer);
    }

    pub(in crate::parser::grammar) fn position(&self) -> usize {
        self.cursor.position()
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
            let after = self.parentheses.after(self.buffer, self.cursor, self.base);
            self.cursor.seek_forward(after);
        }

        true
    }

    pub(in crate::parser::grammar) fn skip_annotations(&mut self) -> bool {
        let start = self.cursor.checkpoint();
        while self.skip_annotation() {}
        self.cursor.checkpoint() != start
    }

    fn skip_bounded_annotation(&mut self) -> bool {
        if !self.at(JavaSyntaxKind::At) || self.nth_kind(1) == JavaSyntaxKind::InterfaceKw {
            return false;
        }

        self.bump();
        if self.at_name_segment() {
            self.bump();
            while self.at(JavaSyntaxKind::Dot) && self.nth_kind(1) == JavaSyntaxKind::Identifier {
                self.bump();
                self.bump();
            }
        }

        if !self.eat(JavaSyntaxKind::LParen) {
            return true;
        }

        let mut paren_depth = 1usize;
        let mut brace_depth = 0usize;
        let mut bracket_depth = 0usize;
        while !self.at_eof() {
            match self.kind() {
                JavaSyntaxKind::LParen => paren_depth += 1,
                JavaSyntaxKind::RParen => {
                    paren_depth -= 1;
                    self.bump();
                    if paren_depth == 0 {
                        return true;
                    }
                    continue;
                }
                JavaSyntaxKind::LBrace => brace_depth += 1,
                JavaSyntaxKind::RBrace if brace_depth > 0 => brace_depth -= 1,
                JavaSyntaxKind::LBracket => bracket_depth += 1,
                JavaSyntaxKind::RBracket if bracket_depth > 0 => bracket_depth -= 1,
                JavaSyntaxKind::Semicolon | JavaSyntaxKind::RBrace
                    if paren_depth == 1 && brace_depth == 0 && bracket_depth == 0 =>
                {
                    return false;
                }
                _ => {}
            }
            self.bump();
        }
        false
    }

    pub(in crate::parser::grammar) fn skip_missing_constructor_throws_clause(&mut self) -> bool {
        loop {
            if matches!(
                self.kind(),
                JavaSyntaxKind::LBrace
                    | JavaSyntaxKind::Semicolon
                    | JavaSyntaxKind::RBrace
                    | JavaSyntaxKind::Eof
            ) {
                return self.at(JavaSyntaxKind::LBrace);
            }
            if self.eat(JavaSyntaxKind::Comma) {
                continue;
            }
            if !self.skip_missing_constructor_throws_type() {
                return false;
            }
            if !self.eat(JavaSyntaxKind::Comma) {
                return self.at(JavaSyntaxKind::LBrace);
            }
        }
    }

    pub(in crate::parser::grammar) fn skip_missing_constructor_throws_type(&mut self) -> bool {
        while self.at(JavaSyntaxKind::At) {
            if !self.skip_bounded_annotation() {
                return false;
            }
        }

        if self.at(JavaSyntaxKind::VoidKw) {
            self.bump();
            loop {
                let checkpoint = self.cursor.checkpoint();
                self.skip_annotations();
                if self.at(JavaSyntaxKind::LBracket) && self.nth_kind(1) == JavaSyntaxKind::RBracket
                {
                    self.bump();
                    self.bump();
                } else {
                    self.cursor.rewind(checkpoint);
                    break;
                }
            }
            true
        } else {
            self.skip_type()
        }
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

    pub(in crate::parser::grammar) fn skip_missing_constructor_parameter_header(&mut self) -> bool {
        let mut paren_depth = 0usize;
        let mut brace_depth = 0usize;
        while !self.at_eof() {
            match missing_constructor_header_action(self.kind(), paren_depth, brace_depth) {
                MissingConstructorHeaderAction::OpenNested => {
                    paren_depth += 1;
                    self.bump();
                }
                MissingConstructorHeaderAction::CloseNested => {
                    paren_depth -= 1;
                    self.bump();
                }
                MissingConstructorHeaderAction::OpenBrace => {
                    brace_depth += 1;
                    self.bump();
                }
                MissingConstructorHeaderAction::CloseBrace => {
                    brace_depth -= 1;
                    self.bump();
                }
                MissingConstructorHeaderAction::CloseHeader => {
                    self.bump();
                    return true;
                }
                MissingConstructorHeaderAction::Boundary => return false,
                MissingConstructorHeaderAction::Bump => self.bump(),
            }
        }
        false
    }

    pub(in crate::parser::grammar) fn skip_type(&mut self) -> bool {
        if !self.skip_type_base() {
            return false;
        }

        loop {
            let checkpoint = self.cursor.checkpoint();
            self.skip_annotations();
            if self.at(JavaSyntaxKind::LBracket) && self.nth_kind(1) == JavaSyntaxKind::RBracket {
                self.bump();
                self.bump();
            } else {
                self.cursor.rewind(checkpoint);
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

        self.generic_depth += 1;
        while !self.at_eof() && !self.at_type_argument_close() {
            if !self.skip_type_argument() {
                while !is_type_argument_recovery_boundary(self.kind()) {
                    self.skip_annotations();
                    if is_type_argument_value_start(self.kind()) {
                        self.skip_type_argument();
                        break;
                    }
                    if is_type_argument_recovery_boundary(self.kind()) {
                        break;
                    }
                    self.bump();
                }
            }

            if !self.eat(JavaSyntaxKind::Comma) {
                break;
            }
        }

        self.eat_type_argument_close();
        self.generic_depth -= 1;
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
        if self.generic_depth > MAX_RECURSIVE_PARSE_OWNERS {
            let end = over_depth_type_end(self.buffer, self.cursor);
            self.cursor.seek_forward(end);
            return true;
        }

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

    pub(in crate::parser::grammar) fn at_type_argument_close(&mut self) -> bool {
        self.kind() == JavaSyntaxKind::Gt
    }

    pub(in crate::parser::grammar) fn eat_type_argument_close(&mut self) -> bool {
        self.eat(JavaSyntaxKind::Gt)
    }

    pub(in crate::parser::grammar) fn at_name_segment(&mut self) -> bool {
        self.kind() == JavaSyntaxKind::Identifier
    }

    pub(in crate::parser::grammar) fn at_variable_identifier(&mut self) -> bool {
        matches!(
            self.kind(),
            JavaSyntaxKind::Identifier | JavaSyntaxKind::UnderscoreKw
        )
    }

    pub(in crate::parser::grammar) fn at_type_start(&mut self) -> bool {
        self.at_non_void_type_start() || self.at(JavaSyntaxKind::VoidKw)
    }

    pub(in crate::parser::grammar) fn at_non_void_type_start(&mut self) -> bool {
        self.at_name_segment() || self.at_primitive_type_start()
    }

    pub(in crate::parser::grammar) fn at_primitive_type_start(&mut self) -> bool {
        is_primitive_type_start(self.kind())
    }

    pub(in crate::parser::grammar) fn at_type_modifier(&mut self) -> bool {
        self.type_modifier_len().is_some()
    }

    fn skip_type_modifier(&mut self) {
        let len = self.type_modifier_len().unwrap_or(1);
        for _ in 0..len {
            self.bump();
        }
    }

    fn type_modifier_len(&mut self) -> Option<usize> {
        type_modifier_len(self.kind(), self.text(), self.nth_kind(1), self.nth_text(2))
    }

    pub(in crate::parser::grammar) fn starts_expression(&mut self) -> bool {
        let kind = self.kind();
        matches!(
            kind,
            JavaSyntaxKind::Identifier
                | JavaSyntaxKind::UnderscoreKw
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
        ) || is_literal_expression_start(kind)
            || self.starts_primitive_or_void_class_literal()
    }

    pub(in crate::parser::grammar) fn starts_expression_not_plus_minus(&mut self) -> bool {
        self.starts_expression()
            && !matches!(self.kind(), JavaSyntaxKind::Plus | JavaSyntaxKind::Minus)
    }

    pub(in crate::parser::grammar) fn starts_literal_expression(&mut self) -> bool {
        is_literal_expression_start(self.kind())
    }

    fn starts_primitive_or_void_class_literal(&mut self) -> bool {
        let kind = self.kind();
        if !is_primitive_type_start(kind) && kind != JavaSyntaxKind::VoidKw {
            return false;
        }

        let mut cursor = self.cursor.fork();
        cursor.bump(self.buffer);
        if kind == JavaSyntaxKind::VoidKw {
            return cursor.kind(self.buffer) == JavaSyntaxKind::Dot
                && cursor.nth_kind(self.buffer, 1) == JavaSyntaxKind::ClassKw;
        }
        while cursor.kind(self.buffer) == JavaSyntaxKind::LBracket
            && cursor.nth_kind(self.buffer, 1) == JavaSyntaxKind::RBracket
        {
            cursor.bump(self.buffer);
            cursor.bump(self.buffer);
        }

        cursor.kind(self.buffer) == JavaSyntaxKind::Dot
            && cursor.nth_kind(self.buffer, 1) == JavaSyntaxKind::ClassKw
    }
}
