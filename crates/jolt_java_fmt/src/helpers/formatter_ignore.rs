pub(crate) use jolt_fmt_ir::formatter_ignore::{
    FormatterIgnoreItemRange, FormatterIgnoreRun, FormatterIgnoreSplice,
    for_each_formatter_ignore_splice, formatter_ignore_content_range, formatter_ignore_run_doc,
    is_formatter_control_marker,
};
use jolt_java_syntax::JavaSyntaxToken;

use super::lexical_safety::JavaLexicalSafety;

pub(crate) fn formatter_ignore_plan<'source>(
    source: &'source str,
    tokens: impl IntoIterator<Item = JavaSyntaxToken<'source>>,
) -> jolt_fmt_ir::formatter_ignore::FormatterIgnorePlan<'source> {
    jolt_fmt_ir::formatter_ignore::formatter_ignore_plan_with_safety(
        source,
        tokens,
        &mut JavaLexicalSafety,
    )
}
