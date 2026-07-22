use jolt_diagnostics::{Diagnostic, DiagnosticCodeId, DiagnosticStage, Severity};
use jolt_fmt_ir::{
    DocBuilder, FormatOptions, FormatSinkResult, RenderOptions, RenderSink, render_source_to,
};
use jolt_kotlin_syntax::{KotlinFile, KotlinSyntaxView, parse_kotlin_file};

use crate::helpers::formatter_ignore::formatter_ignore_plan;
use crate::rules::program::format_file;

/// Stable Kotlin formatter diagnostic codes.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum KotlinFormatDiagnosticCode {
    /// Parsing did not produce a represented syntax tree.
    NoSyntaxTree,
    /// The document renderer rejected a formatter-produced document.
    RenderError,
}

impl KotlinFormatDiagnosticCode {
    const fn id(self) -> DiagnosticCodeId {
        match self {
            Self::NoSyntaxTree => DiagnosticCodeId::new("kotlin.format.no_syntax_tree"),
            Self::RenderError => DiagnosticCodeId::new("kotlin.format.render_error"),
        }
    }
}

/// Formats Kotlin source text into a render sink.
#[inline]
pub fn format_source_to_sink<S: RenderSink + ?Sized>(
    source: &str,
    options: &FormatOptions,
    sink: &mut S,
) -> FormatSinkResult {
    let parse = parse_kotlin_file(source);

    let Some(syntax) = parse.syntax() else {
        return FormatSinkResult::Blocked {
            diagnostics: vec![no_syntax_tree_diagnostic()],
        };
    };

    format_syntax_to_sink(&syntax, *options, sink).0
}

fn format_syntax_to_sink<S: RenderSink + ?Sized>(
    syntax: &KotlinFile<'_>,
    options: FormatOptions,
    sink: &mut S,
) -> (FormatSinkResult, DocBuilderMetrics) {
    let Some(root) = syntax.syntax_node() else {
        return (
            FormatSinkResult::Blocked {
                diagnostics: vec![no_syntax_tree_diagnostic()],
            },
            DocBuilderMetrics::default(),
        );
    };
    let ignores = formatter_ignore_plan(syntax.source_text(), root.tokens());
    let mut builder = DocBuilder::with_source_capacity(syntax.source_text().len());
    builder.set_formatter_ignore_plan(ignores);
    let doc = format_file(syntax, &mut builder);
    let render_options = RenderOptions::from(options);
    let arena = builder.into_arena();
    #[cfg(feature = "bench")]
    let metrics = arena.benchmark_metrics();
    let result = match render_source_to(&arena, doc, render_options, sink, &root) {
        Ok(outcome) if outcome.halted() => FormatSinkResult::Halted,
        Ok(outcome) => {
            #[cfg(debug_assertions)]
            assert!(
                !root.is_recovery_free() || !outcome.used_malformed_verbatim(),
                "recovery-free Kotlin syntax rendered a malformed-verbatim fragment"
            );
            #[cfg(not(debug_assertions))]
            let _ = outcome;
            FormatSinkResult::Complete
        }
        Err(error) => FormatSinkResult::Blocked {
            diagnostics: vec![render_error_diagnostic(&error)],
        },
    };
    #[cfg(not(feature = "bench"))]
    let metrics = ();
    (result, metrics)
}

#[cfg(feature = "bench")]
type DocBuilderMetrics = jolt_fmt_ir::DocArenaMetrics;
#[cfg(not(feature = "bench"))]
type DocBuilderMetrics = ();

/// Formats an already-parsed root and returns its document arena measurements.
#[cfg(feature = "bench")]
pub fn benchmark_format_syntax_to_sink<S: RenderSink + ?Sized>(
    syntax: &KotlinFile<'_>,
    options: &FormatOptions,
    sink: &mut S,
) -> (FormatSinkResult, jolt_fmt_ir::DocArenaMetrics) {
    format_syntax_to_sink(syntax, *options, sink)
}

fn no_syntax_tree_diagnostic() -> Diagnostic {
    Diagnostic {
        code: KotlinFormatDiagnosticCode::NoSyntaxTree.id(),
        severity: Severity::Error,
        stage: DiagnosticStage::Formatter,
        message: "Kotlin formatter received no represented syntax tree".to_owned(),
        range: None,
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

        assert!(matches!(result, FormatSinkResult::Complete), "{result:#?}");
        let formatted = sink.into_string();
        assert!(formatted.contains("value"), "{formatted}");
        assert!(formatted.contains('='), "{formatted}");
    }
}
