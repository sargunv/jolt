//! Simple and qualified names: the dotted name on one line.

use jolt_fmt::{Doc, cat};
use jolt_java_syntax::{
    Name, NameSegmentDotList, NameSyntax, QualifiedName, QualifiedNameSegmentNode,
};
use jolt_syntax::{AstNode, ListElement, SyntaxToken};

use crate::JavaFormatter;

pub(crate) fn name<'a>(f: &mut JavaFormatter<'a>, node: Name<'a>) -> Doc {
    f.token_opt(node.identifier())
}

/// A comment that forces a line break inside the name puts every `.`
/// segment on its own line one indent deeper.
pub(crate) fn qualified_name<'a>(f: &mut JavaFormatter<'a>, node: QualifiedName<'a>) -> Doc {
    let first = f.fmt_opt(node.first_segment());
    let mut segments = Vec::new();
    let mut dot = f.token_opt(node.first_dot());
    for element in node
        .remaining_segments()
        .into_iter()
        .flat_map(NameSegmentDotList::elements)
    {
        let segment = match element {
            ListElement::Item(segment) => f.fmt(segment),
            ListElement::Bogus(bogus) => f.node(bogus),
            ListElement::Separator(next_dot) => {
                segments.push(dot);
                dot = f.token(next_dot);
                continue;
            }
            ListElement::Missing => continue,
        };
        dot = cat![f; dot, segment];
    }
    segments.push(dot);
    let broken = f.will_break(first) || segments.iter().any(|&segment| f.will_break(segment));
    if !broken {
        let rest = f.concat(segments);
        return cat![f; first, rest];
    }
    let mut rest = Vec::with_capacity(segments.len() * 2);
    for segment in segments {
        rest.extend([f.hard_line(), segment]);
    }
    let rest = f.concat(rest);
    let rest = f.indent(rest);
    cat![f; first, rest]
}

pub(crate) fn qualified_name_segment_node<'a>(
    f: &mut JavaFormatter<'a>,
    node: QualifiedNameSegmentNode<'a>,
) -> Doc {
    let annotations = f.fmt_opt(node.annotations());
    let identifier = f.token_opt(node.identifier());
    cat![f; annotations, identifier]
}

/// The dotted name without whitespace or comments, a sort key.
pub(crate) fn name_key(name: NameSyntax<'_>) -> String {
    name.syntax().tokens().map(SyntaxToken::text).collect()
}
