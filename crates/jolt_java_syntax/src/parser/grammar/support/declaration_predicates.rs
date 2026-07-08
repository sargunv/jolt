// Answers declaration-level grammar questions before the parser commits to a branch.
use super::{JavaLookahead, JavaParserExt, JavaSyntaxKind, Parser};

impl Parser<'_> {
    pub(in crate::parser::grammar) fn at_header_clause_end(&mut self) -> bool {
        matches!(
            self.current_kind(),
            JavaSyntaxKind::LBrace
                | JavaSyntaxKind::LParen
                | JavaSyntaxKind::Semicolon
                | JavaSyntaxKind::ImplementsKw
                | JavaSyntaxKind::ExtendsKw
        ) || self.at_contextual("permits")
    }

    pub(in crate::parser::grammar) fn starts_constructor(
        &mut self,
        type_name: Option<usize>,
    ) -> bool {
        let member_header_ends_with_block = self.member_header_ends_with_block();
        let type_name = type_name.and_then(|index| self.text_at(index));
        let mut lookahead = self.lookahead();
        lookahead.skip_type_modifiers();
        if lookahead.at(JavaSyntaxKind::Lt) {
            lookahead.skip_type_parameters();
        }

        if lookahead.at(JavaSyntaxKind::LParen) && member_header_ends_with_block {
            return true;
        }

        lookahead.nth_kind(1) == JavaSyntaxKind::LParen
            && (matches!(type_name, Some(name) if lookahead.text() == Some(name))
                || (lookahead.at_name_segment() && member_header_ends_with_block))
    }

    pub(in crate::parser::grammar) fn starts_compact_constructor(
        &mut self,
        type_name: Option<usize>,
    ) -> bool {
        let type_name = type_name.and_then(|index| self.text_at(index));
        let mut lookahead = self.lookahead();
        lookahead.skip_type_modifiers();
        matches!(type_name, Some(name) if lookahead.text() == Some(name))
            && lookahead.nth_kind(1) == JavaSyntaxKind::LBrace
    }

    pub(in crate::parser::grammar) fn starts_method_declaration(&mut self) -> bool {
        let mut lookahead = self.lookahead();
        lookahead.skip_type_modifiers();
        if lookahead.at(JavaSyntaxKind::Lt) {
            lookahead.skip_type_parameters();
            lookahead.skip_annotations();
        }

        if lookahead.at(JavaSyntaxKind::VoidKw) {
            return matches!(
                (lookahead.nth_kind(1), lookahead.nth_kind(2)),
                (JavaSyntaxKind::Identifier, JavaSyntaxKind::LParen) | (JavaSyntaxKind::LParen, _)
            );
        }

        if !lookahead.at_type_start() {
            return false;
        }

        lookahead.skip_type();
        lookahead.at(JavaSyntaxKind::LParen)
            || (lookahead.at_name_segment() && lookahead.nth_kind(1) == JavaSyntaxKind::LParen)
    }

    pub(in crate::parser::grammar) fn starts_annotation_element(&mut self) -> bool {
        let mut lookahead = self.lookahead();
        lookahead.skip_type_modifiers();
        if !lookahead.at_non_void_type_start() {
            return false;
        }

        lookahead.skip_type();
        (lookahead.at(JavaSyntaxKind::LParen) && lookahead.nth_kind(1) == JavaSyntaxKind::RParen)
            || (lookahead.at_name_segment()
                && lookahead.nth_kind(1) == JavaSyntaxKind::LParen
                && lookahead.nth_kind(2) == JavaSyntaxKind::RParen)
    }

    pub(in crate::parser::grammar) fn starts_constructor_invocation_statement(&mut self) -> bool {
        let mut index = self.position();
        let mut saw_constructor_keyword = false;
        while !matches!(
            self.kind_at(index),
            JavaSyntaxKind::Eof | JavaSyntaxKind::Semicolon | JavaSyntaxKind::RBrace
        ) {
            if let Some(next) = self.skip_balanced_delimiter_at(index) {
                index = next;
                continue;
            }

            if matches!(
                self.kind_at(index),
                JavaSyntaxKind::LBrace | JavaSyntaxKind::RParen | JavaSyntaxKind::RBracket
            ) && index == self.position()
            {
                return false;
            }

            if matches!(
                self.kind_at(index),
                JavaSyntaxKind::ThisKw | JavaSyntaxKind::SuperKw
            ) && self.kind_at(index + 1) == JavaSyntaxKind::LParen
            {
                saw_constructor_keyword = true;
            }
            index += 1;
        }
        saw_constructor_keyword && self.kind_at(index) == JavaSyntaxKind::Semicolon
    }

    pub(in crate::parser::grammar) fn starts_expression_name_qualified_constructor_invocation(
        &mut self,
    ) -> bool {
        if !self.at_name_segment() {
            return false;
        }

        let mut lookahead = self.lookahead();
        lookahead.bump();
        while lookahead.at(JavaSyntaxKind::Dot)
            && lookahead.nth_kind(1) == JavaSyntaxKind::Identifier
        {
            lookahead.bump();
            lookahead.bump();
        }

        if !lookahead.at(JavaSyntaxKind::Dot) {
            return false;
        }

        lookahead.bump();
        Self::lookahead_starts_constructor_super_suffix(&mut lookahead)
    }

    pub(in crate::parser::grammar) fn dot_starts_constructor_super_suffix(&mut self) -> bool {
        if self.current_kind() != JavaSyntaxKind::Dot {
            return false;
        }

        let mut lookahead = self.lookahead();
        lookahead.bump();
        Self::lookahead_starts_constructor_super_suffix(&mut lookahead)
    }

    fn lookahead_starts_constructor_super_suffix(lookahead: &mut JavaLookahead<'_, '_>) -> bool {
        if lookahead.at(JavaSyntaxKind::Lt) {
            lookahead.skip_type_arguments();
        }

        lookahead.at(JavaSyntaxKind::SuperKw) && lookahead.nth_kind(1) == JavaSyntaxKind::LParen
    }

    pub(in crate::parser::grammar) fn starts_package_declaration(&mut self) -> bool {
        let mut lookahead = self.lookahead();
        lookahead.skip_annotations();
        lookahead.at(JavaSyntaxKind::PackageKw)
    }

    pub(in crate::parser::grammar) fn starts_module_declaration(&mut self) -> bool {
        let mut lookahead = self.lookahead();
        lookahead.skip_annotations();
        if lookahead.at_contextual("open") {
            lookahead.bump();
        }

        lookahead.at_contextual("module")
    }

    pub(in crate::parser::grammar) fn starts_misspelled_non_sealed_type_declaration(
        &mut self,
    ) -> bool {
        self.current_text() == Some("non")
            && self.nth_kind(1) == JavaSyntaxKind::Minus
            && matches!(self.text_at(self.position() + 2), Some(text) if text.starts_with("sealed"))
            && self.nth_is_name_segment(3)
    }

    pub(in crate::parser::grammar) fn starts_top_level_type_declaration(&mut self) -> bool {
        let mut lookahead = self.lookahead();
        lookahead.skip_type_modifiers();
        matches!(
            lookahead.kind(),
            JavaSyntaxKind::ClassKw | JavaSyntaxKind::InterfaceKw | JavaSyntaxKind::EnumKw
        ) || (lookahead.at_contextual("record")
            && lookahead.nth_kind(1) == JavaSyntaxKind::Identifier)
            || (lookahead.at(JavaSyntaxKind::At)
                && lookahead.nth_kind(1) == JavaSyntaxKind::InterfaceKw)
    }

    pub(in crate::parser::grammar) fn starts_compact_member_declaration(&mut self) -> bool {
        let mut lookahead = self.lookahead();
        lookahead.skip_type_modifiers();
        lookahead.at_non_void_type_start()
            || (lookahead.at(JavaSyntaxKind::VoidKw)
                && lookahead.nth_kind(1) == JavaSyntaxKind::Identifier
                && lookahead.nth_kind(2) == JavaSyntaxKind::LParen)
    }

    pub(in crate::parser::grammar) fn member_header_ends_with_block(&mut self) -> bool {
        let mut index = self.position();
        while self.kind_at(index) != JavaSyntaxKind::Eof {
            if let Some(next) = self.skip_balanced_delimiter_at(index) {
                index = next;
                continue;
            }

            match self.kind_at(index) {
                JavaSyntaxKind::LBrace => return true,
                JavaSyntaxKind::Semicolon => return false,
                _ => index += 1,
            }
        }

        false
    }

    pub(in crate::parser::grammar) fn at_module_directive_start(&mut self) -> bool {
        matches!(
            self.current_text(),
            Some("requires" | "exports" | "opens" | "uses" | "provides")
        )
    }
}
