use crate::KotlinSyntaxKind;

use super::source::{ParseEvents, Parser};

const STOP_KIND_WORDS: usize = KotlinSyntaxKind::TOKEN_KIND_COUNT.div_ceil(u64::BITS as usize);

#[derive(Clone, Copy)]
struct TokenKindSet([u64; STOP_KIND_WORDS]);

impl TokenKindSet {
    const fn new() -> Self {
        Self([0; STOP_KIND_WORDS])
    }

    fn insert(&mut self, kind: KotlinSyntaxKind) {
        let index = usize::from(u16::from(kind));
        assert!(
            index < KotlinSyntaxKind::TOKEN_KIND_COUNT,
            "expression stop must be a token kind"
        );
        self.0[index / u64::BITS as usize] |= 1 << (index % u64::BITS as usize);
    }

    fn contains(self, kind: KotlinSyntaxKind) -> bool {
        let index = usize::from(u16::from(kind));
        index < KotlinSyntaxKind::TOKEN_KIND_COUNT
            && self.0[index / u64::BITS as usize] & (1 << (index % u64::BITS as usize)) != 0
    }
}

#[derive(Clone, Copy)]
struct StopSet {
    kinds: TokenKindSet,
    position: Option<usize>,
}

impl StopSet {
    fn new(kinds: &[KotlinSyntaxKind]) -> Self {
        let mut set = TokenKindSet::new();
        for &kind in kinds {
            set.insert(kind);
        }
        Self {
            kinds: set,
            position: None,
        }
    }

    fn with_kind(mut self, kind: KotlinSyntaxKind) -> Self {
        self.kinds.insert(kind);
        self
    }

    const fn with_position(self, position: Option<usize>) -> Self {
        Self { position, ..self }
    }

    fn contains(self, kind: KotlinSyntaxKind, position: usize) -> bool {
        self.position == Some(position) || self.kinds.contains(kind)
    }
}

impl From<&[KotlinSyntaxKind]> for StopSet {
    fn from(kinds: &[KotlinSyntaxKind]) -> Self {
        Self::new(kinds)
    }
}

impl<const N: usize> From<&[KotlinSyntaxKind; N]> for StopSet {
    fn from(kinds: &[KotlinSyntaxKind; N]) -> Self {
        Self::new(kinds)
    }
}

impl Parser<'_> {
    fn parse_excessive_braced_contents(&mut self, kind: KotlinSyntaxKind) {
        let contents = self.start();
        let diagnostic = self.pending_excessive_syntax_nesting();
        let mut depth = 0usize;
        while !self.at_eof() {
            match self.current_kind() {
                KotlinSyntaxKind::RBrace if depth == 0 => break,
                KotlinSyntaxKind::LBrace => depth += 1,
                KotlinSyntaxKind::RBrace => depth -= 1,
                _ => {}
            }
            self.bump();
        }
        self.complete_recovery(contents, kind, [diagnostic]);
    }
}

#[cfg(test)]
mod tests {
    use super::{KotlinSyntaxKind as K, StopSet};

    #[test]
    fn composed_expression_stops_retain_every_added_kind() {
        let stops = StopSet::new(&[K::Semicolon])
            .with_kind(K::ElseKw)
            .with_kind(K::RBrace)
            .with_kind(K::NotIs);

        for kind in [K::Semicolon, K::ElseKw, K::RBrace, K::NotIs] {
            assert!(stops.contains(kind, 0), "missing composed stop {kind:?}");
        }
        assert!(!stops.contains(K::RParen, 0));
    }
}

#[path = "grammar/declarations.rs"]
mod declarations;
#[path = "grammar/expressions.rs"]
mod expressions;
#[path = "grammar/file.rs"]
mod file;
#[path = "grammar/statements.rs"]
mod statements;
#[path = "grammar/support/mod.rs"]
mod support;
#[path = "grammar/types.rs"]
mod types;
