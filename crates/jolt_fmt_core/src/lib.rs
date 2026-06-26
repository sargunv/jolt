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

/// Java formatter compatibility profile.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum JavaProfile {
    /// Compatibility target for Google Java Format.
    #[default]
    Google,
    /// Compatibility target for Google Java Format AOSP mode.
    Aosp,
    /// Compatibility target for Palantir Java Format.
    Palantir,
}

/// Kotlin formatter compatibility profile.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum KotlinProfile {
    /// Compatibility target for ktfmt default/Meta style.
    #[default]
    Meta,
    /// Compatibility target for ktfmt Google style.
    Google,
    /// Compatibility target for ktfmt Kotlin language style.
    KotlinLang,
}

/// Formatter options shared by CLI, dprint, tests, and direct engine callers.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct FormatOptions {
    /// Java profile used when formatting Java source.
    pub java_profile: JavaProfile,
    /// Kotlin profile used when formatting Kotlin source.
    pub kotlin_profile: KotlinProfile,
}

impl FormatOptions {
    /// Returns options with a different Java profile.
    #[must_use]
    pub const fn with_java_profile(mut self, java_profile: JavaProfile) -> Self {
        self.java_profile = java_profile;
        self
    }

    /// Returns options with a different Kotlin profile.
    #[must_use]
    pub const fn with_kotlin_profile(mut self, kotlin_profile: KotlinProfile) -> Self {
        self.kotlin_profile = kotlin_profile;
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
