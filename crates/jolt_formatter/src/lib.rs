//! The formatter facade for the CLI and the dprint plugin: language detection
//! and one `format` entry point.

use std::path::Path;

pub use jolt_fmt::{FormatOptions, Sink};
pub use jolt_syntax::{Diagnostic, LineIndex};

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
    Refuse,
    Format,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FormatResult {
    /// Output was written to the sink; `halted` when the sink broke early.
    Formatted { halted: bool },
    /// `Refuse` and the source has syntax errors; nothing was written.
    Refused(Vec<Diagnostic>),
}

/// Formats `source` as `language` into `sink`.
pub fn format<S: Sink + ?Sized>(
    source: &str,
    language: Language,
    options: &FormatOptions,
    on_error: OnSyntaxError,
    sink: &mut S,
) -> FormatResult {
    match language {
        Language::Java => {
            let parse = jolt_java_syntax::parse(source);
            if on_error == OnSyntaxError::Refuse && !parse.diagnostics().is_empty() {
                return FormatResult::Refused(parse.diagnostics().to_vec());
            }
            let halted = jolt_java_fmt::format(&parse, options, sink).is_break();
            FormatResult::Formatted { halted }
        }
    }
}
