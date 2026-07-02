use jolt_diagnostics::{
    Diagnostic, DiagnosticCode, DiagnosticCodeId, DiagnosticStage, Severity, SyntaxOutcome,
};
use jolt_fmt_ir::render;
use jolt_java_syntax::parse_compilation_unit;

use crate::context::JavaFormatter;

/// Java formatter options consumed by the Java layout builder.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct JavaFormatOptions {
    /// Preferred maximum rendered line width.
    pub line_width: u16,
    /// Number of spaces per indentation level when `use_tabs` is false.
    pub indent_width: u8,
    /// Whether indentation should use tabs instead of spaces.
    pub use_tabs: bool,
}

impl Default for JavaFormatOptions {
    fn default() -> Self {
        Self {
            line_width: 80,
            indent_width: 2,
            use_tabs: false,
        }
    }
}

/// Java formatter output plus diagnostics.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JavaFormatResult {
    /// Formatted source text, absent when formatting was blocked.
    pub formatted_source: Option<String>,
    /// Diagnostics produced while formatting.
    pub diagnostics: Vec<Diagnostic>,
}

/// Stable Java formatter diagnostic codes.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum JavaFormatDiagnosticCode {
    /// The document renderer rejected a formatter-produced document.
    RenderError,
}

impl DiagnosticCode for JavaFormatDiagnosticCode {
    fn id(&self) -> DiagnosticCodeId {
        match self {
            Self::RenderError => DiagnosticCodeId::new("java.format.render_error"),
        }
    }
}

/// Formats Java source text using the Jolt Java layout builder.
#[must_use]
pub fn format_source(source: &str, options: &JavaFormatOptions) -> JavaFormatResult {
    let parse = parse_compilation_unit(source);
    let (syntax, diagnostics, outcome) = parse.into_parts();

    if outcome != SyntaxOutcome::Clean {
        return JavaFormatResult {
            formatted_source: None,
            diagnostics,
        };
    }

    let Some(syntax) = syntax else {
        return JavaFormatResult {
            formatted_source: None,
            diagnostics,
        };
    };

    let mut formatter = JavaFormatter::new(options, &syntax);
    let doc = formatter.format_compilation_unit(&syntax);
    match render(&doc, formatter.render_options()) {
        Ok(rendered) => JavaFormatResult {
            formatted_source: Some(rendered.text),
            diagnostics,
        },
        Err(error) => JavaFormatResult {
            formatted_source: None,
            diagnostics: [diagnostics, vec![render_error_diagnostic(&error)]].concat(),
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
    use super::*;

    #[test]
    fn clean_java_formats_through_renderer_and_adds_final_newline() {
        let result = format_source("class A {}", &JavaFormatOptions::default());

        assert_eq!(result.formatted_source.as_deref(), Some("class A {\n}\n"));
        assert!(result.diagnostics.is_empty());
    }

    #[test]
    fn final_newline_is_idempotent() {
        let result = format_source("class A {\n}\n", &JavaFormatOptions::default());

        assert_eq!(result.formatted_source.as_deref(), Some("class A {\n}\n"));
        assert!(result.diagnostics.is_empty());
    }

    #[test]
    fn deeper_declarations_do_not_block_program_rendering() {
        let result = format_source(
            "class A {\n  void method() { return; }\n}\n",
            &JavaFormatOptions::default(),
        );

        assert!(result.formatted_source.is_some());
        assert!(result.diagnostics.is_empty());
    }

    #[test]
    fn parse_errors_block_formatting_without_output() {
        let result = format_source("class", &JavaFormatOptions::default());

        assert_eq!(result.formatted_source, None);
        assert!(!result.diagnostics.is_empty());
        assert!(
            result
                .diagnostics
                .iter()
                .all(|diagnostic| diagnostic.stage != DiagnosticStage::Formatter)
        );
    }

    #[test]
    fn declaration_recovery_nodes_do_not_reach_layout() {
        for source in [
            "enum E { , }",
            "class C { <T>() {} }",
            "class C { void () {} }",
            "@interface A { int (); }",
        ] {
            let result = format_source(source, &JavaFormatOptions::default());

            assert_eq!(result.formatted_source, None, "{source}");
            assert!(!result.diagnostics.is_empty(), "{source}");
            assert!(
                result
                    .diagnostics
                    .iter()
                    .all(|diagnostic| diagnostic.stage != DiagnosticStage::Formatter),
                "{source}: {:#?}",
                result.diagnostics
            );
        }
    }

    #[test]
    fn declaration_recovery_nodes_format_structurally_below_public_gate() {
        for (source, expected) in [
            ("enum E { , }", "enum E {\n  ,\n}\n"),
            ("class C { <T>() {} }", "class C {\n  <T> () {\n  }\n}\n"),
            ("class C { void () {} }", "class C {\n  void () {\n  }\n}\n"),
            ("@interface A { int (); }", "@interface A {\n  int ();\n}\n"),
        ] {
            assert_eq!(
                format_recovered_source_for_test(source),
                expected,
                "{source}"
            );
        }
    }

    fn format_recovered_source_for_test(source: &str) -> String {
        let parse = parse_compilation_unit(source);
        let (syntax, diagnostics, outcome) = parse.into_parts();
        assert_eq!(
            outcome,
            SyntaxOutcome::Recovered,
            "{source}: {diagnostics:#?}"
        );
        let syntax = syntax.expect("recovered parse should still produce syntax");

        let options = JavaFormatOptions::default();
        let mut formatter = JavaFormatter::new(&options, &syntax);
        let doc = formatter.format_compilation_unit(&syntax);
        render(&doc, formatter.render_options())
            .expect("recovered declaration layout should render")
            .text
    }
}
