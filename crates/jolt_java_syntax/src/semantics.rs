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
    /// The elements, each with its partition.
    pub fn partitioned(
        self,
    ) -> impl Iterator<Item = (ModifierElement<'a>, ModifierPartition)> + 'a {
        partition(self.items(), |item| {
            matches!(item, ModifierElement::Annotation(_))
        })
    }
}

impl<'a> ParameterModifierList<'a> {
    /// The elements, each with its partition.
    pub fn partitioned(
        self,
    ) -> impl Iterator<Item = (ParameterModifier<'a>, ModifierPartition)> + 'a {
        partition(self.items(), |item| {
            matches!(item, ParameterModifier::Annotation(_))
        })
    }
}

/// Pairs each modifier list element with its partition: an annotation is a
/// declaration annotation until the first other element.
fn partition<T: Copy>(
    items: impl Iterator<Item = T>,
    is_annotation: impl Fn(T) -> bool,
) -> impl Iterator<Item = (T, ModifierPartition)> {
    let mut saw_modifier = false;
    items.map(move |item| {
        let partition = if is_annotation(item) {
            if saw_modifier {
                ModifierPartition::TypeUseAnnotation
            } else {
                ModifierPartition::DeclarationAnnotation
            }
        } else {
            saw_modifier = true;
            ModifierPartition::Modifier
        };
        (item, partition)
    })
}
