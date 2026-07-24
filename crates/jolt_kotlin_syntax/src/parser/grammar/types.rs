use std::ops::{Deref, DerefMut};

use jolt_syntax::CompletedMarker;

use crate::KotlinSyntaxKind as K;
use crate::parser::source::TokenCursor;

use super::Parser;

#[derive(Clone, Copy)]
struct TypeStops<'a> {
    kinds: &'a [K],
    position: Option<usize>,
}

impl<'a> TypeStops<'a> {
    const fn new(kinds: &'a [K]) -> Self {
        Self {
            kinds,
            position: None,
        }
    }

    const fn at_position(position: usize) -> Self {
        Self {
            kinds: &[],
            position: Some(position),
        }
    }

    fn contains(self, position: usize, kind: K, text: Option<&str>) -> bool {
        self.position == Some(position)
            || self.kinds.contains(&kind)
            || self.kinds.iter().any(|stop| {
                matches!(
                    (stop, text),
                    (K::WhereKw, Some("where")) | (K::GetKw, Some("get")) | (K::SetKw, Some("set"))
                )
            })
    }
}

struct TypeParser<'parser, 'source, 'stops> {
    parser: &'parser mut Parser<'source>,
    outer: TypeStops<'stops>,
    selected: Option<usize>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TypeRecoveryEnd {
    Outer(usize),
    Local(usize),
}

impl TypeRecoveryEnd {
    const fn position(self) -> usize {
        match self {
            Self::Outer(position) | Self::Local(position) => position,
        }
    }
}

impl<'parser, 'source, 'stops> TypeParser<'parser, 'source, 'stops> {
    fn new(parser: &'parser mut Parser<'source>, outer: TypeStops<'stops>) -> Self {
        Self {
            parser,
            outer,
            selected: None,
        }
    }

    fn is_stopped_at(&self, position: usize) -> bool {
        self.selected == Some(position)
    }
}

impl<'source> Deref for TypeParser<'_, 'source, '_> {
    type Target = Parser<'source>;

    fn deref(&self) -> &Self::Target {
        self.parser
    }
}

impl DerefMut for TypeParser<'_, '_, '_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.parser
    }
}

