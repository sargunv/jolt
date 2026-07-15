use std::fmt;

pub use jolt_syntax::{
    Comment as KotlinComment, CommentKind as KotlinCommentKind, Comments as KotlinComments,
};
use jolt_syntax::{SyntaxElement, SyntaxNode, SyntaxSlot, SyntaxToken, SyntaxVerbatimCore};
use jolt_text::TextRange;

use crate::{KotlinSyntaxKind, language::KotlinLanguage};

pub type KotlinSyntaxNode<'source> = SyntaxNode<'source, KotlinLanguage>;
pub type KotlinSyntaxToken<'source> = SyntaxToken<'source, KotlinLanguage>;
pub type KotlinSyntaxVerbatimCore<'source> = SyntaxVerbatimCore<'source, KotlinLanguage>;

/// A fixed Kotlin syntax slot did not contain the element declared by the schema.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KotlinSyntaxInvariantError {
    pub node: KotlinSyntaxKind,
    pub slot: usize,
}

impl fmt::Display for KotlinSyntaxInvariantError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{:?} has an invalid element in slot {}",
            self.node, self.slot
        )
    }
}

impl std::error::Error for KotlinSyntaxInvariantError {}

type KotlinSyntaxResult<T> = Result<T, KotlinSyntaxInvariantError>;

/// A declared grammar role, including represented malformed alternatives.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KotlinSyntaxField<'source, T> {
    Present(T),
    Missing(KotlinMissingSyntax<'source>),
    Malformed(KotlinMalformedSyntax<'source>),
}

impl<'source, T> KotlinSyntaxField<'source, T> {
    pub fn as_ref(&self) -> KotlinSyntaxField<'source, &T> {
        match self {
            Self::Present(value) => KotlinSyntaxField::Present(value),
            Self::Missing(missing) => KotlinSyntaxField::Missing(*missing),
            Self::Malformed(node) => KotlinSyntaxField::Malformed(*node),
        }
    }

    pub fn map<U>(self, map: impl FnOnce(T) -> U) -> KotlinSyntaxField<'source, U> {
        match self {
            Self::Present(value) => KotlinSyntaxField::Present(map(value)),
            Self::Missing(missing) => KotlinSyntaxField::Missing(missing),
            Self::Malformed(node) => KotlinSyntaxField::Malformed(node),
        }
    }
}

/// A syntax-owned malformed node occupying a declared role.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KotlinMalformedSyntax<'source> {
    syntax: KotlinSyntaxNode<'source>,
}

/// Syntax-owned evidence for one represented empty required or optional slot.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KotlinMissingSyntax<'source> {
    owner: KotlinSyntaxNode<'source>,
    slot: usize,
}

impl<'source> KotlinMissingSyntax<'source> {
    /// Returns the exact zero-width source boundary represented by this missing slot.
    ///
    /// # Errors
    ///
    /// Returns an invariant error if the slot is not a represented missing boundary.
    pub fn verbatim_core(
        self,
    ) -> Result<SyntaxVerbatimCore<'source, KotlinLanguage>, KotlinSyntaxInvariantError> {
        let core = self.owner.missing_verbatim_core(self.slot);
        core.ok_or(KotlinSyntaxInvariantError {
            node: self.owner.kind(),
            slot: self.slot,
        })
    }
}

#[derive(Clone, Copy)]
struct KotlinFixedSyntax<'source>(KotlinSyntaxNode<'source>);

impl<'source> KotlinFixedSyntax<'source> {
    #[inline]
    fn kind(self) -> KotlinSyntaxKind {
        self.0.kind()
    }

    #[inline]
    fn slot_at(self, slot: usize) -> Option<SyntaxSlot<'source, KotlinLanguage>> {
        self.0.slot_at(slot)
    }

    #[inline]
    fn missing_owner(self) -> KotlinSyntaxNode<'source> {
        self.0
    }

    #[inline]
    fn text_range(self) -> TextRange {
        self.0.text_range()
    }

    #[inline]
    fn source(self) -> &'source str {
        self.0.source()
    }
}

#[inline]
fn required_slot(
    syntax: KotlinFixedSyntax<'_>,
    slot: usize,
) -> KotlinSyntaxResult<KotlinSyntaxField<'_, SyntaxElement<'_, KotlinLanguage>>> {
    match syntax.slot_at(slot) {
        Some(SyntaxSlot::Node(node)) if node.is_directly_malformed() => {
            Ok(KotlinSyntaxField::Malformed(KotlinMalformedSyntax {
                syntax: node,
            }))
        }
        Some(SyntaxSlot::Node(node)) => Ok(KotlinSyntaxField::Present(SyntaxElement::Node(node))),
        Some(SyntaxSlot::Token(token)) => {
            Ok(KotlinSyntaxField::Present(SyntaxElement::Token(token)))
        }
        None => Err(KotlinSyntaxInvariantError {
            node: syntax.kind(),
            slot,
        }),
        Some(SyntaxSlot::Empty) => Ok(KotlinSyntaxField::Missing(KotlinMissingSyntax {
            owner: syntax.missing_owner(),
            slot,
        })),
    }
}

