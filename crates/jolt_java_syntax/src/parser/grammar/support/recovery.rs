// Contains recovery paths for malformed syntax after a parser error is reported.
use super::{JavaSyntaxKind, Parser};

impl Parser<'_> {
    pub(in crate::parser::grammar) fn error_unexpected_top_level_token(&mut self) {
        let error = self.start();
        self.unexpected_here("unexpected token at top level");
        self.recover_top_level();
        self.complete(error, JavaSyntaxKind::BogusTypeDeclaration);
    }

    pub(in crate::parser::grammar) fn error_unexpected_module_token(&mut self) {
        let module_directive = self.start();
        let error = self.start();
        self.unexpected_here("unexpected token in module declaration");
        self.recover_module_directive();
        self.complete(error, JavaSyntaxKind::BogusModuleDirective);
        self.complete(module_directive, JavaSyntaxKind::ModuleDirective);
    }

    pub(in crate::parser::grammar) fn recover_top_level(&mut self) {
        if self.at_eof() {
            return;
        }

        self.bump();
        while !self.at_eof()
            && !self.at(JavaSyntaxKind::Semicolon)
            && !self.at(JavaSyntaxKind::ImportKw)
            && !self.starts_module_declaration()
            && !self.starts_top_level_type_declaration()
        {
            self.bump();
        }

        self.eat(JavaSyntaxKind::Semicolon);
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
