use jolt_diagnostics::{Diagnostic, DiagnosticCodeId, DiagnosticStage, Severity};
use jolt_fmt_ir::{
    FormatOptions, FormatRootMetrics, FormatSinkResult, RenderSink, SyntaxErrorPolicy,
    format_root_to_sink,
};
use jolt_kotlin_syntax::{KotlinFile, KotlinSyntaxView, parse_kotlin_file};

use crate::helpers::lexical_safety::KotlinLexicalSafety;
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
    syntax_errors: SyntaxErrorPolicy,
    sink: &mut S,
) -> FormatSinkResult {
    let parse = parse_kotlin_file(source);

    if syntax_errors == SyntaxErrorPolicy::Reject
        && let Some(diagnostic) = parse.diagnostics().first()
    {
        return FormatSinkResult::Blocked {
            diagnostic: diagnostic.clone(),
        };
    }

    let Some(syntax) = parse.syntax() else {
        return FormatSinkResult::Blocked {
            diagnostic: no_syntax_tree_diagnostic(),
        };
    };

    format_syntax_to_sink(&syntax, *options, sink).0
}

fn format_syntax_to_sink<S: RenderSink + ?Sized>(
    syntax: &KotlinFile<'_>,
    options: FormatOptions,
    sink: &mut S,
) -> (FormatSinkResult, FormatRootMetrics) {
    let Some(root) = syntax.syntax_node() else {
        return (
            FormatSinkResult::Blocked {
                diagnostic: no_syntax_tree_diagnostic(),
            },
            FormatRootMetrics::default(),
        );
    };

    format_root_to_sink(
        &root,
        options,
        sink,
        KotlinLexicalSafety,
        |doc| format_file(syntax, doc),
        render_error_diagnostic,
    )
}

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
    use jolt_fmt_ir::{FormatOptions, FormatSinkResult, SyntaxErrorPolicy};
    use jolt_test_support::StringSink;

    use super::format_source_to_sink;

    #[test]
    fn syntax_error_policy_controls_represented_recovery_tree() {
        let source = "fun demo() { = value }\n";
        let mut rejected_sink = StringSink::default();
        let rejected = format_source_to_sink(
            source,
            &FormatOptions::default(),
            SyntaxErrorPolicy::Reject,
            &mut rejected_sink,
        );
        assert!(
            matches!(rejected, FormatSinkResult::Blocked { .. }),
            "{rejected:?}"
        );
        assert_eq!(rejected_sink.into_string(), "");

        let mut sink = StringSink::default();
        let result = format_source_to_sink(
            source,
            &FormatOptions::default(),
            SyntaxErrorPolicy::Format,
            &mut sink,
        );

        assert!(matches!(result, FormatSinkResult::Complete), "{result:#?}");
        let formatted = sink.into_string();
        assert!(formatted.contains("value"), "{formatted}");
        assert!(formatted.contains('='), "{formatted}");
    }
}