#[inline]
fn optional_slot(
    syntax: KotlinFixedSyntax<'_>,
    slot: usize,
) -> KotlinSyntaxResult<KotlinSyntaxField<'_, SyntaxElement<'_, KotlinLanguage>>> {
    match syntax.slot_at(slot) {
        Some(SyntaxSlot::Node(node)) if node.is_directly_malformed() => {
            Ok(KotlinSyntaxField::Malformed(KotlinMalformedSyntax {
                syntax: node,
            }))
        }
        Some(SyntaxSlot::Node(node)) => Ok(KotlinSyntaxField::Present(SyntaxElement::Node(node))),
        Some(SyntaxSlot::Token(token)) => {
            Ok(KotlinSyntaxField::Present(SyntaxElement::Token(token)))
        }
        None => Err(KotlinSyntaxInvariantError {
            node: syntax.kind(),
            slot,
        }),
        Some(SyntaxSlot::Empty) => Ok(KotlinSyntaxField::Missing(KotlinMissingSyntax {
            owner: syntax.missing_owner(),
            slot,
        })),
    }
}

fn token_iter<'source, 'node>(
    syntax: &'node KotlinSyntaxNode<'source>,
) -> impl Iterator<Item = KotlinSyntaxToken<'source>> + 'node
where
    'source: 'node,
{
    syntax.tokens()
}

fn first_token<'source>(syntax: &KotlinSyntaxNode<'source>) -> Option<KotlinSyntaxToken<'source>> {
    syntax.first_token()
}

fn last_token<'source>(syntax: &KotlinSyntaxNode<'source>) -> Option<KotlinSyntaxToken<'source>> {
    syntax.last_token()
}

mod private {
    pub trait Sealed {}
}

/// Sealed access to behavior shared by every typed Kotlin syntax view.
pub trait KotlinSyntaxView<'source>: private::Sealed {
    /// Returns the ordinary physical syntax node backing this view.
    fn syntax_node(&self) -> Option<KotlinSyntaxNode<'source>>;

    /// Returns the first token represented by this view.
    fn first_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        self.syntax_node().and_then(|syntax| syntax.first_token())
    }

    #[must_use]
    fn is_malformed(&self) -> bool {
        self.syntax_node()
            .is_some_and(|syntax| syntax.is_directly_malformed())
    }

    /// Whether this represented subtree contains no missing or malformed
    /// recovery syntax.
    #[must_use]
    fn is_recovery_free(&self) -> bool {
        self.syntax_node()
            .is_none_or(|syntax| syntax.is_recovery_free())
    }

    #[must_use]
    fn malformed_verbatim_core(&self) -> Option<SyntaxVerbatimCore<'source, KotlinLanguage>> {
        self.syntax_node()
            .and_then(|syntax| syntax.malformed_verbatim_core())
    }

    /// Whether trivia before this view's first token contains a blank line.
    #[must_use]
    fn starts_after_blank_line(&self) -> bool {
        self.first_token()
            .is_some_and(|token| token.has_leading_blank_line())
    }
}

impl private::Sealed for KotlinMalformedSyntax<'_> {}

impl<'source> KotlinSyntaxView<'source> for KotlinMalformedSyntax<'source> {
    fn syntax_node(&self) -> Option<KotlinSyntaxNode<'source>> {
        Some(self.syntax)
    }
}

pub trait KotlinTypedNode<'source>: Clone + private::Sealed {
    #[doc(hidden)]
    fn cast_element(element: KotlinRoleElement<'source>) -> Option<Self>;
}

pub trait KotlinNode<'source>: KotlinTypedNode<'source> {
    fn cast(syntax: KotlinSyntaxNode<'source>) -> Option<Self>;
}

pub trait KotlinFamily<'source>: Clone + private::Sealed {
    fn cast(syntax: KotlinSyntaxNode<'source>) -> Option<Self>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KotlinRoleElement<'source> {
    Node(KotlinSyntaxNode<'source>),
    Token(KotlinSyntaxToken<'source>),
}

impl<'source> KotlinRoleElement<'source> {
    #[must_use]
    pub fn token(self) -> Option<KotlinSyntaxToken<'source>> {
        match self {
            Self::Token(token) => Some(token),
            Self::Node(_) => None,
        }
    }

    #[must_use]
    pub fn cast_node<N: KotlinTypedNode<'source>>(self) -> Option<N> {
        N::cast_element(self)
    }

    #[must_use]
    pub fn cast_family<F: KotlinFamily<'source>>(self) -> Option<F> {
        match self {
            Self::Node(node) => F::cast(node),
            Self::Token(_) => None,
        }
    }
}

trait KotlinListItem<'source>: Sized {
    fn cast_element(element: KotlinRoleElement<'source>) -> Option<Self>;
}

impl<'source> KotlinListItem<'source> for KotlinRoleElement<'source> {
    fn cast_element(element: KotlinRoleElement<'source>) -> Option<Self> {
        Some(element)
    }
}

