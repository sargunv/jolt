// Answers statement-level grammar questions before the parser commits to a branch.
use super::{JavaSyntaxKind, Parser};

impl Parser<'_> {
    pub(in crate::parser::grammar) fn starts_receiver_parameter(&mut self) -> bool {
        let mut index = self.position();
        while !matches!(
            self.kind_at(index),
            JavaSyntaxKind::Eof
                | JavaSyntaxKind::Comma
                | JavaSyntaxKind::RParen
                | JavaSyntaxKind::Semicolon
        ) {
            if self.kind_at(index) == JavaSyntaxKind::ThisKw {
                return true;
            }
            index += 1;
        }
        false
    }

    pub(in crate::parser::grammar) fn starts_local_class_or_interface_declaration(
        &mut self,
    ) -> bool {
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

    pub(in crate::parser::grammar) fn starts_local_variable_declaration(&mut self) -> bool {
        let mut lookahead = self.lookahead();
        lookahead.skip_variable_modifiers();

        if lookahead.at_contextual("yield") && lookahead.nth_kind(1) != JavaSyntaxKind::Dot {
            return false;
        }

        if lookahead.at_contextual("var") && lookahead.nth_kind(1) != JavaSyntaxKind::Dot {
            return matches!(
                lookahead.nth_kind(1),
                JavaSyntaxKind::Identifier | JavaSyntaxKind::UnderscoreKw
            ) && !matches!(
                lookahead.nth_kind(2),
                JavaSyntaxKind::LParen | JavaSyntaxKind::Dot
            );
        }

        if !lookahead.at_non_void_type_start() {
            return false;
        }

        lookahead.skip_type();
        lookahead.at_variable_identifier()
            && !matches!(lookahead.nth_kind(1), JavaSyntaxKind::LParen)
    }

    pub(in crate::parser::grammar) fn starts_resource_local_variable_declaration(
        &mut self,
    ) -> bool {
        if !self.starts_local_variable_declaration() {
            return false;
        }

        let mut index = self.position();
        while self.kind_at(index) != JavaSyntaxKind::Eof {
            if let Some(next) = self.skip_balanced_delimiter_at(index) {
                index = next;
                continue;
            }

            match self.kind_at(index) {
                JavaSyntaxKind::Assign => return true,
                JavaSyntaxKind::Semicolon | JavaSyntaxKind::RParen | JavaSyntaxKind::RBracket => {
                    return false;
                }
                _ => {}
            }
            index += 1;
        }

        false
    }

    pub(in crate::parser::grammar) fn starts_labeled_statement(&mut self) -> bool {
        self.current_kind() == JavaSyntaxKind::Identifier
            && self.nth_kind(1) == JavaSyntaxKind::Colon
    }

    pub(in crate::parser::grammar) fn starts_yield_statement(&mut self) -> bool {
        self.at_contextual("yield")
            && self
                .assignment_operator_len_at(self.position() + 1)
                .is_none()
            && !matches!(
                self.nth_kind(1),
                JavaSyntaxKind::LBracket
                    | JavaSyntaxKind::Dot
                    | JavaSyntaxKind::Assign
                    | JavaSyntaxKind::PlusPlus
                    | JavaSyntaxKind::MinusMinus
                    | JavaSyntaxKind::PlusEq
                    | JavaSyntaxKind::MinusEq
                    | JavaSyntaxKind::StarEq
                    | JavaSyntaxKind::SlashEq
                    | JavaSyntaxKind::AmpEq
                    | JavaSyntaxKind::BarEq
                    | JavaSyntaxKind::CaretEq
                    | JavaSyntaxKind::PercentEq
                    | JavaSyntaxKind::LShiftEq
                    | JavaSyntaxKind::Semicolon
            )
            && !(self.nth_kind(1) == JavaSyntaxKind::LParen
                && self.tokens_are_adjacent(self.position(), 2))
    }

    pub(in crate::parser::grammar) fn for_header_has_top_level_colon(&mut self) -> bool {
        let mut index = self.position();
        while self.kind_at(index) != JavaSyntaxKind::Eof
            && self.kind_at(index) != JavaSyntaxKind::LParen
        {
            index += 1;
        }

        if self.kind_at(index) != JavaSyntaxKind::LParen {
            return false;
        }

        index += 1;
        let mut conditional_depth = 0usize;
        let mut angle_depth = 0usize;
        while self.kind_at(index) != JavaSyntaxKind::Eof {
            if let Some(next) = self.skip_balanced_delimiter_at(index) {
                index = next;
                continue;
            }

            match self.kind_at(index) {
                JavaSyntaxKind::Lt => angle_depth += 1,
                JavaSyntaxKind::Gt if angle_depth > 0 => angle_depth -= 1,
                JavaSyntaxKind::Question if angle_depth == 0 => conditional_depth += 1,
                JavaSyntaxKind::Colon if angle_depth == 0 && conditional_depth > 0 => {
                    conditional_depth -= 1;
                }
                JavaSyntaxKind::Colon if angle_depth == 0 => return true,
                JavaSyntaxKind::Semicolon | JavaSyntaxKind::RParen | JavaSyntaxKind::RBracket => {
                    return false;
                }
                _ => {}
            }
            index += 1;
        }

        false
    }

    pub(in crate::parser::grammar) fn starts_switch_label(&mut self) -> bool {
        matches!(
            self.current_kind(),
            JavaSyntaxKind::CaseKw | JavaSyntaxKind::DefaultKw
        )
    }

    pub(in crate::parser::grammar) fn switch_label_is_rule(&mut self) -> bool {
        let mut index = self.position();
        while self.kind_at(index) != JavaSyntaxKind::Eof {
            if let Some(next) = self.skip_balanced_delimiter_at(index) {
                index = next;
                continue;
            }

            match self.kind_at(index) {
                JavaSyntaxKind::Arrow => return true,
                JavaSyntaxKind::Colon
                | JavaSyntaxKind::RBrace
                | JavaSyntaxKind::RParen
                | JavaSyntaxKind::RBracket => {
                    return false;
                }
                _ => {}
            }
            index += 1;
        }

        false
    }
}
