// Contains recovery paths for malformed syntax after a parser error is reported.
use super::{JavaSyntaxKind, Parser};

impl Parser<'_> {
    pub(in crate::parser::grammar) fn error_unexpected_top_level_token(&mut self) {
        let error = self.start();
        let diagnostic = self.pending_unexpected("unexpected token at top level");
        self.recover_top_level();
        self.complete_recovery(
            error,
            JavaSyntaxKind::BogusCompilationUnitItem,
            [diagnostic],
        );
    }

    pub(in crate::parser::grammar) fn error_unexpected_module_token(&mut self) {
        let error = self.start();
        let diagnostic = self.pending_unexpected("unexpected token in module declaration");
        self.recover_module_directive();
        self.complete_recovery(error, JavaSyntaxKind::BogusModuleDirective, [diagnostic]);
    }

    pub(in crate::parser::grammar) fn recover_top_level(&mut self) {
        if self.at_eof() {
            return;
        }

        self.bump();
        while !self.at_eof() && !self.at_program_item_recovery_boundary() {
            self.bump();
        }
    }

    pub(in crate::parser::grammar) fn at_program_item_recovery_boundary(&mut self) -> bool {
        self.starts_package_declaration()
            || self.at(JavaSyntaxKind::ImportKw)
            || self.starts_module_declaration()
            || self.starts_type_declaration()
    }

    pub(in crate::parser::grammar) fn recover_module_directive(&mut self) {
        if self.at_eof() || self.at(JavaSyntaxKind::RBrace) {
            return;
        }

        self.bump();
        while !self.at_eof()
            && !self.at(JavaSyntaxKind::Semicolon)
            && !self.at(JavaSyntaxKind::RBrace)
        {
            self.bump();
        }

        self.eat(JavaSyntaxKind::Semicolon);
    }
}
