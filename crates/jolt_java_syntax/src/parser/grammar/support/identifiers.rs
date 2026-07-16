// Handles Java identifier roles that depend on parser context.
use super::{JavaParserExt, JavaSyntaxKind, Parser};
use jolt_syntax::UnresolvedDiagnosticOwner;

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
        self.consume_qualified_name_with_owner(None)
    }

    pub(in crate::parser::grammar) fn consume_qualified_name_owned(
        &mut self,
        owner: UnresolvedDiagnosticOwner,
    ) -> bool {
        self.consume_qualified_name_with_owner(Some(owner))
    }

    fn consume_qualified_name_with_owner(
        &mut self,
        owner: Option<UnresolvedDiagnosticOwner>,
    ) -> bool {
        if !self.at_name_segment() {
            let diagnostic = self.expected_here("expected identifier");
            if let Some(owner) = owner {
                self.own_diagnostic(diagnostic, owner);
            }
            return false;
        }

        let name = self.start();
        if self.nth_kind(1) != JavaSyntaxKind::Dot || !self.nth_is_name_segment(2) {
            self.bump();
            self.complete(name, JavaSyntaxKind::Name);
            return true;
        }

        let first_segment = self.start();
        self.parse_annotations();
        self.bump();
        self.complete(first_segment, JavaSyntaxKind::QualifiedNameSegmentNode);
        self.bump();

        let remaining_segments = self.start();
        loop {
            let segment = self.start();
            self.parse_annotations();
            self.bump();
            self.complete(segment, JavaSyntaxKind::QualifiedNameSegmentNode);
            if !self.at(JavaSyntaxKind::Dot) || !self.nth_is_name_segment(1) {
                break;
            }
            self.bump();
        }
        self.complete(remaining_segments, JavaSyntaxKind::NameSegmentDotList);
        self.complete(name, JavaSyntaxKind::QualifiedName);
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
