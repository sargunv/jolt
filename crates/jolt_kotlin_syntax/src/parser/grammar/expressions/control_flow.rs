use jolt_syntax::CompletedMarker;

use crate::KotlinSyntaxKind as K;

use super::super::{Parser, StopSet};

const MAX_ANONYMOUS_FUNCTION_RECEIVER_LOOKAHEAD: usize = 128;

impl Parser<'_> {
    pub(super) fn parse_if_expression(&mut self, stops: StopSet<'_>) -> CompletedMarker {
        let marker = self.start();
        self.expect(K::IfKw, "expected if");
        if self.at(K::LParen) {
            self.parse_parenthesized_expression();
        }
        if self.at(K::LBrace) {
            self.parse_block();
        } else {
            self.parse_expression_until(stops.with_extra(K::ElseKw));
        }
        if self.eat(K::ElseKw) {
            if self.at(K::LBrace) {
                self.parse_block();
            } else {
                self.parse_expression_until(stops);
            }
        }
        self.complete(marker, K::IfExpression)
    }

    pub(super) fn parse_when_expression(&mut self) -> CompletedMarker {
        let marker = self.start();
        self.expect(K::WhenKw, "expected when");
        let has_subject = self.at(K::LParen);
        if has_subject {
            self.parse_when_subject();
        }
        if self.eat(K::LBrace) {
            let entries = self.start();
            while !matches!(self.current_kind(), K::RBrace | K::Eof) {
                let before = self.position();
                self.parse_when_entry(has_subject);
                self.eat_semicolon_boundary();
                self.ensure_progress(before, "expected when entry");
            }
            self.complete(entries, K::WhenEntryList);
            self.expect(K::RBrace, "expected '}' after when");
        } else {
            self.expected_here("expected '{' after when subject");
            let entries = self.start();
            self.complete(entries, K::WhenEntryList);
        }
        self.complete(marker, K::WhenExpression)
    }

    fn parse_when_entry(&mut self, has_subject: bool) {
        let marker = self.start();
        if self.eat(K::ElseKw) {
            let conditions = self.start();
            self.complete(conditions, K::WhenConditionSeparatedList);
            self.expect(K::Arrow, "expected '->' after else");
        } else {
            let conditions = self.start();
            loop {
                let condition = self.start();
                self.parse_when_condition();
                self.complete(condition, K::WhenCondition);
                if self.eat(K::Comma) {
                    continue;
                }
                break;
            }
            self.complete(conditions, K::WhenConditionSeparatedList);
            if self.at(K::IfKw) {
                let guard = self.start();
                if !has_subject {
                    self.invalid_when_guard_here("when guard requires a subject");
                }
                self.bump();
                self.parse_expression_until(&[K::Arrow]);
                self.complete(guard, K::WhenGuard);
            }
            self.expect(K::Arrow, "expected '->' in when entry");
        }
        self.parse_expression_until(&[K::Semicolon, K::DoubleSemicolon, K::RBrace]);
        self.complete(marker, K::WhenEntry);
    }

    fn parse_when_subject(&mut self) {
        let subject = self.start();
        self.expect(K::LParen, "expected when subject");
        if self.at(K::ValKw) || self.at(K::VarKw) {
            self.bump();
            self.parse_name();
            if self.eat(K::Colon) {
                self.parse_type_reference_until(&[K::Assign, K::RParen]);
            }
            self.expect(K::Assign, "expected '=' in when subject");
            self.parse_expression_until(&[K::RParen]);
        } else if !self.at(K::RParen) {
            self.parse_expression_until(&[K::RParen]);
        }
        self.expect(K::RParen, "expected ')' after when subject");
        self.complete(subject, K::WhenSubject);
    }

    fn parse_when_condition(&mut self) {
        match self.current_kind() {
            K::IsKw | K::NotIs => {
                self.bump();
                self.parse_type_reference_until(&[K::Comma, K::IfKw, K::Arrow]);
            }
            K::InKw | K::NotIn => {
                self.bump();
                self.parse_expression_until(&[K::Comma, K::IfKw, K::Arrow]);
            }
            _ => self.parse_expression_until(&[K::Comma, K::IfKw, K::Arrow]),
        }
    }

    pub(super) fn parse_try_expression(&mut self) -> CompletedMarker {
        let marker = self.start();
        self.expect(K::TryKw, "expected try");
        if self.at(K::LBrace) {
            self.parse_block();
        }
        let catches = self.start();
        while self.at_soft_keyword("catch") {
            let catch = self.start();
            self.bump();
            if self.at(K::LParen) {
                self.parse_value_parameter_list();
            }
            if self.at(K::LBrace) {
                self.parse_block();
            }
            self.complete(catch, K::CatchClause);
        }
        self.complete(catches, K::CatchClauseList);
        if self.at_soft_keyword("finally") {
            let finally = self.start();
            self.bump();
            if self.at(K::LBrace) {
                self.parse_block();
            }
            self.complete(finally, K::FinallyClause);
        }
        self.complete(marker, K::TryExpression)
    }

    pub(super) fn parse_loop_expression(&mut self) -> CompletedMarker {
        let marker = self.start();
        let loop_kind = self.current_kind();
        self.bump();
        if loop_kind == K::ForKw && self.at(K::LParen) {
            self.parse_for_header();
        } else if loop_kind != K::DoKw && self.at(K::LParen) {
            self.parse_parenthesized_expression();
        }
        if self.at(K::LBrace) {
            self.parse_block();
        } else {
            self.parse_expression_until(&[K::WhileKw, K::Semicolon, K::DoubleSemicolon, K::RBrace]);
        }
        if loop_kind == K::DoKw {
            if self.eat(K::WhileKw) {
                if self.at(K::LParen) {
                    self.parse_parenthesized_expression();
                } else {
                    self.expected_here("expected condition after 'while'");
                }
            } else {
                self.expected_here("expected 'while' after do body");
            }
        }
        let kind = match loop_kind {
            K::ForKw => K::ForStatement,
            K::WhileKw => K::WhileStatement,
            K::DoKw => K::DoWhileStatement,
            _ => K::LoopExpression,
        };
        self.complete(marker, kind)
    }

    pub(super) fn parse_anonymous_function_expression(&mut self) -> CompletedMarker {
        let marker = self.start();
        self.expect(K::FunKw, "expected fun");
        if self.anonymous_function_receiver_type_ahead() {
            self.parse_type_reference_until(&[K::Dot]);
            self.expect(K::Dot, "expected receiver separator");
        }
        if self.at(K::LParen) {
            self.parse_value_parameter_list();
        }
        if self.eat(K::Colon) {
            self.parse_type_reference_until(&[K::Assign, K::LBrace, K::Semicolon, K::RBrace]);
        }
        if self.eat(K::Assign) {
            self.parse_expression_until(&[K::Semicolon, K::DoubleSemicolon, K::RBrace]);
        } else if self.at(K::LBrace) {
            self.parse_block();
        }
        self.complete(marker, K::AnonymousFunctionExpression)
    }

    pub(super) fn parse_object_expression(&mut self) -> CompletedMarker {
        let marker = self.start();
        self.expect(K::ObjectKw, "expected object");
        if self.eat(K::Colon) {
            self.parse_delegation_specifier_list();
        }
        if self.at(K::LBrace) {
            self.parse_class_body();
        }
        self.complete(marker, K::ObjectExpression)
    }

    fn parse_for_header(&mut self) {
        self.expect(K::LParen, "expected '(' after for");
        if self.at_destructuring_declaration_start() {
            self.parse_destructuring_declaration();
        } else {
            self.parse_expression_until(&[K::InKw, K::RParen]);
        }
        if self.eat(K::InKw) {
            self.parse_expression_until(&[K::RParen]);
        }
        self.expect(K::RParen, "expected ')' after for header");
    }

    fn anonymous_function_receiver_type_ahead(&mut self) -> bool {
        let mut angle_depth = 0usize;
        let mut paren_depth = 0usize;
        let mut bracket_depth = 0usize;

        for index in (self.position()..).take(MAX_ANONYMOUS_FUNCTION_RECEIVER_LOOKAHEAD) {
            match self.kind_at(index) {
                K::Dot
                    if angle_depth == 0
                        && paren_depth == 0
                        && bracket_depth == 0
                        && self.kind_at(index + 1) == K::LParen =>
                {
                    return true;
                }
                K::LParen => paren_depth += 1,
                K::RParen => {
                    if paren_depth == 0 {
                        return false;
                    }
                    paren_depth -= 1;
                }
                K::LBracket => bracket_depth += 1,
                K::RBracket => {
                    if bracket_depth == 0 {
                        return false;
                    }
                    bracket_depth -= 1;
                }
                K::Lt => angle_depth += 1,
                K::Gt => {
                    if angle_depth == 0 {
                        return false;
                    }
                    angle_depth -= 1;
                }
                K::LBrace
                | K::Colon
                | K::Assign
                | K::Semicolon
                | K::DoubleSemicolon
                | K::RBrace
                | K::Eof
                    if angle_depth == 0 && paren_depth == 0 && bracket_depth == 0 =>
                {
                    return false;
                }
                _ => {}
            }
        }

        false
    }
}
