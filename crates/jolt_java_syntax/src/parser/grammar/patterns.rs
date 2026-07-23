use super::{JavaParserExt, JavaSyntaxKind, Parser};

#[derive(Clone, Copy)]
pub(in crate::parser::grammar) enum PatternStart {
    Type,
    Record { open_paren: usize },
}

impl Parser<'_> {
    pub(super) fn parse_pattern_until(&mut self, start: PatternStart, stops: &[JavaSyntaxKind]) {
        match start {
            PatternStart::Record { open_paren } => {
                if self
                    .with_syntax_nesting(Self::parse_record_pattern)
                    .is_none()
                {
                    self.parse_excessive_record_pattern(open_paren);
                }
            }
            PatternStart::Type => self.parse_type_pattern_until(stops, false),
        }
    }

    fn parse_excessive_record_pattern(&mut self, open_paren: usize) {
        let pattern = self.start();
        let diagnostic = self.pending_excessive_syntax_nesting();
        let end =
            self.skip_balanced_from(open_paren, JavaSyntaxKind::LParen, JavaSyntaxKind::RParen);
        while self.position() < end {
            self.bump();
        }
        self.complete_recovery(pattern, JavaSyntaxKind::BogusPattern, [diagnostic]);
    }

    pub(super) fn parse_type_pattern_until(
        &mut self,
        stops: &[JavaSyntaxKind],
        allow_component_type: bool,
    ) {
        let pattern = self.start();
        let owner = pattern.anchor();
        self.parse_variable_modifiers();
        if allow_component_type {
            self.parse_type();
        } else if self.at_contextual("var") && self.nth_kind(1) != JavaSyntaxKind::Dot {
            let bogus_type = self.start();
            let diagnostic = self.pending_expected("expected reference type");
            self.bump();
            self.complete_recovery(bogus_type, JavaSyntaxKind::BogusType, [diagnostic]);
        } else {
            self.parse_reference_type();
        }
        self.expect_variable_identifier_required(
            "expected pattern variable name",
            owner,
            crate::shape::type_pattern::Slot::name as u16,
            true,
        );
        self.parse_array_dimensions();
        let pattern = self.complete(pattern, JavaSyntaxKind::TypePattern);

        if !self.at_eof()
            && !stops.contains(&self.current_kind())
            && !self.at_contextual("when")
            && self.binary_operator().is_none()
        {
            let bogus = self.precede(pattern);
            let diagnostic = self.pending_unexpected("invalid type pattern declaration");
            while !self.at_eof()
                && !stops.contains(&self.current_kind())
                && !self.at_contextual("when")
                && self.binary_operator().is_none()
            {
                self.bump();
            }
            self.complete_recovery(bogus, JavaSyntaxKind::BogusPattern, [diagnostic]);
        }
    }

    pub(super) fn parse_record_pattern(&mut self) {
        let pattern = self.start();
        self.parse_class_type();
        if !self.eat(JavaSyntaxKind::LParen) {
            let diagnostic = self.pending_expected("expected `(` in record pattern");
            self.missing_required_slot(
                pattern.anchor(),
                crate::shape::record_pattern::Slot::open_paren as u16,
                [diagnostic],
            );
        }
        let components = self.start();
        if !self.at_eof() && !self.at(JavaSyntaxKind::RParen) {
            while !self.at_eof() && !self.at(JavaSyntaxKind::RParen) {
                self.parse_component_pattern();
                if !self.eat(JavaSyntaxKind::Comma) {
                    break;
                }
            }
        }
        self.complete(components, JavaSyntaxKind::ComponentPatternList);
        if !self.eat(JavaSyntaxKind::RParen) {
            let diagnostic = self.pending_expected("expected `)` after record pattern");
            self.missing_required_slot(
                pattern.anchor(),
                crate::shape::record_pattern::Slot::close_paren as u16,
                [diagnostic],
            );
        }
        self.complete(pattern, JavaSyntaxKind::RecordPattern);
    }

    pub(super) fn parse_component_pattern(&mut self) {
        let component = self.start();
        if self.at(JavaSyntaxKind::UnderscoreKw) {
            let match_all = self.start();
            self.bump();
            self.complete(match_all, JavaSyntaxKind::MatchAllPattern);
        } else if let Some(start @ PatternStart::Record { .. }) = self.pattern_start() {
            self.parse_pattern_until(start, &[JavaSyntaxKind::Comma, JavaSyntaxKind::RParen]);
        } else {
            self.parse_type_pattern_until(&[JavaSyntaxKind::Comma, JavaSyntaxKind::RParen], true);
        }
        self.complete(component, JavaSyntaxKind::ComponentPattern);
    }
}
