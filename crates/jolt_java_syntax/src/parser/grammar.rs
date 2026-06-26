use crate::JavaSyntaxKind;

use super::source::{ParseEvents, Parser};

#[derive(Clone, Copy)]
struct StopSet<'a> {
    kinds: &'a [JavaSyntaxKind],
    extra: Option<JavaSyntaxKind>,
}

impl<'a> StopSet<'a> {
    const fn new(kinds: &'a [JavaSyntaxKind]) -> Self {
        Self { kinds, extra: None }
    }

    const fn with_extra(self, extra: JavaSyntaxKind) -> Self {
        Self {
            kinds: self.kinds,
            extra: Some(extra),
        }
    }

    fn contains(self, kind: JavaSyntaxKind) -> bool {
        self.extra == Some(kind) || self.kinds.contains(&kind)
    }
}

impl<'a> From<&'a [JavaSyntaxKind]> for StopSet<'a> {
    fn from(kinds: &'a [JavaSyntaxKind]) -> Self {
        Self::new(kinds)
    }
}

impl<'a, const N: usize> From<&'a [JavaSyntaxKind; N]> for StopSet<'a> {
    fn from(kinds: &'a [JavaSyntaxKind; N]) -> Self {
        Self::new(kinds)
    }
}

#[path = "grammar/compilation_unit.rs"]
mod compilation_unit;
#[path = "grammar/declarations.rs"]
mod declarations;
#[path = "grammar/expressions.rs"]
mod expressions;
#[path = "grammar/patterns.rs"]
mod patterns;
#[path = "grammar/statements.rs"]
mod statements;
#[path = "grammar/support/mod.rs"]
mod support;
#[path = "grammar/types.rs"]
mod types;
