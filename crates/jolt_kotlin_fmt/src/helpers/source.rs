use jolt_kotlin_syntax::KotlinSyntaxToken;

pub(crate) fn source_gap_is_trivia<'source>(
    source: &'source str,
    source_start: usize,
    tokens: impl IntoIterator<Item = KotlinSyntaxToken<'source>>,
    start: usize,
    end: usize,
) -> bool {
    let mut comment_ranges = tokens
        .into_iter()
        .flat_map(|token| token.leading_comments().chain(token.trailing_comments()))
        .filter_map(|comment| {
            let range = comment.text_range();
            let comment_start = range.start().get();
            let comment_end = range.end().get();
            (comment_start >= start && comment_end <= end).then_some((comment_start, comment_end))
        })
        .collect::<Vec<_>>();
    comment_ranges.sort_unstable();

    let mut cursor = start;
    for (comment_start, comment_end) in comment_ranges {
        if comment_start < cursor {
            if comment_end > cursor {
                cursor = comment_end;
            }
            continue;
        }
        if !source_slice_is_whitespace(source, source_start, cursor, comment_start) {
            return false;
        }
        cursor = comment_end;
    }

    source_slice_is_whitespace(source, source_start, cursor, end)
}

fn source_slice_is_whitespace(source: &str, source_start: usize, start: usize, end: usize) -> bool {
    let Some(slice_start) = start.checked_sub(source_start) else {
        return false;
    };
    let Some(slice_end) = end.checked_sub(source_start) else {
        return false;
    };
    let Some(slice) = source.get(slice_start..slice_end) else {
        return false;
    };

    slice.chars().all(char::is_whitespace)
}
