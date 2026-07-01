use jolt_fmt_ir::{Doc, concat, hard_line, indent, text};
use jolt_java_syntax::{JavaComment, JavaSyntaxToken, NameSegment, NameSyntax};

use crate::helpers::comments::{
    comment_forces_line, format_comment, format_leading_comments, format_token_text,
    format_trailing_comments_before_line_break,
};
use crate::helpers::names::qualified_name;

pub(crate) fn format_name(name: &NameSyntax) -> Doc {
    let segments = name.segments_with_annotations().collect::<Vec<_>>();
    if segments_have_line_comments(&segments) {
        return format_multiline_name(segments);
    }
    if segments_have_comments(&segments) {
        return format_inline_name(&segments);
    }

    qualified_name(
        segments
            .into_iter()
            .map(|segment| format_token_text(segment.identifier.text()))
            .collect(),
    )
}

pub(crate) fn name_key(name: &NameSyntax) -> String {
    name.segments()
        .map(|segment| segment.text().to_owned())
        .collect::<Vec<_>>()
        .join(".")
}

fn segments_have_comments(segments: &[NameSegment]) -> bool {
    segments.iter().any(|segment| {
        token_has_comments(&segment.identifier)
            || segment.dot_before.as_ref().is_some_and(token_has_comments)
    })
}

fn segments_have_line_comments(segments: &[NameSegment]) -> bool {
    segments.iter().any(|segment| {
        token_has_line_comments(&segment.identifier)
            || segment
                .dot_before
                .as_ref()
                .is_some_and(token_has_line_comments)
    })
}

fn token_has_line_comments(token: &JavaSyntaxToken) -> bool {
    token
        .leading_comments()
        .iter()
        .chain(token.trailing_comments().iter())
        .any(comment_forces_line)
}

fn token_has_comments(token: &JavaSyntaxToken) -> bool {
    !token.leading_comments().is_empty() || !token.trailing_comments().is_empty()
}

fn format_inline_name(segments: &[NameSegment]) -> Doc {
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

fn format_multiline_name(segments: Vec<NameSegment>) -> Doc {
    let mut segments = segments.into_iter();
    let Some(first) = segments.next() else {
        return jolt_fmt_ir::nil();
    };

    let rest = segments
        .map(|segment| concat([hard_line(), format_leading_dot_segment(&segment)]))
        .collect::<Vec<_>>();

    concat([format_name_segment_identifier(&first), indent(concat(rest))])
}

fn format_leading_dot_segment(segment: &NameSegment) -> Doc {
    concat([
        segment
            .dot_before
            .as_ref()
            .map_or_else(|| text("."), format_name_dot),
        format_name_segment_identifier(segment),
    ])
}

fn format_name_dot(dot: &JavaSyntaxToken) -> Doc {
    concat([
        format_leading_dot_comments(dot.leading_comments()),
        text("."),
        format_inline_comments(dot.trailing_comments()),
    ])
}

fn format_name_segment_identifier(segment: &NameSegment) -> Doc {
    concat([
        format_leading_comments(&segment.identifier),
        format_token_text(segment.identifier.text()),
        format_trailing_comments_before_line_break(&segment.identifier),
    ])
}

fn format_inline_name_segment_identifier(segment: &NameSegment, followed_by_dot: bool) -> Doc {
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

fn format_leading_dot_comments(comments: Vec<JavaComment>) -> Doc {
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

fn format_inline_comments(comments: Vec<JavaComment>) -> Doc {
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
