use jolt_fmt_ir::{Doc, concat, hard_line, literal_text, text};
use jolt_java_syntax::{JavaComment, JavaCommentKind, JavaSyntaxKind, JavaSyntaxToken};

pub(crate) fn format_token_sequence(tokens: &[JavaSyntaxToken]) -> Doc {
    let mut docs = Vec::new();
    let mut previous_kind = None;
    let mut after_forced_break = false;

    for token in tokens {
        for comment in token.leading_comments() {
            if !docs.is_empty() && !after_forced_break {
                docs.push(hard_line());
            }
            docs.push(format_comment(&comment));
            docs.push(hard_line());
            previous_kind = None;
            after_forced_break = true;
        }

        if !after_forced_break
            && previous_kind.is_some_and(|previous| needs_space(previous, token.kind()))
        {
            docs.push(text(" "));
        }

        docs.push(format_token_text(token.text()));
        after_forced_break = false;

        let mut forced_break_after_token = false;
        for comment in token.trailing_comments() {
            docs.push(text(" "));
            docs.push(format_comment(&comment));
            if comment_forces_line(&comment) {
                docs.push(hard_line());
                forced_break_after_token = true;
            }
        }

        if forced_break_after_token {
            previous_kind = None;
            after_forced_break = true;
        } else {
            previous_kind = Some(token.kind());
        }
    }

    concat(docs)
}

pub(crate) fn tokens_have_comments(tokens: &[JavaSyntaxToken]) -> bool {
    tokens
        .iter()
        .any(|token| !token.leading_comments().is_empty() || !token.trailing_comments().is_empty())
}

pub(crate) fn tokens_end_with_forced_line(tokens: &[JavaSyntaxToken]) -> bool {
    tokens
        .last()
        .is_some_and(|token| token.trailing_comments().iter().any(comment_forces_line))
}

pub(crate) fn format_comment(comment: &JavaComment) -> Doc {
    literal_text(comment.text().trim().to_owned())
}

fn format_token_text(token_text: &str) -> Doc {
    if token_text.contains(['\n', '\r']) {
        literal_text(token_text.to_owned())
    } else {
        text(token_text.to_owned())
    }
}

pub(crate) fn comment_forces_line(comment: &JavaComment) -> bool {
    comment.kind() == JavaCommentKind::Line || comment.text().contains(['\n', '\r'])
}

fn needs_space(previous: JavaSyntaxKind, current: JavaSyntaxKind) -> bool {
    if suppresses_space_before(current) || suppresses_space_after(previous) {
        return false;
    }

    if current == JavaSyntaxKind::LParen
        && matches!(
            previous,
            JavaSyntaxKind::Identifier
                | JavaSyntaxKind::ThisKw
                | JavaSyntaxKind::SuperKw
                | JavaSyntaxKind::Gt
                | JavaSyntaxKind::RBracket
        )
    {
        return false;
    }

    if current == JavaSyntaxKind::LBracket {
        return false;
    }

    if previous == JavaSyntaxKind::Comma || previous == JavaSyntaxKind::Semicolon {
        return true;
    }

    true
}

const fn suppresses_space_before(kind: JavaSyntaxKind) -> bool {
    matches!(
        kind,
        JavaSyntaxKind::RParen
            | JavaSyntaxKind::RBracket
            | JavaSyntaxKind::RBrace
            | JavaSyntaxKind::Comma
            | JavaSyntaxKind::Semicolon
            | JavaSyntaxKind::Dot
            | JavaSyntaxKind::DoubleColon
            | JavaSyntaxKind::Colon
            | JavaSyntaxKind::Ellipsis
            | JavaSyntaxKind::Lt
            | JavaSyntaxKind::Gt
    )
}

const fn suppresses_space_after(kind: JavaSyntaxKind) -> bool {
    matches!(
        kind,
        JavaSyntaxKind::LParen
            | JavaSyntaxKind::LBracket
            | JavaSyntaxKind::LBrace
            | JavaSyntaxKind::Dot
            | JavaSyntaxKind::DoubleColon
            | JavaSyntaxKind::At
            | JavaSyntaxKind::Lt
    )
}
