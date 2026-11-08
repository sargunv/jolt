//! The formatter facade for the CLI and the dprint plugin: language detection
//! and one `format` entry point.

use std::{fmt, path::Path};

pub use jolt_fmt::{FormatOptions, Sink};
use jolt_syntax::LineIndex;

/// A source language Jolt formats.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Language {
    Java,
}

impl Language {
    /// Detects a language from a file extension (`.java`).
    #[must_use]
    pub fn from_path(path: &Path) -> Option<Self> {
        match path.extension().and_then(|extension| extension.to_str()) {
            Some("java") => Some(Self::Java),
            _ => None,
        }
    }
}

/// What to do with a source that has syntax errors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OnSyntaxError {
    /// Write nothing and return the errors ([`Outcome::Refused`]).
    Refuse,
    /// Format anyway: the parsed pieces get the normal rules, and what the
    /// parser could not shape prints as written.
    Format,
}

/// What [`format()`] did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// Output was written to the sink; `halted` when the sink broke early.
    Formatted { halted: bool },
    /// `Refuse` and the source has syntax errors; nothing was written.
    Refused(Vec<SyntaxError>),
}

/// A syntax error in the source. Displays as `line:column: code: message`,
/// with a 1-based line and column.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SyntaxError {
    line: u32,
    column: u32,
    code: &'static str,
    message: String,
}

impl fmt::Display for SyntaxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            line,
            column,
            code,
            message,
        } = self;
        write!(f, "{line}:{column}: {code}: {message}")
    }
}

/// Formats `source` as `language` into `sink`.
pub fn format<S: Sink + ?Sized>(
    source: &str,
    language: Language,
    options: &FormatOptions,
    on_error: OnSyntaxError,
    sink: &mut S,
) -> Outcome {
    match language {
        Language::Java => {
            let parse = jolt_java_syntax::parse(source);
            if on_error == OnSyntaxError::Refuse && !parse.diagnostics().is_empty() {
                let lines = LineIndex::new(source);
                let errors = parse
                    .diagnostics()
                    .iter()
                    .map(|diagnostic| {
                        let at = lines.line_col(diagnostic.range.start());
                        SyntaxError {
                            line: at.line + 1,
                            column: at.col + 1,
                            code: diagnostic.code,
                            message: diagnostic.message.clone(),
                        }
                    })
                    .collect();
                return Outcome::Refused(errors);
            }
            let halted = jolt_java_fmt::format(&parse, options, sink).is_break();
            Outcome::Formatted { halted }
        }
    }
}
