use jolt_text::TextSize;

use crate::RawSyntaxKind;

use super::GreenTrivia;

/// An immutable green token.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct GreenToken {
    kind: RawSyntaxKind,
    token_text_len: TextSize,
    text_len: TextSize,
    leading: Box<[GreenTrivia]>,
    trailing: Box<[GreenTrivia]>,
}

impl GreenToken {
    /// Creates a green token with leading and trailing trivia.
    #[must_use]
    pub(crate) fn with_trivia(
        kind: RawSyntaxKind,
        token_text_len: TextSize,
        leading: impl IntoIterator<Item = GreenTrivia>,
        trailing: impl IntoIterator<Item = GreenTrivia>,
    ) -> Self {
        let leading = leading.into_iter().collect::<Box<[_]>>();
        let trailing = trailing.into_iter().collect::<Box<[_]>>();
        let text_len = token_text_len + trivia_text_len(&leading) + trivia_text_len(&trailing);

        Self {
            kind,
            token_text_len,
            text_len,
            leading,
            trailing,
        }
    }

    /// Returns the token kind.
    #[must_use]
    pub(crate) fn kind(&self) -> RawSyntaxKind {
        self.kind
    }

    /// Returns the byte length of this token including attached trivia.
    #[must_use]
    pub(crate) fn text_len(&self) -> TextSize {
        self.text_len
    }

    /// Returns the byte length of this token excluding attached trivia.
    #[must_use]
    pub(crate) fn token_text_len(&self) -> TextSize {
        self.token_text_len
    }

    /// Returns trivia attached before this token.
    #[must_use]
    pub(crate) fn leading(&self) -> &[GreenTrivia] {
        &self.leading
    }

    /// Returns trivia attached after this token.
    #[must_use]
    pub(crate) fn trailing(&self) -> &[GreenTrivia] {
        &self.trailing
    }

    /// Returns true if both handles point at the same green token allocation.
    #[must_use]
    pub(crate) fn ptr_eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

fn trivia_text_len(trivia: &[GreenTrivia]) -> TextSize {
    TextSize::new(trivia.iter().map(|piece| piece.text_len().get()).sum())
}
