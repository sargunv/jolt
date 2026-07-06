use crate::KotlinSyntaxKind as K;

use super::super::Parser;

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
        if self.eat(K::Assign) {
            self.parse_expression_until(&[
                K::Semicolon,
                K::DoubleSemicolon,
                K::RBrace,
                K::GetKw,
                K::SetKw,
            ]);
        }
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
            self.parse_property_accessor();
        }
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
        while !matches!(self.current_kind(), K::Gt | K::Eof) {
            let before = self.position();
            if self.eat(K::Comma) {
                continue;
            }
            let parameter = self.start();
            self.parse_modifier_list();
            self.parse_name();
            if self.eat(K::Colon) {
                self.parse_type_reference_until(&[K::Comma, K::Gt]);
            }
            self.complete(parameter, K::TypeParameter);
            if self.position() == before {
                self.unexpected_here("expected type parameter");
                self.bump();
            }
        }
        self.expect(K::Gt, "expected '>' after type parameters");
        self.complete(marker, K::TypeParameterList);
    }

    pub(in crate::parser::grammar) fn parse_type_constraint_list(&mut self) {
        if !self.at_soft_keyword("where") {
            return;
        }
        let marker = self.start();
        self.bump();
        loop {
            let constraint = self.start();
            self.parse_name();
            if self.eat(K::Colon) {
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
            if !self.eat(K::Comma) {
                break;
            }
        }
        self.complete(marker, K::TypeConstraintList);
    }

    pub(in crate::parser::grammar) fn parse_context_parameter_clause(&mut self) {
        let marker = self.start();
        self.expect_soft_keyword("context", "expected context");
        self.expect(K::LParen, "expected '(' after context");
        while !matches!(self.current_kind(), K::RParen | K::Eof) {
            let before = self.position();
            if self.eat(K::Comma) {
                continue;
            }
            let parameter = self.start();
            self.parse_name();
            if self.eat(K::Colon) {
                self.parse_type_reference_until(&[K::Comma, K::RParen]);
            }
            self.complete(parameter, K::ContextParameter);
            self.ensure_progress(before, "expected context parameter");
        }
        self.expect(K::RParen, "expected ')' after context parameters");
        self.complete(marker, K::ContextParameterClause);
    }

    pub(in crate::parser::grammar) fn parse_delegation_specifier_list(&mut self) {
        let marker = self.start();
        loop {
            let specifier = self.start();
            self.parse_expression_until(&[
                K::Comma,
                K::WhereKw,
                K::LBrace,
                K::Semicolon,
                K::DoubleSemicolon,
                K::RBrace,
            ]);
            self.complete(specifier, K::DelegationSpecifier);
            if !self.eat(K::Comma) {
                break;
            }
        }
        self.complete(marker, K::DelegationSpecifierList);
    }

    pub(in crate::parser::grammar) fn parse_value_parameter_list(&mut self) {
        let marker = self.start();
        self.expect(K::LParen, "expected value parameter list");
        while !matches!(self.current_kind(), K::RParen | K::Eof) {
            let before = self.position();
            if self.eat(K::Comma) {
                continue;
            }
            let parameter = self.start();
            self.parse_modifier_list();
            if self.at(K::ValKw) || self.at(K::VarKw) || self.at(K::VarargKw) {
                self.bump();
            }
            self.parse_name_or_destructuring();
            if self.eat(K::Colon) {
                self.parse_type_reference_until(&[K::Comma, K::RParen, K::Assign]);
            }
            if self.eat(K::Assign) {
                self.parse_expression_until(&[K::Comma, K::RParen]);
            }
            self.complete(parameter, K::ValueParameter);
            if self.position() == before {
                self.unexpected_here("expected value parameter");
                self.bump();
            }
        }
        self.expect(K::RParen, "expected ')' after value parameters");
        self.complete(marker, K::ValueParameterList);
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
        while !matches!(self.current_kind(), K::Eof) && !self.at(close) {
            let before = self.position();
            if self.eat(K::Comma) {
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
            if self.position() == before {
                self.unexpected_here("expected destructuring entry");
                self.bump();
            }
        }
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
        while !matches!(
            self.current_kind(),
            K::LParen
                | K::Colon
                | K::Assign
                | K::WhereKw
                | K::LBrace
                | K::Semicolon
                | K::DoubleSemicolon
                | K::RBrace
                | K::Eof
        ) {
            if self.at(K::Lt) {
                self.parse_type_argument_list();
            } else if self.at_identifier_like()
                || matches!(self.current_kind(), K::Dot | K::Question)
            {
                self.bump();
            } else {
                break;
            }
        }
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
