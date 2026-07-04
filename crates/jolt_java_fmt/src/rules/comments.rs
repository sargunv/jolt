use jolt_fmt_ir::Doc;
use jolt_java_syntax::CompilationUnit;

use crate::helpers::blocks::join_hard_lines;
use crate::helpers::comments::format_comment;

pub(crate) fn format_comment_only_compilation_unit<'source>(
    unit: &CompilationUnit<'source>,
) -> Doc<'source> {
    join_hard_lines(
        unit.last_token()
            .into_iter()
            .flat_map(|token| token.leading_comments())
            .map(|comment| format_comment(&comment)),
    )
}
