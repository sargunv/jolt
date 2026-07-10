use std::cmp::Ordering;

use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_java_syntax::{JavaComment, JavaSyntaxToken, NameSegment, NameSyntax};

use crate::helpers::comments::{
    LeadingTrivia, TrailingTrivia, comment_forces_line, format_comment, format_token,
};

pub(crate) fn format_name<'source>(
    name: &NameSyntax<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    if segments_have_line_comments(name.segments_with_annotations()) {
        return format_multiline_name(doc, name.segments_with_annotations());
    }

    format_inline_name(doc, name.segments_with_annotations())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct NameSortKey<'source> {
    segments: Vec<&'source str>,
    on_demand: bool,
}

impl<'source> NameSortKey<'source> {
    pub(crate) fn new(name: &NameSyntax<'source>, on_demand: bool) -> Self {
        Self {
            segments: name.segments().map(|segment| segment.text()).collect(),
            on_demand,
        }
    }

    pub(crate) fn recovered() -> Self {
        Self {
            segments: Vec::new(),
            on_demand: false,
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

fn segments_have_line_comments<'source>(
    segments: impl IntoIterator<Item = NameSegment<'source>>,
) -> bool {
    segments.into_iter().any(|segment| {
        token_has_line_comments(&segment.identifier)
            || segment
                .dot_before
                .as_ref()
                .is_some_and(token_has_line_comments)
    })
}

fn token_has_line_comments(token: &JavaSyntaxToken<'_>) -> bool {
    token
        .leading_comments()
        .chain(token.trailing_comments())
        .any(|comment| comment_forces_line(&comment))
}

fn format_inline_name<'source>(
    doc: &mut DocBuilder<'source>,
    segments: impl IntoIterator<Item = NameSegment<'source>>,
) -> Doc<'source> {
    let mut segments = segments.into_iter().peekable();
    let mut index = 0;
    doc.concat_list(|docs| {
        while let Some(segment) = segments.next() {
            if index > 0 {
                let dot = segment
                    .dot_before
                    .as_ref()
                    .map_or_else(Doc::nil, |dot| format_name_dot(docs, dot));
                docs.push(dot);
            }
            let identifier =
                format_inline_name_segment_identifier(docs, &segment, segments.peek().is_some());
            docs.push(identifier);
            index += 1;
        }
    })
}

fn format_multiline_name<'source>(
    doc: &mut DocBuilder<'source>,
    segments: impl IntoIterator<Item = NameSegment<'source>>,
) -> Doc<'source> {
    let mut segments = segments.into_iter();
    let Some(first) = segments.next() else {
        return Doc::nil();
    };

    let rest = doc.concat_list(|rest| {
        for segment in segments {
            let hard_line = rest.hard_line();
            let segment = format_leading_dot_segment(rest, &segment);
            let segment = doc_concat!(rest, [hard_line, segment]);
            rest.push(segment);
        }
    });

    doc_concat!(
        doc,
        [
            format_name_segment_identifier(doc, &first),
            doc_indent!(doc, rest),
        ]
    )
}

fn format_leading_dot_segment<'source>(
    doc: &mut DocBuilder<'source>,
    segment: &NameSegment<'source>,
) -> Doc<'source> {
    doc_concat!(
        doc,
        [
            segment
                .dot_before
                .as_ref()
                .map_or_else(Doc::nil, |dot| format_name_dot(doc, dot)),
            format_name_segment_identifier(doc, segment),
        ]
    )
}

fn format_name_dot<'source>(
    doc: &mut DocBuilder<'source>,
    dot: &JavaSyntaxToken<'source>,
) -> Doc<'source> {
    doc_concat!(
        doc,
        [
            format_leading_dot_comments(doc, dot.leading_comments()),
            format_token(
                doc,
                dot,
                LeadingTrivia::SuppressAlreadyHandled,
                TrailingTrivia::RelocatedToEnclosingContext,
            ),
            format_inline_comments(doc, dot.trailing_comments()),
        ]
    )
}

fn format_name_segment_identifier<'source>(
    doc: &mut DocBuilder<'source>,
    segment: &NameSegment<'source>,
) -> Doc<'source> {
    format_token(
        doc,
        &segment.identifier,
        LeadingTrivia::Preserve,
        TrailingTrivia::BeforeLineBreak,
    )
}

fn format_inline_name_segment_identifier<'source>(
    doc: &mut DocBuilder<'source>,
    segment: &NameSegment<'source>,
    followed_by_dot: bool,
) -> Doc<'source> {
    doc_concat!(
        doc,
        [
            format_inline_comments(doc, segment.identifier.leading_comments()),
            format_token(
                doc,
                &segment.identifier,
                LeadingTrivia::SuppressAlreadyHandled,
                TrailingTrivia::RelocatedToEnclosingContext,
            ),
            if followed_by_dot {
                format_leading_dot_comments(doc, segment.identifier.trailing_comments())
            } else {
                format_inline_comments(doc, segment.identifier.trailing_comments())
            },
        ]
    )
}

fn format_leading_dot_comments<'source>(
    doc: &mut DocBuilder<'source>,
    comments: impl IntoIterator<Item = JavaComment<'source>>,
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
    comments: impl IntoIterator<Item = JavaComment<'source>>,
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
