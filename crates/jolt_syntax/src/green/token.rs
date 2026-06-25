use std::sync::Arc;

use jolt_text::TextSize;

use crate::RawSyntaxKind;

use super::GreenTrivia;

/// An immutable green token.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct GreenToken(Arc<GreenTokenData>);

#[derive(Debug, Eq, Hash, PartialEq)]
struct GreenTokenData {
    kind: RawSyntaxKind,
    text: Arc<str>,
    token_text_len: TextSize,
    text_len: TextSize,
    leading: Box<[GreenTrivia]>,
    trailing: Box<[GreenTrivia]>,
}

impl GreenToken {
    /// Creates a green token without trivia.
    #[must_use]
    pub fn new(kind: RawSyntaxKind, text: impl Into<Arc<str>>) -> Self {
        Self::with_trivia(kind, text, [], [])
    }

    /// Creates a green token with leading and trailing trivia.
    #[must_use]
    pub fn with_trivia(
        kind: RawSyntaxKind,
        text: impl Into<Arc<str>>,
        leading: impl IntoIterator<Item = GreenTrivia>,
        trailing: impl IntoIterator<Item = GreenTrivia>,
    ) -> Self {
        let text = text.into();
        let token_text_len = TextSize::new(text.len());
        let leading = leading.into_iter().collect::<Box<[_]>>();
        let trailing = trailing.into_iter().collect::<Box<[_]>>();
        let text_len = token_text_len + trivia_text_len(&leading) + trivia_text_len(&trailing);

        Self(Arc::new(GreenTokenData {
            kind,
            text,
            token_text_len,
            text_len,
            leading,
            trailing,
        }))
    }

    /// Returns the token kind.
    #[must_use]
    pub fn kind(&self) -> RawSyntaxKind {
        self.0.kind
    }

    /// Returns the token text without attached trivia.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.0.text
    }

    /// Returns the byte length of this token including attached trivia.
    #[must_use]
    pub fn text_len(&self) -> TextSize {
        self.0.text_len
    }

    /// Returns the byte length of this token excluding attached trivia.
    #[must_use]
    pub fn token_text_len(&self) -> TextSize {
        self.0.token_text_len
    }

    /// Returns trivia attached before this token.
    #[must_use]
    pub fn leading(&self) -> &[GreenTrivia] {
        &self.0.leading
    }

    /// Returns trivia attached after this token.
    #[must_use]
    pub fn trailing(&self) -> &[GreenTrivia] {
        &self.0.trailing
    }

    /// Returns true if both handles point at the same green token allocation.
    #[must_use]
    pub fn ptr_eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

fn trivia_text_len(trivia: &[GreenTrivia]) -> TextSize {
    TextSize::new(trivia.iter().map(|piece| piece.text_len().get()).sum())
}
