use crate::KotlinSyntaxKind;

use super::source::{ParseEvents, Parser};

#[derive(Clone, Copy)]
struct StopSet<'a> {
    kinds: &'a [KotlinSyntaxKind],
    // Expression parsing composes at most the enclosing control-flow stop and
    // the braced-expression recovery stop. Keep both inline so passing a stop
    // through another expression layer cannot overwrite it.
    extras: [Option<KotlinSyntaxKind>; 2],
    position: Option<usize>,
}

impl<'a> StopSet<'a> {
    const fn new(kinds: &'a [KotlinSyntaxKind]) -> Self {
        Self {
            kinds,
            extras: [None, None],
            position: None,
        }
    }

    fn with_extra(mut self, extra: KotlinSyntaxKind) -> Self {
        if self.extras.contains(&Some(extra)) {
            return self;
        }
        if let Some(slot) = self.extras.iter_mut().find(|slot| slot.is_none()) {
            *slot = Some(extra);
        }
        self
    }

    const fn with_position(self, position: Option<usize>) -> Self {
        Self { position, ..self }
    }

    fn contains(self, kind: KotlinSyntaxKind, position: usize) -> bool {
        self.position == Some(position)
            || self.extras.contains(&Some(kind))
            || self.kinds.contains(&kind)
    }
}

impl<'a> From<&'a [KotlinSyntaxKind]> for StopSet<'a> {
    fn from(kinds: &'a [KotlinSyntaxKind]) -> Self {
        Self::new(kinds)
    }
}

impl<'a, const N: usize> From<&'a [KotlinSyntaxKind; N]> for StopSet<'a> {
    fn from(kinds: &'a [KotlinSyntaxKind; N]) -> Self {
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
