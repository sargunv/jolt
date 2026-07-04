//! Shared lossless syntax tree infrastructure for Jolt.

mod event;
mod kind;
mod language;
mod tree_sink;

mod green;
mod red;

pub use event::{CompletedMarker, Event, Marker};
pub use green::{GreenNode, GreenTrivia, TriviaKind};
pub use jolt_diagnostics::{
    Diagnostic, DiagnosticCode, DiagnosticCodeId, DiagnosticStage, Severity, SyntaxOutcome,
};
pub use kind::RawSyntaxKind;
pub use language::Language;
pub use red::{SyntaxElement, SyntaxNode, SyntaxToken};
pub use tree_sink::{
    BuildGreenTreeError, GreenTokenSource, GreenTree, GreenTriviaPiece, build_green_tree,
};

#[cfg(test)]
mod tests;
