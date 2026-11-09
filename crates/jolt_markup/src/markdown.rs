//! Markdown doc comments (`///`, JEP 467): the tokenizer and the layout.
//!
//! The tokenizer decides whether the run is in the subset that Jolt lays
//! out: paragraphs, blank lines, block tags (`@param` at a line start, whose
//! text continues on lines flush with the frame), and fenced code blocks
//! whose closing line is only its fence. Anything else keeps the whole run
//! as written (`None`): any other block (a line that starts with a word that
//! could start one, a line indented by four or more columns that doesn't
//! continue a paragraph, a table delimiter row, a link reference
//! definition), a fence that isn't closed
//! or that holds a line starting with `@`, `{@snippet`, an inline tag that
//! doesn't close in its paragraph or holds a line starting with `@`, a code
//! span, link or HTML tag that doesn't close on its line, and a hard break
//! (a line that ends in two spaces or `\`, before a non-blank line).
//!
//! The layout fills the paragraphs and prints every other line as written.
//! Blank lines are kept as written; one is added only before the first block
//! tag.

use crate::frame::Line;
use crate::layout::{Join, Layout};
use crate::tags::{self, Arg, InlineTags, html_tag_len, is_space};
use crate::token::{Kind, Sep, Token};

/// The tokens of a `///` run's content lines, or `None` when the run is
/// kept as written.
pub(crate) fn tokenize<'a>(lines: &[Line<'a>]) -> Option<Vec<Token<'a>>> {
    for (index, line) in lines.iter().enumerate() {
        if line.text.contains("{@snippet") || hard_break(lines, index) {
            return None;
        }
    }
    let mut tokens = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        let line = lines[index];
        if line.text.is_empty() {
            tokens.push(Token::new(Kind::Blank, line.raw, Sep::Line));
            index += 1;
            continue;
        }
        if columns(line.text) >= 4 {
            return None;
        }
        if let Some((marker, len)) = opens_fence(line.text.trim_start().as_bytes()) {
            tokens.push(Token::new(Kind::Verbatim, line.raw, Sep::Line));
            index = fence(lines, index + 1, marker, len, &mut tokens)?;
            continue;
        }
        let end = (index..lines.len())
            .find(|&at| {
                let text = lines[at].text;
                text.is_empty() || opens_fence(text.trim_start().as_bytes()).is_some()
            })
            .unwrap_or(lines.len());
        Paragraph::new(&lines[index..end]).run(&mut tokens)?;
        index = end;
    }
    Some(tokens)
}

/// Whether the description of the block tag on a line whose text is
/// `trimmed` starts with a word that starts a block: javadoc reads a block
/// tag's description as Markdown of its own, so `@return # Title` is a
/// heading. The description follows the tag, and the name for tags that
/// have one.
fn description_starts_structure(trimmed: &str) -> bool {
    description(trimmed).is_some_and(|word| tags::starts_structure(word, true))
}

/// The first word of the description of the block tag on a line whose text
/// is `trimmed`, if the line has one.
fn description(trimmed: &str) -> Option<&str> {
    let mut words = trimmed
        .split([' ', '\t', '\x0c'])
        .filter(|word| !word.is_empty());
    let named = words
        .next()
        .is_some_and(|tag| matches!(tag, "@param" | "@throws" | "@exception"));
    if named {
        words.next();
    }
    words.next()
}

/// Whether line `index` ends in a hard break: it has text that ends in two
/// spaces or `\`, and the next line has text. (Inside a paragraph that is a
/// hard break; elsewhere it may not be, which only makes the rule coarser.)
fn hard_break(lines: &[Line<'_>], index: usize) -> bool {
    let line = lines[index];
    !line.text.is_empty()
        && (line.hard_break || line.text.ends_with('\\'))
        && lines
            .get(index + 1)
            .is_some_and(|next| !next.text.is_empty())
}

/// Adds the lines of a fenced code block from line `index` to its closing
/// line; where to go on after it.
fn fence<'a>(
    lines: &[Line<'a>],
    mut index: usize,
    marker: u8,
    len: usize,
    tokens: &mut Vec<Token<'a>>,
) -> Option<usize> {
    loop {
        let line = lines.get(index)?;
        tokens.push(Token::new(Kind::Verbatim, line.raw, Sep::Line));
        index += 1;
        let trimmed = line.text.trim_start();
        if tags::is_block_tag(trimmed) {
            return None;
        }
        // javadoc closes a fence at a line that starts with the fence; so
        // does CommonMark when nothing follows it and it is indented by
        // less than four columns.
        let run = trimmed.bytes().take_while(|&byte| byte == marker).count();
        if run >= len {
            return (run == trimmed.len() && columns(line.text) < 4).then_some(index);
        }
    }
}

/// One paragraph's lines: up to a blank line or a fence.
struct Paragraph<'p, 'a> {
    lines: &'p [Line<'a>],
    tags: InlineTags,
    /// The end of the inline tag being read, or 0.
    tag_end: usize,
    inline_arg: Arg,
    block_arg: Arg,
    /// The last word ends in `\`.
    escape: bool,
}

