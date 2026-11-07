//! The receiver spine: member chains and every other kind the parser's postfix
//! loop wraps around a receiver, walked iteratively so arbitrarily long
//! chains never recurse.

use jolt_fmt::{Doc, cat, group};
use jolt_java_syntax::{
    ArrayAccessExpression, ClassLiteralExpression, FieldAccessExpression, JavaSyntaxKind as K,
    MethodInvocationExpression, MethodInvocationFormSyntax, MethodReferenceExpression,
    ObjectCreationExpression, PostfixExpression, QualifiedMethodInvocation, SuperExpression,
    ThisExpression,
};
use jolt_syntax::{AstNode, SyntaxElement, SyntaxNode, SyntaxToken};

use super::calls::qualified_invocation_segment;
use super::creation::object_creation_tail;
use crate::JavaFormatter;
use crate::helpers::lists::enclosed;

pub(crate) fn method_invocation_expression<'a>(
    f: &mut JavaFormatter<'a>,
    node: MethodInvocationExpression<'a>,
) -> Doc {
    spine(f, node.syntax())
}

pub(crate) fn field_access_expression<'a>(
    f: &mut JavaFormatter<'a>,
    node: FieldAccessExpression<'a>,
) -> Doc {
    spine(f, node.syntax())
}

pub(crate) fn array_access_expression<'a>(
    f: &mut JavaFormatter<'a>,
    node: ArrayAccessExpression<'a>,
) -> Doc {
    spine(f, node.syntax())
}

pub(crate) fn postfix_expression<'a>(
    f: &mut JavaFormatter<'a>,
    node: PostfixExpression<'a>,
) -> Doc {
    spine(f, node.syntax())
}

pub(crate) fn method_reference_expression<'a>(
    f: &mut JavaFormatter<'a>,
    node: MethodReferenceExpression<'a>,
) -> Doc {
    spine(f, node.syntax())
}

pub(crate) fn class_literal_expression<'a>(
    f: &mut JavaFormatter<'a>,
    node: ClassLiteralExpression<'a>,
) -> Doc {
    spine(f, node.syntax())
}

pub(crate) fn object_creation_expression<'a>(
    f: &mut JavaFormatter<'a>,
    node: ObjectCreationExpression<'a>,
) -> Doc {
    spine(f, node.syntax())
}

pub(crate) fn this_expression<'a>(f: &mut JavaFormatter<'a>, node: ThisExpression<'a>) -> Doc {
    spine(f, node.syntax())
}

pub(crate) fn super_expression<'a>(f: &mut JavaFormatter<'a>, node: SuperExpression<'a>) -> Doc {
    spine(f, node.syntax())
}

