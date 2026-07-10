use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_kotlin_syntax::{KotlinComment, KotlinSyntaxToken, QualifiedName};
use std::cmp::Ordering;

use crate::helpers::comments::{
    LeadingTrivia, TrailingTrivia, comment_forces_line, format_comment, format_token_text,
};

pub(crate) fn format_name<'source>(
    doc: &mut DocBuilder<'source>,
    name: &jolt_kotlin_syntax::Name<'source>,
) -> Doc<'source> {
    let Some(token) = name.token_iter().next() else {
        return doc.nil();
    };

    crate::helpers::comments::format_token(
        doc,
        &token,
        LeadingTrivia::Preserve,
        TrailingTrivia::Preserve,
    )
}

pub(crate) fn format_qualified_name<'source>(
    doc: &mut DocBuilder<'source>,
    name: &QualifiedName<'source>,
) -> Doc<'source> {
    if tokens_have_line_comments(name) {
        return format_multiline_qualified_name(doc, name);
    }

    format_inline_qualified_name(doc, name)
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
            segments: name.identifiers().map(|token| token.text()).collect(),
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

fn tokens_have_line_comments(name: &QualifiedName<'_>) -> bool {
    name.identifiers().chain(name.dots()).any(|token| {
        token
            .leading_comments()
            .chain(token.trailing_comments())
            .any(|comment| comment_forces_line(&comment))
    })
}

fn format_inline_qualified_name<'source>(
    doc: &mut DocBuilder<'source>,
    name: &QualifiedName<'source>,
) -> Doc<'source> {
    let mut segments = name.segments().peekable();
    let trailing_dot = name.trailing_dot();
    doc.concat_list(|docs| {
        while let Some(segment) = segments.next() {
            if let Some(dot) = segment.dot_before {
                let dot = format_name_dot(docs, &dot);
                docs.push(dot);
            }

            let is_last = segments.peek().is_none();
            let followed_by_dot = segments
                .peek()
                .is_some_and(|next| next.dot_before.is_some())
                || (is_last && trailing_dot.is_some());

            if let Some(token) = segment.name.first_token() {
                let token = format_inline_name_segment(docs, &token, followed_by_dot);
                docs.push(token);
            }
        }

        if let Some(dot) = trailing_dot {
            let dot = format_name_dot(docs, &dot);
            docs.push(dot);
        }
    })
}

fn format_multiline_qualified_name<'source>(
    doc: &mut DocBuilder<'source>,
    name: &QualifiedName<'source>,
) -> Doc<'source> {
    let mut segments = name.segments().peekable();
    let trailing_dot = name.trailing_dot();
    doc.concat_list(|docs| {
        while segments
            .peek()
            .is_some_and(|segment| segment.dot_before.is_none())
        {
            let segment = segments.next().expect("peeked name segment exists");
            if let Some(token) = segment.name.first_token() {
                let token = format_name_segment(docs, &token);
                docs.push(token);
            }
        }

        let tail = docs.concat_list(|tail_docs| {
            for segment in segments {
                if let Some(dot) = segment.dot_before {
                    let hard_line = tail_docs.hard_line();
                    tail_docs.push(hard_line);
                    let dot = format_name_dot(tail_docs, &dot);
                    tail_docs.push(dot);
                }

                if let Some(token) = segment.name.first_token() {
                    let token = format_name_segment(tail_docs, &token);
                    tail_docs.push(token);
                }
            }

            if let Some(dot) = trailing_dot {
                let hard_line = tail_docs.hard_line();
                tail_docs.push(hard_line);
                let dot = format_name_dot(tail_docs, &dot);
                tail_docs.push(dot);
            }
        });

        let tail = docs.indent(tail);
        docs.push(tail);
    })
}

fn format_name_dot<'source>(
    doc: &mut DocBuilder<'source>,
    dot: &KotlinSyntaxToken<'source>,
) -> Doc<'source> {
    let leading = format_leading_dot_comments(doc, dot.leading_comments());
    let text = format_token_text(doc, dot.text());
    let trailing = format_inline_comments(doc, dot.trailing_comments());
    doc.concat([leading, text, trailing])
}

fn format_name_segment<'source>(
    doc: &mut DocBuilder<'source>,
    segment: &KotlinSyntaxToken<'source>,
) -> Doc<'source> {
    crate::helpers::comments::format_token(
        doc,
        segment,
        LeadingTrivia::Preserve,
        TrailingTrivia::Preserve,
    )
}

fn format_inline_name_segment<'source>(
    doc: &mut DocBuilder<'source>,
    segment: &KotlinSyntaxToken<'source>,
    followed_by_dot: bool,
) -> Doc<'source> {
    let leading = format_inline_comments(doc, segment.leading_comments());
    let text = format_token_text(doc, segment.text());
    let trailing = if followed_by_dot {
        format_leading_dot_comments(doc, segment.trailing_comments())
    } else {
        format_inline_comments(doc, segment.trailing_comments())
    };
    doc.concat([leading, text, trailing])
}

fn format_leading_dot_comments<'source>(
    doc: &mut DocBuilder<'source>,
    comments: impl IntoIterator<Item = KotlinComment<'source>>,
) -> Doc<'source> {
    doc.concat_list(|docs| {
        for comment in comments {
            let space = docs.space();
            docs.push(space);
            let comment_doc = format_comment(docs, &comment);
            docs.push(comment_doc);
            if comment_forces_line(&comment) {
                let hard_line = docs.hard_line();
                docs.push(hard_line);
            }
        }
    })
}

fn format_inline_comments<'source>(
    doc: &mut DocBuilder<'source>,
    comments: impl IntoIterator<Item = KotlinComment<'source>>,
) -> Doc<'source> {
    doc.concat_list(|docs| {
        for comment in comments {
            let space = docs.space();
            docs.push(space);
            let comment_doc = format_comment(docs, &comment);
            docs.push(comment_doc);
            if comment_forces_line(&comment) {
                let hard_line = docs.hard_line();
                docs.push(hard_line);
            } else {
                let space = docs.space();
                docs.push(space);
            }
        }
    })
}