impl<'p, 'a> Paragraph<'p, 'a> {
    fn new(lines: &'p [Line<'a>]) -> Self {
        Self {
            lines,
            tags: InlineTags::new(lines),
            tag_end: 0,
            inline_arg: Arg::None,
            block_arg: Arg::None,
            escape: false,
        }
    }

    fn run(mut self, tokens: &mut Vec<Token<'a>>) -> Option<()> {
        // After a block tag's line with no description on it: an indented
        // line there starts code in javadoc's reading.
        let mut awaiting_description = false;
        for &line in self.lines {
            let text = line.text;
            let trimmed = text.trim_start_matches([' ', '\t']);
            let first = trimmed.split([' ', '\t', '\x0c']).next();
            if is_delimiter_row(trimmed.as_bytes())
                || (trimmed.starts_with('[') && trimmed.contains("]:"))
            {
                return None;
            }
            if tags::is_block_tag(trimmed) {
                if line.offset < self.tag_end || description_starts_structure(trimmed) {
                    return None;
                }
                tokens.push(Token::new(Kind::BlockTag, "", Sep::Line));
                self.block_arg = Arg::Name;
                awaiting_description = description(trimmed).is_none();
            } else if first.is_some_and(|word| tags::starts_structure(word, true))
                || (std::mem::take(&mut awaiting_description)
                    && line.offset >= self.tag_end
                    && columns(text) >= 4)
            {
                return None;
            }
            let bytes = text.as_bytes();
            let mut pos = text.len() - trimmed.len();
            let mut sep = Sep::Line;
            while pos < bytes.len() {
                let in_tag = line.offset + pos < self.tag_end;
                let end = self.word_end(line, pos)?;
                let word = &text[pos..end];
                let glue = if in_tag {
                    self.inline_arg.glue(word)
                } else {
                    self.block_arg.glue(word)
                };
                tokens.push(Token {
                    glue,
                    sticky: tags::starts_structure(word, true) || self.escape,
                    ..Token::new(Kind::Word, word, sep)
                });
                self.escape = word.ends_with('\\');
                if self.tag_end != 0 && line.offset + end >= self.tag_end {
                    self.tag_end = 0;
                    self.inline_arg = Arg::None;
                }
                pos = end;
                while pos < bytes.len() && is_space(bytes[pos]) {
                    pos += 1;
                }
                sep = Sep::Space;
            }
        }
        (self.tag_end == 0).then_some(())
    }

    /// The end of the word at `pos` on `line`: at whitespace outside atoms
    /// (code spans, links, HTML tags), which must close on the line.
    fn word_end(&mut self, line: Line<'a>, pos: usize) -> Option<usize> {
        let text = line.text;
        let bytes = text.as_bytes();
        let mut at = pos;
        while at < bytes.len() {
            match bytes[at] {
                byte if is_space(byte) => break,
                b'\\' => {
                    at += 1 + usize::from(bytes.get(at + 1).is_some_and(u8::is_ascii_punctuation));
                }
                b'`' => at = code_span_end(bytes, at)?,
                b'[' => at = link_end(bytes, at)?,
                b'<' => match bytes.get(at + 1) {
                    Some(next) if next.is_ascii_alphabetic() || *next == b'/' => {
                        at += html_tag_len(&text[at..])?;
                    }
                    Some(b'!' | b'?') => return None,
                    _ => at += 1,
                },
                b'{' if bytes.get(at + 1) == Some(&b'@') && line.offset + at >= self.tag_end => {
                    self.tag_end = self.tags.end(line.offset + at)?;
                    if self.tags.keeps_whitespace(line.offset + at) {
                        return None;
                    }
                    self.inline_arg = Arg::inline(tags::inline_tag_name(&text[at..]));
                    at += 2;
                }
                _ => at += 1,
            }
        }
        Some(at)
    }
}

