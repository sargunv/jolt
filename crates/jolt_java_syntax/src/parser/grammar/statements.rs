use super::{JavaParserExt, JavaSyntaxKind, Parser};
use jolt_syntax::{NodeAnchor, UnresolvedDiagnosticOwner};

const STATEMENT_BOUNDARIES: &[JavaSyntaxKind] = &[
    JavaSyntaxKind::ElseKw,
    JavaSyntaxKind::CaseKw,
    JavaSyntaxKind::DefaultKw,
    JavaSyntaxKind::RBrace,
];
const DO_BODY_BOUNDARIES: &[JavaSyntaxKind] = &[
    JavaSyntaxKind::WhileKw,
    JavaSyntaxKind::ElseKw,
    JavaSyntaxKind::CaseKw,
    JavaSyntaxKind::DefaultKw,
    JavaSyntaxKind::RBrace,
];

impl Parser<'_> {
    fn parse_required_statement(&mut self, owner: NodeAnchor, slot: u16, stops: &[JavaSyntaxKind]) {
        if self.at_eof() || stops.contains(&self.current_kind()) {
            self.expected_owned_slot("expected statement", owner, slot);
        } else {
            self.parse_statement();
        }
    }

    fn parse_required_block(&mut self, owner: NodeAnchor, slot: u16, message: &'static str) {
        if self.at(JavaSyntaxKind::LBrace) {
            self.parse_block();
        } else {
            self.expected_owned_slot(message, owner, slot);
        }
    }

    pub(super) fn parse_block(&mut self) {
        let block = self.start();
        self.expect(JavaSyntaxKind::LBrace, "expected block");

        let statements = self.start();
        while !self.at_eof() && !self.at(JavaSyntaxKind::RBrace) {
            self.parse_block_statement();
        }
        self.complete(statements, JavaSyntaxKind::BlockStatementList);

        self.expect(JavaSyntaxKind::RBrace, "expected `}` after block");
        self.complete(block, JavaSyntaxKind::Block);
    }

    pub(super) fn parse_block_statement(&mut self) {
        let block_statement = self.start();

        if self.starts_local_class_or_interface_declaration() {
            self.parse_local_class_or_interface_declaration();
        } else if self.starts_local_variable_declaration() {
            self.parse_local_variable_declaration_statement();
        } else {
            self.parse_statement();
        }

        self.complete(block_statement, JavaSyntaxKind::BlockStatement);
    }

    pub(super) fn parse_local_class_or_interface_declaration(&mut self) {
        let declaration = self.start();
        self.parse_type_declaration(JavaSyntaxKind::BogusTypeDeclaration);
        self.complete(
            declaration,
            JavaSyntaxKind::LocalClassOrInterfaceDeclaration,
        );
    }

    pub(super) fn parse_local_variable_declaration_statement(&mut self) {
        self.parse_local_variable_declaration_until(&[JavaSyntaxKind::Semicolon]);
        self.expect(
            JavaSyntaxKind::Semicolon,
            "expected `;` after local variable declaration",
        );
    }

    pub(super) fn parse_local_variable_declaration_until(&mut self, stops: &[JavaSyntaxKind]) {
        let declaration = self.start();
        self.parse_variable_modifiers();
        self.parse_local_variable_type();
        self.parse_variable_declarator_list_until(stops);
        self.complete(declaration, JavaSyntaxKind::LocalVariableDeclaration);
    }

    pub(super) fn parse_local_variable_type(&mut self) {
        if self.at_contextual("var") && self.nth_kind(1) != JavaSyntaxKind::Dot {
            self.bump();
        } else {
            self.parse_type();
        }
    }

    pub(super) fn parse_statement(&mut self) {
        match self.current_kind() {
            JavaSyntaxKind::LBrace => self.parse_block(),
            JavaSyntaxKind::Semicolon => self.parse_empty_statement(),
            JavaSyntaxKind::AssertKw => self.parse_assert_statement(),
            JavaSyntaxKind::BreakKw => self.parse_break_statement(),
            JavaSyntaxKind::ContinueKw => self.parse_continue_statement(),
            JavaSyntaxKind::DoKw => self.parse_do_statement(),
            JavaSyntaxKind::ForKw => self.parse_for_statement(),
            JavaSyntaxKind::IfKw => self.parse_if_statement(),
            JavaSyntaxKind::ReturnKw => self.parse_return_statement(),
            JavaSyntaxKind::SwitchKw => self.parse_switch_statement(),
            JavaSyntaxKind::SynchronizedKw => self.parse_synchronized_statement(),
            JavaSyntaxKind::ThrowKw => self.parse_throw_statement(),
            JavaSyntaxKind::TryKw => self.parse_try_statement(),
            JavaSyntaxKind::WhileKw => self.parse_while_statement(),
            _ if self.starts_yield_statement() => self.parse_yield_statement(),
            _ if self.starts_labeled_statement() => self.parse_labeled_statement(),
            _ => self.parse_expression_statement(),
        }
    }

    pub(super) fn parse_empty_statement(&mut self) {
        let statement = self.start();
        self.expect(JavaSyntaxKind::Semicolon, "expected `;`");
        self.complete(statement, JavaSyntaxKind::EmptyStatement);
    }

    pub(super) fn parse_labeled_statement(&mut self) {
        let statement = self.start();
        let owner = statement.anchor();
        self.expect_named_variable_identifier("expected statement label");
        self.expect(JavaSyntaxKind::Colon, "expected `:` after label");
        self.parse_required_statement(
            owner,
            crate::shape::labeled_statement::Slot::body as u16,
            STATEMENT_BOUNDARIES,
        );
        self.complete(statement, JavaSyntaxKind::LabeledStatement);
    }

    pub(super) fn parse_expression_statement(&mut self) {
        let statement = self.start();
        self.consume_statement_expression_until(&[JavaSyntaxKind::Semicolon]);
        self.expect(
            JavaSyntaxKind::Semicolon,
            "expected `;` after expression statement",
        );
        self.complete(statement, JavaSyntaxKind::ExpressionStatement);
    }

    pub(super) fn parse_if_statement(&mut self) {
        let statement = self.start();
        let owner = statement.anchor();
        self.expect(JavaSyntaxKind::IfKw, "expected `if`");
        self.parse_parenthesized_expression(
            "expected `(` before if condition",
            "expected `)` after if condition",
        );
        self.parse_required_statement(
            owner,
            crate::shape::if_statement::Slot::then_branch as u16,
            STATEMENT_BOUNDARIES,
        );
        if self.eat(JavaSyntaxKind::ElseKw) {
            self.parse_required_statement(
                owner,
                crate::shape::if_statement::Slot::else_branch as u16,
                STATEMENT_BOUNDARIES,
            );
        }
        self.complete(statement, JavaSyntaxKind::IfStatement);
    }

    pub(super) fn parse_assert_statement(&mut self) {
        let statement = self.start();
        self.expect(JavaSyntaxKind::AssertKw, "expected `assert`");
        self.parse_expression_until(&[JavaSyntaxKind::Colon, JavaSyntaxKind::Semicolon]);
        if self.eat(JavaSyntaxKind::Colon) {
            self.parse_expression_until(&[JavaSyntaxKind::Semicolon]);
        }
        self.expect(
            JavaSyntaxKind::Semicolon,
            "expected `;` after assert statement",
        );
        self.complete(statement, JavaSyntaxKind::AssertStatement);
    }

    pub(super) fn parse_while_statement(&mut self) {
        let statement = self.start();
        let owner = statement.anchor();
        self.expect(JavaSyntaxKind::WhileKw, "expected `while`");
        self.parse_parenthesized_expression(
            "expected `(` before while condition",
            "expected `)` after while condition",
        );
        self.parse_required_statement(
            owner,
            crate::shape::while_statement::Slot::body as u16,
            STATEMENT_BOUNDARIES,
        );
        self.complete(statement, JavaSyntaxKind::WhileStatement);
    }

    pub(super) fn parse_do_statement(&mut self) {
        let statement = self.start();
        let owner = statement.anchor();
        self.expect(JavaSyntaxKind::DoKw, "expected `do`");
        self.parse_required_statement(
            owner,
            crate::shape::do_statement::Slot::body as u16,
            DO_BODY_BOUNDARIES,
        );
        self.expect(JavaSyntaxKind::WhileKw, "expected `while` after do body");
        self.parse_parenthesized_expression(
            "expected `(` before do condition",
            "expected `)` after do condition",
        );
        self.expect(JavaSyntaxKind::Semicolon, "expected `;` after do statement");
        self.complete(statement, JavaSyntaxKind::DoStatement);
    }

    pub(super) fn parse_for_statement(&mut self) {
        let statement = self.start();
        if self.for_header_has_top_level_colon() {
            let enhanced = self.start();
            let owner = enhanced.anchor();
            self.expect(JavaSyntaxKind::ForKw, "expected `for`");
            self.expect(JavaSyntaxKind::LParen, "expected `(` after `for`");
            self.parse_enhanced_for_variable_declaration_until(&[JavaSyntaxKind::Colon]);
            self.expect(
                JavaSyntaxKind::Colon,
                "expected `:` in enhanced for statement",
            );
            self.parse_expression_until(&[JavaSyntaxKind::RParen]);
            self.expect(
                JavaSyntaxKind::RParen,
                "expected `)` after enhanced for header",
            );
            self.parse_required_statement(
                owner,
                crate::shape::enhanced_for_statement::Slot::body as u16,
                STATEMENT_BOUNDARIES,
            );
            self.complete(enhanced, JavaSyntaxKind::EnhancedForStatement);
        } else {
            let basic = self.start();
            let owner = basic.anchor();
            self.expect(JavaSyntaxKind::ForKw, "expected `for`");
            self.expect(JavaSyntaxKind::LParen, "expected `(` after `for`");
            if !self.at(JavaSyntaxKind::Semicolon) {
                let initializer = self.start();
                if self.starts_local_variable_declaration() {
                    self.parse_local_variable_declaration_until(&[JavaSyntaxKind::Semicolon]);
                } else {
                    self.parse_statement_expression_list(JavaSyntaxKind::Semicolon);
                }
                self.complete(initializer, JavaSyntaxKind::ForInitializer);
            }
            self.expect(JavaSyntaxKind::Semicolon, "expected `;` in for header");
            if !self.at(JavaSyntaxKind::Semicolon) {
                self.parse_expression_until(&[JavaSyntaxKind::Semicolon]);
            }
            self.expect(JavaSyntaxKind::Semicolon, "expected `;` in for header");
            if !self.at(JavaSyntaxKind::RParen) {
                let update = self.start();
                self.parse_statement_expression_list(JavaSyntaxKind::RParen);
                self.complete(update, JavaSyntaxKind::ForUpdate);
            }
            self.expect(JavaSyntaxKind::RParen, "expected `)` after for header");
            self.parse_required_statement(
                owner,
                crate::shape::basic_for_statement::Slot::body as u16,
                STATEMENT_BOUNDARIES,
            );
            self.complete(basic, JavaSyntaxKind::BasicForStatement);
        }
        self.complete(statement, JavaSyntaxKind::ForStatement);
    }

    pub(super) fn parse_enhanced_for_variable_declaration_until(
        &mut self,
        stops: &[JavaSyntaxKind],
    ) {
        let variable = self.start();
        self.parse_variable_modifiers();
        self.parse_local_variable_type();
        self.parse_variable_declarator_id(
            true,
            variable.anchor(),
            crate::shape::enhanced_for_variable::Slot::name as u16,
        );
        let mut malformed = false;
        if self.eat(JavaSyntaxKind::Assign) {
            let diagnostic =
                self.expected_here("enhanced for variable must not have an initializer");
            self.own_diagnostic(
                diagnostic,
                UnresolvedDiagnosticOwner::node(variable.anchor()),
            );
            malformed = true;
            self.parse_variable_initializer_until(stops);
        }

        while self.at(JavaSyntaxKind::Comma) {
            let diagnostic =
                self.unexpected_here("enhanced for statement must declare a single variable");
            self.own_diagnostic(
                diagnostic,
                UnresolvedDiagnosticOwner::node(variable.anchor()),
            );
            malformed = true;
            self.bump();
            if stops.contains(&self.current_kind()) {
                break;
            }
            self.parse_variable_declarator_until(stops, true);
        }

        let kind = if malformed {
            JavaSyntaxKind::BogusEnhancedForVariable
        } else {
            JavaSyntaxKind::EnhancedForVariable
        };
        self.complete(variable, kind);
    }

    pub(super) fn parse_statement_expression_list(&mut self, stop: JavaSyntaxKind) {
        let list = self.start();
        loop {
            self.consume_statement_expression_until(&[JavaSyntaxKind::Comma, stop]);
            if !self.eat(JavaSyntaxKind::Comma) || self.at(stop) {
                break;
            }
        }
        self.complete(list, JavaSyntaxKind::StatementExpressionList);
    }

    pub(super) fn parse_break_statement(&mut self) {
        self.parse_jump_statement(
            JavaSyntaxKind::BreakKw,
            JavaSyntaxKind::BreakStatement,
            "expected `break`",
            "expected `;` after break statement",
        );
    }

    pub(super) fn parse_continue_statement(&mut self) {
        self.parse_jump_statement(
            JavaSyntaxKind::ContinueKw,
            JavaSyntaxKind::ContinueStatement,
            "expected `continue`",
            "expected `;` after continue statement",
        );
    }

    pub(super) fn parse_jump_statement(
        &mut self,
        keyword: JavaSyntaxKind,
        kind: JavaSyntaxKind,
        keyword_message: &str,
        semicolon_message: &str,
    ) {
        let statement = self.start();
        self.expect(keyword, keyword_message);
        if self.at_name_segment() {
            self.bump();
        }
        self.expect(JavaSyntaxKind::Semicolon, semicolon_message);
        self.complete(statement, kind);
    }

    pub(super) fn parse_yield_statement(&mut self) {
        let statement = self.start();
        self.expect_contextual("yield", "expected `yield`");
        self.parse_expression_until(&[JavaSyntaxKind::Semicolon]);
        self.expect(
            JavaSyntaxKind::Semicolon,
            "expected `;` after yield statement",
        );
        self.complete(statement, JavaSyntaxKind::YieldStatement);
    }

    pub(super) fn parse_return_statement(&mut self) {
        let statement = self.start();
        self.expect(JavaSyntaxKind::ReturnKw, "expected `return`");
        if !self.at(JavaSyntaxKind::Semicolon) {
            self.parse_expression_until(&[JavaSyntaxKind::Semicolon]);
        }
        self.expect(
            JavaSyntaxKind::Semicolon,
            "expected `;` after return statement",
        );
        self.complete(statement, JavaSyntaxKind::ReturnStatement);
    }

    pub(super) fn parse_throw_statement(&mut self) {
        let statement = self.start();
        self.expect(JavaSyntaxKind::ThrowKw, "expected `throw`");
        self.parse_expression_until(&[JavaSyntaxKind::Semicolon]);
        self.expect(
            JavaSyntaxKind::Semicolon,
            "expected `;` after throw statement",
        );
        self.complete(statement, JavaSyntaxKind::ThrowStatement);
    }

    pub(super) fn parse_synchronized_statement(&mut self) {
        let statement = self.start();
        let owner = statement.anchor();
        self.expect(JavaSyntaxKind::SynchronizedKw, "expected `synchronized`");
        self.parse_parenthesized_expression(
            "expected `(` before synchronized expression",
            "expected `)` after synchronized expression",
        );
        self.parse_required_block(
            owner,
            crate::shape::synchronized_statement::Slot::body as u16,
            "expected synchronized body",
        );
        self.complete(statement, JavaSyntaxKind::SynchronizedStatement);
    }

    pub(super) fn parse_try_statement(&mut self) {
        if self.nth_kind(1) == JavaSyntaxKind::LParen {
            self.parse_try_with_resources_statement();
            return;
        }

        let statement = self.start();
        let owner = statement.anchor();
        self.expect(JavaSyntaxKind::TryKw, "expected `try`");
        self.parse_required_block(
            owner,
            crate::shape::try_statement::Slot::body as u16,
            "expected try body",
        );
        let mut saw_handler = false;
        let catches = self.start();
        while self.at(JavaSyntaxKind::CatchKw) {
            self.parse_catch_clause();
            saw_handler = true;
        }
        self.complete(catches, JavaSyntaxKind::CatchClauseList);
        if self.at(JavaSyntaxKind::FinallyKw) {
            self.parse_finally_clause();
            saw_handler = true;
        }
        if saw_handler {
            self.complete(statement, JavaSyntaxKind::TryStatement);
        } else {
            let diagnostic = self.expected_here("expected `catch` or `finally` after try block");
            self.own_diagnostic(diagnostic, UnresolvedDiagnosticOwner::node(owner));
            self.complete(statement, JavaSyntaxKind::BogusStatement);
        }
    }

    pub(super) fn parse_try_with_resources_statement(&mut self) {
        let statement = self.start();
        let owner = statement.anchor();
        self.expect(JavaSyntaxKind::TryKw, "expected `try`");
        let specification = self.start();
        self.expect(JavaSyntaxKind::LParen, "expected resource specification");
        let resources = self.start();
        if self.at(JavaSyntaxKind::RParen) {
            let resource = self.start();
            let value = self.start();
            let diagnostic = self.expected_here("expected resource");
            self.own_diagnostic(diagnostic, UnresolvedDiagnosticOwner::node(value.anchor()));
            self.complete(value, JavaSyntaxKind::BogusResourceValue);
            self.complete(resource, JavaSyntaxKind::Resource);
        }
        while !self.at_eof()
            && !self.at(JavaSyntaxKind::RParen)
            && !self.at(JavaSyntaxKind::Semicolon)
        {
            self.parse_resource();
            if !self.at(JavaSyntaxKind::Semicolon)
                || matches!(
                    self.nth_kind(1),
                    JavaSyntaxKind::RParen | JavaSyntaxKind::Eof
                )
            {
                break;
            }
            self.bump();
        }
        self.complete(resources, JavaSyntaxKind::ResourceList);
        self.eat(JavaSyntaxKind::Semicolon);
        self.expect(JavaSyntaxKind::RParen, "expected `)` after resources");
        self.complete(specification, JavaSyntaxKind::ResourceSpecification);
        self.parse_required_block(
            owner,
            crate::shape::try_with_resources_statement::Slot::body as u16,
            "expected try body",
        );
        let catches = self.start();
        while self.at(JavaSyntaxKind::CatchKw) {
            self.parse_catch_clause();
        }
        self.complete(catches, JavaSyntaxKind::CatchClauseList);
        if self.at(JavaSyntaxKind::FinallyKw) {
            self.parse_finally_clause();
        }
        self.complete(statement, JavaSyntaxKind::TryWithResourcesStatement);
    }

    pub(super) fn parse_resource(&mut self) {
        let resource = self.start();
        if self.starts_resource_local_variable_declaration() {
            self.parse_resource_variable_declaration_until(&[
                JavaSyntaxKind::Semicolon,
                JavaSyntaxKind::RParen,
            ]);
        } else {
            self.parse_resource_variable_access_until(&[
                JavaSyntaxKind::Semicolon,
                JavaSyntaxKind::RParen,
            ]);
        }
        self.complete(resource, JavaSyntaxKind::Resource);
    }

    pub(super) fn parse_resource_variable_declaration_until(&mut self, stops: &[JavaSyntaxKind]) {
        let declaration = self.start();
        self.parse_variable_modifiers();
        self.parse_local_variable_type();
        self.parse_variable_declarator_id(
            true,
            declaration.anchor(),
            crate::shape::resource_variable_declaration::Slot::name as u16,
        );
        let mut malformed = false;
        if self.eat(JavaSyntaxKind::Assign) {
            self.parse_variable_initializer_until(stops);
        } else {
            let diagnostic = self.expected_here("expected resource initializer");
            self.own_diagnostic(
                diagnostic,
                UnresolvedDiagnosticOwner::node(declaration.anchor()),
            );
            malformed = true;
        }

        while self.at(JavaSyntaxKind::Comma) {
            let diagnostic =
                self.unexpected_here("resource declaration must declare a single variable");
            self.own_diagnostic(
                diagnostic,
                UnresolvedDiagnosticOwner::node(declaration.anchor()),
            );
            malformed = true;
            self.bump();
            if stops.contains(&self.current_kind()) {
                break;
            }
            self.parse_variable_declarator_until(stops, true);
        }

        self.complete(
            declaration,
            if malformed {
                JavaSyntaxKind::BogusResourceValue
            } else {
                JavaSyntaxKind::ResourceVariableDeclaration
            },
        );
    }

    pub(super) fn parse_resource_variable_access_until(&mut self, stops: &[JavaSyntaxKind]) {
        if self.at_eof() || stops.contains(&self.current_kind()) {
            let access = self.start();
            let diagnostic = self.invalid_resource_variable_access_here(
                "expected resource variable declaration or variable access",
            );
            self.own_diagnostic(diagnostic, UnresolvedDiagnosticOwner::node(access.anchor()));
            self.complete(access, JavaSyntaxKind::BogusResourceValue);
            return;
        }

        let expression = self.parse_expression();
        let expression_kind = JavaSyntaxKind::from_raw(expression.kind());
        let valid = matches!(
            expression_kind,
            Some(JavaSyntaxKind::NameExpression | JavaSyntaxKind::FieldAccessExpression)
        );

        if valid && (self.at_eof() || stops.contains(&self.current_kind())) {
            let access = self.precede(expression);
            self.complete(access, JavaSyntaxKind::VariableAccess);
        } else {
            let access = self.precede(expression);
            let diagnostic = self.invalid_resource_variable_access_here(if valid {
                "unexpected token in resource variable access"
            } else {
                "expected resource variable declaration or variable access"
            });
            while !self.at_eof() && !stops.contains(&self.current_kind()) {
                self.bump();
            }
            self.own_diagnostic(diagnostic, UnresolvedDiagnosticOwner::node(access.anchor()));
            self.complete(access, JavaSyntaxKind::BogusResourceValue);
        }
    }

    pub(super) fn parse_catch_clause(&mut self) {
        let clause = self.start();
        let owner = clause.anchor();
        self.expect(JavaSyntaxKind::CatchKw, "expected `catch`");
        self.expect(JavaSyntaxKind::LParen, "expected `(` after `catch`");
        let parameter = self.start();
        self.parse_variable_modifiers();
        let types = self.start();
        self.parse_class_union_type();
        self.complete(types, JavaSyntaxKind::CatchTypeList);
        self.expect_variable_identifier("expected catch parameter name");
        self.parse_array_dimensions();
        self.complete(parameter, JavaSyntaxKind::CatchParameter);
        self.expect(JavaSyntaxKind::RParen, "expected `)` after catch parameter");
        self.parse_required_block(
            owner,
            crate::shape::catch_clause::Slot::body as u16,
            "expected catch body",
        );
        self.complete(clause, JavaSyntaxKind::CatchClause);
    }

    pub(super) fn parse_finally_clause(&mut self) {
        let clause = self.start();
        let owner = clause.anchor();
        self.expect(JavaSyntaxKind::FinallyKw, "expected `finally`");
        self.parse_required_block(
            owner,
            crate::shape::finally_clause::Slot::body as u16,
            "expected finally body",
        );
        self.complete(clause, JavaSyntaxKind::FinallyClause);
    }

    pub(super) fn parse_switch_statement(&mut self) {
        let statement = self.start();
        let owner = statement.anchor();
        self.expect(JavaSyntaxKind::SwitchKw, "expected `switch`");
        self.parse_parenthesized_expression(
            "expected `(` before switch selector",
            "expected `)` after switch selector",
        );
        if self.at(JavaSyntaxKind::LBrace) {
            self.parse_switch_block();
        } else {
            self.expected_owned_slot(
                "expected switch block",
                owner,
                crate::shape::switch_statement::Slot::body as u16,
            );
        }
        self.complete(statement, JavaSyntaxKind::SwitchStatement);
    }

    pub(super) fn parse_switch_expression_fragment(&mut self) -> jolt_syntax::CompletedMarker {
        let expression = self.start();
        let owner = expression.anchor();
        self.expect(JavaSyntaxKind::SwitchKw, "expected `switch`");
        self.parse_parenthesized_expression(
            "expected `(` before switch selector",
            "expected `)` after switch selector",
        );
        if self.at(JavaSyntaxKind::LBrace) {
            self.parse_switch_block();
        } else {
            self.expected_owned_slot(
                "expected switch block",
                owner,
                crate::shape::switch_expression::Slot::body as u16,
            );
        }
        self.complete(expression, JavaSyntaxKind::SwitchExpression)
    }

    pub(super) fn parse_switch_block(&mut self) {
        let block = self.start();
        self.expect(JavaSyntaxKind::LBrace, "expected switch block");
        let entries = self.start();
        while !self.at_eof() && !self.at(JavaSyntaxKind::RBrace) {
            if self.starts_switch_label() {
                if self.switch_label_is_rule() {
                    self.parse_switch_rule();
                } else {
                    self.parse_switch_block_statement_group_or_label();
                }
            } else {
                let entry = self.start();
                let diagnostic = self.unexpected_here("expected switch label");
                self.parse_block_statement();
                self.own_diagnostic(diagnostic, UnresolvedDiagnosticOwner::node(entry.anchor()));
                self.complete(entry, JavaSyntaxKind::BogusSwitchEntry);
            }
        }
        self.complete(entries, JavaSyntaxKind::SwitchEntryList);
        self.expect(JavaSyntaxKind::RBrace, "expected `}` after switch block");
        self.complete(block, JavaSyntaxKind::SwitchBlock);
    }

    pub(super) fn parse_switch_rule(&mut self) {
        let rule = self.start();
        self.parse_switch_label();
        self.expect(JavaSyntaxKind::Arrow, "expected `->` after switch label");
        if self.at(JavaSyntaxKind::LBrace) {
            self.parse_block();
        } else if self.at(JavaSyntaxKind::ThrowKw) {
            self.parse_throw_statement();
        } else {
            self.parse_expression_until(&[JavaSyntaxKind::Semicolon]);
            self.expect(JavaSyntaxKind::Semicolon, "expected `;` after switch rule");
        }
        self.complete(rule, JavaSyntaxKind::SwitchRule);
    }

    pub(super) fn parse_switch_block_statement_group_or_label(&mut self) {
        let group = self.start();
        let labels = self.start();
        loop {
            self.parse_switch_label();
            self.expect(JavaSyntaxKind::Colon, "expected `:` after switch label");
            if !self.starts_switch_label() || self.switch_label_is_rule() {
                break;
            }
        }
        self.complete(labels, JavaSyntaxKind::SwitchLabelColonList);

        if self.starts_switch_label() || self.at(JavaSyntaxKind::RBrace) {
            self.complete(group, JavaSyntaxKind::SwitchBlockStatementGroup);
            return;
        }

        let statements = self.start();
        while !self.at_eof() && !self.at(JavaSyntaxKind::RBrace) && !self.starts_switch_label() {
            self.parse_block_statement();
        }
        self.complete(statements, JavaSyntaxKind::BlockStatementList);
        self.complete(group, JavaSyntaxKind::SwitchBlockStatementGroup);
    }

    pub(super) fn parse_switch_label(&mut self) {
        let label = self.start();
        if self.eat(JavaSyntaxKind::DefaultKw) {
            let items = self.start();
            self.complete(items, JavaSyntaxKind::SwitchLabelItemList);
            self.complete(label, JavaSyntaxKind::SwitchLabel);
            return;
        }

        self.expect(JavaSyntaxKind::CaseKw, "expected `case`");
        let items = self.start();
        let mut saw_case_item = false;
        let mut previous_was_pattern = false;
        while !self.at_eof()
            && !matches!(
                self.current_kind(),
                JavaSyntaxKind::Colon | JavaSyntaxKind::Arrow
            )
        {
            if self.eat(JavaSyntaxKind::Comma) {
                saw_case_item = false;
                previous_was_pattern = false;
            } else if self.at_contextual("when") && saw_case_item {
                break;
            } else if self.starts_pattern() {
                let case_pattern = self.start();
                self.parse_pattern_until(&[
                    JavaSyntaxKind::Comma,
                    JavaSyntaxKind::Colon,
                    JavaSyntaxKind::Arrow,
                ]);
                self.complete(case_pattern, JavaSyntaxKind::CasePattern);
                saw_case_item = true;
                previous_was_pattern = true;
            } else if self.at(JavaSyntaxKind::DefaultKw) {
                self.bump();
                saw_case_item = true;
                previous_was_pattern = false;
            } else {
                self.parse_case_constant();
                saw_case_item = true;
                previous_was_pattern = false;
            }
        }
        self.complete(items, JavaSyntaxKind::SwitchLabelItemList);
        if self.at_contextual("when") {
            if previous_was_pattern {
                self.parse_guard();
            } else {
                let guard = self.start();
                let diagnostic = self.invalid_switch_guard_here("switch guard requires a pattern");
                self.parse_guard();
                self.own_diagnostic(diagnostic, UnresolvedDiagnosticOwner::node(guard.anchor()));
                self.complete(guard, JavaSyntaxKind::BogusSwitchGuard);
            }
        }
        self.complete(label, JavaSyntaxKind::SwitchLabel);
    }

    pub(super) fn parse_case_constant(&mut self) {
        let case_constant = self.start();
        self.parse_conditional_expression();
        let mut diagnostic = None;
        while !self.at_eof()
            && !matches!(
                self.current_kind(),
                JavaSyntaxKind::Comma | JavaSyntaxKind::Colon | JavaSyntaxKind::Arrow
            )
            && !self.at_contextual("when")
        {
            if diagnostic.is_none() {
                diagnostic = Some(self.unexpected_here("unexpected token in case constant"));
            }
            self.bump();
        }
        if let Some(diagnostic) = diagnostic {
            self.own_diagnostic(
                diagnostic,
                UnresolvedDiagnosticOwner::node(case_constant.anchor()),
            );
            self.complete(case_constant, JavaSyntaxKind::BogusSwitchLabelItem);
        } else {
            self.complete(case_constant, JavaSyntaxKind::CaseConstant);
        }
    }

    pub(super) fn parse_guard(&mut self) {
        let guard = self.start();
        self.expect_contextual("when", "expected `when`");
        if self.starts_parenthesized_lambda_expression() {
            self.parse_parenthesized_expression(
                "expected `(` before switch guard",
                "expected `)` after switch guard",
            );
        } else {
            self.parse_expression_until_without_leading_lambda(&[
                JavaSyntaxKind::Colon,
                JavaSyntaxKind::Arrow,
            ]);
        }
        self.complete(guard, JavaSyntaxKind::Guard);
    }

    pub(super) fn parse_parenthesized_expression(
        &mut self,
        open_message: &'static str,
        close_message: &'static str,
    ) {
        self.expect(JavaSyntaxKind::LParen, open_message);
        self.parse_expression_until(&[JavaSyntaxKind::RParen]);
        self.expect(JavaSyntaxKind::RParen, close_message);
    }
}
