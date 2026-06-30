use jolt_diagnostics::{Diagnostic, DiagnosticCode, DiagnosticStage, Severity, SyntaxOutcome};
use jolt_fmt_ir::render;
use jolt_java_syntax::parse_compilation_unit;

use crate::context::JavaFormatContext;
use crate::diagnostics::JavaFormatDiagnosticCode;
use crate::options::{JavaFormatOptions, JavaFormatProfile};
use crate::rules::format_compilation_unit;

/// Formatter operation status for Java formatting.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum JavaFormatStatus {
    /// Java source was formatted.
    Formatted,
    /// Java formatting was blocked and no formatted source was produced.
    Blocked,
}

/// Java formatter output plus diagnostics.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JavaFormatResult {
    /// Formatted source text, absent when formatting was blocked.
    pub formatted_source: Option<String>,
    /// Diagnostics produced while formatting.
    pub diagnostics: Vec<Diagnostic>,
    /// Formatter operation status.
    pub status: JavaFormatStatus,
}

/// Formats Java source text.
#[must_use]
pub fn format_java_source(source: &str) -> JavaFormatResult {
    format_java_source_with_options(source, JavaFormatOptions::default())
}

/// Formats Java source text with options resolved from a compatibility profile.
#[must_use]
pub fn format_java_source_with_profile(
    source: &str,
    profile: JavaFormatProfile,
) -> JavaFormatResult {
    format_java_source_with_options(source, JavaFormatOptions::for_profile(profile))
}

/// Formats Java source text with explicit Java formatter options.
#[must_use]
pub fn format_java_source_with_options(
    source: &str,
    options: JavaFormatOptions,
) -> JavaFormatResult {
    let parse = parse_compilation_unit(source);
    let (syntax, diagnostics, outcome) = parse.into_parts();

    if outcome != SyntaxOutcome::Clean {
        return blocked(diagnostics);
    }

    let Some(syntax) = syntax else {
        return blocked(vec![Diagnostic {
            code: JavaFormatDiagnosticCode::InternalError.id(),
            severity: Severity::InternalError,
            stage: DiagnosticStage::Formatter,
            message: "Java parser produced a clean outcome without syntax".to_owned(),
            range: None,
        }]);
    };

    let mut context = JavaFormatContext::with_profile(source, options.profile);
    let doc = match format_compilation_unit(&syntax, &mut context) {
        Ok(doc) => doc,
        Err(diagnostic) => return blocked(vec![diagnostic]),
    };

    if context.has_unhandled_comment_trivia() {
        let Some(trivia) = context.next_unhandled_comment_trivia() else {
            return blocked(vec![Diagnostic {
                code: JavaFormatDiagnosticCode::InternalError.id(),
                severity: Severity::InternalError,
                stage: DiagnosticStage::Formatter,
                message: "Java formatter context reported unhandled trivia without a record"
                    .to_owned(),
                range: Some(syntax.text_range()),
            }]);
        };
        return blocked(vec![Diagnostic {
            code: JavaFormatDiagnosticCode::InternalError.id(),
            severity: Severity::InternalError,
            stage: DiagnosticStage::Formatter,
            message: "Java formatter found unhandled comment or ignored trivia".to_owned(),
            range: Some(trivia.trivia.range),
        }]);
    }

    match render(&doc, options.render) {
        Ok(rendered) => {
            let mut formatted_source = rendered.text;
            if !formatted_source.ends_with('\n') {
                formatted_source.push('\n');
            }
            JavaFormatResult {
                formatted_source: Some(formatted_source),
                diagnostics,
                status: JavaFormatStatus::Formatted,
            }
        }
        Err(error) => blocked(vec![Diagnostic {
            code: JavaFormatDiagnosticCode::RenderFailed.id(),
            severity: Severity::InternalError,
            stage: DiagnosticStage::Formatter,
            message: format!("Java formatter failed to render document IR: {error}"),
            range: Some(syntax.text_range()),
        }]),
    }
}

fn blocked(diagnostics: Vec<Diagnostic>) -> JavaFormatResult {
    JavaFormatResult {
        formatted_source: None,
        diagnostics,
        status: JavaFormatStatus::Blocked,
    }
}
