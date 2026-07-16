use super::{JavaParserExt, JavaSyntaxKind, Parser, StopSet};
use jolt_syntax::UnresolvedDiagnosticOwner;

impl Parser<'_> {
    pub(super) fn parse_type_declaration(&mut self) {
        let type_decl = self.start();
        self.parse_modifier_list();

        let kind = if self.at(JavaSyntaxKind::At) && self.nth_kind(1) == JavaSyntaxKind::InterfaceKw
        {
            self.bump();
            self.bump();
            JavaSyntaxKind::AnnotationInterfaceDeclaration
        } else if self.eat(JavaSyntaxKind::ClassKw) {
            JavaSyntaxKind::ClassDeclaration
        } else if self.eat(JavaSyntaxKind::InterfaceKw) {
            JavaSyntaxKind::InterfaceDeclaration
        } else if self.eat(JavaSyntaxKind::EnumKw) {
            JavaSyntaxKind::EnumDeclaration
        } else if self.eat_contextual("record") {
            JavaSyntaxKind::RecordDeclaration
        } else {
            self.expected_here("expected top-level type declaration");
            self.recover_top_level();
            self.complete(type_decl, JavaSyntaxKind::ErrorNode);
            return;
        };

        let type_name = self.current_text().map(|_| self.position());
        let name_slot = match kind {
            JavaSyntaxKind::AnnotationInterfaceDeclaration => {
                crate::shape::annotation_interface_declaration::Slot::name as u16
            }
            JavaSyntaxKind::ClassDeclaration => crate::shape::class_declaration::Slot::name as u16,
            JavaSyntaxKind::InterfaceDeclaration => {
                crate::shape::interface_declaration::Slot::name as u16
            }
            JavaSyntaxKind::EnumDeclaration => crate::shape::enum_declaration::Slot::name as u16,
            JavaSyntaxKind::RecordDeclaration => {
                crate::shape::record_declaration::Slot::name as u16
            }
            _ => unreachable!("matched type declaration kind"),
        };
        self.expect_type_identifier("expected type name", type_decl.anchor(), name_slot);
        self.parse_type_declaration_header(kind);
        self.parse_type_body(body_kind_for_type(kind), type_name);
        self.complete(type_decl, kind);
    }

    pub(super) fn parse_type_body(&mut self, kind: JavaSyntaxKind, type_name: Option<usize>) {
        let body = self.start();
        if !self.at(JavaSyntaxKind::LBrace) {
            self.expected_here("expected type body");
            self.eat(JavaSyntaxKind::Semicolon);
            let members = self.start();
            self.complete(
                members,
                match kind {
                    JavaSyntaxKind::AnnotationInterfaceBody => {
                        JavaSyntaxKind::AnnotationInterfaceBodyMemberList
                    }
                    JavaSyntaxKind::InterfaceBody => JavaSyntaxKind::InterfaceBodyMemberList,
                    JavaSyntaxKind::RecordBody => JavaSyntaxKind::RecordBodyMemberList,
                    _ => JavaSyntaxKind::ClassBodyMemberList,
                },
            );
            self.complete(body, kind);
            return;
        }

        self.bump();

        if kind == JavaSyntaxKind::EnumBody {
            self.parse_enum_body_contents(type_name);
        } else if kind == JavaSyntaxKind::AnnotationInterfaceBody {
            self.parse_annotation_interface_body_contents();
        } else {
            let members = self.start();
            while !self.at_eof() && !self.at(JavaSyntaxKind::RBrace) {
                self.parse_body_declaration(kind, type_name);
            }
            self.complete(
                members,
                match kind {
                    JavaSyntaxKind::ClassBody => JavaSyntaxKind::ClassBodyMemberList,
                    JavaSyntaxKind::RecordBody => JavaSyntaxKind::RecordBodyMemberList,
                    JavaSyntaxKind::InterfaceBody => JavaSyntaxKind::InterfaceBodyMemberList,
                    _ => unreachable!("non-special type bodies own a member-list role"),
                },
            );
        }

        self.expect(JavaSyntaxKind::RBrace, "expected `}` after type body");
        self.complete(body, kind);
    }

    pub(super) fn parse_empty_declaration(&mut self) {
        let empty = self.start();
        self.expect(JavaSyntaxKind::Semicolon, "expected `;`");
        self.complete(empty, JavaSyntaxKind::EmptyDeclaration);
    }

    pub(super) fn parse_compact_member_declaration(&mut self) {
        if self.starts_method_declaration() {
            self.parse_method_declaration();
        } else {
            self.parse_field_declaration();
        }
    }

    pub(super) fn parse_modifier_list(&mut self) {
        self.parse_modifier_list_for(ModifierContext::Type);
    }

    pub(super) fn parse_field_modifier_list(&mut self) {
        self.parse_modifier_list_for(ModifierContext::Field);
    }

    pub(super) fn parse_method_modifier_list(&mut self) {
        self.parse_modifier_list_for(ModifierContext::Method);
    }

    pub(super) fn parse_constructor_modifier_list(&mut self) {
        self.parse_modifier_list_for(ModifierContext::Constructor);
    }

    pub(super) fn parse_annotation_element_modifier_list(&mut self) {
        self.parse_modifier_list_for(ModifierContext::AnnotationElement);
    }

    fn parse_modifier_list_for(&mut self, context: ModifierContext) {
        let modifiers = self.start();
        let start = self.position();

        loop {
            if self.at(JavaSyntaxKind::At) && self.nth_kind(1) != JavaSyntaxKind::InterfaceKw {
                self.parse_annotation();
            } else if self.at_type_modifier() {
                if context.allows(self) {
                    self.bump_type_modifier();
                } else {
                    let error = self.start();
                    self.unexpected_owned_node(context.invalid_message(), error.anchor());
                    self.bump_type_modifier();
                    self.complete(error, JavaSyntaxKind::BogusModifier);
                }
            } else {
                break;
            }
        }

        if self.position() == start {
            self.abandon(modifiers);
        } else {
            self.complete(modifiers, JavaSyntaxKind::ModifierList);
        }
    }

    pub(super) fn parse_annotations(&mut self) {
        let annotations = self.start();
        while self.at(JavaSyntaxKind::At) && self.nth_kind(1) != JavaSyntaxKind::InterfaceKw {
            self.parse_annotation();
        }
        self.complete(annotations, JavaSyntaxKind::AnnotationList);
    }

    fn parse_optional_annotations(&mut self) {
        let annotations = self.start();
        let start = self.position();
        while self.at(JavaSyntaxKind::At) && self.nth_kind(1) != JavaSyntaxKind::InterfaceKw {
            self.parse_annotation();
        }
        if self.position() == start {
            self.abandon(annotations);
        } else {
            self.complete(annotations, JavaSyntaxKind::AnnotationList);
        }
    }

    fn parse_parameter_annotations(&mut self) {
        let modifiers = self.start();
        loop {
            if self.at(JavaSyntaxKind::At) && self.nth_kind(1) != JavaSyntaxKind::InterfaceKw {
                self.parse_annotation();
            } else if self.at_type_modifier() {
                self.parse_bogus_parameter_modifier();
            } else {
                break;
            }
        }
        self.complete(modifiers, JavaSyntaxKind::ParameterModifierList);
    }

    fn parse_bogus_parameter_modifier(&mut self) {
        let bogus = self.start();
        self.unexpected_owned_node("invalid parameter modifier", bogus.anchor());
        self.bump_type_modifier();
        self.complete(bogus, JavaSyntaxKind::BogusModifier);
    }

    pub(super) fn parse_annotation(&mut self) {
        let annotation = self.start();
        let owner = annotation.anchor();
        self.expect_owned(
            JavaSyntaxKind::At,
            "expected `@`",
            owner,
            crate::shape::annotation::Slot::at as u16,
        );
        self.consume_qualified_name_owned(UnresolvedDiagnosticOwner::missing_slot(
            owner,
            crate::shape::annotation::Slot::name as u16,
        ));
        if self.at(JavaSyntaxKind::LParen) {
            let arguments = self.start();
            self.bump();
            if !self.at(JavaSyntaxKind::RParen) {
                self.parse_annotation_element_values(JavaSyntaxKind::RParen);
            }
            self.expect_owned(
                JavaSyntaxKind::RParen,
                "expected `)` after annotation arguments",
                arguments.anchor(),
                crate::shape::annotation_argument_list::Slot::close_paren as u16,
            );
            self.complete(arguments, JavaSyntaxKind::AnnotationArgumentList);
        }
        self.complete(annotation, JavaSyntaxKind::Annotation);
    }

    pub(super) fn parse_type_declaration_header(&mut self, kind: JavaSyntaxKind) {
        match kind {
            JavaSyntaxKind::ClassDeclaration => {
                self.parse_optional_type_parameter_list();
                self.parse_optional_extends_clause();
                self.parse_optional_implements_clause();
                self.parse_optional_permits_clause();
            }
            JavaSyntaxKind::InterfaceDeclaration => {
                self.parse_optional_type_parameter_list();
                self.parse_optional_extends_clause();
                self.parse_optional_permits_clause();
            }
            JavaSyntaxKind::EnumDeclaration => {
                self.parse_optional_implements_clause();
            }
            JavaSyntaxKind::RecordDeclaration => {
                self.parse_optional_type_parameter_list();
                self.parse_record_header();
                self.parse_optional_implements_clause();
            }
            _ => {}
        }
    }

    pub(super) fn parse_optional_type_parameter_list(&mut self) -> bool {
        if !self.at(JavaSyntaxKind::Lt) {
            return false;
        }

        let list = self.start();
        let owner = list.anchor();
        self.bump();
        let parameters = self.start();
        while !self.at_eof() && !self.at_type_argument_close() {
            self.parse_type_parameter();
            if !self.eat(JavaSyntaxKind::Comma) {
                break;
            }
        }
        self.complete(parameters, JavaSyntaxKind::TypeParameterSeparatedList);
        if !self.eat_type_argument_close() {
            self.expected_owned_slot(
                "expected `>` after type parameters",
                owner,
                crate::shape::type_parameter_list::Slot::close_angle as u16,
            );
        }
        self.complete(list, JavaSyntaxKind::TypeParameterList);
        true
    }

    pub(super) fn parse_type_parameter(&mut self) {
        let parameter = self.start();
        let owner = parameter.anchor();
        self.parse_annotations();
        self.expect_type_identifier(
            "expected type parameter name",
            owner,
            crate::shape::type_parameter::Slot::name as u16,
        );
        if self.at(JavaSyntaxKind::ExtendsKw) {
            let bounds = self.start();
            self.bump();
            self.parse_class_intersection_type();
            self.complete(bounds, JavaSyntaxKind::TypeBoundList);
        }
        self.complete(parameter, JavaSyntaxKind::TypeParameter);
    }

    pub(super) fn parse_optional_extends_clause(&mut self) {
        if !self.at(JavaSyntaxKind::ExtendsKw) {
            return;
        }

        let clause = self.start();
        let owner = clause.anchor();
        self.bump();
        self.parse_type_list_until_clause_end(
            owner,
            crate::shape::extends_clause::Slot::types as u16,
        );
        self.complete(clause, JavaSyntaxKind::ExtendsClause);
    }

    pub(super) fn parse_optional_implements_clause(&mut self) {
        if !self.at(JavaSyntaxKind::ImplementsKw) {
            return;
        }

        let clause = self.start();
        let owner = clause.anchor();
        self.bump();
        self.parse_type_list_until_clause_end(
            owner,
            crate::shape::implements_clause::Slot::types as u16,
        );
        self.complete(clause, JavaSyntaxKind::ImplementsClause);
    }

    pub(super) fn parse_optional_permits_clause(&mut self) {
        if !self.at_contextual("permits") {
            return;
        }

        let clause = self.start();
        let owner = clause.anchor();
        self.bump();
        self.parse_type_name_list_until_clause_end(
            owner,
            crate::shape::permits_clause::Slot::names as u16,
        );
        self.complete(clause, JavaSyntaxKind::PermitsClause);
    }

    pub(super) fn parse_type_list_until_clause_end(
        &mut self,
        clause: jolt_syntax::NodeAnchor,
        slot: u16,
    ) {
        if self.at_eof() || self.at_header_clause_end() {
            self.expected_owned_slot("expected type", clause, slot);
            return;
        }
        let types = self.start();
        while !self.at_eof() && !self.at_header_clause_end() {
            self.parse_class_type();
            if !self.eat(JavaSyntaxKind::Comma) {
                break;
            }
        }
        self.complete(types, JavaSyntaxKind::TypeList);
    }

    pub(super) fn parse_type_name_list_until_clause_end(
        &mut self,
        clause: jolt_syntax::NodeAnchor,
        slot: u16,
    ) {
        if self.at_eof() || self.at_header_clause_end() {
            self.expected_owned_slot("expected name", clause, slot);
            return;
        }
        let names = self.start();
        let owner = UnresolvedDiagnosticOwner::node(names.anchor());
        while !self.at_eof() && !self.at_header_clause_end() {
            self.consume_qualified_name_owned(owner);
            if !self.eat(JavaSyntaxKind::Comma) {
                break;
            }
        }
        self.complete(names, JavaSyntaxKind::NameList);
    }

    pub(super) fn parse_record_header(&mut self) {
        if !self.eat(JavaSyntaxKind::LParen) {
            self.expected_here("expected record header");
            return;
        }

        if !self.at(JavaSyntaxKind::RParen) {
            let list = self.start();
            while !self.at_eof() && !self.at(JavaSyntaxKind::RParen) {
                self.parse_record_component();
                if !self.eat(JavaSyntaxKind::Comma) {
                    break;
                }
            }
            self.complete(list, JavaSyntaxKind::RecordComponentList);
        }

        self.expect(JavaSyntaxKind::RParen, "expected `)` after record header");
    }

    pub(super) fn parse_record_component(&mut self) {
        let component = self.start();
        let owner = component.anchor();
        self.parse_parameter_annotations();
        self.parse_type();
        self.parse_annotations();
        self.eat(JavaSyntaxKind::Ellipsis);
        self.expect_named_identifier_owned(
            "expected record component name",
            owner,
            crate::shape::record_component::Slot::name as u16,
        );
        self.complete(component, JavaSyntaxKind::RecordComponent);
    }

    pub(super) fn parse_body_declaration(
        &mut self,
        body_kind: JavaSyntaxKind,
        type_name: Option<usize>,
    ) {
        if self.at(JavaSyntaxKind::Semicolon) {
            self.parse_empty_declaration();
            return;
        }

        if body_kind == JavaSyntaxKind::ClassBody || body_kind == JavaSyntaxKind::RecordBody {
            let declaration = self.start();
            self.parse_class_body_declaration_contents(body_kind, type_name);
            self.complete(declaration, JavaSyntaxKind::ClassBodyDeclaration);
        } else {
            self.parse_member_declaration(type_name, false);
        }
    }

    pub(super) fn parse_class_body_declaration_contents(
        &mut self,
        body_kind: JavaSyntaxKind,
        type_name: Option<usize>,
    ) {
        if self.at(JavaSyntaxKind::LBrace) {
            let initializer = self.start();
            self.parse_block();
            self.complete(initializer, JavaSyntaxKind::InstanceInitializer);
            return;
        }

        if self.at(JavaSyntaxKind::StaticKw) && self.nth_kind(1) == JavaSyntaxKind::LBrace {
            let initializer = self.start();
            self.bump();
            self.parse_block();
            self.complete(initializer, JavaSyntaxKind::StaticInitializer);
            return;
        }

        if body_kind == JavaSyntaxKind::RecordBody && self.starts_compact_constructor(type_name) {
            self.parse_compact_constructor_declaration();
        } else {
            self.parse_member_declaration(type_name, false);
        }
    }

    pub(super) fn parse_member_declaration(
        &mut self,
        type_name: Option<usize>,
        annotation_body: bool,
    ) {
        if self.starts_top_level_type_declaration() {
            self.parse_type_declaration();
            return;
        }

        if annotation_body && self.starts_annotation_element() {
            self.parse_annotation_element();
            return;
        }

        if self.starts_constructor(type_name) {
            self.parse_constructor_declaration();
            return;
        }

        if self.starts_method_declaration() {
            self.parse_method_declaration();
        } else if self.starts_compact_member_declaration() {
            self.parse_field_declaration();
        } else {
            let error = self.start();
            self.unexpected_here("unexpected token in type body");
            self.consume_body_member_fragment();
            self.complete(error, JavaSyntaxKind::ErrorNode);
        }
    }

    pub(super) fn parse_field_declaration(&mut self) {
        let field = self.start();
        self.parse_field_modifier_list();
        self.parse_type();
        self.parse_variable_declarator_list();
        self.expect(
            JavaSyntaxKind::Semicolon,
            "expected `;` after field declaration",
        );
        self.complete(field, JavaSyntaxKind::FieldDeclaration);
    }

    pub(super) fn parse_variable_declarator_list(&mut self) {
        self.parse_variable_declarator_list_until_with(&[JavaSyntaxKind::Semicolon], false);
    }

    pub(super) fn parse_variable_declarator_list_until(&mut self, stops: &[JavaSyntaxKind]) {
        self.parse_variable_declarator_list_until_with(stops, true);
    }

    pub(super) fn parse_variable_declarator_list_until_with(
        &mut self,
        stops: &[JavaSyntaxKind],
        allow_unnamed: bool,
    ) {
        let list = self.start();
        loop {
            self.parse_variable_declarator_until(stops, allow_unnamed);
            if stops.contains(&self.current_kind()) || self.at_contextual("when") {
                break;
            }
            if !self.eat(JavaSyntaxKind::Comma) {
                break;
            }
        }
        self.complete(list, JavaSyntaxKind::VariableDeclaratorList);
    }

    pub(super) fn parse_variable_declarator_until(
        &mut self,
        stops: &[JavaSyntaxKind],
        allow_unnamed: bool,
    ) {
        let declarator = self.start();
        let owner = declarator.anchor();
        self.parse_variable_declarator_id(
            allow_unnamed,
            owner,
            crate::shape::variable_declarator::Slot::name as u16,
        );
        if self.eat(JavaSyntaxKind::Assign) {
            self.parse_variable_initializer_until(stops);
        }
        self.complete(declarator, JavaSyntaxKind::VariableDeclarator);
    }

    pub(super) fn parse_variable_declarator_id(
        &mut self,
        allow_unnamed: bool,
        owner: jolt_syntax::NodeAnchor,
        slot: u16,
    ) {
        self.expect_variable_identifier_owned("expected variable name", owner, slot, allow_unnamed);
        self.parse_array_dimensions();
    }

    pub(super) fn parse_variable_initializer_until(&mut self, stops: &[JavaSyntaxKind]) {
        let initializer = self.start();
        if self.at(JavaSyntaxKind::LBrace) {
            self.parse_array_initializer_fragment();
        } else {
            self.parse_expression_until(StopSet::new(stops).with_extra(JavaSyntaxKind::Comma));
        }
        self.complete(initializer, JavaSyntaxKind::VariableInitializer);
    }

    pub(super) fn parse_method_declaration(&mut self) {
        let method = self.start();
        let owner = method.anchor();
        self.parse_method_modifier_list();
        self.parse_optional_type_parameter_list();
        self.parse_optional_annotations();
        self.parse_result_type();
        self.expect_named_identifier_owned(
            "expected method name",
            owner,
            crate::shape::method_declaration::Slot::name as u16,
        );
        self.parse_formal_parameter_section();
        self.parse_array_dimensions();
        self.parse_optional_throws_clause();
        self.parse_method_body();
        self.complete(method, JavaSyntaxKind::MethodDeclaration);
    }

    pub(super) fn parse_annotation_element(&mut self) {
        let element = self.start();
        let owner = element.anchor();
        self.parse_annotation_element_modifier_list();
        self.parse_type();
        self.expect_named_identifier_owned(
            "expected annotation element name",
            owner,
            crate::shape::annotation_element_declaration::Slot::name as u16,
        );
        self.expect(JavaSyntaxKind::LParen, "expected `(`");
        self.expect(JavaSyntaxKind::RParen, "expected `)`");
        self.parse_array_dimensions();
        if self.at(JavaSyntaxKind::DefaultKw) {
            self.parse_default_value();
        }
        self.expect(
            JavaSyntaxKind::Semicolon,
            "expected `;` after annotation element",
        );
        self.complete(element, JavaSyntaxKind::AnnotationElementDeclaration);
    }

    pub(super) fn parse_default_value(&mut self) {
        let default_value = self.start();
        self.expect(JavaSyntaxKind::DefaultKw, "expected `default`");
        self.parse_annotation_element_value(JavaSyntaxKind::Semicolon);
        self.complete(default_value, JavaSyntaxKind::DefaultValue);
    }

    pub(super) fn parse_constructor_declaration(&mut self) {
        let constructor = self.start();
        self.parse_constructor_modifier_list();
        self.parse_optional_type_parameter_list();
        self.expect_type_identifier(
            "expected constructor name",
            constructor.anchor(),
            crate::shape::constructor_declaration::Slot::name as u16,
        );
        self.parse_formal_parameter_section();
        self.parse_optional_throws_clause();
        self.parse_constructor_block();
        self.complete(constructor, JavaSyntaxKind::ConstructorDeclaration);
    }

    pub(super) fn parse_compact_constructor_declaration(&mut self) {
        let constructor = self.start();
        self.parse_constructor_modifier_list();
        self.expect_type_identifier(
            "expected compact constructor name",
            constructor.anchor(),
            crate::shape::compact_constructor_declaration::Slot::name as u16,
        );
        self.parse_constructor_block();
        self.complete(constructor, JavaSyntaxKind::CompactConstructorDeclaration);
    }

    pub(super) fn parse_result_type(&mut self) {
        if self.at(JavaSyntaxKind::VoidKw) {
            self.parse_void_type();
            return;
        }
        self.parse_type();
    }

    pub(super) fn parse_formal_parameter_section(&mut self) {
        self.expect(JavaSyntaxKind::LParen, "expected `(`");
        if !self.at(JavaSyntaxKind::RParen) {
            let list = self.start();
            let mut allow_receiver = true;
            while !self.at_eof() && !self.at(JavaSyntaxKind::RParen) {
                if self.at(JavaSyntaxKind::Comma) {
                    let bogus = self.start();
                    self.expected_owned_node("expected parameter", bogus.anchor());
                    self.complete(bogus, JavaSyntaxKind::BogusFormalParameter);
                } else if self.starts_receiver_parameter() {
                    self.parse_receiver_parameter_entry(!allow_receiver);
                } else {
                    let was_varargs = self.parse_formal_parameter();
                    if was_varargs && !self.at(JavaSyntaxKind::RParen) && !self.at_eof() {
                        let consumed_comma = self.eat(JavaSyntaxKind::Comma);
                        if consumed_comma {
                            allow_receiver = false;
                            continue;
                        }
                        break;
                    }
                }
                allow_receiver = false;
                if !self.eat(JavaSyntaxKind::Comma) {
                    break;
                }
            }
            self.complete(list, JavaSyntaxKind::FormalParameterList);
        }
        self.expect(JavaSyntaxKind::RParen, "expected `)` after parameters");
    }

    fn parse_receiver_parameter_entry(&mut self, misplaced: bool) {
        let parameter = self.start();
        let owner = parameter.anchor();
        let mut bogus = misplaced;
        if misplaced {
            let diagnostic =
                self.misplaced_receiver_parameter_here("receiver parameter must be first");
            self.own_diagnostic(diagnostic, UnresolvedDiagnosticOwner::node(owner));
        }
        self.parse_annotations();
        while self.at_type_modifier() {
            bogus = true;
            self.unexpected_owned_node("invalid receiver parameter modifier", owner);
            self.bump_type_modifier();
            self.parse_annotations();
        }
        self.parse_type();
        if self.at_name_segment() && self.nth_kind(1) == JavaSyntaxKind::Dot {
            self.bump();
            self.bump();
        }
        if !self.at(JavaSyntaxKind::ThisKw) {
            bogus = true;
            self.unexpected_owned_node("invalid receiver parameter", owner);
            while !self.at_eof()
                && !matches!(
                    self.current_kind(),
                    JavaSyntaxKind::ThisKw | JavaSyntaxKind::Comma | JavaSyntaxKind::RParen
                )
            {
                self.bump();
            }
        }
        self.eat(JavaSyntaxKind::ThisKw);
        self.complete(
            parameter,
            if bogus {
                JavaSyntaxKind::BogusFormalParameter
            } else {
                JavaSyntaxKind::ReceiverParameter
            },
        );
    }

    pub(super) fn parse_formal_parameter(&mut self) -> bool {
        let parameter = self.start();
        let owner = parameter.anchor();
        self.parse_variable_modifiers();
        self.parse_type();
        self.parse_annotations();
        let varargs = self.eat(JavaSyntaxKind::Ellipsis);
        self.expect_variable_identifier_owned(
            "expected parameter name",
            owner,
            crate::shape::formal_parameter::Slot::name as u16,
            true,
        );
        self.parse_array_dimensions();
        let misplaced_varargs = varargs && !self.at(JavaSyntaxKind::RParen) && !self.at_eof();
        if misplaced_varargs {
            self.expected_owned_node("varargs parameter must be last", parameter.anchor());
        }
        self.complete(
            parameter,
            if misplaced_varargs {
                JavaSyntaxKind::BogusFormalParameter
            } else {
                JavaSyntaxKind::FormalParameter
            },
        );
        varargs
    }

    pub(super) fn parse_variable_modifiers(&mut self) -> bool {
        let modifiers = self.start();
        let start = self.position();
        loop {
            if self.at(JavaSyntaxKind::At) && self.nth_kind(1) != JavaSyntaxKind::InterfaceKw {
                self.parse_annotation();
            } else if self.at(JavaSyntaxKind::FinalKw) {
                self.bump();
            } else if self.at_type_modifier() {
                self.parse_bogus_parameter_modifier();
            } else {
                break;
            }
        }
        let has_modifiers = self.position() != start;
        self.complete(modifiers, JavaSyntaxKind::ParameterModifierList);
        has_modifiers
    }

    pub(super) fn parse_optional_throws_clause(&mut self) {
        if !self.at(JavaSyntaxKind::ThrowsKw) {
            return;
        }

        let clause = self.start();
        let owner = clause.anchor();
        self.bump();
        if self.at_eof()
            || matches!(
                self.current_kind(),
                JavaSyntaxKind::LBrace | JavaSyntaxKind::Semicolon
            )
        {
            self.expected_owned_slot(
                "expected exception type",
                owner,
                crate::shape::throws_clause::Slot::exceptions as u16,
            );
            self.complete(clause, JavaSyntaxKind::ThrowsClause);
            return;
        }
        let types = self.start();
        while !self.at_eof()
            && !matches!(
                self.current_kind(),
                JavaSyntaxKind::LBrace | JavaSyntaxKind::Semicolon
            )
        {
            self.parse_class_type();
            if !self.eat(JavaSyntaxKind::Comma) {
                break;
            }
        }
        self.complete(types, JavaSyntaxKind::TypeList);
        self.complete(clause, JavaSyntaxKind::ThrowsClause);
    }

    pub(super) fn parse_method_body(&mut self) {
        if self.at(JavaSyntaxKind::LBrace) {
            self.parse_block();
        } else {
            self.expect(JavaSyntaxKind::Semicolon, "expected method body");
        }
    }

    pub(super) fn parse_constructor_block(&mut self) {
        let block = self.start();
        self.expect(JavaSyntaxKind::LBrace, "expected constructor body");
        let entries = self.start();
        let mut saw_constructor_invocation = false;
        while !self.at_eof() && !self.at(JavaSyntaxKind::RBrace) {
            if self.starts_constructor_invocation_statement() && !saw_constructor_invocation {
                self.parse_constructor_invocation();
                saw_constructor_invocation = true;
            } else if self.starts_constructor_invocation_statement() {
                let error = self.start();
                self.misplaced_constructor_invocation_here(
                    "constructor body must have at most one explicit constructor invocation",
                );
                self.parse_constructor_invocation();
                self.complete(error, JavaSyntaxKind::ErrorNode);
            } else {
                self.parse_block_statement();
            }
        }
        self.complete(entries, JavaSyntaxKind::ConstructorBodyEntryList);
        self.expect(
            JavaSyntaxKind::RBrace,
            "expected `}` after constructor body",
        );
        self.complete(block, JavaSyntaxKind::ConstructorBody);
    }

    pub(super) fn parse_constructor_invocation(&mut self) {
        let invocation = self.start();

        if self.at(JavaSyntaxKind::Lt)
            || matches!(
                self.current_kind(),
                JavaSyntaxKind::ThisKw | JavaSyntaxKind::SuperKw
            )
        {
            self.parse_optional_type_argument_list();
            if self.at(JavaSyntaxKind::ThisKw) || self.at(JavaSyntaxKind::SuperKw) {
                self.bump();
                self.parse_argument_list();
            } else {
                self.expected_here("expected `this` or `super` in constructor invocation");
                let malformed = self.start();
                while !self.at_eof()
                    && !self.at(JavaSyntaxKind::Semicolon)
                    && !self.at(JavaSyntaxKind::RBrace)
                {
                    self.bump();
                }
                self.complete(malformed, JavaSyntaxKind::ErrorNode);
            }
        } else {
            self.parse_constructor_invocation_qualifier();
            self.expect(JavaSyntaxKind::Dot, "expected `.` before `super`");
            self.parse_optional_type_argument_list();
            self.expect(
                JavaSyntaxKind::SuperKw,
                "expected `super` in constructor invocation",
            );
            self.parse_argument_list();
        }

        self.expect(
            JavaSyntaxKind::Semicolon,
            "expected `;` after constructor invocation",
        );
        self.complete(invocation, JavaSyntaxKind::ConstructorInvocation);
    }

    pub(super) fn parse_constructor_invocation_qualifier(&mut self) {
        if self.starts_expression_name_qualified_constructor_invocation() {
            self.consume_qualified_name();
        } else {
            self.parse_constructor_invocation_primary_qualifier();
        }
    }

    pub(super) fn parse_constructor_invocation_primary_qualifier(&mut self) {
        let mut expression = self.parse_primary_expression(false);

        loop {
            match self.current_kind() {
                JavaSyntaxKind::LParen => {
                    self.parse_argument_list();
                    let form = self.precede(expression);
                    let form = self.complete(form, JavaSyntaxKind::UnqualifiedMethodInvocation);
                    let invocation = self.precede(form);
                    expression =
                        self.complete(invocation, JavaSyntaxKind::MethodInvocationExpression);
                }
                JavaSyntaxKind::LBracket => {
                    let access = self.precede(expression);
                    self.bump();
                    self.parse_expression_until(&[JavaSyntaxKind::RBracket]);
                    self.expect(JavaSyntaxKind::RBracket, "expected `]` after array index");
                    expression = self.complete(access, JavaSyntaxKind::ArrayAccessExpression);
                }
                JavaSyntaxKind::Dot if !self.dot_starts_constructor_super_suffix() => {
                    expression = self.parse_dot_suffix(expression);
                }
                _ => break,
            }
        }
    }

    pub(super) fn parse_enum_body_contents(&mut self, type_name: Option<usize>) {
        if !self.at(JavaSyntaxKind::Semicolon) && !self.at(JavaSyntaxKind::RBrace) {
            let list = self.start();
            loop {
                self.parse_enum_constant();
                if !self.eat(JavaSyntaxKind::Comma)
                    || self.at(JavaSyntaxKind::Semicolon)
                    || self.at(JavaSyntaxKind::RBrace)
                {
                    break;
                }
            }
            self.complete(list, JavaSyntaxKind::EnumConstantList);
            self.eat(JavaSyntaxKind::Comma);
        }

        let has_body_separator = self.eat(JavaSyntaxKind::Semicolon);
        let members = self.start();
        if has_body_separator {
            while !self.at_eof() && !self.at(JavaSyntaxKind::RBrace) {
                self.parse_body_declaration(JavaSyntaxKind::ClassBody, type_name);
            }
        }
        self.complete(members, JavaSyntaxKind::ClassBodyMemberList);
    }

    pub(super) fn parse_enum_constant(&mut self) {
        let constant = self.start();
        self.parse_annotations();
        self.expect_named_variable_identifier("expected enum constant name");
        if self.at(JavaSyntaxKind::LParen) {
            self.parse_argument_list();
        }
        if self.at(JavaSyntaxKind::LBrace) {
            self.parse_type_body(JavaSyntaxKind::ClassBody, None);
        }
        self.complete(constant, JavaSyntaxKind::EnumConstant);
    }

    pub(super) fn parse_annotation_interface_body_contents(&mut self) {
        let list = self.start();
        let members = self.start();
        while !self.at_eof() && !self.at(JavaSyntaxKind::RBrace) {
            if self.at(JavaSyntaxKind::Semicolon) {
                self.parse_empty_declaration();
            } else {
                self.parse_member_declaration(None, true);
            }
        }
        self.complete(members, JavaSyntaxKind::AnnotationInterfaceBodyMemberList);
        self.complete(list, JavaSyntaxKind::AnnotationElementList);
    }

    pub(super) fn consume_body_member_fragment(&mut self) {
        while !self.at_eof() && !self.at(JavaSyntaxKind::RBrace) {
            if self.at(JavaSyntaxKind::LBrace) {
                self.consume_balanced_delimited(JavaSyntaxKind::LBrace, JavaSyntaxKind::RBrace);
            } else if self.at(JavaSyntaxKind::Semicolon) {
                self.bump();
                break;
            } else if self.starts_top_level_type_declaration() {
                break;
            } else {
                self.bump();
            }
        }
    }
}

