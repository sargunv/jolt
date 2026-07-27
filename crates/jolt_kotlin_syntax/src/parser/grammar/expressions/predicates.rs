use crate::KotlinSyntaxKind as K;

pub(super) fn is_literal_kind(kind: K) -> bool {
    matches!(
        kind,
        K::IntegerLiteral
            | K::FloatLiteral
            | K::CharacterLiteral
            | K::NullKw
            | K::TrueKw
            | K::FalseKw
    )
}

pub(super) fn is_assignment_operator(kind: K) -> bool {
    matches!(
        kind,
        K::Assign | K::PlusEq | K::MinusEq | K::StarEq | K::SlashEq | K::PercentEq
    )
}

pub(super) fn expression_start_kind(kind: K) -> bool {
    matches!(
        kind,
        K::Identifier
            | K::FieldIdentifier
            | K::IntegerLiteral
            | K::FloatLiteral
            | K::CharacterLiteral
            | K::OpenQuote
            | K::InterpolationPrefix
            | K::ThisKw
            | K::SuperKw
            | K::NullKw
            | K::TrueKw
            | K::FalseKw
            | K::IfKw
            | K::WhenKw
            | K::TryKw
            | K::ForKw
            | K::WhileKw
            | K::DoKw
            | K::ThrowKw
            | K::LParen
            | K::LBracket
            | K::LBrace
            | K::Plus
            | K::Minus
            | K::Bang
            | K::BangBang
            | K::Star
    )
}

pub(super) fn is_binary_operator(kind: K) -> bool {
    matches!(
        kind,
        K::Plus
            | K::Minus
            | K::Star
            | K::Slash
            | K::Percent
            | K::Range
            | K::RangeUntil
            | K::Elvis
            | K::AndAnd
            | K::OrOr
            | K::EqEq
            | K::BangEq
            | K::EqEqEq
            | K::BangEqEqEq
            | K::Lt
            | K::LtEq
            | K::Gt
            | K::GtEq
    )
}

pub(super) fn is_unary_operator(kind: K) -> bool {
    matches!(
        kind,
        K::Plus | K::Minus | K::Bang | K::PlusPlus | K::MinusMinus | K::Star
    )
}

pub(super) fn is_expression_continuation(kind: K) -> bool {
    // Keep this aligned with parse_postfix_expression's newline-allowed suffixes
    // and binary_operator_info. Primary-expression starters such as `(`, `[`, `{`,
    // and `::` must not appear here, or a new statement can be mistaken for an
    // unterminated previous expression and recovery can grow badly on repeated
    // inputs. Besides postfix navigation, only the operators whose grammar
    // explicitly admits a preceding newline continue from the next line.
    matches!(
        kind,
        K::Dot | K::SafeAccess | K::Elvis | K::AndAnd | K::OrOr | K::AsKw | K::AsSafe
    )
}

pub(super) fn expression_can_continue_after(kind: K) -> bool {
    is_binary_operator(kind)
        || matches!(
            kind,
            K::Dot | K::SafeAccess | K::InKw | K::NotIn | K::IsKw | K::NotIs | K::AsKw | K::AsSafe
        )
}