/// One represented part of a variable-length Kotlin syntax-list node.
#[derive(Clone, Copy, Debug)]
pub enum KotlinSyntaxListPart<'source, T> {
    Item(T),
    Separator(KotlinSyntaxToken<'source>),
    Missing(KotlinMissingSyntax<'source>),
    Malformed(KotlinMalformedSyntax<'source>),
}

fn list_parts<'source, T: KotlinListItem<'source>>(
    syntax: KotlinSyntaxNode<'source>,
    separated: bool,
) -> impl Iterator<Item = KotlinSyntaxResult<KotlinSyntaxListPart<'source, T>>> + use<'source, T> {
    (0..syntax.slot_count()).map(move |index| {
        let Some(slot) = syntax.slot_at(index) else {
            return Err(KotlinSyntaxInvariantError {
                node: syntax.kind(),
                slot: index,
            });
        };
        match slot {
            SyntaxSlot::Node(node) if node.is_directly_malformed() => {
                Ok(KotlinSyntaxListPart::Malformed(KotlinMalformedSyntax {
                    syntax: node,
                }))
            }
            SyntaxSlot::Token(token) if separated && index % 2 == 1 => {
                Ok(KotlinSyntaxListPart::Separator(token))
            }
            SyntaxSlot::Node(node) => T::cast_element(KotlinRoleElement::Node(node))
                .map(KotlinSyntaxListPart::Item)
                .ok_or(KotlinSyntaxInvariantError {
                    node: syntax.kind(),
                    slot: index,
                }),
            SyntaxSlot::Token(token) => T::cast_element(KotlinRoleElement::Token(token))
                .map(KotlinSyntaxListPart::Item)
                .ok_or(KotlinSyntaxInvariantError {
                    node: syntax.kind(),
                    slot: index,
                }),
            SyntaxSlot::Empty => Ok(KotlinSyntaxListPart::Missing(KotlinMissingSyntax {
                owner: syntax,
                slot: index,
            })),
        }
    })
}

#[inline]
fn required_token(
    syntax: KotlinFixedSyntax<'_>,
    slot: usize,
) -> KotlinSyntaxResult<KotlinSyntaxField<'_, KotlinSyntaxToken<'_>>> {
    match required_slot(syntax, slot)? {
        KotlinSyntaxField::Present(SyntaxElement::Token(token)) => {
            Ok(KotlinSyntaxField::Present(token))
        }
        KotlinSyntaxField::Missing(missing) => Ok(KotlinSyntaxField::Missing(missing)),
        KotlinSyntaxField::Malformed(node) => Ok(KotlinSyntaxField::Malformed(node)),
        KotlinSyntaxField::Present(SyntaxElement::Node(_)) => Err(KotlinSyntaxInvariantError {
            node: syntax.kind(),
            slot,
        }),
    }
}

#[inline]
fn optional_token(
    syntax: KotlinFixedSyntax<'_>,
    slot: usize,
) -> KotlinSyntaxResult<KotlinSyntaxField<'_, KotlinSyntaxToken<'_>>> {
    match optional_slot(syntax, slot)? {
        KotlinSyntaxField::Present(SyntaxElement::Token(token)) => {
            Ok(KotlinSyntaxField::Present(token))
        }
        KotlinSyntaxField::Missing(missing) => Ok(KotlinSyntaxField::Missing(missing)),
        KotlinSyntaxField::Malformed(node) => Ok(KotlinSyntaxField::Malformed(node)),
        KotlinSyntaxField::Present(SyntaxElement::Node(_)) => Err(KotlinSyntaxInvariantError {
            node: syntax.kind(),
            slot,
        }),
    }
}

#[inline]
fn required_node<'source, N: KotlinNode<'source>>(
    syntax: KotlinFixedSyntax<'source>,
    slot: usize,
) -> KotlinSyntaxResult<KotlinSyntaxField<'source, N>> {
    match required_slot(syntax, slot)? {
        KotlinSyntaxField::Present(SyntaxElement::Node(node)) => N::cast(node)
            .map(KotlinSyntaxField::Present)
            .ok_or(KotlinSyntaxInvariantError {
                node: syntax.kind(),
                slot,
            }),
        KotlinSyntaxField::Missing(missing) => Ok(KotlinSyntaxField::Missing(missing)),
        KotlinSyntaxField::Malformed(node) => Ok(KotlinSyntaxField::Malformed(node)),
        KotlinSyntaxField::Present(SyntaxElement::Token(_)) => Err(KotlinSyntaxInvariantError {
            node: syntax.kind(),
            slot,
        }),
    }
}

#[inline]
fn optional_node<'source, N: KotlinNode<'source>>(
    syntax: KotlinFixedSyntax<'source>,
    slot: usize,
) -> KotlinSyntaxResult<KotlinSyntaxField<'source, N>> {
    match optional_slot(syntax, slot)? {
        KotlinSyntaxField::Present(SyntaxElement::Node(node)) => N::cast(node)
            .map(KotlinSyntaxField::Present)
            .ok_or(KotlinSyntaxInvariantError {
                node: syntax.kind(),
                slot,
            }),
        KotlinSyntaxField::Missing(missing) => Ok(KotlinSyntaxField::Missing(missing)),
        KotlinSyntaxField::Malformed(node) => Ok(KotlinSyntaxField::Malformed(node)),
        KotlinSyntaxField::Present(SyntaxElement::Token(_)) => Err(KotlinSyntaxInvariantError {
            node: syntax.kind(),
            slot,
        }),
    }
}

