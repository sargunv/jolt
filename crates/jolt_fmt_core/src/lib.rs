//! Public formatter engine API for Jolt.

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

/// Diagnostic severity.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Severity {
    /// A fatal formatting problem.
    Error,
    /// A non-fatal problem callers may want to surface.
    Warning,
    /// Informational diagnostic.
    Note,
}

/// A formatter diagnostic.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    severity: Severity,
    message: String,
    code: Option<String>,
    range: Option<TextRange>,
}

impl Diagnostic {
    /// Creates a diagnostic.
    #[must_use]
    pub fn new(severity: Severity, message: impl Into<String>) -> Self {
        Self {
            severity,
            message: message.into(),
            code: None,
            range: None,
        }
    }

    /// Creates an error diagnostic.
    #[must_use]
    pub fn error(message: impl Into<String>) -> Self {
        Self::new(Severity::Error, message)
    }

    /// Returns this diagnostic with a stable machine-readable code.
    #[must_use]
    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.code = Some(code.into());
        self
    }

    /// Returns this diagnostic with a source range.
    #[must_use]
    pub const fn with_range(mut self, range: TextRange) -> Self {
        self.range = Some(range);
        self
    }

    /// Returns the severity.
    #[must_use]
    pub const fn severity(&self) -> Severity {
        self.severity
    }

    /// Returns the human-readable message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Returns the stable machine-readable code, if present.
    #[must_use]
    pub fn code(&self) -> Option<&str> {
        self.code.as_deref()
    }

    /// Returns the source range, if present.
    #[must_use]
    pub const fn range(&self) -> Option<TextRange> {
        self.range
    }
}

/// Formatter output plus diagnostics.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FormatResult {
    formatted_source: String,
    diagnostics: Vec<Diagnostic>,
}

impl FormatResult {
    /// Creates a formatter result.
    #[must_use]
    pub fn new(formatted_source: impl Into<String>, diagnostics: Vec<Diagnostic>) -> Self {
        Self {
            formatted_source: formatted_source.into(),
            diagnostics,
        }
    }

    /// Returns formatted source text.
    #[must_use]
    pub fn formatted_source(&self) -> &str {
        &self.formatted_source
    }

    /// Consumes the result and returns formatted source text.
    #[must_use]
    pub fn into_formatted_source(self) -> String {
        self.formatted_source
    }

    /// Returns formatter diagnostics.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Returns true when at least one diagnostic has error severity.
    #[must_use]
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity() == Severity::Error)
    }
}

/// Formats source text for `language` using `options`.
///
/// Until the language-specific printers land, this contract preserves the input
/// text and reports that formatting is not implemented. This lets wrappers wire
/// against the stable API without risking destructive output.
#[must_use]
pub fn format_source(source: &str, language: Language, _options: &FormatOptions) -> FormatResult {
    let message = match language {
        Language::Java => "Java formatting is not implemented yet",
        Language::Kotlin => "Kotlin formatting is not implemented yet",
    };
    let diagnostic = Diagnostic::error(message).with_code("format.unimplemented");

    FormatResult::new(source, vec![diagnostic])
}
