pub(crate) use jolt_fmt_ir::formatter_ignore::{
    FormatterIgnoreRange, FormatterIgnoreRun, FormatterIgnoreSplice,
    for_each_formatter_ignore_splice, formatter_ignore_run_doc, formatter_ignore_runs,
    is_formatter_control_marker, relative_token_range_between, token_range_between,
};
use jolt_java_syntax::JavaSyntaxToken;

use super::lexical_safety::JavaLexicalSafety;

pub(crate) fn formatter_ignore_ranges<'source>(
    source: &'source str,
    base_start: usize,
    tokens: impl IntoIterator<Item = JavaSyntaxToken<'source>>,
) -> Vec<FormatterIgnoreRange<'source>> {
    jolt_fmt_ir::formatter_ignore::formatter_ignore_ranges_with_safety(
        source,
        base_start,
        tokens,
        &mut JavaLexicalSafety,
    )
}