#[inline]
fn required_role_element(
    syntax: KotlinFixedSyntax<'_>,
    slot: usize,
) -> KotlinSyntaxResult<KotlinSyntaxField<'_, KotlinRoleElement<'_>>> {
    match syntax.slot_at(slot) {
        Some(SyntaxSlot::Node(node)) if node.is_directly_malformed() => {
            Ok(KotlinSyntaxField::Malformed(KotlinMalformedSyntax {
                syntax: node,
            }))
        }
        Some(SyntaxSlot::Node(node)) => {
            Ok(KotlinSyntaxField::Present(KotlinRoleElement::Node(node)))
        }
        Some(SyntaxSlot::Token(token)) => {
            Ok(KotlinSyntaxField::Present(KotlinRoleElement::Token(token)))
        }
        Some(SyntaxSlot::Empty) => Ok(KotlinSyntaxField::Missing(KotlinMissingSyntax {
            owner: syntax.missing_owner(),
            slot,
        })),
        None => Err(KotlinSyntaxInvariantError {
            node: syntax.kind(),
            slot,
        }),
    }
}

#[inline]
fn required_family<'source, F: KotlinFamily<'source>>(
    syntax: KotlinFixedSyntax<'source>,
    slot: usize,
) -> KotlinSyntaxResult<KotlinSyntaxField<'source, F>> {
    match required_slot(syntax, slot)? {
        KotlinSyntaxField::Present(SyntaxElement::Node(node)) => F::cast(node)
            .map(KotlinSyntaxField::Present)
            .ok_or(KotlinSyntaxInvariantError {
                node: syntax.kind(),
                slot,
            }),
        KotlinSyntaxField::Missing(missing) => Ok(KotlinSyntaxField::Missing(missing)),
        KotlinSyntaxField::Malformed(node) => Ok(KotlinSyntaxField::Malformed(node)),
        KotlinSyntaxField::Present(SyntaxElement::Token(_)) => Err(KotlinSyntaxInvariantError {
            node: syntax.kind(),
            slot,
        }),
    }
}

#[inline]
fn optional_family<'source, F: KotlinFamily<'source>>(
    syntax: KotlinFixedSyntax<'source>,
    slot: usize,
) -> KotlinSyntaxResult<KotlinSyntaxField<'source, F>> {
    match optional_slot(syntax, slot)? {
        KotlinSyntaxField::Present(SyntaxElement::Node(node)) => F::cast(node)
            .map(KotlinSyntaxField::Present)
            .ok_or(KotlinSyntaxInvariantError {
                node: syntax.kind(),
                slot,
            }),
        KotlinSyntaxField::Missing(missing) => Ok(KotlinSyntaxField::Missing(missing)),
        KotlinSyntaxField::Malformed(node) => Ok(KotlinSyntaxField::Malformed(node)),
        KotlinSyntaxField::Present(SyntaxElement::Token(_)) => Err(KotlinSyntaxInvariantError {
            node: syntax.kind(),
            slot,
        }),
    }
}

fn syntax_source_text(syntax: KotlinFixedSyntax<'_>) -> &str {
    let range = syntax.text_range();
    &syntax.source()[range.start().get()..range.end().get()]
}

