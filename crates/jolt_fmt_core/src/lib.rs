//! Public formatter engine API for Jolt.

pub use jolt_diagnostics::{
    Diagnostic, DiagnosticCode, DiagnosticCodeId, DiagnosticStage, Severity, SyntaxOutcome,
};
pub use jolt_fmt_ir::{RenderControl, RenderSink};
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
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FormatSinkResult<E> {
    Complete {
        diagnostics: Vec<Diagnostic>,
    },
    Halted {
        diagnostics: Vec<Diagnostic>,
    },
    Blocked {
        diagnostics: Vec<Diagnostic>,
    },
    SinkError {
        diagnostics: Vec<Diagnostic>,
        error: E,
    },
}

impl<E> FormatSinkResult<E> {
    #[must_use]
    pub fn is_blocked(&self) -> bool {
        matches!(self, Self::Blocked { .. })
    }

    #[must_use]
    pub fn is_halted(&self) -> bool {
        matches!(self, Self::Halted { .. })
    }

    pub fn diagnostics(&self) -> &[Diagnostic] {
        match self {
            Self::Complete { diagnostics }
            | Self::Halted { diagnostics }
            | Self::Blocked { diagnostics }
            | Self::SinkError { diagnostics, .. } => diagnostics,
        }
    }
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

/// Formats source text for `language` into a render sink using `options`.
pub fn format_source_to_sink<S: RenderSink + ?Sized>(
    source: &str,
    language: Language,
    options: &FormatOptions,
    sink: &mut S,
) -> FormatSinkResult<S::Error> {
    if language == Language::Java {
        return format_java_source_to_sink(source, *options, sink);
    }

    let message = match language {
        Language::Kotlin => "Kotlin formatting is not implemented yet",
        Language::Java => unreachable!("Java formatting is handled above"),
    };
    let diagnostic = Diagnostic {
        code: FormatDiagnosticCode::Unimplemented.id(),
        severity: Severity::Error,
        stage: DiagnosticStage::Formatter,
        message: message.to_owned(),
        range: None,
    };

    FormatSinkResult::Blocked {
        diagnostics: vec![diagnostic],
    }
}

fn format_java_source_to_sink<S: RenderSink + ?Sized>(
    source: &str,
    options: FormatOptions,
    sink: &mut S,
) -> FormatSinkResult<S::Error> {
    let java_options = jolt_java_fmt::JavaFormatOptions {
        line_width: options.line_width,
        indent_width: options.indent_width,
        use_tabs: options.use_tabs,
    };
    match jolt_java_fmt::format_source_to_sink(source, &java_options, sink) {
        jolt_java_fmt::JavaFormatSinkResult::Complete { diagnostics } => {
            FormatSinkResult::Complete { diagnostics }
        }
        jolt_java_fmt::JavaFormatSinkResult::Halted { diagnostics } => {
            FormatSinkResult::Halted { diagnostics }
        }
        jolt_java_fmt::JavaFormatSinkResult::Blocked { diagnostics } => {
            FormatSinkResult::Blocked { diagnostics }
        }
        jolt_java_fmt::JavaFormatSinkResult::SinkError { diagnostics, error } => {
            FormatSinkResult::SinkError { diagnostics, error }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct TestSink {
        text: String,
    }

    impl RenderSink for TestSink {
        type Error = std::convert::Infallible;

        fn write_str(&mut self, text: &str) -> Result<RenderControl, Self::Error> {
            self.text.push_str(text);
            Ok(RenderControl::Continue)
        }
    }

    #[test]
    fn kotlin_formatter_still_blocks_without_output() {
        let mut sink = TestSink::default();
        let result = format_source_to_sink(
            "fun main() {}",
            Language::Kotlin,
            &FormatOptions::default(),
            &mut sink,
        );

        assert!(result.is_blocked());
        assert_eq!(sink.text, "");
        assert_eq!(result.diagnostics().len(), 1);

        let diagnostic = &result.diagnostics()[0];
        assert_eq!(
            diagnostic.code.as_str(),
            FormatDiagnosticCode::Unimplemented.id().as_str()
        );
        assert_eq!(diagnostic.severity, Severity::Error);
        assert_eq!(diagnostic.stage, DiagnosticStage::Formatter);
        assert_eq!(diagnostic.range, None);
    }

    #[test]
    fn java_formatter_can_render_to_sink_without_owned_output() {
        let mut sink = TestSink::default();
        let result = format_source_to_sink(
            "class A {\n}\n",
            Language::Java,
            &FormatOptions::default(),
            &mut sink,
        );

        assert!(!result.is_blocked());
        assert!(result.diagnostics().is_empty());
        assert!(matches!(result, FormatSinkResult::Complete { .. }));
        assert_eq!(sink.text, "class A {\n}\n");
    }
}
