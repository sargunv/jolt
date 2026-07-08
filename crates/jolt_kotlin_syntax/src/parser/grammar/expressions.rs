use jolt_syntax::CompletedMarker;

use crate::KotlinSyntaxKind as K;

use super::{Parser, StopSet};

#[path = "expressions/control_flow.rs"]
mod control_flow;
#[path = "expressions/literals.rs"]
mod literals;
#[path = "expressions/predicates.rs"]
mod predicates;
#[path = "expressions/strings.rs"]
mod strings;

use self::predicates::{
    expression_start_kind, is_assignment_operator, is_binary_operator, is_expression_continuation,
    is_literal_kind, is_unary_operator,
};

impl Parser<'_> {
    pub(super) fn parse_expression_until<'a>(&mut self, stops: impl Into<StopSet<'a>>) {
        let stops = stops.into();
        if self.at_expression_boundary(stops) {
            let error = self.start();
            self.expected_here("expected expression");
            self.complete(error, K::ErrorNode);
            return;
        }

        self.parse_assignment_expression(stops);

        while !self.at_expression_boundary(stops) {
            if self.at_semicolon_boundary() && !is_expression_continuation(self.current_kind()) {
                break;
            }
            let error = self.start();
            self.unexpected_here("unexpected token in expression");
            self.bump();
            self.complete(error, K::ErrorNode);
        }
    }

    fn parse_assignment_expression(&mut self, stops: StopSet<'_>) -> CompletedMarker {
        let lhs = self.parse_binary_expression(stops, 0);
        if self.at_expression_boundary(stops) || !is_assignment_operator(self.current_kind()) {
            return lhs;
        }

        let lhs = if Self::is_assignment_left_hand_side(lhs.kind()) {
            lhs
        } else {
            let error = self.precede(lhs);
            self.invalid_assignment_target_here("invalid assignment target");
            self.complete(error, K::ErrorNode)
        };

        let assignment = self.precede(lhs);
        self.bump();
        self.parse_assignment_expression(stops);
        self.complete(assignment, K::AssignmentExpression)
    }

    fn parse_binary_expression(
        &mut self,
        stops: StopSet<'_>,
        minimum_precedence: u8,
    ) -> CompletedMarker {
        let mut lhs = self.parse_unary_expression(stops);

        while let Some(info) = self.binary_operator_info(stops) {
            if info.precedence < minimum_precedence {
                break;
            }

            let binary = self.precede(lhs);
            let operator = self.current_kind();
            if operator == K::Elvis && self.elvis_missing_rhs(stops) {
                self.expected_here("expected expression after operator");
                self.bump();
                let error = self.start();
                self.complete(error, K::ErrorNode);
            } else if matches!(operator, K::AsKw | K::AsSafe | K::IsKw | K::NotIs) {
                self.bump();
                self.parse_type_reference_until(&[
                    K::Elvis,
                    K::OrOr,
                    K::AndAnd,
                    K::Arrow,
                    K::IfKw,
                    K::Comma,
                    K::Semicolon,
                    K::DoubleSemicolon,
                    K::RBrace,
                    K::RParen,
                    K::RBracket,
                    K::LongTemplateEntryEnd,
                ]);
            } else {
                self.bump();
                self.parse_binary_expression(stops, info.precedence + 1);
            }
            lhs = self.complete(binary, K::BinaryExpression);
        }

        lhs
    }

    fn parse_unary_expression(&mut self, stops: StopSet<'_>) -> CompletedMarker {
        if is_unary_operator(self.current_kind()) {
            let unary = self.start();
            self.bump();
            self.parse_unary_expression(stops);
            return self.complete(unary, K::UnaryExpression);
        }

        self.parse_postfix_expression(stops)
    }

    fn parse_postfix_expression(&mut self, stops: StopSet<'_>) -> CompletedMarker {
        let mut expression = self.parse_primary_expression(stops);

        loop {
            if self.at_expression_boundary(stops) {
                break;
            }
            if self.newline_before_current()
                && !matches!(
                    self.current_kind(),
                    K::LBrace | K::LBracket | K::Dot | K::SafeAccess | K::ColonColon | K::Elvis
                )
                && !self.at_split_safe_access()
            {
                break;
            }

            match self.current_kind() {
                K::Lt if self.type_argument_list_is_call_suffix_ahead() => {
                    self.parse_type_argument_list();
                }
                K::Lt => {
                    if let Some(message) = self.type_argument_list_issue_ahead() {
                        let error = self.start();
                        self.malformed_type_argument_list_here(message);
                        self.complete(error, K::ErrorNode);
                    }
                    break;
                }
                K::LParen => {
                    if Self::is_callable_reference_expression(expression.kind()) {
                        self.reserved_callable_reference_call_here(
                            "callable reference call syntax is reserved",
                        );
                    }
                    let call = self.precede(expression);
                    self.parse_value_argument_list();
                    expression = self.complete(call, K::CallExpression);
                }
                K::LBracket => {
                    let index = self.precede(expression);
                    self.expect(K::LBracket, "expected '['");
                    self.parse_comma_separated_until(K::RBracket, K::ValueArgument);
                    self.expect(K::RBracket, "expected ']'");
                    expression = self.complete(index, K::IndexExpression);
                }
                K::LBrace => {
                    let call = self.precede(expression);
                    self.parse_lambda_expression();
                    expression = self.complete(call, K::CallExpression);
                }
                kind if self.at_labeled_lambda_start(kind) => {
                    let call = self.precede(expression);
                    self.parse_labeled_lambda_expression();
                    expression = self.complete(call, K::CallExpression);
                }
                K::Dot | K::SafeAccess => {
                    let navigation = self.precede(expression);
                    self.bump();
                    self.parse_navigation_member();
                    expression = self.complete(navigation, K::NavigationExpression);
                }
                K::Question if self.at_split_safe_access() => {
                    let navigation = self.precede(expression);
                    self.bump();
                    self.bump();
                    self.parse_navigation_member();
                    expression = self.complete(navigation, K::NavigationExpression);
                }
                K::ColonColon => {
                    let reference = self.precede(expression);
                    self.bump();
                    if self.at(K::Lt) && self.type_argument_list_is_call_suffix_ahead() {
                        self.parse_type_argument_list();
                    }
                    if self.at_identifier_like() || matches!(self.current_kind(), K::ClassKw) {
                        self.bump();
                        if self.at(K::Lt) {
                            self.parse_type_argument_list();
                        }
                    } else {
                        self.expected_here("expected callable reference name");
                    }
                    expression = self.complete(reference, K::CallableReferenceExpression);
                }
                K::PlusPlus | K::MinusMinus | K::BangBang => {
                    let postfix = self.precede(expression);
                    self.bump();
                    expression = self.complete(postfix, K::PostfixExpression);
                }
                _ => break,
            }
        }

        expression
    }

    fn parse_navigation_member(&mut self) {
        if self.at_identifier_like() || matches!(self.current_kind(), K::ThisKw | K::SuperKw) {
            self.bump();
        } else {
            self.expected_here("expected member name");
        }
    }

    fn parse_primary_expression(&mut self, stops: StopSet<'_>) -> CompletedMarker {
        match self.current_kind() {
            K::At | K::Hash => self.parse_annotated_expression(stops),
            K::IfKw => self.parse_if_expression(),
            K::WhenKw => self.parse_when_expression(),
            K::TryKw => self.parse_try_expression(),
            K::ForKw | K::WhileKw | K::DoKw => self.parse_loop_expression(),
            K::FunKw => self.parse_anonymous_function_expression(),
            K::ObjectKw => self.parse_object_expression(),
            K::ReturnKw | K::BreakKw | K::ContinueKw => self.parse_jump_expression(stops),
            K::ThrowKw => self.parse_throw_expression(stops),
            K::LBrace => self.parse_lambda_expression(),
            K::LBracket => self.parse_collection_literal_expression(),
            K::LParen => self.parse_parenthesized_expression(),
            K::ColonColon => self.parse_callable_reference_expression(),
            K::OpenQuote | K::InterpolationPrefix => self.parse_string_template_expression(),
            K::ThisKw => self.parse_this_expression(),
            K::SuperKw => self.parse_super_expression(),
            kind if is_literal_kind(kind) => {
                self.parse_single_token_expression(K::LiteralExpression)
            }
            kind if self.at_label_start(kind) => self.parse_labeled_expression(stops),
            kind if self.at_identifier_like() || matches!(kind, K::ClassKw) => {
                self.parse_single_token_expression(K::NameExpression)
            }
            _ => {
                let error = self.start();
                self.expected_here("expected expression");
                if !self.at_eof() && !stops.contains(self.current_kind()) {
                    self.bump();
                }
                self.complete(error, K::ErrorNode)
            }
        }
    }

    fn parse_single_token_expression(&mut self, kind: K) -> CompletedMarker {
        let marker = self.start();
        self.bump();
        self.complete(marker, kind)
    }

    fn parse_annotated_expression(&mut self, stops: StopSet<'_>) -> CompletedMarker {
        let marker = self.start();
        self.parse_modifier_list();
        if self.at_expression_boundary(stops) {
            let error = self.start();
            self.expected_here("expected expression");
            self.complete(error, K::ErrorNode);
        } else {
            self.parse_assignment_expression(stops);
        }
        self.complete(marker, K::AnnotatedExpression)
    }

    fn parse_this_expression(&mut self) -> CompletedMarker {
        let marker = self.start();
        self.expect(K::ThisKw, "expected this");
        self.parse_optional_label_reference();
        self.complete(marker, K::ThisExpression)
    }

    fn parse_super_expression(&mut self) -> CompletedMarker {
        let marker = self.start();
        self.expect(K::SuperKw, "expected super");
        if self.at(K::Lt) {
            self.parse_type_argument_list();
        }
        self.parse_optional_label_reference();
        self.complete(marker, K::SuperExpression)
    }

    pub(in crate::parser::grammar) fn parse_value_argument_list(&mut self) {
        let marker = self.start();
        self.expect(K::LParen, "expected argument list");
        self.parse_comma_separated_until(K::RParen, K::ValueArgument);
        self.expect(K::RParen, "expected ')' after arguments");
        self.complete(marker, K::ValueArgumentList);
    }

    fn parse_parenthesized_expression(&mut self) -> CompletedMarker {
        let marker = self.start();
        self.expect(K::LParen, "expected '('");
        if !self.at(K::RParen) {
            self.parse_expression_until(&[K::RParen]);
        }
        self.expect(K::RParen, "expected ')' after expression");
        self.complete(marker, K::ParenthesizedExpression)
    }

    fn parse_jump_expression(&mut self, stops: StopSet<'_>) -> CompletedMarker {
        let marker = self.start();
        self.bump();
        self.parse_optional_label_reference();
        if !self.at_semicolon_boundary()
            && !self.at_expression_boundary(stops.with_extra(K::RBrace))
        {
            self.parse_expression_until(stops.with_extra(K::RBrace));
        }
        self.complete(marker, K::JumpExpression)
    }

    fn parse_throw_expression(&mut self, stops: StopSet<'_>) -> CompletedMarker {
        let marker = self.start();
        self.bump();
        self.parse_expression_until(stops.with_extra(K::RBrace));
        self.complete(marker, K::ThrowExpression)
    }

    fn parse_labeled_expression(&mut self, stops: StopSet<'_>) -> CompletedMarker {
        let marker = self.start();
        self.parse_optional_label_definition();
        self.parse_assignment_expression(stops);
        self.complete(marker, K::NameExpression)
    }

    fn parse_callable_reference_expression(&mut self) -> CompletedMarker {
        let marker = self.start();
        self.expect(K::ColonColon, "expected callable reference");
        if self.at(K::Lt) {
            self.parse_type_argument_list();
        }
        if self.at_identifier_like() || matches!(self.current_kind(), K::ClassKw) {
            self.bump();
            if self.at(K::Lt) {
                self.parse_type_argument_list();
            }
        } else {
            self.expected_here("expected callable reference name");
        }
        self.complete(marker, K::CallableReferenceExpression)
    }

    fn elvis_missing_rhs(&mut self, stops: StopSet<'_>) -> bool {
        let next = self.nth_kind(1);
        next == K::Eof
            || stops.contains(next)
            || matches!(
                next,
                K::Semicolon | K::DoubleSemicolon | K::RBrace | K::RParen | K::LongTemplateEntryEnd
            )
            || (self.newline_between(self.position(), self.position() + 1)
                && matches!(next, K::ReturnKw | K::BreakKw | K::ContinueKw))
    }

    fn at_expression_boundary(&mut self, stops: StopSet<'_>) -> bool {
        self.at_eof() || stops.contains(self.current_kind())
    }

    fn binary_operator_info(&mut self, stops: StopSet<'_>) -> Option<BinaryOperatorInfo> {
        if self.at_expression_boundary(stops) {
            return None;
        }

        if self.newline_before_current()
            && matches!(
                self.current_kind(),
                K::InKw | K::NotIn | K::IsKw | K::NotIs | K::AsKw | K::AsSafe
            )
        {
            return None;
        }

        let precedence = match self.current_kind() {
            K::OrOr => 1,
            K::AndAnd => 2,
            K::EqEq | K::BangEq | K::EqEqEq | K::BangEqEqEq => 3,
            K::Lt | K::LtEq | K::Gt | K::GtEq | K::InKw | K::NotIn | K::IsKw | K::NotIs => 4,
            K::AsKw | K::AsSafe => 5,
            K::Elvis => 6,
            K::Range | K::RangeUntil => 7,
            K::Plus | K::Minus => 8,
            K::Star | K::Slash | K::Percent | K::Amp => 9,
            kind if self.at_infix_function_operator(kind) => 6,
            kind if is_binary_operator(kind) => 4,
            _ => return None,
        };

        Some(BinaryOperatorInfo { precedence })
    }

    fn is_assignment_left_hand_side(kind: jolt_syntax::RawSyntaxKind) -> bool {
        matches!(
            K::from_raw(kind),
            Some(
                K::NameExpression
                    | K::NavigationExpression
                    | K::IndexExpression
                    | K::ThisExpression
                    | K::SuperExpression
                    | K::ParenthesizedExpression
            )
        )
    }

    fn is_callable_reference_expression(kind: jolt_syntax::RawSyntaxKind) -> bool {
        K::from_raw(kind) == Some(K::CallableReferenceExpression)
    }

    fn at_infix_function_operator(&mut self, kind: K) -> bool {
        (self.at_identifier_like() || matches!(kind, K::ByKw))
            && expression_start_kind(self.nth_kind(1))
            && !self.newline_before_current()
    }

    fn at_split_safe_access(&mut self) -> bool {
        self.current_kind() == K::Question
            && self.nth_kind(1) == K::Dot
            && self.tokens_are_adjacent(self.position(), 2)
    }

    fn at_label_start(&mut self, kind: K) -> bool {
        (self.at_identifier_like() || matches!(kind, K::ThisKw | K::SuperKw))
            && self.nth_kind(1) == K::At
            && matches!(
                self.nth_kind(2),
                K::ForKw | K::WhileKw | K::DoKw | K::LBrace | K::IfKw | K::WhenKw | K::TryKw
            )
    }

    fn at_labeled_lambda_start(&mut self, kind: K) -> bool {
        (self.at_identifier_like() || matches!(kind, K::ThisKw | K::SuperKw))
            && self.nth_kind(1) == K::At
            && self.nth_kind(2) == K::LBrace
    }

    fn parse_optional_label_definition(&mut self) {
        if self.at_identifier_like() || matches!(self.current_kind(), K::ThisKw | K::SuperKw) {
            self.bump();
            self.expect(K::At, "expected label");
        }
    }

    fn parse_optional_label_reference(&mut self) {
        if self.eat(K::At) {
            if self.at_identifier_like() || matches!(self.current_kind(), K::ThisKw | K::SuperKw) {
                self.bump();
            } else {
                self.expected_here("expected label name");
            }
        }
    }
}

#[derive(Clone, Copy)]
struct BinaryOperatorInfo {
    precedence: u8,
}
