use jolt_diagnostics::{Diagnostic, DiagnosticCodeId, DiagnosticStage, Severity};
use jolt_fmt_ir::{FormatOptions, FormatSinkResult, RenderSink, render_to};
use jolt_kotlin_syntax::parse_kotlin_file;

use crate::context::KotlinFormatter;

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
    options: &FormatOptions,
    sink: &mut S,
) -> FormatSinkResult {
    let parse = parse_kotlin_file(source);

    let Some(syntax) = parse.syntax() else {
        return FormatSinkResult::Blocked {
            diagnostics: Vec::new(),
        };
    };

    let mut formatter = KotlinFormatter::new(options);
    let doc = formatter.format_file(&syntax);
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
        code: KotlinFormatDiagnosticCode::RenderError.id(),
        severity: Severity::InternalError,
        stage: DiagnosticStage::Formatter,
        message: format!("Kotlin formatter produced an invalid document: {error}"),
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
            "fun demo() { = value }\n",
            &FormatOptions::default(),
            &mut sink,
        );

        assert!(matches!(result, FormatSinkResult::Complete));
        let formatted = sink.into_string();
        assert!(formatted.contains("value"), "{formatted}");
        assert!(formatted.contains('='), "{formatted}");
    }
}
