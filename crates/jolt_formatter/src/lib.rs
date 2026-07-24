//! Shared formatter facade for Jolt's CLI and dprint plugin.

use std::path::Path;

use jolt_diagnostics::Diagnostic;

pub use jolt_fmt_ir::{FormatOptions, FormatSinkResult, RenderControl, RenderSink};

/// Source language to format.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Language {
    /// Java source, typically `.java`.
    Java,
    /// Kotlin source, typically `.kt` or `.kts`.
    Kotlin,
}

/// A source parsed by its selected language frontend.
pub enum ParsedSource<'source> {
    /// Parsed Java compilation unit.
    Java(jolt_java_syntax::JavaParse<'source>),
    /// Parsed Kotlin file.
    Kotlin(jolt_kotlin_syntax::KotlinParse<'source>),
}

impl ParsedSource<'_> {
    /// Returns lexer and parser diagnostics without interpreting them.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        match self {
            Self::Java(parse) => parse.diagnostics(),
            Self::Kotlin(parse) => parse.diagnostics(),
        }
    }
}

/// Parses source text for a selected language.
#[must_use]
pub fn parse_source(source: &str, language: Language) -> ParsedSource<'_> {
    match language {
        Language::Java => ParsedSource::Java(jolt_java_syntax::parse_compilation_unit(source)),
        Language::Kotlin => ParsedSource::Kotlin(jolt_kotlin_syntax::parse_kotlin_file(source)),
    }
}

/// Formats a previously parsed source without interpreting its diagnostics.
pub fn format_parsed_source_to_sink<S: RenderSink + ?Sized>(
    parsed: &ParsedSource<'_>,
    options: &FormatOptions,
    sink: &mut S,
) -> FormatSinkResult {
    match parsed {
        ParsedSource::Java(parse) => jolt_java_fmt::format_parse_to_sink(parse, options, sink),
        ParsedSource::Kotlin(parse) => jolt_kotlin_fmt::format_parse_to_sink(parse, options, sink),
    }
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
    let parsed = parse_source(source, language);
    format_parsed_source_to_sink(&parsed, options, sink)
}
