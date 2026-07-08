use jolt_diagnostics::{Diagnostic, DiagnosticCodeId, DiagnosticStage, Severity};
use jolt_fmt_ir::{RenderSink, render_to};
use jolt_java_syntax::parse_compilation_unit;

use crate::context::JavaFormatter;

/// Java formatter options consumed by the Java layout builder.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct JavaFormatOptions {
    /// Preferred maximum rendered line width.
    pub line_width: u16,
    /// Number of spaces per indentation level when `use_tabs` is false.
    pub indent_width: u8,
    /// Whether indentation should use tabs instead of spaces.
    pub use_tabs: bool,
}

impl Default for JavaFormatOptions {
    fn default<'source>() -> Self {
        Self {
            line_width: 80,
            indent_width: 2,
            use_tabs: false,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum JavaFormatSinkResult {
    Complete,
    Halted,
    Blocked { diagnostics: Vec<Diagnostic> },
}

/// Stable Java formatter diagnostic codes.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum JavaFormatDiagnosticCode {
    /// The document renderer rejected a formatter-produced document.
    RenderError,
}

impl JavaFormatDiagnosticCode {
    const fn id(self) -> DiagnosticCodeId {
        match self {
            Self::RenderError => DiagnosticCodeId::new("java.format.render_error"),
        }
    }
}

/// Formats Java source text into a render sink.
pub fn format_source_to_sink<S: RenderSink + ?Sized>(
    source: &str,
    options: &JavaFormatOptions,
    sink: &mut S,
) -> JavaFormatSinkResult {
    let parse = parse_compilation_unit(source);

    let Some(syntax) = parse.syntax() else {
        return JavaFormatSinkResult::Blocked {
            diagnostics: Vec::new(),
        };
    };

    let mut formatter = JavaFormatter::new(options);
    let doc = formatter.format_compilation_unit(&syntax);
    match render_to(&doc, formatter.render_options(), sink) {
        Ok(outcome) if outcome.halted => JavaFormatSinkResult::Halted,
        Ok(_) => JavaFormatSinkResult::Complete,
        Err(error) => JavaFormatSinkResult::Blocked {
            diagnostics: vec![render_error_diagnostic(&error)],
        },
    }
}

fn render_error_diagnostic(error: &jolt_fmt_ir::RenderError) -> Diagnostic {
    Diagnostic {
        code: JavaFormatDiagnosticCode::RenderError.id(),
        severity: Severity::InternalError,
        stage: DiagnosticStage::Formatter,
        message: format!("Java formatter produced an invalid document: {error}"),
        range: None,
    }
}

#[cfg(test)]
mod tests {
    use jolt_fmt_ir::{RenderControl, RenderSink};

    use super::{JavaFormatOptions, JavaFormatSinkResult, format_source_to_sink};

    #[test]
    fn formats_represented_tree_with_parse_diagnostics() {
        let mut sink = StringSink::default();
        let result = format_source_to_sink(
            "class C { void m() { value + ; } }\n",
            &JavaFormatOptions::default(),
            &mut sink,
        );

        assert!(matches!(result, JavaFormatSinkResult::Complete));
        assert!(sink.output.contains("value"), "{}", sink.output);
        assert!(sink.output.contains('+'), "{}", sink.output);
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
