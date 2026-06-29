use jolt_diagnostics::{
    Diagnostic, DiagnosticCode, DiagnosticCodeId, DiagnosticStage, Severity, TextRange,
};

/// Stable Java formatter diagnostic codes.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum JavaFormatDiagnosticCode {
    /// Parsed Java syntax is not covered by the current layout skeleton.
    MissingLayoutRules,
    /// Rendering the formatter document failed.
    RenderFailed,
}

impl DiagnosticCode for JavaFormatDiagnosticCode {
    fn id(&self) -> DiagnosticCodeId {
        match self {
            Self::MissingLayoutRules => DiagnosticCodeId::new("java.format.missing_layout_rules"),
            Self::RenderFailed => DiagnosticCodeId::new("java.format.render_failed"),
        }
    }
}

pub(crate) type FormatResult<T> = Result<T, Diagnostic>;

pub(crate) fn missing_layout(message: impl Into<String>, range: TextRange) -> Diagnostic {
    Diagnostic {
        code: JavaFormatDiagnosticCode::MissingLayoutRules.id(),
        severity: Severity::Error,
        stage: DiagnosticStage::Formatter,
        message: message.into(),
        range: Some(range),
    }
}
