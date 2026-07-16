use std::fmt;

pub use jolt_syntax::{
    Comment as JavaComment, CommentKind as JavaCommentKind, Comments as JavaComments,
};
use jolt_syntax::{SyntaxElement, SyntaxNode, SyntaxSlot, SyntaxToken, SyntaxVerbatimCore};
use jolt_text::TextRange;

use crate::{JavaSyntaxKind, language::JavaLanguage};

pub type JavaSyntaxNode<'source> = SyntaxNode<'source, JavaLanguage>;
pub type JavaSyntaxToken<'source> = SyntaxToken<'source, JavaLanguage>;
pub type JavaSyntaxVerbatimCore<'source> = SyntaxVerbatimCore<'source, JavaLanguage>;

/// A fixed Java syntax slot did not contain the element declared by the schema.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct JavaSyntaxInvariantError {
    pub node: JavaSyntaxKind,
    pub slot: usize,
}

impl fmt::Display for JavaSyntaxInvariantError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{:?} has an invalid element in slot {}",
            self.node, self.slot
        )
    }
}

impl std::error::Error for JavaSyntaxInvariantError {}

type JavaSyntaxResult<T> = Result<T, JavaSyntaxInvariantError>;

/// A declared grammar role, including represented malformed alternatives.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JavaSyntaxField<'source, T> {
    Present(T),
    Missing(JavaMissingSyntax<'source>),
    Malformed(JavaMalformedSyntax<'source>),
}

impl<'source, T> JavaSyntaxField<'source, T> {
    pub fn as_ref(&self) -> JavaSyntaxField<'source, &T> {
        match self {
            Self::Present(value) => JavaSyntaxField::Present(value),
            Self::Missing(missing) => JavaSyntaxField::Missing(*missing),
            Self::Malformed(node) => JavaSyntaxField::Malformed(*node),
        }
    }

    pub fn map<U>(self, map: impl FnOnce(T) -> U) -> JavaSyntaxField<'source, U> {
        match self {
            Self::Present(value) => JavaSyntaxField::Present(map(value)),
            Self::Missing(missing) => JavaSyntaxField::Missing(missing),
            Self::Malformed(node) => JavaSyntaxField::Malformed(node),
        }
    }
}

/// A syntax-owned malformed node occupying a declared role.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct JavaMalformedSyntax<'source> {
    syntax: JavaSyntaxNode<'source>,
}

/// Syntax-owned evidence for one represented empty required or optional slot.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct JavaMissingSyntax<'source> {
    owner: JavaSyntaxNode<'source>,
    slot: usize,
}

impl<'source> JavaMissingSyntax<'source> {
    /// Returns the exact zero-width source boundary represented by this missing slot.
    ///
    /// # Errors
    ///
    /// Returns an invariant error if the slot is not a represented missing boundary.
    pub fn verbatim_core(
        self,
    ) -> Result<SyntaxVerbatimCore<'source, JavaLanguage>, JavaSyntaxInvariantError> {
        let core = self.owner.missing_verbatim_core(self.slot);
        core.ok_or(JavaSyntaxInvariantError {
            node: self.owner.kind(),
            slot: self.slot,
        })
    }
}

#[derive(Clone, Copy)]
struct JavaFixedSyntax<'source>(JavaSyntaxNode<'source>);

impl<'source> JavaFixedSyntax<'source> {
    #[inline]
    fn kind(self) -> JavaSyntaxKind {
        self.0.kind()
    }

    #[inline]
    fn slot_at(self, slot: usize) -> Option<SyntaxSlot<'source, JavaLanguage>> {
        self.0.slot_at(slot)
    }

    #[inline]
    fn missing_owner(self) -> JavaSyntaxNode<'source> {
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
    syntax: JavaFixedSyntax<'_>,
    slot: usize,
) -> JavaSyntaxResult<JavaSyntaxField<'_, SyntaxElement<'_, JavaLanguage>>> {
    match syntax.slot_at(slot) {
        Some(SyntaxSlot::Node(node)) if node.is_directly_malformed() => {
            Ok(JavaSyntaxField::Malformed(JavaMalformedSyntax {
                syntax: node,
            }))
        }
        Some(SyntaxSlot::Node(node)) => Ok(JavaSyntaxField::Present(SyntaxElement::Node(node))),
        Some(SyntaxSlot::Token(token)) => Ok(JavaSyntaxField::Present(SyntaxElement::Token(token))),
        None => Err(JavaSyntaxInvariantError {
            node: syntax.kind(),
            slot,
        }),
        Some(SyntaxSlot::Empty) => Ok(JavaSyntaxField::Missing(JavaMissingSyntax {
            owner: syntax.missing_owner(),
            slot,
        })),
    }
}

#[inline]
fn optional_slot(
    syntax: JavaFixedSyntax<'_>,
    slot: usize,
) -> JavaSyntaxResult<JavaSyntaxField<'_, SyntaxElement<'_, JavaLanguage>>> {
    match syntax.slot_at(slot) {
        Some(SyntaxSlot::Node(node)) if node.is_directly_malformed() => {
            Ok(JavaSyntaxField::Malformed(JavaMalformedSyntax {
                syntax: node,
            }))
        }
        Some(SyntaxSlot::Node(node)) => Ok(JavaSyntaxField::Present(SyntaxElement::Node(node))),
        Some(SyntaxSlot::Token(token)) => Ok(JavaSyntaxField::Present(SyntaxElement::Token(token))),
        None => Err(JavaSyntaxInvariantError {
            node: syntax.kind(),
            slot,
        }),
        Some(SyntaxSlot::Empty) => Ok(JavaSyntaxField::Missing(JavaMissingSyntax {
            owner: syntax.missing_owner(),
            slot,
        })),
    }
}

/// A Java operator, which may span multiple syntax tokens in ambiguous `>` forms.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct JavaOperator<'source> {
    kind: JavaOperatorKind,
    components: [Option<JavaSyntaxField<'source, JavaSyntaxToken<'source>>>; 4],
    len: usize,
}

impl<'source> JavaOperator<'source> {
    pub(crate) fn single(kind: JavaOperatorKind, token: JavaSyntaxToken<'source>) -> Self {
        Self {
            kind,
            components: [Some(JavaSyntaxField::Present(token)), None, None, None],
            len: 1,
        }
    }

    pub(crate) fn composite(
        kind: JavaOperatorKind,
        components: [Option<JavaSyntaxField<'source, JavaSyntaxToken<'source>>>; 4],
        len: usize,
    ) -> Self {
        Self {
            kind,
            components,
            len,
        }
    }

    #[must_use]
    pub const fn kind(&self) -> JavaOperatorKind {
        self.kind
    }

    #[must_use]
    pub fn text(&self) -> &'static str {
        self.kind.text()
    }

    #[must_use]
    pub fn as_single_token(&self) -> Option<&JavaSyntaxToken<'source>> {
        if self.len == 1 {
            match self.components[0].as_ref() {
                Some(JavaSyntaxField::Present(token)) => Some(token),
                Some(JavaSyntaxField::Missing(_) | JavaSyntaxField::Malformed(_)) | None => None,
            }
        } else {
            None
        }
    }

    pub fn components(
        &self,
    ) -> impl Iterator<Item = JavaSyntaxField<'source, JavaSyntaxToken<'source>>> + '_ {
        self.components.iter().take(self.len).flatten().copied()
    }
}

/// Logical Java operator kinds used to reconstruct composite operator text.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum JavaOperatorKind {
    Assign,
    PlusEq,
    MinusEq,
    StarEq,
    SlashEq,
    AmpEq,
    BarEq,
    CaretEq,
    PercentEq,
    LShiftEq,
    RShiftEq,
    UnsignedRShiftEq,
    Instanceof,
    OrOr,
    AndAnd,
    Bar,
    Caret,
    Amp,
    EqEq,
    BangEq,
    Lt,
    Gt,
    LtEq,
    GtEq,
    LShift,
    RShift,
    UnsignedRShift,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
}

impl JavaOperatorKind {
    #[must_use]
    const fn text(self) -> &'static str {
        match self {
            Self::Assign => "=",
            Self::PlusEq => "+=",
            Self::MinusEq => "-=",
            Self::StarEq => "*=",
            Self::SlashEq => "/=",
            Self::AmpEq => "&=",
            Self::BarEq => "|=",
            Self::CaretEq => "^=",
            Self::PercentEq => "%=",
            Self::LShiftEq => "<<=",
            Self::RShiftEq => ">>=",
            Self::UnsignedRShiftEq => ">>>=",
            Self::Instanceof => "instanceof",
            Self::OrOr => "||",
            Self::AndAnd => "&&",
            Self::Bar => "|",
            Self::Caret => "^",
            Self::Amp => "&",
            Self::EqEq => "==",
            Self::BangEq => "!=",
            Self::Lt => "<",
            Self::Gt => ">",
            Self::LtEq => "<=",
            Self::GtEq => ">=",
            Self::LShift => "<<",
            Self::RShift => ">>",
            Self::UnsignedRShift => ">>>",
            Self::Plus => "+",
            Self::Minus => "-",
            Self::Star => "*",
            Self::Slash => "/",
            Self::Percent => "%",
        }
    }
}