macro_rules! kotlin_field_accessor {
    ($module:ident $field:ident required $matcher:tt => $role:ident) => {
        #[inline]
        #[allow(clippy::missing_errors_doc)]
        pub fn $field(&self) -> KotlinSyntaxResult<KotlinSyntaxField<'source, $role<'source>>> {
            required_role_element(self.fixed_syntax(), crate::shape::$module::Slot::$field as usize).map(|slot| {
                slot.map(|element| $role { element })
            })
        }
    };
    ($module:ident $field:ident optional $matcher:tt => $role:ident) => {
        #[inline]
        #[allow(clippy::missing_errors_doc)]
        pub fn $field(&self) -> KotlinSyntaxResult<KotlinSyntaxField<'source, $role<'source>>> {
            required_role_element(self.fixed_syntax(), crate::shape::$module::Slot::$field as usize)
                .map(|slot| slot.map(|element| $role { element }))
        }
    };
    ($module:ident $field:ident required (token $kind:ident)) => {
        #[inline]
        #[allow(clippy::missing_errors_doc)]
        pub fn $field(&self) -> KotlinSyntaxResult<KotlinSyntaxField<'source, KotlinSyntaxToken<'source>>> {
            required_token(self.fixed_syntax(), crate::shape::$module::Slot::$field as usize)
        }
    };
    ($module:ident $field:ident required (token_set $kinds:tt)) => {
        kotlin_field_accessor!($module $field required (token __schema_token_set));
    };
    ($module:ident $field:ident required (contextual $text:literal)) => {
        kotlin_field_accessor!($module $field required (token __schema_contextual));
    };
    ($module:ident $field:ident optional (token $kind:ident)) => {
        #[inline]
        #[allow(clippy::missing_errors_doc)]
        pub fn $field(&self) -> KotlinSyntaxResult<KotlinSyntaxField<'source, KotlinSyntaxToken<'source>>> {
            optional_token(self.fixed_syntax(), crate::shape::$module::Slot::$field as usize)
        }
    };
    ($module:ident $field:ident optional (token_set $kinds:tt)) => {
        kotlin_field_accessor!($module $field optional (token __schema_token_set));
    };
    ($module:ident $field:ident optional (contextual $text:literal)) => {
        kotlin_field_accessor!($module $field optional (token __schema_contextual));
    };

    ($module:ident $field:ident required (node ModuleDirective)) => {
        #[inline]
        #[allow(clippy::missing_errors_doc)]
        pub fn $field(&self) -> KotlinSyntaxResult<KotlinSyntaxField<'source, ModuleDirectiveNode<'source>>> {
            required_node(self.fixed_syntax(), crate::shape::$module::Slot::$field as usize)
        }
    };
    ($module:ident $field:ident optional (node ModuleDirective)) => {
        #[inline]
        #[allow(clippy::missing_errors_doc)]
        pub fn $field(&self) -> KotlinSyntaxResult<KotlinSyntaxField<'source, ModuleDirectiveNode<'source>>> {
            optional_node(self.fixed_syntax(), crate::shape::$module::Slot::$field as usize)
        }
    };
    ($module:ident $field:ident required (node $target:ident)) => {
        #[inline]
        #[allow(clippy::missing_errors_doc)]
        pub fn $field(&self) -> KotlinSyntaxResult<KotlinSyntaxField<'source, $target<'source>>> {
            required_node(self.fixed_syntax(), crate::shape::$module::Slot::$field as usize)
        }
    };
    ($module:ident $field:ident optional (node $target:ident)) => {
        #[inline]
        #[allow(clippy::missing_errors_doc)]
        pub fn $field(&self) -> KotlinSyntaxResult<KotlinSyntaxField<'source, $target<'source>>> {
            optional_node(self.fixed_syntax(), crate::shape::$module::Slot::$field as usize)
        }
    };
    ($module:ident $field:ident required (constructed $target:ident)) => {
        #[inline]
        #[allow(clippy::missing_errors_doc)]
        pub fn $field(&self) -> KotlinSyntaxResult<KotlinSyntaxField<'source, $target<'source>>> {
            required_node(self.fixed_syntax(), crate::shape::$module::Slot::$field as usize)
        }
    };
    ($module:ident $field:ident optional (constructed $target:ident)) => {
        #[inline]
        #[allow(clippy::missing_errors_doc)]
        pub fn $field(&self) -> KotlinSyntaxResult<KotlinSyntaxField<'source, $target<'source>>> {
            optional_node(self.fixed_syntax(), crate::shape::$module::Slot::$field as usize)
        }
    };
    ($module:ident $field:ident required (list $target:ident)) => {
        #[inline]
        #[allow(clippy::missing_errors_doc)]
        pub fn $field(&self) -> KotlinSyntaxResult<KotlinSyntaxField<'source, $target<'source>>> {
            required_node(self.fixed_syntax(), crate::shape::$module::Slot::$field as usize)
        }
    };
    ($module:ident $field:ident optional (list $target:ident)) => {
        #[inline]
        #[allow(clippy::missing_errors_doc)]
        pub fn $field(&self) -> KotlinSyntaxResult<KotlinSyntaxField<'source, $target<'source>>> {
            optional_node(self.fixed_syntax(), crate::shape::$module::Slot::$field as usize)
        }
    };
    ($module:ident $field:ident required (category $target:ident)) => {
        #[inline]
        #[allow(clippy::missing_errors_doc)]
        pub fn $field(&self) -> KotlinSyntaxResult<KotlinSyntaxField<'source, $target<'source>>> {
            required_family(self.fixed_syntax(), crate::shape::$module::Slot::$field as usize)
        }
    };
    ($module:ident $field:ident optional (category $target:ident)) => {
        #[inline]
        #[allow(clippy::missing_errors_doc)]
        pub fn $field(&self) -> KotlinSyntaxResult<KotlinSyntaxField<'source, $target<'source>>> {
            optional_family(self.fixed_syntax(), crate::shape::$module::Slot::$field as usize)
        }
    };

    // Heterogeneous roles are wrapped by the semantic adapters below. This
    // primitive still reads exactly one declared slot and never searches.
    ($module:ident $field:ident required $matcher:tt) => {
        #[inline]
        #[allow(clippy::missing_errors_doc)]
        pub fn $field(&self) -> KotlinSyntaxResult<KotlinSyntaxField<'source, KotlinRoleElement<'source>>> {
            required_role_element(self.fixed_syntax(), crate::shape::$module::Slot::$field as usize)
        }
    };
    ($module:ident $field:ident optional $matcher:tt) => {
        #[inline]
        #[allow(clippy::missing_errors_doc)]
        pub fn $field(&self) -> KotlinSyntaxResult<KotlinSyntaxField<'source, KotlinRoleElement<'source>>> {
            required_role_element(self.fixed_syntax(), crate::shape::$module::Slot::$field as usize)
        }
    };
    ($module:ident $field:ident many $matcher:tt $(=> $role:ident)?) => {};
    ($module:ident $field:ident one_or_more $matcher:tt $(=> $role:ident)?) => {};
}

