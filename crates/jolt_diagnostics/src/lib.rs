//! Shared plain diagnostic data for Jolt engines.

use std::fmt;

use jolt_text::TextRange;

/// A diagnostic produced by a Jolt engine stage.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    /// Stable machine-readable code.
    pub code: DiagnosticCodeId,
    /// Diagnostic severity.
    pub severity: Severity,
    /// Runtime stage that produced the diagnostic.
    pub stage: DiagnosticStage,
    /// Human-readable message.
    pub message: String,
    /// Source range, when the diagnostic has one.
    pub range: Option<TextRange>,
}

/// A stable machine-readable diagnostic code.
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct DiagnosticCodeId(&'static str);

impl DiagnosticCodeId {
    /// Creates a stable diagnostic code identifier.
    #[must_use]
    pub const fn new(code: &'static str) -> Self {
        Self(code)
    }

    /// Returns this diagnostic code as a string.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

impl fmt::Debug for DiagnosticCodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("DiagnosticCodeId").field(&self.0).finish()
    }
}

impl fmt::Display for DiagnosticCodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

/// A typed source for stable diagnostic codes.
pub trait DiagnosticCode {
    /// Returns the stable machine-readable code identifier.
    fn id(&self) -> DiagnosticCodeId;
}

/// Diagnostic severity.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Severity {
    /// Jolt hit an implementation bug or invariant failure.
    InternalError,
    /// User source or configuration is invalid.
    Error,
    /// User-visible warning.
    Warning,
    /// User-visible note.
    Note,
}

/// Stage that produced a diagnostic.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DiagnosticStage {
    /// Configuration loading or validation.
    Config,
    /// Lexical analysis.
    Lexer,
    /// Parsing.
    Parser,
    /// Formatting.
    Formatter,
}

/// Syntax production outcome.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SyntaxOutcome {
    /// Syntax was produced without diagnostics.
    Clean,
    /// Syntax was produced after recoverable diagnostics.
    Recovered,
    /// Syntax could not be produced as a trustworthy complete tree.
    Aborted,
}
