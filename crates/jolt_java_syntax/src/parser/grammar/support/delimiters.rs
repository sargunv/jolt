// Skips or consumes balanced delimiter groups used by grammar lookahead and recovery.
use super::{JavaSyntaxKind, Parser};
use crate::parser::source::{ParenthesisSummary, TokenBuffer, TokenCursor};

// A normal file spends no heap space on delimiter queries. Once direct scans
// cross this fixed budget, one exact table makes all later parenthesis queries
// constant-time. Thus direct work is at most the budget plus one query, and the
// table construction visits each remaining token once.
const DIRECT_PARENTHESIS_WORK_BUDGET: usize = 16 * 1024;

impl ParenthesisSummary {
    pub(super) fn after(
        &mut self,
        buffer: &mut TokenBuffer<'_>,
        mut cursor: TokenCursor,
        floor: TokenCursor,
    ) -> usize {
        let start = cursor.position();
        if let Some(boundaries) = &self.boundaries {
            let offset = start
                .checked_sub(self.base)
                .expect("parenthesis query precedes summary floor");
            return boundaries
                .get(offset)
                .copied()
                .filter(|boundary| *boundary != 0)
                .expect("summary must contain every in-range opening parenthesis")
                - 1;
        }

        let mut depth = 0usize;
        loop {
            let kind = cursor.kind(buffer);
            if kind == JavaSyntaxKind::Eof {
                break;
            }
            match kind {
                JavaSyntaxKind::LParen => depth += 1,
                JavaSyntaxKind::RParen => depth = depth.saturating_sub(1),
                _ => {}
            }
            cursor.bump(buffer);
            if depth == 0 {
                break;
            }
        }
        self.direct_work = self.direct_work.saturating_add(cursor.position() - start);
        if self.direct_work >= DIRECT_PARENTHESIS_WORK_BUDGET {
            self.build(buffer, floor);
        }
        cursor.position()
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

            #[cfg(test)]
            {
                self.build_work += 1;
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
    fn nested_lambda_rejection_activates_one_bounded_summary() {
        let depth = 128;
        let source = format!("{}value{}", "(".repeat(depth), ")".repeat(depth));
        let mut parser = Parser::new(&source);

        for _ in 0..depth {
            assert!(!parser.starts_parenthesized_lambda_expression());
            parser.bump();
        }

        assert!(parser.parentheses.boundaries.is_some());
        assert!(parser.parentheses.direct_work <= DIRECT_PARENTHESIS_WORK_BUDGET + 2 * depth + 1);
        assert!(
            parser.parentheses.direct_work + parser.parentheses.build_work
                <= DIRECT_PARENTHESIS_WORK_BUDGET + 2 * (2 * depth + 1)
        );
    }

    #[test]
    fn nested_annotation_skips_reuse_the_same_summary() {
        let depth = 96;
        let source = format!("{}value{}", "@A(value=".repeat(depth), ")".repeat(depth));
        let mut parser = Parser::new(&source);
        let mut cached_queries = 0;

        while !parser.at_eof() {
            if parser.at(JavaSyntaxKind::At) {
                let work = parser.parentheses.direct_work;
                let mut lookahead = parser.lookahead();
                assert!(lookahead.skip_annotation());
                if parser.parentheses.boundaries.is_some() && parser.parentheses.direct_work == work
                {
                    cached_queries += 1;
                }
            }
            parser.bump();
        }

        assert!(cached_queries > 0);
        assert!(
            parser.parentheses.direct_work + parser.parentheses.build_work
                <= DIRECT_PARENTHESIS_WORK_BUDGET + 2 * (6 * depth + 1)
        );
    }

    #[test]
    fn annotation_summary_uses_the_lookahead_creation_floor() {
        let mut parser = Parser::new("@A(x) @B(y) value");
        parser.parentheses.direct_work = DIRECT_PARENTHESIS_WORK_BUDGET - 4;
        assert!(parser.lookahead().skip_annotations());
        assert!(parser.parentheses.boundaries.is_some());

        let work = parser.parentheses.direct_work;
        assert!(parser.lookahead().skip_annotations());
        assert_eq!(parser.parentheses.direct_work, work);
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
        parser.parentheses.direct_work = DIRECT_PARENTHESIS_WORK_BUDGET;
        let cursor = parser.inner.fork_cursor();
        assert_eq!(
            parser
                .parentheses
                .after(&mut parser.inner.buffer, cursor, cursor),
            2
        );
        assert_eq!(
            parser
                .parentheses
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
