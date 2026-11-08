//! Lookahead, predicates, and shared helpers for the grammar.

use super::{JavaSyntaxKind, Parser};

mod declaration_predicates;
mod expression_predicates;
mod identifiers;
mod lookahead;
mod lookahead_table;
mod pattern_predicates;
mod recovery;
mod separated_lists;
mod statement_predicates;
mod token_predicates;
mod type_arguments;

pub(in crate::parser::grammar) use crate::is_assignment_operator;
pub(in crate::parser::grammar) use lookahead::JavaLookahead;
pub(in crate::parser) use lookahead_table::LookaheadTable;
pub(in crate::parser::grammar) use separated_lists::SeparatedElement;

pub(super) const fn is_type_argument_value_start(kind: JavaSyntaxKind) -> bool {
    matches!(kind, JavaSyntaxKind::Question | JavaSyntaxKind::Identifier)
        || is_primitive_type_start(kind)
}

pub(super) const fn is_type_argument_recovery_boundary(kind: JavaSyntaxKind) -> bool {
    matches!(
        kind,
        JavaSyntaxKind::Eof
            | JavaSyntaxKind::Gt
            | JavaSyntaxKind::Comma
            | JavaSyntaxKind::Semicolon
            | JavaSyntaxKind::Assign
            | JavaSyntaxKind::LBrace
            | JavaSyntaxKind::RBrace
            | JavaSyntaxKind::LParen
            | JavaSyntaxKind::RParen
            | JavaSyntaxKind::LBracket
            | JavaSyntaxKind::RBracket
            | JavaSyntaxKind::Colon
            | JavaSyntaxKind::Arrow
    )
}

pub(super) const fn is_primitive_type_start(kind: JavaSyntaxKind) -> bool {
    matches!(
        kind,
        JavaSyntaxKind::BooleanKw
            | JavaSyntaxKind::ByteKw
            | JavaSyntaxKind::CharKw
            | JavaSyntaxKind::DoubleKw
            | JavaSyntaxKind::FloatKw
            | JavaSyntaxKind::IntKw
            | JavaSyntaxKind::LongKw
            | JavaSyntaxKind::ShortKw
    )
}

pub(super) const fn is_literal_expression_start(kind: JavaSyntaxKind) -> bool {
    matches!(
        kind,
        JavaSyntaxKind::IntegerLiteral
            | JavaSyntaxKind::FloatingPointLiteral
            | JavaSyntaxKind::BooleanLiteral
            | JavaSyntaxKind::CharacterLiteral
            | JavaSyntaxKind::StringLiteral
            | JavaSyntaxKind::TextBlockLiteral
            | JavaSyntaxKind::NullLiteral
    )
}

/// Whether an annotation starts at a token of `kind` followed by one of
/// `next` (`@interface` starts a declaration).
pub(super) fn starts_annotation(kind: JavaSyntaxKind, next: JavaSyntaxKind) -> bool {
    kind == JavaSyntaxKind::At && next != JavaSyntaxKind::InterfaceKw
}

/// The kind left after removing the first `>` of a `>`-family token.
pub(super) const fn gt_rest(kind: JavaSyntaxKind) -> Option<JavaSyntaxKind> {
    Some(match kind {
        JavaSyntaxKind::RShift => JavaSyntaxKind::Gt,
        JavaSyntaxKind::UnsignedRShift => JavaSyntaxKind::RShift,
        JavaSyntaxKind::GtEq => JavaSyntaxKind::Assign,
        JavaSyntaxKind::RShiftEq => JavaSyntaxKind::GtEq,
        JavaSyntaxKind::UnsignedRShiftEq => JavaSyntaxKind::RShiftEq,
        _ => return None,
    })
}

/// Whether `kind` starts with `>`, so it can close a type argument list.
pub(super) const fn is_gt_family(kind: JavaSyntaxKind) -> bool {
    matches!(kind, JavaSyntaxKind::Gt) || gt_rest(kind).is_some()
}

/// The number of leading `>` characters of a token.
pub(super) const fn gt_count(kind: JavaSyntaxKind) -> usize {
    match kind {
        JavaSyntaxKind::Gt | JavaSyntaxKind::GtEq => 1,
        JavaSyntaxKind::RShift | JavaSyntaxKind::RShiftEq => 2,
        JavaSyntaxKind::UnsignedRShift | JavaSyntaxKind::UnsignedRShiftEq => 3,
        _ => 0,
    }
}

/// A modifier of a declaration's modifier list.
#[derive(Clone, Copy)]
pub(super) enum TypeModifier {
    /// A modifier keyword, or `sealed`: one token.
    Keyword,
    /// `non-sealed`: three tokens.
    NonSealed,
}

impl TypeModifier {
    /// The modifier that starts at a token of `kind` and `text`, followed by
    /// a token of `next_kind` and then one of `next_text`.
    fn from_tokens(
        kind: JavaSyntaxKind,
        text: &str,
        next_kind: JavaSyntaxKind,
        next_text: &str,
    ) -> Option<Self> {
        if matches!(
            kind,
            JavaSyntaxKind::PublicKw
                | JavaSyntaxKind::ProtectedKw
                | JavaSyntaxKind::PrivateKw
                | JavaSyntaxKind::AbstractKw
                | JavaSyntaxKind::StaticKw
                | JavaSyntaxKind::FinalKw
                | JavaSyntaxKind::TransientKw
                | JavaSyntaxKind::VolatileKw
                | JavaSyntaxKind::SynchronizedKw
                | JavaSyntaxKind::NativeKw
                | JavaSyntaxKind::StrictfpKw
                | JavaSyntaxKind::DefaultKw
        ) || text == "sealed"
        {
            Some(Self::Keyword)
        } else if text == "non" && next_kind == JavaSyntaxKind::Minus && next_text == "sealed" {
            Some(Self::NonSealed)
        } else {
            None
        }
    }

    /// The number of tokens the modifier spans.
    const fn len(self) -> usize {
        match self {
            Self::Keyword => 1,
            Self::NonSealed => 3,
        }
    }
}
