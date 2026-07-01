use jolt_fmt_ir::{Doc, text};
use jolt_java_syntax::NameSyntax;

use crate::helpers::names::qualified_name;

pub(crate) fn format_name(name: &NameSyntax) -> Doc {
    qualified_name(
        name.segments()
            .map(|segment| text(segment.text().to_owned()))
            .collect(),
    )
}

pub(crate) fn name_key(name: &NameSyntax) -> String {
    name.segments()
        .map(|segment| segment.text().to_owned())
        .collect::<Vec<_>>()
        .join(".")
}
