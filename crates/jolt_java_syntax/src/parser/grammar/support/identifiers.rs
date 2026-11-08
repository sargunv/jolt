//! Java identifier roles that depend on parser context.

use jolt_syntax::{CompletedMarker, TokenSet};

use super::{JavaSyntaxKind, Parser};
use crate::parser::codes;

impl Parser<'_> {
    pub(in crate::parser::grammar) fn expect_type_identifier(&mut self, message: &str) {
        if self.at_type_identifier() {
            self.bump();
        } else if self.lookahead().at_name_segment() {
            self.error(codes::RESTRICTED_TYPE_IDENTIFIER, message);
            self.bump();
        } else {
            self.error_expected(message);
        }
    }

    /// Eats an identifier, which cannot be `_`.
    pub(in crate::parser::grammar) fn expect_named_identifier(&mut self, message: &str) {
        if self.lookahead().at_name_segment() {
            self.bump();
        } else {
            self.error_expected(message);
        }
    }

    /// Eats an identifier or the unnamed variable `_`.
    pub(in crate::parser::grammar) fn expect_variable_identifier(&mut self, message: &str) {
        if self.lookahead().at_variable_identifier() {
            self.bump();
        } else {
            self.error_expected(message);
        }
    }

    /// A qualified name. Returns whether there was one; reports a diagnostic
    /// otherwise.
    pub(in crate::parser::grammar) fn consume_qualified_name_required(&mut self) -> bool {
        let consumed = self.consume_qualified_name_contents().is_some();
        if !consumed {
            self.error_expected("expected identifier");
        }
        consumed
    }

    /// A qualified name, then (when anything but a `stops` token or the next
    /// name follows) a `Bogus` up to one of those: a name ends where another
    /// name, such as a contextual keyword or the next directive, starts.
    pub(in crate::parser::grammar) fn consume_qualified_name_required_until(
        &mut self,
        stops: TokenSet,
    ) {
        let parsed_name = self.consume_qualified_name_contents();
        let at_boundary = self.at_qualified_name_boundary(stops);

        if parsed_name.is_some() && at_boundary {
            return;
        }
        if parsed_name.is_none() && at_boundary {
            self.error_expected("expected identifier");
            return;
        }

        let name = match parsed_name {
            Some(name) => self.precede(name),
            None => self.start(),
        };
        self.error_expected("expected identifier");
        while !self.at_qualified_name_boundary(stops) {
            self.bump();
        }
        self.complete(name, JavaSyntaxKind::Bogus);
    }

    fn at_qualified_name_boundary(&self, stops: TokenSet) -> bool {
        self.at_eof() || self.at_any(stops) || self.lookahead().at_name_segment()
    }

    fn consume_qualified_name_contents(&mut self) -> Option<CompletedMarker> {
        if !self.lookahead().at_name_segment() {
            return None;
        }

        let name = self.start();
        if self.nth(1) != JavaSyntaxKind::Dot || self.nth(2) != JavaSyntaxKind::Identifier {
            self.bump();
            return Some(self.complete(name, JavaSyntaxKind::Name));
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
            if !self.at(JavaSyntaxKind::Dot) || self.nth(1) != JavaSyntaxKind::Identifier {
                break;
            }
            self.bump();
        }
        self.complete(remaining_segments, JavaSyntaxKind::NameSegmentDotList);
        Some(self.complete(name, JavaSyntaxKind::QualifiedName))
    }

    pub(in crate::parser::grammar) fn at_type_identifier(&self) -> bool {
        self.current() == JavaSyntaxKind::Identifier
            && !matches!(
                self.current_text(),
                "permits" | "record" | "sealed" | "var" | "yield"
            )
    }
}