/// One well-formed spine node and what it adds after its receiver.
#[derive(Clone, Copy)]
enum Link<'a> {
    /// `.name(…)`: the call and its qualified form, both spine nodes.
    Call(
        MethodInvocationExpression<'a>,
        QualifiedMethodInvocation<'a>,
    ),
    /// A call whose form is unqualified, missing or malformed: no receiver.
    Form(MethodInvocationExpression<'a>),
    Field(FieldAccessExpression<'a>),
    Index(ArrayAccessExpression<'a>),
    Postfix(PostfixExpression<'a>),
    Reference(MethodReferenceExpression<'a>),
    ClassLiteral(ClassLiteralExpression<'a>),
    New(ObjectCreationExpression<'a>),
    This(ThisExpression<'a>),
    Super(SuperExpression<'a>),
}

fn cast<'a, N: AstNode<'a>>(node: SyntaxNode<'a>) -> N {
    N::cast(node).expect("the kind was matched")
}

fn link(node: SyntaxNode<'_>) -> Option<Link<'_>> {
    if node.is_malformed() {
        return None;
    }
    Some(match K::from_raw(node.kind()) {
        K::MethodInvocationExpression => {
            let call: MethodInvocationExpression<'_> = cast(node);
            match call.form() {
                Some(MethodInvocationFormSyntax::QualifiedMethodInvocation(form))
                    if !form.syntax().is_malformed() =>
                {
                    Link::Call(call, form)
                }
                _ => Link::Form(call),
            }
        }
        K::FieldAccessExpression => Link::Field(cast(node)),
        K::ArrayAccessExpression => Link::Index(cast(node)),
        K::PostfixExpression => Link::Postfix(cast(node)),
        K::MethodReferenceExpression => Link::Reference(cast(node)),
        K::ClassLiteralExpression => Link::ClassLiteral(cast(node)),
        K::ObjectCreationExpression => Link::New(cast(node)),
        K::ThisExpression => Link::This(cast(node)),
        K::SuperExpression => Link::Super(cast(node)),
        _ => return None,
    })
}

fn syntax<'a, N: AstNode<'a>>(node: Option<N>) -> Option<SyntaxNode<'a>> {
    node.map(AstNode::syntax)
}

impl<'a> Link<'a> {
    /// The node the link wraps: an expression, or a type before `::` or
    /// `.class`.
    fn receiver(self) -> Option<SyntaxNode<'a>> {
        match self {
            Self::Call(_, form) => syntax(form.receiver()),
            Self::Form(_) => None,
            Self::Field(node) => syntax(node.receiver()),
            Self::Index(node) => syntax(node.array()),
            Self::Postfix(node) => syntax(node.operand()),
            Self::Reference(node) => syntax(node.receiver()),
            Self::ClassLiteral(node) => syntax(node.target()),
            Self::New(node) => syntax(node.qualifier()),
            Self::This(node) => syntax(node.qualifier()),
            Self::Super(node) => syntax(node.qualifier()),
        }
    }

    /// The spine nodes the link owns, outermost first.
    fn nodes(self) -> (SyntaxNode<'a>, Option<SyntaxNode<'a>>) {
        match self {
            Self::Call(call, form) => (call.syntax(), Some(form.syntax())),
            Self::Form(node) => (node.syntax(), None),
            Self::Field(node) => (node.syntax(), None),
            Self::Index(node) => (node.syntax(), None),
            Self::Postfix(node) => (node.syntax(), None),
            Self::Reference(node) => (node.syntax(), None),
            Self::ClassLiteral(node) => (node.syntax(), None),
            Self::New(node) => (node.syntax(), None),
            Self::This(node) => (node.syntax(), None),
            Self::Super(node) => (node.syntax(), None),
        }
    }

    /// Member links (`.name(…)`, `.field`) form member chains.
    const fn is_member(self) -> bool {
        matches!(self, Self::Call(..) | Self::Field(_))
    }

    /// The `.` of a member link.
    fn dot(self) -> Option<SyntaxToken<'a>> {
        match self {
            Self::Call(_, form) => form.dot(),
            Self::Field(node) => node.dot(),
            _ => None,
        }
    }
}

/// Whether the link's `.` directly after its receiver would lex into an
/// integer literal before it.
fn merges_with_dot(link: Link<'_>) -> bool {
    let dot = match link {
        Link::Call(_, form) => form.dot(),
        Link::Field(node) => node.dot(),
        Link::ClassLiteral(node) if node.dimensions().is_none() => node.dot(),
        Link::New(node) => node.dot(),
        Link::This(node) => node.dot(),
        Link::Super(node) => node.dot(),
        _ => None,
    };
    dot.and_then(SyntaxToken::prev)
        .is_some_and(|prev| K::from_raw(prev.kind()) == K::IntegerLiteral)
}

/// What the link adds after its receiver.
fn segment<'a>(f: &mut JavaFormatter<'a>, link: Link<'a>) -> Doc {
    match link {
        Link::Call(_, form) => qualified_invocation_segment(f, form),
        Link::Form(call) => f.fmt_opt(call.form()),
        Link::Field(node) => {
            let dot = f.token_opt(node.dot());
            let name = f.token_opt(node.name());
            let type_arguments = f.fmt_opt(node.type_arguments());
            cat![f; dot, name, type_arguments]
        }
        Link::Index(node) => {
            let index = f.fmt_opt(node.index());
            let brackets = (node.open_bracket(), node.close_bracket());
            let index = enclosed(f, node.syntax(), brackets, index, true);
            f.group(index)
        }
        Link::Postfix(node) => f.token_opt(node.operator()),
        Link::Reference(node) => {
            let receiver_type_arguments = f.fmt_opt(node.receiver_type_arguments());
            let double_colon = f.token_opt(node.double_colon());
            let target_type_arguments = f.fmt_opt(node.target_type_arguments());
            let target = f.token_opt(node.target());
            cat![f; receiver_type_arguments, double_colon, target_type_arguments, target]
        }
        Link::ClassLiteral(node) => {
            let dimensions = f.fmt_opt(node.dimensions());
            let dot = f.token_opt(node.dot());
            let class = f.token_opt(node.class_keyword());
            cat![f; dimensions, dot, class]
        }
        Link::New(node) => {
            let dot = f.token_opt(node.dot());
            let tail = object_creation_tail(f, node);
            cat![f; dot, tail]
        }
        Link::This(node) => {
            let dot = f.token_opt(node.dot());
            let keyword = f.token_opt(node.this_keyword());
            cat![f; dot, keyword]
        }
        Link::Super(node) => {
            let dot = f.token_opt(node.dot());
            let keyword = f.token_opt(node.super_keyword());
            cat![f; dot, keyword]
        }
    }
}

/// A pending member link of a chain.
struct Member {
    doc: Doc,
    call: bool,
}

/// The spine below `outer`, innermost first: the head (the first node that is
/// not a well-formed spine node, if any), then each link's segment. Runs of
/// `.name(…)` and `.field` links form member chains; every other link is
/// appended to what precedes it. Interior spine nodes print their leading
/// comments before the head and their trailing comments after their segment;
/// the outermost node's are printed by `node()`.
fn spine<'a>(f: &mut JavaFormatter<'a>, outer: SyntaxNode<'a>) -> Doc {
    let mut links = Vec::new();
    let mut head = Some(outer);
    while let Some(link) = head.and_then(link) {
        links.push(link);
        head = link.receiver();
    }
    let mut docs = Vec::with_capacity(links.len() + 1);
    for (index, link) in links.iter().enumerate() {
        let (node, form) = link.nodes();
        if index > 0 {
            docs.push(f.leading_comments(node));
        }
        if let Some(form) = form {
            docs.push(f.leading_comments(form));
        }
    }
    docs.push(head.map_or(Doc::NIL, |head| f.node(head)));
    let mut formatted = f.concat(docs);
    // Whether the first segment of a member chain may join the line of the
    // root before it: the root is simple and nothing forces a line after it.
    let joins = |f: &JavaFormatter<'a>, root: Option<SyntaxNode<'a>>, simple: bool| {
        simple && root.is_some_and(|root| !f.has_trailing_comments(SyntaxElement::Node(root)))
    };
    let simple = head.is_some_and(|head| K::from_raw(head.kind()) == K::NameExpression);
    let mut root_joins = joins(f, head, simple);
    let mut chain = Vec::new();
    let mut join_first = false;
    for (index, &link) in links.iter().enumerate().rev() {
        let segment = segment(f, link);
        // `1 .class` (recovery only): `1.` would lex as a float literal.
        let separator = if merges_with_dot(link) {
            f.space()
        } else {
            Doc::NIL
        };
        let (node, form) = link.nodes();
        let form_trailing = form.map_or(Doc::NIL, |form| f.trailing_comments(form));
        let trailing = if index > 0 {
            f.trailing_comments(node)
        } else {
            Doc::NIL
        };
        let doc = cat![f; separator, segment, form_trailing, trailing];
        if link.is_member() {
            if chain.is_empty() {
                join_first = root_joins
                    && !link
                        .dot()
                        .is_some_and(|dot| f.has_leading_comments(SyntaxElement::Token(dot)));
            }
            chain.push(Member {
                doc,
                call: matches!(link, Link::Call(..)),
            });
            continue;
        }
        formatted = member_chain(f, formatted, &mut chain, join_first);
        // No group around the two: the segment has no line breaks of its
        // own, so one would never change the layout, and nesting a group per
        // link would make each fit check walk the whole spine.
        formatted = cat![f; formatted, doc];
        let simple = matches!(link, Link::This(_) | Link::Super(_) | Link::ClassLiteral(_));
        root_joins = joins(f, Some(node), simple);
    }
    member_chain(f, formatted, &mut chain, join_first)
}

/// `root` followed by the member links: flat if it fits; otherwise every
/// call after the root starts its own line one indent deeper. Consecutive
/// field accesses stay together. With `join_first`, the first segment joins
/// the root's line: after a simple root (a name, `this`, `super`, a class
/// literal) that has no trailing comment, when no comment leads the `.`.
fn member_chain(
    f: &mut JavaFormatter<'_>,
    root: Doc,
    chain: &mut Vec<Member>,
    join_first: bool,
) -> Doc {
    if chain.is_empty() {
        return root;
    }
    let mut segments = Vec::with_capacity(chain.len());
    let mut field_run = Vec::new();
    for member in chain.drain(..) {
        if !member.call {
            field_run.push(member.doc);
            continue;
        }
        if !field_run.is_empty() {
            segments.push(f.concat(field_run.drain(..)));
        }
        segments.push(member.doc);
    }
    if !field_run.is_empty() {
        segments.push(f.concat(field_run));
    }
    let mut segments = segments.into_iter();
    let head = if join_first {
        let first = segments.next().unwrap_or(Doc::NIL);
        cat![f; root, first]
    } else {
        root
    };
    let mut rest = Vec::new();
    for segment in segments {
        rest.push(f.soft_line());
        rest.push(segment);
    }
    if rest.is_empty() {
        return f.group(head);
    }
    let rest = f.concat(rest);
    let rest = f.indent(rest);
    group![f; head, rest]
}
