use super::{JavaParserExt, JavaSyntaxKind, ParseEvents, Parser};
use jolt_syntax::NodeAnchor;

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
            } else if self.starts_type_declaration() {
                self.parse_type_declaration(JavaSyntaxKind::BogusCompilationUnitItem);
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
        self.expect_required(
            JavaSyntaxKind::PackageKw,
            "expected `package`",
            owner,
            crate::shape::package_declaration::Slot::package_keyword as u16,
        );
        self.consume_qualified_name_required(
            owner,
            crate::shape::package_declaration::Slot::name as u16,
        );
        self.expect_required(
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
        self.expect_required(
            JavaSyntaxKind::ImportKw,
            "expected `import`",
            owner,
            crate::shape::import_declaration::Slot::import_keyword as u16,
        );

        if self.at_contextual("module") && self.nth_is_name_segment(1) {
            self.bump();
            self.consume_qualified_name_required(
                owner,
                crate::shape::import_declaration::Slot::name as u16,
            );
            self.expect_required(
                JavaSyntaxKind::Semicolon,
                "expected `;` after module import",
                owner,
                crate::shape::import_declaration::Slot::semicolon as u16,
            );
            self.complete(import, JavaSyntaxKind::ImportDeclaration);
            return;
        }

        self.eat(JavaSyntaxKind::StaticKw);
        if self.consume_qualified_name_required(
            owner,
            crate::shape::import_declaration::Slot::name as u16,
        ) && self.at(JavaSyntaxKind::Dot)
            && self.nth_kind(1) == JavaSyntaxKind::Star
        {
            self.bump();
            self.bump();
        }
        if !self.at_eof() && !self.at(JavaSyntaxKind::Semicolon) {
            let suffix = self.start();
            let diagnostic = self.pending_unexpected("unexpected token in import declaration");
            while !self.at_eof()
                && !self.at(JavaSyntaxKind::Semicolon)
                && !self.at_program_item_recovery_boundary()
            {
                self.bump();
            }
            self.complete_recovery(suffix, JavaSyntaxKind::BogusImportSuffix, [diagnostic]);
        }
        self.expect_required(
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
        self.expect_contextual_required(
            "module",
            "expected `module`",
            owner,
            crate::shape::module_declaration::Slot::module_keyword as u16,
        );
        self.consume_qualified_name_required_until(
            owner,
            crate::shape::module_declaration::Slot::name as u16,
            &[JavaSyntaxKind::LBrace, JavaSyntaxKind::RBrace],
            Self::module_directive_boundary_at,
            false,
        );

        let has_open_brace = self.eat(JavaSyntaxKind::LBrace);
        if !has_open_brace {
            let diagnostic = self.pending_expected("expected module body");
            self.missing_required_slot(
                owner,
                crate::shape::module_declaration::Slot::open_brace as u16,
                [diagnostic],
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
        self.expect_required(
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
        let (kind, recovery) = match self.current_text() {
            Some("requires") => {
                self.parse_requires_directive_rest(owner);
                (JavaSyntaxKind::RequiresDirective, None)
            }
            Some("exports") => {
                self.bump();
                self.consume_qualified_name_required_until(
                    owner,
                    crate::shape::exports_directive::Slot::package as u16,
                    &[JavaSyntaxKind::Semicolon, JavaSyntaxKind::RBrace],
                    Self::module_target_boundary_at,
                    true,
                );
                if !Self::module_directive_boundary_at(self, 0) {
                    self.parse_optional_module_target_clause();
                }
                self.expect_required(
                    JavaSyntaxKind::Semicolon,
                    "expected `;` after exports directive",
                    owner,
                    crate::shape::exports_directive::Slot::semicolon as u16,
                );
                (JavaSyntaxKind::ExportsDirective, None)
            }
            Some("opens") => {
                self.bump();
                self.consume_qualified_name_required_until(
                    owner,
                    crate::shape::opens_directive::Slot::package as u16,
                    &[JavaSyntaxKind::Semicolon, JavaSyntaxKind::RBrace],
                    Self::module_target_boundary_at,
                    true,
                );
                if !Self::module_directive_boundary_at(self, 0) {
                    self.parse_optional_module_target_clause();
                }
                self.expect_required(
                    JavaSyntaxKind::Semicolon,
                    "expected `;` after opens directive",
                    owner,
                    crate::shape::opens_directive::Slot::semicolon as u16,
                );
                (JavaSyntaxKind::OpensDirective, None)
            }
            Some("uses") => {
                self.bump();
                self.consume_qualified_name_required_until(
                    owner,
                    crate::shape::uses_directive::Slot::service as u16,
                    &[JavaSyntaxKind::Semicolon, JavaSyntaxKind::RBrace],
                    Self::module_directive_boundary_at,
                    false,
                );
                self.expect_required(
                    JavaSyntaxKind::Semicolon,
                    "expected `;` after uses directive",
                    owner,
                    crate::shape::uses_directive::Slot::semicolon as u16,
                );
                (JavaSyntaxKind::UsesDirective, None)
            }
            Some("provides") => {
                self.parse_provides_directive_rest(owner);
                (JavaSyntaxKind::ProvidesDirective, None)
            }
            _ => {
                let diagnostic = self.pending_expected("expected module directive");
                self.recover_module_directive();
                (JavaSyntaxKind::BogusModuleDirective, Some(diagnostic))
            }
        };
        if let Some(diagnostic) = recovery {
            self.complete_recovery(directive, kind, [diagnostic]);
        } else {
            self.complete(directive, kind);
        }
    }

    fn parse_requires_directive_rest(&mut self, owner: NodeAnchor) {
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
        self.consume_qualified_name_required_until(
            owner,
            crate::shape::requires_directive::Slot::module as u16,
            &[JavaSyntaxKind::Semicolon, JavaSyntaxKind::RBrace],
            Self::module_directive_boundary_at,
            false,
        );
        self.expect_required(
            JavaSyntaxKind::Semicolon,
            "expected `;` after requires directive",
            owner,
            crate::shape::requires_directive::Slot::semicolon as u16,
        );
    }

    fn parse_provides_directive_rest(&mut self, owner: NodeAnchor) {
        self.bump();
        self.consume_qualified_name_required_until(
            owner,
            crate::shape::provides_directive::Slot::service as u16,
            &[JavaSyntaxKind::Semicolon, JavaSyntaxKind::RBrace],
            Self::module_implementation_boundary_at,
            true,
        );
        if Self::module_directive_boundary_at(self, 0) {
            let diagnostic = self.pending_expected("expected `with` in provides directive");
            self.missing_required_slot(
                owner,
                crate::shape::provides_directive::Slot::implementation as u16,
                [diagnostic],
            );
        } else {
            self.parse_module_implementation_clause();
        }
        self.expect_required(
            JavaSyntaxKind::Semicolon,
            "expected `;` after provides directive",
            owner,
            crate::shape::provides_directive::Slot::semicolon as u16,
        );
    }

    fn parse_module_implementation_clause(&mut self) {
        let implementation = self.start();
        let implementation_owner = implementation.anchor();
        self.expect_contextual_required(
            "with",
            "expected `with` in provides directive",
            implementation_owner,
            crate::shape::module_implementation_clause::Slot::with_keyword as u16,
        );
        let implementations = self.start();
        let list_owner = implementations.anchor();
        let mut item_slot = 0;
        self.parse_module_name_list_item(list_owner, item_slot);
        while self.eat(JavaSyntaxKind::Comma) {
            item_slot += 2;
            self.parse_module_name_list_item(list_owner, item_slot);
        }
        self.complete(implementations, JavaSyntaxKind::ModuleNameList);
        self.complete(implementation, JavaSyntaxKind::ModuleImplementationClause);
    }

    fn parse_module_name_list_item(&mut self, owner: NodeAnchor, slot: u16) {
        if Self::module_directive_boundary_at(self, 0) {
            let diagnostic = self.pending_expected("expected identifier");
            self.missing_required_slot(owner, slot, [diagnostic]);
            return;
        }
        self.consume_qualified_name_required_until(
            owner,
            slot,
            &[
                JavaSyntaxKind::Comma,
                JavaSyntaxKind::Semicolon,
                JavaSyntaxKind::RBrace,
            ],
            Self::module_directive_boundary_at,
            false,
        );
    }

    fn module_directive_boundary_at(&mut self, offset: usize) -> bool {
        let index = self.position() + offset;
        if !matches!(
            self.text_at(index),
            Some("requires" | "exports" | "opens" | "uses" | "provides")
        ) {
            return false;
        }
        self.is_name_segment_at(index + 1) || self.kind_at(index + 1) == JavaSyntaxKind::StaticKw
    }

    fn module_target_boundary_at(&mut self, offset: usize) -> bool {
        if Self::module_directive_boundary_at(self, offset) {
            return true;
        }
        let index = self.position() + offset;
        self.kind_at(index) == JavaSyntaxKind::Identifier
            && self.text_at(index) == Some("to")
            && self.kind_at(index + 1) == JavaSyntaxKind::Identifier
    }

    fn module_implementation_boundary_at(&mut self, offset: usize) -> bool {
        if Self::module_directive_boundary_at(self, offset) {
            return true;
        }
        let index = self.position() + offset;
        self.kind_at(index) == JavaSyntaxKind::Identifier
            && self.text_at(index) == Some("with")
            && self.kind_at(index + 1) == JavaSyntaxKind::Identifier
    }

    fn parse_optional_module_target_clause(&mut self) {
        if !self.at_contextual("to") && !self.nth_is_name_segment(0) {
            return;
        }

        let clause = self.start();
        let owner = clause.anchor();
        self.expect_contextual_required(
            "to",
            "expected `to` before module target list",
            owner,
            crate::shape::module_target_clause::Slot::to_keyword as u16,
        );
        let modules = self.start();
        let list_owner = modules.anchor();
        let mut item_slot = 0;
        self.parse_module_name_list_item(list_owner, item_slot);
        while self.eat(JavaSyntaxKind::Comma) {
            item_slot += 2;
            self.parse_module_name_list_item(list_owner, item_slot);
        }
        self.complete(modules, JavaSyntaxKind::ModuleNameList);
        self.complete(clause, JavaSyntaxKind::ModuleTargetClause);
    }
}
