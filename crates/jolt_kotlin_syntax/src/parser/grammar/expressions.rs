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
            let diagnostic = self.pending_expected("expected expression");

            self.complete_recovery(error, K::BogusExpression, [diagnostic]);
            return;
        }

        let mut expression = self.parse_assignment_expression(stops);
        if self.at_expression_boundary(stops)
            || self.at_semicolon_boundary() && !is_expression_continuation(self.current_kind())
        {
            return;
        }

        while expression_start_kind(self.current_kind())
            && !self.at_expression_rhs_declaration_boundary()
        {
            let combined = self.precede(expression);
            let diagnostic = self.pending_unexpected("unexpected token in expression");
            self.parse_assignment_expression(stops);
            expression = self.complete_recovery(combined, K::BinaryExpression, [diagnostic]);
            if self.at_expression_boundary(stops)
                || self.at_semicolon_boundary() && !is_expression_continuation(self.current_kind())
            {
                return;
            }
        }

        while !self.at_expression_boundary(stops)
            && (!self.at_semicolon_boundary() || is_expression_continuation(self.current_kind()))
        {
            let error = self.precede(expression);
            let diagnostic = self.pending_unexpected("unexpected token in expression");
            self.bump();
            expression = self.complete_recovery(error, K::BogusExpression, [diagnostic]);
        }
    }

    fn parse_assignment_expression(&mut self, stops: StopSet<'_>) -> CompletedMarker {
        if let Some(expression) =
            self.with_syntax_nesting(|parser| parser.parse_assignment_expression_inner(stops))
        {
            return expression;
        }

        self.parse_excessive_expression(stops)
    }

    fn parse_assignment_expression_inner(&mut self, stops: StopSet<'_>) -> CompletedMarker {
        let lhs = self.parse_binary_expression(stops, 0);
        if self.at_expression_boundary(stops) || !is_assignment_operator(self.current_kind()) {
            return lhs;
        }

        let lhs = if Self::is_assignment_left_hand_side(lhs.kind()) {
            lhs
        } else {
            let error = self.precede(lhs);
            let diagnostic = self.invalid_assignment_target_here("invalid assignment target");

            self.complete_recovery(error, K::BogusExpression, [diagnostic])
        };

        let assignment = self.precede(lhs);
        self.bump();
        if self.at_expression_boundary(stops) || self.at_expression_rhs_declaration_boundary() {
            let rhs = self.start();
            let diagnostic = self.pending_expected("expected expression after operator");

            self.complete_recovery(rhs, K::BogusExpression, [diagnostic]);
        } else {
            self.parse_assignment_expression(stops);
        }
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
                let diagnostic = self.pending_expected("expected expression after operator");
                self.bump();
                let error = self.start();

                self.complete_recovery(error, K::BogusExpression, [diagnostic]);
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
                if self.at_expression_boundary(stops)
                    || self.at_expression_rhs_declaration_boundary()
                {
                    let rhs = self.start();
                    let diagnostic = self.pending_expected("expected expression after operator");

                    self.complete_recovery(rhs, K::BogusExpression, [diagnostic]);
                } else {
                    self.parse_binary_expression(stops, info.precedence + 1);
                }
            }
            lhs = self.complete(binary, K::BinaryExpression);
        }

        lhs
    }

    fn parse_unary_expression(&mut self, stops: StopSet<'_>) -> CompletedMarker {
        if let Some(expression) =
            self.with_syntax_nesting(|parser| parser.parse_unary_expression_inner(stops))
        {
            return expression;
        }

        self.parse_excessive_expression(stops)
    }

    fn parse_unary_expression_inner(&mut self, stops: StopSet<'_>) -> CompletedMarker {
        if is_unary_operator(self.current_kind()) {
            let unary = self.start();
            self.bump();
            self.parse_unary_expression(stops);
            return self.complete(unary, K::UnaryExpression);
        }

        self.parse_postfix_expression(stops)
    }

    fn parse_excessive_expression(&mut self, stops: StopSet<'_>) -> CompletedMarker {
        let expression = self.start();
        let diagnostic = self.pending_excessive_syntax_nesting();
        let (mut parens, mut brackets, mut braces, mut long_templates) =
            (0usize, 0usize, 0usize, 0usize);
        let mut consumed_outside = false;

        loop {
            let current = self.current_kind();
            let outside = parens == 0 && brackets == 0 && braces == 0 && long_templates == 0;
            if current == K::Eof
                || current == K::RParen && parens == 0
                || current == K::RBracket && brackets == 0
                || current == K::RBrace && braces == 0
                || current == K::LongTemplateEntryEnd && long_templates == 0
                || outside
                    && (stops.contains(current, self.position())
                        || matches!(current, K::Semicolon | K::DoubleSemicolon)
                        || consumed_outside
                            && self.newline_before_current()
                            && !is_expression_continuation(current)
                        || self.at_expression_rhs_declaration_boundary())
            {
                break;
            }

            consumed_outside |= outside;
            match current {
                K::LParen => parens += 1,
                K::RParen => parens -= 1,
                K::LBracket => brackets += 1,
                K::RBracket => brackets -= 1,
                K::LBrace => braces += 1,
                K::RBrace => braces -= 1,
                K::LongTemplateEntryStart => long_templates += 1,
                K::LongTemplateEntryEnd => long_templates -= 1,
                _ => {}
            }
            self.bump();
        }

        self.complete_recovery(expression, K::BogusExpression, [diagnostic])
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
                    expression = self.parse_call_suffix(expression);
                }
                K::Lt => {
                    if let Some(message) = self.type_argument_list_issue_ahead() {
                        let error = self.precede(expression);
                        let diagnostic = self.malformed_type_argument_list_here(message);

                        expression =
                            self.complete_recovery(error, K::BogusExpression, [diagnostic]);
                    }
                    break;
                }
                K::LParen => {
                    if Self::is_callable_reference_expression(expression.kind()) {
                        self.reserved_callable_reference_call_here(
                            "callable reference call syntax is reserved",
                        );
                    }
                    expression = self.parse_call_suffix(expression);
                }
                K::LBracket => {
                    expression = self.parse_index_suffix(expression);
                }
                K::LBrace => {
                    expression = self.parse_call_suffix(expression);
                }
                kind if self.at_labeled_lambda_start(kind) => {
                    expression = self.parse_call_suffix(expression);
                }
                K::Dot | K::SafeAccess => {
                    expression = self.parse_navigation_suffix(expression, false);
                }
                K::Question if self.at_split_safe_access() => {
                    expression = self.parse_navigation_suffix(expression, true);
                }
                K::ColonColon => {
                    expression = self.parse_callable_reference_suffix(expression);
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

    fn parse_index_suffix(&mut self, expression: CompletedMarker) -> CompletedMarker {
        let index = self.precede(expression);
        self.eat_asserted(K::LBracket);
        self.parse_value_arguments_until(K::RBracket, K::ValueArgumentEntryList);
        if !self.eat(K::RBracket) {
            let diagnostic = self.pending_expected("expected ']'");
            self.missing_required_slot(
                index.anchor(),
                crate::shape::index_expression::Slot::close_bracket as u16,
                [diagnostic],
            );
        }
        self.complete(index, K::IndexExpression)
    }

    fn parse_navigation_suffix(
        &mut self,
        expression: CompletedMarker,
        split_safe_access: bool,
    ) -> CompletedMarker {
        let navigation = self.precede(expression);
        if split_safe_access {
            let split = self.start();
            self.bump();
            self.bump();
            self.complete(split, K::SplitSafeNavigationOperator);
        } else {
            self.bump();
        }
        self.parse_navigation_member();
        self.complete(navigation, K::NavigationExpression)
    }

    fn parse_callable_reference_suffix(&mut self, expression: CompletedMarker) -> CompletedMarker {
        let receiver = self.precede(expression);
        let receiver = self.complete(receiver, K::CallableReferenceReceiver);
        let reference = self.precede(receiver);
        self.bump();
        if self.at_identifier_like() || matches!(self.current_kind(), K::ClassKw) {
            let target = self.start();
            self.bump();
            self.complete(target, K::CallableReferenceTarget);
        } else {
            self.complete_missing_callable_reference_target();
        }
        let type_arguments = self.start();
        while self.at(K::Lt) {
            self.parse_type_argument_list();
        }
        self.complete(type_arguments, K::TypeArgumentListList);
        self.complete(reference, K::CallableReferenceExpression)
    }

    fn parse_navigation_member(&mut self) {
        if self.at_identifier_like() {
            self.bump();
        } else if self.at(K::ThisKw) {
            self.parse_this_expression();
        } else if self.at(K::SuperKw) {
            self.parse_super_expression();
        } else {
            let selector = self.start();
            let diagnostic = self.pending_expected("expected member name");

            self.complete_recovery(selector, K::BogusNavigationSelector, [diagnostic]);
        }
    }

    fn parse_call_suffix(&mut self, callee: CompletedMarker) -> CompletedMarker {
        let call = self.precede(callee);
        let type_arguments = self.start();
        while self.at(K::Lt) && self.type_argument_list_is_call_suffix_ahead() {
            self.parse_type_argument_list();
        }
        self.complete(type_arguments, K::TypeArgumentListList);

        if self.at(K::LParen) {
            self.parse_value_argument_list();
        }

        let lambdas = self.start();
        loop {
            let kind = self.current_kind();
            if !self.at(K::LBrace) && !self.at_labeled_lambda_start(kind) {
                break;
            }
            if self.at(K::LBrace) {
                self.parse_lambda_expression();
            } else {
                self.parse_labeled_lambda_expression();
            }
        }
        self.complete(lambdas, K::LambdaExpressionList);
        self.complete(call, K::CallExpression)
    }

    fn parse_primary_expression(&mut self, stops: StopSet<'_>) -> CompletedMarker {
        match self.current_kind() {
            K::At | K::Hash => self.parse_annotated_expression(stops),
            K::IfKw => self.parse_if_expression(stops),
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
                let diagnostic = self.pending_expected("expected expression");
                if !self.at_eof() && !stops.contains(self.current_kind(), self.position()) {
                    self.bump();
                }

                self.complete_recovery(error, K::BogusExpression, [diagnostic])
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
        let prefix = self.start();
        while self.at_modifier_or_annotation() {
            if self.at(K::At) || self.at(K::Hash) {
                self.parse_annotation();
            } else {
                while self.at_modifier_or_annotation() && !self.at(K::At) && !self.at(K::Hash) {
                    self.bump();
                }
            }
        }
        self.complete(prefix, K::ModifierList);
        if self.at_expression_boundary(stops) {
            let error = self.start();
            let diagnostic = self.pending_expected("expected expression");

            self.complete_recovery(error, K::BogusExpression, [diagnostic]);
        } else {
            self.parse_assignment_expression(stops);
        }
        self.complete(marker, K::AnnotatedExpression)
    }

    fn parse_this_expression(&mut self) -> CompletedMarker {
        let marker = self.start();
        self.eat_asserted(K::ThisKw);
        self.parse_optional_typed_label_reference();
        self.complete(marker, K::ThisExpression)
    }

    fn parse_super_expression(&mut self) -> CompletedMarker {
        let marker = self.start();
        self.eat_asserted(K::SuperKw);
        if self.at(K::Lt) {
            self.parse_type_argument_list();
        }
        self.parse_optional_typed_label_reference();
        self.complete(marker, K::SuperExpression)
    }

    pub(in crate::parser::grammar) fn parse_value_argument_list(&mut self) {
        let marker = self.start();
        self.eat_asserted(K::LParen);
        self.parse_value_arguments_until(K::RParen, K::ValueArgumentEntryList);
        if !self.eat(K::RParen) {
            let diagnostic = self.pending_expected("expected ')' after arguments");
            self.missing_required_slot(
                marker.anchor(),
                crate::shape::value_argument_list::Slot::close_paren as u16,
                [diagnostic],
            );
        }
        self.complete(marker, K::ValueArgumentList);
    }

    pub(in crate::parser::grammar) fn parse_value_arguments_until(
        &mut self,
        close: K,
        list_kind: K,
    ) {
        let entries = self.start();
        let mut expect_item = true;
        while !self.at(K::Eof) && !self.at(close) {
            let before = self.position();
            if self.at(K::Comma) {
                if expect_item {
                    let missing = self.start();
                    let diagnostic = self.pending_expected("expected list item");

                    self.complete_recovery(missing, K::BogusValueArgument, [diagnostic]);
                }
                self.bump();
                expect_item = true;
                continue;
            }
            self.parse_value_argument(close);
            expect_item = false;
            debug_assert!(self.position() > before);
        }
        self.complete(entries, list_kind);
    }

    fn parse_value_argument(&mut self, close: K) {
        let argument = self.start();
        let prefix = self.start();
        while self.at(K::Star) || self.at(K::At) || self.at(K::Hash) {
            let item = self.start();
            if self.at(K::Star) {
                self.bump();
            } else {
                self.parse_annotation();
            }
            self.complete(item, K::ValueArgumentPrefix);
        }
        self.complete(prefix, K::ValueArgumentPrefixList);
        if self.at_identifier_like() && self.nth_kind(1) == K::Assign {
            self.parse_name();
            self.bump();
        }
        self.parse_expression_until(&[K::Comma, close]);
        self.complete(argument, K::ValueArgument);
    }

    fn parse_parenthesized_expression(&mut self) -> CompletedMarker {
        let marker = self.start();
        self.eat_asserted(K::LParen);
        if self.at(K::RParen) {
            let expression = self.start();
            let diagnostic = self.pending_expected("expected parenthesized expression");

            self.complete_recovery(expression, K::BogusExpression, [diagnostic]);
        } else {
            self.parse_expression_until(&[K::RParen]);
        }
        if !self.eat(K::RParen) {
            let diagnostic = self.pending_expected("expected ')' after expression");
            self.missing_required_slot(
                marker.anchor(),
                crate::shape::parenthesized_expression::Slot::close_paren as u16,
                [diagnostic],
            );
        }
        self.complete(marker, K::ParenthesizedExpression)
    }

    fn parse_jump_expression(&mut self, stops: StopSet<'_>) -> CompletedMarker {
        let marker = self.start();
        let keyword = self.current_kind();
        self.bump();
        self.parse_optional_typed_label_reference();
        if !self.at_semicolon_boundary()
            && !self.at_expression_boundary(stops.with_extra(K::RBrace))
            && !self.at_expression_rhs_declaration_boundary()
        {
            if keyword == K::ReturnKw {
                self.parse_expression_until(stops.with_extra(K::RBrace));
            } else {
                let expression = self.start();
                let diagnostic =
                    self.pending_unexpected("break and continue do not accept an expression");

                self.parse_expression_until(stops.with_extra(K::RBrace));
                self.complete_recovery(expression, K::BogusExpression, [diagnostic]);
            }
        }
        self.complete(marker, K::JumpExpression)
    }

    fn parse_throw_expression(&mut self, stops: StopSet<'_>) -> CompletedMarker {
        let marker = self.start();
        self.bump();
        if self.at_semicolon_boundary()
            || self.at_expression_boundary(stops.with_extra(K::RBrace))
            || self.at_expression_rhs_declaration_boundary()
        {
            self.complete_missing_expression("expected expression after 'throw'");
        } else {
            self.parse_expression_until(stops.with_extra(K::RBrace));
        }
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
        self.eat_asserted(K::ColonColon);
        if self.at_identifier_like() || matches!(self.current_kind(), K::ClassKw) {
            let target = self.start();
            self.bump();
            self.complete(target, K::CallableReferenceTarget);
        } else {
            self.complete_missing_callable_reference_target();
        }
        let type_arguments = self.start();
        while self.at(K::Lt) {
            self.parse_type_argument_list();
        }
        self.complete(type_arguments, K::TypeArgumentListList);
        self.complete(marker, K::CallableReferenceExpression)
    }

    fn complete_missing_callable_reference_target(&mut self) {
        let target = self.start();
        let diagnostic = self.pending_expected("expected callable reference name");
        self.missing_required_slot(
            target.anchor(),
            crate::shape::callable_reference_target::Slot::target as u16,
            [diagnostic],
        );
        self.complete(target, K::CallableReferenceTarget);
    }

    fn elvis_missing_rhs(&mut self, stops: StopSet<'_>) -> bool {
        let next = self.nth_kind(1);
        next == K::Eof
            || stops.contains(next, self.position() + 1)
            || matches!(
                next,
                K::Semicolon | K::DoubleSemicolon | K::RBrace | K::RParen | K::LongTemplateEntryEnd
            )
            || (self.newline_between(self.position(), self.position() + 1)
                && matches!(next, K::ReturnKw | K::BreakKw | K::ContinueKw))
    }

    fn at_expression_boundary(&mut self, stops: StopSet<'_>) -> bool {
        self.at_eof() || stops.contains(self.current_kind(), self.position())
    }

    pub(in crate::parser::grammar) fn at_expression_rhs_declaration_boundary(&mut self) -> bool {
        if !self.newline_before_current() || !self.at_declaration_start(true) {
            return false;
        }
        if !self.at(K::FunKw) {
            return true;
        }
        if self.nth_kind(1) == K::LParen {
            return false;
        }

        let mut angle_depth = 0usize;
        for offset in 1..256 {
            match self.nth_kind(offset) {
                K::Dot if angle_depth == 0 => return false,
                K::LParen if angle_depth == 0 => return true,
                K::Lt => angle_depth += 1,
                K::Gt if angle_depth > 0 => angle_depth -= 1,
                K::Eof | K::Semicolon | K::DoubleSemicolon | K::RBrace => return true,
                _ => {}
            }
        }
        true
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
        let position = self.position();
        self.current_kind() == K::Question
            && self.nth_kind(1) == K::Dot
            && self.tokens_are_adjacent(position, 2)
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
            self.eat_asserted(K::At);
        }
    }

    pub(in crate::parser::grammar) fn parse_optional_typed_label_reference(&mut self) {
        if !self.at(K::At) {
            return;
        }

        let label = self.start();
        self.bump();
        if self.at(K::Identifier) {
            self.bump();
        } else {
            let diagnostic = self.pending_expected("expected label name");
            self.missing_required_slot(
                label.anchor(),
                crate::shape::label_reference::Slot::label as u16,
                [diagnostic],
            );
        }
        self.complete(label, K::LabelReference);
    }

    pub(in crate::parser::grammar) fn complete_missing_expression(
        &mut self,
        message: &'static str,
    ) -> CompletedMarker {
        let expression = self.start();
        let diagnostic = self.pending_expected(message);

        self.complete_recovery(expression, K::BogusExpression, [diagnostic])
    }

    pub(in crate::parser::grammar) fn complete_missing_parenthesized_expression(
        &mut self,
        message: &'static str,
    ) -> CompletedMarker {
        let condition = self.start();
        let open = self.pending_expected("expected '(' before condition");
        self.missing_required_slot(
            condition.anchor(),
            crate::shape::parenthesized_expression::Slot::open_paren as u16,
            [open],
        );
        let expression = self.pending_expected(message);
        self.missing_required_slot(
            condition.anchor(),
            crate::shape::parenthesized_expression::Slot::expression as u16,
            [expression],
        );
        let close = self.pending_expected("expected ')' after condition");
        self.missing_required_slot(
            condition.anchor(),
            crate::shape::parenthesized_expression::Slot::close_paren as u16,
            [close],
        );
        self.complete(condition, K::ParenthesizedExpression)
    }
}

#[derive(Clone, Copy)]
struct BinaryOperatorInfo {
    precedence: u8,
}
