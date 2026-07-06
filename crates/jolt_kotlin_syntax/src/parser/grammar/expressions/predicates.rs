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
            | K::Amp
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
    matches!(
        kind,
        K::Dot
            | K::SafeAccess
            | K::ColonColon
            | K::LParen
            | K::LBracket
            | K::LBrace
            | K::Plus
            | K::Minus
            | K::Star
            | K::Slash
            | K::Percent
            | K::Range
            | K::RangeUntil
            | K::Elvis
            | K::AndAnd
            | K::OrOr
            | K::Amp
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
