//! Declaration-level grammar questions, answered before the parser commits
//! to a branch.

use super::{JavaLookahead, JavaSyntaxKind, Parser, gt_count};

/// The most tokens [`Parser::starts_receiver_parameter`] looks at.
const RECEIVER_SCAN_LIMIT: usize = 512;

impl Parser<'_> {
    pub(in crate::parser::grammar) fn at_header_clause_end(&self) -> bool {
        matches!(
            self.current(),
            JavaSyntaxKind::LBrace
                | JavaSyntaxKind::LParen
                | JavaSyntaxKind::Semicolon
                | JavaSyntaxKind::ImplementsKw
                | JavaSyntaxKind::ExtendsKw
        ) || self.at_contextual("permits")
    }

    pub(in crate::parser::grammar) fn starts_constructor(&self, type_name: Option<usize>) -> bool {
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
        &self,
        type_name: Option<usize>,
    ) -> bool {
        let type_name = type_name.map(|index| self.text_at(index));
        let mut lookahead = self.lookahead();
        lookahead.skip_type_modifiers();
        type_name.is_some_and(|name| lookahead.text() == name)
            && lookahead.nth(1) == JavaSyntaxKind::LBrace
    }

    pub(in crate::parser::grammar) fn starts_method_declaration(&self) -> bool {
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

    pub(in crate::parser::grammar) fn starts_annotation_element(&self) -> bool {
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
        &self,
    ) -> bool {
        let mut lookahead = self.lookahead();
        if !lookahead.at_name_segment() {
            return false;
        }

        lookahead.bump();
        while lookahead.at(JavaSyntaxKind::Dot) && lookahead.nth(1) == JavaSyntaxKind::Identifier {
            lookahead.bump();
            lookahead.bump();
        }

        if !lookahead.at(JavaSyntaxKind::Dot) {
            return false;
        }

        lookahead.bump();
        lookahead.starts_constructor_super_suffix()
    }

    pub(in crate::parser::grammar) fn dot_starts_constructor_super_suffix(&self) -> bool {
        if self.current() != JavaSyntaxKind::Dot {
            return false;
        }

        let mut lookahead = self.lookahead();
        lookahead.bump();
        lookahead.starts_constructor_super_suffix()
    }

    pub(in crate::parser::grammar) fn starts_package_declaration(&self) -> bool {
        let mut lookahead = self.lookahead();
        lookahead.skip_annotations();
        lookahead.at(JavaSyntaxKind::PackageKw)
    }

    pub(in crate::parser::grammar) fn starts_module_declaration(&self) -> bool {
        let mut lookahead = self.lookahead();
        lookahead.skip_annotations();
        if lookahead.at_contextual("open") {
            lookahead.bump();
        }

        lookahead.at_contextual("module")
    }

    pub(in crate::parser::grammar) fn starts_type_declaration(&self) -> bool {
        let mut lookahead = self.lookahead();
        lookahead.skip_type_modifiers();
        matches!(
            lookahead.kind(),
            JavaSyntaxKind::ClassKw | JavaSyntaxKind::InterfaceKw | JavaSyntaxKind::EnumKw
        ) || (lookahead.at_contextual("record") && lookahead.nth(1) == JavaSyntaxKind::Identifier)
            || (lookahead.at(JavaSyntaxKind::At) && lookahead.nth(1) == JavaSyntaxKind::InterfaceKw)
    }

    pub(in crate::parser::grammar) fn starts_field_declaration(&self) -> bool {
        let mut lookahead = self.lookahead();
        lookahead.skip_type_modifiers();
        lookahead.skip_type() && lookahead.at_variable_identifier()
    }

    /// Whether the member header at the current token reaches a `{` before a
    /// `;` (bracketed groups skipped).
    pub(in crate::parser::grammar) fn member_header_ends_with_block(&self) -> bool {
        self.kind_at(self.table.header_end(self.position())) == JavaSyntaxKind::LBrace
    }

    /// Whether `this` appears before the end of the current parameter: a
    /// `,` or `)` outside the brackets and type arguments the parameter
    /// opens. The scan stops after `RECEIVER_SCAN_LIMIT` tokens (a receiver's
    /// type is never that long), so a query is `O(1)` even when recovery asks
    /// again at every token of a malformed list.
    pub(in crate::parser::grammar) fn starts_receiver_parameter(&self) -> bool {
        // Brackets, and type arguments outside them (inside, `<` is an
        // annotation argument's comparison).
        let (mut brackets, mut angles) = (0usize, 0usize);
        let start = self.position();
        for index in start..start + RECEIVER_SCAN_LIMIT {
            let kind = self.kind_at(index);
            match kind {
                JavaSyntaxKind::ThisKw => return true,
                JavaSyntaxKind::Eof
                | JavaSyntaxKind::Semicolon
                | JavaSyntaxKind::LBrace
                | JavaSyntaxKind::RBrace => return false,
                JavaSyntaxKind::Comma | JavaSyntaxKind::RParen if brackets == 0 && angles == 0 => {
                    return false;
                }
                JavaSyntaxKind::LParen | JavaSyntaxKind::LBracket => brackets += 1,
                JavaSyntaxKind::RParen | JavaSyntaxKind::RBracket => {
                    brackets = brackets.saturating_sub(1);
                }
                _ if brackets > 0 => {}
                JavaSyntaxKind::Lt => angles += 1,
                JavaSyntaxKind::Gt | JavaSyntaxKind::RShift | JavaSyntaxKind::UnsignedRShift => {
                    angles = angles.saturating_sub(gt_count(kind));
                }
                _ => {}
            }
        }
        false
    }

    pub(in crate::parser::grammar) fn at_module_directive_start(&self) -> bool {
        matches!(
            self.current_text(),
            "requires" | "exports" | "opens" | "uses" | "provides"
        )
    }
}

impl JavaLookahead<'_, '_> {
    /// Whether `super(`, after optional type arguments, starts here.
    fn starts_constructor_super_suffix(&mut self) -> bool {
        if self.at(JavaSyntaxKind::Lt) {
            self.skip_type_arguments();
        }

        self.at(JavaSyntaxKind::SuperKw) && self.nth(1) == JavaSyntaxKind::LParen
    }
}
