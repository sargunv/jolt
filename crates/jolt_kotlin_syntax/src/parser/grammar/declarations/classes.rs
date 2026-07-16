use jolt_syntax::UnresolvedDiagnosticOwner;

use crate::KotlinSyntaxKind as K;

use super::super::Parser;
use super::super::support::is_identifier_like_kind;

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
        let recovered_delegation = !self.at(K::Colon)
            && is_identifier_like_kind(self.current_kind())
            && matches!(self.nth_kind(1), K::LParen | K::ByKw | K::Comma);
        if self.at(K::Colon) || recovered_delegation {
            self.parse_delegation_clause(recovered_delegation);
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
        let recovered_delegation = !self.at(K::Colon)
            && is_identifier_like_kind(self.current_kind())
            && matches!(self.nth_kind(1), K::LParen | K::ByKw | K::Comma);
        if self.at(K::Colon) || recovered_delegation {
            self.parse_delegation_clause(recovered_delegation);
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
            if self.at(K::Comma) {
                let member = self.start();
                let diagnostic = self.unexpected_here("unexpected orphan class member comma");
                self.own_diagnostic(diagnostic, UnresolvedDiagnosticOwner::node(member.anchor()));
                self.bump();
                self.complete(member, K::BogusClassMember);
                continue;
            }
            let before = self.position();
            if self.at_enum_entry_start() {
                self.parse_enum_entry();
            } else if self.at(K::Plus)
                && matches!(
                    self.nth_kind(1),
                    K::Semicolon | K::DoubleSemicolon | K::RBrace | K::Eof
                )
            {
                let member = self.start();
                let diagnostic = self.unexpected_here("unexpected orphan class member");
                self.own_diagnostic(diagnostic, UnresolvedDiagnosticOwner::node(member.anchor()));
                self.bump();
                self.complete(member, K::BogusClassMember);
            } else {
                let member = self.start();
                self.parse_class_member_declaration_or_statement();
                if self.position() == before {
                    let diagnostic = self.unexpected_here("expected class member");
                    self.own_diagnostic(
                        diagnostic,
                        UnresolvedDiagnosticOwner::node(member.anchor()),
                    );
                    if !self.at_block_end() {
                        self.bump();
                    }
                    self.complete(member, K::BogusClassMember);
                } else {
                    self.complete(member, K::ClassMemberDeclaration);
                }
            }
        }
        self.complete(members, K::ClassMemberList);
        if !self.eat(K::RBrace) {
            let diagnostic = self.expected_here("expected '}' after class body");
            self.own_diagnostic(
                diagnostic,
                UnresolvedDiagnosticOwner::missing_slot(
                    marker.anchor(),
                    crate::shape::class_body::Slot::close_brace as u16,
                ),
            );
        }
        self.complete(marker, K::ClassBody);
    }

    fn parse_primary_constructor(&mut self) {
        let marker = self.start();
        self.parse_modifier_list();
        if self.at_soft_keyword("constructor") {
            self.bump();
        }
        if self.at(K::LParen) {
            self.parse_value_parameter_list();
        } else {
            self.complete_missing_value_parameter_list();
        }
        self.complete(marker, K::PrimaryConstructor);
    }

    fn parse_enum_entry(&mut self) {
        let marker = self.start();
        self.parse_modifier_list();
        if self.at_identifier_like() {
            self.parse_name();
        } else {
            let name = self.start();
            let diagnostic = self.expected_here("expected enum entry name");
            self.own_diagnostic(diagnostic, UnresolvedDiagnosticOwner::node(marker.anchor()));
            self.complete(name, K::Name);
            self.bump();
        }
        if self.at(K::LParen) {
            self.parse_value_argument_list();
        }
        if self.at(K::LBrace) {
            self.parse_class_body();
        }
        let _ = self.eat(K::Comma);
        self.complete(marker, K::EnumEntry);
    }

    fn parse_delegation_clause(&mut self, recovered: bool) {
        let clause = self.start();
        if !self.eat(K::Colon) {
            debug_assert!(recovered);
            let diagnostic = self.expected_here("expected ':' before delegation specifiers");
            self.own_diagnostic(
                diagnostic,
                UnresolvedDiagnosticOwner::missing_slot(
                    clause.anchor(),
                    crate::shape::delegation_clause::Slot::colon as u16,
                ),
            );
        }
        self.parse_delegation_specifier_entries();
        self.complete(clause, K::DelegationClause);
    }

    fn at_primary_constructor_start(&mut self) -> bool {
        self.at(K::LParen)
            || self.at_soft_keyword("constructor")
            || (self.at_modifier_or_annotation()
                && self.nth_non_modifier_is_soft_keyword("constructor"))
    }

    fn at_enum_entry_start(&mut self) -> bool {
        let start = self.position();
        let entry = if self.is_modifier_or_annotation_start_at(start) {
            let Some(entry) = self.skip_modifier_prefix(start) else {
                return false;
            };
            entry
        } else {
            start
        };
        (is_identifier_like_kind(self.kind_at(entry))
            && !self.is_soft_kind_at(entry, "constructor")
            && !self.is_soft_kind_at(entry, "init")
            && matches!(
                self.kind_at(entry + 1),
                K::LParen | K::LBrace | K::Comma | K::Semicolon | K::DoubleSemicolon | K::RBrace
            ))
            || (self.at(K::RParen) && self.nth_kind(1) == K::Comma)
    }
}
