use crate::KotlinSyntaxKind as K;
use crate::parser::source::{TokenBuffer, TokenCursor};

use super::super::Parser;

const MAX_TYPE_ARGUMENT_LOOKAHEAD: usize = 64;

impl<'source> Parser<'source> {
    pub(in crate::parser::grammar) fn type_argument_list_issue_ahead(
        &mut self,
    ) -> Option<&'static str> {
        if self.position() == 0 || !type_argument_receiver_kind(self.kind_at(self.position() - 1)) {
            return None;
        }

        let mut lookahead = self.lookahead();
        lookahead.type_argument_list_issue()
    }

    pub(in crate::parser::grammar) fn type_argument_list_is_call_suffix_ahead(&mut self) -> bool {
        if self.position() == 0 || !type_argument_receiver_kind(self.kind_at(self.position() - 1)) {
            return false;
        }

        let mut lookahead = self.lookahead();
        lookahead.type_argument_list_is_call_suffix()
    }

    fn lookahead(&mut self) -> KotlinLookahead<'_, 'source> {
        let cursor = self.fork_cursor();
        KotlinLookahead::new(&mut self.buffer, cursor)
    }
}

struct KotlinLookahead<'buffer, 'source> {
    buffer: &'buffer mut TokenBuffer<'source>,
    cursor: TokenCursor,
}

impl<'buffer, 'source> KotlinLookahead<'buffer, 'source> {
    fn new(buffer: &'buffer mut TokenBuffer<'source>, cursor: TokenCursor) -> Self {
        Self { buffer, cursor }
    }

    fn kind(&mut self) -> K {
        self.cursor.kind(self.buffer)
    }

    fn nth_kind(&mut self, n: usize) -> K {
        self.cursor.nth_kind(self.buffer, n)
    }

    fn tokens_are_adjacent(&mut self, offset: usize, count: usize) -> bool {
        self.buffer
            .tokens_are_adjacent(self.cursor.position() + offset, count)
    }

    fn bump(&mut self) {
        self.cursor.bump(self.buffer);
    }

    fn type_argument_list_issue(&mut self) -> Option<&'static str> {
        let start = self.cursor.checkpoint();
        let mut depth = 0usize;
        let mut expect_argument = false;
        let mut previous_kind = None;

        for _ in 0..MAX_TYPE_ARGUMENT_LOOKAHEAD {
            let kind = self.kind();
            match kind {
                K::Lt => {
                    depth += 1;
                    expect_argument = true;
                }
                K::Gt if depth > 0 => {
                    if expect_argument {
                        self.cursor.rewind(start);
                        return Some("malformed type argument list");
                    }
                    depth -= 1;
                    if depth == 0 {
                        self.cursor.rewind(start);
                        return None;
                    }
                    expect_argument = false;
                }
                K::Comma if depth > 0 => {
                    if expect_argument && !matches!(self.nth_kind(1), K::Gt | K::Eof) {
                        self.cursor.rewind(start);
                        return Some("malformed type argument list");
                    }
                    expect_argument = true;
                }
                K::LParen | K::LBrace if depth > 0 => {
                    let issue = previous_kind == Some(K::Gt);
                    self.cursor.rewind(start);
                    return issue.then_some("malformed type argument list");
                }
                K::OrOr
                | K::AndAnd
                | K::Plus
                | K::Minus
                | K::Slash
                | K::Percent
                | K::Range
                | K::RangeUntil
                | K::Elvis
                | K::EqEq
                | K::BangEq
                | K::EqEqEq
                | K::BangEqEqEq
                | K::RParen
                | K::RBracket
                | K::Semicolon
                | K::DoubleSemicolon
                | K::RBrace
                | K::Eof
                    if depth > 0 =>
                {
                    self.cursor.rewind(start);
                    return None;
                }
                _ if depth > 0 && !matches!(kind, K::Dot | K::Question | K::ColonColon) => {
                    expect_argument = false;
                }
                _ => {}
            }
            previous_kind = Some(kind);
            self.bump();
        }

        self.cursor.rewind(start);
        None
    }

    fn type_argument_list_is_call_suffix(&mut self) -> bool {
        let start = self.cursor.checkpoint();
        let mut depth = 0usize;

        for _ in 0..MAX_TYPE_ARGUMENT_LOOKAHEAD {
            match self.kind() {
                K::Lt => depth += 1,
                K::Gt if depth > 0 => {
                    depth -= 1;
                    if depth == 0 {
                        let is_suffix = matches!(
                            self.nth_kind(1),
                            K::LParen | K::LBrace | K::Dot | K::SafeAccess | K::ColonColon
                        ) || self.nth_kind(1) == K::Question
                            && self.nth_kind(2) == K::Dot
                            && self.tokens_are_adjacent(1, 2);
                        self.cursor.rewind(start);
                        return is_suffix;
                    }
                }
                K::Semicolon | K::DoubleSemicolon | K::RBrace | K::Eof => {
                    self.cursor.rewind(start);
                    return false;
                }
                _ => {}
            }
            self.bump();
        }

        self.cursor.rewind(start);
        false
    }
}

fn type_argument_receiver_kind(kind: K) -> bool {
    matches!(kind, K::Identifier | K::FieldIdentifier)
}
