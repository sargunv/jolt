//! A language-agnostic tree dump for parser snapshots.

use std::fmt::Write as _;

use crate::{Language, SyntaxElement, SyntaxNode, TriviaPiece};

/// Dumps `root` one element per line, indented two spaces per level. Tokens
/// show their text, range, and trivia; empty slots are not shown.
#[must_use]
pub fn debug_tree<L: Language>(root: SyntaxNode<'_>) -> String {
    let mut output = String::new();
    let mut stack = vec![(SyntaxElement::Node(root), 0usize)];
    while let Some((element, depth)) = stack.pop() {
        for _ in 0..depth {
            output.push_str("  ");
        }
        match element {
            SyntaxElement::Node(node) => {
                output.push_str(L::kind_name(node.kind()));
                stack.extend(node.children().rev().map(|child| (child, depth + 1)));
            }
            SyntaxElement::Token(token) => {
                let range = token.text_range();
                write!(
                    output,
                    "{} {:?} @ {range}",
                    L::kind_name(token.kind()),
                    token.text()
                )
                .expect("writing to a string cannot fail");
                write_trivia(&mut output, " leading=", token.leading());
                write_trivia(&mut output, " trailing=", token.trailing());
            }
        }
        output.push('\n');
    }
    output
}

fn write_trivia<'a>(
    output: &mut String,
    label: &str,
    trivia: impl Iterator<Item = TriviaPiece<'a>>,
) {
    let mut trivia = trivia.peekable();
    if trivia.peek().is_none() {
        return;
    }
    output.push_str(label);
    output.push('[');
    for (index, piece) in trivia.enumerate() {
        if index > 0 {
            output.push_str(", ");
        }
        write!(output, "{:?} {:?}", piece.kind, piece.text)
            .expect("writing to a string cannot fail");
    }
    output.push(']');
}
