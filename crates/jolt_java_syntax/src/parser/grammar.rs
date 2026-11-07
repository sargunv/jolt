use jolt_syntax::TokenSet;

use crate::JavaSyntaxKind;

use super::source::Parser;

const fn token_set(kinds: &[JavaSyntaxKind]) -> TokenSet {
    let mut set = TokenSet::EMPTY;
    let mut index = 0;
    while index < kinds.len() {
        set = set.with(kinds[index].to_raw());
        index += 1;
    }
    set
}

/// Where a too-deeply nested expression's recovery stops.
const EXPRESSION_STOPS: TokenSet = token_set(&[
    JavaSyntaxKind::Semicolon,
    JavaSyntaxKind::Comma,
    JavaSyntaxKind::RParen,
    JavaSyntaxKind::RBracket,
    JavaSyntaxKind::RBrace,
]);

/// Where a too-deeply nested record pattern's recovery stops.
const PATTERN_STOPS: TokenSet = EXPRESSION_STOPS.union(token_set(&[
    JavaSyntaxKind::Arrow,
    JavaSyntaxKind::Colon,
    JavaSyntaxKind::AndAnd,
    JavaSyntaxKind::OrOr,
    JavaSyntaxKind::Question,
]));

/// Where a too-deeply nested statement or body member's recovery stops.
const BODY_STOPS: TokenSet = token_set(&[JavaSyntaxKind::RBrace]);

/// Where a too-deeply nested type's recovery stops.
const TYPE_STOPS: TokenSet = token_set(&[
    JavaSyntaxKind::Gt,
    JavaSyntaxKind::RShift,
    JavaSyntaxKind::UnsignedRShift,
    JavaSyntaxKind::GtEq,
    JavaSyntaxKind::RShiftEq,
    JavaSyntaxKind::UnsignedRShiftEq,
    JavaSyntaxKind::Comma,
    JavaSyntaxKind::RParen,
    JavaSyntaxKind::RBracket,
    JavaSyntaxKind::LBrace,
]);

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

pub(super) use support::ScanTables;
#[path = "grammar/types.rs"]
mod types;
