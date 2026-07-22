//! Java formatter implementation for Jolt.

macro_rules! doc_concat {
    ($doc:expr, $docs:expr $(,)?) => {{
        let docs = $docs;
        $doc.concat(docs)
    }};
}

macro_rules! doc_group {
    ($doc:expr, $contents:expr $(,)?) => {{
        let contents = $contents;
        $doc.group(contents)
    }};
}

macro_rules! doc_force_group {
    ($doc:expr, $contents:expr $(,)?) => {{
        let contents = $contents;
        $doc.force_group(contents)
    }};
}

macro_rules! doc_indent {
    ($doc:expr, $contents:expr $(,)?) => {{
        let contents = $contents;
        $doc.indent(contents)
    }};
}

macro_rules! doc_join {
    ($doc:expr, $separator:expr, $docs:expr $(,)?) => {{
        let separator = $separator;
        let docs = $docs;
        $doc.join(separator, docs)
    }};
}

macro_rules! doc_if_break {
    ($doc:expr, $breaks:expr, $flat:expr $(,)?) => {{
        let breaks = $breaks;
        let flat = $flat;
        $doc.if_break(breaks, flat)
    }};
}

mod format;
mod helpers;
mod rules;

#[cfg(feature = "bench")]
pub use format::benchmark_format_syntax_to_sink;
pub use format::format_source_to_sink;
