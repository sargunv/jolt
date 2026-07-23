use super::{JavaParserExt, JavaSyntaxKind, Parser};

mod declaration_predicates;
mod delimiters;
mod expression_predicates;
mod identifiers;
mod lookahead;
mod pattern_predicates;
mod recovery;
mod statement_predicates;
mod token_predicates;
mod type_arguments;

pub(in crate::parser::grammar) use lookahead::JavaLookahead;
pub(super) use type_arguments::{MAX_GENERIC_TYPE_DEPTH, over_depth_type_end};

#[derive(Clone, Copy)]
pub(super) enum MissingConstructorHeaderAction {
    OpenNested,
    CloseNested,
    OpenBrace,
    CloseBrace,
    CloseHeader,
    Boundary,
    Bump,
}

pub(super) fn missing_constructor_header_action(
    kind: JavaSyntaxKind,
    paren_depth: usize,
    brace_depth: usize,
) -> MissingConstructorHeaderAction {
    match kind {
        JavaSyntaxKind::LParen => MissingConstructorHeaderAction::OpenNested,
        JavaSyntaxKind::RParen if paren_depth == 0 => MissingConstructorHeaderAction::CloseHeader,
        JavaSyntaxKind::RParen => MissingConstructorHeaderAction::CloseNested,
        JavaSyntaxKind::LBrace if paren_depth > 0 || brace_depth > 0 => {
            MissingConstructorHeaderAction::OpenBrace
        }
        JavaSyntaxKind::RBrace if brace_depth > 0 => MissingConstructorHeaderAction::CloseBrace,
        JavaSyntaxKind::LBrace | JavaSyntaxKind::RBrace => MissingConstructorHeaderAction::Boundary,
        JavaSyntaxKind::Semicolon if paren_depth == 0 && brace_depth == 0 => {
            MissingConstructorHeaderAction::Boundary
        }
        _ => MissingConstructorHeaderAction::Bump,
    }
}

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

fn type_modifier_len(
    kind: JavaSyntaxKind,
    text: Option<&str>,
    next_kind: JavaSyntaxKind,
    next_text: Option<&str>,
) -> Option<usize> {
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
    ) || text == Some("sealed")
    {
        Some(1)
    } else if text == Some("non")
        && next_kind == JavaSyntaxKind::Minus
        && next_text == Some("sealed")
    {
        Some(3)
    } else {
        None
    }
}