pub(crate) struct JavaOperatorPattern {
    pub(crate) kind: JavaOperatorKind,
    pub(crate) tokens: &'static [JavaSyntaxKind],
}

pub(crate) const COMPOSITE_ASSIGNMENT_OPERATORS: &[JavaOperatorPattern] = &[
    JavaOperatorPattern {
        kind: JavaOperatorKind::UnsignedRShiftEq,
        tokens: &[
            JavaSyntaxKind::Gt,
            JavaSyntaxKind::Gt,
            JavaSyntaxKind::Gt,
            JavaSyntaxKind::Assign,
        ],
    },
    JavaOperatorPattern {
        kind: JavaOperatorKind::RShiftEq,
        tokens: &[
            JavaSyntaxKind::Gt,
            JavaSyntaxKind::Gt,
            JavaSyntaxKind::Assign,
        ],
    },
];

pub(crate) const COMPOSITE_BINARY_OPERATORS: &[JavaOperatorPattern] = &[
    JavaOperatorPattern {
        kind: JavaOperatorKind::GtEq,
        tokens: &[JavaSyntaxKind::Gt, JavaSyntaxKind::Assign],
    },
    JavaOperatorPattern {
        kind: JavaOperatorKind::UnsignedRShift,
        tokens: &[JavaSyntaxKind::Gt, JavaSyntaxKind::Gt, JavaSyntaxKind::Gt],
    },
    JavaOperatorPattern {
        kind: JavaOperatorKind::RShift,
        tokens: &[JavaSyntaxKind::Gt, JavaSyntaxKind::Gt],
    },
];

pub(crate) fn assignment_operator_kind(kind: JavaSyntaxKind) -> Option<JavaOperatorKind> {
    Some(match kind {
        JavaSyntaxKind::Assign => JavaOperatorKind::Assign,
        JavaSyntaxKind::PlusEq => JavaOperatorKind::PlusEq,
        JavaSyntaxKind::MinusEq => JavaOperatorKind::MinusEq,
        JavaSyntaxKind::StarEq => JavaOperatorKind::StarEq,
        JavaSyntaxKind::SlashEq => JavaOperatorKind::SlashEq,
        JavaSyntaxKind::AmpEq => JavaOperatorKind::AmpEq,
        JavaSyntaxKind::BarEq => JavaOperatorKind::BarEq,
        JavaSyntaxKind::CaretEq => JavaOperatorKind::CaretEq,
        JavaSyntaxKind::PercentEq => JavaOperatorKind::PercentEq,
        JavaSyntaxKind::LShiftEq => JavaOperatorKind::LShiftEq,
        _ => return None,
    })
}

pub(crate) fn binary_operator_kind(kind: JavaSyntaxKind) -> Option<JavaOperatorKind> {
    Some(match kind {
        JavaSyntaxKind::OrOr => JavaOperatorKind::OrOr,
        JavaSyntaxKind::AndAnd => JavaOperatorKind::AndAnd,
        JavaSyntaxKind::Bar => JavaOperatorKind::Bar,
        JavaSyntaxKind::Caret => JavaOperatorKind::Caret,
        JavaSyntaxKind::Amp => JavaOperatorKind::Amp,
        JavaSyntaxKind::EqEq => JavaOperatorKind::EqEq,
        JavaSyntaxKind::BangEq => JavaOperatorKind::BangEq,
        JavaSyntaxKind::Lt => JavaOperatorKind::Lt,
        JavaSyntaxKind::Gt => JavaOperatorKind::Gt,
        JavaSyntaxKind::LtEq => JavaOperatorKind::LtEq,
        JavaSyntaxKind::LShift => JavaOperatorKind::LShift,
        JavaSyntaxKind::Plus => JavaOperatorKind::Plus,
        JavaSyntaxKind::Minus => JavaOperatorKind::Minus,
        JavaSyntaxKind::Star => JavaOperatorKind::Star,
        JavaSyntaxKind::Slash => JavaOperatorKind::Slash,
        JavaSyntaxKind::Percent => JavaOperatorKind::Percent,
        JavaSyntaxKind::InstanceofKw => JavaOperatorKind::Instanceof,
        _ => return None,
    })
}

#[must_use]
pub const fn binary_operator_precedence(kind: JavaOperatorKind) -> Option<u8> {
    Some(match kind {
        JavaOperatorKind::OrOr => 1,
        JavaOperatorKind::AndAnd => 2,
        JavaOperatorKind::Bar => 3,
        JavaOperatorKind::Caret => 4,
        JavaOperatorKind::Amp => 5,
        JavaOperatorKind::EqEq | JavaOperatorKind::BangEq => 6,
        JavaOperatorKind::Lt
        | JavaOperatorKind::Gt
        | JavaOperatorKind::LtEq
        | JavaOperatorKind::GtEq
        | JavaOperatorKind::Instanceof => 7,
        JavaOperatorKind::LShift | JavaOperatorKind::RShift | JavaOperatorKind::UnsignedRShift => 8,
        JavaOperatorKind::Plus | JavaOperatorKind::Minus => 9,
        JavaOperatorKind::Star | JavaOperatorKind::Slash | JavaOperatorKind::Percent => 10,
        JavaOperatorKind::Assign
        | JavaOperatorKind::PlusEq
        | JavaOperatorKind::MinusEq
        | JavaOperatorKind::StarEq
        | JavaOperatorKind::SlashEq
        | JavaOperatorKind::AmpEq
        | JavaOperatorKind::BarEq
        | JavaOperatorKind::CaretEq
        | JavaOperatorKind::PercentEq
        | JavaOperatorKind::LShiftEq
        | JavaOperatorKind::RShiftEq
        | JavaOperatorKind::UnsignedRShiftEq => return None,
    })
}

#[must_use]
pub const fn is_shift_operator(kind: JavaOperatorKind) -> bool {
    matches!(
        kind,
        JavaOperatorKind::LShift | JavaOperatorKind::RShift | JavaOperatorKind::UnsignedRShift
    )
}

#[must_use]
pub const fn is_bitwise_or_shift_operator(kind: JavaOperatorKind) -> bool {
    matches!(
        kind,
        JavaOperatorKind::Bar
            | JavaOperatorKind::Caret
            | JavaOperatorKind::Amp
            | JavaOperatorKind::LShift
            | JavaOperatorKind::RShift
            | JavaOperatorKind::UnsignedRShift
    )
}

#[must_use]
pub const fn is_multiplicative_operator(kind: JavaOperatorKind) -> bool {
    matches!(
        kind,
        JavaOperatorKind::Star | JavaOperatorKind::Slash | JavaOperatorKind::Percent
    )
}

mod private {
    pub trait Sealed {}
}

/// Sealed access to behavior shared by every typed Java syntax view.
pub trait JavaSyntaxView<'source>: private::Sealed {
    /// Returns the ordinary physical syntax node backing this view.
    fn syntax_node(&self) -> Option<JavaSyntaxNode<'source>>;

    /// Returns the first token represented by this view.
    fn first_token(&self) -> Option<JavaSyntaxToken<'source>> {
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
    fn malformed_verbatim_core(&self) -> Option<SyntaxVerbatimCore<'source, JavaLanguage>> {
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

impl private::Sealed for JavaMalformedSyntax<'_> {}

impl<'source> JavaSyntaxView<'source> for JavaMalformedSyntax<'source> {
    fn syntax_node(&self) -> Option<JavaSyntaxNode<'source>> {
        Some(self.syntax)
    }
}

pub trait JavaTypedNode<'source>: Clone + private::Sealed {
    #[doc(hidden)]
    fn cast_element(element: JavaRoleElement<'source>) -> Option<Self>;
}

pub trait JavaNode<'source>: JavaTypedNode<'source> {
    fn cast(syntax: JavaSyntaxNode<'source>) -> Option<Self>;
}

pub trait JavaFamily<'source>: Clone + private::Sealed {
    fn cast(syntax: JavaSyntaxNode<'source>) -> Option<Self>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JavaRoleElement<'source> {
    Node(JavaSyntaxNode<'source>),
    Token(JavaSyntaxToken<'source>),
}

impl<'source> JavaRoleElement<'source> {
    #[must_use]
    pub fn token(self) -> Option<JavaSyntaxToken<'source>> {
        match self {
            Self::Token(token) => Some(token),
            Self::Node(_) => None,
        }
    }

    #[must_use]
    pub fn cast_node<N: JavaTypedNode<'source>>(self) -> Option<N> {
        N::cast_element(self)
    }

    #[must_use]
    pub fn cast_family<F: JavaFamily<'source>>(self) -> Option<F> {
        match self {
            Self::Node(node) => F::cast(node),
            Self::Token(_) => None,
        }
    }
}

trait JavaListItem<'source>: Sized {
    const IS_FAMILY: bool;

