//! Shared formatter engine boundary for Jolt's CLI and dprint plugin.

use jolt_diagnostics::Diagnostic;
use jolt_fmt_ir::RenderSink;

/// Source language to format.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Language {
    /// Java source, typically `.java`.
    Java,
}

/// Formatter options shared by CLI and dprint.
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FormatSinkResult<E> {
    Complete,
    Halted,
    Blocked { diagnostics: Vec<Diagnostic> },
    SinkError { error: E },
}

/// Formats source text for `language` into a render sink using `options`.
pub fn format_source_to_sink<S: RenderSink + ?Sized>(
    source: &str,
    language: Language,
    options: &FormatOptions,
    sink: &mut S,
) -> FormatSinkResult<S::Error> {
    match language {
        Language::Java => format_java_source_to_sink(source, *options, sink),
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
        jolt_java_fmt::JavaFormatSinkResult::Complete => FormatSinkResult::Complete,
        jolt_java_fmt::JavaFormatSinkResult::Halted => FormatSinkResult::Halted,
        jolt_java_fmt::JavaFormatSinkResult::Blocked { diagnostics } => {
            FormatSinkResult::Blocked { diagnostics }
        }
        jolt_java_fmt::JavaFormatSinkResult::SinkError { error } => {
            FormatSinkResult::SinkError { error }
        }
    }
}
