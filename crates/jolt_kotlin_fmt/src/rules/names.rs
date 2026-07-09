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
    let mut docs = doc.list();
    let trailing_dot = name.trailing_dot();

    while let Some(segment) = segments.next() {
        if let Some(dot) = segment.dot_before {
            let dot = format_name_dot(doc, &dot);
            docs.push(dot, doc);
        }

        let is_last = segments.peek().is_none();
        let followed_by_dot = segments
            .peek()
            .is_some_and(|next| next.dot_before.is_some())
            || (is_last && trailing_dot.is_some());

        if let Some(token) = segment.name.first_token() {
            let token = format_inline_name_segment(doc, &token, followed_by_dot);
            docs.push(token, doc);
        }
    }

    if let Some(dot) = trailing_dot {
        let dot = format_name_dot(doc, &dot);
        docs.push(dot, doc);
    }

    docs.finish(doc)
}

fn format_multiline_qualified_name<'source>(
    doc: &mut DocBuilder<'source>,
    name: &QualifiedName<'source>,
) -> Doc<'source> {
    let mut docs = doc.list();
    let mut tail_docs = doc.list();
    let mut before_first_dot = true;
    let trailing_dot = name.trailing_dot();

    for segment in name.segments() {
        if let Some(dot) = segment.dot_before {
            before_first_dot = false;
            let hard_line = doc.hard_line();
            tail_docs.push(hard_line, doc);
            let dot = format_name_dot(doc, &dot);
            tail_docs.push(dot, doc);
        }

        let Some(token) = segment.name.first_token() else {
            continue;
        };

        if before_first_dot {
            let token = format_name_segment(doc, &token);
            docs.push(token, doc);
        } else {
            let token = format_name_segment(doc, &token);
            tail_docs.push(token, doc);
        }
    }

    if let Some(dot) = trailing_dot {
        let hard_line = doc.hard_line();
        tail_docs.push(hard_line, doc);
        let dot = format_name_dot(doc, &dot);
        tail_docs.push(dot, doc);
    }

    let tail = tail_docs.finish(doc);
    let tail = doc.indent(tail);
    docs.push(tail, doc);
    docs.finish(doc)
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
    let mut docs = doc.list();
    for comment in comments {
        let space = doc.space();
        docs.push(space, doc);
        let comment_doc = format_comment(doc, &comment);
        docs.push(comment_doc, doc);
        if comment_forces_line(&comment) {
            let hard_line = doc.hard_line();
            docs.push(hard_line, doc);
        }
    }
    docs.finish(doc)
}

fn format_inline_comments<'source>(
    doc: &mut DocBuilder<'source>,
    comments: impl IntoIterator<Item = KotlinComment<'source>>,
) -> Doc<'source> {
    let mut docs = doc.list();
    for comment in comments {
        let space = doc.space();
        docs.push(space, doc);
        let comment_doc = format_comment(doc, &comment);
        docs.push(comment_doc, doc);
        if comment_forces_line(&comment) {
            let hard_line = doc.hard_line();
            docs.push(hard_line, doc);
        } else {
            let space = doc.space();
            docs.push(space, doc);
        }
    }
    docs.finish(doc)
}
