use super::{JavaParserExt, JavaSyntaxKind, Parser};
use jolt_syntax::{NodeAnchor, PendingDiagnostic};

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
    fn pending_invalid_resource_variable_access(&mut self, message: &str) -> PendingDiagnostic {
        self.pending_error(
            crate::parser::JavaParseDiagnosticCode::InvalidResourceVariableAccess.id(),
            message,
        )
    }

    fn pending_invalid_switch_guard(&mut self, message: &str) -> PendingDiagnostic {
        self.pending_error(
            crate::parser::JavaParseDiagnosticCode::InvalidSwitchGuard.id(),
            message,
        )
    }

    fn parse_required_statement(&mut self, owner: NodeAnchor, slot: u16, stops: &[JavaSyntaxKind]) {
        if self.at_eof() || stops.contains(&self.current_kind()) {
            let diagnostic = self.pending_expected("expected statement");
            self.missing_required_slot(owner, slot, [diagnostic]);
        } else {
            self.parse_statement();
        }
    }

    fn parse_required_block(&mut self, owner: NodeAnchor, slot: u16, message: &'static str) {
        if self.at(JavaSyntaxKind::LBrace) {
            self.parse_block();
        } else {
            let diagnostic = self.pending_expected(message);
            self.missing_required_slot(owner, slot, [diagnostic]);
        }
    }

    pub(super) fn parse_block(&mut self) {
        let block = self.start();
        let owner = block.anchor();
        self.expect_required(
            JavaSyntaxKind::LBrace,
            "expected block",
            owner,
            crate::shape::block::Slot::open_brace as u16,
        );

        let statements = self.start();
        while !self.at_eof() && !self.at(JavaSyntaxKind::RBrace) {
            self.parse_block_statement();
        }
        self.complete(statements, JavaSyntaxKind::BlockStatementList);

        self.expect_required(
            JavaSyntaxKind::RBrace,
            "expected `}` after block",
            owner,
            crate::shape::block::Slot::close_brace as u16,
        );
        self.complete(block, JavaSyntaxKind::Block);
    }

    pub(super) fn parse_block_statement(&mut self) {
        let block_statement = self.start();

        if self.starts_type_declaration() {
            self.parse_local_class_or_interface_declaration();
        } else if self.starts_local_variable_declaration() {
            self.parse_local_variable_declaration_statement(
                block_statement.anchor(),
                crate::shape::block_statement::Slot::local_declaration_semicolon as u16,
            );
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

    pub(super) fn parse_local_variable_declaration_statement(
        &mut self,
        owner: NodeAnchor,
        semicolon_slot: u16,
    ) {
        self.parse_local_variable_declaration_until(&[JavaSyntaxKind::Semicolon]);
        self.expect_required(
            JavaSyntaxKind::Semicolon,
            "expected `;` after local variable declaration",
            owner,
            semicolon_slot,
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
        if self
            .with_syntax_nesting(Self::parse_statement_inner)
            .is_none()
        {
            self.parse_excessive_statement();
        }
    }

    fn parse_statement_inner(&mut self) {
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

    fn parse_excessive_statement(&mut self) {
        let statement = self.start();
        let diagnostic = self.pending_excessive_syntax_nesting();
        self.consume_until_enclosing_brace();
        self.complete_recovery(statement, JavaSyntaxKind::BogusStatement, [diagnostic]);
    }

    pub(super) fn parse_empty_statement(&mut self) {
        let statement = self.start();
        self.expect_required(
            JavaSyntaxKind::Semicolon,
            "expected `;`",
            statement.anchor(),
            crate::shape::empty_statement::Slot::semicolon as u16,
        );
        self.complete(statement, JavaSyntaxKind::EmptyStatement);
    }

    pub(super) fn parse_labeled_statement(&mut self) {
        let statement = self.start();
        let owner = statement.anchor();
        self.expect_variable_identifier_required(
            "expected statement label",
            owner,
            crate::shape::labeled_statement::Slot::label as u16,
            false,
        );
        self.expect_required(
            JavaSyntaxKind::Colon,
            "expected `:` after label",
            owner,
            crate::shape::labeled_statement::Slot::colon as u16,
        );
        self.parse_required_statement(
            owner,
            crate::shape::labeled_statement::Slot::body as u16,
            STATEMENT_BOUNDARIES,
        );
        self.complete(statement, JavaSyntaxKind::LabeledStatement);
    }

    pub(super) fn parse_expression_statement(&mut self) {
        let statement = self.start();
        let owner = statement.anchor();
        self.consume_statement_expression_until(&[JavaSyntaxKind::Semicolon]);
        self.expect_required(
            JavaSyntaxKind::Semicolon,
            "expected `;` after expression statement",
            owner,
            crate::shape::expression_statement::Slot::semicolon as u16,
        );
        self.complete(statement, JavaSyntaxKind::ExpressionStatement);
    }

    pub(super) fn parse_if_statement(&mut self) {
        let statement = self.start();
        let owner = statement.anchor();
        self.expect_required(
            JavaSyntaxKind::IfKw,
            "expected `if`",
            owner,
            crate::shape::if_statement::Slot::if_keyword as u16,
        );
        self.parse_parenthesized_expression(
            owner,
            crate::shape::if_statement::Slot::open_paren as u16,
            crate::shape::if_statement::Slot::close_paren as u16,
            "expected `(` before if condition",
            "expected `)` after if condition",
        );
        self.parse_required_statement(
            owner,
            crate::shape::if_statement::Slot::then_branch as u16,
            STATEMENT_BOUNDARIES,
        );
        let else_recovery = if self.eat(JavaSyntaxKind::ElseKw) {
            if self.at_eof() || STATEMENT_BOUNDARIES.contains(&self.current_kind()) {
                Some(self.pending_expected("expected statement"))
            } else {
                self.parse_statement();
                None
            }
        } else {
            None
        };
        if let Some(diagnostic) = else_recovery {
            self.complete_recovery(statement, JavaSyntaxKind::IfStatement, [diagnostic]);
        } else {
            self.complete(statement, JavaSyntaxKind::IfStatement);
        }
    }

    pub(super) fn parse_assert_statement(&mut self) {
        let statement = self.start();
        let owner = statement.anchor();
        self.expect_required(
            JavaSyntaxKind::AssertKw,
            "expected `assert`",
            owner,
            crate::shape::assert_statement::Slot::assert_keyword as u16,
        );
        self.parse_expression_until(&[JavaSyntaxKind::Colon, JavaSyntaxKind::Semicolon]);
        if self.eat(JavaSyntaxKind::Colon) {
            self.parse_expression_until(&[JavaSyntaxKind::Semicolon]);
        }
        self.expect_required(
            JavaSyntaxKind::Semicolon,
            "expected `;` after assert statement",
            owner,
            crate::shape::assert_statement::Slot::semicolon as u16,
        );
        self.complete(statement, JavaSyntaxKind::AssertStatement);
    }

    pub(super) fn parse_while_statement(&mut self) {
        let statement = self.start();
        let owner = statement.anchor();
        self.expect_required(
            JavaSyntaxKind::WhileKw,
            "expected `while`",
            owner,
            crate::shape::while_statement::Slot::while_keyword as u16,
        );
        self.parse_parenthesized_expression(
            owner,
            crate::shape::while_statement::Slot::open_paren as u16,
            crate::shape::while_statement::Slot::close_paren as u16,
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
        self.expect_required(
            JavaSyntaxKind::DoKw,
            "expected `do`",
            owner,
            crate::shape::do_statement::Slot::do_keyword as u16,
        );
        self.parse_required_statement(
            owner,
            crate::shape::do_statement::Slot::body as u16,
            DO_BODY_BOUNDARIES,
        );
        self.expect_required(
            JavaSyntaxKind::WhileKw,
            "expected `while` after do body",
            owner,
            crate::shape::do_statement::Slot::while_keyword as u16,
        );
        self.parse_parenthesized_expression(
            owner,
            crate::shape::do_statement::Slot::open_paren as u16,
            crate::shape::do_statement::Slot::close_paren as u16,
            "expected `(` before do condition",
            "expected `)` after do condition",
        );
        self.expect_required(
            JavaSyntaxKind::Semicolon,
            "expected `;` after do statement",
            owner,
            crate::shape::do_statement::Slot::semicolon as u16,
        );
        self.complete(statement, JavaSyntaxKind::DoStatement);
    }

    pub(super) fn parse_for_statement(&mut self) {
        let statement = self.start();
        if self.for_header_has_top_level_colon() {
            let enhanced = self.start();
            let owner = enhanced.anchor();
            self.expect_required(
                JavaSyntaxKind::ForKw,
                "expected `for`",
                owner,
                crate::shape::enhanced_for_statement::Slot::for_keyword as u16,
            );
            self.expect_required(
                JavaSyntaxKind::LParen,
                "expected `(` after `for`",
                owner,
                crate::shape::enhanced_for_statement::Slot::open_paren as u16,
            );
            self.parse_enhanced_for_variable_declaration_until(&[JavaSyntaxKind::Colon]);
            self.expect_required(
                JavaSyntaxKind::Colon,
                "expected `:` in enhanced for statement",
                owner,
                crate::shape::enhanced_for_statement::Slot::colon as u16,
            );
            self.parse_expression_until(&[JavaSyntaxKind::RParen]);
            self.expect_required(
                JavaSyntaxKind::RParen,
                "expected `)` after enhanced for header",
                owner,
                crate::shape::enhanced_for_statement::Slot::close_paren as u16,
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
            self.expect_required(
                JavaSyntaxKind::ForKw,
                "expected `for`",
                owner,
                crate::shape::basic_for_statement::Slot::for_keyword as u16,
            );
            self.expect_required(
                JavaSyntaxKind::LParen,
                "expected `(` after `for`",
                owner,
                crate::shape::basic_for_statement::Slot::open_paren as u16,
            );
            if !self.at(JavaSyntaxKind::Semicolon) {
                let initializer = self.start();
                if self.starts_local_variable_declaration() {
                    self.parse_local_variable_declaration_until(&[JavaSyntaxKind::Semicolon]);
                } else {
                    self.parse_statement_expression_list(JavaSyntaxKind::Semicolon);
                }
                self.complete(initializer, JavaSyntaxKind::ForInitializer);
            }
            self.expect_required(
                JavaSyntaxKind::Semicolon,
                "expected `;` in for header",
                owner,
                crate::shape::basic_for_statement::Slot::first_semicolon as u16,
            );
            if !self.at(JavaSyntaxKind::Semicolon) {
                self.parse_expression_until(&[JavaSyntaxKind::Semicolon]);
            }
            self.expect_required(
                JavaSyntaxKind::Semicolon,
                "expected `;` in for header",
                owner,
                crate::shape::basic_for_statement::Slot::second_semicolon as u16,
            );
            if !self.at(JavaSyntaxKind::RParen) {
                let update = self.start();
                self.parse_statement_expression_list(JavaSyntaxKind::RParen);
                self.complete(update, JavaSyntaxKind::ForUpdate);
            }
            self.expect_required(
                JavaSyntaxKind::RParen,
                "expected `)` after for header",
                owner,
                crate::shape::basic_for_statement::Slot::close_paren as u16,
            );
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
        let mut diagnostics = Vec::new();
        if self.eat(JavaSyntaxKind::Assign) {
            diagnostics
                .push(self.pending_expected("enhanced for variable must not have an initializer"));
            self.parse_variable_initializer_until(stops);
        }

        while self.at(JavaSyntaxKind::Comma) {
            diagnostics.push(
                self.pending_unexpected("enhanced for statement must declare a single variable"),
            );
            self.bump();
            if stops.contains(&self.current_kind()) {
                break;
            }
            self.parse_variable_declarator_until(stops, true);
        }

        if diagnostics.is_empty() {
            self.complete(variable, JavaSyntaxKind::EnhancedForVariable);
        } else {
            self.complete_recovery(
                variable,
                JavaSyntaxKind::BogusEnhancedForVariable,
                diagnostics,
            );
        }
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
            crate::shape::break_statement::Slot::break_keyword as u16,
            crate::shape::break_statement::Slot::semicolon as u16,
            "expected `break`",
            "expected `;` after break statement",
        );
    }

    pub(super) fn parse_continue_statement(&mut self) {
        self.parse_jump_statement(
            JavaSyntaxKind::ContinueKw,
            JavaSyntaxKind::ContinueStatement,
            crate::shape::continue_statement::Slot::continue_keyword as u16,
            crate::shape::continue_statement::Slot::semicolon as u16,
            "expected `continue`",
            "expected `;` after continue statement",
        );
    }

    pub(super) fn parse_jump_statement(
        &mut self,
        keyword: JavaSyntaxKind,
        kind: JavaSyntaxKind,
        keyword_slot: u16,
        semicolon_slot: u16,
        keyword_message: &str,
        semicolon_message: &str,
    ) {
        let statement = self.start();
        let owner = statement.anchor();
        self.expect_required(keyword, keyword_message, owner, keyword_slot);
        if self.at_name_segment() {
            self.bump();
        }
        self.expect_required(
            JavaSyntaxKind::Semicolon,
            semicolon_message,
            owner,
            semicolon_slot,
        );
        self.complete(statement, kind);
    }

    pub(super) fn parse_yield_statement(&mut self) {
        let statement = self.start();
        let owner = statement.anchor();
        self.expect_contextual_required(
            "yield",
            "expected `yield`",
            owner,
            crate::shape::yield_statement::Slot::yield_keyword as u16,
        );
        self.parse_expression_until(&[JavaSyntaxKind::Semicolon]);
        self.expect_required(
            JavaSyntaxKind::Semicolon,
            "expected `;` after yield statement",
            owner,
            crate::shape::yield_statement::Slot::semicolon as u16,
        );
        self.complete(statement, JavaSyntaxKind::YieldStatement);
    }

    pub(super) fn parse_return_statement(&mut self) {
        let statement = self.start();
        let owner = statement.anchor();
        self.expect_required(
            JavaSyntaxKind::ReturnKw,
            "expected `return`",
            owner,
            crate::shape::return_statement::Slot::return_keyword as u16,
        );
        if !self.at(JavaSyntaxKind::Semicolon) {
            self.parse_expression_until(&[JavaSyntaxKind::Semicolon]);
        }
        self.expect_required(
            JavaSyntaxKind::Semicolon,
            "expected `;` after return statement",
            owner,
            crate::shape::return_statement::Slot::semicolon as u16,
        );
        self.complete(statement, JavaSyntaxKind::ReturnStatement);
    }

    pub(super) fn parse_throw_statement(&mut self) {
        let statement = self.start();
        let owner = statement.anchor();
        self.expect_required(
            JavaSyntaxKind::ThrowKw,
            "expected `throw`",
            owner,
            crate::shape::throw_statement::Slot::throw_keyword as u16,
        );
        self.parse_expression_until(&[JavaSyntaxKind::Semicolon]);
        self.expect_required(
            JavaSyntaxKind::Semicolon,
            "expected `;` after throw statement",
            owner,
            crate::shape::throw_statement::Slot::semicolon as u16,
        );
        self.complete(statement, JavaSyntaxKind::ThrowStatement);
    }

    pub(super) fn parse_synchronized_statement(&mut self) {
        let statement = self.start();
        let owner = statement.anchor();
        self.expect_required(
            JavaSyntaxKind::SynchronizedKw,
            "expected `synchronized`",
            owner,
            crate::shape::synchronized_statement::Slot::synchronized_keyword as u16,
        );
        self.parse_parenthesized_expression(
            owner,
            crate::shape::synchronized_statement::Slot::open_paren as u16,
            crate::shape::synchronized_statement::Slot::close_paren as u16,
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
        self.expect_required(
            JavaSyntaxKind::TryKw,
            "expected `try`",
            owner,
            crate::shape::try_statement::Slot::try_keyword as u16,
        );
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
            let diagnostic = self.pending_expected("expected `catch` or `finally` after try block");
            self.complete_recovery(statement, JavaSyntaxKind::BogusStatement, [diagnostic]);
        }
    }

    pub(super) fn parse_try_with_resources_statement(&mut self) {
        let statement = self.start();
        let owner = statement.anchor();
        self.expect_required(
            JavaSyntaxKind::TryKw,
            "expected `try`",
            owner,
            crate::shape::try_with_resources_statement::Slot::try_keyword as u16,
        );
        let specification = self.start();
        let specification_owner = specification.anchor();
        self.expect_required(
            JavaSyntaxKind::LParen,
            "expected resource specification",
            specification_owner,
            crate::shape::resource_specification::Slot::open_paren as u16,
        );
        let resources = self.start();
        if self.at(JavaSyntaxKind::RParen) {
            let resource = self.start();
            let value = self.start();
            let diagnostic = self.pending_expected("expected resource");
            self.complete_recovery(value, JavaSyntaxKind::BogusResourceValue, [diagnostic]);
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
        self.expect_required(
            JavaSyntaxKind::RParen,
            "expected `)` after resources",
            specification_owner,
            crate::shape::resource_specification::Slot::close_paren as u16,
        );
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
        self.parse_variable_declarator_id(
            true,
            declaration.anchor(),
            crate::shape::resource_variable_declaration::Slot::name as u16,
        );
        let mut diagnostics = Vec::new();
        if self.eat(JavaSyntaxKind::Assign) {
            self.parse_variable_initializer_until(stops);
        } else {
            diagnostics.push(self.pending_expected("expected resource initializer"));
        }

        while self.at(JavaSyntaxKind::Comma) {
            diagnostics.push(
                self.pending_unexpected("resource declaration must declare a single variable"),
            );
            self.bump();
            if stops.contains(&self.current_kind()) {
                break;
            }
            self.parse_variable_declarator_until(stops, true);
        }

        if diagnostics.is_empty() {
            self.complete(declaration, JavaSyntaxKind::ResourceVariableDeclaration);
        } else {
            self.complete_recovery(declaration, JavaSyntaxKind::BogusResourceValue, diagnostics);
        }
    }

    pub(super) fn parse_resource_variable_access_until(&mut self, stops: &[JavaSyntaxKind]) {
        if self.at_eof() || stops.contains(&self.current_kind()) {
            let access = self.start();
            let diagnostic = self.pending_invalid_resource_variable_access(
                "expected resource variable declaration or variable access",
            );
            self.complete_recovery(access, JavaSyntaxKind::BogusResourceValue, [diagnostic]);
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
            let diagnostic = self.pending_invalid_resource_variable_access(if valid {
                "unexpected token in resource variable access"
            } else {
                "expected resource variable declaration or variable access"
            });
            while !self.at_eof() && !stops.contains(&self.current_kind()) {
                self.bump();
            }
            self.complete_recovery(access, JavaSyntaxKind::BogusResourceValue, [diagnostic]);
        }
    }

    pub(super) fn parse_catch_clause(&mut self) {
        let clause = self.start();
        let owner = clause.anchor();
        self.expect_required(
            JavaSyntaxKind::CatchKw,
            "expected `catch`",
            owner,
            crate::shape::catch_clause::Slot::catch_keyword as u16,
        );
        self.expect_required(
            JavaSyntaxKind::LParen,
            "expected `(` after `catch`",
            owner,
            crate::shape::catch_clause::Slot::open_paren as u16,
        );
        let parameter = self.start();
        let parameter_owner = parameter.anchor();
        self.parse_variable_modifiers();
        let types = self.start();
        self.parse_class_union_type();
        self.complete(types, JavaSyntaxKind::CatchTypeList);
        self.expect_variable_identifier_required(
            "expected catch parameter name",
            parameter_owner,
            crate::shape::catch_parameter::Slot::name as u16,
            true,
        );
        self.parse_array_dimensions();
        self.complete(parameter, JavaSyntaxKind::CatchParameter);
        self.expect_required(
            JavaSyntaxKind::RParen,
            "expected `)` after catch parameter",
            owner,
            crate::shape::catch_clause::Slot::close_paren as u16,
        );
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
        self.expect_required(
            JavaSyntaxKind::FinallyKw,
            "expected `finally`",
            owner,
            crate::shape::finally_clause::Slot::finally_keyword as u16,
        );
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
        self.expect_required(
            JavaSyntaxKind::SwitchKw,
            "expected `switch`",
            owner,
            crate::shape::switch_statement::Slot::switch_keyword as u16,
        );
        self.parse_parenthesized_expression(
            owner,
            crate::shape::switch_statement::Slot::open_paren as u16,
            crate::shape::switch_statement::Slot::close_paren as u16,
            "expected `(` before switch selector",
            "expected `)` after switch selector",
        );
        if self.at(JavaSyntaxKind::LBrace) {
            self.parse_switch_block();
        } else {
            let diagnostic = self.pending_expected("expected switch block");
            self.missing_required_slot(
                owner,
                crate::shape::switch_statement::Slot::body as u16,
                [diagnostic],
            );
        }
        self.complete(statement, JavaSyntaxKind::SwitchStatement);
    }

    pub(super) fn parse_switch_expression_fragment(&mut self) -> jolt_syntax::CompletedMarker {
        let expression = self.start();
        let owner = expression.anchor();
        self.expect_required(
            JavaSyntaxKind::SwitchKw,
            "expected `switch`",
            owner,
            crate::shape::switch_expression::Slot::switch_keyword as u16,
        );
        self.parse_parenthesized_expression(
            owner,
            crate::shape::switch_expression::Slot::open_paren as u16,
            crate::shape::switch_expression::Slot::close_paren as u16,
            "expected `(` before switch selector",
            "expected `)` after switch selector",
        );
        if self.at(JavaSyntaxKind::LBrace) {
            self.parse_switch_block();
        } else {
            let diagnostic = self.pending_expected("expected switch block");
            self.missing_required_slot(
                owner,
                crate::shape::switch_expression::Slot::body as u16,
                [diagnostic],
            );
        }
        self.complete(expression, JavaSyntaxKind::SwitchExpression)
    }

    pub(super) fn parse_switch_block(&mut self) {
        let block = self.start();
        let owner = block.anchor();
        self.expect_required(
            JavaSyntaxKind::LBrace,
            "expected switch block",
            owner,
            crate::shape::switch_block::Slot::open_brace as u16,
        );
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
                let diagnostic = self.pending_unexpected("expected switch label");
                self.parse_block_statement();
                self.complete_recovery(entry, JavaSyntaxKind::BogusSwitchEntry, [diagnostic]);
            }
        }
        self.complete(entries, JavaSyntaxKind::SwitchEntryList);
        self.expect_required(
            JavaSyntaxKind::RBrace,
            "expected `}` after switch block",
            owner,
            crate::shape::switch_block::Slot::close_brace as u16,
        );
        self.complete(block, JavaSyntaxKind::SwitchBlock);
    }

    pub(super) fn parse_switch_rule(&mut self) {
        let rule = self.start();
        let owner = rule.anchor();
        self.parse_switch_label();
        self.expect_required(
            JavaSyntaxKind::Arrow,
            "expected `->` after switch label",
            owner,
            crate::shape::switch_rule::Slot::arrow as u16,
        );
        if self.at(JavaSyntaxKind::LBrace) {
            self.parse_block();
        } else if self.at(JavaSyntaxKind::ThrowKw) {
            self.parse_throw_statement();
        } else {
            self.parse_expression_until(&[JavaSyntaxKind::Semicolon]);
            self.expect_required(
                JavaSyntaxKind::Semicolon,
                "expected `;` after switch rule",
                owner,
                crate::shape::switch_rule::Slot::semicolon as u16,
            );
        }
        self.complete(rule, JavaSyntaxKind::SwitchRule);
    }

    pub(super) fn parse_switch_block_statement_group_or_label(&mut self) {
        let group = self.start();
        let labels = self.start();
        let labels_owner = labels.anchor();
        let mut colon_slot = 1;
        loop {
            self.parse_switch_label();
            self.expect_required(
                JavaSyntaxKind::Colon,
                "expected `:` after switch label",
                labels_owner,
                colon_slot,
            );
            colon_slot += 2;
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
        let owner = label.anchor();
        if self.eat(JavaSyntaxKind::DefaultKw) {
            let items = self.start();
            self.complete(items, JavaSyntaxKind::SwitchLabelItemList);
            self.complete(label, JavaSyntaxKind::SwitchLabel);
            return;
        }

        self.expect_required(
            JavaSyntaxKind::CaseKw,
            "expected `case`",
            owner,
            crate::shape::switch_label::Slot::keyword as u16,
        );
        let items = self.start();
        let items_owner = items.anchor();
        let mut item_slot = 0;
        let mut saw_case_item = false;
        let mut previous_was_pattern = false;
        while !self.at_eof()
            && !matches!(
                self.current_kind(),
                JavaSyntaxKind::Colon | JavaSyntaxKind::Arrow
            )
        {
            if self.at(JavaSyntaxKind::Comma) {
                if !saw_case_item {
                    let diagnostic = self.pending_expected("expected switch label item");
                    self.missing_required_slot(items_owner, item_slot, [diagnostic]);
                }
                self.bump();
                item_slot += 2;
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
        let empty_items_recovery = if !saw_case_item && item_slot == 0 {
            Some(self.pending_expected("expected switch label item"))
        } else if !saw_case_item {
            let diagnostic = self.pending_expected("expected switch label item");
            self.missing_required_slot(items_owner, item_slot, [diagnostic]);
            None
        } else {
            None
        };
        if let Some(diagnostic) = empty_items_recovery {
            self.complete_recovery(items, JavaSyntaxKind::SwitchLabelItemList, [diagnostic]);
        } else {
            self.complete(items, JavaSyntaxKind::SwitchLabelItemList);
        }
        if self.at_contextual("when") {
            if previous_was_pattern {
                self.parse_guard();
            } else {
                let guard = self.start();
                let diagnostic =
                    self.pending_invalid_switch_guard("switch guard requires a pattern");
                self.parse_guard();
                self.complete_recovery(guard, JavaSyntaxKind::BogusSwitchGuard, [diagnostic]);
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
                diagnostic = Some(self.pending_unexpected("unexpected token in case constant"));
            }
            self.bump();
        }
        if let Some(diagnostic) = diagnostic {
            self.complete_recovery(
                case_constant,
                JavaSyntaxKind::BogusSwitchLabelItem,
                [diagnostic],
            );
        } else {
            self.complete(case_constant, JavaSyntaxKind::CaseConstant);
        }
    }

    pub(super) fn parse_guard(&mut self) {
        let guard = self.start();
        let owner = guard.anchor();
        self.expect_contextual_required(
            "when",
            "expected `when`",
            owner,
            crate::shape::guard::Slot::when_keyword as u16,
        );
        if self.starts_parenthesized_lambda_expression() {
            self.parse_parenthesized_expression(
                owner,
                crate::shape::guard::Slot::open_paren as u16,
                crate::shape::guard::Slot::close_paren as u16,
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
        owner: NodeAnchor,
        open_slot: u16,
        close_slot: u16,
        open_message: &'static str,
        close_message: &'static str,
    ) {
        self.expect_required(JavaSyntaxKind::LParen, open_message, owner, open_slot);
        self.parse_expression_until(&[JavaSyntaxKind::RParen]);
        self.expect_required(JavaSyntaxKind::RParen, close_message, owner, close_slot);
    }
}
