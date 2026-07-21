// Handles Java identifier roles that depend on parser context.
use super::{JavaParserExt, JavaSyntaxKind, Parser};
use jolt_syntax::{CompletedMarker, NodeAnchor, PendingDiagnostic};

impl Parser<'_> {
    pub(in crate::parser::grammar) fn expect_type_identifier(
        &mut self,
        message: &str,
        owner: NodeAnchor,
        slot: u16,
    ) -> Option<PendingDiagnostic> {
        if self.at_type_identifier() {
            self.bump();
            None
        } else if self.at_name_segment() {
            let diagnostic = self.restricted_type_identifier_here(message);
            self.bump();
            Some(diagnostic)
        } else {
            let diagnostic = self.pending_expected(message);
            self.missing_required_slot(owner, slot, [diagnostic]);
            None
        }
    }

    pub(in crate::parser::grammar) fn expect_method_identifier_required(
        &mut self,
        message: &str,
        owner: NodeAnchor,
        slot: u16,
    ) {
        if self.at_name_segment() {
            self.bump();
        } else {
            let diagnostic = self.pending_expected(message);
            self.missing_required_slot(owner, slot, [diagnostic]);
        }
    }

    pub(in crate::parser::grammar) fn expect_named_identifier_required(
        &mut self,
        message: &str,
        owner: NodeAnchor,
        slot: u16,
    ) {
        if self.at_name_segment() {
            self.bump();
        } else {
            let diagnostic = self.pending_expected(message);
            self.missing_required_slot(owner, slot, [diagnostic]);
        }
    }

    pub(in crate::parser::grammar) fn expect_variable_identifier_required(
        &mut self,
        message: &str,
        owner: NodeAnchor,
        slot: u16,
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
            let diagnostic = self.pending_expected(message);
            self.missing_required_slot(owner, slot, [diagnostic]);
        }
    }

    pub(in crate::parser::grammar) fn consume_qualified_name_required(
        &mut self,
        owner: NodeAnchor,
        slot: u16,
    ) -> bool {
        if self.consume_qualified_name_contents(|_, _| false).is_some() {
            true
        } else {
            let diagnostic = self.pending_expected("expected identifier");
            self.missing_required_slot(owner, slot, [diagnostic]);
            false
        }
    }

    pub(in crate::parser::grammar) fn consume_qualified_name_required_until<F>(
        &mut self,
        owner: NodeAnchor,
        slot: u16,
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
            let diagnostic = self.pending_expected("expected identifier");
            self.missing_required_slot(owner, slot, [diagnostic]);
            return;
        }

        let name = match parsed_name {
            Some(name) => self.precede(name),
            None => self.start(),
        };
        let diagnostic = self.pending_expected("expected identifier");
        while !self.at_qualified_name_boundary(stops, contextual_boundary, stop_at_name_segment) {
            self.bump();
        }
        self.complete_recovery(name, JavaSyntaxKind::BogusName, [diagnostic]);
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
            || stops.contains(&self.current_kind())
            || stop_at_name_segment && self.at_name_segment()
            || contextual_boundary(self, 0)
    }

    pub(in crate::parser::grammar) fn consume_qualified_name_cause(
        &mut self,
    ) -> Option<PendingDiagnostic> {
        self.consume_qualified_name_contents(|_, _| false)
            .is_none()
            .then(|| self.pending_expected("expected identifier"))
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
        if self.nth_kind(1) != JavaSyntaxKind::Dot
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
        self.current_kind() == JavaSyntaxKind::Identifier
            && !matches!(
                self.current_text(),
                Some("permits" | "record" | "sealed" | "var" | "yield")
            )
    }
}
