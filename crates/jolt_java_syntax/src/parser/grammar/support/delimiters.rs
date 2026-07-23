// Skips or consumes balanced delimiter groups used by grammar lookahead and recovery.
use super::{JavaSyntaxKind, Parser};
use crate::parser::source::{LookaheadSummary, TokenBuffer, TokenCursor};

// Built on the first parenthesis or annotation query. Token-disjoint entries
// store the position after a matching `)` for `(`, or the end of a maximal
// annotation run for queried `@`; zero means no cached fact.
impl LookaheadSummary {
    pub(super) fn after(
        &mut self,
        buffer: &mut TokenBuffer<'_>,
        cursor: TokenCursor,
        floor: TokenCursor,
    ) -> usize {
        self.ensure_built(buffer, floor);

        let start = cursor.position();
        let offset = start
            .checked_sub(self.base)
            .expect("parenthesis query precedes summary floor");
        self.boundaries
            .as_ref()
            .and_then(|boundaries| boundaries.get(offset))
            .copied()
            .filter(|boundary| *boundary != 0)
            .expect("summary must contain every in-range opening parenthesis")
            - 1
    }

    pub(super) fn ensure_built(&mut self, buffer: &mut TokenBuffer<'_>, floor: TokenCursor) {
        if self.boundaries.is_none() {
            self.build(buffer, floor);
        }
    }

    fn build(&mut self, buffer: &mut TokenBuffer<'_>, mut cursor: TokenCursor) {
        self.base = cursor.position();
        let mut boundaries = Vec::new();
        let mut open = 0usize;
        loop {
            let kind = cursor.kind(buffer);
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
            cursor.bump(buffer);
            if kind == JavaSyntaxKind::RParen && open != 0 {
                let index = open - 1;
                open = boundaries[index];
                boundaries[index] = cursor.position() + 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_compilation_unit;

    fn count_nodes(source: &str, kind: JavaSyntaxKind) -> usize {
        let parse = parse_compilation_unit(source);
        let root = parse.syntax().expect("represented compilation unit");
        assert_eq!(root.source_text(), source);
        let mut nodes = vec![*root.syntax()];
        let mut count = 0;
        while let Some(node) = nodes.pop() {
            nodes.extend(node.children());
            count += usize::from(node.kind() == kind);
        }
        count
    }

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
        assert_eq!(lookahead.text(), Some("value"));
    }

    #[test]
    fn full_parse_handles_deep_parentheses_and_malformed_annotations() {
        let depth = 128;
        let nested = format!(
            "class C {{ Object value = {}input{}; }} class D {{}}",
            "(".repeat(depth),
            ")".repeat(depth)
        );
        assert_eq!(
            count_nodes(&nested, JavaSyntaxKind::ParenthesizedExpression),
            depth
        );
        assert_eq!(count_nodes(&nested, JavaSyntaxKind::ClassDeclaration), 2);

        let malformed = format!("{}value; class Following {{}}", "@A(value=".repeat(96));
        assert_eq!(count_nodes(&malformed, JavaSyntaxKind::ClassDeclaration), 1);
        assert!(!parse_compilation_unit(&malformed).diagnostics().is_empty());
    }

    #[test]
    fn unmatched_parenthesis_summary_and_bounded_annotation_recovery_are_exact() {
        let mut parser = Parser::new("(value");
        let cursor = parser.inner.fork_cursor();
        assert_eq!(
            parser
                .lookahead_summary
                .after(&mut parser.inner.buffer, cursor, cursor),
            2
        );
        assert_eq!(
            parser
                .lookahead_summary
                .after(&mut parser.inner.buffer, cursor, cursor),
            2
        );

        for (source, classes) in [
            ("import a @A(value; class C {}", 1),
            ("@A( class C {}", 1),
            ("@module 0", 0),
        ] {
            assert_eq!(
                count_nodes(source, JavaSyntaxKind::ClassDeclaration),
                classes
            );
            assert!(!parse_compilation_unit(source).diagnostics().is_empty());
        }
    }

    #[test]
    fn annotation_run_summary_preserves_malformed_recovery_boundaries() {
        let depth = 128;
        let annotations = "@A ".repeat(depth);
        let cases = [
            (format!("{annotations}0; class Following {{}}"), 1, true),
            (
                format!("import a {annotations}; class Following {{}}"),
                1,
                true,
            ),
            (
                format!("class Outer {{ {annotations}0; class Nested {{}} }}"),
                2,
                true,
            ),
            (
                format!("class Outer {{ Outer int value) {annotations}0; class Nested {{}} }}"),
                2,
                true,
            ),
            ("@A(@B) class C {}".to_owned(), 1, false),
        ];

        for (source, classes, malformed) in cases {
            assert_eq!(
                count_nodes(&source, JavaSyntaxKind::ClassDeclaration),
                classes,
                "{source}"
            );
            assert_eq!(
                !parse_compilation_unit(&source).diagnostics().is_empty(),
                malformed,
                "{source}"
            );
        }

        for (source, malformed) in [("@A @interface C {}", false), ("@ @interface C {}", true)] {
            assert_eq!(
                count_nodes(source, JavaSyntaxKind::AnnotationInterfaceDeclaration),
                1,
                "{source}"
            );
            assert_eq!(
                !parse_compilation_unit(source).diagnostics().is_empty(),
                malformed,
                "{source}"
            );
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
