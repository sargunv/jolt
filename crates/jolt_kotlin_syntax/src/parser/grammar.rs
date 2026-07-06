use crate::KotlinSyntaxKind;

use super::source::{ParseEvents, Parser};

#[derive(Clone, Copy)]
struct StopSet<'a> {
    kinds: &'a [KotlinSyntaxKind],
    extra: Option<KotlinSyntaxKind>,
}

impl<'a> StopSet<'a> {
    const fn new(kinds: &'a [KotlinSyntaxKind]) -> Self {
        Self { kinds, extra: None }
    }

    const fn with_extra(self, extra: KotlinSyntaxKind) -> Self {
        Self {
            kinds: self.kinds,
            extra: Some(extra),
        }
    }

    fn contains(self, kind: KotlinSyntaxKind) -> bool {
        self.extra == Some(kind) || self.kinds.contains(&kind)
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
