//! The Java formatter: the Java `FormatLanguage`, node dispatch, shared
//! helpers, and the layout rules. See `docs/internals/formatter.md` and
//! `docs/internals/java-style.md`.

mod dispatch;
mod helpers;
mod language;
mod rules;

use std::ops::ControlFlow;

use jolt_fmt::{FormatOptions, Sink};
use jolt_java_syntax::JavaParse;

pub use language::JavaFormat;

/// The formatter context every Java rule receives.
pub type JavaFormatter<'a> = jolt_fmt::Formatter<'a, JavaFormat>;

/// Formats a parsed compilation unit into `sink`.
pub fn format<S: Sink + ?Sized>(
    parse: &JavaParse<'_>,
    options: &FormatOptions,
    sink: &mut S,
) -> ControlFlow<()> {
    jolt_fmt::format::<JavaFormat, S>(parse.syntax(), options, sink)
}

#[doc(hidden)]
pub use helpers::placeholder::take_placeholder_kinds;