impl TypeParser<'_, '_, '_> {
    fn unclosed_type_recovery_stop(
        &mut self,
        mut cursor: TokenCursor,
        stops: TypeStops<'_>,
    ) -> Option<TypeRecoveryEnd> {
        let (mut angles, mut parens, mut brackets, mut braces) = (0usize, 0usize, 0usize, 0usize);
        let mut deferred = None;
        loop {
            let position = cursor.position();
            let current = self.kind_at(position);
            let outside_delimiters = parens == 0 && brackets == 0 && braces == 0;
            let outside = outside_delimiters && angles == 0;
            if current == K::Eof
                || current == K::RParen && parens == 0
                || current == K::RBracket && brackets == 0
                || current == K::RBrace && braces == 0
                || current == K::Gt && outside_delimiters && angles == 0
            {
                return deferred;
            }

            let requested = if self.is_requested_type_stop_at(self.outer, position) {
                Some(TypeRecoveryEnd::Outer(position))
            } else if self.is_requested_type_stop_at(stops, position) {
                Some(TypeRecoveryEnd::Local(position))
            } else {
                None
            };
            if outside {
                match requested {
                    Some(TypeRecoveryEnd::Outer(_)) => return requested,
                    Some(TypeRecoveryEnd::Local(_)) => return None,
                    None => {}
                }
            }
            if !outside && deferred.is_none() {
                deferred = requested;
            }

            match current {
                K::Lt if outside_delimiters => angles += 1,
                K::Gt if outside_delimiters => angles -= 1,
                K::LParen => parens += 1,
                K::RParen => parens -= 1,
                K::LBracket => brackets += 1,
                K::RBracket => brackets -= 1,
                K::LBrace => braces += 1,
                K::RBrace => braces -= 1,
                _ => {}
            }
            cursor.bump(&mut self.buffer);
            if angles == 0 && parens == 0 && brackets == 0 && braces == 0 {
                deferred = None;
            }
        }
    }

    fn parse_type_reference(&mut self, stops: TypeStops<'_>) {
        let marker = self.start();
        if self.at_type_stop(stops) || self.at_eof() || self.at_type_recovery_declaration_boundary()
        {
            self.complete_missing_type();
        } else {
            self.parse_type_until(stops);
        }
        self.complete(marker, K::TypeReference);
    }

    fn parse_type_until(&mut self, stops: TypeStops<'_>) -> CompletedMarker {
        self.parse_type_until_with_recovery(stops, K::BogusType)
    }

    fn parse_required_function_type_until(&mut self, stops: TypeStops<'_>) -> CompletedMarker {
        self.parse_type_until_with_recovery(stops, K::FunctionType)
    }

    fn parse_type_until_with_recovery(
        &mut self,
        stops: TypeStops<'_>,
        recovery_kind: K,
    ) -> CompletedMarker {
        if self.enter_syntax_nesting() {
            let ty = self.parse_type_until_inner(stops);
            self.leave_syntax_nesting();
            return ty;
        }

        self.parse_excessive_type(stops, recovery_kind)
    }

    fn parse_type_until_inner(&mut self, stops: TypeStops<'_>) -> CompletedMarker {
        let mut ty = self.parse_type_atom(stops);

        loop {
            let is_function_type_arrow = self.current_kind() == K::Arrow
                && stops.kinds.contains(&K::Arrow)
                && type_can_continue_with_arrow(ty.kind());
            if self.is_stopped_at(self.position())
                || self.at_requested_type_stop(stops) && !is_function_type_arrow
                || self.at_eof()
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
                    self.parse_type_atom(stops);
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
                    self.parse_type_atom(stops);
                    ty = self.complete(receiver, K::ReceiverType);
                }
                K::Arrow => {
                    let form = self.precede(ty);
                    self.bump();
                    self.parse_type_until(stops);
                    let form = self.complete(form, K::ArrowFunctionType);
                    let function = self.precede(form);
                    ty = self.complete(function, K::FunctionType);
                }
                _ => break,
            }
        }

        ty
    }

    fn parse_type_atom(&mut self, stops: TypeStops<'_>) -> CompletedMarker {
        let marker = self.start();
        let prefix = self.start();
        let prefix_start = self.position();
        self.parse_type_prefix_annotations();

        match self.current_kind() {
            _ if self.at_soft_keyword("suspend") => {
                self.finish_optional_type_annotations(prefix, prefix_start);
                let form = self.start();
                self.bump();
                self.parse_required_function_type_until(stops);
                self.complete(form, K::SuspendedFunctionType);
                self.complete(marker, K::FunctionType)
            }
            _ if self.at_soft_keyword("context") && self.nth_kind(1) == K::LParen => {
                self.finish_optional_type_annotations(prefix, prefix_start);
                self.bump();
                self.parse_parenthesized_type_contents(
                    K::FunctionTypeParameterSeparatedList,
                    marker.anchor(),
                    crate::shape::context_function_type::Slot::close_paren as u16,
                );
                if !self.at_type_stop(stops) && !self.at_eof() {
                    self.parse_required_function_type_until(stops);
                }
                self.complete(marker, K::ContextFunctionType)
            }
            K::LParen => {
                self.complete(prefix, K::AnnotationList);
                self.parse_parenthesized_type_contents(
                    K::ParenthesizedTypeEntryList,
                    marker.anchor(),
                    crate::shape::parenthesized_type::Slot::close_paren as u16,
                );
                self.complete(marker, K::ParenthesizedType)
            }
            kind if self.at_identifier_like() || is_literal_kind(kind) => {
                let segment = if self.position() == prefix_start {
                    self.abandon(prefix);
                    self.start()
                } else {
                    let annotations = self.complete(prefix, K::AnnotationList);
                    self.precede(annotations)
                };
                self.parse_user_type_tail(segment, stops.position);
                self.complete(marker, K::UserType)
            }
            _ => {
                self.finish_optional_type_annotations(prefix, prefix_start);
                let diagnostic = self.pending_expected("expected type");
                let mut consumed = false;
                while !(self.at_type_stop(stops)
                    || self.at_eof()
                    || self.at_type_recovery_declaration_boundary()
                    || consumed && self.newline_before_current())
                {
                    self.bump();
                    consumed = true;
                }

                self.complete_recovery(marker, K::BogusType, [diagnostic])
            }
        }
    }

    fn finish_optional_type_annotations(&mut self, annotations: jolt_syntax::Marker, start: usize) {
        if self.position() == start {
            self.abandon(annotations);
        } else {
            self.complete(annotations, K::AnnotationList);
        }
    }

    fn parse_type_prefix_annotations(&mut self) {
        while self.at(K::At) || self.at(K::Hash) {
            let before = self.position();
            self.parse_type_prefix_annotation();
            debug_assert!(self.position() > before);
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

        let index = self.position() - 1;
        self.tokens_are_adjacent(index, 2)
    }

    fn parse_user_type_tail(
        &mut self,
        first_segment: jolt_syntax::Marker,
        stop_position: Option<usize>,
    ) {
        let first_segment = self.parse_user_type_segment_tail(first_segment);
        let segments = self.precede(first_segment);
        let mut list_recovery = None;
        while !self.at_position_stop(stop_position) && !self.is_stopped_at(self.position()) {
            if self.at(K::Dot) && self.nth_kind(1) != K::LParen {
                let separator_position = self.position();
                self.bump();
                let segment_position = self.position();
                let crosses_line = self.newline_between(separator_position, segment_position);
                let segment = self.start();
                if self.at(K::At) || self.at(K::Hash) {
                    let annotations = self.start();
                    self.parse_type_prefix_annotations();
                    self.complete(annotations, K::AnnotationList);
                }
                if !crosses_line
                    && (self.at_identifier_like() || is_literal_kind(self.current_kind()))
                {
                    self.parse_user_type_segment_tail(segment);
                } else {
                    let diagnostic = self.pending_expected("expected type segment");

                    self.complete_recovery(segment, K::BogusUserTypeSegment, [diagnostic]);
                    break;
                }
            } else if self.at(K::Range) {
                if list_recovery.is_none() {
                    list_recovery =
                        Some(self.pending_expected("expected one '.' between type segments"));
                }
                self.bump();
                let segment = self.start();
                self.parse_type_prefix_annotations();
                if self.at_identifier_like() || is_literal_kind(self.current_kind()) {
                    self.parse_user_type_segment_tail(segment);
                } else {
                    let diagnostic = self.pending_expected("expected type segment");
                    self.missing_required_slot(
                        segment.anchor(),
                        crate::shape::user_type_segment::Slot::name as u16,
                        [diagnostic],
                    );
                    self.complete(segment, K::UserTypeSegment);
                }
            } else {
                break;
            }
        }
        if let Some(diagnostic) = list_recovery {
            self.complete_recovery(segments, K::UserTypeSegmentList, [diagnostic]);
        } else {
            self.complete(segments, K::UserTypeSegmentList);
        }
    }

    fn parse_user_type_segment_tail(&mut self, segment: jolt_syntax::Marker) -> CompletedMarker {
        if self.at_identifier_like() {
            self.bump();
            if self.at(K::Lt) {
                self.parse_type_argument_list();
            }
            self.complete(segment, K::UserTypeSegment)
        } else {
            let diagnostic = self.pending_unexpected("expected identifier in type segment");
            self.bump();

            self.complete_recovery(segment, K::BogusUserTypeSegment, [diagnostic])
        }
    }

    fn parse_parenthesized_type_contents(
        &mut self,
        entries_kind: K,
        owner: jolt_syntax::NodeAnchor,
        close_slot: u16,
    ) {
        self.eat_asserted(K::LParen);
        let entries = self.start();
        let mut expect_parameter = true;
        while !matches!(self.current_kind(), K::RParen | K::Eof)
            && !self.is_stopped_at(self.position())
        {
            let before = self.position();
            if self.at(K::Comma) {
                if expect_parameter && !matches!(self.nth_kind(1), K::RParen | K::Eof) {
                    let error = self.start();
                    let diagnostic =
                        self.pending_unexpected("expected function type parameter between commas");

                    self.complete_recovery(error, K::BogusFunctionTypeParameter, [diagnostic]);
                }
                self.bump();
                expect_parameter = true;
                continue;
            }
            self.parse_function_type_parameter();
            expect_parameter = false;
            debug_assert!(self.position() > before || self.is_stopped_at(self.position()));
        }
        self.complete(entries, entries_kind);
        if self.is_stopped_at(self.position()) || !self.eat(K::RParen) {
            let diagnostic = self.pending_expected("expected ')' in type");
            self.missing_required_slot(owner, close_slot, [diagnostic]);
        }
    }

    fn parse_function_type_parameter(&mut self) {
        let parameter = self.start();
        if self.at_identifier_like() && self.nth_kind(1) == K::Colon {
            self.parse_name();
            self.bump();
        }
        self.parse_type_reference(TypeStops::new(&[K::Comma, K::RParen]));
        self.complete(parameter, K::FunctionTypeParameter);
    }

    fn at_type_stop(&mut self, stops: TypeStops<'_>) -> bool {
        self.is_stopped_at(self.position()) || self.at_requested_type_stop(stops)
    }

    fn at_requested_type_stop(&mut self, stops: TypeStops<'_>) -> bool {
        let position = self.position();
        let kind = self.current_kind();
        self.at_position_stop(stops.position)
            || stops.contains(position, kind, self.text_at(position))
    }

    fn is_requested_type_stop_at(&mut self, stops: TypeStops<'_>, position: usize) -> bool {
        let kind = self.kind_at(position);
        stops.contains(position, kind, self.text_at(position))
    }

    fn at_position_stop(&self, stop_position: Option<usize>) -> bool {
        stop_position.is_some_and(|position| self.position() >= position)
    }

    fn at_type_recovery_declaration_boundary(&mut self) -> bool {
        self.newline_before_current()
            && (self.at_declaration_start(false)
                || matches!(self.current_kind(), K::PackageKw | K::ImportKw))
    }

    fn parse_excessive_type(&mut self, stops: TypeStops<'_>, kind: K) -> CompletedMarker {
        let cursor = self.fork_cursor();
        debug_assert_eq!(self.selected, None);
        let recovery_end = self.unclosed_type_recovery_stop(cursor, stops);
        if let Some(TypeRecoveryEnd::Outer(position)) = recovery_end {
            self.selected = Some(position);
        }
        let ty = self.start();
        let diagnostic = self.pending_excessive_syntax_nesting();
        let (mut angles, mut parens, mut brackets, mut braces) = (0usize, 0usize, 0usize, 0usize);
        let mut consumed_outside = false;

        loop {
            let current = self.current_kind();
            let outside_delimiters = parens == 0 && brackets == 0 && braces == 0;
            let outside = outside_delimiters && angles == 0;
            if recovery_end.is_some_and(|end| end.position() == self.position())
                || current == K::Eof
                || outside && self.at_position_stop(stops.position)
                || current == K::RParen && parens == 0
                || current == K::RBracket && brackets == 0
                || current == K::RBrace && braces == 0
                || current == K::Gt && outside_delimiters && angles == 0
                || outside
                    && (self.at_requested_type_stop(TypeStops::new(stops.kinds))
                        || self.at_type_recovery_declaration_boundary()
                        || consumed_outside
                            && self.newline_before_current()
                            && !matches!(current, K::Question | K::BangBang | K::Amp))
            {
                break;
            }

            consumed_outside |= outside;
            match current {
                K::Lt if outside_delimiters => angles += 1,
                K::Gt if outside_delimiters => angles -= 1,
                K::LParen => parens += 1,
                K::RParen => parens -= 1,
                K::LBracket => brackets += 1,
                K::RBracket => brackets -= 1,
                K::LBrace => braces += 1,
                K::RBrace => braces -= 1,
                _ => {}
            }
            self.bump();
        }

        self.complete_recovery(ty, kind, [diagnostic])
    }

    fn parse_type_argument_list(&mut self) {
        let marker = self.start();
        self.eat_asserted(K::Lt);
        let entries = self.start();
        let mut expect_argument = true;
        while !matches!(self.current_kind(), K::Gt | K::Eof) && !self.is_stopped_at(self.position())
        {
            let before = self.position();
            if self.at(K::Comma) {
                if expect_argument && !matches!(self.nth_kind(1), K::Gt | K::Eof) {
                    let error = self.start();
                    let diagnostic =
                        self.malformed_type_argument_list_here("malformed type argument list");

                    self.complete_recovery(error, K::BogusTypeArgument, [diagnostic]);
                }
                self.bump();
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
                    self.parse_type_reference(TypeStops::new(&[K::Comma, K::Gt]));

                    self.complete_recovery(argument, K::BogusTypeArgument, [diagnostic]);
                }
                expect_argument = false;
                continue;
            }
            if self.at(K::InKw) || self.at_soft_keyword("out") {
                self.bump();
                self.parse_type_reference(TypeStops::new(&[K::Comma, K::Gt]));
                self.complete(argument, K::TypeProjection);
            } else {
                self.abandon(argument);
                self.parse_type_reference(TypeStops::new(&[K::Comma, K::Gt]));
            }
            expect_argument = false;
            debug_assert!(self.position() > before || self.is_stopped_at(self.position()));
        }
        self.complete(entries, K::TypeProjectionSeparatedList);
        if self.is_stopped_at(self.position()) || !self.eat(K::Gt) {
            let diagnostic = self.pending_expected("expected '>' after type arguments");
            self.missing_required_slot(
                marker.anchor(),
                crate::shape::type_argument_list::Slot::close_angle as u16,
                [diagnostic],
            );
        }
        self.complete(marker, K::TypeArgumentList);
    }

    fn complete_missing_type(&mut self) {
        let missing = self.start();
        let diagnostic = self.pending_expected("expected type");

        self.complete_recovery(missing, K::BogusType, [diagnostic]);
    }
}

impl Parser<'_> {
    pub(super) fn parse_type_reference_until(&mut self, kinds: &[K]) {
        let stops = TypeStops::new(kinds);
        let mut parser = TypeParser::new(self, stops);
        parser.parse_type_reference(stops);
        debug_assert!(parser.selected.is_none_or(|stop| stop == parser.position()));
    }

    pub(in crate::parser::grammar) fn parse_type_reference_until_position(
        &mut self,
        position: usize,
    ) {
        let stops = TypeStops::at_position(position);
        let mut parser = TypeParser::new(self, stops);
        parser.parse_type_reference(stops);
        debug_assert!(parser.selected.is_none_or(|stop| stop == parser.position()));
    }

    pub(super) fn parse_type_argument_list(&mut self) {
        TypeParser::new(self, TypeStops::new(&[])).parse_type_argument_list();
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
