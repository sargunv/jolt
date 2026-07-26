//! Language-agnostic comment text helpers shared by Java and Kotlin formatters.
//!
//! Comment *policy* (which kinds get star-block treatment, trailing flatten,
//! token trivia placement) stays in each language crate. These helpers only
//! normalize comment *text* and emit line/star-block Doc fragments.

use std::borrow::Cow;

use crate::{ConcatBuilder, Doc, DocBuilder};

/// Splits comment text on `\n` and bare `\r`, preserving empty lines.
fn universal_comment_lines(comment: &str) -> impl Iterator<Item = &str> {
    comment
        .split('\n')
        .flat_map(|line| line.strip_suffix('\r').unwrap_or(line).split('\r'))
}

/// Trims each line of a line comment (or already-delimited body).
pub fn preserved_comment_lines(comment: &str) -> impl Iterator<Item = &str> {
    comment.trim().lines().map(str::trim)
}

/// Trims each line of a block comment including delimiters in the source text.
pub fn preserved_block_comment_lines(comment: &str) -> impl Iterator<Item = &str> {
    universal_comment_lines(comment.trim()).map(str::trim)
}

/// Strips a leading `/**` or `/*` and a trailing `*/` from a block comment.
#[must_use]
fn strip_block_comment_delimiters(comment: &str) -> &str {
    let trimmed = comment.trim();
    let body = trimmed.strip_suffix("*/").unwrap_or(trimmed);
    body.strip_prefix("/**")
        .or_else(|| body.strip_prefix("/*"))
        .unwrap_or(body)
}

/// True when a block comment is a single line with empty body (e.g. `/**/`).
#[must_use]
pub fn is_empty_single_line_block_comment(comment: &str) -> bool {
    !comment.contains(['\n', '\r']) && strip_block_comment_delimiters(comment).trim().is_empty()
}

/// Strips a leading `*` from a star-block body line after left-trim.
#[must_use]
fn normalize_star_block_body_line(line: &str) -> &str {
    line.trim_start()
        .strip_prefix('*')
        .map_or_else(|| line.trim(), str::trim_start)
}

/// True when a body line carries the aligning `*` of a star block.
#[must_use]
fn starts_with_star(line: &str) -> bool {
    line.trim_start().starts_with('*')
}

/// True when a block comment uses star-aligned layout, or its body is an empty
/// multiline block (Javadoc-style layout).
///
/// Text trailing the opening delimiter shares that delimiter's line, so it can
/// never carry the aligning `*`. The first non-empty continuation line decides
/// the layout too, which is what makes `/* opener` + ` * body` a star block.
#[must_use]
pub fn is_star_block_comment(comment: &str) -> bool {
    let content = strip_block_comment_delimiters(comment);
    let mut lines = universal_comment_lines(content);
    let opener_line = lines.next().unwrap_or("");
    let continuation = lines.find(|line| !line.trim().is_empty());
    starts_with_star(opener_line)
        || continuation.is_some_and(starts_with_star)
        || (content.contains(['\n', '\r']) && content.trim().is_empty())
}

/// Emits trimmed comment lines separated by hard lines.
pub fn format_comment_lines<'source>(
    doc: &mut DocBuilder<'source>,
    lines: impl IntoIterator<Item = impl Into<Cow<'source, str>>>,
) -> Doc<'source> {
    doc.concat_list(|docs| {
        for line in lines {
            if !docs.is_empty() {
                let hard_line = docs.hard_line();
                docs.push(hard_line);
            }
            let line = docs.text(line);
            docs.push(line);
        }
    })
}

/// Whether text trailing the opening delimiter keeps its own line.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StarBlockOpener {
    /// `/** text` — doc comments reflow that text onto its own star line.
    Reflow,
    /// `/* text` — plain star blocks keep it on the opening line.
    Keep,
}

/// Formats a Javadoc-style `/** … */` or `/* … */` star-block body.
pub fn format_star_block_comment<'source>(
    doc: &mut DocBuilder<'source>,
    comment: &'source str,
    opening_delimiter: &'static str,
    opener: StarBlockOpener,
) -> Doc<'source> {
    let content = strip_block_comment_delimiters(comment);
    let multiline = content.contains(['\n', '\r']);
    doc.concat_list(|docs| {
        let open = docs.literal_text(opening_delimiter);
        docs.push(open);

        let mut has_content = false;
        let mut pending_blank_lines = 0;
        let mut lines = universal_comment_lines(content);
        if opener == StarBlockOpener::Keep && multiline {
            // The opener line shares the delimiter's line, so it holds no
            // aligning `*` to strip: only trim it, or a body that starts with a
            // literal `*` would lose that character.
            let opener_text = lines.next().unwrap_or("").trim();
            if !opener_text.is_empty() {
                has_content = true;
                let space = docs.literal_text(" ");
                let text = docs.text(opener_text);
                let opener_line = docs.concat([space, text]);
                docs.push(opener_line);
            }
        }
        for line in lines.map(|line| {
            if multiline {
                normalize_star_block_body_line(line)
            } else {
                line.trim()
            }
        }) {
            if line.is_empty() {
                if has_content {
                    pending_blank_lines += 1;
                }
                continue;
            }

            has_content = true;
            for _ in 0..pending_blank_lines {
                let blank = docs.literal_text(" *");
                push_comment_line(docs, blank);
            }
            pending_blank_lines = 0;
            let prefix = docs.literal_text(" * ");
            let line = docs.text(line);
            let line = docs.concat([prefix, line]);
            push_comment_line(docs, line);
        }

        let close = docs.literal_text(" */");
        push_comment_line(docs, close);
    })
}

fn push_comment_line<'source>(docs: &mut ConcatBuilder<'_, 'source>, line: Doc<'source>) {
    if !docs.is_empty() {
        let hard_line = docs.hard_line();
        docs.push(hard_line);
    }
    docs.push(line);
}
