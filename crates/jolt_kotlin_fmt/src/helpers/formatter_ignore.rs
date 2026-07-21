pub(crate) use jolt_fmt_ir::formatter_ignore::{
    FormatterIgnoreRange, FormatterIgnoreRun, FormatterIgnoreSplice,
    for_each_formatter_ignore_splice, formatter_ignore_run_doc, formatter_ignore_runs,
    is_formatter_control_marker, relative_token_range_between,
};
use jolt_kotlin_syntax::KotlinSyntaxToken;

use super::lexical_safety::KotlinLexicalSafety;

pub(crate) fn formatter_ignore_ranges<'source>(
    source: &'source str,
    base_start: usize,
    tokens: impl IntoIterator<Item = KotlinSyntaxToken<'source>>,
) -> Vec<FormatterIgnoreRange<'source>> {
    jolt_fmt_ir::formatter_ignore::formatter_ignore_ranges_with_safety(
        source,
        base_start,
        tokens,
        &mut KotlinLexicalSafety,
    )
}
