use jolt_diagnostics::{Diagnostic, DiagnosticCodeId, DiagnosticStage, Severity};
use jolt_fmt_ir::{
    FormatOptions, FormatRootMetrics, FormatSinkResult, RenderSink, format_root_to_sink,
};
use jolt_java_syntax::{CompilationUnit, JavaParse, JavaSyntaxView, parse_compilation_unit};

use crate::helpers::lexical_safety::JavaLexicalSafety;
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
    format_parse_to_sink(&parse, options, sink)
}

/// Formats a previously parsed Java source without inspecting its diagnostics.
#[doc(hidden)]
pub fn format_parse_to_sink<S: RenderSink + ?Sized>(
    parse: &JavaParse<'_>,
    options: &FormatOptions,
    sink: &mut S,
) -> FormatSinkResult {
    let Some(syntax) = parse.syntax() else {
        return FormatSinkResult::Blocked {
            diagnostic: no_syntax_tree_diagnostic(),
        };
    };

    format_syntax_to_sink(&syntax, *options, sink).0
}

fn format_syntax_to_sink<S: RenderSink + ?Sized>(
    syntax: &CompilationUnit<'_>,
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
        JavaLexicalSafety,
        |doc| format_compilation_unit(syntax, doc),
        render_error_diagnostic,
    )
}

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