/// The end of the code span whose backtick run starts at `at`: after the
/// next run of the same length on the line.
fn code_span_end(bytes: &[u8], at: usize) -> Option<usize> {
    let run = |from: usize| {
        bytes[from..]
            .iter()
            .take_while(|&&byte| byte == b'`')
            .count()
    };
    let len = run(at);
    let mut next = at + len;
    while next < bytes.len() {
        if bytes[next] == b'`' {
            let found = run(next);
            if found == len {
                return Some(next + len);
            }
            next += found;
        } else {
            next += 1;
        }
    }
    None
}

/// The end of the link whose `[` is at `at`: its text up to the `]` that
/// balances it, and a destination `(…)` or a reference `[…]` right after,
/// all on the line. Code spans and escapes inside count as text.
fn link_end(bytes: &[u8], at: usize) -> Option<usize> {
    let close = |open: u8, close: u8, mut at: usize| {
        let mut depth = 0usize;
        while let Some(&byte) = bytes.get(at) {
            match byte {
                b'\\' => at += 1,
                b'`' => at = code_span_end(bytes, at)? - 1,
                b'"' if open == b'(' => {
                    at += 1 + bytes[at + 1..].iter().position(|&next| next == b'"')?;
                }
                byte if byte == open => depth += 1,
                byte if byte == close => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(at + 1);
                    }
                }
                _ => {}
            }
            at += 1;
        }
        None
    };
    let end = close(b'[', b']', at)?;
    match bytes.get(end) {
        Some(b'(') => close(b'(', b')', end),
        Some(b'[') => close(b'[', b']', end),
        _ => Some(end),
    }
}

/// The columns of the whitespace that starts `text` (a tab advances to the
/// next multiple of 4).
fn columns(text: &str) -> usize {
    text.bytes()
        .take_while(|&byte| matches!(byte, b' ' | b'\t'))
        .fold(0, |col, byte| {
            if byte == b'\t' {
                col + 4 - col % 4
            } else {
                col + 1
            }
        })
}

/// The fence that `rest` opens: three or more `` ` `` (with no `` ` `` in
/// the info string after them) or `~`.
fn opens_fence(rest: &[u8]) -> Option<(u8, usize)> {
    let &marker @ (b'`' | b'~') = rest.first()? else {
        return None;
    };
    let len = rest.iter().take_while(|&&byte| byte == marker).count();
    (len >= 3 && !(marker == b'`' && rest[len..].contains(&b'`'))).then_some((marker, len))
}

/// Whether `rest` may be a table's delimiter row (`|---|:--:|`): only `|`,
/// `:`, `-` and whitespace, with a `-` and a `|` or `:`.
fn is_delimiter_row(rest: &[u8]) -> bool {
    rest.iter()
        .all(|&byte| matches!(byte, b'|' | b':' | b'-' | b' ' | b'\t'))
        && rest.contains(&b'-')
        && (rest.contains(&b'|') || rest.contains(&b':'))
}

/// Lays out a `///` run's tokens.
pub(crate) fn lay_out<'a>(tokens: &[Token<'a>], layout: &mut Layout<'_, 'a>) {
    // A blank line goes before the next line (the first block tag's).
    let mut blank = false;
    // The last line printed is blank.
    let mut last_blank = false;
    let mut tagged = false;
    let mut in_paragraph = false;
    for token in tokens {
        match token.kind {
            Kind::BlockTag => {
                if !std::mem::replace(&mut tagged, true) {
                    blank = !layout.at_start() && !last_blank;
                }
                in_paragraph = false;
            }
            Kind::Word => {
                if !std::mem::replace(&mut in_paragraph, true) {
                    last_blank = false;
                    layout.line(std::mem::take(&mut blank), 0, 0);
                }
                let join = if token.glue { Join::Space } else { Join::Wrap };
                layout.word(token.text, join, token.sticky);
            }
            _ => {
                if std::mem::take(&mut blank) {
                    layout.line(true, 0, 0);
                }
                layout.verbatim(token.text);
                last_blank = token.kind == Kind::Blank;
                in_paragraph = false;
            }
        }
    }
}
