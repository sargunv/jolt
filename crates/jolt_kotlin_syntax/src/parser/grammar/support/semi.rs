use crate::KotlinSyntaxKind as K;

use super::super::Parser;

impl Parser<'_> {
    pub(in crate::parser::grammar) fn eat_optional_separators(&mut self) -> bool {
        self.eat_semicolon_boundary()
    }

    pub(in crate::parser::grammar) fn at_block_end(&mut self) -> bool {
        matches!(self.current_kind(), K::RBrace | K::Eof)
    }
}
