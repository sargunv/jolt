// Skips or consumes balanced delimiter groups used by grammar lookahead and recovery.
use jolt_syntax::Lookahead;

use super::{JavaSyntaxKind, Parser};
use crate::JavaLanguage;
use crate::parser::source::LookaheadSummary;

// Built on the first parenthesis or annotation query. Token-disjoint entries
// store the position after a matching `)` for `(`, or the end of a maximal
// annotation run for queried `@`; zero means no cached fact.
impl LookaheadSummary {
    pub(super) fn after(&mut self, cursor: Lookahead<'_, '_, JavaLanguage>, floor: usize) -> usize {
        self.ensure_built(cursor, floor);

        let offset = cursor.position() - self.base;
        self.boundaries
            .as_ref()
            .and_then(|boundaries| boundaries.get(offset))
            .copied()
            .filter(|boundary| *boundary != 0)
            .expect("summary must contain every in-range opening parenthesis")
            - 1
    }

    /// Builds the table from `floor` on the first query, or again if a query
    /// starts before the floor of the existing table.
    pub(super) fn ensure_built(&mut self, cursor: Lookahead<'_, '_, JavaLanguage>, floor: usize) {
        if self.boundaries.is_none() || floor < self.base {
            let mut scan = cursor;
            scan.set_position(floor);
            self.build(scan);
        }
    }

    fn build(&mut self, mut cursor: Lookahead<'_, '_, JavaLanguage>) {
        self.base = cursor.position();
        self.modifier_ends.clear();
        let mut boundaries = Vec::new();
        let mut open = 0usize;
        loop {
            let kind = cursor.current();
            if kind == JavaSyntaxKind::Eof {
                while open != 0 {
                    let index = open - 1;
                    open = boundaries[index];
                    boundaries[index] = cursor.position() + 1;
                }
                self.boundaries = Some(boundaries);
                return;
            }

            boundaries.push(0);
            if kind == JavaSyntaxKind::LParen {
                let index = cursor.position() - self.base;
                boundaries[index] = open;
                open = index + 1;
            }
            cursor.bump();
            if kind == JavaSyntaxKind::RParen && open != 0 {
                let index = open - 1;
                open = boundaries[index];
                boundaries[index] = cursor.position() + 1;
            }
        }
    }
}

impl Parser<'_> {
    pub(in crate::parser::grammar) fn skip_balanced_from(
        &mut self,
        mut index: usize,
        open: JavaSyntaxKind,
        close: JavaSyntaxKind,
    ) -> usize {
        let mut depth = 0usize;
        while self.kind_at(index) != JavaSyntaxKind::Eof {
            if self.kind_at(index) == open {
                depth += 1;
            } else if self.kind_at(index) == close {
                depth = depth.saturating_sub(1);
                index += 1;
                if depth == 0 {
                    return index;
                }
                continue;
            }
            index += 1;
        }
        index
    }

    pub(in crate::parser::grammar) fn skip_balanced_delimiter_at(
        &mut self,
        index: usize,
    ) -> Option<usize> {
        match self.kind_at(index) {
            JavaSyntaxKind::LParen => {
                Some(self.skip_balanced_from(index, JavaSyntaxKind::LParen, JavaSyntaxKind::RParen))
            }
            JavaSyntaxKind::LBracket => Some(self.skip_balanced_from(
                index,
                JavaSyntaxKind::LBracket,
                JavaSyntaxKind::RBracket,
            )),
            _ => None,
        }
    }

    pub(in crate::parser::grammar) fn consume_balanced_delimited(
        &mut self,
        open: JavaSyntaxKind,
        close: JavaSyntaxKind,
    ) {
        if !self.at(open) {
            return;
        }

        let mut depth = 0usize;
        while !self.at_eof() {
            if self.at(open) {
                depth += 1;
            } else if self.at(close) {
                depth = depth.saturating_sub(1);
                self.bump();
                if depth == 0 {
                    return;
                }
                continue;
            }
            self.bump();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Corpus fixtures can verify parse results, but cannot prove summary activation,
    // cache reuse, or the counted-work bound that makes repeated lookahead linear.

    #[test]
    fn nested_lambda_rejection_builds_one_lazy_summary() {
        let depth = 128;
        let source = format!("{}value{}", "(".repeat(depth), ")".repeat(depth));
        let mut parser = Parser::new(&source);

        for _ in 0..depth {
            assert!(!parser.starts_parenthesized_lambda_expression());
            parser.bump();
        }

        assert!(parser.lookahead_summary.boundaries.is_some());
    }

    #[test]
    fn nested_annotation_skips_reuse_the_same_summary() {
        let depth = 96;
        let source = format!("{}value{}", "@A(value=".repeat(depth), ")".repeat(depth));
        let mut parser = Parser::new(&source);
        while !parser.at_eof() {
            if parser.at(JavaSyntaxKind::At) {
                let mut lookahead = parser.lookahead();
                assert!(lookahead.skip_annotations());
            }
            parser.bump();
        }

        assert!(parser.lookahead_summary.boundaries.is_some());
    }

    #[test]
    fn annotation_summary_uses_the_lookahead_creation_floor() {
        let mut parser = Parser::new("@A(x) @B(y) value");
        assert!(parser.lookahead().skip_annotations());
        assert!(parser.lookahead_summary.boundaries.is_some());

        assert!(parser.lookahead().skip_annotations());
    }

    // Corpus fixtures prove parse results, but cannot prove that every suffix of
    // one annotation run has a constant-time cached endpoint.
    #[test]
    fn flat_qualified_annotation_suffixes_share_one_cached_endpoint() {
        let depth = 256;
        let segments = 8;
        let annotation = format!("@{} ", vec!["name"; segments].join("."));
        let source = format!("{};", annotation.repeat(depth));
        let tokens_per_annotation = 2 * segments;
        let token_count = tokens_per_annotation * depth + 1;
        let mut parser = Parser::new(&source);

        let mut lookahead = parser.lookahead();
        assert!(lookahead.skip_annotations());
        assert!(lookahead.at(JavaSyntaxKind::Semicolon));

        let boundaries = parser
            .lookahead_summary
            .boundaries
            .as_ref()
            .expect("annotation query builds summary");
        for start in (0..tokens_per_annotation * depth).step_by(tokens_per_annotation) {
            assert_eq!(boundaries[start], token_count);
        }
    }

    #[test]
    fn annotation_cache_survives_speculative_cursor_rewind() {
        let mut parser = Parser::new("T @A value");
        let mut lookahead = parser.lookahead();
        assert!(lookahead.skip_type());
        assert!(lookahead.at(JavaSyntaxKind::At));
        assert!(lookahead.skip_annotations());
        assert_eq!(lookahead.text(), "value");
    }

    // The forced-summary EOF endpoint is an internal cache contract; surrounding
    // recovery can compensate for an off-by-one result in a fixture parse.
    #[test]
    fn unmatched_parenthesis_summary_ends_at_eof() {
        let mut parser = Parser::new("(value");
        let cursor = parser.inner.lookahead();
        assert_eq!(parser.lookahead_summary.after(cursor, 0), 2);
        assert_eq!(parser.lookahead_summary.after(cursor, 0), 2);
    }
}
