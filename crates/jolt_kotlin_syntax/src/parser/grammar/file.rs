use crate::KotlinSyntaxKind as K;

use super::{ParseEvents, Parser};

impl Parser<'_> {
    pub(crate) fn parse_kotlin_file(mut self) -> ParseEvents {
        let file = self.start();
        self.parse_file_annotations();
        self.parse_package_header();
        self.parse_import_list();

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

        self.expect(K::Eof, "expected end of file");
        self.complete(file, K::KotlinFile);
        self.finish()
    }

    fn parse_file_annotations(&mut self) {
        while self.at(K::At) || self.at(K::Hash) {
            self.parse_annotation();
        }
    }

    fn parse_package_header(&mut self) {
        if !self.at(K::PackageKw) {
            return;
        }

        let marker = self.start();
        self.bump();
        self.parse_qualified_name();
        self.eat_optional_separators();
        self.complete(marker, K::PackageHeader);
    }

    fn parse_import_list(&mut self) {
        let marker = self.start();
        while self.at_soft_keyword("import") {
            self.parse_import_directive();
        }
        self.complete(marker, K::ImportList);
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
        self.eat_optional_separators();
        self.complete(marker, K::ImportDirective);
    }

    pub(super) fn parse_comma_separated_until(&mut self, close: K, item_kind: K) {
        let mut expect_item = true;
        while !matches!(self.current_kind(), K::Eof) && !self.at(close) {
            let before = self.position();
            if self.eat(K::Comma) {
                if expect_item && !matches!(self.current_kind(), K::Eof) && !self.at(close) {
                    let missing = self.start();
                    self.expected_here("expected list item");
                    self.complete(missing, K::ErrorNode);
                }
                expect_item = true;
                continue;
            }
            let item = self.start();
            self.parse_expression_until(&[K::Comma, close]);
            self.complete(item, item_kind);
            expect_item = false;
            if self.position() == before {
                self.unexpected_here("expected list item");
                self.bump();
            }
        }
    }
}
