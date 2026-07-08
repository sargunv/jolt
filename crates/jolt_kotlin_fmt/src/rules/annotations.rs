use jolt_fmt_ir::{Doc, concat};
use jolt_kotlin_syntax::{
    Annotation, AnnotationArgumentList, AnnotationUseSiteTarget, KotlinSyntaxToken,
};

use crate::helpers::comments::{
    LeadingTrivia, TrailingTrivia, format_token, format_token_sequence, token_has_comments,
};
use crate::helpers::lists::{CommaListItem, compact_parenthesized_list, parenthesized_list};
use crate::helpers::source::source_gap_is_trivia;
use crate::rules::expressions::format_value_argument;
use crate::rules::names::format_qualified_name;

pub(crate) fn format_annotation<'source>(annotation: &Annotation<'source>) -> Doc<'source> {
    format_annotation_with_leading(annotation, LeadingTrivia::Preserve)
}

pub(crate) fn format_annotation_with_leading<'source>(
    annotation: &Annotation<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    concat([
        annotation
            .at_token()
            .map_or_else(jolt_fmt_ir::nil, |token| {
                format_token(&token, leading, TrailingTrivia::RelocatedToEnclosingContext)
            }),
        annotation
            .use_site_target()
            .map_or_else(jolt_fmt_ir::nil, |target| {
                format_annotation_use_site_target(&target)
            }),
        annotation
            .name()
            .map_or_else(jolt_fmt_ir::nil, |name| format_qualified_name(&name)),
        annotation
            .argument_list()
            .map_or_else(jolt_fmt_ir::nil, |arguments| {
                format_annotation_argument_list(&arguments)
            }),
    ])
}

fn format_annotation_use_site_target<'source>(
    target: &AnnotationUseSiteTarget<'source>,
) -> Doc<'source> {
    concat([
        target
            .target_token()
            .map_or_else(jolt_fmt_ir::nil, |token| {
                format_token(
                    &token,
                    LeadingTrivia::Preserve,
                    TrailingTrivia::RelocatedToEnclosingContext,
                )
            }),
        target.colon_token().map_or_else(jolt_fmt_ir::nil, |token| {
            format_token(
                &token,
                LeadingTrivia::Preserve,
                TrailingTrivia::RelocatedToEnclosingContext,
            )
        }),
    ])
}

fn format_annotation_argument_list<'source>(
    arguments: &AnnotationArgumentList<'source>,
) -> Doc<'source> {
    let AnnotationArgumentListItems {
        items,
        has_recovered_tokens,
    } = annotation_argument_list_items(arguments);

    if has_recovered_tokens || annotation_argument_list_has_internal_comments(arguments) {
        return parenthesized_list(
            arguments.open_paren().as_ref(),
            arguments.close_paren().as_ref(),
            items,
        );
    }

    compact_parenthesized_list(
        arguments.open_paren().as_ref(),
        arguments.close_paren().as_ref(),
        items,
    )
}

struct AnnotationArgumentListItems<'source> {
    items: Vec<CommaListItem<'source>>,
    has_recovered_tokens: bool,
}

fn annotation_argument_list_items<'source>(
    arguments: &AnnotationArgumentList<'source>,
) -> AnnotationArgumentListItems<'source> {
    let source_start = arguments.text_range().start().get();
    let source = arguments.source_text();
    let tokens = arguments.token_iter().collect::<Vec<_>>();
    let mut token_cursor = 0;
    let mut covered_until = arguments.open_paren().map_or_else(
        || arguments.text_range().start().get(),
        |open| open.token_text_range().end().get(),
    );
    let mut items = Vec::new();
    let mut has_recovered_tokens = false;

    for entry in arguments.entries() {
        has_recovered_tokens |= push_recovered_annotation_argument_gap(
            &mut items,
            source,
            source_start,
            &tokens,
            &mut token_cursor,
            covered_until,
            entry.argument.text_range().start().get(),
        );
        items.push(CommaListItem {
            doc: format_value_argument(&entry.argument),
            comma: entry.comma,
        });
        covered_until = entry.comma.map_or_else(
            || entry.argument.text_range().end().get(),
            |comma| comma.token_text_range().end().get(),
        );
    }

    let list_end = arguments.close_paren().map_or_else(
        || arguments.text_range().end().get(),
        |close| close.token_text_range().start().get(),
    );
    has_recovered_tokens |= push_recovered_annotation_argument_gap(
        &mut items,
        source,
        source_start,
        &tokens,
        &mut token_cursor,
        covered_until,
        list_end,
    );

    AnnotationArgumentListItems {
        items,
        has_recovered_tokens,
    }
}

fn push_recovered_annotation_argument_gap<'source>(
    items: &mut Vec<CommaListItem<'source>>,
    source: &'source str,
    source_start: usize,
    tokens: &[KotlinSyntaxToken<'source>],
    token_cursor: &mut usize,
    start: usize,
    end: usize,
) -> bool {
    if source_gap_is_trivia(source, source_start, tokens.iter().copied(), start, end) {
        return false;
    }

    let mut gap_tokens = Vec::new();
    while *token_cursor < tokens.len() {
        let range = tokens[*token_cursor].token_text_range();
        if range.end().get() <= start {
            *token_cursor += 1;
            continue;
        }
        if range.start().get() >= end {
            break;
        }
        if range.start().get() >= start && range.end().get() <= end {
            gap_tokens.push(tokens[*token_cursor]);
            *token_cursor += 1;
            continue;
        }
        break;
    }

    if gap_tokens.is_empty() {
        return false;
    }

    items.push(CommaListItem {
        doc: format_token_sequence(gap_tokens, LeadingTrivia::Preserve),
        comma: None,
    });
    true
}

fn annotation_argument_list_has_internal_comments(arguments: &AnnotationArgumentList<'_>) -> bool {
    let close = arguments.close_paren();
    let close_range = close
        .as_ref()
        .map(jolt_kotlin_syntax::KotlinSyntaxToken::token_text_range);

    arguments.token_iter().any(|token| {
        if Some(token.token_text_range()) == close_range {
            return !token.leading_comments().is_empty();
        }
        token_has_comments(&token)
    })
}
