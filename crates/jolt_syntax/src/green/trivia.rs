use std::sync::Arc;

use jolt_text::TextSize;

/// A trivia kind stored in shared green tokens.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TriviaKind {
    /// Horizontal whitespace.
    Whitespace,
    /// A source line break.
    Newline,
    /// A line comment.
    LineComment,
    /// A block comment.
    BlockComment,
    /// A documentation comment.
    DocComment,
    /// Input ignored by the language specification but preserved for lossless output.
    Ignored,
}

/// A lossless trivia piece attached to a green token.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct GreenTrivia {
    kind: TriviaKind,
    text: Arc<str>,
    text_len: TextSize,
}

impl GreenTrivia {
    /// Creates a green trivia piece.
    #[must_use]
    pub fn new(kind: TriviaKind, text: impl Into<Arc<str>>) -> Self {
        let text = text.into();
        let text_len = TextSize::new(text.len());

        Self {
            kind,
            text,
            text_len,
        }
    }

    /// Returns the trivia kind.
    #[must_use]
    pub const fn kind(&self) -> TriviaKind {
        self.kind
    }

    /// Returns the raw trivia text.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Returns the byte length of the raw trivia text.
    #[must_use]
    pub const fn text_len(&self) -> TextSize {
        self.text_len
    }
}
