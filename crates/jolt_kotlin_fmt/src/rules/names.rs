use jolt_fmt_ir::{Doc, concat, hard_line, indent, space};
use jolt_kotlin_syntax::{KotlinComment, KotlinSyntaxKind, KotlinSyntaxToken, QualifiedName};
use std::cmp::Ordering;

use crate::helpers::comments::{
    LeadingTrivia, TrailingTrivia, comment_forces_line, format_comment, format_token_text,
};

pub(crate) fn format_name<'source>(name: &jolt_kotlin_syntax::Name<'source>) -> Doc<'source> {
    let Some(token) = name.token_iter().next() else {
        return jolt_fmt_ir::nil();
    };

    crate::helpers::comments::format_token(
        &token,
        LeadingTrivia::Preserve,
        TrailingTrivia::Preserve,
    )
}

pub(crate) fn format_qualified_name<'source>(name: &QualifiedName<'source>) -> Doc<'source> {
    let tokens = name.token_iter().collect::<Vec<_>>();
    if tokens_have_line_comments(&tokens) {
        return format_multiline_qualified_name(&tokens);
    }

    format_inline_qualified_name(&tokens)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct NameSortKey<'source> {
    segments: Vec<&'source str>,
    on_demand: bool,
}

impl<'source> NameSortKey<'source> {
    pub(crate) fn empty() -> Self {
        Self {
            segments: Vec::new(),
            on_demand: false,
        }
    }

    pub(crate) fn new(name: &QualifiedName<'source>, on_demand: bool) -> Self {
        Self {
            segments: name
                .token_iter()
                .filter(is_name_segment_token)
                .map(|token| token.text())
                .collect(),
            on_demand,
        }
    }

    fn chars(&self) -> impl Iterator<Item = char> + '_ {
        self.segments
            .iter()
            .enumerate()
            .flat_map(|(index, segment)| {
                (index > 0)
                    .then_some(".")
                    .into_iter()
                    .chain(std::iter::once(*segment))
            })
            .chain(self.on_demand.then_some(".*"))
            .flat_map(str::chars)
    }
}

impl Ord for NameSortKey<'_> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.chars().cmp(other.chars())
    }
}

impl PartialOrd for NameSortKey<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn tokens_have_line_comments(tokens: &[KotlinSyntaxToken<'_>]) -> bool {
    tokens.iter().any(|token| {
        token
            .leading_comments()
            .chain(token.trailing_comments())
            .any(|comment| comment_forces_line(&comment))
    })
}

fn format_inline_qualified_name<'source>(tokens: &[KotlinSyntaxToken<'source>]) -> Doc<'source> {
    let mut docs = Vec::new();

    for (index, token) in tokens.iter().enumerate() {
        if token.kind() == KotlinSyntaxKind::Dot {
            docs.push(format_name_dot(token));
            continue;
        }

        docs.push(format_inline_name_segment(
            token,
            tokens
                .get(index + 1)
                .is_some_and(|next| next.kind() == KotlinSyntaxKind::Dot),
        ));
    }

    concat(docs)
}

fn format_multiline_qualified_name<'source>(tokens: &[KotlinSyntaxToken<'source>]) -> Doc<'source> {
    let mut docs = Vec::new();
    let mut tail_docs = Vec::new();
    let mut before_first_dot = true;

    for token in tokens {
        if token.kind() == KotlinSyntaxKind::Dot {
            before_first_dot = false;
            tail_docs.push(hard_line());
            tail_docs.push(format_name_dot(token));
            continue;
        }

        if before_first_dot {
            docs.push(format_name_segment(token));
        } else {
            tail_docs.push(format_name_segment(token));
        }
    }

    docs.push(indent(concat(tail_docs)));
    concat(docs)
}

fn format_name_dot<'source>(dot: &KotlinSyntaxToken<'source>) -> Doc<'source> {
    concat([
        format_leading_dot_comments(dot.leading_comments()),
        format_token_text(dot.text()),
        format_inline_comments(dot.trailing_comments()),
    ])
}

fn format_name_segment<'source>(segment: &KotlinSyntaxToken<'source>) -> Doc<'source> {
    crate::helpers::comments::format_token(
        segment,
        LeadingTrivia::Preserve,
        TrailingTrivia::Preserve,
    )
}

fn format_inline_name_segment<'source>(
    segment: &KotlinSyntaxToken<'source>,
    followed_by_dot: bool,
) -> Doc<'source> {
    concat([
        format_inline_comments(segment.leading_comments()),
        format_token_text(segment.text()),
        if followed_by_dot {
            format_leading_dot_comments(segment.trailing_comments())
        } else {
            format_inline_comments(segment.trailing_comments())
        },
    ])
}

fn format_leading_dot_comments<'source>(
    comments: impl IntoIterator<Item = KotlinComment<'source>>,
) -> Doc<'source> {
    let mut docs = Vec::new();
    for comment in comments {
        docs.push(space());
        docs.push(format_comment(&comment));
        if comment_forces_line(&comment) {
            docs.push(hard_line());
        }
    }
    concat(docs)
}

fn format_inline_comments<'source>(
    comments: impl IntoIterator<Item = KotlinComment<'source>>,
) -> Doc<'source> {
    let mut docs = Vec::new();
    for comment in comments {
        docs.push(space());
        docs.push(format_comment(&comment));
        if comment_forces_line(&comment) {
            docs.push(hard_line());
        } else {
            docs.push(space());
        }
    }
    concat(docs)
}

fn is_name_segment_token(token: &KotlinSyntaxToken<'_>) -> bool {
    matches!(
        token.kind(),
        KotlinSyntaxKind::Identifier | KotlinSyntaxKind::FieldIdentifier
    )
}
