//! Type parameters and arguments, wildcards, and intersection and union
//! types.

use jolt_fmt::{Doc, cat, indent};
use jolt_java_syntax::{
    IntersectionType, JavaSyntaxKind as K, Type, TypeArgument, TypeArgumentList, TypeBoundList,
    TypeParameter, TypeParameterList, UnionType, WildcardType,
};
use jolt_syntax::{AstNode, ListElement, SyntaxElement, SyntaxToken};

use crate::JavaFormatter;
use crate::helpers::lists::{ListLayout, TrailingComma, delimited};

const ANGLES: ListLayout = ListLayout {
    padded: false,
    trailing_comma: TrailingComma::Never,
};

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
            .flat_map(jolt_java_syntax::TypeParameterSeparatedList::elements),
        node.close_angle(),
        ANGLES,
    )
}

pub(crate) fn type_parameter<'a>(f: &mut JavaFormatter<'a>, node: TypeParameter<'a>) -> Doc {
    let annotations = f.fmt_opt(node.annotations());
    let name = f.token_opt(node.name());
    let Some(bounds) = node.bounds() else {
        return cat![f; annotations, name];
    };
    let bounds = f.fmt(bounds);
    cat![f; annotations, name, f.space(), bounds]
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
            .flat_map(jolt_java_syntax::TypeArgumentSeparatedList::elements),
        node.close_angle(),
        ANGLES,
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
    let mut docs = vec![question];
    if let Some(keyword) = node.bound_keyword() {
        docs.extend([f.space(), f.token(keyword)]);
    }
    if let Some(bound) = node.bound() {
        docs.extend([f.space(), f.fmt(bound)]);
    }
    f.concat(docs)
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
            .flat_map(jolt_java_syntax::TypeAmpList::elements),
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
            .flat_map(jolt_java_syntax::TypeBarList::elements),
    );
    cat![f; first, rest]
}

/// `line op space type` for each operator and the type after it. An
/// operator followed by an end-of-line comment puts its type on the next line
/// one indent deeper.
fn operator_chain<'a>(
    f: &mut JavaFormatter<'a>,
    first_operator: Option<SyntaxToken<'a>>,
    rest: impl Iterator<Item = ListElement<'a, Type<'a>>>,
) -> Doc {
    let mut docs = Vec::new();
    let mut operator = first_operator.map(|token| operator_token(f, token));
    for element in rest {
        let ty = match element {
            ListElement::Item(ty) => f.fmt(ty),
            ListElement::Bogus(bogus) => f.node(bogus),
            ListElement::Separator(token) => {
                if let Some(pending) = operator.take() {
                    docs.push(operand(f, Some(pending), Doc::NIL));
                }
                operator = Some(operator_token(f, token));
                continue;
            }
            ListElement::Missing => continue,
        };
        docs.push(operand(f, operator.take(), ty));
    }
    if let Some(pending) = operator {
        docs.push(operand(f, Some(pending), Doc::NIL));
    }
    f.concat(docs)
}

/// The operator's doc, and whether an end-of-line comment follows it.
fn operator_token<'a>(f: &mut JavaFormatter<'a>, token: SyntaxToken<'a>) -> (Doc, bool) {
    let commented = f.has_trailing_comments(SyntaxElement::Token(token));
    let doc = f.token(token);
    (doc, commented && f.will_break(doc))
}

fn operand(f: &mut JavaFormatter<'_>, operator: Option<(Doc, bool)>, ty: Doc) -> Doc {
    let Some((operator, ends_line)) = operator else {
        return cat![f; f.line(), ty];
    };
    if ends_line {
        let ty = indent![f; f.hard_line(), ty];
        cat![f; f.line(), operator, ty]
    } else {
        cat![f; f.line(), operator, f.space(), ty]
    }
}