    fn cast_element(element: JavaRoleElement<'source>) -> Option<Self>;
}

impl<'source> JavaListItem<'source> for JavaRoleElement<'source> {
    const IS_FAMILY: bool = false;

    fn cast_element(element: JavaRoleElement<'source>) -> Option<Self> {
        Some(element)
    }
}

/// One represented part of a variable-length Java syntax-list node.
#[derive(Clone, Copy, Debug)]
pub enum JavaSyntaxListPart<'source, T> {
    Item(T),
    Separator(JavaSyntaxToken<'source>),
    Missing(JavaMissingSyntax<'source>),
    Malformed(JavaMalformedSyntax<'source>),
}

fn list_parts<'source, T: JavaListItem<'source>>(
    syntax: JavaSyntaxNode<'source>,
    separated: bool,
) -> impl Iterator<Item = JavaSyntaxResult<JavaSyntaxListPart<'source, T>>> + use<'source, T> {
    (0..syntax.slot_count()).map(move |index| {
        let Some(slot) = syntax.slot_at(index) else {
            return Err(JavaSyntaxInvariantError {
                node: syntax.kind(),
                slot: index,
            });
        };
        match slot {
            SyntaxSlot::Token(token) if separated && index % 2 == 1 => {
                Ok(JavaSyntaxListPart::Separator(token))
            }
            SyntaxSlot::Node(node) => {
                let item = T::cast_element(JavaRoleElement::Node(node));
                match item {
                    Some(item)
                        if !node.is_directly_malformed()
                            || (T::IS_FAMILY && java_kind_is_category_bogus(node.kind())) =>
                    {
                        Ok(JavaSyntaxListPart::Item(item))
                    }
                    _ if node.is_directly_malformed() => {
                        Ok(JavaSyntaxListPart::Malformed(JavaMalformedSyntax {
                            syntax: node,
                        }))
                    }
                    _ => Err(JavaSyntaxInvariantError {
                        node: syntax.kind(),
                        slot: index,
                    }),
                }
            }
            SyntaxSlot::Token(token) => T::cast_element(JavaRoleElement::Token(token))
                .map(JavaSyntaxListPart::Item)
                .ok_or(JavaSyntaxInvariantError {
                    node: syntax.kind(),
                    slot: index,
                }),
            SyntaxSlot::Empty => Ok(JavaSyntaxListPart::Missing(JavaMissingSyntax {
                owner: syntax,
                slot: index,
            })),
        }
    })
}

#[inline]
fn required_token(
    syntax: JavaFixedSyntax<'_>,
    slot: usize,
) -> JavaSyntaxResult<JavaSyntaxField<'_, JavaSyntaxToken<'_>>> {
    match required_slot(syntax, slot)? {
        JavaSyntaxField::Present(SyntaxElement::Token(token)) => {
            Ok(JavaSyntaxField::Present(token))
        }
        JavaSyntaxField::Missing(missing) => Ok(JavaSyntaxField::Missing(missing)),
        JavaSyntaxField::Malformed(node) => Ok(JavaSyntaxField::Malformed(node)),
        JavaSyntaxField::Present(SyntaxElement::Node(_)) => Err(JavaSyntaxInvariantError {
            node: syntax.kind(),
            slot,
        }),
    }
}

#[inline]
fn optional_token(
    syntax: JavaFixedSyntax<'_>,
    slot: usize,
) -> JavaSyntaxResult<JavaSyntaxField<'_, JavaSyntaxToken<'_>>> {
    match optional_slot(syntax, slot)? {
        JavaSyntaxField::Present(SyntaxElement::Token(token)) => {
            Ok(JavaSyntaxField::Present(token))
        }
        JavaSyntaxField::Missing(missing) => Ok(JavaSyntaxField::Missing(missing)),
        JavaSyntaxField::Malformed(node) => Ok(JavaSyntaxField::Malformed(node)),
        JavaSyntaxField::Present(SyntaxElement::Node(_)) => Err(JavaSyntaxInvariantError {
            node: syntax.kind(),
            slot,
        }),
    }
}

#[inline]
fn required_node<'source, N: JavaNode<'source>>(
    syntax: JavaFixedSyntax<'source>,
    slot: usize,
) -> JavaSyntaxResult<JavaSyntaxField<'source, N>> {
    match required_slot(syntax, slot)? {
        JavaSyntaxField::Present(SyntaxElement::Node(node)) => N::cast(node)
            .map(JavaSyntaxField::Present)
            .ok_or(JavaSyntaxInvariantError {
                node: syntax.kind(),
                slot,
            }),
        JavaSyntaxField::Missing(missing) => Ok(JavaSyntaxField::Missing(missing)),
        JavaSyntaxField::Malformed(node) => Ok(JavaSyntaxField::Malformed(node)),
        JavaSyntaxField::Present(SyntaxElement::Token(_)) => Err(JavaSyntaxInvariantError {
            node: syntax.kind(),
            slot,
        }),
    }
}

#[inline]
fn optional_node<'source, N: JavaNode<'source>>(
    syntax: JavaFixedSyntax<'source>,
    slot: usize,
) -> JavaSyntaxResult<JavaSyntaxField<'source, N>> {
    match optional_slot(syntax, slot)? {
        JavaSyntaxField::Present(SyntaxElement::Node(node)) => N::cast(node)
            .map(JavaSyntaxField::Present)
            .ok_or(JavaSyntaxInvariantError {
                node: syntax.kind(),
                slot,
            }),
        JavaSyntaxField::Missing(missing) => Ok(JavaSyntaxField::Missing(missing)),
        JavaSyntaxField::Malformed(node) => Ok(JavaSyntaxField::Malformed(node)),
        JavaSyntaxField::Present(SyntaxElement::Token(_)) => Err(JavaSyntaxInvariantError {
            node: syntax.kind(),
            slot,
        }),
    }
}

#[inline]
fn required_role_element(
    syntax: JavaFixedSyntax<'_>,
    slot: usize,
) -> JavaSyntaxResult<JavaSyntaxField<'_, JavaRoleElement<'_>>> {
    match syntax.slot_at(slot) {
        Some(SyntaxSlot::Node(node)) if node.is_directly_malformed() => {
            Ok(JavaSyntaxField::Malformed(JavaMalformedSyntax {
                syntax: node,
            }))
        }
        Some(SyntaxSlot::Node(node)) => Ok(JavaSyntaxField::Present(JavaRoleElement::Node(node))),
        Some(SyntaxSlot::Token(token)) => {
            Ok(JavaSyntaxField::Present(JavaRoleElement::Token(token)))
        }
        Some(SyntaxSlot::Empty) => Ok(JavaSyntaxField::Missing(JavaMissingSyntax {
            owner: syntax.missing_owner(),
            slot,
        })),
        None => Err(JavaSyntaxInvariantError {
            node: syntax.kind(),
            slot,
        }),
    }
}

#[inline]
fn required_family<'source, F: JavaFamily<'source>>(
    syntax: JavaFixedSyntax<'source>,
    slot: usize,
) -> JavaSyntaxResult<JavaSyntaxField<'source, F>> {
    match syntax.slot_at(slot) {
        Some(SyntaxSlot::Node(node)) => match F::cast(node) {
            Some(value)
                if !node.is_directly_malformed() || java_kind_is_category_bogus(node.kind()) =>
            {
                Ok(JavaSyntaxField::Present(value))
            }
            Some(_) => Ok(JavaSyntaxField::Malformed(JavaMalformedSyntax {
                syntax: node,
            })),
            None if node.is_directly_malformed() => {
                Ok(JavaSyntaxField::Malformed(JavaMalformedSyntax {
                    syntax: node,
                }))
            }
            None => Err(JavaSyntaxInvariantError {
                node: syntax.kind(),
                slot,
            }),
        },
        Some(SyntaxSlot::Empty) => Ok(JavaSyntaxField::Missing(JavaMissingSyntax {
            owner: syntax.missing_owner(),
            slot,
        })),
        Some(SyntaxSlot::Token(_)) | None => Err(JavaSyntaxInvariantError {
            node: syntax.kind(),
            slot,
        }),
    }
}