#[allow(unused_macros)]
macro_rules! define_kotlin_role {
    ($role:ident) => {
        #[derive(Clone, Copy, Debug)]
        pub struct $role<'source> {
            element: KotlinRoleElement<'source>,
        }

        impl<'source> $role<'source> {
            #[must_use]
            pub fn token(self) -> Option<KotlinSyntaxToken<'source>> {
                match self.element {
                    KotlinRoleElement::Token(token) => Some(token),
                    KotlinRoleElement::Node(_) => None,
                }
            }

            #[must_use]
            pub fn first_token(self) -> Option<KotlinSyntaxToken<'source>> {
                match self.element {
                    KotlinRoleElement::Node(node) => node.first_token(),
                    KotlinRoleElement::Token(token) => Some(token),
                }
            }

            #[must_use]
            pub fn last_token(self) -> Option<KotlinSyntaxToken<'source>> {
                match self.element {
                    KotlinRoleElement::Node(node) => node.last_token(),
                    KotlinRoleElement::Token(token) => Some(token),
                }
            }

            #[must_use]
            pub fn cast_node<N: KotlinTypedNode<'source>>(self) -> Option<N> {
                N::cast_element(self.element)
            }

            #[must_use]
            pub fn cast_family<F: KotlinFamily<'source>>(self) -> Option<F> {
                match self.element {
                    KotlinRoleElement::Node(node) => F::cast(node),
                    KotlinRoleElement::Token(_) => None,
                }
            }
        }

        impl<'source> KotlinListItem<'source> for $role<'source> {
            fn cast_element(element: KotlinRoleElement<'source>) -> Option<Self> {
                Some(Self { element })
            }
        }
    };
}

macro_rules! kotlin_list_item_type {
    ($source:lifetime; $matcher:tt => $role:ident) => { $role<$source> };
    ($source:lifetime; (node ModuleDirective)) => { ModuleDirectiveNode<$source> };
    ($source:lifetime; (node $target:ident)) => { $target<$source> };
    ($source:lifetime; (constructed $target:ident)) => { $target<$source> };
    ($source:lifetime; (category $target:ident)) => { $target<$source> };
    ($source:lifetime; $matcher:tt) => { KotlinRoleElement<$source> };
}

macro_rules! kotlin_list_item_type_optional_role {
    ($source:lifetime; $matcher:tt; $role:ident) => {
        kotlin_list_item_type!($source; $matcher => $role)
    };
    ($source:lifetime; $matcher:tt;) => {
        kotlin_list_item_type!($source; $matcher)
    };
}

