// Handles Java identifier roles that depend on parser context.
use super::{JavaSyntaxKind, Parser};
use crate::parser::codes;
use jolt_syntax::CompletedMarker;

impl Parser<'_> {
    pub(in crate::parser::grammar) fn expect_type_identifier(&mut self, message: &str) {
        if self.at_type_identifier() {
            self.bump();
        } else if self.at_name_segment() {
            self.error(codes::RESTRICTED_TYPE_IDENTIFIER, message);
            self.bump();
        } else {
            self.error_expected(message);
        }
    }

    pub(in crate::parser::grammar) fn expect_named_identifier(&mut self, message: &str) {
        if self.at_name_segment() {
            self.bump();
        } else {
            self.error_expected(message);
        }
    }

    pub(in crate::parser::grammar) fn expect_variable_identifier(
        &mut self,
        message: &str,
        allow_unnamed: bool,
    ) {
        let accepted = if allow_unnamed {
            self.at_variable_identifier()
        } else {
            self.at_name_segment()
        };
        if accepted {
            self.bump();
        } else {
            self.error_expected(message);
        }
    }

    pub(in crate::parser::grammar) fn consume_qualified_name_required(&mut self) -> bool {
        if self.consume_qualified_name_contents(|_, _| false).is_some() {
            true
        } else {
            self.error_expected("expected identifier");
            false
        }
    }

    pub(in crate::parser::grammar) fn consume_qualified_name_required_until<F>(
        &mut self,
        stops: &[JavaSyntaxKind],
        contextual_boundary: F,
        stop_at_name_segment: bool,
    ) where
        F: Fn(&mut Self, usize) -> bool + Copy,
    {
        let parsed_name = self.consume_qualified_name_contents(contextual_boundary);
        let at_boundary =
            self.at_qualified_name_boundary(stops, contextual_boundary, stop_at_name_segment);

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
        while !self.at_qualified_name_boundary(stops, contextual_boundary, stop_at_name_segment) {
            self.bump();
        }
        self.complete(name, JavaSyntaxKind::Bogus);
    }

    fn at_qualified_name_boundary<F>(
        &mut self,
        stops: &[JavaSyntaxKind],
        contextual_boundary: F,
        stop_at_name_segment: bool,
    ) -> bool
    where
        F: Fn(&mut Self, usize) -> bool,
    {
        self.at_eof()
            || stops.contains(&self.current())
            || stop_at_name_segment && self.at_name_segment()
            || contextual_boundary(self, 0)
    }

    /// Returns whether a name was consumed; reports a diagnostic otherwise.
    pub(in crate::parser::grammar) fn consume_qualified_name_or_report(&mut self) -> bool {
        let consumed = self.consume_qualified_name_contents(|_, _| false).is_some();
        if !consumed {
            self.error_expected("expected identifier");
        }
        consumed
    }

    fn consume_qualified_name_contents<F>(
        &mut self,
        contextual_boundary: F,
    ) -> Option<CompletedMarker>
    where
        F: Fn(&mut Self, usize) -> bool,
    {
        if !self.at_name_segment() {
            return None;
        }

        let name = self.start();
        if self.nth(1) != JavaSyntaxKind::Dot
            || !self.nth_is_name_segment(2)
            || contextual_boundary(self, 2)
        {
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
            if !self.at(JavaSyntaxKind::Dot)
                || !self.nth_is_name_segment(1)
                || contextual_boundary(self, 1)
            {
                break;
            }
            self.bump();
        }
        self.complete(remaining_segments, JavaSyntaxKind::NameSegmentDotList);
        Some(self.complete(name, JavaSyntaxKind::QualifiedName))
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
        self.current() == JavaSyntaxKind::Identifier
            && !matches!(
                self.current_text(),
                "permits" | "record" | "sealed" | "var" | "yield"
            )
    }
}
