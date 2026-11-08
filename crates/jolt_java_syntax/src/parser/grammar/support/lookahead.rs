//! A markerless grammar scanner over the same tokens as the parser.

use jolt_syntax::Lookahead;

use super::{
    JavaSyntaxKind, LookaheadTable, Parser, TypeModifier, gt_rest, is_gt_family,
    is_literal_expression_start, is_primitive_type_start, is_type_argument_recovery_boundary,
    is_type_argument_value_start, starts_annotation,
};
use crate::JavaLanguage;
use crate::parser::source::MAX_DEPTH;

impl<'source> Parser<'source> {
    /// A speculative scanner at the current token.
    pub(in crate::parser::grammar) fn lookahead(&self) -> JavaLookahead<'_, 'source> {
        JavaLookahead {
            cursor: self.inner.lookahead(),
            split: 0,
            table: &self.table,
            generic_depth: self.generic_depth,
        }
    }
}

/// A speculative scanner. It never splits tokens: closing a type argument
/// list on a `>`-family token consumes one `>` of it virtually (`split`).
pub(in crate::parser::grammar) struct JavaLookahead<'p, 'source> {
    cursor: Lookahead<'p, 'source, JavaLanguage>,
    /// `>` characters already consumed from the current token.
    split: u8,
    table: &'p LookaheadTable,
    generic_depth: usize,
}

#[derive(Clone, Copy)]
pub(in crate::parser::grammar) struct Checkpoint {
    position: usize,
    split: u8,
}

