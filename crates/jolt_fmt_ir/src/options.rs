//! Shared formatter options and sink results.
//!
//! These types are shared between the per-language formatter crates,
//! the `jolt_formatter` facade, the CLI, and the dprint plugin.

use jolt_diagnostics::Diagnostic;

/// Formatter options shared by CLI and dprint.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct FormatOptions {
    /// Preferred maximum rendered line width.
    pub line_width: u16,
    /// Number of spaces per indentation level when `use_tabs` is false.
    pub indent_width: u8,
    /// Whether indentation should use tabs instead of spaces.
    pub use_tabs: bool,
}

impl Default for FormatOptions {
    fn default() -> Self {
        Self {
            line_width: 80,
            indent_width: 2,
            use_tabs: false,
        }
    }
}

/// Outcome of formatting source text into a render sink.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FormatSinkResult {
    /// The document was fully rendered.
    Complete,
    /// The sink asked the renderer to halt early.
    Halted,
    /// Formatting was blocked by an error, carrying its diagnostic.
    Blocked { diagnostic: Diagnostic },
}
