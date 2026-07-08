use jolt_diagnostics::{Diagnostic, DiagnosticCodeId, DiagnosticStage, Severity};
use jolt_fmt_ir::{RenderSink, render_to};
use jolt_kotlin_syntax::parse_kotlin_file;

use crate::context::KotlinFormatter;

/// Kotlin formatter options consumed by the Kotlin layout builder.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct KotlinFormatOptions {
    /// Preferred maximum rendered line width.
    pub line_width: u16,
    /// Number of spaces per indentation level when `use_tabs` is false.
    pub indent_width: u8,
    /// Whether indentation should use tabs instead of spaces.
    pub use_tabs: bool,
}

impl Default for KotlinFormatOptions {
    fn default() -> Self {
        Self {
            line_width: 80,
            indent_width: 2,
            use_tabs: false,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum KotlinFormatSinkResult {
    Complete,
    Halted,
    Blocked { diagnostics: Vec<Diagnostic> },
}

/// Stable Kotlin formatter diagnostic codes.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum KotlinFormatDiagnosticCode {
    /// The document renderer rejected a formatter-produced document.
    RenderError,
}

impl KotlinFormatDiagnosticCode {
    const fn id(self) -> DiagnosticCodeId {
        match self {
            Self::RenderError => DiagnosticCodeId::new("kotlin.format.render_error"),
        }
    }
}

/// Formats Kotlin source text into a render sink.
pub fn format_source_to_sink<S: RenderSink + ?Sized>(
    source: &str,
    options: &KotlinFormatOptions,
    sink: &mut S,
) -> KotlinFormatSinkResult {
    let parse = parse_kotlin_file(source);

    let Some(syntax) = parse.syntax() else {
        return KotlinFormatSinkResult::Blocked {
            diagnostics: Vec::new(),
        };
    };

    let mut formatter = KotlinFormatter::new(options);
    let doc = formatter.format_file(&syntax);
    match render_to(&doc, formatter.render_options(), sink) {
        Ok(outcome) if outcome.halted => KotlinFormatSinkResult::Halted,
        Ok(_) => KotlinFormatSinkResult::Complete,
        Err(error) => KotlinFormatSinkResult::Blocked {
            diagnostics: vec![render_error_diagnostic(&error)],
        },
    }
}

fn render_error_diagnostic(error: &jolt_fmt_ir::RenderError) -> Diagnostic {
    Diagnostic {
        code: KotlinFormatDiagnosticCode::RenderError.id(),
        severity: Severity::InternalError,
        stage: DiagnosticStage::Formatter,
        message: format!("Kotlin formatter produced an invalid document: {error}"),
        range: None,
    }
}

#[cfg(test)]
mod tests {
    use jolt_fmt_ir::{RenderControl, RenderSink};

    use super::{KotlinFormatOptions, KotlinFormatSinkResult, format_source_to_sink};

    #[test]
    fn formats_represented_tree_with_parse_diagnostics() {
        let mut sink = StringSink::default();
        let result = format_source_to_sink(
            "fun demo() { = value }\n",
            &KotlinFormatOptions::default(),
            &mut sink,
        );

        assert!(matches!(result, KotlinFormatSinkResult::Complete));
        assert!(sink.output.contains("value"), "{}", sink.output);
        assert!(sink.output.contains('='), "{}", sink.output);
    }

    #[derive(Default)]
    struct StringSink {
        output: String,
    }

    impl RenderSink for StringSink {
        fn write_str(&mut self, text: &str) -> RenderControl {
            self.output.push_str(text);
            RenderControl::Continue
        }
    }
}
