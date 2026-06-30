use jolt_fmt_ir::{
    Doc, concat, fill, fill_entry, group, hard_line, indent_by, line, literal_text, text,
};

use crate::policy::JavaFormatPolicy;

pub(crate) fn string_literal_reflow(
    token_text: &str,
    start_column: usize,
    trailing_width: usize,
    policy: JavaFormatPolicy,
) -> Option<Doc> {
    if !policy.reflows_string_literals()
        || !token_text.is_ascii()
        || token_text.contains('\\')
        || token_text.contains(['\n', '\r'])
    {
        return None;
    }

    let body = token_text.strip_prefix('"')?.strip_suffix('"')?;
    if body.is_empty()
        || start_column + token_text.len() + trailing_width <= policy.max_line_length()
    {
        return None;
    }

    let pieces = split_string_literal_body(body, start_column, trailing_width, policy)?;
    if pieces.len() < 2 {
        return None;
    }

    let docs = pieces
        .into_iter()
        .enumerate()
        .map(|(index, piece)| {
            let literal = text(format!("\"{piece}\""));
            if index == 0 {
                literal
            } else {
                concat([text("+ "), literal])
            }
        })
        .collect::<Vec<_>>();
    let last = docs.last().expect("length checked above").clone();
    let entries = docs
        .iter()
        .take(docs.len() - 1)
        .cloned()
        .map(|doc| fill_entry(doc, line()));

    Some(group(indent_by(
        policy.continuation_indent_levels(),
        fill(entries, last),
    )))
}

fn split_string_literal_body(
    body: &str,
    start_column: usize,
    trailing_width: usize,
    policy: JavaFormatPolicy,
) -> Option<Vec<&str>> {
    let max_line_length = policy.max_line_length();
    let first_literal_width = max_line_length.checked_sub(start_column)?;
    let continuation_literal_width = max_line_length.checked_sub(start_column + 6)?;
    let mut pieces = Vec::new();
    let mut start = 0;
    let mut first_line = true;

    while start < body.len() {
        let base_width = if first_line {
            first_literal_width
        } else {
            continuation_literal_width
        };
        let Some(end) = next_string_piece_end(body, start, base_width, trailing_width) else {
            return None;
        };
        pieces.push(&body[start..end]);
        start = end;
        first_line = false;
    }

    Some(pieces)
}

fn next_string_piece_end(
    body: &str,
    start: usize,
    base_literal_width: usize,
    trailing_width: usize,
) -> Option<usize> {
    let mut best = None;
    for end in string_piece_boundaries(body, start) {
        let is_final = end == body.len();
        let literal_width = 2 + end - start;
        let line_width = literal_width + usize::from(is_final) * trailing_width;
        if line_width <= base_literal_width {
            best = Some(end);
        } else {
            break;
        }
    }
    best
}

fn string_piece_boundaries(body: &str, start: usize) -> impl Iterator<Item = usize> + '_ {
    body[start..]
        .match_indices(' ')
        .filter_map(move |(index, _)| {
            let boundary = start + index;
            (boundary > start).then_some(boundary)
        })
        .chain(std::iter::once(body.len()))
}

pub(crate) struct TextBlockLiteral {
    doc: Doc,
    opening_indent: TextBlockOpeningIndent,
}

impl TextBlockLiteral {
    pub(crate) fn into_doc(self) -> Doc {
        self.doc
    }

