//! Root formatting-run coordination shared by language formatters.

use jolt_diagnostics::Diagnostic;
use jolt_syntax::{Language, SyntaxNode};

use crate::formatter_ignore::formatter_ignore_plan_with_safety;
use crate::render::render_source_to;
use crate::{
    Doc, DocBuilder, FormatOptions, FormatSinkResult, LexicalSafety, RenderError, RenderSink,
};

/// Document arena measurements returned to benchmark-enabled language crates.
#[cfg(feature = "bench")]
pub type FormatRootMetrics = crate::DocArenaMetrics;

/// Zero-cost metrics placeholder outside benchmark builds.
#[cfg(not(feature = "bench"))]
pub type FormatRootMetrics = ();

/// Builds and renders one already-parsed language root.
///
/// Parsing, typed-root extraction, layout rules, and stable diagnostics remain
/// language-owned. This function owns only the shared run lifecycle: ignore
/// discovery, document construction, source-aware rendering, and sink outcome
/// mapping.
#[doc(hidden)]
pub fn format_root_to_sink<'source, L, S>(
    root: &SyntaxNode<'source, L>,
    options: FormatOptions,
    sink: &mut S,
    mut safety: impl LexicalSafety<L>,
    layout: impl FnOnce(&mut DocBuilder<'source>) -> Doc<'source>,
    render_error_diagnostic: impl FnOnce(&RenderError) -> Diagnostic,
) -> (FormatSinkResult, FormatRootMetrics)
where
    L: Language,
    S: RenderSink + ?Sized,
{
    let source = root.source();
    let ignore_plan = formatter_ignore_plan_with_safety(source, root.tokens(), &mut safety);
    let mut builder = DocBuilder::for_root(source.len(), ignore_plan);
    let doc = layout(&mut builder);
    let arena = builder.into_arena();
    #[cfg(feature = "bench")]
    let metrics = arena.benchmark_metrics();
    #[cfg(not(feature = "bench"))]
    let metrics = ();

    let result = match render_source_to(&arena, doc, options, sink, root) {
        Ok(outcome) if outcome.halted() => FormatSinkResult::Halted,
        Ok(_) => FormatSinkResult::Complete,
        Err(error) => FormatSinkResult::Blocked {
            diagnostic: render_error_diagnostic(&error),
        },
    };

    (result, metrics)
}
