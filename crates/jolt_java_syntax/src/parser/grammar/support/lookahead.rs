// Provides a markerless grammar scanner over the same tokens as the parser.
use jolt_syntax::Lookahead;

use super::{
    JavaSyntaxKind, MissingConstructorHeaderAction, Parser, is_literal_expression_start,
    is_primitive_type_start, is_type_argument_recovery_boundary, is_type_argument_value_start,
    missing_constructor_header_action, type_modifier_len,
};
use crate::JavaLanguage;
use crate::parser::source::{LookaheadSummary, MAX_DEPTH};

impl<'source> Parser<'source> {
    pub(in crate::parser::grammar) fn lookahead(&mut self) -> JavaLookahead<'_, 'source> {
        let cursor = self.inner.lookahead();
        JavaLookahead {
            cursor,
            split: 0,
            base: cursor.position(),
            summary: &mut self.lookahead_summary,
            generic_depth: self.generic_depth,
        }
    }
}

/// The kind left after removing the first `>` of a `>`-family token.
pub(in crate::parser::grammar) const fn gt_rest(kind: JavaSyntaxKind) -> Option<JavaSyntaxKind> {
    Some(match kind {
        JavaSyntaxKind::RShift => JavaSyntaxKind::Gt,
        JavaSyntaxKind::UnsignedRShift => JavaSyntaxKind::RShift,
        JavaSyntaxKind::GtEq => JavaSyntaxKind::Assign,
        JavaSyntaxKind::RShiftEq => JavaSyntaxKind::GtEq,
        JavaSyntaxKind::UnsignedRShiftEq => JavaSyntaxKind::RShiftEq,
        _ => return None,
    })
}

/// Whether `kind` starts with `>`, so it can close a type argument list.
pub(in crate::parser::grammar) const fn is_gt_family(kind: JavaSyntaxKind) -> bool {
    matches!(kind, JavaSyntaxKind::Gt) || gt_rest(kind).is_some()
}

/// A speculative scanner. It never splits tokens: closing a type argument
/// list on a `>`-family token consumes one `>` of it virtually (`split`).
pub(in crate::parser::grammar) struct JavaLookahead<'p, 'source> {
    cursor: Lookahead<'p, 'source, JavaLanguage>,
    /// `>` characters already consumed from the current token.
    split: u8,
    base: usize,
    summary: &'p mut LookaheadSummary,
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

    fn skip_annotation(&mut self) -> bool {
        if !self.at_annotation_start() {
            return false;
        }

        self.bump();
        let has_name = self.at_name_segment();
        if has_name {
            self.bump();
            while self.at(JavaSyntaxKind::Dot) && self.nth(1) == JavaSyntaxKind::Identifier {
                self.bump();
                self.bump();
            }
        }
        if has_name && self.at(JavaSyntaxKind::LParen) {
            let after = self.summary.after(self.cursor, self.base);
            self.seek_forward(after);
        }

        true
    }

    pub(in crate::parser::grammar) fn skip_annotations(&mut self) -> bool {
        if !self.at_annotation_start() {
            return false;
        }

        self.summary.ensure_built(self.cursor, self.base);
        let start = self.position();
        let mut pending = 0;
        while self.at_annotation_start() {
            let offset = self.position() - self.summary.base;
            let entry = self.summary.boundaries.as_mut().expect("built summary")[offset];
            if entry != 0 {
                self.seek_forward(entry - 1);
                break;
            }
            let entry = &mut self.summary.boundaries.as_mut().expect("built summary")[offset];
            *entry = pending;
            pending = offset + 1;

            let skipped = self.skip_annotation();
            debug_assert!(skipped);
        }

        let end = self.position();
        let endpoint = end + 1;
        let boundaries = self.summary.boundaries.as_mut().expect("built summary");
        while pending != 0 {
            let index = pending - 1;
            pending = boundaries[index];
            boundaries[index] = endpoint;
        }
        debug_assert!(end != start);
        true
    }

    fn at_annotation_start(&mut self) -> bool {
        self.at(JavaSyntaxKind::At) && self.nth(1) != JavaSyntaxKind::InterfaceKw
    }

    fn skip_bounded_annotation(&mut self) -> bool {
        if !self.at(JavaSyntaxKind::At) || self.nth(1) == JavaSyntaxKind::InterfaceKw {
            return false;
        }

        self.bump();
        if self.at_name_segment() {
            self.bump();
            while self.at(JavaSyntaxKind::Dot) && self.nth(1) == JavaSyntaxKind::Identifier {
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
                let checkpoint = self.checkpoint();
                self.skip_annotations();
                if self.at(JavaSyntaxKind::LBracket) && self.nth(1) == JavaSyntaxKind::RBracket {
                    self.bump();
                    self.bump();
                } else {
                    self.rewind(checkpoint);
                    break;
                }
            }
            true
        } else {
            self.skip_type()
        }
    }

    /// Skips a run of annotations and modifiers. Every position the scan
    /// passes records the run's end, so rescans from inside the run (one per
    /// token in recovery) are O(1).
    pub(in crate::parser::grammar) fn skip_type_modifiers(&mut self) {
        self.summary.ensure_built(self.cursor, self.base);
        let len = self
            .summary
            .boundaries
            .as_ref()
            .expect("built summary")
            .len();
        if self.summary.modifier_ends.len() != len {
            self.summary.modifier_ends = vec![0; len];
        }
        let mut pending = 0;
        loop {
            let offset = self.position() - self.summary.base;
            let cached = self.summary.modifier_ends.get(offset).copied().unwrap_or(0);
            if cached != 0 {
                self.seek_forward(cached - 1);
                break;
            }
            if let Some(entry) = self.summary.modifier_ends.get_mut(offset) {
                *entry = pending;
                pending = offset + 1;
            }
            if self.skip_annotations() {
                continue;
            }
            if self.at_type_modifier() {
                self.skip_type_modifier();
                continue;
            }
            break;
        }
        let endpoint = self.position() + 1;
        while pending != 0 {
            let index = pending - 1;
            pending = self.summary.modifier_ends[index];
            self.summary.modifier_ends[index] = endpoint;
        }
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
        type_modifier_len(self.kind(), self.text(), self.nth(1), self.nth_text(2))
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
            && !matches!(
                self.kind(),
                JavaSyntaxKind::Plus
                    | JavaSyntaxKind::Minus
                    | JavaSyntaxKind::PlusPlus
                    | JavaSyntaxKind::MinusMinus
            )
    }

    pub(in crate::parser::grammar) fn starts_literal_expression(&mut self) -> bool {
        is_literal_expression_start(self.kind())
    }

    fn starts_primitive_or_void_class_literal(&mut self) -> bool {
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
