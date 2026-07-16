use jolt_syntax::{CompletedMarker, UnresolvedDiagnosticOwner};

use crate::KotlinSyntaxKind as K;

use super::Parser;

impl Parser<'_> {
    pub(super) fn parse_type_reference_until(&mut self, stops: &[K]) {
        let marker = self.start();
        if self.at_type_stop(stops, None)
            || self.at_eof()
            || self.at_type_recovery_declaration_boundary()
        {
            self.complete_missing_type();
        } else {
            self.parse_type_until(stops, None);
        }
        self.complete(marker, K::TypeReference);
    }

    pub(in crate::parser::grammar) fn parse_type_reference_until_position(
        &mut self,
        stop_position: usize,
    ) {
        let marker = self.start();
        if self.position() >= stop_position || self.at_eof() {
            self.complete_missing_type();
        } else {
            self.parse_type_until(&[], Some(stop_position));
        }
        self.complete(marker, K::TypeReference);
    }

    fn parse_type_until(&mut self, stops: &[K], stop_position: Option<usize>) -> CompletedMarker {
        let mut ty = self.parse_type_atom(stops, stop_position);

        loop {
            let is_function_type_arrow = self.current_kind() == K::Arrow
                && stops.contains(&K::Arrow)
                && type_can_continue_with_arrow(ty.kind());
            if (self.at_type_stop(stops, stop_position) && !is_function_type_arrow) || self.at_eof()
            {
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
                    let form = self.precede(ty);
                    self.bump();
                    self.parse_type_atom(stops, stop_position);
                    let form = self.complete(form, K::IntersectionDefinitelyNonNullableType);
                    let definitely_non_nullable = self.precede(form);
                    ty = self.complete(definitely_non_nullable, K::DefinitelyNonNullableType);
                }
                K::BangBang => {
                    let form = self.precede(ty);
                    self.bump();
                    let form = self.complete(form, K::BangDefinitelyNonNullableType);
                    let definitely_non_nullable = self.precede(form);
                    ty = self.complete(definitely_non_nullable, K::DefinitelyNonNullableType);
                }
                K::Dot if self.nth_kind(1) == K::LParen => {
                    let receiver = self.precede(ty);
                    self.bump();
                    self.parse_type_atom(stops, stop_position);
                    ty = self.complete(receiver, K::ReceiverType);
                }
                K::Arrow => {
                    let form = self.precede(ty);
                    self.bump();
                    self.parse_type_until(stops, stop_position);
                    let form = self.complete(form, K::ArrowFunctionType);
                    let function = self.precede(form);
                    ty = self.complete(function, K::FunctionType);
                }
                _ => break,
            }
        }

        ty
    }

    fn parse_type_atom(&mut self, stops: &[K], stop_position: Option<usize>) -> CompletedMarker {
        let marker = self.start();
        let prefix = self.start();
        self.parse_type_prefix_annotations();

        match self.current_kind() {
            _ if self.at_soft_keyword("suspend") => {
                self.abandon(prefix);
                let form = self.start();
                self.bump();
                self.parse_type_until(stops, stop_position);
                self.complete(form, K::SuspendedFunctionType);
                self.complete(marker, K::FunctionType)
            }
            _ if self.at_soft_keyword("context") && self.nth_kind(1) == K::LParen => {
                self.abandon(prefix);
                self.bump();
                self.parse_parenthesized_type_contents(K::FunctionTypeParameterSeparatedList);
                if !self.at_type_stop(stops, stop_position) && !self.at_eof() {
                    self.parse_type_until(stops, stop_position);
                }
                self.complete(marker, K::ContextFunctionType)
            }
            K::LParen => {
                self.complete(prefix, K::AnnotationList);
                self.parse_parenthesized_type_contents(K::ParenthesizedTypeEntryList);
                self.complete(marker, K::ParenthesizedType)
            }
            kind if self.at_identifier_like() || is_literal_kind(kind) => {
                let annotations = self.complete(prefix, K::AnnotationList);
                let segment = self.precede(annotations);
                self.parse_user_type_tail(segment, stop_position);
                self.complete(marker, K::UserType)
            }
            _ => {
                self.abandon(prefix);
                let diagnostic = self.expected_here("expected type");
                if !self.at_type_stop(stops, stop_position) && !self.at_eof() {
                    self.bump();
                }
                self.own_diagnostic(diagnostic, UnresolvedDiagnosticOwner::node(marker.anchor()));
                self.complete(marker, K::BogusType)
            }
        }
    }

    fn parse_type_prefix_annotations(&mut self) {
        while self.at(K::At) || self.at(K::Hash) {
            let before = self.position();
            self.parse_type_prefix_annotation();
            self.ensure_progress(before, "expected type annotation");
        }
    }

    fn parse_type_prefix_annotation(&mut self) {
        let marker = self.start();
        let _ = self.eat(K::At) || self.eat(K::Hash);
        if self.at_annotation_use_site_target() && self.nth_kind(1) == K::Colon {
            let target = self.start();
            self.bump();
            self.bump();
            self.complete(target, K::AnnotationUseSiteTarget);
        }
        self.parse_qualified_name();
        if self.type_prefix_annotation_has_argument_list() {
            self.parse_annotation_argument_list();
        }
        self.complete(marker, K::Annotation);
    }

    fn type_prefix_annotation_has_argument_list(&mut self) -> bool {
        if !self.at(K::LParen) || self.position() == 0 {
            return false;
        }

        self.tokens_are_adjacent(self.position() - 1, 2)
    }

    fn parse_user_type_tail(
        &mut self,
        first_segment: jolt_syntax::Marker,
        stop_position: Option<usize>,
    ) {
        let first_segment = self.parse_user_type_segment_tail(first_segment);
        let segments = self.precede(first_segment);
        while !self.at_position_stop(stop_position) {
            if self.at(K::Dot) && self.nth_kind(1) != K::LParen {
                let separator_position = self.position();
                self.bump();
                let crosses_line = self.newline_between(separator_position, self.position());
                let segment = self.start();
                let annotations = self.start();
                self.parse_type_prefix_annotations();
                self.complete(annotations, K::AnnotationList);
                if !crosses_line
                    && (self.at_identifier_like() || is_literal_kind(self.current_kind()))
                {
                    self.parse_user_type_segment_tail(segment);
                } else {
                    let diagnostic = self.expected_here("expected type segment");
                    self.own_diagnostic(
                        diagnostic,
                        UnresolvedDiagnosticOwner::node(segment.anchor()),
                    );
                    self.complete(segment, K::BogusUserTypeSegment);
                    break;
                }
            } else if self.at(K::Range) {
                let segment = self.start();
                let diagnostic = self.expected_here("expected one '.' between type segments");
                self.bump();
                self.parse_type_prefix_annotations();
                if self.at_identifier_like() || is_literal_kind(self.current_kind()) {
                    self.bump();
                    if self.at(K::Lt) {
                        self.parse_type_argument_list();
                    }
                } else if self.at(K::Lt) {
                    self.parse_type_argument_list();
                }
                self.own_diagnostic(
                    diagnostic,
                    UnresolvedDiagnosticOwner::node(segment.anchor()),
                );
                self.complete(segment, K::BogusUserTypeSegment);
            } else {
                break;
            }
        }
        self.complete(segments, K::UserTypeSegmentList);
    }

    fn parse_user_type_segment_tail(&mut self, segment: jolt_syntax::Marker) -> CompletedMarker {
        if self.at_identifier_like() {
            self.parse_name();
            if self.at(K::Lt) {
                self.parse_type_argument_list();
            }
            self.complete(segment, K::UserTypeSegment)
        } else {
            let diagnostic = self.unexpected_here("expected identifier in type segment");
            self.bump();
            self.own_diagnostic(
                diagnostic,
                UnresolvedDiagnosticOwner::node(segment.anchor()),
            );
            self.complete(segment, K::BogusUserTypeSegment)
        }
    }

    fn parse_parenthesized_type_contents(&mut self, entries_kind: K) {
        self.expect(K::LParen, "expected '(' in type");
        let entries = self.start();
        let mut expect_parameter = true;
        while !matches!(self.current_kind(), K::RParen | K::Eof) {
            let before = self.position();
            if self.eat(K::Comma) {
                if expect_parameter && !matches!(self.current_kind(), K::RParen | K::Eof) {
                    let error = self.start();
                    let diagnostic =
                        self.unexpected_here("expected function type parameter between commas");
                    self.own_diagnostic(
                        diagnostic,
                        UnresolvedDiagnosticOwner::node(error.anchor()),
                    );
                    self.complete(error, K::BogusFunctionTypeParameter);
                }
                expect_parameter = true;
                continue;
            }
            self.parse_function_type_parameter();
            expect_parameter = false;
            self.ensure_progress(before, "expected type");
        }
        self.complete(entries, entries_kind);
        self.expect(K::RParen, "expected ')' in type");
    }

    fn parse_function_type_parameter(&mut self) {
        let parameter = self.start();
        if self.at_identifier_like() && self.nth_kind(1) == K::Colon {
            self.parse_name();
            self.bump();
        }
        self.parse_type_reference_until(&[K::Comma, K::RParen]);
        self.complete(parameter, K::FunctionTypeParameter);
    }

    fn at_type_stop(&mut self, stops: &[K], stop_position: Option<usize>) -> bool {
        self.at_position_stop(stop_position)
            || stops.contains(&self.current_kind())
            || stops.iter().any(|kind| match kind {
                K::WhereKw => self.at_soft_keyword("where"),
                K::GetKw => self.at_soft_keyword("get"),
                K::SetKw => self.at_soft_keyword("set"),
                _ => false,
            })
    }

    fn at_position_stop(&self, stop_position: Option<usize>) -> bool {
        stop_position.is_some_and(|position| self.position() >= position)
    }

    fn at_type_recovery_declaration_boundary(&mut self) -> bool {
        self.newline_before_current()
            && (self.at_declaration_start(false)
                || matches!(self.current_kind(), K::PackageKw | K::ImportKw))
    }

    pub(super) fn parse_type_argument_list(&mut self) {
        let marker = self.start();
        self.expect(K::Lt, "expected type argument list");
        let entries = self.start();
        let mut expect_argument = true;
        while !matches!(self.current_kind(), K::Gt | K::Eof) {
            let before = self.position();
            if self.eat(K::Comma) {
                if expect_argument && !matches!(self.current_kind(), K::Gt | K::Eof) {
                    let error = self.start();
                    let diagnostic =
                        self.malformed_type_argument_list_here("malformed type argument list");
                    self.own_diagnostic(
                        diagnostic,
                        UnresolvedDiagnosticOwner::node(error.anchor()),
                    );
                    self.complete(error, K::BogusTypeArgument);
                }
                expect_argument = true;
                continue;
            }

            let argument = self.start();
            if self.eat(K::Star) {
                if matches!(self.current_kind(), K::Comma | K::Gt | K::Eof) {
                    self.complete(argument, K::StarProjection);
                } else {
                    let diagnostic = self.malformed_type_argument_list_here(
                        "star projection cannot include a simultaneous type",
                    );
                    self.parse_type_reference_until(&[K::Comma, K::Gt]);
                    self.own_diagnostic(
                        diagnostic,
                        UnresolvedDiagnosticOwner::node(argument.anchor()),
                    );
                    self.complete(argument, K::BogusTypeArgument);
                }
                expect_argument = false;
                continue;
            }
            if self.at(K::InKw) || self.at_soft_keyword("out") {
                self.bump();
                self.parse_type_reference_until(&[K::Comma, K::Gt]);
                self.complete(argument, K::TypeProjection);
            } else {
                self.abandon(argument);
                self.parse_type_reference_until(&[K::Comma, K::Gt]);
            }
            expect_argument = false;
            self.ensure_progress(before, "expected type argument");
        }
        self.complete(entries, K::TypeProjectionSeparatedList);
        self.expect(K::Gt, "expected '>' after type arguments");
        self.complete(marker, K::TypeArgumentList);
    }

    fn complete_missing_type(&mut self) {
        let missing = self.start();
        let diagnostic = self.expected_here("expected type");
        self.own_diagnostic(
            diagnostic,
            UnresolvedDiagnosticOwner::node(missing.anchor()),
        );
        self.complete(missing, K::BogusType);
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
