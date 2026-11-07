use super::{JavaSyntaxKind, Parser};
use jolt_syntax::ParseOutput;

impl Parser<'_> {
    pub(in crate::parser) fn parse_compilation_unit(mut self) -> ParseOutput {
        let unit = self.start();

        let items = self.start();
        let mut guard = self.loop_guard();
        while self.check(&mut guard) {
            if self.starts_package_declaration() {
                self.parse_package_declaration();
            } else if self.at(JavaSyntaxKind::ImportKw) {
                self.parse_import_declaration();
            } else if self.at(JavaSyntaxKind::Semicolon) {
                self.parse_empty_declaration();
            } else if self.starts_module_declaration() {
                self.parse_module_declaration();
            } else if self.starts_type_declaration() {
                self.parse_type_declaration();
            } else if self.starts_method_declaration() || self.starts_field_declaration() {
                self.parse_compact_member_declaration();
            } else {
                self.error_unexpected_top_level_token();
            }
        }
        self.complete(items, JavaSyntaxKind::CompilationUnitItemList);

        self.bump();
        self.complete(unit, JavaSyntaxKind::CompilationUnit);
        self.finish()
    }

    pub(super) fn parse_package_declaration(&mut self) {
        let package = self.start();
        self.parse_annotations();
        self.expect(JavaSyntaxKind::PackageKw, "expected `package`");
        self.consume_qualified_name_required();
        self.expect(
            JavaSyntaxKind::Semicolon,
            "expected `;` after package declaration",
        );
        self.complete(package, JavaSyntaxKind::PackageDeclaration);
    }

    pub(super) fn parse_import_declaration(&mut self) {
        let import = self.start();
        self.expect(JavaSyntaxKind::ImportKw, "expected `import`");

        if self.at_contextual("module") && self.nth_is_name_segment(1) {
            self.bump();
            self.consume_qualified_name_required();
            self.expect(
                JavaSyntaxKind::Semicolon,
                "expected `;` after module import",
            );
            self.complete(import, JavaSyntaxKind::ImportDeclaration);
            return;
        }

        self.eat(JavaSyntaxKind::StaticKw);
        if self.consume_qualified_name_required()
            && self.at(JavaSyntaxKind::Dot)
            && self.nth(1) == JavaSyntaxKind::Star
        {
            self.bump();
            self.bump();
        }
        if !self.at_eof() && !self.at(JavaSyntaxKind::Semicolon) {
            let suffix = self.start();
            self.error_unexpected("unexpected token in import declaration");
            while !self.at_eof()
                && !self.at(JavaSyntaxKind::Semicolon)
                && !self.at_program_item_recovery_boundary()
            {
                self.bump();
            }
            self.complete(suffix, JavaSyntaxKind::Bogus);
        }
        self.expect(
            JavaSyntaxKind::Semicolon,
            "expected `;` after import declaration",
        );
        self.complete(import, JavaSyntaxKind::ImportDeclaration);
    }

    pub(super) fn parse_module_declaration(&mut self) {
        let module = self.start();
        self.parse_annotations();
        self.eat_contextual("open");
        self.expect_contextual("module", "expected `module`");
        self.consume_qualified_name_required_until(&[
            JavaSyntaxKind::LBrace,
            JavaSyntaxKind::RBrace,
        ]);

        let has_open_brace = self.eat(JavaSyntaxKind::LBrace);
        if !has_open_brace {
            self.error_expected("expected module body");
        }
        let directives = self.start();
        let mut guard = self.loop_guard();
        if has_open_brace {
            while !self.at(JavaSyntaxKind::RBrace) && self.check(&mut guard) {
                if self.at_module_directive_start() {
                    self.parse_module_directive();
                } else {
                    self.error_unexpected_module_token();
                }
            }
        } else {
            while self.at_module_directive_start() && self.check(&mut guard) {
                self.parse_module_directive();
            }
        }
        self.complete(directives, JavaSyntaxKind::ModuleDirectiveList);
        self.expect(
            JavaSyntaxKind::RBrace,
            "expected `}` after module declaration",
        );

        self.complete(module, JavaSyntaxKind::ModuleDeclaration);
    }

    pub(super) fn parse_module_directive(&mut self) {
        let directive = self.start();
        let kind = match self.current_text() {
            "requires" => {
                self.parse_requires_directive_rest();
                JavaSyntaxKind::RequiresDirective
            }
            "exports" => {
                self.bump();
                self.consume_qualified_name_required_until(&[
                    JavaSyntaxKind::Semicolon,
                    JavaSyntaxKind::RBrace,
                ]);
                self.parse_optional_module_target_clause();
                self.expect(
                    JavaSyntaxKind::Semicolon,
                    "expected `;` after exports directive",
                );
                JavaSyntaxKind::ExportsDirective
            }
            "opens" => {
                self.bump();
                self.consume_qualified_name_required_until(&[
                    JavaSyntaxKind::Semicolon,
                    JavaSyntaxKind::RBrace,
                ]);
                self.parse_optional_module_target_clause();
                self.expect(
                    JavaSyntaxKind::Semicolon,
                    "expected `;` after opens directive",
                );
                JavaSyntaxKind::OpensDirective
            }
            "uses" => {
                self.bump();
                self.consume_qualified_name_required_until(&[
                    JavaSyntaxKind::Semicolon,
                    JavaSyntaxKind::RBrace,
                ]);
                self.expect(
                    JavaSyntaxKind::Semicolon,
                    "expected `;` after uses directive",
                );
                JavaSyntaxKind::UsesDirective
            }
            "provides" => {
                self.parse_provides_directive_rest();
                JavaSyntaxKind::ProvidesDirective
            }
            _ => {
                self.error_expected("expected module directive");
                self.recover_module_directive();
                JavaSyntaxKind::Bogus
            }
        };
        self.complete(directive, kind);
    }

    fn parse_requires_directive_rest(&mut self) {
        self.bump();
        let modifiers = self.start();
        while self.at(JavaSyntaxKind::StaticKw)
            || (self.at_contextual("transitive")
                && !matches!(self.nth(1), JavaSyntaxKind::Dot | JavaSyntaxKind::Semicolon))
        {
            self.bump();
        }
        self.complete(modifiers, JavaSyntaxKind::RequiresModifierList);
        self.consume_qualified_name_required_until(&[
            JavaSyntaxKind::Semicolon,
            JavaSyntaxKind::RBrace,
        ]);
        self.expect(
            JavaSyntaxKind::Semicolon,
            "expected `;` after requires directive",
        );
    }

    fn parse_provides_directive_rest(&mut self) {
        self.bump();
        self.consume_qualified_name_required_until(&[
            JavaSyntaxKind::Semicolon,
            JavaSyntaxKind::RBrace,
        ]);
        self.parse_module_implementation_clause();
        self.expect(
            JavaSyntaxKind::Semicolon,
            "expected `;` after provides directive",
        );
    }

    fn parse_module_implementation_clause(&mut self) {
        let implementation = self.start();
        self.expect_contextual("with", "expected `with` in provides directive");
        let implementations = self.start();
        self.parse_module_name_list_item();
        while self.eat(JavaSyntaxKind::Comma) {
            self.parse_module_name_list_item();
        }
        self.complete(implementations, JavaSyntaxKind::ModuleNameList);
        self.complete(implementation, JavaSyntaxKind::ModuleImplementationClause);
    }

    fn parse_module_name_list_item(&mut self) {
        self.consume_qualified_name_required_until(&[
            JavaSyntaxKind::Comma,
            JavaSyntaxKind::Semicolon,
            JavaSyntaxKind::RBrace,
        ]);
    }

    fn parse_optional_module_target_clause(&mut self) {
        if !self.at_contextual("to") && !self.nth_is_name_segment(0) {
            return;
        }

        let clause = self.start();
        self.expect_contextual("to", "expected `to` before module target list");
        let modules = self.start();
        self.parse_module_name_list_item();
        while self.eat(JavaSyntaxKind::Comma) {
            self.parse_module_name_list_item();
        }
        self.complete(modules, JavaSyntaxKind::ModuleNameList);
        self.complete(clause, JavaSyntaxKind::ModuleTargetClause);
    }
}
