use super::{JavaParserExt, JavaSyntaxKind, ParseEvents, Parser};

impl Parser<'_> {
    pub(in crate::parser) fn parse_compilation_unit(mut self) -> ParseEvents {
        let unit = self.start();

        if self.starts_package_declaration() {
            self.parse_package_declaration();
        }

        let imports = self.start();
        while self.at(JavaSyntaxKind::ImportKw) {
            self.parse_import_declaration();
        }
        self.complete(imports, JavaSyntaxKind::ImportDeclarationList);

        if self.starts_module_declaration() {
            self.parse_module_declaration();
        }

        let declarations = self.start();
        while !self.at_eof() {
            if self.at(JavaSyntaxKind::Semicolon) {
                self.parse_empty_declaration();
            } else if self.starts_module_declaration() {
                // A module after a type declaration is invalid, but it must not
                // make the entire top-level declaration list malformed. Keep
                // the parsed module inside the list's narrow bogus item.
                let bogus = self.start();
                self.parse_module_declaration();
                self.complete(bogus, JavaSyntaxKind::BogusTypeDeclaration);
            } else if self.starts_top_level_type_declaration() {
                self.parse_type_declaration();
            } else if self.starts_misspelled_non_sealed_type_declaration() {
                self.error_unexpected_top_level_token();
            } else if self.starts_compact_member_declaration() {
                self.parse_compact_member_declaration();
            } else {
                self.error_unexpected_top_level_token();
            }
        }
        self.complete(declarations, JavaSyntaxKind::CompilationUnitDeclarationList);

        self.bump();
        self.complete(unit, JavaSyntaxKind::CompilationUnit);
        self.finish()
    }

    pub(super) fn parse_package_declaration(&mut self) {
        let package = self.start();
        self.parse_annotations();
        self.expect(JavaSyntaxKind::PackageKw, "expected `package`");
        self.consume_qualified_name();
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
            self.consume_qualified_name();
            self.expect(
                JavaSyntaxKind::Semicolon,
                "expected `;` after module import",
            );
            self.complete(import, JavaSyntaxKind::ImportDeclaration);
            return;
        }

        self.eat(JavaSyntaxKind::StaticKw);
        if self.consume_qualified_name()
            && self.at(JavaSyntaxKind::Dot)
            && self.nth_kind(1) == JavaSyntaxKind::Star
        {
            self.bump();
            self.bump();
        }
        if !self.at_eof() && !self.at(JavaSyntaxKind::Semicolon) {
            let error = self.start();
            self.unexpected_here("unexpected token in import declaration");
            while !self.at_eof() && !self.at(JavaSyntaxKind::Semicolon) {
                self.bump();
            }
            self.complete(error, JavaSyntaxKind::ErrorNode);
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
        self.consume_qualified_name();

        if self.eat(JavaSyntaxKind::LBrace) {
            let directives = self.start();
            while !self.at_eof() && !self.at(JavaSyntaxKind::RBrace) {
                if self.at_module_directive_start() {
                    self.parse_module_directive();
                } else {
                    self.error_unexpected_module_token();
                }
            }
            self.complete(directives, JavaSyntaxKind::ModuleDirectiveList);
            self.expect(
                JavaSyntaxKind::RBrace,
                "expected `}` after module declaration",
            );
        } else {
            self.expected_here("expected module body");
        }

        self.complete(module, JavaSyntaxKind::ModuleDeclaration);
    }

    pub(super) fn parse_module_directive(&mut self) {
        let module_directive = self.start();
        let directive = self.start();
        let kind = match self.current_text() {
            Some("requires") => {
                self.bump();
                let modifiers = self.start();
                while self.at(JavaSyntaxKind::StaticKw)
                    || (self.at_contextual("transitive")
                        && !matches!(
                            self.nth_kind(1),
                            JavaSyntaxKind::Dot | JavaSyntaxKind::Semicolon
                        ))
                {
                    self.bump();
                }
                self.complete(modifiers, JavaSyntaxKind::RequiresModifierList);
                self.consume_qualified_name();
                self.expect(
                    JavaSyntaxKind::Semicolon,
                    "expected `;` after requires directive",
                );
                JavaSyntaxKind::RequiresDirective
            }
            Some("exports") => {
                self.bump();
                self.consume_qualified_name();
                self.parse_optional_module_list_after_to();
                self.expect(
                    JavaSyntaxKind::Semicolon,
                    "expected `;` after exports directive",
                );
                JavaSyntaxKind::ExportsDirective
            }
            Some("opens") => {
                self.bump();
                self.consume_qualified_name();
                self.parse_optional_module_list_after_to();
                self.expect(
                    JavaSyntaxKind::Semicolon,
                    "expected `;` after opens directive",
                );
                JavaSyntaxKind::OpensDirective
            }
            Some("uses") => {
                self.bump();
                self.consume_qualified_name();
                self.expect(
                    JavaSyntaxKind::Semicolon,
                    "expected `;` after uses directive",
                );
                JavaSyntaxKind::UsesDirective
            }
            Some("provides") => {
                self.bump();
                self.consume_qualified_name();
                self.expect_contextual("with", "expected `with` in provides directive");
                let implementations = self.start();
                self.consume_qualified_name();
                while self.eat(JavaSyntaxKind::Comma) {
                    self.consume_qualified_name();
                }
                self.complete(implementations, JavaSyntaxKind::NameList);
                self.expect(
                    JavaSyntaxKind::Semicolon,
                    "expected `;` after provides directive",
                );
                JavaSyntaxKind::ProvidesDirective
            }
            _ => {
                self.expected_here("expected module directive");
                self.recover_module_directive();
                JavaSyntaxKind::ModuleDirective
            }
        };
        self.complete(directive, kind);
        self.complete(module_directive, JavaSyntaxKind::ModuleDirective);
    }

    pub(super) fn parse_optional_module_list_after_to(&mut self) {
        if !self.eat_contextual("to") {
            let modules = self.start();
            self.complete(modules, JavaSyntaxKind::NameList);
            return;
        }

        let modules = self.start();
        self.consume_qualified_name();
        while self.eat(JavaSyntaxKind::Comma) {
            self.consume_qualified_name();
        }
        self.complete(modules, JavaSyntaxKind::NameList);
    }
}
