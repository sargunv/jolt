use super::{JavaParserExt, JavaSyntaxKind, ParseEvents, Parser};
use jolt_syntax::{NodeAnchor, UnresolvedDiagnosticOwner};

impl Parser<'_> {
    pub(in crate::parser) fn parse_compilation_unit(mut self) -> ParseEvents {
        let unit = self.start();

        let items = self.start();
        while !self.at_eof() {
            if self.starts_package_declaration() {
                self.parse_package_declaration();
            } else if self.at(JavaSyntaxKind::ImportKw) {
                self.parse_import_declaration();
            } else if self.at(JavaSyntaxKind::Semicolon) {
                self.parse_empty_declaration();
            } else if self.starts_module_declaration() {
                self.parse_module_declaration();
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
        self.complete(items, JavaSyntaxKind::CompilationUnitItemList);

        self.bump();
        self.complete(unit, JavaSyntaxKind::CompilationUnit);
        self.finish()
    }

    pub(super) fn parse_package_declaration(&mut self) {
        let package = self.start();
        let owner = package.anchor();
        self.parse_annotations();
        self.expect_owned(
            JavaSyntaxKind::PackageKw,
            "expected `package`",
            owner,
            crate::shape::package_declaration::Slot::package_keyword as u16,
        );
        self.consume_qualified_name_owned(UnresolvedDiagnosticOwner::missing_slot(
            owner,
            crate::shape::package_declaration::Slot::name as u16,
        ));
        self.expect_owned(
            JavaSyntaxKind::Semicolon,
            "expected `;` after package declaration",
            owner,
            crate::shape::package_declaration::Slot::semicolon as u16,
        );
        self.complete(package, JavaSyntaxKind::PackageDeclaration);
    }

    pub(super) fn parse_import_declaration(&mut self) {
        let import = self.start();
        let owner = import.anchor();
        self.expect_owned(
            JavaSyntaxKind::ImportKw,
            "expected `import`",
            owner,
            crate::shape::import_declaration::Slot::import_keyword as u16,
        );

        if self.at_contextual("module") && self.nth_is_name_segment(1) {
            self.bump();
            self.consume_qualified_name_owned(UnresolvedDiagnosticOwner::missing_slot(
                owner,
                crate::shape::import_declaration::Slot::name as u16,
            ));
            self.expect_owned(
                JavaSyntaxKind::Semicolon,
                "expected `;` after module import",
                owner,
                crate::shape::import_declaration::Slot::semicolon as u16,
            );
            self.complete(import, JavaSyntaxKind::ImportDeclaration);
            return;
        }

        self.eat(JavaSyntaxKind::StaticKw);
        if self.consume_qualified_name_owned(UnresolvedDiagnosticOwner::missing_slot(
            owner,
            crate::shape::import_declaration::Slot::name as u16,
        )) && self.at(JavaSyntaxKind::Dot)
            && self.nth_kind(1) == JavaSyntaxKind::Star
        {
            self.bump();
            self.bump();
        }
        if !self.at_eof() && !self.at(JavaSyntaxKind::Semicolon) {
            let suffix = self.start();
            let diagnostic = self.unexpected_here("unexpected token in import declaration");
            self.own_diagnostic(diagnostic, UnresolvedDiagnosticOwner::node(suffix.anchor()));
            while !self.at_eof()
                && !self.at(JavaSyntaxKind::Semicolon)
                && !self.at_program_item_recovery_boundary()
            {
                self.bump();
            }
            self.complete(suffix, JavaSyntaxKind::BogusImportSuffix);
        }
        self.expect_owned(
            JavaSyntaxKind::Semicolon,
            "expected `;` after import declaration",
            owner,
            crate::shape::import_declaration::Slot::semicolon as u16,
        );
        self.complete(import, JavaSyntaxKind::ImportDeclaration);
    }

    pub(super) fn parse_module_declaration(&mut self) {
        let module = self.start();
        let owner = module.anchor();
        self.parse_annotations();
        self.eat_contextual("open");
        self.expect_contextual_owned(
            "module",
            "expected `module`",
            owner,
            crate::shape::module_declaration::Slot::module_keyword as u16,
        );
        self.consume_qualified_name_owned(UnresolvedDiagnosticOwner::missing_slot(
            owner,
            crate::shape::module_declaration::Slot::name as u16,
        ));

        let has_open_brace = self.eat(JavaSyntaxKind::LBrace);
        if !has_open_brace {
            let diagnostic = self.expected_here("expected module body");
            self.own_diagnostic(
                diagnostic,
                UnresolvedDiagnosticOwner::missing_slot(
                    owner,
                    crate::shape::module_declaration::Slot::open_brace as u16,
                ),
            );
        }
        let directives = self.start();
        if has_open_brace {
            while !self.at_eof() && !self.at(JavaSyntaxKind::RBrace) {
                if self.at_module_directive_start() {
                    self.parse_module_directive();
                } else {
                    self.error_unexpected_module_token();
                }
            }
        } else {
            while !self.at_eof() && self.at_module_directive_start() {
                self.parse_module_directive();
            }
        }
        self.complete(directives, JavaSyntaxKind::ModuleDirectiveList);
        self.expect_owned(
            JavaSyntaxKind::RBrace,
            "expected `}` after module declaration",
            owner,
            crate::shape::module_declaration::Slot::close_brace as u16,
        );

        self.complete(module, JavaSyntaxKind::ModuleDeclaration);
    }

    pub(super) fn parse_module_directive(&mut self) {
        let directive = self.start();
        let owner = directive.anchor();
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
                self.consume_qualified_name_owned(UnresolvedDiagnosticOwner::missing_slot(
                    owner,
                    crate::shape::requires_directive::Slot::module as u16,
                ));
                self.expect_owned(
                    JavaSyntaxKind::Semicolon,
                    "expected `;` after requires directive",
                    owner,
                    crate::shape::requires_directive::Slot::semicolon as u16,
                );
                JavaSyntaxKind::RequiresDirective
            }
            Some("exports") => {
                self.bump();
                self.consume_qualified_name_owned(UnresolvedDiagnosticOwner::missing_slot(
                    owner,
                    crate::shape::exports_directive::Slot::package as u16,
                ));
                self.parse_optional_module_target_clause();
                self.expect_owned(
                    JavaSyntaxKind::Semicolon,
                    "expected `;` after exports directive",
                    owner,
                    crate::shape::exports_directive::Slot::semicolon as u16,
                );
                JavaSyntaxKind::ExportsDirective
            }
            Some("opens") => {
                self.bump();
                self.consume_qualified_name_owned(UnresolvedDiagnosticOwner::missing_slot(
                    owner,
                    crate::shape::opens_directive::Slot::package as u16,
                ));
                self.parse_optional_module_target_clause();
                self.expect_owned(
                    JavaSyntaxKind::Semicolon,
                    "expected `;` after opens directive",
                    owner,
                    crate::shape::opens_directive::Slot::semicolon as u16,
                );
                JavaSyntaxKind::OpensDirective
            }
            Some("uses") => {
                self.bump();
                self.consume_qualified_name_owned(UnresolvedDiagnosticOwner::missing_slot(
                    owner,
                    crate::shape::uses_directive::Slot::service as u16,
                ));
                self.expect_owned(
                    JavaSyntaxKind::Semicolon,
                    "expected `;` after uses directive",
                    owner,
                    crate::shape::uses_directive::Slot::semicolon as u16,
                );
                JavaSyntaxKind::UsesDirective
            }
            Some("provides") => {
                self.parse_provides_directive_rest(owner);
                JavaSyntaxKind::ProvidesDirective
            }
            _ => {
                let diagnostic = self.expected_here("expected module directive");
                self.own_diagnostic(diagnostic, UnresolvedDiagnosticOwner::node(owner));
                self.recover_module_directive();
                JavaSyntaxKind::BogusModuleDirective
            }
        };
        self.complete(directive, kind);
    }

    fn parse_provides_directive_rest(&mut self, owner: NodeAnchor) {
        self.bump();
        self.consume_qualified_name_owned(UnresolvedDiagnosticOwner::missing_slot(
            owner,
            crate::shape::provides_directive::Slot::service as u16,
        ));
        let implementation = self.start();
        let implementation_owner = implementation.anchor();
        self.expect_contextual_owned(
            "with",
            "expected `with` in provides directive",
            implementation_owner,
            crate::shape::module_implementation_clause::Slot::with_keyword as u16,
        );
        let implementations = self.start();
        let list_owner = UnresolvedDiagnosticOwner::node(implementations.anchor());
        self.consume_qualified_name_owned(list_owner);
        while self.eat(JavaSyntaxKind::Comma) {
            self.consume_qualified_name_owned(list_owner);
        }
        self.complete(implementations, JavaSyntaxKind::ModuleNameList);
        self.complete(implementation, JavaSyntaxKind::ModuleImplementationClause);
        self.expect_owned(
            JavaSyntaxKind::Semicolon,
            "expected `;` after provides directive",
            owner,
            crate::shape::provides_directive::Slot::semicolon as u16,
        );
    }

    fn parse_optional_module_target_clause(&mut self) {
        if !self.at_contextual("to") && !self.nth_is_name_segment(0) {
            return;
        }

        let clause = self.start();
        let owner = clause.anchor();
        self.expect_contextual_owned(
            "to",
            "expected `to` before module target list",
            owner,
            crate::shape::module_target_clause::Slot::to_keyword as u16,
        );
        let modules = self.start();
        let modules_owner = UnresolvedDiagnosticOwner::node(modules.anchor());
        self.consume_qualified_name_owned(modules_owner);
        while self.eat(JavaSyntaxKind::Comma) {
            self.consume_qualified_name_owned(modules_owner);
        }
        self.complete(modules, JavaSyntaxKind::ModuleNameList);
        self.complete(clause, JavaSyntaxKind::ModuleTargetClause);
    }

    fn expect_owned(&mut self, kind: JavaSyntaxKind, message: &str, node: NodeAnchor, slot: u16) {
        if !self.eat(kind) {
            let diagnostic = self.expected_here(message);
            self.own_diagnostic(
                diagnostic,
                UnresolvedDiagnosticOwner::missing_slot(node, slot),
            );
        }
    }

    fn expect_contextual_owned(&mut self, text: &str, message: &str, node: NodeAnchor, slot: u16) {
        if !self.eat_contextual(text) {
            let diagnostic = self.expected_here(message);
            self.own_diagnostic(
                diagnostic,
                UnresolvedDiagnosticOwner::missing_slot(node, slot),
            );
        }
    }
}