impl<'source> JavaLookahead<'_, 'source> {
    pub(in crate::parser::grammar) fn kind(&self) -> JavaSyntaxKind {
        let mut kind = self.cursor.current();
        for _ in 0..self.split {
            kind = gt_rest(kind).unwrap_or(kind);
        }
        kind
    }

    pub(in crate::parser::grammar) fn nth(&self, n: usize) -> JavaSyntaxKind {
        if n == 0 {
            self.kind()
        } else {
            self.cursor.nth(n)
        }
    }

    pub(in crate::parser::grammar) fn text(&self) -> &'source str {
        self.cursor.nth_text(0)
    }

    fn nth_text(&self, n: usize) -> &'source str {
        self.cursor.nth_text(n)
    }

    pub(in crate::parser::grammar) fn at(&self, kind: JavaSyntaxKind) -> bool {
        self.kind() == kind
    }

    pub(in crate::parser::grammar) fn at_contextual(&self, text: &str) -> bool {
        self.at(JavaSyntaxKind::Identifier) && self.text() == text
    }

    pub(in crate::parser::grammar) fn at_eof(&self) -> bool {
        self.at(JavaSyntaxKind::Eof)
    }

    pub(in crate::parser::grammar) fn bump(&mut self) {
        self.split = 0;
        self.cursor.bump();
    }

    pub(in crate::parser::grammar) fn position(&self) -> usize {
        self.cursor.position()
    }

    fn checkpoint(&self) -> Checkpoint {
        Checkpoint {
            position: self.cursor.position(),
            split: self.split,
        }
    }

    fn rewind(&mut self, checkpoint: Checkpoint) {
        self.cursor.set_position(checkpoint.position);
        self.split = checkpoint.split;
    }

    fn seek_forward(&mut self, position: usize) {
        debug_assert!(
            position >= self.cursor.position(),
            "lookahead cannot seek backward"
        );
        self.split = 0;
        self.cursor.set_position(position);
    }

    pub(in crate::parser::grammar) fn eat(&mut self, kind: JavaSyntaxKind) -> bool {
        if self.at(kind) {
            self.bump();
            true
        } else {
            false
        }
    }

    /// Skips a run of annotations, if one starts here.
    pub(in crate::parser::grammar) fn skip_annotations(&mut self) -> bool {
        if !self.at_annotation_start() {
            return false;
        }
        self.seek_forward(self.table.annotations_end(self.position()));
        true
    }

    /// Whether an annotation starts here (`@interface` starts a declaration).
    pub(in crate::parser::grammar) fn at_annotation_start(&self) -> bool {
        starts_annotation(self.kind(), self.nth(1))
    }

    /// Skips a run of annotations and modifiers.
    pub(in crate::parser::grammar) fn skip_type_modifiers(&mut self) {
        self.seek_forward(self.table.modifiers_end(self.position()));
    }

    pub(in crate::parser::grammar) fn skip_variable_modifiers(&mut self) {
        loop {
            if self.skip_annotations() {
                continue;
            }

            if self.eat(JavaSyntaxKind::FinalKw) {
                continue;
            }

            return;
        }
    }

    pub(in crate::parser::grammar) fn skip_type(&mut self) -> bool {
        if !self.skip_type_base() {
            return false;
        }

        loop {
            let checkpoint = self.checkpoint();
            self.skip_annotations();
            if self.at(JavaSyntaxKind::LBracket) && self.nth(1) == JavaSyntaxKind::RBracket {
                self.bump();
                self.bump();
            } else {
                self.rewind(checkpoint);
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
            let checkpoint = self.checkpoint();
            self.bump();
            self.skip_annotations();
            if !self.at_name_segment() {
                self.rewind(checkpoint);
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
            let checkpoint = self.checkpoint();
            if !self.eat(JavaSyntaxKind::Amp) {
                return true;
            }

            self.skip_annotations();
            if !self.at_type_start() || !self.skip_type() {
                self.rewind(checkpoint);
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
        // Past the depth limit the consuming parser recovers anyway; failing
        // here keeps every scan bounded.
        if self.generic_depth > MAX_DEPTH as usize {
            return false;
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

    pub(in crate::parser::grammar) fn at_type_argument_close(&self) -> bool {
        is_gt_family(self.kind())
    }

    /// Consumes one `>`, which may be the first character of a longer token.
    pub(in crate::parser::grammar) fn eat_type_argument_close(&mut self) -> bool {
        let kind = self.kind();
        if kind == JavaSyntaxKind::Gt {
            self.bump();
        } else if is_gt_family(kind) {
            self.split += 1;
        } else {
            return false;
        }
        true
    }

    pub(in crate::parser::grammar) fn at_name_segment(&self) -> bool {
        self.at(JavaSyntaxKind::Identifier)
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
        is_primitive_type_start(self.kind())
    }

    /// The type modifier that starts here.
    pub(in crate::parser::grammar) fn type_modifier(&self) -> Option<TypeModifier> {
        TypeModifier::from_tokens(self.kind(), self.text(), self.nth(1), self.nth_text(2))
    }

    pub(in crate::parser::grammar) fn at_type_modifier(&self) -> bool {
        self.type_modifier().is_some()
    }

    pub(in crate::parser::grammar) fn starts_expression(&self) -> bool {
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

    pub(in crate::parser::grammar) fn starts_expression_not_plus_minus(&self) -> bool {
        self.starts_expression()
            && !matches!(
                self.kind(),
                JavaSyntaxKind::Plus
                    | JavaSyntaxKind::Minus
                    | JavaSyntaxKind::PlusPlus
                    | JavaSyntaxKind::MinusMinus
            )
    }

    pub(in crate::parser::grammar) fn starts_literal_expression(&self) -> bool {
        is_literal_expression_start(self.kind())
    }

    /// Whether a class literal of a primitive type or `void` starts here:
    /// `int[].class`, `void.class`.
    pub(in crate::parser::grammar) fn starts_primitive_or_void_class_literal(&self) -> bool {
        let kind = self.kind();
        if !is_primitive_type_start(kind) && kind != JavaSyntaxKind::VoidKw {
            return false;
        }

        let mut cursor = self.cursor;
        cursor.bump();
        if kind == JavaSyntaxKind::VoidKw {
            return cursor.at(JavaSyntaxKind::Dot) && cursor.nth(1) == JavaSyntaxKind::ClassKw;
        }
        while cursor.at(JavaSyntaxKind::LBracket) && cursor.nth(1) == JavaSyntaxKind::RBracket {
            cursor.bump();
            cursor.bump();
        }

        cursor.at(JavaSyntaxKind::Dot) && cursor.nth(1) == JavaSyntaxKind::ClassKw
    }
}
