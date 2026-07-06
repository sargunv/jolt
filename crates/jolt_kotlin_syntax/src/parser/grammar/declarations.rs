use crate::KotlinSyntaxKind as K;

use super::Parser;

#[path = "declarations/callables.rs"]
mod callables;
#[path = "declarations/classes.rs"]
mod classes;

impl Parser<'_> {
    pub(super) fn parse_declaration_or_statement(&mut self) {
        let marker = self.start();
        self.parse_modifier_list();
        if self.at_context_parameter_clause() {
            self.parse_context_parameter_clause();
        }

        let kind = match self.current_kind() {
            K::ClassKw => {
                self.parse_class_or_interface_tail();
                K::ClassDeclaration
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
                self.parse_type_alias_tail();
                K::TypeAliasDeclaration
            }
            kind if self.is_soft_kind(kind, "constructor") => {
                self.parse_secondary_constructor_tail();
                K::SecondaryConstructor
            }
            kind if self.is_soft_kind(kind, "init") => {
                self.bump();
                self.parse_block();
                K::InitializerBlock
            }
            kind if self.is_soft_kind(kind, "companion") && self.nth_kind(1) == K::ObjectKw => {
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

    pub(super) fn parse_modifier_list(&mut self) {
        if !self.at_modifier_or_annotation() {
            return;
        }

        let marker = self.start();
        while self.at_modifier_or_annotation() {
            if self.at(K::At) || self.at(K::Hash) {
                self.parse_annotation();
            } else {
                self.bump();
            }
        }
        self.complete(marker, K::ModifierList);
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

    fn parse_annotation_argument_list(&mut self) {
        let marker = self.start();
        self.expect(K::LParen, "expected annotation argument list");
        self.parse_comma_separated_until(K::RParen, K::ValueArgument);
        self.expect(K::RParen, "expected ')' after annotation arguments");
        self.complete(marker, K::AnnotationArgumentList);
    }
}
