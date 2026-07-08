use jolt_diagnostics::{Diagnostic, DiagnosticCodeId, DiagnosticStage, Severity};
use jolt_fmt_ir::{FormatOptions, FormatSinkResult, RenderSink, render_to};
use jolt_java_syntax::parse_compilation_unit;

use crate::context::JavaFormatter;

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
    options: &FormatOptions,
    sink: &mut S,
) -> FormatSinkResult {
    let parse = parse_compilation_unit(source);

    let Some(syntax) = parse.syntax() else {
        return FormatSinkResult::Blocked {
            diagnostics: Vec::new(),
        };
    };

    let mut formatter = JavaFormatter::new(options);
    let doc = formatter.format_compilation_unit(&syntax);
    match render_to(&doc, formatter.render_options(), sink) {
        Ok(outcome) if outcome.halted => FormatSinkResult::Halted,
        Ok(_) => FormatSinkResult::Complete,
        Err(error) => FormatSinkResult::Blocked {
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
    use jolt_fmt_ir::{FormatOptions, FormatSinkResult};
    use jolt_test_support::StringSink;

    use super::format_source_to_sink;

    #[test]
    fn formats_represented_tree_with_parse_diagnostics() {
        let mut sink = StringSink::default();
        let result = format_source_to_sink(
            "class C { void m() { value + ; } }\n",
            &FormatOptions::default(),
            &mut sink,
        );

        assert!(matches!(result, FormatSinkResult::Complete));
        let formatted = sink.into_string();
        assert!(formatted.contains("value"), "{formatted}");
        assert!(formatted.contains('+'), "{formatted}");
    }
}
