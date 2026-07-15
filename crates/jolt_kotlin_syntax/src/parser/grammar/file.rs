use crate::KotlinSyntaxKind as K;

use super::{ParseEvents, Parser};

impl Parser<'_> {
    pub(crate) fn parse_kotlin_file(mut self) -> ParseEvents {
        let file = self.start();
        self.parse_file_annotations();
        self.parse_package_header();
        self.parse_import_list();

        let items = self.start();
        while !self.at_eof() {
            if self.eat_optional_separators() && self.at(K::RBrace) {
                self.unexpected_here("unexpected closing brace at top level");
                self.bump();
                continue;
            }
            if self.at_eof() {
                break;
            }
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

    fn parse_package_header(&mut self) {
        if !self.at(K::PackageKw) {
            return;
        }

        let marker = self.start();
        self.bump();
        self.parse_qualified_name();
        let terminators = self.start();
        self.eat_optional_separators();
        self.complete(terminators, K::TerminatorList);
        self.complete(marker, K::PackageHeader);
    }

    fn parse_import_list(&mut self) {
        let directives = self.start();
        while self.at_soft_keyword("import") {
            let before = self.position();
            self.parse_import_directive();
            self.ensure_progress(before, "expected import directive");
        }
        self.complete(directives, K::ImportDirectiveList);
    }

    fn parse_import_directive(&mut self) {
        let marker = self.start();
        self.expect_soft_keyword("import", "expected import");
        self.parse_qualified_name();
        if self.at(K::Star) {
            self.bump();
        } else if self.eat(K::Dot) {
            self.expect(K::Star, "expected import name or star");
        }
        if self.at(K::AsKw) {
            let alias = self.start();
            self.bump();
            self.parse_name();
            self.complete(alias, K::ImportAlias);
        }
        let terminators = self.start();
        self.eat_optional_separators();
        self.complete(terminators, K::TerminatorList);
        self.complete(marker, K::ImportDirective);
    }
}
