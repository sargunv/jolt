use crate::green::{GreenElement, GreenNode, GreenToken};

/// Reconstructs the exact source text represented by a green tree.
#[must_use]
pub fn green_text(node: &GreenNode) -> String {
    let mut text = String::with_capacity(node.text_len().get());
    write_node_text(node, &mut text);
    text
}

fn write_node_text(node: &GreenNode, out: &mut String) {
    for child in node.children() {
        match child {
            GreenElement::Node(node) => write_node_text(node, out),
            GreenElement::Token(token) => write_token_text(token, out),
        }
    }
}

fn write_token_text(token: &GreenToken, out: &mut String) {
    for trivia in token.leading() {
        out.push_str(trivia.text());
    }

    out.push_str(token.text());

    for trivia in token.trailing() {
        out.push_str(trivia.text());
    }
}
