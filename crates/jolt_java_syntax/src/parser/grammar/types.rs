use super::{JavaSyntaxKind, Parser};

impl Parser<'_> {
    pub(super) fn parse_type(&mut self) -> jolt_syntax::CompletedMarker {
        let ty = self.start();
        self.parse_annotations();

        let base_kind = if self.at_primitive_type() {
            self.bump();
            JavaSyntaxKind::PrimitiveType
        } else if self.at_name_segment() {
            self.parse_class_type_tail();
            JavaSyntaxKind::ClassType
        } else {
            self.expected_here("expected type");
            return self.complete(ty, JavaSyntaxKind::ErrorNode);
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
        self.expect(JavaSyntaxKind::VoidKw, "expected `void`");
        self.complete(ty, JavaSyntaxKind::VoidType)
    }

    pub(super) fn parse_class_type(&mut self) -> jolt_syntax::CompletedMarker {
        let ty = self.start();
        self.parse_annotations();

        if self.at_name_segment() {
            self.parse_class_type_tail();
            let completed = self.complete(ty, JavaSyntaxKind::ClassType);
            if self.starts_array_dimensions() {
                let error = self.precede(completed);
                self.expected_here("expected class or interface type");
                self.parse_array_dimensions();
                self.complete(error, JavaSyntaxKind::ErrorNode)
            } else {
                completed
            }
        } else {
            self.expected_here("expected class or interface type");
            if self.at_primitive_type() || self.at(JavaSyntaxKind::VoidKw) {
                self.bump();
                self.parse_array_dimensions();
            }
            self.complete(ty, JavaSyntaxKind::ErrorNode)
        }
    }

    pub(super) fn parse_intersection_type(&mut self) -> jolt_syntax::CompletedMarker {
        let ty = self.parse_type();
        if !self.at(JavaSyntaxKind::Amp) {
            return ty;
        }

        let intersection = self.precede(ty);
        while self.eat(JavaSyntaxKind::Amp) {
            self.parse_type();
        }
        self.complete(intersection, JavaSyntaxKind::IntersectionType)
    }

    pub(super) fn parse_class_intersection_type(&mut self) -> jolt_syntax::CompletedMarker {
        let ty = self.parse_class_type();
        if !self.at(JavaSyntaxKind::Amp) {
            return ty;
        }

        let intersection = self.precede(ty);
        while self.eat(JavaSyntaxKind::Amp) {
            self.parse_class_type();
        }
        self.complete(intersection, JavaSyntaxKind::IntersectionType)
    }

    pub(super) fn parse_class_union_type(&mut self) -> jolt_syntax::CompletedMarker {
        let ty = self.parse_class_type();
        if !self.at(JavaSyntaxKind::Bar) {
            return ty;
        }

        let union = self.precede(ty);
        while self.eat(JavaSyntaxKind::Bar) {
            self.parse_class_type();
        }
        self.complete(union, JavaSyntaxKind::UnionType)
    }

    pub(super) fn parse_class_type_tail(&mut self) {
        if !self.at_name_segment() {
            self.expected_here("expected type");
            return;
        }

        self.parse_class_type_name_run();
        self.parse_optional_type_argument_list();
        while self.at(JavaSyntaxKind::Dot) {
            let after_dot = self.skip_annotations_from(self.position() + 1);
            if !self.is_name_segment_at(after_dot) {
                break;
            }
            self.bump();
            self.parse_annotations();
            self.parse_class_type_name_run();
            self.parse_optional_type_argument_list();
        }
    }

    pub(super) fn parse_class_type_name_run(&mut self) {
        let name = self.start();
        self.bump();
        let mut qualified = false;
        while !self.at(JavaSyntaxKind::Lt) && self.at(JavaSyntaxKind::Dot) {
            let after_dot = self.skip_annotations_from(self.position() + 1);
            if !self.is_name_segment_at(after_dot) {
                break;
            }
            qualified = true;
            self.bump();
            self.parse_annotations();
            self.bump();
        }
        self.complete(
            name,
            if qualified {
                JavaSyntaxKind::QualifiedName
            } else {
                JavaSyntaxKind::Name
            },
        );
    }

    pub(super) fn parse_optional_type_argument_list(&mut self) -> bool {
        if !self.at(JavaSyntaxKind::Lt) {
            return false;
        }

        let list = self.start();
        self.bump();
        while !self.at_eof() && !self.at_type_argument_close() {
            self.parse_type_argument();
            if !self.eat(JavaSyntaxKind::Comma) {
                break;
            }
        }
        self.eat_type_argument_close();
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
            self.parse_annotations();
            self.expect(JavaSyntaxKind::LBracket, "expected `[`");
            self.expect(JavaSyntaxKind::RBracket, "expected `]`");
            self.complete(dimension, JavaSyntaxKind::ArrayDimension);
        }
        self.complete(dimensions, JavaSyntaxKind::ArrayDimensions);
        true
    }

    pub(super) fn parse_annotation_element_values(&mut self, stop: JavaSyntaxKind) {
        let list = self.start();
        while !self.at_eof() && !self.at(stop) {
            self.parse_annotation_element_value_or_pair(stop);
            if !self.eat(JavaSyntaxKind::Comma) {
                break;
            }
        }
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
            self.bump();
            while !self.at_eof() && !self.at(JavaSyntaxKind::RBrace) {
                self.parse_annotation_element_value(JavaSyntaxKind::RBrace);
                self.eat(JavaSyntaxKind::Comma);
            }
            self.expect(
                JavaSyntaxKind::RBrace,
                "expected `}` after annotation array initializer",
            );
            self.complete(initializer, JavaSyntaxKind::AnnotationArrayInitializer);
        } else if self.at(JavaSyntaxKind::At) && self.nth_kind(1) != JavaSyntaxKind::InterfaceKw {
            self.parse_annotation();
        } else {
            self.parse_expression_until(&[JavaSyntaxKind::Comma, stop]);
        }
    }
}
