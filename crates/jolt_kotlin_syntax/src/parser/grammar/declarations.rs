use crate::KotlinSyntaxKind as K;

use super::Parser;

const MAX_DECLARATION_LOOKAHEAD: usize = 256;

#[path = "declarations/callables.rs"]
mod callables;
#[path = "declarations/classes.rs"]
mod classes;

impl Parser<'_> {
    pub(super) fn parse_declaration_or_statement(&mut self) {
        self.parse_declaration_or_statement_with_class_members(false);
    }

    pub(in crate::parser::grammar) fn parse_class_member_declaration_or_statement(&mut self) {
        self.parse_declaration_or_statement_with_class_members(true);
    }

    fn parse_declaration_or_statement_with_class_members(&mut self, allow_class_members: bool) {
        if !self.at_declaration_start(allow_class_members) {
            let marker = self.start();
            self.parse_statement_tail();
            self.complete(marker, K::Statement);
            return;
        }

        if allow_class_members && self.at_soft_keyword("init") {
            let marker = self.start();
            self.bump();
            self.parse_block();
            self.complete(marker, K::InitializerBlock);
            return;
        }

        let marker = self.start();
        self.parse_modifier_list();
        if self.at_context_parameter_clause() {
            self.parse_context_parameter_clause();
        }
        self.parse_modifier_list();

        let kind = match self.current_kind() {
            K::ClassKw => {
                self.parse_class_or_interface_tail();
                K::ClassDeclaration
            }
            K::FunKw if self.nth_kind(1) == K::InterfaceKw => {
                self.bump();
                self.parse_class_or_interface_tail();
                K::InterfaceDeclaration
            }
            K::InterfaceKw => {
                self.parse_class_or_interface_tail();
                K::InterfaceDeclaration
            }
            K::ObjectKw => {
                self.parse_object_tail();
                K::ObjectDeclaration
            }
            K::FunKw => {
                self.parse_function_tail();
                K::FunctionDeclaration
            }
            K::ValKw | K::VarKw => {
                self.parse_property_tail();
                K::PropertyDeclaration
            }
            K::TypeAliasKw => {
                self.parse_type_alias_tail(marker.anchor());
                K::TypeAliasDeclaration
            }
            kind if self.is_soft_kind(kind, "constructor") => {
                self.parse_secondary_constructor_tail();
                K::SecondaryConstructor
            }
            kind if allow_class_members && self.is_soft_kind(kind, "init") => {
                self.bump();
                self.parse_block();
                K::InitializerBlock
            }
            kind if allow_class_members
                && self.is_soft_kind(kind, "companion")
                && self.nth_kind(1) == K::ObjectKw =>
            {
                self.bump();
                self.parse_object_tail();
                K::CompanionObject
            }
            _ => {
                self.parse_statement_tail();
                K::Statement
            }
        };

        self.complete(marker, kind);
    }

    pub(in crate::parser::grammar) fn at_declaration_start(
        &mut self,
        allow_class_members: bool,
    ) -> bool {
        let Some(mut index) = self.skip_modifier_prefix(self.position()) else {
            return false;
        };

        if self.is_soft_kind_at(index, "context") && self.kind_at(index + 1) == K::LParen {
            let Some(after_context) = self.skip_balanced_delimiter(index + 1, K::LParen, K::RParen)
            else {
                return false;
            };
            let Some(after_context_prefix) = self.skip_modifier_prefix(after_context) else {
                return false;
            };
            index = after_context_prefix;
        }

        self.declaration_head_at(index, allow_class_members)
    }

    pub(in crate::parser::grammar) fn skip_modifier_prefix(
        &mut self,
        mut index: usize,
    ) -> Option<usize> {
        let end = self.position() + MAX_DECLARATION_LOOKAHEAD;
        while index < end && self.is_modifier_or_annotation_start_at(index) {
            if matches!(self.kind_at(index), K::At | K::Hash) {
                index = self.skip_annotation_at(index)?;
            } else {
                index += 1;
            }
        }
        (index < end).then_some(index)
    }

    fn skip_annotation_at(&mut self, mut index: usize) -> Option<usize> {
        index += 1;
        if self.is_annotation_use_site_target_at(index) && self.kind_at(index + 1) == K::Colon {
            index += 2;
        }

        let end = self.position() + MAX_DECLARATION_LOOKAHEAD;
        let mut consumed_name = false;
        while index < end {
            if matches!(self.kind_at(index), K::Identifier)
                || self.text_at(index).is_some_and(|text| {
                    text.chars()
                        .next()
                        .is_some_and(|character| character == '_' || character.is_alphabetic())
                })
            {
                consumed_name = true;
                index += 1;
                if self.kind_at(index) == K::Dot {
                    index += 1;
                    continue;
                }
            }
            break;
        }

        if !consumed_name {
            return None;
        }

        if self.kind_at(index) == K::LParen {
            index = self.skip_balanced_delimiter(index, K::LParen, K::RParen)?;
        }

        Some(index)
    }

    fn skip_balanced_delimiter(&mut self, mut index: usize, open: K, close: K) -> Option<usize> {
        debug_assert_eq!(self.kind_at(index), open);
        let end = self.position() + MAX_DECLARATION_LOOKAHEAD;
        let mut depth = 0usize;

        while index < end {
            match self.kind_at(index) {
                kind if kind == open => depth += 1,
                kind if kind == close => {
                    depth = depth.saturating_sub(1);
                    index += 1;
                    if depth == 0 {
                        return Some(index);
                    }
                    continue;
                }
                K::Eof => return None,
                _ => {}
            }
            index += 1;
        }
        None
    }

    fn declaration_head_at(&mut self, index: usize, allow_class_members: bool) -> bool {
        matches!(
            self.kind_at(index),
            K::ClassKw | K::InterfaceKw | K::FunKw | K::ValKw | K::VarKw | K::TypeAliasKw
        ) || self.is_soft_kind_at(index, "constructor")
            || (self.kind_at(index) == K::ObjectKw && self.object_declaration_head_at(index))
            || (allow_class_members && self.is_soft_kind_at(index, "init"))
            || (allow_class_members
                && self.is_soft_kind_at(index, "companion")
                && self.kind_at(index + 1) == K::ObjectKw)
    }

    fn object_declaration_head_at(&mut self, index: usize) -> bool {
        (index > 0 && self.is_soft_kind_at(index - 1, "companion"))
            || !matches!(self.kind_at(index + 1), K::Colon | K::LBrace | K::Eof)
    }

    pub(super) fn parse_modifier_list(&mut self) {
        let modifiers = self.start();
        while self.at_modifier_or_annotation() {
            let before = self.position();
            if self.at(K::At) || self.at(K::Hash) {
                self.parse_annotation();
            } else {
                self.bump();
            }
            debug_assert!(self.position() > before);
        }
        self.complete(modifiers, K::ModifierList);
    }

    pub(super) fn parse_annotation(&mut self) {
        let marker = self.start();
        let _ = self.eat(K::At) || self.eat(K::Hash);
        if self.at_annotation_use_site_target() && self.nth_kind(1) == K::Colon {
            let target = self.start();
            self.bump();
            self.bump();
            self.complete(target, K::AnnotationUseSiteTarget);
        }
        self.parse_qualified_name();
        if self.at(K::LParen) {
            self.parse_annotation_argument_list();
        }
        self.complete(marker, K::Annotation);
    }

    pub(super) fn parse_annotation_argument_list(&mut self) {
        let marker = self.start();
        self.eat_asserted(K::LParen);
        self.parse_value_arguments_until(K::RParen, K::ValueArgumentSeparatedList);
        if !self.eat(K::RParen) {
            let diagnostic = self.pending_expected("expected ')' after annotation arguments");
            self.missing_required_slot(
                marker.anchor(),
                crate::shape::annotation_argument_list::Slot::close_paren as u16,
                [diagnostic],
            );
        }
        self.complete(marker, K::AnnotationArgumentList);
    }
}
