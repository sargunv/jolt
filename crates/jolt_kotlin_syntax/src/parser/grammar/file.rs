use crate::KotlinSyntaxKind as K;
use jolt_syntax::UnresolvedDiagnosticOwner;

use super::{ParseEvents, Parser};

impl Parser<'_> {
    pub(crate) fn parse_kotlin_file(mut self) -> ParseEvents {
        let file = self.start();
        self.parse_file_annotations();

        let items = self.start();
        let mut saw_package = false;
        let mut saw_imports = false;
        let mut saw_body = false;
        while !self.at_eof() {
            self.eat_optional_separators();
            if self.at_eof() {
                break;
            }
            if self.at(K::PackageKw) {
                self.parse_package_header(saw_package || saw_imports || saw_body);
                saw_package = true;
                continue;
            }
            if self.at_soft_keyword("import") {
                self.parse_import_list(saw_body);
                saw_imports = true;
                continue;
            }
            if self.at(K::RBrace) {
                let bogus = self.start();
                let diagnostic = self.unexpected_here("unexpected closing brace at top level");
                self.own_diagnostic(diagnostic, UnresolvedDiagnosticOwner::node(bogus.anchor()));
                self.bump();
                self.complete(bogus, K::BogusKotlinFileItem);
                continue;
            }
            saw_body = true;
            let before = self.position();
            self.parse_declaration_or_statement();
            self.ensure_progress(before, "expected declaration or statement");
        }
        self.complete(items, K::KotlinFileItemList);

        self.expect(K::Eof, "expected end of file");
        self.complete(file, K::KotlinFile);
        self.finish()
    }

    fn parse_file_annotations(&mut self) {
        let annotations = self.start();
        while self.at(K::At) || self.at(K::Hash) {
            let before = self.position();
            self.parse_annotation();
            self.ensure_progress(before, "expected file annotation");
        }
        self.complete(annotations, K::AnnotationList);
    }

    fn parse_package_header(&mut self, misplaced: bool) {
        if !self.at(K::PackageKw) {
            return;
        }

        let marker = self.start();
        if misplaced {
            self.unexpected_owned_node(
                "unexpected package header after file header",
                marker.anchor(),
            );
        }
        self.bump();
        self.parse_file_qualified_name();
        if !self.at_semicolon_boundary() {
            self.parse_directive_suffix(
                K::BogusPackageSuffix,
                "unexpected token in package header",
            );
        }
        let terminators = self.start();
        self.eat_optional_separators();
        self.complete(terminators, K::TerminatorList);
        self.complete(marker, K::PackageHeader);
    }

    fn parse_import_list(&mut self, misplaced: bool) {
        let directives = self.start();
        if misplaced {
            self.unexpected_owned_node("unexpected import after file item", directives.anchor());
        }
        debug_assert!(self.at_soft_keyword("import"));
        while self.at_soft_keyword("import") {
            let before = self.position();
            self.parse_import_directive();
            self.ensure_progress(before, "expected import directive");
        }
        self.complete(directives, K::ImportDirectiveList);
    }

    fn parse_import_directive(&mut self) {
        let marker = self.start();
        debug_assert!(self.at_soft_keyword("import"));
        self.bump();
        self.parse_file_qualified_name();
        if self.at(K::Star) || self.at(K::Dot) && self.nth_kind(1) == K::Star {
            let suffix = self.start();
            let owner = suffix.anchor();
            self.expect_owned(
                K::Dot,
                "expected `.` before import star",
                owner,
                crate::shape::import_on_demand_suffix::Slot::dot as u16,
            );
            self.expect_owned(
                K::Star,
                "expected `*` after import dot",
                owner,
                crate::shape::import_on_demand_suffix::Slot::star as u16,
            );
            self.complete(suffix, K::ImportOnDemandSuffix);
        }
        if self.at(K::AsKw) {
            let alias = self.start();
            self.bump();
            self.parse_file_name();
            self.complete(alias, K::ImportAlias);
        }
        if !self.at_semicolon_boundary() {
            self.parse_directive_suffix(
                K::BogusImportSuffix,
                "unexpected token in import directive",
            );
        }
        let terminators = self.start();
        self.eat_optional_separators();
        self.complete(terminators, K::TerminatorList);
        self.complete(marker, K::ImportDirective);
    }

    fn parse_directive_suffix(&mut self, kind: K, message: &str) {
        let suffix = self.start();
        let diagnostic = self.unexpected_here(message);
        self.own_diagnostic(diagnostic, UnresolvedDiagnosticOwner::node(suffix.anchor()));
        loop {
            self.bump();
            if self.at_semicolon_boundary() {
                break;
            }
        }
        self.complete(suffix, kind);
    }
}
