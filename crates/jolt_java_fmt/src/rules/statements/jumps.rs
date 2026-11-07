//! `return`, `throw`, `yield`, `break`, `continue` and `assert`: a keyword,
//! an optional value, and `;`.

use jolt_fmt::{Doc, cat, indent};
use jolt_java_syntax::{
    AssertStatement, BreakStatement, ContinueStatement, ReturnStatement, ThrowStatement,
    YieldStatement,
};
use jolt_syntax::{AstNode, SyntaxElement, SyntaxToken};

use crate::JavaFormatter;

/// `keyword value;`. When a comment ends the keyword's line, the value (and
/// `;`) continues one indent deeper: after a trailing comment of the keyword
/// (`return // c`) the pending comment breaks the group and the value may
/// break further by its own rule; after a comment leading the value that
/// ends its line, the value keeps that comment on the keyword's line.
fn keyword_value<'a>(
    f: &mut JavaFormatter<'a>,
    keyword: Option<SyntaxToken<'a>>,
    value: Option<(SyntaxElement<'a>, Doc)>,
    semicolon: Option<SyntaxToken<'a>>,
) -> Doc {
    let keyword_comments =
        keyword.is_some_and(|keyword| f.has_trailing_comments(SyntaxElement::Token(keyword)));
    let keyword = f.token_opt(keyword);
    let semicolon = f.token_opt(semicolon);
    let Some((element, value)) = value else {
        return cat![f; keyword, semicolon];
    };
    let value = if keyword_comments {
        let value = indent![f; f.line(), value, semicolon];
        f.group(value)
    } else if f.leading_comments_break(element) {
        let value = indent![f; value, semicolon];
        cat![f; f.space(), value]
    } else {
        cat![f; f.space(), value, semicolon]
    };
    cat![f; keyword, value]
}

/// The value of a keyword statement, with the element it starts with.
fn value<'a, N: AstNode<'a>>(
    f: &mut JavaFormatter<'a>,
    node: Option<N>,
) -> Option<(SyntaxElement<'a>, Doc)> {
    let node = node?;
    let doc = f.fmt(node);
    Some((SyntaxElement::Node(node.syntax()), doc))
}

fn label<'a>(
    f: &mut JavaFormatter<'a>,
    token: Option<SyntaxToken<'a>>,
) -> Option<(SyntaxElement<'a>, Doc)> {
    let token = token?;
    let doc = f.token(token);
    Some((SyntaxElement::Token(token), doc))
}

pub(crate) fn return_statement<'a>(f: &mut JavaFormatter<'a>, node: ReturnStatement<'a>) -> Doc {
    let value = value(f, node.expression());
    keyword_value(f, node.return_keyword(), value, node.semicolon())
}

pub(crate) fn throw_statement<'a>(f: &mut JavaFormatter<'a>, node: ThrowStatement<'a>) -> Doc {
    let value = value(f, node.expression());
    keyword_value(f, node.throw_keyword(), value, node.semicolon())
}

pub(crate) fn yield_statement<'a>(f: &mut JavaFormatter<'a>, node: YieldStatement<'a>) -> Doc {
    let value = value(f, node.expression());
    keyword_value(f, node.yield_keyword(), value, node.semicolon())
}

pub(crate) fn break_statement<'a>(f: &mut JavaFormatter<'a>, node: BreakStatement<'a>) -> Doc {
    let label = label(f, node.label());
    keyword_value(f, node.break_keyword(), label, node.semicolon())
}

pub(crate) fn continue_statement<'a>(
    f: &mut JavaFormatter<'a>,
    node: ContinueStatement<'a>,
) -> Doc {
    let label = label(f, node.label());
    keyword_value(f, node.continue_keyword(), label, node.semicolon())
}

/// `assert condition : message;`.
pub(crate) fn assert_statement<'a>(f: &mut JavaFormatter<'a>, node: AssertStatement<'a>) -> Doc {
    let message = if node.colon().is_some() || node.message().is_some() {
        let colon = f.token_opt(node.colon());
        let message = f.fmt_opt(node.message());
        cat![f; f.space(), colon, f.space(), message]
    } else {
        Doc::NIL
    };
    let value = match value(f, node.condition()) {
        Some((element, condition)) => Some((element, cat![f; condition, message])),
        // A recovered assert without a condition still prints its message.
        None => node
            .colon()
            .map(SyntaxElement::Token)
            .or_else(|| {
                node.message()
                    .map(|message| SyntaxElement::Node(message.syntax()))
            })
            .map(|element| (element, message)),
    };
    keyword_value(f, node.assert_keyword(), value, node.semicolon())
}
