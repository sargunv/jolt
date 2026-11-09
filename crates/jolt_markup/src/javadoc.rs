//! The tokenizer of Javadoc comments (`/**`).
//!
//! It reads the comment once and decides whether it is in the subset that
//! Jolt lays out. The subset is:
//!
//! - words and entities;
//! - inline tags (`{@…}`, brace-balanced, which may span lines), read
//!   first: nothing inside one is a block tag, an HTML tag or a region;
//! - block tags (`@param` at a line start);
//! - HTML tags that open and close on one line, with their quoted attribute
//!   values closed on that line: an atom, its inner whitespace kept;
//! - the block elements `<p>`, `<ul>`, `<ol>`, `<li>`, `<br>` and `<h1>` to
//!   `<h6>`, and `<pre>` regions closed by `</pre>`, kept as written.
//!
//! Anything else makes the whole comment fall back to N9 (`None`):
//! `<!--`, `{@snippet`, `<table>`, `<blockquote>`, `<dl>`, `<dt>`, `<dd>`, a
//! `<` and a letter or `/` without a `>` on its line, an unclosed `<pre>`, a
//! line without a leading `*`, and a paragraph whose lines are indented
//! differently (see [`Unit`]). A `<` followed by anything else is text.

use crate::frame::{self, Line};
use crate::tags::{self, Arg, InlineTags, html_tag_len, is_space};
use crate::token::{Kind, Name, Sep, Token};

/// The tokens of a `/**` comment, or `None` when it falls back to N9.
pub(crate) fn tokenize(text: &str) -> Option<Vec<Token<'_>>> {
    // Java decodes Unicode escapes (`\u0040return`) before javadoc reads the
    // comment, so the source text can hide its structure.
    if text.contains("<!--") || text.contains("{@snippet") || text.contains("\\u") {
        return None;
    }
    let lines = frame::javadoc(text)?;
    let mut tokenizer = Tokenizer {
        lines: &lines,
        tags: InlineTags::new(&lines),
        // About one token per word.
        tokens: Vec::with_capacity(text.len() / 6),
        tag_end: 0,
        inline_arg: Arg::None,
        block_arg: Arg::None,
        unit: None,
        unit_start: true,
    };
    tokenizer.run()?;
    Some(tokenizer.tokens)
}

/// What the block element whose tag starts `text` (at its `<`) is: one the
/// layout places (`Some(name)`), or one outside the subset (`None`), and
/// whether the tag closes it. `None` for any other tag.
fn block_element(text: &str) -> Option<(Option<Name>, bool)> {
    let after = text.strip_prefix('<')?;
    let closing = after.starts_with('/');
    let name = &after[usize::from(closing)..];
    let len = name.bytes().take_while(u8::is_ascii_alphanumeric).count();
    if !name[len..]
        .bytes()
        .next()
        .is_none_or(|byte| byte == b'>' || byte == b'/' || byte.is_ascii_whitespace())
    {
        return None;
    }
    let name = &name[..len];
    let is = |candidate: &str| name.eq_ignore_ascii_case(candidate);
    let laid_out = if is("p") {
        Name::P
    } else if is("ul") || is("ol") {
        Name::List
    } else if is("li") {
        Name::Item
    } else if is("pre") {
        Name::Pre
    } else if is("br") {
        Name::Br
    } else if len == 2
        && name.as_bytes()[0].eq_ignore_ascii_case(&b'h')
        && matches!(name.as_bytes()[1], b'1'..=b'6')
    {
        Name::Heading
    } else if ["dl", "dt", "dd", "table", "blockquote"]
        .iter()
        .any(|candidate| is(candidate))
    {
        return Some((None, closing));
    } else {
        return None;
    };
    Some((Some(laid_out), closing))
}

/// A unit of lines whose indentation is checked: a run of non-blank lines
/// from a blank line, a block tag or a `<p>` up to the next of those or a
/// line that starts with another block element. A unit of plain text (no
/// inline tag, no block element after its leading `<p>`) whose lines are
/// indented differently, by two columns or more, is ASCII art, a grammar or
/// aligned text: the comment falls back. Its first line doesn't count when
/// it starts with a block tag or `<p>` or starts the comment.
#[derive(Clone, Copy, Debug)]
struct Unit {
    least: usize,
    most: usize,
    lines: usize,
    plain: bool,
}

struct Tokenizer<'t, 'a> {
    lines: &'t [Line<'a>],
    tags: InlineTags,
    tokens: Vec<Token<'a>>,
    /// The end of the inline tag being read, or 0.
    tag_end: usize,
    inline_arg: Arg,
    block_arg: Arg,
    unit: Option<Unit>,
    /// The next line starts a unit: after a blank line or a region.
    unit_start: bool,
}

