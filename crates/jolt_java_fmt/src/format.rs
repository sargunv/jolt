use jolt_diagnostics::{
    Diagnostic, DiagnosticCode, DiagnosticCodeId, DiagnosticStage, Severity, SyntaxOutcome,
};
use jolt_fmt_ir::{RenderSink, RenderToError, render_to};
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
    fn default<'source>() -> Self {
        Self {
            line_width: 80,
            indent_width: 2,
            use_tabs: false,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum JavaFormatSinkResult<E> {
    Complete {
        diagnostics: Vec<Diagnostic>,
    },
    Halted {
        diagnostics: Vec<Diagnostic>,
    },
    Blocked {
        diagnostics: Vec<Diagnostic>,
    },
    SinkError {
        diagnostics: Vec<Diagnostic>,
        error: E,
    },
}

/// Stable Java formatter diagnostic codes.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum JavaFormatDiagnosticCode {
    /// The document renderer rejected a formatter-produced document.
    RenderError,
}

impl DiagnosticCode for JavaFormatDiagnosticCode {
    fn id<'source>(&self) -> DiagnosticCodeId {
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
) -> JavaFormatSinkResult<S::Error> {
    let parse = parse_compilation_unit(source);
    let diagnostics = parse.diagnostics().to_vec();
    let outcome = parse.outcome();

    if outcome != SyntaxOutcome::Clean {
        return JavaFormatSinkResult::Blocked { diagnostics };
    }

    let Some(syntax) = parse.syntax() else {
        return JavaFormatSinkResult::Blocked { diagnostics };
    };

    let mut formatter = JavaFormatter::new(options);
    let doc = formatter.format_compilation_unit(&syntax);
    match render_to(&doc, formatter.render_options(), sink) {
        Ok(outcome) if outcome.halted => JavaFormatSinkResult::Halted { diagnostics },
        Ok(_) => JavaFormatSinkResult::Complete { diagnostics },
        Err(RenderToError::Render(error)) => JavaFormatSinkResult::Blocked {
            diagnostics: [diagnostics, vec![render_error_diagnostic(&error)]].concat(),
        },
        Err(RenderToError::Sink(error)) => JavaFormatSinkResult::SinkError { diagnostics, error },
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
