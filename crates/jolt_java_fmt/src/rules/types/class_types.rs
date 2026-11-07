//! Primitive, void and class types.

use jolt_fmt::{Doc, cat};
use jolt_java_syntax::{ClassType, ClassTypeSegmentNode, PrimitiveType, VoidType};
use jolt_syntax::ListElement;

use crate::JavaFormatter;

pub(crate) fn primitive_type<'a>(f: &mut JavaFormatter<'a>, node: PrimitiveType<'a>) -> Doc {
    let annotations = f.fmt_opt(node.annotations());
    let keyword = f.token_opt(node.keyword());
    cat![f; annotations, keyword]
}

pub(crate) fn void_type<'a>(f: &mut JavaFormatter<'a>, node: VoidType<'a>) -> Doc {
    f.token_opt(node.void_keyword())
}

/// Dotted segments, each with its annotations and type arguments.
pub(crate) fn class_type<'a>(f: &mut JavaFormatter<'a>, node: ClassType<'a>) -> Doc {
    let mut docs = Vec::new();
    for element in node
        .segments()
        .into_iter()
        .flat_map(jolt_java_syntax::ClassTypeSegmentList::elements)
    {
        match element {
            ListElement::Item(segment) => docs.push(f.fmt(segment)),
            ListElement::Bogus(bogus) => docs.push(f.node(bogus)),
            ListElement::Separator(dot) => docs.push(f.token(dot)),
            ListElement::Missing => {}
        }
    }
    f.concat(docs)
}

pub(crate) fn class_type_segment_node<'a>(
    f: &mut JavaFormatter<'a>,
    node: ClassTypeSegmentNode<'a>,
) -> Doc {
    let annotations = f.fmt_opt(node.annotations());
    let name = f.fmt_opt(node.name());
    let type_arguments = f.fmt_opt(node.type_arguments());
    cat![f; annotations, name, type_arguments]
}
