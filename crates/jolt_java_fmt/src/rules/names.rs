use jolt_fmt_ir::space;
use std::cmp::Ordering;

use jolt_fmt_ir::{Doc, concat, hard_line, indent};
use jolt_java_syntax::{JavaComment, JavaSyntaxToken, NameSegment, NameSyntax};

use crate::helpers::comments::{
    LeadingTrivia, TrailingTrivia, comment_forces_line, format_comment, format_token,
    format_token_text,
};

pub(crate) fn format_name<'source>(name: &NameSyntax<'source>) -> Doc<'source> {
    if segments_have_line_comments(name.segments_with_annotations()) {
        return format_multiline_name(name.segments_with_annotations());
    }

    format_inline_name(name.segments_with_annotations())
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
    segments: impl IntoIterator<Item = NameSegment<'source>>,
) -> Doc<'source> {
    let mut docs = Vec::new();
    let mut segments = segments.into_iter().peekable();
    let mut index = 0;
    while let Some(segment) = segments.next() {
        if index > 0 {
            docs.push(
                segment
                    .dot_before
                    .as_ref()
                    .map_or_else(jolt_fmt_ir::nil, format_name_dot),
            );
        }
        docs.push(format_inline_name_segment_identifier(
            &segment,
            segments.peek().is_some(),
        ));
        index += 1;
    }
    concat(docs)
}

fn format_multiline_name<'source>(
    segments: impl IntoIterator<Item = NameSegment<'source>>,
) -> Doc<'source> {
    let mut segments = segments.into_iter();
    let Some(first) = segments.next() else {
        return jolt_fmt_ir::nil();
    };

    concat([
        format_name_segment_identifier(&first),
        indent(concat(segments.map(|segment| {
            concat([hard_line(), format_leading_dot_segment(&segment)])
        }))),
    ])
}

fn format_leading_dot_segment<'source>(segment: &NameSegment<'source>) -> Doc<'source> {
    concat([
        segment
            .dot_before
            .as_ref()
            .map_or_else(jolt_fmt_ir::nil, format_name_dot),
        format_name_segment_identifier(segment),
    ])
}

fn format_name_dot<'source>(dot: &JavaSyntaxToken<'source>) -> Doc<'source> {
    concat([
        format_leading_dot_comments(dot.leading_comments()),
        format_token_text(dot.text()),
        format_inline_comments(dot.trailing_comments()),
    ])
}

fn format_name_segment_identifier<'source>(segment: &NameSegment<'source>) -> Doc<'source> {
    format_token(
        &segment.identifier,
        LeadingTrivia::Preserve,
        TrailingTrivia::BeforeLineBreak,
    )
}

fn format_inline_name_segment_identifier<'source>(
    segment: &NameSegment<'source>,
    followed_by_dot: bool,
) -> Doc<'source> {
    concat([
        format_inline_comments(segment.identifier.leading_comments()),
        format_token_text(segment.identifier.text()),
        if followed_by_dot {
            format_leading_dot_comments(segment.identifier.trailing_comments())
        } else {
            format_inline_comments(segment.identifier.trailing_comments())
        },
    ])
}

fn format_leading_dot_comments<'source>(
    comments: impl IntoIterator<Item = JavaComment<'source>>,
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
    comments: impl IntoIterator<Item = JavaComment<'source>>,
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
