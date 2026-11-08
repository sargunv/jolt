//! Annotations and their arguments.

use jolt_fmt::{Doc, cat};
use jolt_java_syntax::{
    Annotation, AnnotationArgumentList, AnnotationArrayInitializer, AnnotationElementArgumentList,
    AnnotationElementValue, AnnotationElementValueList, AnnotationElementValuePair,
};
use jolt_syntax::AstNode;

use crate::JavaFormatter;
use crate::helpers::lists::{ListLayout, delimited};

pub(crate) fn annotation<'a>(f: &mut JavaFormatter<'a>, node: Annotation<'a>) -> Doc {
    let at = f.token_opt(node.at());
    let name = f.fmt_opt(node.name());
    let arguments = f.fmt_opt(node.arguments());
    cat![f; at, name, arguments]
}

/// `(a, b)` with the break-all list layout of `delimited`.
pub(crate) fn annotation_argument_list<'a>(
    f: &mut JavaFormatter<'a>,
    node: AnnotationArgumentList<'a>,
) -> Doc {
    delimited(
        f,
        node.syntax(),
        node.open_paren(),
        node.elements()
            .into_iter()
            .flat_map(AnnotationElementArgumentList::elements),
        node.close_paren(),
        ListLayout::Plain,
    )
}

pub(crate) fn annotation_element_value<'a>(
    f: &mut JavaFormatter<'a>,
    node: AnnotationElementValue<'a>,
) -> Doc {
    f.fmt_opt(node.value())
}

/// `name = value`; never breaks after `=`.
pub(crate) fn annotation_element_value_pair<'a>(
    f: &mut JavaFormatter<'a>,
    node: AnnotationElementValuePair<'a>,
) -> Doc {
    let name = f.token_opt(node.name());
    let assign = f.token_opt(node.assign());
    let value = f.fmt_opt(node.value());
    cat![f; name, f.space(), assign, f.space(), value]
}

/// `{ a, b }` when flat, `{}` when empty; broken, one value per line with a
/// trailing comma (N8).
pub(crate) fn annotation_array_initializer<'a>(
    f: &mut JavaFormatter<'a>,
    node: AnnotationArrayInitializer<'a>,
) -> Doc {
    delimited(
        f,
        node.syntax(),
        node.open_brace(),
        node.values()
            .into_iter()
            .flat_map(AnnotationElementValueList::elements),
        node.close_brace(),
        ListLayout::Initializer,
    )
}
