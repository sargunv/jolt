//! Shared formatter facade for Jolt's CLI and dprint plugin.

use std::path::Path;

pub use jolt_fmt_ir::{FormatOptions, FormatSinkResult, RenderControl, RenderSink};

/// Source language to format.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Language {
    /// Java source, typically `.java`.
    Java,
    /// Kotlin source, typically `.kt` or `.kts`.
    Kotlin,
}

impl Language {
    /// Detects a supported language from a file path's extension.
    #[must_use]
    pub fn from_path(path: &Path) -> Option<Self> {
        match path.extension().and_then(|extension| extension.to_str()) {
            Some("java") => Some(Self::Java),
            Some("kt" | "kts") => Some(Self::Kotlin),
            _ => None,
        }
    }
}

/// Formats source text for `language` into a render sink using `options`.
pub fn format_source_to_sink<S: RenderSink + ?Sized>(
    source: &str,
    language: Language,
    options: &FormatOptions,
    sink: &mut S,
) -> FormatSinkResult {
    match language {
        Language::Java => jolt_java_fmt::format_source_to_sink(source, options, sink),
        Language::Kotlin => jolt_kotlin_fmt::format_source_to_sink(source, options, sink),
    }
}
