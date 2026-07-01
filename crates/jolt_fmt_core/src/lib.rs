//! Public formatter engine API for Jolt.

pub use jolt_diagnostics::{
    Diagnostic, DiagnosticCode, DiagnosticCodeId, DiagnosticStage, Severity, SyntaxOutcome,
};
pub use jolt_text::{LineCol, LineIndex, TextRange, TextSize};

/// Source language to format.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Language {
    /// Java source, typically `.java`.
    Java,
    /// Kotlin source, typically `.kt` or `.kts`.
    Kotlin,
}

/// Formatter options shared by CLI, dprint, tests, and direct engine callers.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct FormatOptions {
    /// Preferred maximum rendered line width.
    pub line_width: u16,
    /// Number of spaces per indentation level when `use_tabs` is false.
    pub indent_width: u8,
    /// Whether indentation should use tabs instead of spaces.
    pub use_tabs: bool,
}

impl Default for FormatOptions {
    fn default() -> Self {
        Self {
            line_width: 80,
            indent_width: 2,
            use_tabs: false,
        }
    }
}

impl FormatOptions {
    /// Returns options with a different line width.
    #[must_use]
    pub const fn with_line_width(mut self, line_width: u16) -> Self {
        self.line_width = line_width;
        self
    }

    /// Returns options with a different indentation width.
    #[must_use]
    pub const fn with_indent_width(mut self, indent_width: u8) -> Self {
        self.indent_width = indent_width;
        self
    }

    /// Returns options with tab indentation enabled or disabled.
    #[must_use]
    pub const fn with_tabs(mut self, use_tabs: bool) -> Self {
        self.use_tabs = use_tabs;
        self
    }
}

/// Formatter operation status.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum FormatStatus {
    /// The source changed.
    Formatted,
    /// The source was already formatted.
    Unchanged,
    /// Formatting was blocked and no formatted source was produced.
    Blocked,
}

/// Formatter output plus diagnostics.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FormatResult {
    /// Formatted source text, absent when formatting was blocked.
    pub formatted_source: Option<String>,
    /// Diagnostics produced while formatting.
    pub diagnostics: Vec<Diagnostic>,
    /// Formatter operation status.
    pub status: FormatStatus,
}

/// Stable formatter diagnostic codes.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum FormatDiagnosticCode {
    Unimplemented,
}

impl DiagnosticCode for FormatDiagnosticCode {
    fn id(&self) -> DiagnosticCodeId {
        match self {
            Self::Unimplemented => DiagnosticCodeId::new("format.unimplemented"),
        }
    }
}

/// Formats source text for `language` using `options`.
///
/// Until the language-specific printers land, this contract blocks formatting
/// and reports that formatting is not implemented. This lets wrappers wire
/// against the stable API without risking destructive output.
#[must_use]
pub fn format_source(_source: &str, language: Language, _options: &FormatOptions) -> FormatResult {
    let message = match language {
        Language::Java => "Java formatting is not implemented yet",
        Language::Kotlin => "Kotlin formatting is not implemented yet",
    };
    let diagnostic = Diagnostic {
        code: FormatDiagnosticCode::Unimplemented.id(),
        severity: Severity::Error,
        stage: DiagnosticStage::Formatter,
        message: message.to_owned(),
        range: None,
    };

    FormatResult {
        formatted_source: None,
        diagnostics: vec![diagnostic],
        status: FormatStatus::Blocked,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unimplemented_formatter_blocks_without_output() {
        let result = format_source("class A {}", Language::Java, &FormatOptions::default());

        assert_eq!(result.status, FormatStatus::Blocked);
        assert_eq!(result.formatted_source, None);
        assert_eq!(result.diagnostics.len(), 1);

        let diagnostic = &result.diagnostics[0];
        assert_eq!(
            diagnostic.code.as_str(),
            FormatDiagnosticCode::Unimplemented.id().as_str()
        );
        assert_eq!(diagnostic.severity, Severity::Error);
        assert_eq!(diagnostic.stage, DiagnosticStage::Formatter);
        assert_eq!(diagnostic.range, None);
    }
}