#[inline]
fn optional_family<'source, F: JavaFamily<'source>>(
    syntax: JavaFixedSyntax<'source>,
    slot: usize,
) -> JavaSyntaxResult<JavaSyntaxField<'source, F>> {
    required_family(syntax, slot)
}

fn syntax_source_text(syntax: JavaFixedSyntax<'_>) -> &str {
    let range = syntax.text_range();
    &syntax.source()[range.start().get()..range.end().get()]
}

macro_rules! java_field_accessor {
    ($module:ident $field:ident required $matcher:tt => $role:ident) => {
        #[inline]
        #[allow(clippy::missing_errors_doc)]
        pub fn $field(&self) -> JavaSyntaxResult<JavaSyntaxField<'source, $role<'source>>> {
            required_role_element(self.fixed_syntax(), crate::shape::$module::Slot::$field as usize).map(|slot| {
                slot.map(|element| $role { element })
            })
        }
    };
    ($module:ident $field:ident optional $matcher:tt => $role:ident) => {
        #[inline]
        #[allow(clippy::missing_errors_doc)]
        pub fn $field(&self) -> JavaSyntaxResult<JavaSyntaxField<'source, $role<'source>>> {
            required_role_element(self.fixed_syntax(), crate::shape::$module::Slot::$field as usize)
                .map(|slot| slot.map(|element| $role { element }))
        }
    };
    ($module:ident $field:ident required (token $kind:ident)) => {
        #[inline]
        #[allow(clippy::missing_errors_doc)]
        pub fn $field(&self) -> JavaSyntaxResult<JavaSyntaxField<'source, JavaSyntaxToken<'source>>> {
            required_token(self.fixed_syntax(), crate::shape::$module::Slot::$field as usize)
        }
    };
    ($module:ident $field:ident required (token_set $kinds:tt)) => {
        java_field_accessor!($module $field required (token __schema_token_set));
    };
    ($module:ident $field:ident required (contextual $text:literal)) => {
        java_field_accessor!($module $field required (token __schema_contextual));
    };
    ($module:ident $field:ident optional (token $kind:ident)) => {
        #[inline]
        #[allow(clippy::missing_errors_doc)]
        pub fn $field(&self) -> JavaSyntaxResult<JavaSyntaxField<'source, JavaSyntaxToken<'source>>> {
            optional_token(self.fixed_syntax(), crate::shape::$module::Slot::$field as usize)
        }
    };
    ($module:ident $field:ident optional (token_set $kinds:tt)) => {
        java_field_accessor!($module $field optional (token __schema_token_set));
    };
    ($module:ident $field:ident optional (contextual $text:literal)) => {
        java_field_accessor!($module $field optional (token __schema_contextual));
    };

    ($module:ident $field:ident required (node $target:ident)) => {
        #[inline]
        #[allow(clippy::missing_errors_doc)]
        pub fn $field(&self) -> JavaSyntaxResult<JavaSyntaxField<'source, $target<'source>>> {
            required_node(self.fixed_syntax(), crate::shape::$module::Slot::$field as usize)
        }
    };
    ($module:ident $field:ident optional (node $target:ident)) => {
        #[inline]
        #[allow(clippy::missing_errors_doc)]
        pub fn $field(&self) -> JavaSyntaxResult<JavaSyntaxField<'source, $target<'source>>> {
            optional_node(self.fixed_syntax(), crate::shape::$module::Slot::$field as usize)
        }
    };
    ($module:ident $field:ident required (constructed $target:ident)) => {
        #[inline]
        #[allow(clippy::missing_errors_doc)]
        pub fn $field(&self) -> JavaSyntaxResult<JavaSyntaxField<'source, $target<'source>>> {
            required_node(self.fixed_syntax(), crate::shape::$module::Slot::$field as usize)
        }
    };
    ($module:ident $field:ident optional (constructed $target:ident)) => {
        #[inline]
        #[allow(clippy::missing_errors_doc)]
        pub fn $field(&self) -> JavaSyntaxResult<JavaSyntaxField<'source, $target<'source>>> {
            optional_node(self.fixed_syntax(), crate::shape::$module::Slot::$field as usize)
        }
    };
    ($module:ident $field:ident required (list $target:ident)) => {
        #[inline]
        #[allow(clippy::missing_errors_doc)]
        pub fn $field(&self) -> JavaSyntaxResult<JavaSyntaxField<'source, $target<'source>>> {
            required_node(self.fixed_syntax(), crate::shape::$module::Slot::$field as usize)
        }
    };
    ($module:ident $field:ident optional (list $target:ident)) => {
        #[inline]
        #[allow(clippy::missing_errors_doc)]
        pub fn $field(&self) -> JavaSyntaxResult<JavaSyntaxField<'source, $target<'source>>> {
            optional_node(self.fixed_syntax(), crate::shape::$module::Slot::$field as usize)
        }
    };
    ($module:ident $field:ident required (category $target:ident)) => {
        #[inline]
        #[allow(clippy::missing_errors_doc)]
        pub fn $field(&self) -> JavaSyntaxResult<JavaSyntaxField<'source, $target<'source>>> {
            required_family(self.fixed_syntax(), crate::shape::$module::Slot::$field as usize)
        }
    };
    ($module:ident $field:ident optional (category $target:ident)) => {
        #[inline]
        #[allow(clippy::missing_errors_doc)]
        pub fn $field(&self) -> JavaSyntaxResult<JavaSyntaxField<'source, $target<'source>>> {
            optional_family(self.fixed_syntax(), crate::shape::$module::Slot::$field as usize)
        }
    };

    // Heterogeneous roles are wrapped by the semantic adapters below. This
    // primitive still reads exactly one declared slot and never searches.
    ($module:ident $field:ident required $matcher:tt) => {
        #[inline]
        pub(crate) fn $field(&self) -> JavaSyntaxResult<JavaSyntaxField<'source, JavaRoleElement<'source>>> {
            required_role_element(self.fixed_syntax(), crate::shape::$module::Slot::$field as usize)
        }
    };
    ($module:ident $field:ident optional $matcher:tt) => {
        #[inline]
        pub(crate) fn $field(&self) -> JavaSyntaxResult<JavaSyntaxField<'source, JavaRoleElement<'source>>> {
            required_role_element(self.fixed_syntax(), crate::shape::$module::Slot::$field as usize)
        }
    };
    ($module:ident $field:ident many $matcher:tt $(=> $role:ident)?) => {};
    ($module:ident $field:ident one_or_more $matcher:tt $(=> $role:ident)?) => {};
}

macro_rules! define_java_role {
    ($role:ident) => {
        #[derive(Clone, Copy, Debug)]
        pub struct $role<'source> {
            element: JavaRoleElement<'source>,
        }

        impl<'source> $role<'source> {
            #[must_use]
            pub fn token(self) -> Option<JavaSyntaxToken<'source>> {
                match self.element {
                    JavaRoleElement::Token(token) => Some(token),
                    JavaRoleElement::Node(_) => None,
                }
            }

            #[must_use]
            pub fn first_token(self) -> Option<JavaSyntaxToken<'source>> {
                match self.element {
                    JavaRoleElement::Node(node) => node.first_token(),
                    JavaRoleElement::Token(token) => Some(token),
                }
            }

            #[must_use]
            pub fn last_token(self) -> Option<JavaSyntaxToken<'source>> {
                match self.element {
                    JavaRoleElement::Node(node) => node.last_token(),
                    JavaRoleElement::Token(token) => Some(token),
                }
            }

            #[must_use]
            pub fn cast_node<N: JavaTypedNode<'source>>(self) -> Option<N> {
                N::cast_element(self.element)
            }

            #[must_use]
            pub fn cast_family<F: JavaFamily<'source>>(self) -> Option<F> {
                match self.element {
                    JavaRoleElement::Node(node) => F::cast(node),
                    JavaRoleElement::Token(_) => None,
                }
            }
        }

        impl<'source> JavaListItem<'source> for $role<'source> {
            const IS_FAMILY: bool = false;

            fn cast_element(element: JavaRoleElement<'source>) -> Option<Self> {
                Some(Self { element })
            }
        }
    };
}

macro_rules! java_list_item_type {
    ($source:lifetime; $matcher:tt => $role:ident) => { $role<$source> };
    ($source:lifetime; (node $target:ident)) => { $target<$source> };
    ($source:lifetime; (constructed $target:ident)) => { $target<$source> };
    ($source:lifetime; (category $target:ident)) => { $target<$source> };
    ($source:lifetime; $matcher:tt) => { JavaRoleElement<$source> };
}

macro_rules! java_list_item_type_optional_role {
    ($source:lifetime; $matcher:tt; $role:ident) => {
        java_list_item_type!($source; $matcher => $role)
    };
    ($source:lifetime; $matcher:tt;) => {
        java_list_item_type!($source; $matcher)
    };
}

