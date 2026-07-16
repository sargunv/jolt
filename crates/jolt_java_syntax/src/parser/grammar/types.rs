use super::{JavaSyntaxKind, Parser};

impl Parser<'_> {
    pub(super) fn parse_type(&mut self) -> jolt_syntax::CompletedMarker {
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
            if self.at_primitive_type() {
                self.bump();
                JavaSyntaxKind::PrimitiveType
            } else {
                self.expected_owned_node("expected type", ty.anchor());
                return self.complete(ty, JavaSyntaxKind::BogusType);
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

    pub(super) fn parse_void_type(&mut self) -> jolt_syntax::CompletedMarker {
        let ty = self.start();
        let owner = ty.anchor();
        self.expect_owned(
            JavaSyntaxKind::VoidKw,
            "expected `void`",
            owner,
            crate::shape::void_type::Slot::void_keyword as u16,
        );
        self.complete(ty, JavaSyntaxKind::VoidType)
    }

    pub(super) fn parse_class_type(&mut self) -> jolt_syntax::CompletedMarker {
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
                self.expected_owned_node("expected class or interface type", error.anchor());
                self.parse_array_dimensions();
                self.complete(error, JavaSyntaxKind::BogusType)
            } else {
                completed
            }
        } else {
            self.parse_annotations();
            self.expected_owned_node("expected class or interface type", ty.anchor());
            if self.at_primitive_type() || self.at(JavaSyntaxKind::VoidKw) {
                self.bump();
                self.parse_array_dimensions();
            }
            self.complete(ty, JavaSyntaxKind::BogusType)
        }
    }

    pub(super) fn parse_reference_type(&mut self) -> jolt_syntax::CompletedMarker {
        let ty = self.parse_type();
        if JavaSyntaxKind::from_raw(ty.kind()) == Some(JavaSyntaxKind::PrimitiveType) {
            let error = self.precede(ty);
            self.expected_owned_node("expected reference type", error.anchor());
            self.complete(error, JavaSyntaxKind::BogusType)
        } else {
            ty
        }
    }

    pub(super) fn parse_intersection_type(&mut self) -> jolt_syntax::CompletedMarker {
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

    pub(super) fn parse_class_intersection_type(&mut self) -> jolt_syntax::CompletedMarker {
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

    pub(super) fn parse_class_union_type(&mut self) -> jolt_syntax::CompletedMarker {
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

    fn parse_class_type_tail_from(
        &mut self,
        segments: jolt_syntax::Marker,
        mut segment: jolt_syntax::Marker,
    ) {
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
        let owner = list.anchor();
        self.bump();
        let arguments = self.start();
        while !self.at_eof() && !self.at_type_argument_close() {
            self.parse_type_argument();
            if !self.eat(JavaSyntaxKind::Comma) {
                break;
            }
        }
        self.complete(arguments, JavaSyntaxKind::TypeArgumentSeparatedList);
        if !self.eat_type_argument_close() {
            self.expected_owned_slot(
                "expected `>` after type arguments",
                owner,
                crate::shape::type_argument_list::Slot::close_angle as u16,
            );
        }
        self.complete(list, JavaSyntaxKind::TypeArgumentList);
        true
    }

    pub(super) fn parse_type_argument(&mut self) {
        let argument = self.start();
        self.parse_annotations();
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
        self.complete(argument, JavaSyntaxKind::TypeArgument);
    }

    pub(super) fn parse_array_dimensions(&mut self) -> bool {
        if !self.starts_array_dimensions() {
            return false;
        }

        let dimensions = self.start();
        while self.starts_array_dimensions() {
            let dimension = self.start();
            let owner = dimension.anchor();
            self.parse_annotations();
            self.expect_owned(
                JavaSyntaxKind::LBracket,
                "expected `[`",
                owner,
                crate::shape::array_dimension::Slot::open_bracket as u16,
            );
            self.expect_owned(
                JavaSyntaxKind::RBracket,
                "expected `]`",
                owner,
                crate::shape::array_dimension::Slot::close_bracket as u16,
            );
            self.complete(dimension, JavaSyntaxKind::ArrayDimension);
        }
        self.complete(dimensions, JavaSyntaxKind::ArrayDimensions);
        true
    }

    pub(super) fn parse_annotation_element_values(&mut self, stop: JavaSyntaxKind) {
        let list = self.start();
        let arguments = self.start();
        while !self.at_eof() && !self.at(stop) {
            if self.at(JavaSyntaxKind::Comma) {
                let bogus = self.start();
                self.expected_owned_node("expected annotation argument", bogus.anchor());
                self.complete(bogus, JavaSyntaxKind::BogusAnnotationArgument);
            } else {
                self.parse_annotation_element_value_or_pair(stop);
            }
            if !self.eat(JavaSyntaxKind::Comma) {
                break;
            }
        }
        self.complete(arguments, JavaSyntaxKind::AnnotationElementArgumentList);
        self.complete(list, JavaSyntaxKind::AnnotationElementList);
    }

    pub(super) fn parse_annotation_element_value_or_pair(&mut self, stop: JavaSyntaxKind) {
        if self.at_name_segment() && self.nth_kind(1) == JavaSyntaxKind::Assign {
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
        self.parse_annotation_element_value_contents(stop);
        self.complete(value, JavaSyntaxKind::AnnotationElementValue);
    }

    pub(super) fn parse_annotation_element_value_contents(&mut self, stop: JavaSyntaxKind) {
        if self.at(JavaSyntaxKind::LBrace) {
            let initializer = self.start();
            let owner = initializer.anchor();
            self.bump();
            let values = self.start();
            while !self.at_eof() && !self.at(JavaSyntaxKind::RBrace) {
                self.parse_annotation_element_value(JavaSyntaxKind::RBrace);
                self.eat(JavaSyntaxKind::Comma);
            }
            self.complete(values, JavaSyntaxKind::AnnotationElementValueList);
            self.expect_owned(
                JavaSyntaxKind::RBrace,
                "expected `}` after annotation array initializer",
                owner,
                crate::shape::annotation_array_initializer::Slot::close_brace as u16,
            );
            self.complete(initializer, JavaSyntaxKind::AnnotationArrayInitializer);
        } else if self.at(JavaSyntaxKind::At) && self.nth_kind(1) != JavaSyntaxKind::InterfaceKw {
            self.parse_annotation();
        } else {
            self.parse_expression_until(&[JavaSyntaxKind::Comma, stop]);
        }
    }
}
