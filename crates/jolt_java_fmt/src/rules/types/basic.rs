//! Primitive, void, class and array types.

use jolt_fmt::{Doc, cat};
use jolt_java_syntax::{
    ArrayDimension, ArrayDimensions, ArrayType, ClassType, ClassTypeSegmentNode, PrimitiveType,
    VoidType,
};
use jolt_syntax::{AstNode, ListElement, SyntaxElement};

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

pub(crate) fn array_type<'a>(f: &mut JavaFormatter<'a>, node: ArrayType<'a>) -> Doc {
    let element = f.fmt_opt(node.element_type());
    let dimensions = f.fmt_opt(node.dimensions());
    cat![f; element, dimensions]
}

/// `[]` per dimension, joined without spaces.
pub(crate) fn array_dimensions<'a>(f: &mut JavaFormatter<'a>, node: ArrayDimensions<'a>) -> Doc {
    let mut docs = Vec::new();
    for child in node.syntax().children() {
        docs.push(match child {
            SyntaxElement::Node(child) => f.node(child),
            SyntaxElement::Token(token) => f.token(token),
        });
    }
    f.concat(docs)
}

/// `[]`, or ` @A []` when annotated. Comments inside the brackets print
/// between them, on their own lines when one is a line comment.
pub(crate) fn array_dimension<'a>(f: &mut JavaFormatter<'a>, node: ArrayDimension<'a>) -> Doc {
    let annotations = f.fmt_opt(node.annotations());
    let annotations = if annotations.is_nil() {
        Doc::NIL
    } else {
        cat![f; f.space(), annotations]
    };
    let open = f.token_opt(node.open_bracket());
    let dangling = f.dangling(node.syntax());
    let close = f.token_opt(node.close_bracket());
    if dangling.is_nil() {
        return cat![f; annotations, open, close];
    }
    let inner = cat![f; f.soft_line(), dangling];
    let inner = f.indent(inner);
    let brackets = cat![f; open, inner, f.soft_line(), close];
    let brackets = f.group(brackets);
    cat![f; annotations, brackets]
}