macro_rules! java_list_is_separated {
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

macro_rules! java_variable_slot_view {
    (list; $field:ident: $cardinality:ident $matcher:tt $(=> $role:ident)? $([$($policy:tt)*])?;) => {
        /// Returns this list's represented elements and separators in source order.
        pub fn parts(
            &self,
        ) -> impl Iterator<
            Item = JavaSyntaxResult<JavaSyntaxListPart<
                'source,
                java_list_item_type_optional_role!('source; $matcher; $($role)?),
            >>,
        > + '_ {
            list_parts::<java_list_item_type_optional_role!('source; $matcher; $($role)?)>(
                self.syntax,
                java_list_is_separated!($([$($policy)*])?),
            )
        }
    };
    ($class:ident; $($fields:tt)*) => {};
}

macro_rules! define_java_cst_node {
    ($node:ident => $kind:ident [list]) => {
        define_java_cst_node!($node => $kind [physical]);
    };
    ($node:ident => $kind:ident [constructed]) => {
        define_java_cst_node!($node => $kind [physical]);
    };
    ($node:ident => $kind:ident [$class:ident]) => {
        #[derive(Clone, Copy, Eq, PartialEq)]
        pub struct $node<'source> {
            syntax: JavaSyntaxNode<'source>,
        }

        impl<'source> $node<'source> {
            #[inline]
            fn fixed_syntax(&self) -> JavaFixedSyntax<'source> {
                JavaFixedSyntax(self.syntax)
            }
            #[must_use]
            pub fn kind(&self) -> JavaSyntaxKind {
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

            pub fn token_iter(&self) -> impl Iterator<Item = JavaSyntaxToken<'source>> + '_ {
                token_iter(&self.syntax)
            }

            #[must_use]
            pub fn first_token(&self) -> Option<JavaSyntaxToken<'source>> {
                first_token(&self.syntax)
            }

            #[must_use]
            pub fn last_token(&self) -> Option<JavaSyntaxToken<'source>> {
                last_token(&self.syntax)
            }
        }

        impl private::Sealed for $node<'_> {}

        impl<'source> JavaSyntaxView<'source> for $node<'source> {
            fn syntax_node(&self) -> Option<JavaSyntaxNode<'source>> {
                Some(self.syntax)
            }
        }

        impl<'source> JavaNode<'source> for $node<'source> {
            fn cast(syntax: JavaSyntaxNode<'source>) -> Option<Self> {
                matches!(syntax.kind(), JavaSyntaxKind::$kind).then_some(Self { syntax })
            }
        }

        impl<'source> JavaTypedNode<'source> for $node<'source> {
            fn cast_element(element: JavaRoleElement<'source>) -> Option<Self> {
                match element {
                    JavaRoleElement::Node(node) => Self::cast(node),
                    JavaRoleElement::Token(_) => None,
                }
            }
        }

        impl<'source> JavaListItem<'source> for $node<'source> {
            const IS_FAMILY: bool = false;

            fn cast_element(element: JavaRoleElement<'source>) -> Option<Self> {
                <Self as JavaTypedNode<'source>>::cast_element(element)
            }
        }

        impl fmt::Debug for $node<'_> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.syntax.fmt(f)
            }
        }
    };
}

macro_rules! java_cst {
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
        $(define_java_cst_node!($node => $kind [$class]);)*

        $(
            #[derive(Clone, Copy, Debug, Eq, PartialEq)]
            pub enum $family<'source> {
                $($variant($variant<'source>),)+
            }

            impl<'source> $family<'source> {
                #[must_use]
                pub fn kind(&self) -> JavaSyntaxKind {
                    self.syntax().kind()
                }

                #[must_use]
                pub fn text_range(&self) -> TextRange {
                    self.syntax().text_range()
                }

                #[must_use]
                pub fn source_text(&self) -> &'source str {
                    syntax_source_text(JavaFixedSyntax(*self.syntax()))
                }

                pub fn token_iter(&self) -> impl Iterator<Item = JavaSyntaxToken<'source>> + '_ {
                    token_iter(self.syntax())
                }

                #[must_use]
                pub fn first_token(&self) -> Option<JavaSyntaxToken<'source>> {
                    first_token(self.syntax())
                }

                #[must_use]
                pub fn last_token(&self) -> Option<JavaSyntaxToken<'source>> {
                    last_token(self.syntax())
                }

                pub(crate) fn syntax(&self) -> &JavaSyntaxNode<'source> {
                    match self {
                        $(Self::$variant(node) => &node.syntax,)+
                    }
                }
            }

            impl<'source> JavaFamily<'source> for $family<'source> {
                fn cast(syntax: JavaSyntaxNode<'source>) -> Option<Self> {
                    match syntax.kind() {
                        $(
                            JavaSyntaxKind::$variant => {
                                <$variant<'source> as JavaNode<'source>>::cast(syntax).map(Self::$variant)
                            }
                        )+
                        _ => None,
                    }
                }
            }

            impl<'source> JavaListItem<'source> for $family<'source> {
                const IS_FAMILY: bool = true;

                fn cast_element(element: JavaRoleElement<'source>) -> Option<Self> {
                    match element {
                        JavaRoleElement::Node(node) => Self::cast(node),
                        JavaRoleElement::Token(_) => None,
                    }
                }

            }

            impl private::Sealed for $family<'_> {}

            impl<'source> JavaSyntaxView<'source> for $family<'source> {
                fn syntax_node(&self) -> Option<JavaSyntaxNode<'source>> {
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