impl<'a> Tokenizer<'_, 'a> {
    fn run(&mut self) -> Option<()> {
        let (mut index, mut pos) = (0, 0);
        while index < self.lines.len() {
            let line = self.lines[index];
            if pos == 0 && self.tag_end <= line.offset && !self.line_start(index)? {
                index += 1;
                continue;
            }
            (index, pos) = self.lex(index, pos)?;
        }
        self.close_unit()
    }

    fn push(&mut self, kind: Kind, text: &'a str, sep: Sep) {
        self.tokens.push(Token::new(kind, text, sep));
    }

    fn word(&mut self, text: &'a str, sep: Sep, glue: bool) {
        self.tokens.push(Token {
            glue,
            sticky: tags::starts_structure(text, false),
            ..Token::new(Kind::Word, text, sep)
        });
    }

    /// Reads the start of line `index`, outside inline tags: blank lines,
    /// block tags, list-like lines and rulers, and the unit checks. Returns
    /// whether the line's text is still to be read.
    fn line_start(&mut self, index: usize) -> Option<bool> {
        let text = self.lines[index].text;
        if text.is_empty() {
            self.close_unit()?;
            self.unit_start = true;
            self.push(Kind::Blank, "", Sep::Line);
            return Some(false);
        }
        let rest = text.trim_start_matches([' ', '\t']);
        let indent = text.len() - rest.len();
        let tag = tags::is_block_tag(text);
        let element = block_element(rest);
        let paragraph = element == Some((Some(Name::P), false));
        let lead_text = !tag && !paragraph;
        if self.unit.is_some() && (element.is_some() || tag) {
            self.close_unit()?;
        }
        if self.unit_start || !lead_text {
            self.unit = (!(element.is_some() && lead_text)).then_some(Unit {
                least: usize::MAX,
                most: 0,
                lines: 0,
                plain: true,
            });
        }
        self.unit_start = false;
        if let Some(unit) = &mut self.unit {
            if !(unit.lines == 0 && (!lead_text || index == 0)) {
                unit.least = unit.least.min(indent);
                unit.most = unit.most.max(indent);
            }
            // After the `<` of the unit's leading `<p>`.
            let from = if paragraph && unit.lines == 0 {
                indent + 1
            } else {
                0
            };
            if text.contains("{@")
                || text[from..]
                    .match_indices('<')
                    .any(|(at, _)| block_element(&text[from + at..]).is_some())
            {
                unit.plain = false;
            }
            unit.lines += 1;
        }
        if tag {
            self.block_arg = Arg::Name;
            self.push(Kind::BlockTag, "", Sep::Line);
        } else if index > 0 {
            if tags::is_ruler(text) {
                self.push(Kind::Break, "", Sep::Line);
                self.word(text, Sep::Line, false);
                self.push(Kind::Break, "", Sep::Line);
                return Some(false);
            }
            let first = rest.split(|ch: char| ch.is_ascii_whitespace()).next();
            if first.is_some_and(tags::is_list_marker) {
                self.push(Kind::Break, "", Sep::Line);
            }
        }
        Some(true)
    }

    /// Ends the open unit; `None` when it is plain text indented
    /// differently.
    fn close_unit(&mut self) -> Option<()> {
        match self.unit.take() {
            Some(unit) if unit.plain && unit.least != usize::MAX && unit.most >= unit.least + 2 => {
                None
            }
            _ => Some(()),
        }
    }

    /// Reads line `index` from byte `pos`. Returns where to go on.
    fn lex(&mut self, index: usize, mut pos: usize) -> Option<(usize, usize)> {
        let line = self.lines[index];
        let text = line.text;
        let bytes = text.as_bytes();
        let space_from = |at: usize| (at..bytes.len()).find(|&at| is_space(bytes[at]));
        let mut sep = if pos == 0 { Sep::Line } else { Sep::None };
        loop {
            let start = pos;
            while pos < bytes.len() && is_space(bytes[pos]) {
                pos += 1;
            }
            if pos > start && sep == Sep::None {
                sep = Sep::Space;
            }
            if pos >= bytes.len() {
                return Some((index + 1, 0));
            }
            let offset = line.offset + pos;
            let in_tag = offset < self.tag_end;
            let opens_tag = !in_tag && text[pos..].starts_with("{@");
            if in_tag || opens_tag {
                if let Some(end) = opens_tag.then(|| self.tags.end(offset)).flatten() {
                    if self.tags.keeps_whitespace(offset) {
                        return None;
                    }
                    self.tag_end = end;
                    self.inline_arg = Arg::inline(tags::inline_tag_name(&text[pos..]));
                }
                if offset < self.tag_end {
                    // A piece of an inline tag: up to a space or the tag's end.
                    let end = space_from(pos)
                        .unwrap_or(bytes.len())
                        .min(self.tag_end - line.offset);
                    let piece = &text[pos..end];
                    let glue = if opens_tag {
                        self.block_arg.glue(piece)
                    } else {
                        self.inline_arg.glue(piece)
                    };
                    self.word(piece, sep, glue);
                    (pos, sep) = (end, Sep::None);
                    if line.offset + pos >= self.tag_end {
                        self.tag_end = 0;
                        self.inline_arg = Arg::None;
                    }
                    continue;
                }
            }
            if let Some((name, closing)) = block_element(&text[pos..]) {
                let end = pos + html_tag_len(&text[pos..])?;
                self.block_arg = Arg::None;
                match name? {
                    Name::Pre if !closing => return self.pre(index, pos, end, sep),
                    name => self.push(Kind::Element { name, closing }, &text[pos..end], sep),
                }
                (pos, sep) = (end, Sep::None);
                continue;
            }
            let end = self.word_end(line, pos)?;
            let piece = &text[pos..end];
            let glue = self.block_arg.glue(piece);
            self.word(piece, sep, glue);
            (pos, sep) = (end, Sep::None);
        }
    }

    /// The end of the word at `pos`: at a space, or where a block element or
    /// an inline tag starts. An HTML tag is part of the word, spaces
    /// included; `None` when one doesn't close on its line.
    fn word_end(&mut self, line: Line<'_>, pos: usize) -> Option<usize> {
        let text = line.text;
        let bytes = text.as_bytes();
        let mut at = pos;
        while at < bytes.len() {
            match bytes[at] {
                byte if is_space(byte) => break,
                b'{' if at > pos
                    && bytes.get(at + 1) == Some(&b'@')
                    && self.tags.end(line.offset + at).is_some() =>
                {
                    break;
                }
                b'<' if at > pos && block_element(&text[at..]).is_some() => break,
                b'<' if bytes
                    .get(at + 1)
                    .is_some_and(|&next| next.is_ascii_alphabetic() || next == b'/') =>
                {
                    at += html_tag_len(&text[at..])?;
                }
                _ => at += 1,
            }
        }
        Some(at)
    }

    /// A `<pre>` region whose tag runs from `pos` to `end` on line `index`,
    /// up to its `</pre>`; `None` when it isn't closed, or an inline tag
    /// crosses its end.
    fn pre(&mut self, index: usize, pos: usize, end: usize, sep: Sep) -> Option<(usize, usize)> {
        let (close_line, close) = self.closing_pre(index, end)?;
        let last = self.lines[close_line];
        let close_end = close + html_tag_len(&last.text[close..])?;
        let region = (self.lines[index].offset + pos, last.offset + close_end);
        if self.tags.crosses(region.0, region.1) {
            return None;
        }
        let first = self.lines[index].text;
        if close_line == index {
            self.push(Kind::Pre, &first[pos..close_end], sep);
        } else {
            self.push(Kind::Pre, &first[pos..], sep);
            for line in &self.lines[index + 1..close_line] {
                self.push(Kind::Verbatim, line.raw, Sep::Line);
            }
            let frame = last.raw.len() - last.text.len();
            self.push(Kind::Verbatim, &last.raw[..frame + close_end], Sep::Line);
        }
        self.push(Kind::PreEnd, "", Sep::None);
        if close_end >= last.text.len() {
            self.unit_start = true;
            Some((close_line + 1, 0))
        } else {
            Some((close_line, close_end))
        }
    }

    /// The line and byte of the `</pre>` after byte `from` of line `index`,
    /// outside the other HTML tags in the region, whose quoted values may hold
    /// `</pre>`. A `<` that doesn't start a tag closed on its line, as in
    /// `Map<String, List<Integer>>`, is code.
    fn closing_pre(&self, index: usize, from: usize) -> Option<(usize, usize)> {
        for (line, text) in self.lines[index..].iter().map(|line| line.text).enumerate() {
            let mut at = if line == 0 { from } else { 0 };
            while let Some(found) = text[at..].find('<') {
                at += found;
                if block_element(&text[at..]) == Some((Some(Name::Pre), true)) {
                    return Some((index + line, at));
                }
                let starts_tag = text.as_bytes()[at + 1..]
                    .first()
                    .is_some_and(|&next| next.is_ascii_alphabetic() || next == b'/');
                at += starts_tag
                    .then(|| tags::html_tag_len(&text[at..]))
                    .flatten()
                    .unwrap_or(1);
            }
        }
        None
    }
}
