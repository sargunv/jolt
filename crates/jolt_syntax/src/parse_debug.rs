//! Shared `Debug` rendering for language parse results.

use std::fmt;

use jolt_diagnostics::Diagnostic;

/// Writes one diagnostic in the shared parser debug format.
///
/// # Errors
///
/// Returns any error produced by the underlying formatter.
pub fn fmt_diagnostic(f: &mut fmt::Formatter<'_>, diagnostic: &Diagnostic) -> fmt::Result {
    write!(
        f,
        "  code={} severity={:?} stage={:?}",
        diagnostic.code, diagnostic.severity, diagnostic.stage
    )?;
    if let Some(range) = diagnostic.range {
        write!(f, " range={range}")?;
    } else {
        write!(f, " range=<none>")?;
    }
    writeln!(f, " message={:?}", diagnostic.message)
}

/// Writes a language parse result in the shared `syntax:`/`diagnostics:` format.
///
/// `syntax` is the represented typed root, if any; language crates pass their
/// own typed root by reference so its `Debug` renders the tree.
///
/// # Errors
///
/// Returns any error produced by the underlying formatter.
pub fn fmt_parse_debug(
    f: &mut fmt::Formatter<'_>,
    syntax: Option<&dyn fmt::Debug>,
    diagnostics: &[Diagnostic],
) -> fmt::Result {
    writeln!(f, "syntax:")?;
    if let Some(syntax) = syntax {
        writeln!(f, "{syntax:?}")?;
    } else {
        writeln!(f, "  (none)")?;
    }

    writeln!(f)?;
    writeln!(f, "diagnostics:")?;
    if diagnostics.is_empty() {
        writeln!(f, "  (none)")?;
    } else {
        for diagnostic in diagnostics {
            fmt_diagnostic(f, diagnostic)?;
        }
    }

    Ok(())
}
