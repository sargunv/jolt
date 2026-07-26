//! Language-neutral typed CST field representation.
//!
//! These types are ordinary generics rather than macro output so they carry
//! rustdoc and resolve in an editor, matching how [`SyntaxNode`] and
//! [`SyntaxToken`] are already modelled. Language crates alias them to their
//! own names.

use std::fmt;

use crate::red::SyntaxNode;
use crate::{Language, SyntaxVerbatimCore};

/// A represented node whose declared slot holds an element the schema forbids.
pub struct SyntaxInvariantError<L: Language> {
    pub node: L::Kind,
    pub slot: usize,
}

impl<L: Language> fmt::Display for SyntaxInvariantError<L> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{:?} has an invalid element in slot {}",
            self.node, self.slot
        )
    }
}

impl<L: Language> std::error::Error for SyntaxInvariantError<L> {}

/// A declared grammar role, including represented malformed alternatives.
pub enum SyntaxField<'source, L: Language, T> {
    Present(T),
    Missing(MissingSyntax<'source, L>),
    Malformed(MalformedSyntax<'source, L>),
}

impl<'source, L: Language, T> SyntaxField<'source, L, T> {
    pub fn as_ref(&self) -> SyntaxField<'source, L, &T> {
        match self {
            Self::Present(value) => SyntaxField::Present(value),
            Self::Missing(missing) => SyntaxField::Missing(*missing),
            Self::Malformed(node) => SyntaxField::Malformed(*node),
        }
    }

    pub fn map<U>(self, map: impl FnOnce(T) -> U) -> SyntaxField<'source, L, U> {
        match self {
            Self::Present(value) => SyntaxField::Present(map(value)),
            Self::Missing(missing) => SyntaxField::Missing(missing),
            Self::Malformed(node) => SyntaxField::Malformed(node),
        }
    }
}

/// A syntax-owned malformed node occupying a declared role.
pub struct MalformedSyntax<'source, L: Language> {
    syntax: SyntaxNode<'source, L>,
}

impl<'source, L: Language> MalformedSyntax<'source, L> {
    /// Wraps a node the caller has already classified as directly malformed.
    #[doc(hidden)]
    #[must_use]
    pub const fn new(syntax: SyntaxNode<'source, L>) -> Self {
        Self { syntax }
    }

    #[must_use]
    pub const fn syntax(self) -> SyntaxNode<'source, L> {
        self.syntax
    }
}

/// Syntax-owned evidence for one represented empty required or optional slot.
pub struct MissingSyntax<'source, L: Language> {
    owner: SyntaxNode<'source, L>,
    slot: usize,
}

impl<'source, L: Language> MissingSyntax<'source, L> {
    /// Records the owning node and slot the caller has already read as empty.
    #[doc(hidden)]
    #[must_use]
    pub const fn new(owner: SyntaxNode<'source, L>, slot: usize) -> Self {
        Self { owner, slot }
    }

    /// Returns the exact zero-width source boundary represented by this slot.
    ///
    /// # Errors
    ///
    /// Returns an invariant error if the owning node does not represent an
    /// empty boundary at this slot.
    pub fn verbatim_core(self) -> Result<SyntaxVerbatimCore<'source, L>, SyntaxInvariantError<L>> {
        self.owner
            .missing_verbatim_core(self.slot)
            .ok_or(SyntaxInvariantError {
                node: self.owner.kind(),
                slot: self.slot,
            })
    }
}

/// One represented part of a variable-length syntax-list node.
pub enum SyntaxListPart<'source, L: Language, T> {
    Item(T),
    Separator(crate::red::SyntaxToken<'source, L>),
    Missing(MissingSyntax<'source, L>),
    Malformed(MalformedSyntax<'source, L>),
}

// `L` is a marker parameter, so these are written by hand: deriving them would
// demand `L: Clone + Copy + Debug + Eq`, which no language type satisfies.
// `SyntaxNode` and `SyntaxToken` are modelled the same way.

/// Projects the marker-free `Copy`/`Debug`/`Eq` impls for one field type.
macro_rules! marker_impls {
    (
        $name:ident $(<$param:ident>)?
        $(where $($bound_param:ident: $bound:ident),+)?;
        debug $debug:literal { $($field:ident),+ $(,)? }
    ) => {
        impl<L: Language $(, $param)?> Clone for $name<'_, L $(, $param)?>
        where $($($bound_param: Clone),+)?
        {
            fn clone(&self) -> Self {
                *self
            }
        }

        impl<L: Language $(, $param)?> Copy for $name<'_, L $(, $param)?>
        where $($($bound_param: Copy),+)? {}

        impl<L: Language $(, $param)?> fmt::Debug for $name<'_, L $(, $param)?>
        where $($($bound_param: fmt::Debug),+)?
        {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter
                    .debug_struct($debug)
                    $(.field(stringify!($field), &self.$field))+
                    .finish()
            }
        }

        impl<L: Language $(, $param)?> PartialEq for $name<'_, L $(, $param)?>
        where $($($bound_param: PartialEq),+)?
        {
            fn eq(&self, other: &Self) -> bool {
                $(self.$field == other.$field)&&+
            }
        }

        impl<L: Language $(, $param)?> Eq for $name<'_, L $(, $param)?>
        where $($($bound_param: Eq),+)? {}
    };
}

marker_impls!(MalformedSyntax; debug "MalformedSyntax" { syntax });
marker_impls!(MissingSyntax; debug "MissingSyntax" { owner, slot });

impl<L: Language> Clone for SyntaxInvariantError<L> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<L: Language> Copy for SyntaxInvariantError<L> {}

impl<L: Language> fmt::Debug for SyntaxInvariantError<L> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SyntaxInvariantError")
            .field("node", &self.node)
            .field("slot", &self.slot)
            .finish()
    }
}

impl<L: Language> PartialEq for SyntaxInvariantError<L> {
    fn eq(&self, other: &Self) -> bool {
        self.node == other.node && self.slot == other.slot
    }
}

impl<L: Language> Eq for SyntaxInvariantError<L> {}

/// Projects the marker-free impls for one field enum.
macro_rules! marker_enum_impls {
    ($name:ident; $($variant:ident),+ $(,)?) => {
        impl<L: Language, T: Clone> Clone for $name<'_, L, T> {
            fn clone(&self) -> Self {
                match self {
                    $(Self::$variant(value) => Self::$variant(value.clone())),+
                }
            }
        }

        impl<L: Language, T: Copy> Copy for $name<'_, L, T> {}

        impl<L: Language, T: fmt::Debug> fmt::Debug for $name<'_, L, T> {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                match self {
                    $(Self::$variant(value) => {
                        formatter.debug_tuple(stringify!($variant)).field(value).finish()
                    })+
                }
            }
        }

        impl<L: Language, T: PartialEq> PartialEq for $name<'_, L, T> {
            fn eq(&self, other: &Self) -> bool {
                match (self, other) {
                    $((Self::$variant(left), Self::$variant(right)) => left == right,)+
                    _ => false,
                }
            }
        }

        impl<L: Language, T: Eq> Eq for $name<'_, L, T> {}
    };
}

marker_enum_impls!(SyntaxField; Present, Missing, Malformed);
marker_enum_impls!(SyntaxListPart; Item, Separator, Missing, Malformed);
