// Skips or consumes balanced delimiter groups used by grammar lookahead and recovery.
use super::{JavaSyntaxKind, Parser};

impl Parser<'_> {
    pub(in crate::parser::grammar) fn skip_balanced_from(
        &self,
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
        &self,
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