#[derive(Clone, Copy)]
enum ModifierContext {
    Type,
    Field,
    Method,
    Constructor,
    AnnotationElement,
}

impl ModifierContext {
    fn allows(self, parser: &mut Parser<'_>) -> bool {
        match self {
            Self::Type => !matches!(
                parser.current_kind(),
                JavaSyntaxKind::NativeKw
                    | JavaSyntaxKind::SynchronizedKw
                    | JavaSyntaxKind::TransientKw
                    | JavaSyntaxKind::VolatileKw
                    | JavaSyntaxKind::DefaultKw
            ),
            Self::Field => matches!(
                parser.current_kind(),
                JavaSyntaxKind::PublicKw
                    | JavaSyntaxKind::ProtectedKw
                    | JavaSyntaxKind::PrivateKw
                    | JavaSyntaxKind::StaticKw
                    | JavaSyntaxKind::FinalKw
                    | JavaSyntaxKind::TransientKw
                    | JavaSyntaxKind::VolatileKw
            ),
            Self::Method => matches!(
                parser.current_kind(),
                JavaSyntaxKind::PublicKw
                    | JavaSyntaxKind::ProtectedKw
                    | JavaSyntaxKind::PrivateKw
                    | JavaSyntaxKind::AbstractKw
                    | JavaSyntaxKind::StaticKw
                    | JavaSyntaxKind::FinalKw
                    | JavaSyntaxKind::SynchronizedKw
                    | JavaSyntaxKind::NativeKw
                    | JavaSyntaxKind::StrictfpKw
                    | JavaSyntaxKind::DefaultKw
            ),
            Self::Constructor => matches!(
                parser.current_kind(),
                JavaSyntaxKind::PublicKw | JavaSyntaxKind::ProtectedKw | JavaSyntaxKind::PrivateKw
            ),
            Self::AnnotationElement => matches!(
                parser.current_kind(),
                JavaSyntaxKind::PublicKw | JavaSyntaxKind::AbstractKw
            ),
        }
    }

    const fn invalid_message(self) -> &'static str {
        match self {
            Self::Type => "invalid type modifier",
            Self::Field => "invalid field modifier",
            Self::Method => "invalid method modifier",
            Self::Constructor => "invalid constructor modifier",
            Self::AnnotationElement => "invalid annotation element modifier",
        }
    }
}

fn body_kind_for_type(kind: JavaSyntaxKind) -> JavaSyntaxKind {
    match kind {
        JavaSyntaxKind::InterfaceDeclaration => JavaSyntaxKind::InterfaceBody,
        JavaSyntaxKind::AnnotationInterfaceDeclaration => JavaSyntaxKind::AnnotationInterfaceBody,
        JavaSyntaxKind::EnumDeclaration => JavaSyntaxKind::EnumBody,
        JavaSyntaxKind::RecordDeclaration => JavaSyntaxKind::RecordBody,
        _ => JavaSyntaxKind::ClassBody,
    }
}
