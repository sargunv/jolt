// Handles Java identifier roles that depend on parser context.
use super::{JavaParserExt, JavaSyntaxKind, Parser};

impl Parser<'_> {
    pub(in crate::parser::grammar) fn expect_type_identifier(&mut self, message: &str) {
        if self.at_type_identifier() {
            self.bump();
        } else if self.at_name_segment() {
            let error = self.start();
            self.restricted_type_identifier_here(message);
            self.bump();
            self.complete(error, JavaSyntaxKind::ErrorNode);
        } else {
            self.expected_here(message);
        }
    }

    pub(in crate::parser::grammar) fn expect_method_identifier(&mut self, message: &str) {
        if self.at_name_segment() {
            self.bump();
        } else {
            self.expected_here(message);
        }
    }

    pub(in crate::parser::grammar) fn expect_variable_identifier(&mut self, message: &str) {
        if self.at_variable_identifier() {
            self.bump();
        } else {
            self.expected_here(message);
        }
    }

    pub(in crate::parser::grammar) fn expect_named_variable_identifier(&mut self, message: &str) {
        if self.at_name_segment() {
            self.bump();
        } else {
            self.expected_here(message);
        }
    }

    pub(in crate::parser::grammar) fn consume_qualified_name(&mut self) -> bool {
        if !self.at_name_segment() {
            self.expected_here("expected identifier");
            return false;
        }

        let name = self.start();
        self.bump();
        let mut qualified = false;
        while self.at(JavaSyntaxKind::Dot) && self.nth_is_name_segment(1) {
            qualified = true;
            self.bump();
            self.bump();
        }
        self.complete(
            name,
            if qualified {
                JavaSyntaxKind::QualifiedName
            } else {
                JavaSyntaxKind::Name
            },
        );
        true
    }

    pub(in crate::parser::grammar) fn at_variable_identifier(&mut self) -> bool {
        self.is_variable_identifier_at_offset(self.position())
    }

    pub(in crate::parser::grammar) fn is_variable_identifier_at_offset(
        &mut self,
        index: usize,
    ) -> bool {
        matches!(
            self.kind_at(index),
            JavaSyntaxKind::Identifier | JavaSyntaxKind::UnderscoreKw
        )
    }

    pub(in crate::parser::grammar) fn at_type_identifier(&mut self) -> bool {
        self.current_kind() == JavaSyntaxKind::Identifier
            && !matches!(
                self.current_text(),
                Some("permits" | "record" | "sealed" | "var" | "yield")
            )
    }
}
