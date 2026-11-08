//! Types, type arguments, and annotation element values.

use jolt_syntax::{CompletedMarker, Marker};

use super::{
    EXPRESSION_STOPS, JavaSyntaxKind, Parser, TYPE_STOPS,
    support::{is_type_argument_recovery_boundary, is_type_argument_value_start},
};

impl Parser<'_> {
    pub(super) fn parse_type(&mut self) -> CompletedMarker {
        let ty = self.start();
        let mut lookahead = self.lookahead();
        lookahead.skip_annotations();
        let starts_class_type = lookahead.at_name_segment();

        let base_kind = if starts_class_type {
            let segments = self.start();
            let segment = self.start();
            self.parse_annotations();
            self.parse_class_type_tail_from(segments, segment);
            JavaSyntaxKind::ClassType
        } else {
            self.parse_annotations();
            if self.lookahead().at_primitive_type_start() {
                self.bump();
                JavaSyntaxKind::PrimitiveType
            } else {
                self.error_expected("expected type");
                return self.complete(ty, JavaSyntaxKind::Bogus);
            }
        };

        if self.starts_array_dimensions() {
            let completed = self.complete(ty, base_kind);
            let array = self.precede(completed);
            self.parse_array_dimensions();
            self.complete(array, JavaSyntaxKind::ArrayType)
        } else {
            self.complete(ty, base_kind)
        }
    }

    pub(super) fn parse_void_type(&mut self) -> CompletedMarker {
        let ty = self.start();
        if !self.eat(JavaSyntaxKind::VoidKw) {
            self.error_expected("expected `void`");
        }
        self.complete(ty, JavaSyntaxKind::VoidType)
    }

    pub(super) fn parse_class_type(&mut self) -> CompletedMarker {
        let ty = self.start();
        let mut lookahead = self.lookahead();
        lookahead.skip_annotations();

        if lookahead.at_name_segment() {
            let segments = self.start();
            let segment = self.start();
            self.parse_annotations();
            self.parse_class_type_tail_from(segments, segment);
            let completed = self.complete(ty, JavaSyntaxKind::ClassType);
            if self.starts_array_dimensions() {
                let error = self.precede(completed);
                self.error_expected("expected class or interface type");
                self.parse_array_dimensions();
                self.complete(error, JavaSyntaxKind::Bogus)
            } else {
                completed
            }
        } else {
            self.parse_annotations();
            self.error_expected("expected class or interface type");
            if self.lookahead().at_primitive_type_start() || self.at(JavaSyntaxKind::VoidKw) {
                self.bump();
                self.parse_array_dimensions();
            }
            self.complete(ty, JavaSyntaxKind::Bogus)
        }
    }

    pub(super) fn parse_reference_type(&mut self) -> CompletedMarker {
        let ty = self.parse_type();
        if JavaSyntaxKind::from_raw(ty.kind()) == JavaSyntaxKind::PrimitiveType {
            let error = self.precede(ty);
            self.error_expected("expected reference type");
            self.complete(error, JavaSyntaxKind::Bogus)
        } else {
            ty
        }
    }

    pub(super) fn parse_intersection_type(&mut self) -> CompletedMarker {
        let ty = self.parse_type();
        if !self.at(JavaSyntaxKind::Amp) {
            return ty;
        }

        let intersection = self.precede(ty);
        self.bump();
        let remaining = self.start();
        self.parse_type();
        while self.eat(JavaSyntaxKind::Amp) {
            self.parse_type();
        }
        self.complete(remaining, JavaSyntaxKind::TypeAmpList);
        self.complete(intersection, JavaSyntaxKind::IntersectionType)
    }

    pub(super) fn parse_class_intersection_type(&mut self) -> CompletedMarker {
        let ty = self.parse_class_type();
        if !self.at(JavaSyntaxKind::Amp) {
            return ty;
        }

        let intersection = self.precede(ty);
        self.bump();
        let remaining = self.start();
        self.parse_class_type();
        while self.eat(JavaSyntaxKind::Amp) {
            self.parse_class_type();
        }
        self.complete(remaining, JavaSyntaxKind::TypeAmpList);
        self.complete(intersection, JavaSyntaxKind::IntersectionType)
    }

    pub(super) fn parse_class_union_type(&mut self) -> CompletedMarker {
        let ty = self.parse_class_type();
        if !self.at(JavaSyntaxKind::Bar) {
            return ty;
        }

        let union = self.precede(ty);
        self.bump();
        let remaining = self.start();
        self.parse_class_type();
        while self.eat(JavaSyntaxKind::Bar) {
            self.parse_class_type();
        }
        self.complete(remaining, JavaSyntaxKind::TypeBarList);
        self.complete(union, JavaSyntaxKind::UnionType)
    }

    fn parse_class_type_tail_from(&mut self, segments: Marker, mut segment: Marker) {
        self.parse_class_type_name_run();
        self.parse_optional_type_argument_list();
        self.complete(segment, JavaSyntaxKind::ClassTypeSegmentNode);
        while self.dot_is_followed_by_annotated_name() {
            self.bump();
            segment = self.start();
            self.parse_annotations();
            self.parse_class_type_name_run();
            self.parse_optional_type_argument_list();
            self.complete(segment, JavaSyntaxKind::ClassTypeSegmentNode);
        }
        self.complete(segments, JavaSyntaxKind::ClassTypeSegmentList);
    }

    pub(super) fn parse_class_type_name_run(&mut self) {
        let name = self.start();
        let mut lookahead = self.lookahead();
        lookahead.bump();
        let qualified = if lookahead.at(JavaSyntaxKind::Dot) {
            lookahead.bump();
            lookahead.skip_annotations();
            lookahead.at_name_segment()
        } else {
            false
        };
        if !qualified {
            self.bump();
            self.complete(name, JavaSyntaxKind::Name);
            return;
        }

        let first_segment = self.start();
        self.parse_annotations();
        self.bump();
        self.complete(first_segment, JavaSyntaxKind::QualifiedNameSegmentNode);
        self.bump();

        let remaining_segments = self.start();
        loop {
            let segment = self.start();
            self.parse_annotations();
            self.bump();
            self.complete(segment, JavaSyntaxKind::QualifiedNameSegmentNode);
            if self.at(JavaSyntaxKind::Lt) || !self.dot_is_followed_by_annotated_name() {
                break;
            }
            self.bump();
        }
        self.complete(remaining_segments, JavaSyntaxKind::NameSegmentDotList);
        self.complete(name, JavaSyntaxKind::QualifiedName);
    }

    pub(super) fn parse_optional_type_argument_list(&mut self) -> bool {
        if !self.at(JavaSyntaxKind::Lt) {
            return false;
        }

        let list = self.start();
        self.bump();
        self.generic_depth += 1;
        let arguments = self.start();
        self.parse_comma_separated(
            "expected type argument",
            Self::at_type_argument_close,
            Self::parse_type_argument,
        );
        self.complete(arguments, JavaSyntaxKind::TypeArgumentSeparatedList);
        if !self.eat_type_argument_close() {
            self.error_expected("expected `>` after type arguments");
        }
        self.generic_depth -= 1;
        self.complete(list, JavaSyntaxKind::TypeArgumentList);
        true
    }

    fn parse_type_argument(&mut self) {
        let argument = self.start();
        self.parse_annotations();
        if is_type_argument_value_start(self.current()) {
            self.parse_type_argument_value();
        } else {
            self.parse_malformed_type_argument_value();
        }
        self.complete(argument, JavaSyntaxKind::TypeArgument);
    }

    /// A type argument's type: the recursive owner of nested type arguments.
    pub(super) fn parse_type_argument_value(&mut self) {
        if self.nested(Self::parse_type_argument_value_inner).is_none() {
            self.recover_too_deep(TYPE_STOPS);
        }
    }

    fn parse_type_argument_value_inner(&mut self) {
        if self.at(JavaSyntaxKind::Question) {
            let wildcard = self.start();
            self.bump();
            if self.at(JavaSyntaxKind::ExtendsKw) || self.at(JavaSyntaxKind::SuperKw) {
                self.bump();
                self.parse_type();
            }
            self.complete(wildcard, JavaSyntaxKind::WildcardType);
        } else {
            self.parse_type();
        }
    }

    fn parse_malformed_type_argument_value(&mut self) {
        let ty = self.start();
        self.error_expected("expected type");
        let mut guard = self.loop_guard();
        loop {
            if is_type_argument_recovery_boundary(self.current()) || !self.check(&mut guard) {
                break;
            }
            if self.at(JavaSyntaxKind::At) {
                let annotation_start = self.position();
                self.parse_annotations();
                if is_type_argument_value_start(self.current()) {
                    self.parse_type_argument_value();
                    break;
                }
                if self.position() == annotation_start {
                    self.bump();
                }
                continue;
            }
            if is_type_argument_value_start(self.current()) {
                self.parse_type_argument_value();
                break;
            }
            self.bump();
        }
        self.complete(ty, JavaSyntaxKind::Bogus);
    }

    pub(super) fn parse_array_dimensions(&mut self) -> bool {
        if !self.starts_array_dimensions() {
            return false;
        }

        let dimensions = self.start();
        let mut guard = self.loop_guard();
        while self.starts_array_dimensions() && self.check(&mut guard) {
            let dimension = self.start();
            self.parse_annotations();
            if !self.eat(JavaSyntaxKind::LBracket) {
                self.error_expected("expected `[`");
            }
            if !self.eat(JavaSyntaxKind::RBracket) {
                self.error_expected("expected `]`");
            }
            self.complete(dimension, JavaSyntaxKind::ArrayDimension);
        }
        self.complete(dimensions, JavaSyntaxKind::ArrayDimensions);
        true
    }

    pub(super) fn parse_annotation_element_values(&mut self, stop: JavaSyntaxKind) {
        let arguments = self.start();
        self.parse_comma_separated(
            "expected annotation argument",
            move |parser| parser.at_expression_list_close(stop),
            move |parser| {
                if parser.at(JavaSyntaxKind::Comma) {
                    let bogus = parser.start();
                    parser.error_expected("expected annotation argument");
                    parser.complete(bogus, JavaSyntaxKind::Bogus);
                } else {
                    parser.parse_annotation_element_value_or_pair(stop);
                }
            },
        );
        self.complete(arguments, JavaSyntaxKind::AnnotationElementArgumentList);
    }

    pub(super) fn parse_annotation_element_value_or_pair(&mut self, stop: JavaSyntaxKind) {
        if self.lookahead().at_name_segment() && self.nth(1) == JavaSyntaxKind::Assign {
            let pair = self.start();
            self.bump();
            self.bump();
            self.parse_annotation_element_value(stop);
            self.complete(pair, JavaSyntaxKind::AnnotationElementValuePair);
        } else {
            self.parse_annotation_element_value(stop);
        }
    }

    pub(super) fn parse_annotation_element_value(&mut self, stop: JavaSyntaxKind) {
        let value = self.start();
        if self
            .nested(|parser| parser.parse_annotation_element_value_contents(stop))
            .is_none()
        {
            self.recover_too_deep(EXPRESSION_STOPS);
        }
        self.complete(value, JavaSyntaxKind::AnnotationElementValue);
    }

    pub(super) fn parse_annotation_element_value_contents(&mut self, stop: JavaSyntaxKind) {
        if self.at(JavaSyntaxKind::LBrace) {
            let initializer = self.start();
            self.bump();
            let values = self.start();
            let mut guard = self.loop_guard();
            while !self.at_expression_list_close(JavaSyntaxKind::RBrace) && self.check(&mut guard) {
                self.parse_annotation_element_value(JavaSyntaxKind::RBrace);
                self.eat(JavaSyntaxKind::Comma);
            }
            self.complete(values, JavaSyntaxKind::AnnotationElementValueList);
            if !self.eat(JavaSyntaxKind::RBrace) {
                self.error_expected("expected `}` after annotation array initializer");
            }
            self.complete(initializer, JavaSyntaxKind::AnnotationArrayInitializer);
        } else if self.lookahead().at_annotation_start() {
            self.parse_annotation();
        } else {
            self.parse_expression_until(token_set![Comma].with(stop.to_raw()));
        }
    }
}