macro_rules! kotlin_list_is_separated {
    ([separated $($policy:tt)*]) => {
        true
    };
    ([$($policy:tt)*]) => {
        false
    };
    () => {
        false
    };
}

macro_rules! kotlin_variable_slot_view {
    (list; $field:ident: $cardinality:ident $matcher:tt $(=> $role:ident)? $([$($policy:tt)*])?;) => {
        /// Returns this list's represented elements and separators in source order.
        pub fn parts(
            &self,
        ) -> impl Iterator<
            Item = KotlinSyntaxResult<KotlinSyntaxListPart<
                'source,
                kotlin_list_item_type_optional_role!('source; $matcher; $($role)?),
            >>,
        > + '_ {
            list_parts::<kotlin_list_item_type_optional_role!('source; $matcher; $($role)?)>(
                self.syntax,
                kotlin_list_is_separated!($([$($policy)*])?),
            )
        }
    };
    ($class:ident; $($fields:tt)*) => {};
}

macro_rules! define_kotlin_cst_node {
    ($node:ident => $kind:ident [list]) => {
        define_kotlin_cst_node!($node => $kind [physical]);
    };
    ($node:ident => $kind:ident [constructed]) => {
        define_kotlin_cst_node!($node => $kind [physical]);
    };
    ($node:ident => $kind:ident [$class:ident]) => {
        #[derive(Clone, Copy, Eq, PartialEq)]
        pub struct $node<'source> {
            syntax: KotlinSyntaxNode<'source>,
        }

        impl<'source> $node<'source> {
            #[inline]
            fn fixed_syntax(&self) -> KotlinFixedSyntax<'source> {
                KotlinFixedSyntax(self.syntax)
            }
            #[must_use]
            pub fn kind(&self) -> KotlinSyntaxKind {
                self.syntax.kind()
            }

            #[must_use]
            pub fn text_range(&self) -> TextRange {
                self.syntax.text_range()
            }

            #[must_use]
            pub fn source_text(&self) -> &'source str {
                syntax_source_text(self.fixed_syntax())
            }

            pub fn token_iter(&self) -> impl Iterator<Item = KotlinSyntaxToken<'source>> + '_ {
                token_iter(&self.syntax)
            }

            #[must_use]
            pub fn first_token(&self) -> Option<KotlinSyntaxToken<'source>> {
                first_token(&self.syntax)
            }

            #[must_use]
            pub fn last_token(&self) -> Option<KotlinSyntaxToken<'source>> {
                last_token(&self.syntax)
            }

        }

        impl private::Sealed for $node<'_> {}

        impl<'source> KotlinSyntaxView<'source> for $node<'source> {
            fn syntax_node(&self) -> Option<KotlinSyntaxNode<'source>> {
                Some(self.syntax)
            }
        }

        impl<'source> KotlinNode<'source> for $node<'source> {
            fn cast(syntax: KotlinSyntaxNode<'source>) -> Option<Self> {
                matches!(syntax.kind(), KotlinSyntaxKind::$kind).then_some(Self { syntax })
            }
        }

        impl<'source> KotlinTypedNode<'source> for $node<'source> {
            fn cast_element(element: KotlinRoleElement<'source>) -> Option<Self> {
                match element {
                    KotlinRoleElement::Node(node) => Self::cast(node),
                    KotlinRoleElement::Token(_) => None,
                }
            }
        }

        impl<'source> KotlinListItem<'source> for $node<'source> {
            fn cast_element(element: KotlinRoleElement<'source>) -> Option<Self> {
                <Self as KotlinTypedNode<'source>>::cast_element(element)
            }
        }

        impl fmt::Debug for $node<'_> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.syntax.fmt(f)
            }
        }
    };
}

