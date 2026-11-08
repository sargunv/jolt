//! The Java grammar: one recursive-descent function per construct, and the
//! lookahead that decides between them.

use jolt_syntax::TokenSet;

use crate::JavaSyntaxKind;

use super::source::Parser;

/// A [`TokenSet`] of the named Java token kinds.
macro_rules! token_set {
    ($($kind:ident),* $(,)?) => {
        jolt_syntax::TokenSet::new(&[$(crate::JavaSyntaxKind::$kind.to_raw()),*])
    };
}

/// Where a too-deeply nested expression's recovery stops.
const EXPRESSION_STOPS: TokenSet = token_set![Semicolon, Comma, RParen, RBracket, RBrace];

/// Where a too-deeply nested record pattern's recovery stops.
const PATTERN_STOPS: TokenSet =
    EXPRESSION_STOPS.union(token_set![Arrow, Colon, AndAnd, OrOr, Question]);

/// Where a too-deeply nested statement or body member's recovery stops.
const BODY_STOPS: TokenSet = token_set![RBrace];

/// Where a too-deeply nested type's recovery stops.
const TYPE_STOPS: TokenSet = token_set![
    Gt,
    RShift,
    UnsignedRShift,
    GtEq,
    RShiftEq,
    UnsignedRShiftEq,
    Comma,
    RParen,
    RBracket,
    LBrace,
];

mod compilation_unit;
mod declarations;
mod expressions;
mod patterns;
mod statements;
mod support;
mod types;

pub(super) use support::LookaheadTable;
