use super::{BODY_STOPS, JavaSyntaxKind, Parser};
use crate::parser::codes;

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
    fn parse_required_statement(&mut self, stops: &[JavaSyntaxKind]) {
        if self.at_eof() || stops.contains(&self.current()) {
            self.error_expected("expected statement");
        } else {
            self.parse_statement();
        }
    }

    fn parse_required_block(&mut self, message: &'static str) {
        if self.at(JavaSyntaxKind::LBrace) {
            self.parse_block();
        } else {
            self.error_expected(message);
        }
    }

    pub(super) fn parse_block(&mut self) {
        let block = self.start();
        self.expect(JavaSyntaxKind::LBrace, "expected block");

        let statements = self.start();
        let mut guard = self.loop_guard();
        while !self.at(JavaSyntaxKind::RBrace) && self.check(&mut guard) {
            self.parse_block_statement();
        }
        self.complete(statements, JavaSyntaxKind::BlockStatementList);

        self.expect(JavaSyntaxKind::RBrace, "expected `}` after block");
        self.complete(block, JavaSyntaxKind::Block);
    }

    pub(super) fn parse_block_statement(&mut self) {
        let block_statement = self.start();

        if self.starts_type_declaration() {
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
        self.parse_type_declaration();
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
        if self.at_contextual("var") && self.nth(1) != JavaSyntaxKind::Dot {
            self.bump();
        } else {
            self.parse_type();
        }
    }

    pub(super) fn parse_statement(&mut self) {
        if self.nested(Self::parse_statement_inner).is_none() {
            self.recover_too_deep(BODY_STOPS);
        }
    }

    fn parse_statement_inner(&mut self) {
        match self.current() {
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
        self.expect_variable_identifier("expected statement label", false);
        self.expect(JavaSyntaxKind::Colon, "expected `:` after label");
        self.parse_required_statement(STATEMENT_BOUNDARIES);
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
        self.expect(JavaSyntaxKind::IfKw, "expected `if`");
        self.parse_parenthesized_expression(
            "expected `(` before if condition",
            "expected `)` after if condition",
        );
        self.parse_required_statement(STATEMENT_BOUNDARIES);
        if self.eat(JavaSyntaxKind::ElseKw) {
            if self.at_eof() || STATEMENT_BOUNDARIES.contains(&self.current()) {
                // The else branch is optional in the schema, so an absent
                // one is represented by an empty bogus statement.
                let missing = self.start();
                self.error_expected("expected statement");
                self.complete(missing, JavaSyntaxKind::Bogus);
            } else {
                self.parse_statement();
            }
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
        self.expect(JavaSyntaxKind::WhileKw, "expected `while`");
        self.parse_parenthesized_expression(
            "expected `(` before while condition",
            "expected `)` after while condition",
        );
        self.parse_required_statement(STATEMENT_BOUNDARIES);
        self.complete(statement, JavaSyntaxKind::WhileStatement);
    }

    pub(super) fn parse_do_statement(&mut self) {
        let statement = self.start();
        self.expect(JavaSyntaxKind::DoKw, "expected `do`");
        self.parse_required_statement(DO_BODY_BOUNDARIES);
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
            self.parse_required_statement(STATEMENT_BOUNDARIES);
            self.complete(enhanced, JavaSyntaxKind::EnhancedForStatement);
        } else {
            let basic = self.start();
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
            self.parse_required_statement(STATEMENT_BOUNDARIES);
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
        self.parse_variable_declarator_id(true);
        let mut valid = true;
        if self.eat(JavaSyntaxKind::Assign) {
            self.error_expected("enhanced for variable must not have an initializer");
            valid = false;
            self.parse_variable_initializer_until(stops);
        }

        while self.at(JavaSyntaxKind::Comma) {
            self.error_unexpected("enhanced for statement must declare a single variable");
            valid = false;
            self.bump();
            if stops.contains(&self.current()) {
                break;
            }
            self.parse_variable_declarator_until(stops, true);
        }

        if valid {
            self.complete(variable, JavaSyntaxKind::EnhancedForVariable);
        } else {
            self.complete(variable, JavaSyntaxKind::Bogus);
        }
    }

    pub(super) fn parse_statement_expression_list(&mut self, stop: JavaSyntaxKind) {
        let list = self.start();
        self.parse_comma_separated(
            "expected statement expression",
            move |parser| parser.at(stop),
            move |parser| {
                parser.consume_statement_expression_until(&[JavaSyntaxKind::Comma, stop]);
            },
        );
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
        self.expect(JavaSyntaxKind::SynchronizedKw, "expected `synchronized`");
        self.parse_parenthesized_expression(
            "expected `(` before synchronized expression",
            "expected `)` after synchronized expression",
        );
        self.parse_required_block("expected synchronized body");
        self.complete(statement, JavaSyntaxKind::SynchronizedStatement);
    }

    pub(super) fn parse_try_statement(&mut self) {
        if self.nth(1) == JavaSyntaxKind::LParen {
            self.parse_try_with_resources_statement();
            return;
        }

        let statement = self.start();
        self.expect(JavaSyntaxKind::TryKw, "expected `try`");
        self.parse_required_block("expected try body");
        let mut saw_handler = false;
        let catches = self.start();
        let mut guard = self.loop_guard();
        while self.at(JavaSyntaxKind::CatchKw) && self.check(&mut guard) {
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
            self.error_expected("expected `catch` or `finally` after try block");
            self.complete(statement, JavaSyntaxKind::Bogus);
        }
    }

    pub(super) fn parse_try_with_resources_statement(&mut self) {
        let statement = self.start();
        self.expect(JavaSyntaxKind::TryKw, "expected `try`");
        let specification = self.start();
        self.expect(JavaSyntaxKind::LParen, "expected resource specification");
        let resources = self.start();
        // An empty resource list is represented by a resource with an empty
        // bogus value. A leading `;` reaches this with no resource just as `)`
        // does.
        if self.at(JavaSyntaxKind::RParen) || self.at(JavaSyntaxKind::Semicolon) {
            let resource = self.start();
            let value = self.start();
            self.error_expected("expected resource");
            self.complete(value, JavaSyntaxKind::Bogus);
            self.complete(resource, JavaSyntaxKind::Resource);
        }
        let mut guard = self.loop_guard();
        while !self.at(JavaSyntaxKind::RParen)
            && !self.at(JavaSyntaxKind::Semicolon)
            && self.check(&mut guard)
        {
            self.parse_resource();
            if !self.at(JavaSyntaxKind::Semicolon)
                || matches!(self.nth(1), JavaSyntaxKind::RParen | JavaSyntaxKind::Eof)
            {
                break;
            }
            self.bump();
        }
        self.complete(resources, JavaSyntaxKind::ResourceList);
        self.eat(JavaSyntaxKind::Semicolon);
        self.expect(JavaSyntaxKind::RParen, "expected `)` after resources");
        self.complete(specification, JavaSyntaxKind::ResourceSpecification);
        self.parse_required_block("expected try body");
        let catches = self.start();
        let mut guard = self.loop_guard();
        while self.at(JavaSyntaxKind::CatchKw) && self.check(&mut guard) {
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
        if self.starts_local_variable_declaration() {
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
        self.parse_variable_declarator_id(true);
        let mut valid = true;
        if self.eat(JavaSyntaxKind::Assign) {
            self.parse_variable_initializer_until(stops);
        } else {
            self.error_expected("expected resource initializer");
            valid = false;
        }

        while self.at(JavaSyntaxKind::Comma) {
            self.error_unexpected("resource declaration must declare a single variable");
            valid = false;
            self.bump();
            if stops.contains(&self.current()) {
                break;
            }
            self.parse_variable_declarator_until(stops, true);
        }

        if valid {
            self.complete(declaration, JavaSyntaxKind::ResourceVariableDeclaration);
        } else {
            self.complete(declaration, JavaSyntaxKind::Bogus);
        }
    }

    pub(super) fn parse_resource_variable_access_until(&mut self, stops: &[JavaSyntaxKind]) {
        if self.at_eof() || stops.contains(&self.current()) {
            let access = self.start();
            self.error(
                codes::INVALID_RESOURCE_VARIABLE_ACCESS,
                "expected resource variable declaration or variable access",
            );
            self.complete(access, JavaSyntaxKind::Bogus);
            return;
        }

        let expression = self.parse_expression();
        let expression_kind = JavaSyntaxKind::from_raw(expression.kind());
        let valid = matches!(
            expression_kind,
            JavaSyntaxKind::NameExpression | JavaSyntaxKind::FieldAccessExpression
        );

        if valid && (self.at_eof() || stops.contains(&self.current())) {
            let access = self.precede(expression);
            self.complete(access, JavaSyntaxKind::VariableAccess);
        } else {
            let access = self.precede(expression);
            self.error(
                codes::INVALID_RESOURCE_VARIABLE_ACCESS,
                if valid {
                    "unexpected token in resource variable access"
                } else {
                    "expected resource variable declaration or variable access"
                },
            );
            while !self.at_eof() && !stops.contains(&self.current()) {
                self.bump();
            }
            self.complete(access, JavaSyntaxKind::Bogus);
        }
    }

    pub(super) fn parse_catch_clause(&mut self) {
        let clause = self.start();
        self.expect(JavaSyntaxKind::CatchKw, "expected `catch`");
        self.expect(JavaSyntaxKind::LParen, "expected `(` after `catch`");
        let parameter = self.start();
        self.parse_variable_modifiers();
        let types = self.start();
        self.parse_class_union_type();
        self.complete(types, JavaSyntaxKind::CatchTypeList);
        self.expect_variable_identifier("expected catch parameter name", true);
        self.parse_array_dimensions();
        self.complete(parameter, JavaSyntaxKind::CatchParameter);
        self.expect(JavaSyntaxKind::RParen, "expected `)` after catch parameter");
        self.parse_required_block("expected catch body");
        self.complete(clause, JavaSyntaxKind::CatchClause);
    }

    pub(super) fn parse_finally_clause(&mut self) {
        let clause = self.start();
        self.expect(JavaSyntaxKind::FinallyKw, "expected `finally`");
        self.parse_required_block("expected finally body");
        self.complete(clause, JavaSyntaxKind::FinallyClause);
    }

    pub(super) fn parse_switch_statement(&mut self) {
        let statement = self.start();
        self.expect(JavaSyntaxKind::SwitchKw, "expected `switch`");
        self.parse_parenthesized_expression(
            "expected `(` before switch selector",
            "expected `)` after switch selector",
        );
        if self.at(JavaSyntaxKind::LBrace) {
            self.parse_switch_block();
        } else {
            self.error_expected("expected switch block");
        }
        self.complete(statement, JavaSyntaxKind::SwitchStatement);
    }

    pub(super) fn parse_switch_expression_fragment(&mut self) -> jolt_syntax::CompletedMarker {
        let expression = self.start();
        self.expect(JavaSyntaxKind::SwitchKw, "expected `switch`");
        self.parse_parenthesized_expression(
            "expected `(` before switch selector",
            "expected `)` after switch selector",
        );
        if self.at(JavaSyntaxKind::LBrace) {
            self.parse_switch_block();
        } else {
            self.error_expected("expected switch block");
        }
        self.complete(expression, JavaSyntaxKind::SwitchExpression)
    }

    pub(super) fn parse_switch_block(&mut self) {
        let block = self.start();
        self.expect(JavaSyntaxKind::LBrace, "expected switch block");
        let entries = self.start();
        let mut guard = self.loop_guard();
        while !self.at(JavaSyntaxKind::RBrace) && self.check(&mut guard) {
            if self.starts_switch_label() {
                if self.switch_label_is_rule() {
                    self.parse_switch_rule();
                } else {
                    self.parse_switch_block_statement_group_or_label();
                }
            } else {
                let entry = self.start();
                self.error_unexpected("expected switch label");
                self.parse_block_statement();
                self.complete(entry, JavaSyntaxKind::Bogus);
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
        let mut guard = self.loop_guard();
        while self.check(&mut guard) {
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
        let mut guard = self.loop_guard();
        while !self.at(JavaSyntaxKind::RBrace)
            && !self.starts_switch_label()
            && self.check(&mut guard)
        {
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
        let mut saw_any_item = false;
        let mut saw_case_item = false;
        let mut previous_was_pattern = false;
        let mut guard = self.loop_guard();
        while !matches!(
            self.current(),
            JavaSyntaxKind::Colon | JavaSyntaxKind::Arrow
        ) && self.check(&mut guard)
        {
            if self.at(JavaSyntaxKind::Comma) {
                if !saw_case_item {
                    self.error_expected("expected switch label item");
                }
                self.bump();
                saw_any_item = true;
                saw_case_item = false;
                previous_was_pattern = false;
            } else if self.at_contextual("when") && saw_case_item {
                break;
            } else if let Some(pattern_start) = self.pattern_start() {
                let case_pattern = self.start();
                self.parse_pattern_until(
                    pattern_start,
                    &[
                        JavaSyntaxKind::Comma,
                        JavaSyntaxKind::Colon,
                        JavaSyntaxKind::Arrow,
                    ],
                );
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
        if !saw_case_item && !saw_any_item {
            // A `case` needs at least one item; represent the absent one.
            let missing = self.start();
            self.error_expected("expected switch label item");
            self.complete(missing, JavaSyntaxKind::Bogus);
        } else if !saw_case_item {
            self.error_expected("expected switch label item");
        }
        self.complete(items, JavaSyntaxKind::SwitchLabelItemList);
        if self.at_contextual("when") {
            if previous_was_pattern {
                self.parse_guard();
            } else {
                let guard = self.start();
                self.error(
                    codes::INVALID_SWITCH_GUARD,
                    "switch guard requires a pattern",
                );
                self.parse_guard();
                self.complete(guard, JavaSyntaxKind::Bogus);
            }
        }
        self.complete(label, JavaSyntaxKind::SwitchLabel);
    }

    pub(super) fn parse_case_constant(&mut self) {
        let case_constant = self.start();
        self.parse_conditional_expression();
        let mut valid = true;
        while !self.at_eof()
            && !matches!(
                self.current(),
                JavaSyntaxKind::Comma | JavaSyntaxKind::Colon | JavaSyntaxKind::Arrow
            )
            && !self.at_contextual("when")
        {
            if valid {
                self.error_unexpected("unexpected token in case constant");
                valid = false;
            }
            self.bump();
        }
        if valid {
            self.complete(case_constant, JavaSyntaxKind::CaseConstant);
        } else {
            self.complete(case_constant, JavaSyntaxKind::Bogus);
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
