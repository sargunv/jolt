//! Helpers shared by the rule areas.

pub(crate) mod assignment;
pub(crate) mod bodies;
pub(crate) mod lists;

use jolt_fmt::{Doc, cat, indent};
use jolt_syntax::{AstNode, SyntaxElement, SyntaxToken};

use crate::JavaFormatter;

/// A space, then `doc`; NIL when `doc` is NIL.
pub(crate) fn spaced(f: &mut JavaFormatter<'_>, doc: Doc) -> Doc {
    if doc.is_nil() {
        doc
    } else {
        cat![f; f.space(), doc]
    }
}

/// A space, then `node`; NIL when `node` is missing or formats to nothing.
pub(crate) fn spaced_node<'a, N: AstNode<'a>>(f: &mut JavaFormatter<'a>, node: Option<N>) -> Doc {
    let doc = f.fmt_opt(node);
    spaced(f, doc)
}

/// `token value tail`, for the value after a keyword, `->`, `&` or `|`. When
/// a trailing comment of `token` ends the line (`return // c`), the value
/// and `tail` continue on the next line one indent deeper: the pending
/// comment breaks the group, and the value may break further by its own
/// rule. When a leading comment of the value breaks the line (a comment on
/// its own line before it), the value and `tail` go one indent deeper too.
pub(crate) fn after_token<'a>(
    f: &mut JavaFormatter<'a>,
    token: Option<SyntaxToken<'a>>,
    value: Option<(SyntaxElement<'a>, Doc)>,
    tail: Doc,
) -> Doc {
    let token_breaks =
        token.is_some_and(|token| f.trailing_comments_break(SyntaxElement::Token(token)));
    let token = f.token_opt(token);
    let Some((element, value)) = value else {
        return cat![f; token, tail];
    };
    let value = if token_breaks {
        let value = indent![f; f.line(), value, tail];
        f.group(value)
    } else if f.leading_comments_break(element) {
        let value = indent![f; value, tail];
        cat![f; f.space(), value]
    } else {
        cat![f; f.space(), value, tail]
    };
    cat![f; token, value]
}
