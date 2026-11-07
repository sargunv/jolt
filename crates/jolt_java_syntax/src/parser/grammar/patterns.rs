use super::{JavaSyntaxKind, PATTERN_STOPS, Parser};

#[derive(Clone, Copy)]
pub(in crate::parser::grammar) enum PatternStart {
    Type,
    Record,
}

impl Parser<'_> {
    pub(super) fn parse_pattern_until(&mut self, start: PatternStart, stops: &[JavaSyntaxKind]) {
        match start {
            PatternStart::Record => {
                if self.nested(Self::parse_record_pattern).is_none() {
                    self.recover_too_deep(PATTERN_STOPS);
                }
            }
            PatternStart::Type => self.parse_type_pattern_until(stops, false),
        }
    }

    pub(super) fn parse_type_pattern_until(
        &mut self,
        stops: &[JavaSyntaxKind],
        allow_component_type: bool,
    ) {
        let pattern = self.start();
        self.parse_variable_modifiers();
        if allow_component_type {
            self.parse_type();
        } else if self.at_contextual("var") && self.nth(1) != JavaSyntaxKind::Dot {
            let bogus_type = self.start();
            self.error_expected("expected reference type");
            self.bump();
            self.complete(bogus_type, JavaSyntaxKind::Bogus);
        } else {
            self.parse_reference_type();
        }
        self.expect_variable_identifier("expected pattern variable name", true);
        self.parse_array_dimensions();
        let pattern = self.complete(pattern, JavaSyntaxKind::TypePattern);

        if !self.at_eof()
            && !stops.contains(&self.current())
            && !self.at_contextual("when")
            && self.binary_operator().is_none()
        {
            let bogus = self.precede(pattern);
            self.error_unexpected("invalid type pattern declaration");
            while !self.at_eof()
                && !stops.contains(&self.current())
                && !self.at_contextual("when")
                && self.binary_operator().is_none()
            {
                self.bump();
            }
            self.complete(bogus, JavaSyntaxKind::Bogus);
        }
    }

    pub(super) fn parse_record_pattern(&mut self) {
        let pattern = self.start();
        self.parse_class_type();
        if !self.eat(JavaSyntaxKind::LParen) {
            self.error_expected("expected `(` in record pattern");
        }
        let components = self.start();
        self.parse_comma_separated(
            "expected component pattern",
            |parser| parser.at(JavaSyntaxKind::RParen),
            Self::parse_component_pattern,
        );
        self.complete(components, JavaSyntaxKind::ComponentPatternList);
        if !self.eat(JavaSyntaxKind::RParen) {
            self.error_expected("expected `)` after record pattern");
        }
        self.complete(pattern, JavaSyntaxKind::RecordPattern);
    }

    pub(super) fn parse_component_pattern(&mut self) {
        let component = self.start();
        if self.at(JavaSyntaxKind::UnderscoreKw) {
            let match_all = self.start();
            self.bump();
            self.complete(match_all, JavaSyntaxKind::MatchAllPattern);
        } else if let Some(start @ PatternStart::Record) = self.pattern_start() {
            self.parse_pattern_until(start, &[JavaSyntaxKind::Comma, JavaSyntaxKind::RParen]);
        } else {
            self.parse_type_pattern_until(&[JavaSyntaxKind::Comma, JavaSyntaxKind::RParen], true);
        }
        self.complete(component, JavaSyntaxKind::ComponentPattern);
    }
}
