// Answers declaration-level grammar questions before the parser commits to a branch.
use super::{JavaLookahead, JavaSyntaxKind, Parser};

impl Parser<'_> {
    pub(in crate::parser::grammar) fn at_header_clause_end(&mut self) -> bool {
        matches!(
            self.current(),
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
        let type_name = type_name.map(|index| self.text_at(index));
        let mut lookahead = self.lookahead();
        lookahead.skip_type_modifiers();
        if lookahead.at(JavaSyntaxKind::Lt) {
            lookahead.skip_type_parameters();
        }

        if lookahead.at(JavaSyntaxKind::LParen) && member_header_ends_with_block {
            return true;
        }

        let has_constructor_name = type_name.is_some_and(|name| lookahead.text() == name);
        lookahead.nth(1) == JavaSyntaxKind::LParen
            && (has_constructor_name
                || (lookahead.at_name_segment() && member_header_ends_with_block))
    }

    pub(in crate::parser::grammar) fn starts_compact_constructor(
        &mut self,
        type_name: Option<usize>,
    ) -> bool {
        let type_name = type_name.map(|index| self.text_at(index));
        let mut lookahead = self.lookahead();
        lookahead.skip_type_modifiers();
        type_name.is_some_and(|name| lookahead.text() == name)
            && lookahead.nth(1) == JavaSyntaxKind::LBrace
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
                (lookahead.nth(1), lookahead.nth(2)),
                (JavaSyntaxKind::Identifier, JavaSyntaxKind::LParen) | (JavaSyntaxKind::LParen, _)
            );
        }

        if !lookahead.at_type_start() {
            return false;
        }

        lookahead.skip_type();
        lookahead.at(JavaSyntaxKind::LParen)
            || (lookahead.at_name_segment() && lookahead.nth(1) == JavaSyntaxKind::LParen)
    }

    pub(in crate::parser::grammar) fn starts_annotation_element(&mut self) -> bool {
        let mut lookahead = self.lookahead();
        lookahead.skip_type_modifiers();
        if !lookahead.at_non_void_type_start() {
            return false;
        }

        lookahead.skip_type();
        (lookahead.at(JavaSyntaxKind::LParen) && lookahead.nth(1) == JavaSyntaxKind::RParen)
            || (lookahead.at_name_segment()
                && lookahead.nth(1) == JavaSyntaxKind::LParen
                && lookahead.nth(2) == JavaSyntaxKind::RParen)
    }

    pub(in crate::parser::grammar) fn starts_expression_name_qualified_constructor_invocation(
        &mut self,
    ) -> bool {
        if !self.at_name_segment() {
            return false;
        }

        let mut lookahead = self.lookahead();
        lookahead.bump();
        while lookahead.at(JavaSyntaxKind::Dot) && lookahead.nth(1) == JavaSyntaxKind::Identifier {
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
        if self.current() != JavaSyntaxKind::Dot {
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

        lookahead.at(JavaSyntaxKind::SuperKw) && lookahead.nth(1) == JavaSyntaxKind::LParen
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
        if self.current_text() != "non" || self.nth(1) != JavaSyntaxKind::Minus {
            return false;
        }
        let index = self.position() + 2;
        self.text_at(index).starts_with("sealed") && self.nth_is_name_segment(3)
    }

    pub(in crate::parser::grammar) fn starts_type_declaration(&mut self) -> bool {
        let mut lookahead = self.lookahead();
        lookahead.skip_type_modifiers();
        matches!(
            lookahead.kind(),
            JavaSyntaxKind::ClassKw | JavaSyntaxKind::InterfaceKw | JavaSyntaxKind::EnumKw
        ) || (lookahead.at_contextual("record") && lookahead.nth(1) == JavaSyntaxKind::Identifier)
            || (lookahead.at(JavaSyntaxKind::At) && lookahead.nth(1) == JavaSyntaxKind::InterfaceKw)
    }

    pub(in crate::parser::grammar) fn starts_compact_member_declaration(&mut self) -> bool {
        let mut lookahead = self.lookahead();
        lookahead.skip_type_modifiers();
        lookahead.at_non_void_type_start()
            || (lookahead.at(JavaSyntaxKind::VoidKw)
                && lookahead.nth(1) == JavaSyntaxKind::Identifier
                && lookahead.nth(2) == JavaSyntaxKind::LParen)
    }

    pub(in crate::parser::grammar) fn starts_field_declaration(&mut self) -> bool {
        let mut lookahead = self.lookahead();
        lookahead.skip_type_modifiers();
        lookahead.skip_type() && lookahead.at_variable_identifier()
    }

    pub(in crate::parser::grammar) fn at_module_directive_start(&mut self) -> bool {
        matches!(
            self.current_text(),
            "requires" | "exports" | "opens" | "uses" | "provides"
        )
    }
}
