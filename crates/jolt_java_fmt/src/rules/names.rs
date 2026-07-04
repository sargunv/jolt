use std::cmp::Ordering;

use jolt_fmt_ir::{Doc, concat, hard_line, indent, text};
use jolt_java_syntax::{JavaComment, JavaSyntaxToken, NameSegment, NameSyntax};

use crate::helpers::comments::{
    LeadingTrivia, TrailingTrivia, comment_forces_line, format_comment, format_token,
    format_token_text, format_token_with_comments,
};

pub(crate) fn format_name<'source>(name: &NameSyntax<'source>) -> Doc<'source> {
    let segments = name.segments_with_annotations().collect::<Vec<_>>();
    if segments_have_line_comments(&segments) {
        return format_multiline_name(segments);
    }
    if segments_have_comments(&segments) {
        return format_inline_name(&segments);
    }

    jolt_fmt_ir::join(
        &text("."),
        segments
            .into_iter()
            .map(|segment| format_token_with_comments(&segment.identifier)),
    )
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

fn segments_have_comments(segments: &[NameSegment<'_>]) -> bool {
    segments.iter().any(|segment| {
        token_has_comments(&segment.identifier)
            || segment.dot_before.as_ref().is_some_and(token_has_comments)
    })
}

fn segments_have_line_comments(segments: &[NameSegment<'_>]) -> bool {
    segments.iter().any(|segment| {
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

fn token_has_comments(token: &JavaSyntaxToken<'_>) -> bool {
    !token.leading_comments().is_empty() || !token.trailing_comments().is_empty()
}

fn format_inline_name<'source>(segments: &[NameSegment<'source>]) -> Doc<'source> {
    let mut docs = Vec::new();
    let segments_len = segments.len();
    for (index, segment) in segments.iter().enumerate() {
        if index > 0 {
            docs.push(
                segment
                    .dot_before
                    .as_ref()
                    .map_or_else(|| text("."), format_name_dot),
            );
        }
        docs.push(format_inline_name_segment_identifier(
            segment,
            index + 1 < segments_len,
        ));
    }
    concat(docs)
}

fn format_multiline_name(segments: Vec<NameSegment<'_>>) -> Doc<'_> {
    let mut segments = segments.into_iter();
    let Some(first) = segments.next() else {
        return jolt_fmt_ir::nil();
    };

    let rest = segments
        .map(|segment| concat([hard_line(), format_leading_dot_segment(&segment)]))
        .collect::<Vec<_>>();

    concat([format_name_segment_identifier(&first), indent(concat(rest))])
}

fn format_leading_dot_segment<'source>(segment: &NameSegment<'source>) -> Doc<'source> {
    concat([
        segment
            .dot_before
            .as_ref()
            .map_or_else(|| text("."), format_name_dot),
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
        docs.push(text(" "));
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
        docs.push(text(" "));
        docs.push(format_comment(&comment));
        if comment_forces_line(&comment) {
            docs.push(hard_line());
        } else {
            docs.push(text(" "));
        }
    }
    concat(docs)
}
