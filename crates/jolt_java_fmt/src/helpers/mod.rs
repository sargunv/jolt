//! Helpers shared by the rule areas.

pub(crate) mod assignment;
pub(crate) mod bodies;
pub(crate) mod lists;

use jolt_fmt::{Doc, cat};
use jolt_syntax::AstNode;

use crate::JavaFormatter;

/// A space, then `doc`; NIL when `doc` is NIL.
pub(crate) fn spaced(f: &mut JavaFormatter<'_>, doc: Doc) -> Doc {
    if doc.is_nil() {
        doc
    } else {
        cat![f; f.space(), doc]
    }
}

/// A space, then `node`; NIL when `node` is missing. (A node without tokens,
/// which recovery can produce, keeps the space.)
pub(crate) fn spaced_node<'a, N: AstNode<'a>>(f: &mut JavaFormatter<'a>, node: Option<N>) -> Doc {
    match node {
        Some(node) => {
            let doc = f.fmt(node);
            cat![f; f.space(), doc]
        }
        None => Doc::NIL,
    }
}
