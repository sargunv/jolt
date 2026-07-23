// Handles generic type argument closes, consuming one `>` atom at a time.
use super::{JavaSyntaxKind, Parser};
use crate::parser::source::{TokenBuffer, TokenCursor};

pub(in crate::parser::grammar) fn over_depth_type_end(
    buffer: &mut TokenBuffer<'_>,
    mut cursor: TokenCursor,
) -> usize {
    let (mut angles, mut parens, mut braces, mut brackets) = (0usize, 0usize, 0usize, 0usize);
    loop {
        let kind = cursor.kind(buffer);
        let outside_delimiters = parens == 0 && braces == 0 && brackets == 0;
        if matches!(kind, JavaSyntaxKind::Eof | JavaSyntaxKind::Semicolon)
            || matches!(kind, JavaSyntaxKind::RParen) && parens == 0
            || matches!(kind, JavaSyntaxKind::RBrace) && braces == 0
            || matches!(kind, JavaSyntaxKind::RBracket) && brackets == 0
            || (outside_delimiters
                && (matches!(
                    kind,
                    JavaSyntaxKind::Assign
                        | JavaSyntaxKind::LBrace
                        | JavaSyntaxKind::Colon
                        | JavaSyntaxKind::Arrow
                ) || (angles == 0
                    && matches!(
                        kind,
                        JavaSyntaxKind::Comma
                            | JavaSyntaxKind::Gt
                            | JavaSyntaxKind::Amp
                            | JavaSyntaxKind::Bar
                    ))))
        {
            return cursor.position();
        }

        match kind {
            JavaSyntaxKind::Lt if outside_delimiters => angles += 1,
            JavaSyntaxKind::Gt if outside_delimiters => angles = angles.saturating_sub(1),
            JavaSyntaxKind::LParen => parens += 1,
            JavaSyntaxKind::RParen => parens = parens.saturating_sub(1),
            JavaSyntaxKind::LBrace => braces += 1,
            JavaSyntaxKind::RBrace => braces = braces.saturating_sub(1),
            JavaSyntaxKind::LBracket => brackets += 1,
            JavaSyntaxKind::RBracket => brackets = brackets.saturating_sub(1),
            _ => {}
        }
        cursor.bump(buffer);
    }
}

impl Parser<'_> {
    pub(in crate::parser::grammar) fn at_type_argument_close(&mut self) -> bool {
        self.current_kind() == JavaSyntaxKind::Gt
    }

    pub(in crate::parser::grammar) fn eat_type_argument_close(&mut self) -> bool {
        if self.at_type_argument_close() {
            self.bump();
            true
        } else {
            false
        }
    }

    pub(in crate::parser::grammar) fn type_arguments_are_followed_by_double_colon(
        &mut self,
    ) -> bool {
        let mut lookahead = self.lookahead();
        lookahead.skip_type_arguments();
        lookahead.at(JavaSyntaxKind::DoubleColon)
    }

    pub(in crate::parser::grammar) fn type_arguments_are_followed_by_dot(&mut self) -> bool {
        let mut lookahead = self.lookahead();
        lookahead.skip_type_arguments();
        lookahead.at(JavaSyntaxKind::Dot)
    }

    pub(in crate::parser::grammar) fn dot_is_followed_by_annotated_name(&mut self) -> bool {
        if !self.at(JavaSyntaxKind::Dot) {
            return false;
        }

        let mut lookahead = self.lookahead();
        lookahead.bump();
        lookahead.skip_annotations();
        lookahead.at_name_segment()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A fixture snapshot cannot isolate this scanner contract: surrounding grammar
    // can compensate for an off-by-one endpoint and still produce the same tree.

    #[test]
    fn over_depth_scan_stops_at_outer_and_hard_recovery_boundaries() {
        for (fragment, end) in [
            ("Type<Nested>, tail", 4),
            ("Type @A(value = {1, 2}), tail", 12),
            ("Broken(value; int following", 3),
            ("Broken[value; int following", 3),
            ("Broken[> value; int following", 2),
            ("Broken[, tail", 2),
            ("Broken(value } class Following", 3),
            ("Broken[value ) tail", 3),
            ("Type & Other", 1),
            ("Type | Other", 1),
        ] {
            let mut parser = Parser::new(fragment);
            let cursor = parser.inner.fork_cursor();
            assert_eq!(
                over_depth_type_end(&mut parser.inner.buffer, cursor),
                end,
                "fragment: {fragment}"
            );
        }
    }
}
