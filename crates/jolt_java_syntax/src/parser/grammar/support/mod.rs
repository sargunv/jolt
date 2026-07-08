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
