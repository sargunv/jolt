use jolt_text::TextRange;

/// A parser diagnostic with a source range.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParseDiagnostic {
    kind: ParseDiagnosticKind,
    range: TextRange,
}

impl ParseDiagnostic {
    /// Creates a parser diagnostic.
    #[must_use]
    pub fn new(kind: ParseDiagnosticKind, range: TextRange) -> Self {
        Self { kind, range }
    }

    /// Creates a parser diagnostic from a plain message.
    #[must_use]
    pub fn message(message: impl Into<String>, range: TextRange) -> Self {
        Self::new(ParseDiagnosticKind::Message(message.into()), range)
    }

    /// Returns the diagnostic kind.
    #[must_use]
    pub const fn kind(&self) -> &ParseDiagnosticKind {
        &self.kind
    }

    /// Returns the source range associated with this diagnostic.
    #[must_use]
    pub const fn range(&self) -> TextRange {
        self.range
    }
}

/// The language-neutral kind of a parser diagnostic.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ParseDiagnosticKind {
    /// A parser-supplied diagnostic message.
    Message(String),
}
