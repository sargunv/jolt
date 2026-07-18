use super::{ParseEvents, Parser};
use crate::KotlinSyntaxKind as K;

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
                let diagnostic = self.pending_unexpected("unexpected closing brace at top level");
                self.bump();
                self.complete_recovery(bogus, K::BogusKotlinFileItem, [diagnostic]);
                continue;
            }
            saw_body = true;
            let before = self.position();
            self.parse_declaration_or_statement();
            if self.position() == before {
                let bogus = self.start();
                let diagnostic = self.pending_unexpected("expected declaration or statement");
                if !self.at_eof() {
                    self.bump();
                }
                self.complete_recovery(bogus, K::BogusKotlinFileItem, [diagnostic]);
            }
        }
        self.complete(items, K::KotlinFileItemList);

        self.eat_asserted(K::Eof);
        self.complete(file, K::KotlinFile);
        self.finish()
    }

    fn parse_file_annotations(&mut self) {
        let annotations = self.start();
        while self.at(K::At) || self.at(K::Hash) {
            let before = self.position();
            self.parse_annotation();
            debug_assert!(self.position() > before);
        }
        self.complete(annotations, K::AnnotationList);
    }

    fn parse_package_header(&mut self, misplaced: bool) {
        if !self.at(K::PackageKw) {
            return;
        }

        let marker = self.start();
        let diagnostic = misplaced
            .then(|| self.pending_unexpected("unexpected package header after file header"));
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
        if let Some(diagnostic) = diagnostic {
            self.complete_recovery(marker, K::PackageHeader, [diagnostic]);
        } else {
            self.complete(marker, K::PackageHeader);
        }
    }

    fn parse_import_list(&mut self, misplaced: bool) {
        let directives = self.start();
        let diagnostic =
            misplaced.then(|| self.pending_unexpected("unexpected import after file item"));
        debug_assert!(self.at_soft_keyword("import"));
        while self.at_soft_keyword("import") {
            let before = self.position();
            self.parse_import_directive();
            debug_assert!(self.position() > before);
        }
        if let Some(diagnostic) = diagnostic {
            self.complete_recovery(directives, K::ImportDirectiveList, [diagnostic]);
        } else {
            self.complete(directives, K::ImportDirectiveList);
        }
    }

    fn parse_import_directive(&mut self) {
        let marker = self.start();
        debug_assert!(self.at_soft_keyword("import"));
        self.bump();
        self.parse_file_qualified_name();
        if self.at(K::Star) || self.at(K::Dot) && self.nth_kind(1) == K::Star {
            let suffix = self.start();
            let owner = suffix.anchor();
            if !self.eat(K::Dot) {
                let diagnostic = self.pending_expected("expected `.` before import star");
                self.missing_required_slot(
                    owner,
                    crate::shape::import_on_demand_suffix::Slot::dot as u16,
                    [diagnostic],
                );
            }
            if !self.eat(K::Star) {
                let diagnostic = self.pending_expected("expected `*` after import dot");
                self.missing_required_slot(
                    owner,
                    crate::shape::import_on_demand_suffix::Slot::star as u16,
                    [diagnostic],
                );
            }
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
        let diagnostic = self.pending_unexpected(message);
        loop {
            self.bump();
            if self.at_semicolon_boundary() {
                break;
            }
        }
        self.complete_recovery(suffix, kind, [diagnostic]);
    }
}
