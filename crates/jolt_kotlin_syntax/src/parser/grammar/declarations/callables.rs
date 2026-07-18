use crate::KotlinSyntaxKind as K;

use super::super::support::is_identifier_like_kind;
use super::super::{Parser, StopSet};
use super::MAX_DECLARATION_LOOKAHEAD;

impl Parser<'_> {
    pub(in crate::parser::grammar) fn parse_secondary_constructor_tail(&mut self) {
        self.eat_soft_keyword_asserted("constructor");
        self.parse_value_parameter_list();
        if self.at(K::Colon) || matches!(self.current_kind(), K::ThisKw | K::SuperKw) {
            let delegation = self.start();
            if !self.eat(K::Colon) {
                let diagnostic =
                    self.pending_expected("expected ':' before constructor delegation");
                self.missing_required_slot(
                    delegation.anchor(),
                    crate::shape::constructor_delegation::Slot::colon as u16,
                    [diagnostic],
                );
            }
            let marker = self.start();
            if self.newline_before_current() && self.at_declaration_start(true)
                || matches!(
                    self.current_kind(),
                    K::LBrace | K::Semicolon | K::DoubleSemicolon | K::RBrace | K::Eof
                )
            {
                let diagnostic = self.pending_expected("expected constructor delegation call");
                self.missing_required_slot(
                    marker.anchor(),
                    crate::shape::constructor_delegation_call::Slot::expression as u16,
                    [diagnostic],
                );
            } else {
                self.parse_expression_until(&[
                    K::LBrace,
                    K::Semicolon,
                    K::DoubleSemicolon,
                    K::RBrace,
                ]);
            }
            self.complete(marker, K::ConstructorDelegationCall);
            self.complete(delegation, K::ConstructorDelegation);
        }
        if self.at(K::LBrace) {
            self.parse_block();
        }
    }

    pub(in crate::parser::grammar) fn parse_function_tail(&mut self) {
        self.eat_asserted(K::FunKw);
        if self.at(K::Lt) {
            self.parse_type_parameter_list();
        }
        self.parse_modifier_list();
        if !self.parse_callable_name_prefix(true) {
            let missing = self.start();
            let diagnostic = self.pending_expected("expected function name");

            self.complete_recovery(missing, K::BogusCallableDeclarationName, [diagnostic]);
        }
        if self.at(K::LParen) {
            self.parse_value_parameter_list();
        } else {
            self.complete_missing_value_parameter_list();
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
        } else if !self.parse_callable_name_prefix(false) {
            let missing = self.start();
            let diagnostic = self.pending_expected("expected property binding");

            self.complete_recovery(missing, K::BogusPropertyBinding, [diagnostic]);
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
        self.parse_property_initializer();
        let body_members = self.start();
        if self.at_soft_keyword("field") {
            let field = self.start();
            self.bump();
            if !self.eat(K::Assign) {
                let diagnostic = self.pending_expected("expected '=' after backing field");
                self.missing_required_slot(
                    field.anchor(),
                    crate::shape::explicit_backing_field::Slot::assign as u16,
                    [diagnostic],
                );
            }
            if self.at_property_accessor_start() || self.at_semicolon_boundary() {
                let diagnostic = self.pending_expected("expected backing field expression");
                self.missing_required_slot(
                    field.anchor(),
                    crate::shape::explicit_backing_field::Slot::expression as u16,
                    [diagnostic],
                );
            } else {
                self.parse_property_expression_until_accessor(false);
            }
            self.complete(field, K::ExplicitBackingField);
        }
        while self.at_property_accessor_start() {
            let before = self.position();
            self.parse_property_accessor();
            debug_assert!(self.position() > before);
        }
        self.complete(body_members, K::PropertyBodyMemberList);
    }

    fn parse_property_initializer(&mut self) {
        if self.at(K::Assign) || self.at_soft_keyword("by") {
            let initializer = self.start();
            self.bump();
            if self.at_property_accessor_start()
                || matches!(
                    self.current_kind(),
                    K::Semicolon | K::DoubleSemicolon | K::RBrace | K::Eof
                )
                || self.at_expression_rhs_declaration_boundary()
            {
                let diagnostic = self.pending_expected("expected property initializer expression");
                self.missing_required_slot(
                    initializer.anchor(),
                    crate::shape::property_initializer::Slot::expression as u16,
                    [diagnostic],
                );
            } else {
                self.parse_property_expression_until_accessor(true);
            }
            self.complete(initializer, K::PropertyInitializer);
            return;
        }

        if self.at_semicolon_boundary()
            || self.at_property_accessor_start()
            || self.at_soft_keyword("field")
            || self.at_expression_rhs_declaration_boundary()
        {
            return;
        }

        let initializer = self.start();
        let diagnostic = self.pending_expected("expected property initializer operator");
        self.missing_required_slot(
            initializer.anchor(),
            crate::shape::property_initializer::Slot::operator as u16,
            [diagnostic],
        );
        self.parse_property_expression_until_accessor(true);
        self.complete(initializer, K::PropertyInitializer);
    }

    pub(in crate::parser::grammar) fn parse_type_alias_tail(
        &mut self,
        declaration: jolt_syntax::NodeAnchor,
    ) {
        self.eat_asserted(K::TypeAliasKw);
        self.parse_name();
        if self.at(K::Lt) {
            self.parse_type_parameter_list();
        }
        if self.eat(K::Assign) {
            self.parse_type_reference_until(&[K::Semicolon, K::DoubleSemicolon, K::RBrace]);
        } else {
            let diagnostic = self.pending_expected("expected '=' in typealias");
            self.missing_required_slot(
                declaration,
                crate::shape::type_alias_declaration::Slot::assign as u16,
                [diagnostic],
            );
            if !self.at_semicolon_boundary() {
                self.parse_type_reference_until(&[K::Semicolon, K::DoubleSemicolon, K::RBrace]);
            }
        }
    }

    pub(in crate::parser::grammar) fn parse_type_parameter_list(&mut self) {
        let marker = self.start();
        self.eat_asserted(K::Lt);
        let entries = self.start();
        let mut expect_parameter = true;
        while !matches!(self.current_kind(), K::Gt | K::Eof) {
            let before = self.position();
            if self.at(K::Comma) {
                if expect_parameter && !matches!(self.nth_kind(1), K::Gt | K::Eof) {
                    let error = self.start();
                    let diagnostic =
                        self.pending_unexpected("expected type parameter between commas");

                    self.complete_recovery(error, K::BogusTypeParameter, [diagnostic]);
                }
                self.bump();
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
            let recovery = (!has_colon
                && !matches!(self.current_kind(), K::Comma | K::Gt | K::Eof))
            .then(|| self.pending_expected("expected ':' before type parameter bound"));
            if has_colon || !matches!(self.current_kind(), K::Comma | K::Gt | K::Eof) {
                self.parse_type_reference_until(&[K::Comma, K::Gt]);
            }
            if let Some(diagnostic) = recovery {
                self.complete_recovery(parameter, K::TypeParameter, [diagnostic]);
            } else {
                self.complete(parameter, K::TypeParameter);
            }
            expect_parameter = false;
            debug_assert!(self.position() > before);
        }
        self.complete(entries, K::TypeParameterSeparatedList);
        if !self.eat(K::Gt) {
            let diagnostic = self.pending_expected("expected '>' after type parameters");
            self.missing_required_slot(
                marker.anchor(),
                crate::shape::type_parameter_list::Slot::close_angle as u16,
                [diagnostic],
            );
        }
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
            let diagnostic = self.pending_expected("expected 'where' before type constraints");
            self.missing_required_slot(
                marker.anchor(),
                crate::shape::type_constraint_list::Slot::where_token as u16,
                [diagnostic],
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
                    let diagnostic = self.pending_expected("expected type constraint");

                    self.complete_recovery(bogus, K::BogusTypeConstraint, [diagnostic]);
                }
                break;
            }
            if self.at(K::Comma) {
                if expect_constraint {
                    let bogus = self.start();
                    let diagnostic =
                        self.pending_unexpected("expected type constraint between commas");

                    self.complete_recovery(bogus, K::BogusTypeConstraint, [diagnostic]);
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
                let diagnostic = self.pending_expected("expected ':' before type constraint bound");
                self.missing_required_slot(
                    constraint.anchor(),
                    crate::shape::type_constraint::Slot::colon as u16,
                    [diagnostic],
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
            debug_assert!(self.position() > before);
            if !self.at(K::Comma) {
                break;
            }
        }
        self.complete(entries, K::TypeConstraintSeparatedList);
        self.complete(marker, K::TypeConstraintList);
    }

    pub(in crate::parser::grammar) fn parse_context_parameter_clause(&mut self) {
        let marker = self.start();
        self.eat_soft_keyword_asserted("context");
        self.eat_asserted(K::LParen);
        let entries = self.start();
        let mut expect_parameter = true;
        while !matches!(self.current_kind(), K::RParen | K::Eof) {
            let before = self.position();
            if self.at(K::Comma) {
                if expect_parameter && !matches!(self.nth_kind(1), K::RParen | K::Eof) {
                    let bogus = self.start();
                    let diagnostic = self.pending_unexpected("expected context parameter");

                    self.complete_recovery(bogus, K::BogusContextParameter, [diagnostic]);
                }
                self.bump();
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
            let recovery = (!has_assign
                && !matches!(self.current_kind(), K::Comma | K::RParen | K::Eof))
            .then(|| self.pending_expected("expected '=' before context parameter default"));
            if has_assign || !matches!(self.current_kind(), K::Comma | K::RParen | K::Eof) {
                self.parse_expression_until(&[K::Comma, K::RParen]);
            }
            if let Some(diagnostic) = recovery {
                self.complete_recovery(parameter, K::ContextParameter, [diagnostic]);
            } else {
                self.complete(parameter, K::ContextParameter);
            }
            expect_parameter = false;
            debug_assert!(self.position() > before);
        }
        self.complete(entries, K::ContextParameterSeparatedList);
        if !self.eat(K::RParen) {
            let diagnostic = self.pending_expected("expected ')' after context parameters");
            self.missing_required_slot(
                marker.anchor(),
                crate::shape::context_parameter_clause::Slot::close_paren as u16,
                [diagnostic],
            );
        }
        self.complete(marker, K::ContextParameterClause);
    }

    pub(in crate::parser::grammar) fn parse_delegation_specifier_entries(&mut self) {
        let entries = self.start();
        let mut expect_specifier = true;
        loop {
            if matches!(
                self.current_kind(),
                K::WhereKw | K::LBrace | K::Semicolon | K::DoubleSemicolon | K::RBrace | K::Eof
            ) {
                if expect_specifier {
                    let bogus = self.start();
                    let diagnostic = self.pending_expected("expected delegation specifier");

                    self.complete_recovery(bogus, K::BogusDelegationSpecifier, [diagnostic]);
                }
                break;
            }
            if self.at(K::Comma) {
                if expect_specifier {
                    let bogus = self.start();
                    let diagnostic =
                        self.pending_unexpected("expected delegation specifier between commas");

                    self.complete_recovery(bogus, K::BogusDelegationSpecifier, [diagnostic]);
                }
                self.bump();
                expect_specifier = true;
                continue;
            }
            let specifier = self.start();
            self.parse_delegation_specifier();
            self.complete(specifier, K::DelegationSpecifier);
            expect_specifier = false;
            if !self.at(K::Comma) {
                break;
            }
        }
        self.complete(entries, K::DelegationSpecifierSeparatedList);
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
        if self.at_soft_keyword("by") {
            let by_clause = self.start();
            self.bump();
            if matches!(
                self.current_kind(),
                K::Comma | K::LBrace | K::Semicolon | K::DoubleSemicolon | K::RBrace | K::Eof
            ) {
                let diagnostic = self.pending_expected("expected delegation expression after 'by'");
                self.missing_required_slot(
                    by_clause.anchor(),
                    crate::shape::delegation_by_clause::Slot::delegate as u16,
                    [diagnostic],
                );
            } else {
                self.parse_expression_until(DELEGATION_STOPS);
            }
            self.complete(by_clause, K::DelegationByClause);
        }
    }

    pub(in crate::parser::grammar) fn parse_value_parameter_list(&mut self) {
        let marker = self.start();
        if !self.eat(K::LParen) {
            let diagnostic = self.pending_expected("expected value parameter list");
            self.missing_required_slot(
                marker.anchor(),
                crate::shape::value_parameter_list::Slot::open_paren as u16,
                [diagnostic],
            );
        }
        let entries = self.start();
        let mut expect_parameter = true;
        while !matches!(self.current_kind(), K::RParen | K::Eof) {
            let before = self.position();
            if self.at(K::Comma) {
                if expect_parameter && !matches!(self.nth_kind(1), K::RParen | K::Eof) {
                    let error = self.start();
                    let diagnostic =
                        self.pending_unexpected("expected value parameter between commas");

                    self.complete_recovery(error, K::BogusValueParameter, [diagnostic]);
                }
                self.bump();
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
            let recovery = (!has_assign
                && !matches!(self.current_kind(), K::Comma | K::RParen | K::Eof))
            .then(|| self.pending_expected("expected '=' before parameter default"));
            if has_assign || !matches!(self.current_kind(), K::Comma | K::RParen | K::Eof) {
                self.parse_expression_until(&[K::Comma, K::RParen]);
            }
            if let Some(diagnostic) = recovery {
                self.complete_recovery(parameter, K::ValueParameter, [diagnostic]);
            } else {
                self.complete(parameter, K::ValueParameter);
            }
            expect_parameter = false;
            debug_assert!(self.position() > before);
        }
        self.complete(entries, K::ValueParameterSeparatedList);
        if !self.eat(K::RParen) {
            let diagnostic = self.pending_expected("expected ')' after value parameters");
            self.missing_required_slot(
                marker.anchor(),
                crate::shape::value_parameter_list::Slot::close_paren as u16,
                [diagnostic],
            );
        }
        self.complete(marker, K::ValueParameterList);
    }

    pub(in crate::parser::grammar) fn parse_value_parameter_modifier_list(&mut self) {
        let modifiers = self.start();
        while self.at_modifier_or_annotation() && !self.at(K::VarargKw) {
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
            _ => unreachable!("destructuring parser requires an opening delimiter"),
        };
        let marker = self.start();
        self.eat_asserted(open);
        let entries = self.start();
        let mut expect_entry = true;
        while !matches!(self.current_kind(), K::Eof) && !self.at(close) {
            let before = self.position();
            if self.at(K::Comma) {
                let next = self.nth_kind(1);
                if expect_entry && next != close && next != K::Eof {
                    let error = self.start();
                    let diagnostic =
                        self.pending_unexpected("expected destructuring entry between commas");

                    self.complete_recovery(error, K::BogusDestructuringEntry, [diagnostic]);
                }
                self.bump();
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
            debug_assert!(self.position() > before);
        }
        self.complete(entries, K::DestructuringEntrySeparatedList);
        if !self.eat(close) {
            let diagnostic =
                self.pending_expected("expected closing delimiter after destructuring declaration");
            self.missing_required_slot(
                marker.anchor(),
                crate::shape::destructuring_declaration::Slot::close_delimiter as u16,
                [diagnostic],
            );
        }
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
            self.parse_property_accessor_body();
        } else {
            let diagnostic = self.pending_unexpected("expected property accessor");

            while !matches!(
                self.current_kind(),
                K::Semicolon | K::DoubleSemicolon | K::RBrace | K::Eof
            ) {
                self.bump();
            }
            self.complete_recovery(marker, K::BogusPropertyBodyMember, [diagnostic]);
            return;
        }
        self.complete(marker, K::PropertyAccessor);
    }

    fn parse_callable_name_prefix(&mut self, recover_missing_dot: bool) -> bool {
        if let Some(separator_position) = self.callable_receiver_separator_position() {
            let marker = self.start();
            self.parse_type_reference_until_position(separator_position);
            self.eat_asserted(K::Dot);
            self.parse_name();
            self.complete(marker, K::CallableName);
            true
        } else if recover_missing_dot
            && self.at_identifier_like()
            && is_identifier_like_kind(self.nth_kind(1))
            && self.callable_name_boundary_at(self.position() + 2, self.position())
        {
            let marker = self.start();
            self.parse_type_reference_until_position(self.position() + 1);
            let diagnostic = self.pending_expected("expected receiver separator");
            self.missing_required_slot(
                marker.anchor(),
                crate::shape::callable_name::Slot::dot as u16,
                [diagnostic],
            );
            self.parse_name();
            self.complete(marker, K::CallableName);
            true
        } else if self.at_identifier_like() {
            self.parse_name();
            true
        } else {
            false
        }
    }

    pub(in crate::parser::grammar) fn complete_missing_value_parameter_list(&mut self) {
        let list = self.start();
        let diagnostic = self.pending_expected("expected value parameter list");
        self.complete_recovery(list, K::ValueParameterList, [diagnostic]);
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
                        && ((is_identifier_like_kind(self.kind_at(index + 1))
                            && self.callable_name_boundary_at(index + 2, start))
                            || self.callable_name_boundary_at(index + 1, start)) =>
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
        if self.at(K::Assign) {
            let body = self.start();
            self.parse_expression_body_after_assign(body, false);
        } else if self.at(K::LBrace) {
            let body = self.start();
            self.parse_block();
            self.complete(body, K::BlockBody);
        }
    }

    fn parse_property_accessor_body(&mut self) {
        if self.at(K::LBrace) {
            let block = self.start();
            self.parse_block();
            let block = self.complete(block, K::BlockBody);
            if self.at(K::Assign) {
                let combined = self.precede(block);
                let diagnostic = self
                    .pending_unexpected("property accessor has both block and expression bodies");
                let expression = self.start();
                self.parse_expression_body_after_assign(expression, true);

                self.complete_recovery(combined, K::BogusDeclarationBody, [diagnostic]);
            }
            return;
        }

        if self.at(K::Assign) {
            let body = self.start();
            self.parse_expression_body_after_assign(body, true);
            return;
        }

        let position = self.position();
        let accessor_keyword = self.property_accessor_keyword_position(position);
        let separated_expression = accessor_keyword
            .is_some_and(|keyword| keyword > position && self.newline_between(position, keyword));
        if (accessor_keyword.is_none() || separated_expression)
            && !self.at_expression_rhs_declaration_boundary()
            && !matches!(
                self.current_kind(),
                K::Semicolon | K::DoubleSemicolon | K::RBrace | K::Eof
            )
        {
            let body = self.start();
            let diagnostic =
                self.pending_expected("expected '=' before property accessor expression");
            self.missing_required_slot(
                body.anchor(),
                crate::shape::expression_body::Slot::assign as u16,
                [diagnostic],
            );
            self.parse_property_expression_until_accessor(false);
            self.complete(body, K::ExpressionBody);
        }
    }

    fn at_property_accessor_start(&mut self) -> bool {
        self.property_accessor_keyword_position(self.position())
            .is_some()
    }

    fn parse_property_expression_until_accessor(&mut self, include_where: bool) {
        const STOPS: &[K] = &[
            K::Semicolon,
            K::DoubleSemicolon,
            K::RBrace,
            K::GetKw,
            K::SetKw,
        ];
        const STOPS_WITH_WHERE: &[K] = &[
            K::WhereKw,
            K::Semicolon,
            K::DoubleSemicolon,
            K::RBrace,
            K::GetKw,
            K::SetKw,
        ];
        let stops = if include_where {
            StopSet::new(STOPS_WITH_WHERE)
        } else {
            StopSet::new(STOPS)
        }
        .with_position(self.next_property_accessor_start_position());
        self.parse_expression_until(stops);
    }

    fn parse_expression_body_after_assign(
        &mut self,
        body: jolt_syntax::Marker,
        accessor: bool,
    ) -> jolt_syntax::CompletedMarker {
        self.bump();
        let missing_expression = matches!(
            self.current_kind(),
            K::Semicolon | K::DoubleSemicolon | K::RBrace | K::Eof
        ) || accessor && self.at_property_accessor_start()
            || self.at_expression_rhs_declaration_boundary();
        if missing_expression {
            let diagnostic = self.pending_expected("expected declaration body expression");
            self.missing_required_slot(
                body.anchor(),
                crate::shape::expression_body::Slot::expression as u16,
                [diagnostic],
            );
        } else if accessor {
            self.parse_property_expression_until_accessor(false);
        } else {
            self.parse_expression_until(&[K::Semicolon, K::DoubleSemicolon, K::RBrace]);
        }
        self.complete(body, K::ExpressionBody)
    }

    fn property_accessor_keyword_position(&mut self, index: usize) -> Option<usize> {
        if self.is_soft_kind_at(index, "get") || self.is_soft_kind_at(index, "set") {
            return Some(index);
        }
        if !self.is_modifier_or_annotation_start_at(index) {
            return None;
        }
        let after_prefix = self.skip_modifier_prefix(index)?;
        (self.is_soft_kind_at(after_prefix, "get") || self.is_soft_kind_at(after_prefix, "set"))
            .then_some(after_prefix)
    }

    fn next_property_accessor_start_position(&mut self) -> Option<usize> {
        let start = self.position();
        for index in start + 1..start + MAX_DECLARATION_LOOKAHEAD {
            if matches!(
                self.kind_at(index),
                K::Semicolon | K::DoubleSemicolon | K::RBrace | K::Eof
            ) {
                return None;
            }
            if self.newline_between(start, index)
                && self.property_accessor_keyword_position(index).is_some()
            {
                return Some(index);
            }
        }
        None
    }
}