    pub(crate) const fn opening_indent(&self) -> TextBlockOpeningIndent {
        self.opening_indent
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TextBlockOpeningIndent {
    Continuation,
    Absolute,
}

pub(crate) fn text_block_literal(token_text: &str) -> TextBlockLiteral {
    let Some(parts) = TextBlockParts::parse(token_text) else {
        return TextBlockLiteral {
            doc: literal_text(token_text),
            opening_indent: TextBlockOpeningIndent::Continuation,
        };
    };

    let mut lines = content_lines(parts.content);
    if !parts.closing_on_own_line
        && let Some(last_line) = lines.last_mut()
    {
        if last_line.ends_with('\\') {
            return TextBlockLiteral {
                doc: literal_text(token_text),
                opening_indent: TextBlockOpeningIndent::Continuation,
            };
        }
        last_line.push('\\');
    }

    let incidental_indent = incidental_indent(&lines, parts.closing_indent);
    if incidental_indent == 0 {
        return TextBlockLiteral {
            doc: literal_text(rewrite_absolute_text_block(parts, &lines)),
            opening_indent: TextBlockOpeningIndent::Absolute,
        };
    }

    let mut docs = Vec::new();
    docs.push(text("\"\"\""));
    for line in strip_line_indent(lines, incidental_indent) {
        if line.is_empty() {
            docs.push(literal_text("\n"));
        } else {
            docs.push(hard_line());
            docs.push(text(line));
        }
    }
    docs.push(hard_line());
    docs.push(text("\"\"\""));
    TextBlockLiteral {
        doc: concat(docs),
        opening_indent: TextBlockOpeningIndent::Continuation,
    }
}

pub(crate) fn text_block_opening_indent(token_text: &str) -> TextBlockOpeningIndent {
    text_block_literal(token_text).opening_indent()
}

struct TextBlockParts<'a> {
    content: &'a str,
    closing_on_own_line: bool,
    closing_indent: Option<usize>,
}

impl<'a> TextBlockParts<'a> {
    fn parse(token_text: &'a str) -> Option<Self> {
        let after_opening = token_text.strip_prefix("\"\"\"")?;
        let opening_terminator = find_line_terminator(after_opening)?;
        let content_start = 3 + opening_terminator.1;
        let closing_start = token_text.rfind("\"\"\"")?;
        if closing_start < content_start {
            return None;
        }

        let closing_line_start = token_text[..closing_start]
            .rfind(['\n', '\r'])
            .map_or(content_start, |index| index + 1);
        let closing_on_own_line = token_text[closing_line_start..closing_start]
            .chars()
            .all(is_text_block_whitespace);
        let closing_indent = closing_on_own_line
            .then(|| leading_ascii_whitespace(&token_text[closing_line_start..closing_start]));
        let content_end = if closing_on_own_line {
            closing_line_start
        } else {
            closing_start
        };

        Some(Self {
            content: &token_text[content_start..content_end],
            closing_on_own_line,
            closing_indent,
        })
    }
}

fn content_lines(content: &str) -> Vec<String> {
    let mut lines = Vec::new();
    let mut start = 0;
    while start < content.len() {
        if let Some((terminator_start, terminator_end)) = find_line_terminator(&content[start..]) {
            let end = start + terminator_start;
            lines.push(content[start..end].to_owned());
            start += terminator_end;
        } else {
            lines.push(content[start..].to_owned());
            start = content.len();
        }
    }
    lines
}

fn find_line_terminator(text: &str) -> Option<(usize, usize)> {
    let bytes = text.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'\n' => return Some((index, index + 1)),
            b'\r' if bytes.get(index + 1) == Some(&b'\n') => {
                return Some((index, index + 2));
            }
            b'\r' => return Some((index, index + 1)),
            _ => index += 1,
        }
    }
    None
}

fn incidental_indent(lines: &[String], closing_indent: Option<usize>) -> usize {
    lines
        .iter()
        .filter(|line| !line.trim().is_empty())
        .map(|line| leading_ascii_whitespace(line))
        .chain(closing_indent)
        .min()
        .unwrap_or_default()
}

fn strip_line_indent(lines: Vec<String>, indent: usize) -> Vec<String> {
    lines
        .into_iter()
        .map(|line| {
            if line.trim().is_empty() {
                String::new()
            } else {
                line.get(indent..).unwrap_or_default().to_owned()
            }
        })
        .collect()
}

fn rewrite_absolute_text_block(parts: TextBlockParts<'_>, lines: &[String]) -> String {
    if parts.closing_on_own_line {
        return concat_absolute_text_block(parts.content, "\"\"\"");
    }

    let mut content = lines.join("\n");
    content.push('\n');
    concat_absolute_text_block(&content, "\"\"\"")
}

fn concat_absolute_text_block(content: &str, closing: &str) -> String {
    let mut text = String::from("\"\"\"\n");
    text.push_str(content);
    text.push_str(closing);
    text
}

fn leading_ascii_whitespace(line: &str) -> usize {
    line.as_bytes()
        .iter()
        .take_while(|byte| matches!(byte, b' ' | b'\t' | b'\x0c'))
        .count()
}

fn is_text_block_whitespace(ch: char) -> bool {
    matches!(ch, ' ' | '\t' | '\u{000C}')
}