macro_rules! kotlin_cst {
    (
        nodes {
            $($node:ident => $kind:ident [$class:ident],)*
        }
        enums {
            $(
                $family:ident =
                    $($variant:ident)|+;
            )*
        }
    ) => {
        $(define_kotlin_cst_node!($node => $kind [$class]);)*

        #[derive(Clone, Copy, Debug, Eq, PartialEq)]
        pub enum AnyKotlinNode<'source> {
            $($node($node<'source>),)*
        }

        impl<'source> AnyKotlinNode<'source> {
            #[must_use]
            pub fn cast(syntax: KotlinSyntaxNode<'source>) -> Option<Self> {
                match syntax.kind() {
                    $(KotlinSyntaxKind::$kind => {
                        <$node<'source> as KotlinNode<'source>>::cast(syntax).map(Self::$node)
                    })*
                    _ => None,
                }
            }
        }

        $(
            impl<'source> From<$node<'source>> for AnyKotlinNode<'source> {
                fn from(node: $node<'source>) -> Self {
                    Self::$node(node)
                }
            }
        )*

        $(
            #[derive(Clone, Copy, Debug, Eq, PartialEq)]
            pub enum $family<'source> {
                $($variant($variant<'source>),)+
            }

            impl<'source> $family<'source> {
                #[must_use]
                pub fn kind(&self) -> KotlinSyntaxKind {
                    self.syntax().kind()
                }

                #[must_use]
                pub fn text_range(&self) -> TextRange {
                    self.syntax().text_range()
                }

                #[must_use]
                pub fn source_text(&self) -> &'source str {
                    syntax_source_text(KotlinFixedSyntax(*self.syntax()))
                }

                pub fn token_iter(&self) -> impl Iterator<Item = KotlinSyntaxToken<'source>> + '_ {
                    token_iter(self.syntax())
                }

                #[must_use]
                pub fn first_token(&self) -> Option<KotlinSyntaxToken<'source>> {
                    first_token(self.syntax())
                }

                #[must_use]
                pub fn last_token(&self) -> Option<KotlinSyntaxToken<'source>> {
                    last_token(self.syntax())
                }

                pub(crate) fn syntax(&self) -> &KotlinSyntaxNode<'source> {
                    match self {
                        $(Self::$variant(node) => &node.syntax,)+
                    }
                }
            }

            impl<'source> KotlinFamily<'source> for $family<'source> {
                fn cast(syntax: KotlinSyntaxNode<'source>) -> Option<Self> {
                    match syntax.kind() {
                        $(
                            KotlinSyntaxKind::$variant => {
                                <$variant<'source> as KotlinNode<'source>>::cast(syntax).map(Self::$variant)
                            }
                        )+
                        _ => None,
                    }
                }
            }

            impl<'source> KotlinListItem<'source> for $family<'source> {
                fn cast_element(element: KotlinRoleElement<'source>) -> Option<Self> {
                    match element {
                        KotlinRoleElement::Node(node) => Self::cast(node),
                        KotlinRoleElement::Token(_) => None,
                    }
                }
            }

            impl private::Sealed for $family<'_> {}

            impl<'source> KotlinSyntaxView<'source> for $family<'source> {
                fn syntax_node(&self) -> Option<KotlinSyntaxNode<'source>> {
                    Some(*self.syntax())
                }
            }

            $(
                impl<'source> From<$variant<'source>> for $family<'source> {
                    fn from(node: $variant<'source>) -> Self {
                        Self::$variant(node)
                    }
                }
            )+
        )*

    };
}

macro_rules! define_kotlin_cst_from_schema {
    (
        tokens { $($token:ident,)* }
        categories { $($family:ident => $bogus:ident { $($member:ident,)* })* }
        nodes { $($kind:ident => $wrapper:ident [$module:ident $class:ident] { $($fields:tt)* })* }
    ) => {
        kotlin_cst! {
            nodes {
                $($wrapper => $kind [$class],)*
                $($bogus => $bogus [malformed],)*
            }
            enums {
                $($family = $($member)|+|$bogus;)*
            }
        }
    };
}

kotlin_syntax_schema!(define_kotlin_cst_from_schema);

macro_rules! define_kotlin_accessors_from_schema {
    (
        tokens { $($token:ident,)* }
        categories { $($family:ident => $bogus:ident { $($member:ident,)* })* }
        nodes {
            $(
                $kind:ident => $wrapper:ident [$module:ident $class:ident] {
                    $(
                        $field:ident: $cardinality:ident $matcher:tt
                        $(=> $role:ident)?
                        $([$($policy:tt)*])?;
                    )*
                }
            )*
        }
    ) => {
        $($( $(define_kotlin_role!($role);)? )*)*
        $(
            impl<'source> $wrapper<'source> {
                $(kotlin_field_accessor!($module $field $cardinality $matcher $(=> $role)?);)*
                kotlin_variable_slot_view!(
                    $class;
                    $($field: $cardinality $matcher $(=> $role)? $([$($policy)*])?;)*
                );
            }
        )*
    };
}

kotlin_syntax_schema!(define_kotlin_accessors_from_schema);

impl Block<'_> {
    /// Returns whether the represented block interior contains only trivia.
    #[must_use]
    pub fn inner_is_whitespace(&self) -> bool {
        matches!(self.open_brace(), Ok(KotlinSyntaxField::Present(_)))
            && matches!(self.close_brace(), Ok(KotlinSyntaxField::Present(_)))
            && matches!(
                self.items(),
                Ok(KotlinSyntaxField::Present(items)) if items.first_token().is_none()
            )
    }
}

pub(crate) fn cast_kotlin_file(syntax: KotlinSyntaxNode<'_>) -> Option<KotlinFile<'_>> {
    <KotlinFile<'_> as KotlinNode<'_>>::cast(syntax)
}
