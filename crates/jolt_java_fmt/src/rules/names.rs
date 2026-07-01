use jolt_fmt_ir::Doc;
use jolt_java_syntax::NameSyntax;

use crate::helpers::comments::format_token_text;
use crate::helpers::names::qualified_name;

pub(crate) fn format_name(name: &NameSyntax) -> Doc {
    qualified_name(
        name.segments()
            .map(|segment| format_token_text(segment.text()))
            .collect(),
    )
}

pub(crate) fn name_key(name: &NameSyntax) -> String {
    name.segments()
        .map(|segment| segment.text().to_owned())
        .collect::<Vec<_>>()
        .join(".")
}
