use jolt_syntax::CompletedMarker;

use crate::KotlinSyntaxKind as K;

use super::Parser;

impl Parser<'_> {
    pub(super) fn parse_type_reference_until(&mut self, stops: &[K]) {
        let marker = self.start();
        if self.at_type_stop(stops) || self.at_eof() {
            let error = self.start();
            self.expected_here("expected type");
            self.complete(error, K::ErrorNode);
        } else {
            self.parse_type_until(stops);
        }
        self.complete(marker, K::TypeReference);
    }

    fn parse_type_until(&mut self, stops: &[K]) -> CompletedMarker {
        let mut ty = self.parse_type_atom(stops);

        loop {
            let is_function_type_arrow = self.current_kind() == K::Arrow
                && stops.contains(&K::Arrow)
                && type_can_continue_with_arrow(ty.kind());
            if (self.at_type_stop(stops) && !is_function_type_arrow) || self.at_eof() {
                break;
            }
            if self.newline_before_current()
                && !matches!(self.current_kind(), K::Question | K::BangBang | K::Amp)
            {
                break;
            }

            match self.current_kind() {
                K::Question => {
                    let nullable = self.precede(ty);
                    self.bump();
                    ty = self.complete(nullable, K::NullableType);
                }
                K::Amp => {
                    let definitely_non_nullable = self.precede(ty);
                    self.bump();
                    self.parse_type_atom(stops);
                    ty = self.complete(definitely_non_nullable, K::DefinitelyNonNullableType);
                }
                K::BangBang => {
                    let definitely_non_nullable = self.precede(ty);
                    self.bump();
                    ty = self.complete(definitely_non_nullable, K::DefinitelyNonNullableType);
                }
                K::Dot if self.nth_kind(1) == K::LParen => {
                    let receiver = self.precede(ty);
                    self.bump();
                    self.parse_type_atom(stops);
                    ty = self.complete(receiver, K::ReceiverType);
                }
                K::Arrow => {
                    let function = self.precede(ty);
                    self.bump();
                    self.parse_type_until(stops);
                    ty = self.complete(function, K::FunctionType);
                }
                _ => break,
            }
        }

        ty
    }

    fn parse_type_atom(&mut self, stops: &[K]) -> CompletedMarker {
        let marker = self.start();
        self.parse_type_prefix_annotations();

        match self.current_kind() {
            _ if self.at_soft_keyword("suspend") => {
                self.bump();
                self.parse_type_until(stops);
                self.complete(marker, K::FunctionType)
            }
            _ if self.at_soft_keyword("context") && self.nth_kind(1) == K::LParen => {
                self.bump();
                self.parse_parenthesized_type_contents();
                if !self.at_type_stop(stops) && !self.at_eof() {
                    self.parse_type_until(stops);
                }
                self.complete(marker, K::ContextFunctionType)
            }
            K::LParen => {
                self.parse_parenthesized_type_contents();
                self.complete(marker, K::ParenthesizedType)
            }
            kind if self.at_identifier_like() || is_literal_kind(kind) => {
                self.parse_user_type_tail();
                self.complete(marker, K::UserType)
            }
            _ => {
                self.expected_here("expected type");
                if !self.at_type_stop(stops) && !self.at_eof() {
                    self.bump();
                }
                self.complete(marker, K::ErrorNode)
            }
        }
    }

    fn parse_type_prefix_annotations(&mut self) {
        while self.at(K::At) || self.at(K::Hash) {
            self.parse_annotation();
        }
    }

    fn parse_user_type_tail(&mut self) {
        self.bump();
        if self.at(K::Lt) {
            self.parse_type_argument_list();
        }
        while self.at(K::Dot) && self.nth_kind(1) != K::LParen {
            self.bump();
            self.parse_type_prefix_annotations();
            if self.at_identifier_like() || is_literal_kind(self.current_kind()) {
                self.bump();
                if self.at(K::Lt) {
                    self.parse_type_argument_list();
                }
            } else {
                self.expected_here("expected type segment");
                break;
            }
        }
    }

    fn parse_parenthesized_type_contents(&mut self) {
        self.expect(K::LParen, "expected '(' in type");
        while !matches!(self.current_kind(), K::RParen | K::Eof) {
            let before = self.position();
            if self.eat(K::Comma) {
                continue;
            }
            self.parse_type_reference_until(&[K::Comma, K::RParen]);
            if self.position() == before {
                self.unexpected_here("expected type");
                self.bump();
            }
        }
        self.expect(K::RParen, "expected ')' in type");
    }

    fn at_type_stop(&mut self, stops: &[K]) -> bool {
        stops.contains(&self.current_kind())
            || stops.iter().any(|kind| match kind {
                K::WhereKw => self.at_soft_keyword("where"),
                K::GetKw => self.at_soft_keyword("get"),
                K::SetKw => self.at_soft_keyword("set"),
                _ => false,
            })
    }

    pub(super) fn parse_type_argument_list(&mut self) {
        let marker = self.start();
        self.expect(K::Lt, "expected type argument list");
        let projections = self.start();
        let mut expect_argument = true;
        while !matches!(self.current_kind(), K::Gt | K::Eof) {
            let before = self.position();
            if self.eat(K::Comma) {
                if expect_argument && !matches!(self.current_kind(), K::Gt | K::Eof) {
                    self.malformed_type_argument_list_here("malformed type argument list");
                    let error = self.start();
                    self.complete(error, K::ErrorNode);
                }
                expect_argument = true;
                continue;
            }

            let argument = self.start();
            let projection = self.start();
            if self.eat(K::Star) {
                self.complete(projection, K::TypeProjection);
                self.complete(argument, K::TypeArgument);
                expect_argument = false;
                continue;
            }
            if self.at(K::InKw) || self.at_soft_keyword("out") {
                self.bump();
            }
            self.parse_type_reference_until(&[K::Comma, K::Gt]);
            self.complete(projection, K::TypeProjection);
            self.complete(argument, K::TypeArgument);
            expect_argument = false;
            if self.position() == before {
                self.unexpected_here("expected type argument");
                self.bump();
            }
        }
        self.complete(projections, K::TypeProjectionList);
        self.expect(K::Gt, "expected '>' after type arguments");
        self.complete(marker, K::TypeArgumentList);
    }
}

fn is_literal_kind(kind: K) -> bool {
    matches!(
        kind,
        K::IntegerLiteral
            | K::FloatLiteral
            | K::CharacterLiteral
            | K::NullKw
            | K::TrueKw
            | K::FalseKw
    )
}

fn type_can_continue_with_arrow(kind: jolt_syntax::RawSyntaxKind) -> bool {
    kind == K::ParenthesizedType.to_raw() || kind == K::ReceiverType.to_raw()
}
