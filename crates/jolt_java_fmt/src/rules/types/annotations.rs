//! Annotations and their arguments.

use jolt_fmt::{Doc, ListItem, cat};
use jolt_java_syntax::{
    Annotation, AnnotationArgumentList, AnnotationArrayInitializer, AnnotationElementList,
    AnnotationElementValue, AnnotationElementValuePair, JavaSyntaxKind as K,
};
use jolt_syntax::{AstNode, ListElement};

use crate::JavaFormatter;
use crate::helpers::bodies::member_separator;
use crate::helpers::lists::{ListLayout, TrailingComma, delimited};

pub(crate) fn annotation<'a>(f: &mut JavaFormatter<'a>, node: Annotation<'a>) -> Doc {
    let at = f.token_opt(node.at());
    let name = f.fmt_opt(node.name());
    let arguments = f.fmt_opt(node.arguments());
    cat![f; at, name, arguments]
}

/// `(a, b)` with the break-all list layout of `delimited`. The elements are
/// one node (`AnnotationElementList`) that can carry comments of its own, so
/// it is formatted with `f.fmt` inside the parentheses instead of item by
/// item.
pub(crate) fn annotation_argument_list<'a>(
    f: &mut JavaFormatter<'a>,
    node: AnnotationArgumentList<'a>,
) -> Doc {
    let open = f.token_opt(node.open_paren());
    let elements = f.fmt_opt(node.elements());
    let dangling = f.dangling(node.syntax());
    let close = f.token_opt(node.close_paren());
    let mut inner = Vec::with_capacity(3);
    if !elements.is_nil() {
        inner.push(elements);
    }
    if !dangling.is_nil() {
        if !elements.is_nil() {
            let blank = f.blank_line_before_dangling(node.syntax());
            inner.push(if blank { f.empty_line() } else { f.hard_line() });
        }
        inner.push(dangling);
    }
    if inner.is_empty() {
        return cat![f; open, close];
    }
    let inner = f.concat(inner);
    let inner = cat![f; f.soft_line(), inner];
    let inner = f.indent(inner);
    let list = cat![f; open, inner, f.soft_line(), close];
    f.group(list)
}

/// Annotation arguments: `a, b` with a `line` after each comma (the
/// enclosing argument list's group decides). In an annotation interface body
/// the same node holds the member declarations, which are joined like class
/// members.
pub(crate) fn annotation_element_list<'a>(
    f: &mut JavaFormatter<'a>,
    node: AnnotationElementList<'a>,
) -> Doc {
    let mut docs = Vec::new();
    let mut after_item = false;
    let elements: Vec<_> = node
        .arguments()
        .into_iter()
        .flat_map(jolt_java_syntax::AnnotationElementArgumentList::elements)
        .filter(|element| !matches!(element, ListElement::Missing))
        .collect();
    let count = elements.len();
    for (index, element) in elements.into_iter().enumerate() {
        match element {
            ListElement::Item(item) => {
                if after_item {
                    docs.push(f.line());
                }
                docs.push(f.fmt(item));
                after_item = true;
            }
            ListElement::Bogus(bogus) => {
                if after_item {
                    docs.push(f.line());
                }
                docs.push(f.node(bogus));
                after_item = true;
            }
            ListElement::Separator(comma) => {
                docs.push(f.token(comma));
                if index + 1 < count {
                    docs.push(f.line());
                }
                after_item = false;
            }
            ListElement::Missing => {}
        }
    }
    if let Some(declarations) = node.declarations() {
        let items = declarations
            .syntax()
            .children()
            .filter_map(jolt_syntax::SyntaxElement::as_node)
            .map(|member| {
                if K::from_raw(member.kind()) == K::EmptyDeclaration {
                    ListItem::Elide(member)
                } else {
                    ListItem::Format(member)
                }
            });
        docs.push(f.item_list(items, member_separator));
    }
    f.concat(docs)
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
            .flat_map(jolt_java_syntax::AnnotationElementValueList::elements),
        node.close_brace(),
        ListLayout {
            padded: true,
            trailing_comma: TrailingComma::IfBroken,
        },
    )
}
