use jolt_fmt_ir::{Doc, concat, hard_line, literal_text, text};

pub(crate) fn text_block_literal(token_text: &str) -> Doc {
    let Some(parts) = TextBlockParts::parse(token_text) else {
        return literal_text(token_text);
    };

    let mut lines = content_lines(parts.content);
    if !parts.closing_on_own_line
        && let Some(last_line) = lines.last_mut()
    {
        if last_line.ends_with('\\') {
            return literal_text(token_text);
        }
        last_line.push('\\');
    }

    let incidental_indent = incidental_indent(&lines, parts.closing_indent);
    if incidental_indent == 0 {
        return literal_text(rewrite_absolute_text_block(parts, &lines));
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
    concat(docs)
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
