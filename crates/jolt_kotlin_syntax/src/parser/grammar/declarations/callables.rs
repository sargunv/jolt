use jolt_syntax::UnresolvedDiagnosticOwner;

use crate::KotlinSyntaxKind as K;

use super::super::Parser;
use super::super::support::is_identifier_like_kind;
use super::MAX_DECLARATION_LOOKAHEAD;

impl Parser<'_> {
    pub(in crate::parser::grammar) fn parse_secondary_constructor_tail(&mut self) {
        self.expect_soft_keyword("constructor", "expected constructor");
        self.parse_value_parameter_list();
        if self.eat(K::Colon) {
            let marker = self.start();
            self.parse_expression_until(&[K::LBrace, K::Semicolon, K::DoubleSemicolon, K::RBrace]);
            self.complete(marker, K::ConstructorDelegationCall);
        }
        if self.at(K::LBrace) {
            self.parse_block();
        }
    }

    pub(in crate::parser::grammar) fn parse_function_tail(&mut self) {
        self.expect(K::FunKw, "expected fun");
        if self.at(K::Lt) {
            self.parse_type_parameter_list();
        }
        self.parse_modifier_list();
        self.parse_callable_name_prefix();
        if self.at(K::LParen) {
            self.parse_value_parameter_list();
        }
        if self.eat(K::Colon) {
            self.parse_type_reference_until(&[
                K::WhereKw,
                K::Assign,
                K::LBrace,
                K::Semicolon,
                K::DoubleSemicolon,
                K::RBrace,
            ]);
        }
        self.parse_type_constraint_list();
        self.parse_optional_body();
    }

    pub(in crate::parser::grammar) fn parse_property_tail(&mut self) {
        self.bump();
        if self.at(K::Lt) {
            self.parse_type_parameter_list();
        }
        if self.at_destructuring_declaration_start() {
            self.parse_destructuring_declaration();
        } else {
            self.parse_callable_name_prefix();
        }
        if self.eat(K::Colon) {
            self.parse_type_reference_until(&[
                K::WhereKw,
                K::Assign,
                K::LBrace,
                K::Semicolon,
                K::DoubleSemicolon,
                K::RBrace,
                K::GetKw,
                K::SetKw,
            ]);
        }
        self.parse_type_constraint_list();
        if self.eat(K::Assign) || self.eat_soft_keyword("by") {
            self.parse_expression_until(&[
                K::WhereKw,
                K::Semicolon,
                K::DoubleSemicolon,
                K::RBrace,
                K::GetKw,
                K::SetKw,
            ]);
        }
        let body_members = self.start();
        if self.at_soft_keyword("field") && self.nth_kind(1) == K::Assign {
            let field = self.start();
            self.bump();
            self.bump();
            self.parse_expression_until(&[
                K::Semicolon,
                K::DoubleSemicolon,
                K::RBrace,
                K::GetKw,
                K::SetKw,
            ]);
            self.complete(field, K::ExplicitBackingField);
        }
        while self.at_property_accessor_start() {
            let before = self.position();
            self.parse_property_accessor();
            self.ensure_progress(before, "expected property accessor");
        }
        self.complete(body_members, K::PropertyBodyMemberList);
    }

    pub(in crate::parser::grammar) fn parse_type_alias_tail(&mut self) {
        self.expect(K::TypeAliasKw, "expected typealias");
        self.parse_name();
        if self.at(K::Lt) {
            self.parse_type_parameter_list();
        }
        if self.eat(K::Assign) {
            self.parse_type_reference_until(&[K::Semicolon, K::DoubleSemicolon, K::RBrace]);
        } else {
            self.expected_here("expected '=' in typealias");
        }
    }

    pub(in crate::parser::grammar) fn parse_type_parameter_list(&mut self) {
        let marker = self.start();
        self.expect(K::Lt, "expected type parameter list");
        let entries = self.start();
        let mut expect_parameter = true;
        while !matches!(self.current_kind(), K::Gt | K::Eof) {
            let before = self.position();
            if self.eat(K::Comma) {
                if expect_parameter && !matches!(self.current_kind(), K::Gt | K::Eof) {
                    let error = self.start();
                    let diagnostic = self.unexpected_here("expected type parameter between commas");
                    self.own_diagnostic(
                        diagnostic,
                        UnresolvedDiagnosticOwner::node(error.anchor()),
                    );
                    self.complete(error, K::BogusTypeParameter);
                }
                expect_parameter = true;
                continue;
            }
            let parameter = self.start();
            self.parse_modifier_list();
            if self.at(K::InKw) {
                self.bump();
            }
            self.parse_name();
            let has_colon = self.eat(K::Colon);
            if has_colon || !matches!(self.current_kind(), K::Comma | K::Gt | K::Eof) {
                if !has_colon {
                    let diagnostic = self.expected_here("expected ':' before type parameter bound");
                    self.own_diagnostic(
                        diagnostic,
                        UnresolvedDiagnosticOwner::missing_slot(
                            parameter.anchor(),
                            crate::shape::type_parameter::Slot::colon as u16,
                        ),
                    );
                }
                self.parse_type_reference_until(&[K::Comma, K::Gt]);
            }
            self.complete(parameter, K::TypeParameter);
            expect_parameter = false;
            self.ensure_progress(before, "expected type parameter");
        }
        self.complete(entries, K::TypeParameterSeparatedList);
        self.expect(K::Gt, "expected '>' after type parameters");
        self.complete(marker, K::TypeParameterList);
    }

    pub(in crate::parser::grammar) fn parse_type_constraint_list(&mut self) {
        let has_where = self.at_soft_keyword("where");
        let has_recovered_constraint =
            is_identifier_like_kind(self.current_kind()) && self.nth_kind(1) == K::Colon;
        if !has_where && !has_recovered_constraint {
            return;
        }
        let marker = self.start();
        if has_where {
            self.bump();
        } else {
            let diagnostic = self.expected_here("expected 'where' before type constraints");
            self.own_diagnostic(
                diagnostic,
                UnresolvedDiagnosticOwner::missing_slot(
                    marker.anchor(),
                    crate::shape::type_constraint_list::Slot::where_token as u16,
                ),
            );
        }
        let entries = self.start();
        let mut expect_constraint = true;
        loop {
            if matches!(
                self.current_kind(),
                K::Assign | K::LBrace | K::Semicolon | K::DoubleSemicolon | K::RBrace | K::Eof
            ) {
                if expect_constraint {
                    let bogus = self.start();
                    let diagnostic = self.expected_here("expected type constraint");
                    self.own_diagnostic(
                        diagnostic,
                        UnresolvedDiagnosticOwner::node(bogus.anchor()),
                    );
                    self.complete(bogus, K::BogusTypeConstraint);
                }
                break;
            }
            if self.at(K::Comma) {
                if expect_constraint {
                    let bogus = self.start();
                    let diagnostic =
                        self.unexpected_here("expected type constraint between commas");
                    self.own_diagnostic(
                        diagnostic,
                        UnresolvedDiagnosticOwner::node(bogus.anchor()),
                    );
                    self.complete(bogus, K::BogusTypeConstraint);
                }
                self.bump();
                expect_constraint = true;
                continue;
            }
            let before = self.position();
            let constraint = self.start();
            self.parse_name();
            let has_colon = self.eat(K::Colon);
            if !has_colon {
                let diagnostic = self.expected_here("expected ':' before type constraint bound");
                self.own_diagnostic(
                    diagnostic,
                    UnresolvedDiagnosticOwner::missing_slot(
                        constraint.anchor(),
                        crate::shape::type_constraint::Slot::colon as u16,
                    ),
                );
            }
            if has_colon
                || !matches!(
                    self.current_kind(),
                    K::Comma
                        | K::Assign
                        | K::LBrace
                        | K::Semicolon
                        | K::DoubleSemicolon
                        | K::RBrace
                        | K::Eof
                )
            {
                self.parse_type_reference_until(&[
                    K::Comma,
                    K::Assign,
                    K::LBrace,
                    K::Semicolon,
                    K::DoubleSemicolon,
                    K::RBrace,
                ]);
            }
            self.complete(constraint, K::TypeConstraint);
            expect_constraint = false;
            if self.position() == before {
                self.unexpected_here("expected type constraint");
                break;
            }
            if !self.at(K::Comma) {
                break;
            }
        }
        self.complete(entries, K::TypeConstraintSeparatedList);
        self.complete(marker, K::TypeConstraintList);
    }

    pub(in crate::parser::grammar) fn parse_context_parameter_clause(&mut self) {
        let marker = self.start();
        self.expect_soft_keyword("context", "expected context");
        self.expect(K::LParen, "expected '(' after context");
        let entries = self.start();
        let mut expect_parameter = true;
        while !matches!(self.current_kind(), K::RParen | K::Eof) {
            let before = self.position();
            if self.eat(K::Comma) {
                if expect_parameter && !matches!(self.current_kind(), K::RParen | K::Eof) {
                    let bogus = self.start();
                    let diagnostic = self.unexpected_here("expected context parameter");
                    self.own_diagnostic(
                        diagnostic,
                        UnresolvedDiagnosticOwner::node(bogus.anchor()),
                    );
                    self.complete(bogus, K::BogusContextParameter);
                }
                expect_parameter = true;
                continue;
            }
            let parameter = self.start();
            if self.at_identifier_like() && self.nth_kind(1) == K::Colon {
                self.parse_name();
                self.bump();
            }
            self.parse_type_reference_until(&[K::Comma, K::RParen, K::Assign]);
            let has_assign = self.eat(K::Assign);
            if has_assign || !matches!(self.current_kind(), K::Comma | K::RParen | K::Eof) {
                if !has_assign {
                    let diagnostic =
                        self.expected_here("expected '=' before context parameter default");
                    self.own_diagnostic(
                        diagnostic,
                        UnresolvedDiagnosticOwner::missing_slot(
                            parameter.anchor(),
                            crate::shape::context_parameter::Slot::assign as u16,
                        ),
                    );
                }
                self.parse_expression_until(&[K::Comma, K::RParen]);
            }
            self.complete(parameter, K::ContextParameter);
            expect_parameter = false;
            self.ensure_progress(before, "expected context parameter");
        }
        self.complete(entries, K::ContextParameterSeparatedList);
        self.expect(K::RParen, "expected ')' after context parameters");
        self.complete(marker, K::ContextParameterClause);
    }

    pub(in crate::parser::grammar) fn parse_delegation_specifier_list(&mut self) {
        let marker = self.start();
        let entries = self.start();
        loop {
            let before = self.position();
            let specifier = self.start();
            self.parse_delegation_specifier();
            self.complete(specifier, K::DelegationSpecifier);
            self.ensure_progress(before, "expected delegation specifier");
            if !self.eat(K::Comma) {
                break;
            }
        }
        self.complete(entries, K::DelegationSpecifierSeparatedList);
        self.complete(marker, K::DelegationSpecifierList);
    }

    fn parse_delegation_specifier(&mut self) {
        const DELEGATION_STOPS: &[K] = &[
            K::Comma,
            K::WhereKw,
            K::LBrace,
            K::Semicolon,
            K::DoubleSemicolon,
            K::RBrace,
        ];

        self.parse_type_reference_until(&[
            K::ByKw,
            K::Comma,
            K::WhereKw,
            K::LBrace,
            K::Semicolon,
            K::DoubleSemicolon,
            K::RBrace,
        ]);
        if self.at(K::LParen) {
            self.parse_value_argument_list();
        }
        if self.eat_soft_keyword("by") {
            self.parse_expression_until(DELEGATION_STOPS);
        }
    }

    pub(in crate::parser::grammar) fn parse_value_parameter_list(&mut self) {
        let marker = self.start();
        self.expect(K::LParen, "expected value parameter list");
        let entries = self.start();
        let mut expect_parameter = true;
        while !matches!(self.current_kind(), K::RParen | K::Eof) {
            let before = self.position();
            if self.eat(K::Comma) {
                if expect_parameter && !matches!(self.current_kind(), K::RParen | K::Eof) {
                    let error = self.start();
                    let diagnostic =
                        self.unexpected_here("expected value parameter between commas");
                    self.own_diagnostic(
                        diagnostic,
                        UnresolvedDiagnosticOwner::node(error.anchor()),
                    );
                    self.complete(error, K::BogusValueParameter);
                }
                expect_parameter = true;
                continue;
            }
            let parameter = self.start();
            self.parse_value_parameter_modifier_list();
            if self.at(K::ValKw) || self.at(K::VarKw) || self.at(K::VarargKw) {
                self.bump();
            }
            self.parse_name_or_destructuring();
            if self.eat(K::Colon) {
                self.parse_type_reference_until(&[K::Comma, K::RParen, K::Assign]);
            }
            let has_assign = self.eat(K::Assign);
            if has_assign || !matches!(self.current_kind(), K::Comma | K::RParen | K::Eof) {
                if !has_assign {
                    let diagnostic = self.expected_here("expected '=' before parameter default");
                    self.own_diagnostic(
                        diagnostic,
                        UnresolvedDiagnosticOwner::missing_slot(
                            parameter.anchor(),
                            crate::shape::value_parameter::Slot::assign as u16,
                        ),
                    );
                }
                self.parse_expression_until(&[K::Comma, K::RParen]);
            }
            self.complete(parameter, K::ValueParameter);
            expect_parameter = false;
            self.ensure_progress(before, "expected value parameter");
        }
        self.complete(entries, K::ValueParameterSeparatedList);
        self.expect(K::RParen, "expected ')' after value parameters");
        self.complete(marker, K::ValueParameterList);
    }

    fn parse_value_parameter_modifier_list(&mut self) {
        let modifiers = self.start();
        while self.at_modifier_or_annotation() && !self.at(K::VarargKw) {
            let before = self.position();
            if self.at(K::At) || self.at(K::Hash) {
                self.parse_annotation();
            } else {
                self.bump();
            }
            self.ensure_progress(before, "expected value parameter modifier or annotation");
        }
        self.complete(modifiers, K::ModifierList);
    }

    pub(in crate::parser::grammar) fn parse_name_or_destructuring(&mut self) {
        if self.at(K::LParen) {
            self.parse_destructuring_declaration();
        } else {
            self.parse_name();
        }
    }

    pub(in crate::parser::grammar) fn parse_destructuring_declaration(&mut self) {
        let (open, close) = match self.current_kind() {
            K::LParen => (K::LParen, K::RParen),
            K::LBracket => (K::LBracket, K::RBracket),
            _ => {
                self.expected_here("expected destructuring declaration");
                return;
            }
        };
        let marker = self.start();
        self.expect(open, "expected destructuring declaration");
        let entries = self.start();
        let mut expect_entry = true;
        while !matches!(self.current_kind(), K::Eof) && !self.at(close) {
            let before = self.position();
            if self.eat(K::Comma) {
                if expect_entry && !self.at(close) && !self.at(K::Eof) {
                    let error = self.start();
                    let diagnostic =
                        self.unexpected_here("expected destructuring entry between commas");
                    self.own_diagnostic(
                        diagnostic,
                        UnresolvedDiagnosticOwner::node(error.anchor()),
                    );
                    self.complete(error, K::BogusDestructuringEntry);
                }
                expect_entry = true;
                continue;
            }
            let entry = self.start();
            if self.at(K::ValKw) || self.at(K::VarKw) {
                self.bump();
            }
            self.parse_name();
            if self.eat(K::Assign) {
                self.parse_expression_until(&[K::Comma, close]);
            }
            if self.eat(K::Colon) {
                self.parse_type_reference_until(&[K::Comma, close]);
            }
            self.complete(entry, K::DestructuringEntry);
            expect_entry = false;
            self.ensure_progress(before, "expected destructuring entry");
        }
        self.complete(entries, K::DestructuringEntrySeparatedList);
        self.expect(
            close,
            "expected closing delimiter after destructuring declaration",
        );
        self.complete(marker, K::DestructuringDeclaration);
    }

    pub(in crate::parser::grammar) fn at_destructuring_declaration_start(&mut self) -> bool {
        matches!(self.current_kind(), K::LParen | K::LBracket)
    }

    pub(in crate::parser::grammar) fn at_context_parameter_clause(&mut self) -> bool {
        self.at_soft_keyword("context") && self.nth_kind(1) == K::LParen
    }

    fn parse_property_accessor(&mut self) {
        let marker = self.start();
        self.parse_modifier_list();
        if self.eat_soft_keyword("get") || self.eat_soft_keyword("set") {
            if self.at(K::LParen) {
                self.parse_value_parameter_list();
            }
            if self.eat(K::Colon) {
                self.parse_type_reference_until(&[
                    K::Assign,
                    K::LBrace,
                    K::Semicolon,
                    K::DoubleSemicolon,
                    K::RBrace,
                    K::GetKw,
                    K::SetKw,
                ]);
            }
            self.parse_optional_body();
        } else {
            self.unexpected_here("expected property accessor");
            self.recover_declaration();
        }
        self.complete(marker, K::PropertyAccessor);
    }

    fn parse_callable_name_prefix(&mut self) {
        let marker = self.start();

        if let Some(separator_position) = self.callable_receiver_separator_position() {
            let parts = self.start();
            self.parse_type_reference_until_position(separator_position);
            self.expect(K::Dot, "expected receiver separator");
            self.parse_name();
            self.complete(parts, K::CallableNamePartList);
            self.complete(marker, K::CallableName);
        } else if self.at_identifier_like() {
            let parts = self.start();
            self.parse_name();
            self.complete(parts, K::CallableNamePartList);
            self.complete(marker, K::CallableName);
        } else {
            self.abandon(marker);
        }
    }

    fn callable_receiver_separator_position(&mut self) -> Option<usize> {
        let mut paren_depth = 0usize;
        let mut bracket_depth = 0usize;
        let mut brace_depth = 0usize;
        let mut angle_depth = 0usize;
        let mut separator = None;

        for offset in 0..MAX_DECLARATION_LOOKAHEAD {
            let index = self.position() + offset;
            let start = self.position();
            let kind = self.kind_at(index);
            let at_top_level =
                paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 && angle_depth == 0;

            if at_top_level && self.callable_name_boundary_at(index, start) {
                break;
            }
            if kind == K::Eof {
                break;
            }

            match kind {
                K::LParen => paren_depth += 1,
                K::RParen if paren_depth > 0 => paren_depth -= 1,
                K::LBracket => bracket_depth += 1,
                K::RBracket if bracket_depth > 0 => bracket_depth -= 1,
                K::LBrace => brace_depth += 1,
                K::RBrace if brace_depth > 0 => brace_depth -= 1,
                K::RParen | K::RBracket | K::RBrace => break,
                K::Lt => angle_depth += 1,
                K::Gt if angle_depth > 0 => angle_depth -= 1,
                K::Dot
                    if at_top_level
                        && is_identifier_like_kind(self.kind_at(index + 1))
                        && self.callable_name_boundary_at(index + 2, start) =>
                {
                    separator = Some(index);
                }
                _ => {}
            }
        }

        separator
    }

    fn callable_name_boundary_at(&mut self, index: usize, start: usize) -> bool {
        let kind = self.kind_at(index);
        matches!(
            kind,
            K::LParen
                | K::Colon
                | K::Assign
                | K::WhereKw
                | K::LBrace
                | K::Semicolon
                | K::DoubleSemicolon
                | K::RBrace
                | K::Eof
        ) || index > start
            && self.kind_at(index - 1) != K::Dot
            && matches!(self.text_at(index), Some("by" | "get" | "set"))
    }

    fn parse_optional_body(&mut self) {
        if self.eat(K::Assign) {
            self.parse_expression_until(&[K::Semicolon, K::DoubleSemicolon, K::RBrace]);
        } else if self.at(K::LBrace) {
            self.parse_block();
        }
    }

    fn at_property_accessor_start(&mut self) -> bool {
        self.at_soft_keyword("get")
            || self.at_soft_keyword("set")
            || (self.at_modifier_or_annotation()
                && (self.nth_non_modifier_is_soft_keyword("get")
                    || self.nth_non_modifier_is_soft_keyword("set")))
    }
}
