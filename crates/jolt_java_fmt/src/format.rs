use jolt_diagnostics::{Diagnostic, DiagnosticCodeId, DiagnosticStage, Severity};
use jolt_fmt_ir::{
    DocBuilder, FormatOptions, FormatSinkResult, RenderOptions, RenderSink, render_source_to,
};
use jolt_java_syntax::{CompilationUnit, JavaSyntaxView, parse_compilation_unit};

use crate::rules::program::format_compilation_unit;

/// Stable Java formatter diagnostic codes.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum JavaFormatDiagnosticCode {
    /// Parsing did not produce a represented syntax tree.
    NoSyntaxTree,
    /// The document renderer rejected a formatter-produced document.
    RenderError,
}

impl JavaFormatDiagnosticCode {
    const fn id(self) -> DiagnosticCodeId {
        match self {
            Self::NoSyntaxTree => DiagnosticCodeId::new("java.format.no_syntax_tree"),
            Self::RenderError => DiagnosticCodeId::new("java.format.render_error"),
        }
    }
}

/// Formats Java source text into a render sink.
#[inline]
pub fn format_source_to_sink<S: RenderSink + ?Sized>(
    source: &str,
    options: &FormatOptions,
    sink: &mut S,
) -> FormatSinkResult {
    let parse = parse_compilation_unit(source);

    let Some(syntax) = parse.syntax() else {
        return FormatSinkResult::Blocked {
            diagnostics: vec![no_syntax_tree_diagnostic()],
        };
    };

    format_syntax_to_sink(&syntax, *options, sink).0
}

fn format_syntax_to_sink<S: RenderSink + ?Sized>(
    syntax: &CompilationUnit<'_>,
    options: FormatOptions,
    sink: &mut S,
) -> (FormatSinkResult, DocBuilderMetrics) {
    let mut builder = DocBuilder::with_source_capacity(syntax.source_text().len());
    let doc = format_compilation_unit(syntax, &mut builder);
    let render_options = RenderOptions::from(options);
    let arena = builder.into_arena();
    #[cfg(feature = "bench")]
    let metrics = arena.benchmark_metrics();
    let Some(root) = syntax.syntax_node() else {
        return (
            FormatSinkResult::Blocked {
                diagnostics: vec![no_syntax_tree_diagnostic()],
            },
            DocBuilderMetrics::default(),
        );
    };
    let result = match render_source_to(&arena, doc, render_options, sink, &root) {
        Ok(outcome) if outcome.halted() => FormatSinkResult::Halted,
        Ok(outcome) => {
            #[cfg(debug_assertions)]
            assert!(
                !root.is_recovery_free() || !outcome.used_malformed_verbatim(),
                "recovery-free Java syntax rendered a malformed-verbatim fragment"
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
    syntax: &CompilationUnit<'_>,
    options: &FormatOptions,
    sink: &mut S,
) -> (FormatSinkResult, jolt_fmt_ir::DocArenaMetrics) {
    format_syntax_to_sink(syntax, *options, sink)
}

fn no_syntax_tree_diagnostic() -> Diagnostic {
    Diagnostic {
        code: JavaFormatDiagnosticCode::NoSyntaxTree.id(),
        severity: Severity::Error,
        stage: DiagnosticStage::Formatter,
        message: "Java formatter received no represented syntax tree".to_owned(),
        range: None,
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

        let formatted = sink.into_string();
        assert!(
            matches!(result, FormatSinkResult::Complete),
            "{result:?}; output={formatted:?}"
        );
        assert!(formatted.contains("value"), "{formatted}");
        assert!(formatted.contains('+'), "{formatted}");
    }

    #[test]
    fn malformed_declaration_field_does_not_drop_siblings() {
        let mut sink = StringSink::default();
        let result = format_source_to_sink(
            "class C { void () {} }\n",
            &FormatOptions::default(),
            &mut sink,
        );

        let formatted = sink.into_string();
        assert!(
            matches!(result, FormatSinkResult::Complete),
            "{result:?}; output={formatted:?}"
        );
        for represented in ["class", "C", "void", "(", ")", "{", "}"] {
            assert!(
                formatted.contains(represented),
                "missing {represented:?}: {formatted}"
            );
        }
    }

    #[test]
    fn malformed_expression_preserves_composite_operator_components() {
        let mut sink = StringSink::default();
        let result = format_source_to_sink(
            "class C { void m() { value >>= ; } }\n",
            &FormatOptions::default(),
            &mut sink,
        );

        let formatted = sink.into_string();
        assert!(
            matches!(result, FormatSinkResult::Complete),
            "{result:?}; output={formatted:?}"
        );
        assert!(formatted.contains("value"), "{formatted}");
        assert!(formatted.contains(">>="), "{formatted}");
        assert!(formatted.contains(';'), "{formatted}");
    }
}
