use crate::KotlinSyntaxKind as K;

use super::super::Parser;

impl Parser<'_> {
    pub(in crate::parser::grammar) fn parse_class_or_interface_tail(&mut self) {
        self.bump();
        self.parse_name();
        if self.at(K::Lt) {
            self.parse_type_parameter_list();
        }
        if self.at_primary_constructor_start() {
            self.parse_primary_constructor();
        }
        if self.eat(K::Colon) {
            self.parse_delegation_specifier_list();
        }
        self.parse_type_constraint_list();
        if self.at(K::LBrace) {
            self.parse_class_body();
        }
    }

    pub(in crate::parser::grammar) fn parse_object_tail(&mut self) {
        self.expect(K::ObjectKw, "expected object");
        if !matches!(self.current_kind(), K::Colon | K::LBrace | K::Eof) {
            self.parse_name();
        }
        if self.eat(K::Colon) {
            self.parse_delegation_specifier_list();
        }
        if self.at(K::LBrace) {
            self.parse_class_body();
        }
    }

    pub(in crate::parser::grammar) fn parse_class_body(&mut self) {
        let marker = self.start();
        self.expect(K::LBrace, "expected class body");
        let members = self.start();
        while !self.at_block_end() {
            if self.eat_optional_separators() && self.at_block_end() {
                break;
            }
            let member = self.start();
            let before = self.position();
            if self.at_enum_entry_start() {
                self.parse_enum_entry();
            } else {
                self.parse_class_member_declaration_or_statement();
            }
            if self.position() == before {
                self.recover_class_member();
                self.ensure_progress(before, "expected class member");
            }
            if self.at(K::Comma) {
                self.bump();
            }
            self.complete(member, K::ClassMemberDeclaration);
        }
        self.complete(members, K::ClassMemberList);
        self.expect(K::RBrace, "expected '}' after class body");
        self.complete(marker, K::ClassBody);
    }

    fn parse_primary_constructor(&mut self) {
        let marker = self.start();
        self.parse_modifier_list();
        if self.at_soft_keyword("constructor") {
            self.bump();
        }
        self.parse_value_parameter_list();
        self.complete(marker, K::PrimaryConstructor);
    }

    fn parse_enum_entry(&mut self) {
        let marker = self.start();
        self.parse_expression_until(&[K::Comma, K::Semicolon, K::DoubleSemicolon, K::RBrace]);
        self.complete(marker, K::EnumEntry);
    }

    fn at_primary_constructor_start(&mut self) -> bool {
        self.at(K::LParen)
            || self.at_soft_keyword("constructor")
            || (self.at_modifier_or_annotation()
                && self.nth_non_modifier_is_soft_keyword("constructor"))
    }

    fn at_enum_entry_start(&mut self) -> bool {
        self.at_identifier_like()
            && !self.at_soft_keyword("constructor")
            && !self.at_soft_keyword("init")
            && matches!(
                self.nth_kind(1),
                K::LParen | K::LBrace | K::Comma | K::Semicolon | K::DoubleSemicolon
            )
    }
}
