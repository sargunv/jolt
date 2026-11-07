//! Java facts that are not schema projections.

use jolt_syntax::SyntaxToken;

use crate::{
    JavaSyntaxKind, ModifierElement, ModifierList, ParameterModifier, ParameterModifierList,
};

/// Assignment operator tokens.
#[must_use]
pub const fn is_assignment_operator(kind: JavaSyntaxKind) -> bool {
    matches!(
        kind,
        JavaSyntaxKind::Assign
            | JavaSyntaxKind::PlusEq
            | JavaSyntaxKind::MinusEq
            | JavaSyntaxKind::StarEq
            | JavaSyntaxKind::SlashEq
            | JavaSyntaxKind::AmpEq
            | JavaSyntaxKind::BarEq
            | JavaSyntaxKind::CaretEq
            | JavaSyntaxKind::PercentEq
            | JavaSyntaxKind::LShiftEq
            | JavaSyntaxKind::RShiftEq
            | JavaSyntaxKind::UnsignedRShiftEq
    )
}

/// The precedence of a binary operator token, from 1 (`||`) to 10 (`*`).
/// `instanceof` shares the relational level.
#[must_use]
pub const fn binary_precedence(kind: JavaSyntaxKind) -> Option<u8> {
    use JavaSyntaxKind as K;
    Some(match kind {
        K::OrOr => 1,
        K::AndAnd => 2,
        K::Bar => 3,
        K::Caret => 4,
        K::Amp => 5,
        K::EqEq | K::BangEq => 6,
        K::Lt | K::Gt | K::LtEq | K::GtEq | K::InstanceofKw => 7,
        K::LShift | K::RShift | K::UnsignedRShift => 8,
        K::Plus | K::Minus => 9,
        K::Star | K::Slash | K::Percent => 10,
        _ => return None,
    })
}

/// Whether `token` is the contextual keyword `text` (an identifier whose
/// text, after Unicode escape translation, is `text`).
#[must_use]
pub fn is_contextual(token: SyntaxToken<'_>, text: &str) -> bool {
    JavaSyntaxKind::from_raw(token.kind()) == JavaSyntaxKind::Identifier
        && crate::lexer::lexical_text_is(token.text(), text)
}

/// What a modifier list element is. Annotations before the first keyword
/// modifier apply to the declaration; annotations after one apply to the type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModifierPartition {
    DeclarationAnnotation,
    TypeUseAnnotation,
    Modifier,
}

impl<'a> ModifierList<'a> {
    pub fn partitioned(
        self,
    ) -> impl Iterator<Item = (ModifierElement<'a>, ModifierPartition)> + 'a {
        let mut saw_modifier = false;
        self.items().map(move |item| {
            let partition = if let ModifierElement::Annotation(_) = item {
                annotation_partition(saw_modifier)
            } else {
                saw_modifier = true;
                ModifierPartition::Modifier
            };
            (item, partition)
        })
    }
}

impl<'a> ParameterModifierList<'a> {
    pub fn partitioned(
        self,
    ) -> impl Iterator<Item = (ParameterModifier<'a>, ModifierPartition)> + 'a {
        let mut saw_modifier = false;
        self.items().map(move |item| {
            let partition = if let ParameterModifier::Annotation(_) = item {
                annotation_partition(saw_modifier)
            } else {
                saw_modifier = true;
                ModifierPartition::Modifier
            };
            (item, partition)
        })
    }
}

const fn annotation_partition(saw_modifier: bool) -> ModifierPartition {
    if saw_modifier {
        ModifierPartition::TypeUseAnnotation
    } else {
        ModifierPartition::DeclarationAnnotation
    }
}
