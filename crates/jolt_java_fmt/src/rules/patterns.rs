use jolt_fmt_ir::{Doc, concat, force_group, group, hard_line, indent, line, soft_line, text};
use jolt_java_syntax::{
    ComponentPattern, JavaComment, JavaSyntaxToken, MatchAllPattern, Pattern, RecordPattern,
    RecordPatternComponentEntry, TypePattern,
};

use crate::helpers::comments::{
    comment_forces_line, format_comment, format_leading_comments, format_token_text,
    format_trailing_comments, format_trailing_comments_before_line_break,
};
use crate::rules::types::format_type;
use crate::rules::variables::format_local_variable_declaration;

pub(crate) fn format_pattern(pattern: &Pattern) -> Doc {
    match pattern {
        Pattern::TypePattern(pattern) => format_type_pattern(pattern),
        Pattern::RecordPattern(pattern) => format_record_pattern(pattern),
        Pattern::ComponentPattern(pattern) => format_component_pattern(pattern),
        Pattern::MatchAllPattern(pattern) => format_match_all_pattern(pattern),
    }
}

fn format_type_pattern(pattern: &TypePattern) -> Doc {
    pattern
        .variable()
        .map_or_else(jolt_fmt_ir::nil, |variable| {
            format_local_variable_declaration(&variable)
        })
}

fn format_record_pattern(pattern: &RecordPattern) -> Doc {
    concat([
        pattern
            .ty()
            .map_or_else(jolt_fmt_ir::nil, |ty| format_type(&ty)),
        format_record_pattern_components(pattern),
    ])
}

fn format_record_pattern_components(pattern: &RecordPattern) -> Doc {
    let entries = pattern.entries().collect::<Vec<_>>();
    if entries.is_empty() {
        return format_empty_record_pattern_components(pattern);
    }

    group(concat([
        format_record_pattern_open(pattern),
        indent(concat([
            format_open_record_pattern_spacing(pattern),
            format_record_pattern_component_entries(entries),
        ])),
        format_record_pattern_close_with_spacing(pattern),
    ]))
}

fn format_empty_record_pattern_components(pattern: &RecordPattern) -> Doc {
    if !record_pattern_has_dangling_comments(pattern) {
        return concat([
            format_record_pattern_open(pattern),
            format_record_pattern_close_delimiter(pattern),
        ]);
    }

    force_group(concat([
        format_record_pattern_open(pattern),
        indent(concat([
            hard_line(),
            format_record_pattern_dangling_comments(pattern),
        ])),
        hard_line(),
        format_record_pattern_close_delimiter_without_leading(pattern),
    ]))
}

fn record_pattern_has_dangling_comments(pattern: &RecordPattern) -> bool {
    pattern
        .open_paren()
        .is_some_and(|token| !token.trailing_comments().is_empty())
        || pattern
            .close_paren()
            .is_some_and(|token| !token.leading_comments().is_empty())
}

fn format_record_pattern_open(pattern: &RecordPattern) -> Doc {
    pattern.open_paren().map_or_else(
        || text("("),
        |open| concat([format_leading_comments(&open), text("(")]),
    )
}

fn format_open_record_pattern_spacing(pattern: &RecordPattern) -> Doc {
    let Some(open) = pattern.open_paren() else {
        return soft_line();
    };

    if open.trailing_comments().is_empty() {
        return soft_line();
    }

    concat([
        format_trailing_comments_before_line_break(&open),
        if open.trailing_comments().iter().any(comment_forces_line) {
            hard_line()
        } else {
            soft_line()
        },
    ])
}

fn format_record_pattern_component_entries(entries: Vec<RecordPatternComponentEntry>) -> Doc {
    let mut docs = Vec::new();
    let entries_len = entries.len();

    for (index, entry) in entries.into_iter().enumerate() {
        docs.push(format_component_pattern(&entry.component));
        if let Some(comma) = entry.comma {
            docs.push(format_record_pattern_separator(&comma));
        } else if index + 1 < entries_len {
            docs.push(line());
        }
    }

    concat(docs)
}

fn format_record_pattern_separator(comma: &JavaSyntaxToken) -> Doc {
    concat([
        format_leading_comments(comma),
        text(","),
        format_trailing_comments_before_line_break(comma),
        if comma.trailing_comments().iter().any(comment_forces_line) {
            hard_line()
        } else {
            line()
        },
    ])
}

fn format_record_pattern_close_with_spacing(pattern: &RecordPattern) -> Doc {
    let close_has_leading_comments = pattern
        .close_paren()
        .as_ref()
        .is_some_and(|token| !token.leading_comments().is_empty());

    concat([
        if close_has_leading_comments {
            line()
        } else {
            soft_line()
        },
        format_record_pattern_close_delimiter(pattern),
    ])
}

fn format_record_pattern_close_delimiter(pattern: &RecordPattern) -> Doc {
    let close = pattern.close_paren();
    let close_has_leading_comments = close
        .as_ref()
        .is_some_and(|token| !token.leading_comments().is_empty());
    close.map_or_else(
        || text(")"),
        |close| {
            concat([
                if close_has_leading_comments {
                    format_leading_comments(&close)
                } else {
                    jolt_fmt_ir::nil()
                },
                text(")"),
                format_trailing_comments(&close),
            ])
        },
    )
}

fn format_record_pattern_close_delimiter_without_leading(pattern: &RecordPattern) -> Doc {
    pattern.close_paren().map_or_else(
        || text(")"),
        |close| concat([text(")"), format_trailing_comments(&close)]),
    )
}

fn format_record_pattern_dangling_comments(pattern: &RecordPattern) -> Doc {
    let mut docs = Vec::new();

    if let Some(open) = pattern.open_paren() {
        push_dangling_comments(&mut docs, open.trailing_comments());
    }
    if let Some(close) = pattern.close_paren() {
        push_dangling_comments(&mut docs, close.leading_comments());
    }

    concat(docs)
}

fn push_dangling_comments(docs: &mut Vec<Doc>, comments: Vec<JavaComment>) {
    for comment in comments {
        if !docs.is_empty() {
            docs.push(hard_line());
        }
        docs.push(format_comment(&comment));
    }
}

fn format_component_pattern(pattern: &ComponentPattern) -> Doc {
    pattern
        .pattern()
        .map_or_else(jolt_fmt_ir::nil, |pattern| format_pattern(&pattern))
}

fn format_match_all_pattern(pattern: &MatchAllPattern) -> Doc {
    pattern
        .underscore()
        .map_or_else(|| text("_"), |token| format_token_text(token.text()))
}
