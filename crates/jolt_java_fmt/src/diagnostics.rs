use jolt_diagnostics::{Diagnostic, DiagnosticCode, DiagnosticCodeId};

/// Stable Java formatter diagnostic codes.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum JavaFormatDiagnosticCode {
    /// The formatter hit an impossible internal state.
    InternalError,
    /// Rendering the formatter document failed.
    RenderFailed,
}

impl DiagnosticCode for JavaFormatDiagnosticCode {
    fn id(&self) -> DiagnosticCodeId {
        match self {
            Self::InternalError => DiagnosticCodeId::new("java.format.internal_error"),
            Self::RenderFailed => DiagnosticCodeId::new("java.format.render_failed"),
        }
    }
}

pub(crate) type FormatResult<T> = Result<T, Diagnostic>;
