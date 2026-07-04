use super::{JavaSyntaxKind, Parser, StopSet};

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

        let type_name = self.current_text().map(str::to_owned);
        self.expect_type_identifier("expected type name");
        self.parse_type_declaration_header(kind);
        self.parse_type_body(body_kind_for_type(kind), type_name.as_deref());
        self.complete(type_decl, kind);
    }

    pub(super) fn parse_type_body(&mut self, kind: JavaSyntaxKind, type_name: Option<&str>) {
        if !self.at(JavaSyntaxKind::LBrace) {
            let error = self.start();
            self.expected_here("expected type body");
            self.eat(JavaSyntaxKind::Semicolon);
            self.complete(error, JavaSyntaxKind::ErrorNode);
            return;
        }

        let body = self.start();
        self.bump();

        if kind == JavaSyntaxKind::EnumBody {
            self.parse_enum_body_contents(type_name);
        } else if kind == JavaSyntaxKind::AnnotationInterfaceBody {
            self.parse_annotation_interface_body_contents();
        } else {
            while !self.at_eof() && !self.at(JavaSyntaxKind::RBrace) {
                self.parse_body_declaration(kind, type_name);
            }
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
                    self.unexpected_here(context.invalid_message());
                    self.bump_type_modifier();
                    self.complete(error, JavaSyntaxKind::ErrorNode);
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
        while self.at(JavaSyntaxKind::At) && self.nth_kind(1) != JavaSyntaxKind::InterfaceKw {
            self.parse_annotation();
        }
    }

    pub(super) fn parse_annotation(&mut self) {
        let annotation = self.start();
        self.expect(JavaSyntaxKind::At, "expected `@`");
        self.consume_qualified_name();
        if self.at(JavaSyntaxKind::LParen) {
            let arguments = self.start();
            self.bump();
            if !self.at(JavaSyntaxKind::RParen) {
                self.parse_annotation_element_values(JavaSyntaxKind::RParen);
            }
            self.expect(
                JavaSyntaxKind::RParen,
                "expected `)` after annotation arguments",
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
        self.bump();
        while !self.at_eof() && !self.at_type_argument_close() {
            self.parse_type_parameter();
            if !self.eat(JavaSyntaxKind::Comma) {
                break;
            }
        }
        self.eat_type_argument_close();
        self.complete(list, JavaSyntaxKind::TypeParameterList);
        true
    }

    pub(super) fn parse_type_parameter(&mut self) {
        let parameter = self.start();
        self.parse_annotations();
        self.expect_type_identifier("expected type parameter name");
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
        self.bump();
        self.parse_type_list_until_clause_end();
        self.complete(clause, JavaSyntaxKind::ExtendsClause);
    }

    pub(super) fn parse_optional_implements_clause(&mut self) {
        if !self.at(JavaSyntaxKind::ImplementsKw) {
            return;
        }

        let clause = self.start();
        self.bump();
        self.parse_type_list_until_clause_end();
        self.complete(clause, JavaSyntaxKind::ImplementsClause);
    }

    pub(super) fn parse_optional_permits_clause(&mut self) {
        if !self.at_contextual("permits") {
            return;
        }

        let clause = self.start();
        self.bump();
        self.parse_type_name_list_until_clause_end();
        self.complete(clause, JavaSyntaxKind::PermitsClause);
    }

    pub(super) fn parse_type_list_until_clause_end(&mut self) {
        while !self.at_eof() && !self.at_header_clause_end() {
            self.parse_class_type();
            if !self.eat(JavaSyntaxKind::Comma) {
                break;
            }
        }
    }

    pub(super) fn parse_type_name_list_until_clause_end(&mut self) {
        while !self.at_eof() && !self.at_header_clause_end() {
            self.consume_qualified_name();
            if !self.eat(JavaSyntaxKind::Comma) {
                break;
            }
        }
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
        self.parse_annotations();
        self.parse_type();
        self.parse_annotations();
        self.eat(JavaSyntaxKind::Ellipsis);
        self.expect_named_variable_identifier("expected record component name");
        self.complete(component, JavaSyntaxKind::RecordComponent);
    }

    pub(super) fn parse_body_declaration(
        &mut self,
        body_kind: JavaSyntaxKind,
        type_name: Option<&str>,
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
        type_name: Option<&str>,
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
        type_name: Option<&str>,
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
        self.parse_variable_declarator_id(allow_unnamed);
        if self.eat(JavaSyntaxKind::Assign) {
            self.parse_variable_initializer_until(stops);
        }
        self.complete(declarator, JavaSyntaxKind::VariableDeclarator);
    }

    pub(super) fn parse_variable_declarator_id(&mut self, allow_unnamed: bool) {
        if allow_unnamed {
            self.expect_variable_identifier("expected variable name");
        } else {
            self.expect_named_variable_identifier("expected variable name");
        }
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
        self.parse_method_modifier_list();
        self.parse_optional_type_parameter_list();
        self.parse_annotations();
        self.parse_result_type();
        self.expect_method_identifier("expected method name");
        self.parse_formal_parameter_section();
        self.parse_array_dimensions();
        self.parse_optional_throws_clause();
        self.parse_method_body();
        self.complete(method, JavaSyntaxKind::MethodDeclaration);
    }

    pub(super) fn parse_annotation_element(&mut self) {
        let element = self.start();
        self.parse_annotation_element_modifier_list();
        self.parse_type();
        self.expect_method_identifier("expected annotation element name");
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
        self.expect_type_identifier("expected constructor name");
        self.parse_formal_parameter_section();
        self.parse_optional_throws_clause();
        self.parse_constructor_block();
        self.complete(constructor, JavaSyntaxKind::ConstructorDeclaration);
    }

    pub(super) fn parse_compact_constructor_declaration(&mut self) {
        let constructor = self.start();
        self.parse_constructor_modifier_list();
        self.expect_type_identifier("expected compact constructor name");
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
                if allow_receiver && self.starts_receiver_parameter() {
                    self.parse_receiver_parameter();
                } else if self.starts_receiver_parameter() {
                    let error = self.start();
                    self.misplaced_receiver_parameter_here("receiver parameter must be first");
                    self.parse_receiver_parameter();
                    self.complete(error, JavaSyntaxKind::ErrorNode);
                } else {
                    let was_varargs = self.parse_formal_parameter();
                    if was_varargs && !self.at(JavaSyntaxKind::RParen) && !self.at_eof() {
                        let error = self.start();
                        self.expected_here("varargs parameter must be last");
                        let consumed_comma = self.eat(JavaSyntaxKind::Comma);
                        self.complete(error, JavaSyntaxKind::ErrorNode);
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

    pub(super) fn parse_receiver_parameter(&mut self) {
        let parameter = self.start();
        self.parse_annotations();
        self.parse_type();
        if self.at_name_segment() && self.nth_kind(1) == JavaSyntaxKind::Dot {
            self.bump();
            self.bump();
        }
        self.expect(
            JavaSyntaxKind::ThisKw,
            "expected `this` in receiver parameter",
        );
        self.complete(parameter, JavaSyntaxKind::ReceiverParameter);
    }

    pub(super) fn parse_formal_parameter(&mut self) -> bool {
        let parameter = self.start();
        self.parse_variable_modifiers();
        self.parse_type();
        self.parse_annotations();
        let varargs = self.eat(JavaSyntaxKind::Ellipsis);
        self.expect_variable_identifier("expected parameter name");
        self.parse_array_dimensions();
        self.complete(parameter, JavaSyntaxKind::FormalParameter);
        varargs
    }

    pub(super) fn parse_variable_modifiers(&mut self) -> bool {
        let start = self.position();
        loop {
            if self.at(JavaSyntaxKind::At) && self.nth_kind(1) != JavaSyntaxKind::InterfaceKw {
                self.parse_annotation();
            } else if self.at(JavaSyntaxKind::FinalKw) {
                self.bump();
            } else {
                break;
            }
        }
        self.position() != start
    }

    pub(super) fn parse_optional_throws_clause(&mut self) {
        if !self.at(JavaSyntaxKind::ThrowsKw) {
            return;
        }

        let clause = self.start();
        self.bump();
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
        let mut allow_constructor_invocation = true;
        while !self.at_eof() && !self.at(JavaSyntaxKind::RBrace) {
            if allow_constructor_invocation && self.starts_constructor_invocation_statement() {
                self.parse_constructor_invocation();
                allow_constructor_invocation = false;
            } else if self.starts_constructor_invocation_statement() {
                let error = self.start();
                self.misplaced_constructor_invocation_here(
                    "explicit constructor invocation must be first in constructor body",
                );
                self.parse_constructor_invocation();
                self.complete(error, JavaSyntaxKind::ErrorNode);
            } else {
                self.parse_block_statement();
                allow_constructor_invocation = false;
            }
        }
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
            } else {
                self.expected_here("expected `this` or `super` in constructor invocation");
            }
            self.parse_argument_list();
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
                    let invocation = self.precede(expression);
                    self.parse_argument_list();
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

    pub(super) fn parse_enum_body_contents(&mut self, type_name: Option<&str>) {
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

        if self.eat(JavaSyntaxKind::Semicolon) {
            while !self.at_eof() && !self.at(JavaSyntaxKind::RBrace) {
                self.parse_body_declaration(JavaSyntaxKind::ClassBody, type_name);
            }
        }
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
        while !self.at_eof() && !self.at(JavaSyntaxKind::RBrace) {
            if self.at(JavaSyntaxKind::Semicolon) {
                self.parse_empty_declaration();
            } else {
                self.parse_member_declaration(None, true);
            }
        }
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
            Self::Type => true,
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
