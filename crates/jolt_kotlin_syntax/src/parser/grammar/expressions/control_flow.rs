use jolt_syntax::CompletedMarker;

use crate::KotlinSyntaxKind as K;

use super::super::{Parser, StopSet};
use super::predicates::is_expression_continuation;
use crate::parser::grammar::support::is_identifier_like_kind;

const MAX_ANONYMOUS_FUNCTION_RECEIVER_LOOKAHEAD: usize = 128;
const MAX_FOR_HEADER_RECOVERY_LOOKAHEAD: usize = 128;

impl Parser<'_> {
    pub(super) fn parse_if_expression(&mut self, stops: StopSet<'_>) -> CompletedMarker {
        let marker = self.start();
        self.eat_asserted(K::IfKw);
        if self.at(K::LParen) {
            self.parse_parenthesized_expression();
        } else {
            self.complete_missing_parenthesized_expression("expected condition after 'if'");
        }
        self.parse_control_structure_body(
            stops.with_extra(K::ElseKw),
            "expected branch after 'if' condition",
        );
        if self.eat(K::ElseKw) {
            self.parse_control_structure_body(stops, "expected branch after 'else'");
        }
        self.complete(marker, K::IfExpression)
    }

    pub(super) fn parse_when_expression(&mut self) -> CompletedMarker {
        let marker = self.start();
        self.eat_asserted(K::WhenKw);
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
                debug_assert!(self.position() > before);
            }
            self.complete(entries, K::WhenEntryList);
            if !self.eat(K::RBrace) {
                let diagnostic = self.pending_expected("expected '}' after when");
                self.missing_required_slot(
                    marker.anchor(),
                    crate::shape::when_expression::Slot::close_brace as u16,
                    [diagnostic],
                );
            }
        } else {
            let diagnostic = self.pending_expected("expected '{' after when subject");
            self.missing_required_slot(
                marker.anchor(),
                crate::shape::when_expression::Slot::open_brace as u16,
                [diagnostic],
            );
            let entries = self.start();
            while !matches!(self.current_kind(), K::RBrace | K::Eof)
                && !self.at_expression_rhs_declaration_boundary()
                && self.current_line_has_when_arrow()
            {
                let before = self.position();
                self.parse_when_entry(has_subject);
                self.eat_semicolon_boundary();
                debug_assert!(self.position() > before);
            }
            self.complete(entries, K::WhenEntryList);
            let diagnostic = self.pending_expected("expected '}' after when");
            self.missing_required_slot(
                marker.anchor(),
                crate::shape::when_expression::Slot::close_brace as u16,
                [diagnostic],
            );
        }
        self.complete(marker, K::WhenExpression)
    }

    fn parse_when_entry(&mut self, has_subject: bool) {
        let marker = self.start();
        let mut next_entry_boundary = None;
        if self.eat(K::ElseKw) {
            let conditions = self.start();
            self.complete(conditions, K::WhenConditionSeparatedList);
            if !self.eat(K::Arrow) {
                let diagnostic = self.pending_expected("expected '->' after else");
                self.missing_required_slot(
                    marker.anchor(),
                    crate::shape::when_entry::Slot::arrow as u16,
                    [diagnostic],
                );
            }
        } else {
            next_entry_boundary = self.parse_when_condition_list();
            if self.at(K::IfKw) {
                let guard = self.start();
                if !has_subject {
                    self.invalid_when_guard_here("when guard requires a subject");
                }
                self.bump();
                if self.at(K::Arrow) {
                    self.complete_missing_expression("expected expression after when guard");
                } else {
                    self.parse_expression_until(&[K::Arrow]);
                }
                self.complete(guard, K::WhenGuard);
            }
            if !self.eat(K::Arrow) {
                let diagnostic = self.pending_expected("expected '->' in when entry");
                self.missing_required_slot(
                    marker.anchor(),
                    crate::shape::when_entry::Slot::arrow as u16,
                    [diagnostic],
                );
            }
        }
        let body = self.start();
        if self.at(K::LBrace) {
            self.parse_block();
        } else if next_entry_boundary == Some(self.position())
            || matches!(
                self.current_kind(),
                K::Semicolon | K::DoubleSemicolon | K::RBrace | K::Eof
            )
            || self.at_expression_rhs_declaration_boundary()
        {
            self.complete_missing_expression("expected when entry body");
        } else {
            let boundary = self.next_when_entry_boundary_position();
            self.parse_expression_until(
                StopSet::new(&[K::Semicolon, K::DoubleSemicolon, K::RBrace])
                    .with_position(boundary),
            );
        }
        self.complete(body, K::WhenEntryBody);
        self.complete(marker, K::WhenEntry);
    }

    fn parse_when_condition_list(&mut self) -> Option<usize> {
        let conditions = self.start();
        let mut parsed_condition = false;
        let mut expect_condition = true;
        let mut next_entry_boundary = None;
        loop {
            if matches!(
                self.current_kind(),
                K::IfKw | K::Arrow | K::Semicolon | K::DoubleSemicolon | K::RBrace | K::Eof
            ) {
                if !parsed_condition {
                    let condition = self.start();
                    let diagnostic = self.pending_expected("expected when condition");

                    self.complete_recovery(condition, K::BogusWhenCondition, [diagnostic]);
                }
                break;
            }
            if self.at(K::Comma) {
                if expect_condition {
                    let condition = self.start();
                    let diagnostic =
                        self.pending_unexpected("expected when condition between commas");

                    self.complete_recovery(condition, K::BogusWhenCondition, [diagnostic]);
                }
                self.bump();
                expect_condition = true;
                continue;
            }
            let condition = self.start();
            let boundary = self.next_when_entry_boundary_position();
            next_entry_boundary = boundary;
            self.parse_when_condition(boundary);
            self.complete(condition, K::WhenCondition);
            parsed_condition = true;
            expect_condition = false;
            if !self.at(K::Comma) {
                break;
            }
        }
        self.complete(conditions, K::WhenConditionSeparatedList);
        next_entry_boundary
    }

    fn parse_when_subject(&mut self) {
        let subject = self.start();
        self.eat_asserted(K::LParen);
        if self.at(K::ValKw) || self.at(K::VarKw) {
            self.bump();
            self.parse_name();
            if self.eat(K::Colon) {
                self.parse_type_reference_until(&[K::Assign, K::RParen]);
            }
            if !self.eat(K::Assign) {
                let diagnostic = self.pending_expected("expected '=' in when subject");
                self.missing_required_slot(
                    subject.anchor(),
                    crate::shape::when_subject::Slot::assign as u16,
                    [diagnostic],
                );
            }
            if self.at(K::RParen) {
                self.complete_missing_expression("expected when subject expression");
            } else {
                self.parse_expression_until(&[K::RParen]);
            }
        } else if !self.at(K::RParen) {
            self.parse_expression_until(&[K::RParen]);
        } else {
            self.complete_missing_expression("expected when subject expression");
        }
        if !self.eat(K::RParen) {
            let diagnostic = self.pending_expected("expected ')' after when subject");
            self.missing_required_slot(
                subject.anchor(),
                crate::shape::when_subject::Slot::close_paren as u16,
                [diagnostic],
            );
        }
        self.complete(subject, K::WhenSubject);
    }

    fn parse_when_condition(&mut self, boundary: Option<usize>) {
        match self.current_kind() {
            K::IsKw | K::NotIs => {
                self.bump();
                if let Some(boundary) = boundary {
                    let mut stop_position = boundary;
                    for position in self.position()..boundary {
                        if matches!(self.kind_at(position), K::Comma | K::IfKw | K::Arrow) {
                            stop_position = position;
                            break;
                        }
                    }
                    self.parse_type_reference_until_position(stop_position);
                } else {
                    self.parse_type_reference_until(&[K::Comma, K::IfKw, K::Arrow]);
                }
            }
            K::InKw | K::NotIn => {
                self.bump();
                self.parse_expression_until(
                    StopSet::new(&[K::Comma, K::IfKw, K::Arrow]).with_position(boundary),
                );
            }
            _ => self.parse_expression_until(
                StopSet::new(&[K::Comma, K::IfKw, K::Arrow]).with_position(boundary),
            ),
        }
    }

    fn next_when_entry_boundary_position(&mut self) -> Option<usize> {
        for offset in 1..256 {
            let position = self.position() + offset;
            let kind = self.nth_kind(offset);
            if matches!(kind, K::RBrace | K::Eof) {
                return None;
            }
            if self.newline_between(position - 1, position)
                && !is_expression_continuation(self.kind_at(position - 1))
                && !is_expression_continuation(kind)
                && self.line_has_when_arrow_from(offset)
            {
                return Some(position);
            }
        }
        None
    }

    fn current_line_has_when_arrow(&mut self) -> bool {
        self.line_has_when_arrow_from(0)
    }

    fn line_has_when_arrow_from(&mut self, start_offset: usize) -> bool {
        let mut paren_depth = 0usize;
        let mut bracket_depth = 0usize;
        let mut brace_depth = 0usize;
        for offset in start_offset..(start_offset + 256) {
            let position = self.position() + offset;
            if offset > start_offset && self.newline_between(position - 1, position) {
                return false;
            }
            match self.nth_kind(offset) {
                K::LParen => paren_depth += 1,
                K::RParen if paren_depth > 0 => paren_depth -= 1,
                K::LBracket => bracket_depth += 1,
                K::RBracket if bracket_depth > 0 => bracket_depth -= 1,
                K::LBrace => brace_depth += 1,
                K::RBrace if brace_depth > 0 => brace_depth -= 1,
                K::Arrow if paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 => {
                    return true;
                }
                K::RBrace | K::Eof => return false,
                _ => {}
            }
        }
        false
    }

    pub(super) fn parse_try_expression(&mut self) -> CompletedMarker {
        let marker = self.start();
        self.eat_asserted(K::TryKw);
        if self.at(K::LBrace) {
            self.parse_block();
        } else {
            self.complete_missing_block("expected block after 'try'");
        }
        let clauses = self.start();
        let mut has_handler = false;
        let mut saw_finally = false;
        while self.at_soft_keyword("catch") || self.at_soft_keyword("finally") {
            has_handler = true;
            let is_catch = self.at_soft_keyword("catch");
            let invalid_order = saw_finally;
            let recovery = invalid_order.then(|| self.start());
            let clause = self.start();
            self.bump();
            if is_catch {
                if self.at(K::LParen) {
                    self.parse_catch_parameter();
                } else {
                    self.complete_missing_catch_parameter();
                }
                if self.at(K::LBrace) {
                    self.parse_block();
                } else {
                    self.complete_missing_block("expected block after 'catch'");
                }
                self.complete(clause, K::CatchClause);
            } else {
                saw_finally = true;
                if self.at(K::LBrace) {
                    self.parse_block();
                } else {
                    self.complete_missing_block("expected block after 'finally'");
                }
                self.complete(clause, K::FinallyClause);
            }
            if let Some(recovery) = recovery {
                let diagnostic = self.pending_unexpected(if is_catch {
                    "catch clause must precede finally"
                } else {
                    "try expression has more than one finally clause"
                });

                self.complete_recovery(recovery, K::BogusTryClause, [diagnostic]);
            }
        }
        self.complete(clauses, K::TryClauseList);
        if has_handler {
            self.complete(marker, K::TryExpression)
        } else {
            let diagnostic = self.pending_expected("expected 'catch' or 'finally' after try block");
            self.complete_recovery(marker, K::TryExpression, [diagnostic])
        }
    }

    fn parse_catch_parameter(&mut self) {
        let parameter = self.start();
        self.bump();
        self.parse_control_variable_modifier_list();
        self.parse_name();
        if !self.eat(K::Colon) {
            let diagnostic = self.pending_expected("expected ':' in catch parameter");
            self.missing_required_slot(
                parameter.anchor(),
                crate::shape::catch_parameter::Slot::colon as u16,
                [diagnostic],
            );
        }
        self.parse_type_reference_until(&[K::RParen]);
        if !self.eat(K::RParen) {
            let diagnostic = self.pending_expected("expected ')' after catch parameter");
            self.missing_required_slot(
                parameter.anchor(),
                crate::shape::catch_parameter::Slot::close_paren as u16,
                [diagnostic],
            );
        }
        self.complete(parameter, K::CatchParameter);
    }

    fn complete_missing_catch_parameter(&mut self) {
        let parameter = self.start();
        let diagnostic = self.pending_expected("expected catch parameter");
        self.complete_recovery(parameter, K::CatchParameter, [diagnostic]);
    }

    pub(super) fn parse_loop_expression(&mut self) -> CompletedMarker {
        let marker = self.start();
        let loop_kind = self.current_kind();
        self.bump();
        let kind = match loop_kind {
            K::ForKw => {
                if !self.eat(K::LParen) {
                    let diagnostic = self.pending_expected("expected '(' after 'for'");
                    self.missing_required_slot(
                        marker.anchor(),
                        crate::shape::for_statement::Slot::open_paren as u16,
                        [diagnostic],
                    );
                }
                if matches!(self.current_kind(), K::InKw | K::RParen | K::Eof) {
                    self.complete_missing_for_variable();
                } else {
                    let variable = self.start();
                    self.parse_control_variable_modifier_list();
                    self.parse_name_or_destructuring();
                    if self.eat(K::Colon) {
                        self.parse_type_reference_until(&[K::InKw, K::RParen]);
                    }
                    self.complete(variable, K::ForVariable);
                }
                if !self.eat(K::InKw) {
                    let diagnostic = self.pending_expected("expected 'in' after loop variable");
                    self.missing_required_slot(
                        marker.anchor(),
                        crate::shape::for_statement::Slot::in_token as u16,
                        [diagnostic],
                    );
                }
                if matches!(self.current_kind(), K::RParen | K::Eof) {
                    self.complete_missing_expression("expected loop iterable");
                } else {
                    let body_boundary = self.for_missing_close_body_boundary();
                    self.parse_expression_until(
                        StopSet::new(&[K::RParen]).with_position(body_boundary),
                    );
                }
                if !self.eat(K::RParen) {
                    let diagnostic = self.pending_expected("expected ')' after for header");
                    self.missing_required_slot(
                        marker.anchor(),
                        crate::shape::for_statement::Slot::close_paren as u16,
                        [diagnostic],
                    );
                }
                self.parse_control_structure_body(
                    (&[K::Semicolon, K::DoubleSemicolon, K::RBrace]).into(),
                    "expected body after 'for' header",
                );
                K::ForStatement
            }
            K::WhileKw => {
                if self.at(K::LParen) {
                    self.parse_parenthesized_expression();
                } else {
                    self.complete_missing_parenthesized_expression(
                        "expected condition after 'while'",
                    );
                }
                self.parse_control_structure_body(
                    (&[K::Semicolon, K::DoubleSemicolon, K::RBrace]).into(),
                    "expected body after 'while' condition",
                );
                K::WhileStatement
            }
            K::DoKw => {
                self.parse_control_structure_body(
                    (&[K::WhileKw, K::Semicolon, K::DoubleSemicolon, K::RBrace]).into(),
                    "expected body after 'do'",
                );
                if !self.eat(K::WhileKw) {
                    let diagnostic = self.pending_expected("expected 'while' after do body");
                    self.missing_required_slot(
                        marker.anchor(),
                        crate::shape::do_while_statement::Slot::while_token as u16,
                        [diagnostic],
                    );
                }
                if self.at(K::LParen) {
                    self.parse_parenthesized_expression();
                } else {
                    self.complete_missing_parenthesized_expression(
                        "expected condition after 'while'",
                    );
                }
                K::DoWhileStatement
            }
            _ => unreachable!("loop parser called without a loop keyword"),
        };
        self.complete(marker, kind)
    }

    pub(super) fn parse_anonymous_function_expression(&mut self) -> CompletedMarker {
        let marker = self.start();
        self.eat_asserted(K::FunKw);
        if self.anonymous_function_receiver_type_ahead() {
            self.parse_type_reference_until(&[K::Dot]);
            self.eat_asserted(K::Dot);
        }
        if self.at(K::LParen) {
            self.parse_value_parameter_list();
        } else {
            self.complete_missing_value_parameter_list();
        }
        if self.eat(K::Colon) {
            self.parse_type_reference_until(&[K::Assign, K::LBrace, K::Semicolon, K::RBrace]);
        }
        if self.at(K::Assign) {
            let body = self.start();
            self.bump();
            self.parse_expression_until(&[K::Semicolon, K::DoubleSemicolon, K::RBrace]);
            self.complete(body, K::ExpressionBody);
        } else if self.at(K::LBrace) {
            let body = self.start();
            self.parse_block();
            self.complete(body, K::BlockBody);
        } else {
            let body = self.start();
            let diagnostic = self.pending_expected("expected anonymous function body");
            self.complete_recovery(body, K::BogusDeclarationBody, [diagnostic]);
        }
        self.complete(marker, K::AnonymousFunctionExpression)
    }

    pub(super) fn parse_object_expression(&mut self) -> CompletedMarker {
        let marker = self.start();
        self.eat_asserted(K::ObjectKw);
        if self.at(K::Colon) {
            let delegation = self.start();
            self.bump();
            self.parse_delegation_specifier_entries();
            self.complete(delegation, K::DelegationClause);
        }
        if self.at(K::LBrace) {
            self.parse_class_body();
        } else {
            let body = self.start();
            let diagnostic = self.pending_expected("expected object body");
            self.complete_recovery(body, K::ClassBody, [diagnostic]);
        }
        self.complete(marker, K::ObjectExpression)
    }

    fn parse_control_structure_body(&mut self, stops: StopSet<'_>, message: &'static str) {
        if self.at(K::LBrace) {
            self.parse_block();
        } else if matches!(self.current_kind(), K::Semicolon | K::DoubleSemicolon) {
            let empty = self.start();
            self.bump();
            self.complete(empty, K::EmptyStatement);
        } else if self.at_expression_boundary(stops)
            || self.at_expression_rhs_declaration_boundary()
        {
            self.complete_missing_expression(message);
        } else {
            self.parse_expression_until(stops);
        }
    }

    fn complete_missing_for_variable(&mut self) {
        let variable = self.start();
        let diagnostic = self.pending_expected("expected loop variable");
        self.complete_recovery(variable, K::ForVariable, [diagnostic]);
    }

    fn for_missing_close_body_boundary(&mut self) -> Option<usize> {
        let mut parenthesis_depth = 0_usize;
        let mut bracket_depth = 0_usize;
        let mut brace_depth = 0_usize;
        let mut body_boundary = None;
        for offset in 0..MAX_FOR_HEADER_RECOVERY_LOOKAHEAD {
            let kind = self.nth_kind(offset);
            match kind {
                K::LParen => parenthesis_depth += 1,
                K::RParen if parenthesis_depth == 0 => return None,
                K::RParen => parenthesis_depth -= 1,
                K::LBracket => bracket_depth += 1,
                K::RBracket if bracket_depth > 0 => bracket_depth -= 1,
                K::LBrace => brace_depth += 1,
                K::RBrace if brace_depth > 0 => brace_depth -= 1,
                K::Semicolon | K::DoubleSemicolon | K::RBrace | K::Eof
                    if parenthesis_depth == 0 && bracket_depth == 0 && brace_depth == 0 =>
                {
                    break;
                }
                _ => {}
            }
            if offset > 0
                && parenthesis_depth == 0
                && bracket_depth == 0
                && brace_depth <= usize::from(kind == K::LBrace)
                && body_boundary.is_none()
                && (kind == K::LBrace
                    || (is_identifier_like_kind(kind) && self.nth_kind(offset + 1) == K::LParen))
            {
                body_boundary = self.position().checked_add(offset);
            }
        }
        body_boundary
    }

    fn parse_control_variable_modifier_list(&mut self) {
        let modifiers = self.start();
        while self.at(K::At) || self.at(K::Hash) {
            let before = self.position();
            self.parse_annotation();
            debug_assert!(self.position() > before);
        }
        self.complete(modifiers, K::ModifierList);
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
