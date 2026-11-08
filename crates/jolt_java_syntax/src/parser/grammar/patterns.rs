//! Patterns: type patterns and record patterns.

use jolt_syntax::TokenSet;

use super::{JavaSyntaxKind, PATTERN_STOPS, Parser};

#[derive(Clone, Copy)]
pub(in crate::parser::grammar) enum PatternStart {
    Type,
    Record,
}

impl Parser<'_> {
    pub(super) fn parse_pattern_until(&mut self, start: PatternStart, stops: TokenSet) {
        match start {
            PatternStart::Record => {
                if self.nested(Self::parse_record_pattern).is_none() {
                    self.recover_too_deep(PATTERN_STOPS);
                }
            }
            PatternStart::Type => self.parse_type_pattern_until(stops, Self::parse_pattern_type),
        }
    }

    /// The type of a type pattern outside a record pattern: a reference type.
    fn parse_pattern_type(&mut self) {
        if self.at_contextual("var") && self.nth(1) != JavaSyntaxKind::Dot {
            let bogus_type = self.start();
            self.error_expected("expected reference type");
            self.bump();
            self.complete(bogus_type, JavaSyntaxKind::Bogus);
        } else {
            self.parse_reference_type();
        }
    }

    /// A type pattern whose type `parse_type` parses.
    fn parse_type_pattern_until(&mut self, stops: TokenSet, parse_type: impl FnOnce(&mut Self)) {
        let pattern = self.start();
        self.parse_variable_modifiers();
        parse_type(self);
        self.expect_variable_identifier("expected pattern variable name");
        self.parse_array_dimensions();
        let pattern = self.complete(pattern, JavaSyntaxKind::TypePattern);

        if !self.at_eof()
            && !self.at_any(stops)
            && !self.at_contextual("when")
            && self.binary_operator().is_none()
        {
            let bogus = self.precede(pattern);
            self.error_unexpected("invalid type pattern declaration");
            while !self.at_eof()
                && !self.at_any(stops)
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
            self.parse_pattern_until(start, token_set![Comma, RParen]);
        } else {
            // A record component's type may be primitive.
            self.parse_type_pattern_until(token_set![Comma, RParen], |parser| {
                parser.parse_type();
            });
        }
        self.complete(component, JavaSyntaxKind::ComponentPattern);
    }
}
