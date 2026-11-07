//! Literals and names.

use jolt_fmt::{Doc, cat};
use jolt_java_syntax::{LiteralExpression, NameExpression};

use crate::JavaFormatter;

pub(crate) fn literal_expression<'a>(
    f: &mut JavaFormatter<'a>,
    node: LiteralExpression<'a>,
) -> Doc {
    f.token_opt(node.literal())
}

pub(crate) fn name_expression<'a>(f: &mut JavaFormatter<'a>, node: NameExpression<'a>) -> Doc {
    let annotations = f.fmt_opt(node.annotations());
    let identifier = f.token_opt(node.identifier());
    cat![f; annotations, identifier]
}