macro_rules! define_java_cst_from_schema {
    (
        tokens { $($token:ident,)* }
        categories { $($family:ident => $bogus:ident { $($member:ident,)* })* }
        nodes { $($kind:ident => $wrapper:ident [$module:ident $class:ident] { $($fields:tt)* })* }
    ) => {
        fn java_kind_is_category_bogus(kind: JavaSyntaxKind) -> bool {
            matches!(kind, $(JavaSyntaxKind::$bogus)|*)
        }

        java_cst! {
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

java_syntax_schema!(define_java_cst_from_schema);

macro_rules! define_java_accessors_from_schema {
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
        $($( $(define_java_role!($role);)? )*)*
        $(
            impl<'source> $wrapper<'source> {
                $(java_field_accessor!($module $field $cardinality $matcher $(=> $role)?);)*
                java_variable_slot_view!(
                    $class;
                    $($field: $cardinality $matcher $(=> $role)? $([$($policy)*])?;)*
                );
            }
        )*
    };
}

java_syntax_schema!(define_java_accessors_from_schema);

#[derive(Clone, Copy, Debug)]
pub enum ModifierItem<'source> {
    Annotation(Annotation<'source>),
    Bogus(BogusModifier<'source>),
    Sealed(JavaSyntaxToken<'source>),
    Token(JavaSyntaxToken<'source>),
    NonSealed(NonSealedModifier<'source>),
}

impl<'source> ModifierElement<'source> {
    /// Classifies one modifier-list element by its declared syntax role.
    ///
    /// # Errors
    ///
    /// Returns an invariant error if the represented element is not a declared modifier.
    pub fn classify(self) -> Result<ModifierItem<'source>, JavaSyntaxInvariantError> {
        if let Some(annotation) = self.cast_node::<Annotation<'source>>() {
            Ok(ModifierItem::Annotation(annotation))
        } else if let Some(bogus) = self.cast_node::<BogusModifier<'source>>() {
            Ok(ModifierItem::Bogus(bogus))
        } else if let Some(non_sealed) = self.cast_node::<NonSealedModifier<'source>>() {
            Ok(ModifierItem::NonSealed(non_sealed))
        } else if let Some(token) = self.token() {
            if token.kind() == JavaSyntaxKind::Identifier
                && crate::lexer::lexical_text_is(token.text(), "sealed")
            {
                Ok(ModifierItem::Sealed(token))
            } else {
                Ok(ModifierItem::Token(token))
            }
        } else {
            Err(JavaSyntaxInvariantError {
                node: JavaSyntaxKind::ModifierList,
                slot: 0,
            })
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum PartitionedModifierItem<'source> {
    DeclarationAnnotation(Annotation<'source>),
    TypeUseAnnotation(Annotation<'source>),
    Token(JavaSyntaxToken<'source>),
    Sealed(JavaSyntaxToken<'source>),
    NonSealed(NonSealedModifier<'source>),
    Bogus(BogusModifier<'source>),
    Missing(JavaMissingSyntax<'source>),
    Malformed(JavaMalformedSyntax<'source>),
}

impl<'source> ModifierList<'source> {
    pub fn partitioned_items(
        &self,
    ) -> impl Iterator<Item = Result<PartitionedModifierItem<'source>, JavaSyntaxInvariantError>> + '_
    {
        let mut saw_modifier = false;
        self.parts().filter_map(move |part| match part {
            Ok(JavaSyntaxListPart::Item(item)) => Some(match item.classify() {
                Ok(ModifierItem::Annotation(annotation)) if saw_modifier => {
                    Ok(PartitionedModifierItem::TypeUseAnnotation(annotation))
                }
                Ok(ModifierItem::Annotation(annotation)) => {
                    Ok(PartitionedModifierItem::DeclarationAnnotation(annotation))
                }
                Ok(ModifierItem::Token(token)) => {
                    saw_modifier = true;
                    Ok(PartitionedModifierItem::Token(token))
                }
                Ok(ModifierItem::Sealed(token)) => {
                    saw_modifier = true;
                    Ok(PartitionedModifierItem::Sealed(token))
                }
                Ok(ModifierItem::NonSealed(non_sealed)) => {
                    saw_modifier = true;
                    Ok(PartitionedModifierItem::NonSealed(non_sealed))
                }
                Ok(ModifierItem::Bogus(bogus)) => {
                    saw_modifier = true;
                    Ok(PartitionedModifierItem::Bogus(bogus))
                }
                Err(error) => Err(error),
            }),
            Ok(JavaSyntaxListPart::Malformed(malformed)) => {
                Some(Ok(PartitionedModifierItem::Malformed(malformed)))
            }
            Ok(JavaSyntaxListPart::Missing(missing)) => {
                Some(Ok(PartitionedModifierItem::Missing(missing)))
            }
            Ok(JavaSyntaxListPart::Separator(_)) => None,
            Err(error) => Some(Err(error)),
        })
    }
}

impl<'source> ParameterModifierList<'source> {
    /// Partitions parameter modifiers by their grammar position. Annotations
    /// before `final` are declaration annotations; annotations after it apply
    /// to the type.
    pub fn partitioned_items(
        &self,
    ) -> impl Iterator<Item = Result<PartitionedModifierItem<'source>, JavaSyntaxInvariantError>> + '_
    {
        let mut saw_modifier = false;
        self.parts().filter_map(move |part| match part {
            Ok(JavaSyntaxListPart::Item(item)) => Some(
                if let Some(annotation) = item.cast_node::<Annotation<'source>>() {
                    if saw_modifier {
                        Ok(PartitionedModifierItem::TypeUseAnnotation(annotation))
                    } else {
                        Ok(PartitionedModifierItem::DeclarationAnnotation(annotation))
                    }
                } else if let Some(bogus) = item.cast_node::<BogusModifier<'source>>() {
                    saw_modifier = true;
                    Ok(PartitionedModifierItem::Bogus(bogus))
                } else if let Some(token) = item.token() {
                    saw_modifier = true;
                    Ok(PartitionedModifierItem::Token(token))
                } else {
                    Err(JavaSyntaxInvariantError {
                        node: JavaSyntaxKind::ParameterModifierList,
                        slot: 0,
                    })
                },
            ),
            Ok(JavaSyntaxListPart::Malformed(malformed)) => {
                Some(Ok(PartitionedModifierItem::Malformed(malformed)))
            }
            Ok(JavaSyntaxListPart::Missing(missing)) => {
                Some(Ok(PartitionedModifierItem::Missing(missing)))
            }
            Ok(JavaSyntaxListPart::Separator(_)) => None,
            Err(error) => Some(Err(error)),
        })
    }
}

#[cfg(test)]
impl<'source> CompilationUnit<'source> {
    pub(crate) fn syntax(&self) -> &JavaSyntaxNode<'source> {
        &self.syntax
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SwitchLabelCaseItem<'source> {
    Constant(CaseConstant<'source>),
    Pattern(CasePattern<'source>),
    Default(JavaSyntaxToken<'source>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SwitchLabelCaseEntry<'source> {
    pub item: SwitchLabelCaseItem<'source>,
    pub comma: Option<JavaSyntaxToken<'source>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SwitchBlockStatementGroupLabel<'source> {
    pub label: SwitchLabel<'source>,
    pub colon: Option<JavaSyntaxToken<'source>>,
}

#[derive(Clone, Copy, Debug)]
pub enum AnnotationElementValueContentItem<'source> {
    Expression(Expression<'source>),
    Annotation(Annotation<'source>),
    ArrayInitializer(AnnotationArrayInitializer<'source>),
}

impl<'source> AnnotationElementValueContent<'source> {
    pub fn classify(self) -> Option<AnnotationElementValueContentItem<'source>> {
        self.cast_family::<Expression<'source>>()
            .map(AnnotationElementValueContentItem::Expression)
            .or_else(|| {
                self.cast_node::<Annotation<'source>>()
                    .map(AnnotationElementValueContentItem::Annotation)
            })
            .or_else(|| {
                self.cast_node::<AnnotationArrayInitializer<'source>>()
                    .map(AnnotationElementValueContentItem::ArrayInitializer)
            })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnnotationArrayInitializerEntry<'source> {
    pub value: AnnotationElementValue<'source>,
    pub comma: Option<JavaSyntaxToken<'source>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ImportKind<'source> {
    SingleType(NameSyntax<'source>),
    TypeOnDemand(NameSyntax<'source>),
    SingleStatic(NameSyntax<'source>),
    StaticOnDemand(NameSyntax<'source>),
    SingleModule(NameSyntax<'source>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ModuleDirectiveRole<'source> {
    Requires {
        module: NameSyntax<'source>,
        is_static: bool,
        is_transitive: bool,
    },
    Exports {
        package: NameSyntax<'source>,
    },
    Opens {
        package: NameSyntax<'source>,
    },
    Uses {
        service: NameSyntax<'source>,
    },
    Provides {
        service: NameSyntax<'source>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModuleNameListEntry<'source> {
    pub name: NameSyntax<'source>,
    pub comma: Option<JavaSyntaxToken<'source>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StatementBody<'source> {
    Block(Block<'source>),
    Empty(EmptyStatement<'source>),
    Unbraced(Statement<'source>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WildcardBound<'source> {
    Extends(Type<'source>),
    Super(Type<'source>),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExpressionParentRole {
    ParenthesizedExpression,
    AssignmentLeft,
    AssignmentRight,
    ConditionalCondition,
    ConditionalTrueExpression,
    ConditionalFalseExpression,
    BinaryLeft,
    BinaryRight,
    UnaryOperand,
    PostfixOperand,
    CastOperand,
    InstanceofOperand,
    FieldAccessReceiver,
    MethodInvocationQualifier,
    MethodInvocationCallee,
    MethodReferenceReceiver,
    ArrayAccessArray,
    ArrayAccessIndex,
    ObjectCreationQualifier,
    ArrayCreationDimension,
    ClassLiteralTarget,
    LambdaBody,
    SwitchExpressionSelector,
    Argument,
    AnnotationElementValue,
    VariableInitializer,
    ExpressionStatement,
    IfCondition,
    WhileCondition,
    DoCondition,
    BasicForCondition,
    EnhancedForIterable,
    SynchronizedExpression,
    AssertCondition,
    AssertDetail,
    ReturnValue,
    ThrowValue,
    YieldValue,
    SwitchStatementSelector,
}

pub(crate) fn cast_compilation_unit(syntax: JavaSyntaxNode<'_>) -> Option<CompilationUnit<'_>> {
    <CompilationUnit<'_> as JavaNode<'_>>::cast(syntax)
}

fn token_iter<'source>(
    syntax: &JavaSyntaxNode<'source>,
) -> impl Iterator<Item = JavaSyntaxToken<'source>> + use<'source> {
    syntax.tokens()
}

fn first_token<'source>(syntax: &JavaSyntaxNode<'source>) -> Option<JavaSyntaxToken<'source>> {
    syntax.first_token()
}

fn last_token<'source>(syntax: &JavaSyntaxNode<'source>) -> Option<JavaSyntaxToken<'source>> {
    syntax.last_token()
}

fn operator_from_element(
    element: JavaRoleElement<'_>,
    single: fn(JavaSyntaxKind) -> Option<JavaOperatorKind>,
) -> JavaSyntaxResult<JavaOperator<'_>> {
    match element {
        JavaRoleElement::Token(token) => single(token.kind())
            .map(|kind| JavaOperator::single(kind, token))
            .ok_or(JavaSyntaxInvariantError {
                node: token.kind(),
                slot: 0,
            }),
        JavaRoleElement::Node(node) => {
            let mut components = [None; 4];
            let (kind, len) = match node.kind() {
                JavaSyntaxKind::GreaterThanOrEqualOperator => {
                    let operator =
                        GreaterThanOrEqualOperator::cast(node).ok_or(JavaSyntaxInvariantError {
                            node: node.kind(),
                            slot: 0,
                        })?;
                    components[0] = Some(operator.greater_than()?);
                    components[1] = Some(operator.assign()?);
                    (JavaOperatorKind::GtEq, 2)
                }
                JavaSyntaxKind::RightShiftOperator => {
                    let operator =
                        RightShiftOperator::cast(node).ok_or(JavaSyntaxInvariantError {
                            node: node.kind(),
                            slot: 0,
                        })?;
                    components[0] = Some(operator.first_greater_than()?);
                    components[1] = Some(operator.second_greater_than()?);
                    (JavaOperatorKind::RShift, 2)
                }
                JavaSyntaxKind::UnsignedRightShiftOperator => {
                    let operator =
                        UnsignedRightShiftOperator::cast(node).ok_or(JavaSyntaxInvariantError {
                            node: node.kind(),
                            slot: 0,
                        })?;
                    components[0] = Some(operator.first_greater_than()?);
                    components[1] = Some(operator.second_greater_than()?);
                    components[2] = Some(operator.third_greater_than()?);
                    (JavaOperatorKind::UnsignedRShift, 3)
                }
                JavaSyntaxKind::RightShiftAssignmentOperator => {
                    let operator = RightShiftAssignmentOperator::cast(node).ok_or(
                        JavaSyntaxInvariantError {
                            node: node.kind(),
                            slot: 0,
                        },
                    )?;
                    components[0] = Some(operator.first_greater_than()?);
                    components[1] = Some(operator.second_greater_than()?);
                    components[2] = Some(operator.assign()?);
                    (JavaOperatorKind::RShiftEq, 3)
                }
                JavaSyntaxKind::UnsignedRightShiftAssignmentOperator => {
                    let operator = UnsignedRightShiftAssignmentOperator::cast(node).ok_or(
                        JavaSyntaxInvariantError {
                            node: node.kind(),
                            slot: 0,
                        },
                    )?;
                    components[0] = Some(operator.first_greater_than()?);
                    components[1] = Some(operator.second_greater_than()?);
                    components[2] = Some(operator.third_greater_than()?);
                    components[3] = Some(operator.assign()?);
                    (JavaOperatorKind::UnsignedRShiftEq, 4)
                }
                _ => {
                    return Err(JavaSyntaxInvariantError {
                        node: node.kind(),
                        slot: 0,
                    });
                }
            };
            Ok(JavaOperator::composite(kind, components, len))
        }
    }
}

impl<'source> AssignmentOperatorRole<'source> {
    /// Returns the logical operator represented by this declared role.
    ///
    /// # Errors
    ///
    /// Returns an invariant error if the role has an invalid token or node shape.
    pub fn as_operator(&self) -> JavaSyntaxResult<JavaOperator<'source>> {
        operator_from_element(self.element, assignment_operator_kind)
    }
}

impl<'source> BinaryOperatorRole<'source> {
    /// Returns the logical operator represented by this declared role.
    ///
    /// # Errors
    ///
    /// Returns an invariant error if the role has an invalid token or node shape.
    pub fn as_operator(&self) -> JavaSyntaxResult<JavaOperator<'source>> {
        operator_from_element(self.element, binary_operator_kind)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InvocationNameSyntax<'source> {
    NameExpression(NameExpression<'source>),
    Identifier(JavaSyntaxToken<'source>),
}

fn invocation_name_syntax(
    element: JavaRoleElement<'_>,
) -> JavaSyntaxResult<InvocationNameSyntax<'_>> {
    match element {
        JavaRoleElement::Token(token) if token.kind() == JavaSyntaxKind::Identifier => {
            Ok(InvocationNameSyntax::Identifier(token))
        }
        JavaRoleElement::Node(node) => NameExpression::cast(node)
            .map(InvocationNameSyntax::NameExpression)
            .ok_or(JavaSyntaxInvariantError {
                node: node.kind(),
                slot: 0,
            }),
        JavaRoleElement::Token(token) => Err(JavaSyntaxInvariantError {
            node: token.kind(),
            slot: 0,
        }),
    }
}

macro_rules! impl_invocation_name {
    ($($role:ident),+ $(,)?) => {$(
        impl<'source> $role<'source> {
            #[allow(clippy::missing_errors_doc)]
            pub fn classify(self) -> JavaSyntaxResult<InvocationNameSyntax<'source>> {
                invocation_name_syntax(self.element)
            }
        }
    )+};
}

impl_invocation_name!(QualifiedInvocationName, UnqualifiedInvocationName);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LambdaModifierSyntax<'source> {
    Annotation(Annotation<'source>),
    Final(JavaSyntaxToken<'source>),
    Var(JavaSyntaxToken<'source>),
}

impl<'source> LambdaModifier<'source> {
    #[allow(clippy::missing_errors_doc)]
    pub fn classify(self) -> JavaSyntaxResult<LambdaModifierSyntax<'source>> {
        match self.element {
            JavaRoleElement::Node(node) => Annotation::cast(node)
                .map(LambdaModifierSyntax::Annotation)
                .ok_or(JavaSyntaxInvariantError {
                    node: node.kind(),
                    slot: 0,
                }),
            JavaRoleElement::Token(token) if token.kind() == JavaSyntaxKind::FinalKw => {
                Ok(LambdaModifierSyntax::Final(token))
            }
            JavaRoleElement::Token(token)
                if token.kind() == JavaSyntaxKind::Identifier
                    && crate::lexer::lexical_text_is(token.text(), "var") =>
            {
                Ok(LambdaModifierSyntax::Var(token))
            }
            JavaRoleElement::Token(token) => Err(JavaSyntaxInvariantError {
                node: token.kind(),
                slot: 0,
            }),
        }
    }
}

macro_rules! define_family_projection {
    (
        $category:ident => $value:ident {
            special { $($special:ident => $special_value:ident),* $(,)? }
            families { $($family:ident => $family_value:ident),+ $(,)? }
        }
    ) => {
        #[derive(Clone, Copy, Debug, Eq, PartialEq)]
        pub enum $value<'source> {
            $($special_value($special<'source>),)*
            $($family_value($family<'source>),)+
        }

        impl<'source> $category<'source> {
            #[allow(clippy::missing_errors_doc)]
            pub fn classify(self) -> JavaSyntaxResult<$value<'source>> {
                match self {
                    $(Self::$special(value) => Ok($value::$special_value(value)),)*
                    value => {
                        let syntax = *value.syntax();
                        $(if let Some(value) = $family::cast(syntax) {
                            return Ok($value::$family_value(value));
                        })+
                        Err(JavaSyntaxInvariantError {
                            node: syntax.kind(),
                            slot: 0,
                        })
                    }
                }
            }
        }
    };
}

define_family_projection! {
    LambdaBodySyntax => LambdaBodyValue {
        special { Block => Block, BogusLambdaBody => Bogus }
        families { Expression => Expression }
    }
}

define_family_projection! {
    MethodReferenceReceiverSyntax => MethodReferenceReceiverValue {
        special { BogusMethodReferenceReceiver => Bogus }
        families { Expression => Expression, Type => Type }
    }
}

macro_rules! define_node_role_projection {
    ($role:ident => $value:ident { $($node:ident),+ $(,)? }) => {
        #[derive(Clone, Copy, Debug, Eq, PartialEq)]
        pub enum $value<'source> {
            $($node($node<'source>),)+
        }

        impl<'source> $role<'source> {
            #[allow(clippy::missing_errors_doc)]
            pub fn classify(self) -> JavaSyntaxResult<$value<'source>> {
                $(if let Some(value) = self.cast_node::<$node<'source>>() {
                    return Ok($value::$node(value));
                })+
                Err(JavaSyntaxInvariantError {
                    node: self
                        .first_token()
                        .map_or(JavaSyntaxKind::ErrorNode, |token| token.kind()),
                    slot: 0,
                })
            }
        }
    };
}

define_node_role_projection! {
    ForStatementForm => ForStatementFormSyntax {
        BasicForStatement,
        EnhancedForStatement,
    }
}

define_node_role_projection! {
    ForInitializerValue => ForInitializerSyntax {
        LocalVariableDeclaration,
        StatementExpressionList,
    }
}

define_node_role_projection! {
    LocalTypeDeclaration => LocalTypeDeclarationSyntax {
        ClassDeclaration,
        InterfaceDeclaration,
        BogusTypeDeclaration,
    }
}

define_node_role_projection! {
    VariableAccessExpression => VariableAccessSyntax {
        NameExpression,
        FieldAccessExpression,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SwitchRuleBodySyntax<'source> {
    Expression(Expression<'source>),
    Block(Block<'source>),
    ThrowStatement(ThrowStatement<'source>),
}

impl<'source> SwitchRuleBody<'source> {
    #[allow(clippy::missing_errors_doc)]
    pub fn classify(self) -> JavaSyntaxResult<SwitchRuleBodySyntax<'source>> {
        if let Some(value) = self.cast_family::<Expression<'source>>() {
            Ok(SwitchRuleBodySyntax::Expression(value))
        } else if let Some(value) = self.cast_node::<Block<'source>>() {
            Ok(SwitchRuleBodySyntax::Block(value))
        } else if let Some(value) = self.cast_node::<ThrowStatement<'source>>() {
            Ok(SwitchRuleBodySyntax::ThrowStatement(value))
        } else {
            Err(JavaSyntaxInvariantError {
                node: self
                    .first_token()
                    .map_or(JavaSyntaxKind::ErrorNode, |token| token.kind()),
                slot: 0,
            })
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SwitchLabelItemSyntax<'source> {
    CaseConstant(CaseConstant<'source>),
    CasePattern(CasePattern<'source>),
    BogusSwitchLabelItem(BogusSwitchLabelItem<'source>),
    Default(JavaSyntaxToken<'source>),
}

impl<'source> SwitchLabelItem<'source> {
    #[allow(clippy::missing_errors_doc)]
    pub fn classify(self) -> JavaSyntaxResult<SwitchLabelItemSyntax<'source>> {
        if let Some(value) = self.cast_node::<CaseConstant<'source>>() {
            Ok(SwitchLabelItemSyntax::CaseConstant(value))
        } else if let Some(value) = self.cast_node::<CasePattern<'source>>() {
            Ok(SwitchLabelItemSyntax::CasePattern(value))
        } else if let Some(value) = self.cast_node::<BogusSwitchLabelItem<'source>>() {
            Ok(SwitchLabelItemSyntax::BogusSwitchLabelItem(value))
        } else if let Some(token) = self.token()
            && token.kind() == JavaSyntaxKind::DefaultKw
        {
            Ok(SwitchLabelItemSyntax::Default(token))
        } else {
            Err(JavaSyntaxInvariantError {
                node: self
                    .first_token()
                    .map_or(JavaSyntaxKind::ErrorNode, |token| token.kind()),
                slot: 0,
            })
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VariableTypeSyntax<'source> {
    Type(Type<'source>),
    Var(JavaSyntaxToken<'source>),
}

fn classify_variable_type(
    element: JavaRoleElement<'_>,
) -> JavaSyntaxResult<VariableTypeSyntax<'_>> {
    if let Some(value) = element.cast_family::<Type<'_>>() {
        Ok(VariableTypeSyntax::Type(value))
    } else if let Some(token) = element.token() {
        Ok(VariableTypeSyntax::Var(token))
    } else {
        Err(JavaSyntaxInvariantError {
            node: JavaSyntaxKind::ErrorNode,
            slot: 0,
        })
    }
}

macro_rules! impl_variable_type {
    ($($role:ident),+ $(,)?) => {$(
        impl<'source> $role<'source> {
            #[allow(clippy::missing_errors_doc)]
            pub fn classify(self) -> JavaSyntaxResult<VariableTypeSyntax<'source>> {
                classify_variable_type(self.element)
            }
        }
    )+};
}

impl_variable_type!(
    LocalVariableType,
    EnhancedForVariableType,
    ResourceVariableType
);

impl<'source> CatchParameterTypes<'source> {
    #[allow(clippy::missing_errors_doc)]
    pub fn as_type(self) -> JavaSyntaxResult<Type<'source>> {
        self.cast_family::<Type<'source>>()
            .ok_or(JavaSyntaxInvariantError {
                node: self
                    .first_token()
                    .map_or(JavaSyntaxKind::ErrorNode, |token| token.kind()),
                slot: 0,
            })
    }
}

impl Expression<'_> {
    /// Returns this expression's grammar role from its parent's fixed slot.
    #[must_use]
    #[allow(clippy::too_many_lines)]
    pub fn parent_role(&self) -> Option<ExpressionParentRole> {
        let syntax = self.syntax();
        let parent = syntax.parent()?;
        let slot = syntax.index();

        macro_rules! role {
            ($module:ident, $field:ident, $role:ident) => {
                (slot == crate::shape::$module::Slot::$field as usize)
                    .then_some(ExpressionParentRole::$role)
            };
        }

        match parent.kind() {
            JavaSyntaxKind::ParenthesizedExpression => role!(
                parenthesized_expression,
                expression,
                ParenthesizedExpression
            ),
            JavaSyntaxKind::AssignmentExpression => {
                role!(assignment_expression, left, AssignmentLeft)
                    .or_else(|| role!(assignment_expression, right, AssignmentRight))
            }
            JavaSyntaxKind::ConditionalExpression => {
                role!(conditional_expression, condition, ConditionalCondition)
                    .or_else(|| {
                        role!(
                            conditional_expression,
                            then_expression,
                            ConditionalTrueExpression
                        )
                    })
                    .or_else(|| {
                        role!(
                            conditional_expression,
                            else_expression,
                            ConditionalFalseExpression
                        )
                    })
            }
            JavaSyntaxKind::BinaryExpression => role!(binary_expression, left, BinaryLeft)
                .or_else(|| role!(binary_expression, right, BinaryRight)),
            JavaSyntaxKind::UnaryExpression => role!(unary_expression, operand, UnaryOperand),
            JavaSyntaxKind::PostfixExpression => role!(postfix_expression, operand, PostfixOperand),
            JavaSyntaxKind::CastExpression => role!(cast_expression, expression, CastOperand),
            JavaSyntaxKind::InstanceofExpression => {
                role!(instanceof_expression, expression, InstanceofOperand)
            }
            JavaSyntaxKind::FieldAccessExpression => {
                role!(field_access_expression, receiver, FieldAccessReceiver)
            }
            JavaSyntaxKind::QualifiedMethodInvocation => role!(
                qualified_method_invocation,
                receiver,
                MethodInvocationQualifier
            )
            .or_else(|| role!(qualified_method_invocation, name, MethodInvocationCallee)),
            JavaSyntaxKind::UnqualifiedMethodInvocation => {
                role!(unqualified_method_invocation, name, MethodInvocationCallee)
            }
            JavaSyntaxKind::MethodReferenceExpression => role!(
                method_reference_expression,
                receiver,
                MethodReferenceReceiver
            ),
            JavaSyntaxKind::ArrayAccessExpression => {
                role!(array_access_expression, array, ArrayAccessArray)
                    .or_else(|| role!(array_access_expression, index, ArrayAccessIndex))
            }
            JavaSyntaxKind::ObjectCreationExpression => role!(
                object_creation_expression,
                qualifier,
                ObjectCreationQualifier
            ),
            JavaSyntaxKind::DimExpression => {
                role!(dim_expression, expression, ArrayCreationDimension)
            }
            JavaSyntaxKind::ClassLiteralExpression => {
                role!(class_literal_expression, target, ClassLiteralTarget)
            }
            JavaSyntaxKind::LambdaExpression => role!(lambda_expression, body, LambdaBody),
            JavaSyntaxKind::SwitchExpression => {
                role!(switch_expression, selector, SwitchExpressionSelector)
            }
            JavaSyntaxKind::ExpressionList => Some(ExpressionParentRole::Argument),
            JavaSyntaxKind::AnnotationElementValue => {
                role!(annotation_element_value, value, AnnotationElementValue)
            }
            JavaSyntaxKind::VariableInitializer => {
                role!(variable_initializer, value, VariableInitializer)
            }
            JavaSyntaxKind::ExpressionStatement => {
                role!(expression_statement, expression, ExpressionStatement)
            }
            JavaSyntaxKind::IfStatement => role!(if_statement, condition, IfCondition),
            JavaSyntaxKind::WhileStatement => role!(while_statement, condition, WhileCondition),
            JavaSyntaxKind::DoStatement => role!(do_statement, condition, DoCondition),
            JavaSyntaxKind::BasicForStatement => {
                role!(basic_for_statement, condition, BasicForCondition)
            }
            JavaSyntaxKind::EnhancedForStatement => {
                role!(enhanced_for_statement, iterable, EnhancedForIterable)
            }
            JavaSyntaxKind::SynchronizedStatement => {
                role!(synchronized_statement, expression, SynchronizedExpression)
            }
            JavaSyntaxKind::AssertStatement => role!(assert_statement, condition, AssertCondition)
                .or_else(|| role!(assert_statement, message, AssertDetail)),
            JavaSyntaxKind::ReturnStatement => role!(return_statement, expression, ReturnValue),
            JavaSyntaxKind::ThrowStatement => role!(throw_statement, expression, ThrowValue),
            JavaSyntaxKind::YieldStatement => role!(yield_statement, expression, YieldValue),
            JavaSyntaxKind::SwitchStatement => {
                role!(switch_statement, selector, SwitchStatementSelector)
            }
            _ => None,
        }
    }
}
