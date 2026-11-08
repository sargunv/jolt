//! Type parameters and arguments, wildcards, and intersection and union
//! types.

use jolt_fmt::{Doc, cat, indent};
use jolt_java_syntax::{
    IntersectionType, JavaSyntaxKind as K, Type, TypeAmpList, TypeArgument, TypeArgumentList,
    TypeArgumentSeparatedList, TypeBarList, TypeBoundList, TypeParameter, TypeParameterList,
    TypeParameterSeparatedList, UnionType, WildcardType,
};
use jolt_syntax::{AstNode, ListElement, SyntaxElement, SyntaxToken};

use crate::JavaFormatter;
use crate::helpers::lists::{ListLayout, delimited};
use crate::helpers::{after_token, spaced, spaced_node};

/// `<T, U extends V>` as a break-all list.
pub(crate) fn type_parameter_list<'a>(
    f: &mut JavaFormatter<'a>,
    node: TypeParameterList<'a>,
) -> Doc {
    delimited(
        f,
        node.syntax(),
        node.open_angle(),
        node.parameters()
            .into_iter()
            .flat_map(TypeParameterSeparatedList::elements),
        node.close_angle(),
        ListLayout::Plain,
    )
}

pub(crate) fn type_parameter<'a>(f: &mut JavaFormatter<'a>, node: TypeParameter<'a>) -> Doc {
    let annotations = f.fmt_opt(node.annotations());
    let name = f.token_opt(node.name());
    let bounds = spaced_node(f, node.bounds());
    cat![f; annotations, name, bounds]
}

/// `extends Bound`; an intersection bound breaks with the enclosing list.
pub(crate) fn type_bound_list<'a>(f: &mut JavaFormatter<'a>, node: TypeBoundList<'a>) -> Doc {
    let keyword = f.token_opt(node.extends_keyword());
    let bound = f.fmt_opt(node.bounds());
    cat![f; keyword, f.space(), bound]
}

/// `<A, B>` as a break-all list; `<>` when empty.
pub(crate) fn type_argument_list<'a>(f: &mut JavaFormatter<'a>, node: TypeArgumentList<'a>) -> Doc {
    delimited(
        f,
        node.syntax(),
        node.open_angle(),
        node.arguments()
            .into_iter()
            .flat_map(TypeArgumentSeparatedList::elements),
        node.close_angle(),
        ListLayout::Plain,
    )
}

pub(crate) fn type_argument<'a>(f: &mut JavaFormatter<'a>, node: TypeArgument<'a>) -> Doc {
    let annotations = f.fmt_opt(node.annotations());
    let ty = f.fmt_opt(node.r#type());
    cat![f; annotations, ty]
}

/// `?`, `? extends T`, `? super T`.
pub(crate) fn wildcard_type<'a>(f: &mut JavaFormatter<'a>, node: WildcardType<'a>) -> Doc {
    let question = f.token_opt(node.question());
    let keyword = f.token_opt(node.bound_keyword());
    let keyword = spaced(f, keyword);
    let bound = spaced_node(f, node.bound());
    cat![f; question, keyword, bound]
}

/// `A & B`: flat if it fits, else a line break before each `&` one indent
/// deeper. As a type parameter bound it has no group of its own and breaks
/// with the enclosing list.
pub(crate) fn intersection_type<'a>(f: &mut JavaFormatter<'a>, node: IntersectionType<'a>) -> Doc {
    let first = f.fmt_opt(node.first_type());
    let rest = operator_chain(
        f,
        node.first_amp(),
        node.remaining_types()
            .into_iter()
            .flat_map(TypeAmpList::elements),
    );
    let chain = indent![f; rest];
    let chain = cat![f; first, chain];
    let in_bound = node
        .syntax()
        .parent()
        .is_some_and(|parent| K::from_raw(parent.kind()) == K::TypeBoundList);
    if in_bound { chain } else { f.group(chain) }
}

/// `A | B` (catch parameters): a line break before each `|` at the same
/// indent, decided by the enclosing group.
pub(crate) fn union_type<'a>(f: &mut JavaFormatter<'a>, node: UnionType<'a>) -> Doc {
    let first = f.fmt_opt(node.first_type());
    let rest = operator_chain(
        f,
        node.first_bar(),
        node.remaining_types()
            .into_iter()
            .flat_map(TypeBarList::elements),
    );
    cat![f; first, rest]
}

/// `line op space type` for each operator and the type after it (see
/// `after_token`).
fn operator_chain<'a>(
    f: &mut JavaFormatter<'a>,
    first_operator: Option<SyntaxToken<'a>>,
    rest: impl Iterator<Item = ListElement<'a, Type<'a>>>,
) -> Doc {
    let mut docs = Vec::new();
    let mut operator = first_operator;
    for element in rest {
        let ty = match element {
            ListElement::Item(ty) => ty.syntax(),
            ListElement::Bogus(bogus) => bogus,
            ListElement::Separator(token) => {
                if let Some(pending) = operator.replace(token) {
                    docs.extend([f.line(), after_token(f, Some(pending), None, Doc::NIL)]);
                }
                continue;
            }
            ListElement::Missing => continue,
        };
        let doc = f.node(ty);
        let operand = match operator.take() {
            Some(operator) => after_token(
                f,
                Some(operator),
                Some((SyntaxElement::Node(ty), doc)),
                Doc::NIL,
            ),
            None => doc,
        };
        docs.extend([f.line(), operand]);
    }
    if let Some(pending) = operator {
        docs.extend([f.line(), after_token(f, Some(pending), None, Doc::NIL)]);
    }
    f.concat(docs)
}
