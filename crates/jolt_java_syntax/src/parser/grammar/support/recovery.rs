// Contains recovery paths for malformed syntax after a parser error is reported.
use super::{JavaSyntaxKind, Parser};
use jolt_syntax::UnresolvedDiagnosticOwner;

impl Parser<'_> {
    pub(in crate::parser::grammar) fn error_unexpected_top_level_token(&mut self) {
        let error = self.start();
        let diagnostic = self.unexpected_here("unexpected token at top level");
        self.own_diagnostic(diagnostic, UnresolvedDiagnosticOwner::node(error.anchor()));
        self.recover_top_level();
        self.complete(error, JavaSyntaxKind::BogusCompilationUnitItem);
    }

    pub(in crate::parser::grammar) fn error_unexpected_module_token(&mut self) {
        let error = self.start();
        let diagnostic = self.unexpected_here("unexpected token in module declaration");
        self.own_diagnostic(diagnostic, UnresolvedDiagnosticOwner::node(error.anchor()));
        self.recover_module_directive();
        self.complete(error, JavaSyntaxKind::BogusModuleDirective);
    }

    pub(in crate::parser::grammar) fn recover_top_level(&mut self) {
        if self.at_eof() {
            return;
        }

        self.bump();
        while !self.at_eof()
            && !self.at(JavaSyntaxKind::Semicolon)
            && !self.at_program_item_recovery_boundary()
        {
            self.bump();
        }

        self.eat(JavaSyntaxKind::Semicolon);
    }

    pub(in crate::parser::grammar) fn at_program_item_recovery_boundary(&mut self) -> bool {
        self.starts_package_declaration()
            || self.at(JavaSyntaxKind::ImportKw)
            || self.starts_module_declaration()
            || self.starts_top_level_type_declaration()
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
