use super::{Doc, NameSyntax, join, text};

pub(super) fn format_name(name: &NameSyntax) -> Doc {
    join(
        text("."),
        name.segments().map(|segment| text(segment.text())),
    )
}
