//! The oracle's canonical forms of doc comments that Jolt may reflow: what a
//! doc comment means to javadoc, so that formatting may rewrap its text but
//! not change it.
//!
//! These rules are deliberately simpler than the formatter's tokenizers and
//! don't share code with them. Each form applies only when a comment passes
//! a coarse precondition of the oracle's own; for any other comment the
//! oracle compares the text exactly, as for every other comment. A comment
//! that Jolt doesn't reflow keeps its text, so it has the same form either
//! way.
//!
//! - A `/**` comment is its words, its HTML tags, its inline tags, its block
//!   tags and its `<pre>` regions ([`javadoc`]).
//! - A run of `///` lines is the Markdown events of its description and of
//!   each block tag, read by pulldown-cmark ([`markdown`]).

use std::fmt::Write as _;

use pulldown_cmark::{CodeBlockKind, CowStr, Event, Options, Parser, Tag, TagEnd};

/// HTML elements that start or end a block. HTML collapses the whitespace
/// around them, so the form splits them off as words of their own: gluing
/// `<p>` to the next word, or moving it to a line of its own, doesn't change
/// the form.
const BLOCK_ELEMENTS: &[&str] = &[
    "p",
    "ul",
    "ol",
    "li",
    "dl",
    "dt",
    "dd",
    "pre",
    "table",
    "blockquote",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "br",
];

/// One content line of a `/**` comment, without trailing whitespace.
struct Line<'t> {
    /// After the leading whitespace, the run of `*`s and one space.
    text: &'t str,
    /// After the leading whitespace and the run of `*`s: what javadoc reads
    /// in a `<pre>` region.
    code: &'t str,
}

/// The form of a `/**` comment that Jolt may reflow, or `None` when the
/// oracle compares it exactly. Without trailing whitespace, and after the
/// leading whitespace, the run of `*`s and one space of each line (as
/// javadoc reads it; a comment on one line loses only its delimiters), the
/// form is a list of items:
///
/// - the words of the text, split at whitespace;
/// - an HTML tag, from a `<` and a letter or `/` to the next `>` on its
///   line, exactly: it is part of the word around it, except a block-level
///   element's tag, which is a word of its own;
/// - an inline tag (`{@` to the `}` that balances it), with its whitespace
///   collapsed to one space: part of the word around it;
/// - a block tag (a line that starts with `@` and a letter);
/// - a `<pre>` region, from `<pre` to `</pre>`, exactly: its first line
///   from `<pre`, and the others after their run of `*`s, as javadoc reads
///   them.
///
/// Inline tags are read first, then the rest from left to right. The
/// precondition: the comment is a terminated `/**` (not `/***`) without
/// `<!--` or `{@snippet`, each content line after the first starts with
/// `*`, each `<pre>` is closed, and each HTML tag outside inline tags and
/// `<pre>` regions closes on its line.
pub(crate) fn javadoc(text: &str) -> Option<String> {
    if text.starts_with("/***") || text.contains("<!--") || text.contains("{@snippet") {
        return None;
    }
    let lines = content_lines(text)?;
    let mut form = Form::default();
    let (mut index, mut at) = (0, 0);
    while index < lines.len() {
        let text = lines[index].text;
        if at == 0 {
            form.boundary();
            let rest = text.trim_start_matches([' ', '\t']);
            if rest.starts_with('@') && rest[1..].starts_with(|ch: char| ch.is_ascii_alphabetic()) {
                form.items.push("BlockTag".to_owned());
            }
        }
        let Some(ch) = text[at..].chars().next() else {
            (index, at) = (index + 1, 0);
            continue;
        };
        let rest = &text[at..];
        if rest.starts_with("{@")
            && let Some((end_line, end)) = inline_tag_end(&lines, index, at)
        {
            let tag: Vec<&str> = (index..=end_line)
                .map(|line| {
                    let text = lines[line].text;
                    let to = if line == end_line { end } else { text.len() };
                    &text[if line == index { at } else { 0 }..to]
                })
                .collect();
            form.word.push_str(&inline_tag_form(&tag.join("\n")));
            (index, at) = (end_line, end);
            continue;
        }
        let next = rest[ch.len_utf8()..].chars().next();
        if ch == '<' && next.is_some_and(|next| next.is_ascii_alphabetic() || next == '/') {
            let len = rest.find('>')? + 1;
            let tag = &rest[..len];
            if element_name(tag) == Some(("pre", false)) {
                let (end_line, end) = closing_pre(&lines, index, at + len)?;
                form.boundary();
                let mut region = rest.to_owned();
                for line in &lines[index + 1..=end_line] {
                    region.push('\n');
                    region.push_str(line.code);
                }
                if end_line > index {
                    let last = &lines[end_line];
                    region.truncate(region.len() - (last.text.len() - end));
                } else {
                    region.truncate(end - at);
                }
                form.items.push(format!("Pre({region:?})"));
                (index, at) = (end_line, end);
                continue;
            }
            let block = element_name(tag).is_some_and(|(name, _)| BLOCK_ELEMENTS.contains(&name));
            if block {
                form.boundary();
            }
            form.word.push_str(tag);
            if block {
                form.boundary();
            }
            at += len;
            continue;
        }
        if ch.is_whitespace() {
            form.boundary();
        } else {
            form.word.push(ch);
        }
        at += ch.len_utf8();
    }
    form.boundary();
    Some(form.items.join("\n"))
}

/// The content lines of a terminated `/**` comment, or `None` when a line
/// after the first doesn't start with `*`.
fn content_lines(text: &str) -> Option<Vec<Line<'_>>> {
    let body = text.strip_prefix("/**")?.strip_suffix("*/")?;
    let parts: Vec<&str> = body
        .split("\r\n")
        .flat_map(|line| line.split(['\r', '\n']))
        .collect();
    let mut lines = Vec::with_capacity(parts.len());
    for (index, part) in parts.iter().enumerate() {
        let trimmed = part.trim_start();
        let line = match trimmed.strip_prefix('*') {
            Some(after) if parts.len() > 1 => {
                let code = after.trim_start_matches('*').trim_end();
                Line {
                    text: code.strip_prefix(' ').unwrap_or(code),
                    code,
                }
            }
            _ if index > 0 && !trimmed.trim().is_empty() => return None,
            _ => Line {
                text: trimmed.trim(),
                code: "",
            },
        };
        lines.push(line);
    }
    Some(lines)
}

/// The form of a `/**` comment with a content line after the first that
/// doesn't start with `*`, or `None` for another comment. Jolt moves such a
/// comment as a unit, so its form is the first line trimmed, then each other
/// non-blank line without trailing whitespace and without `indentation`, the
/// indentation of the source line the comment starts on, when it starts with
/// it.
pub(crate) fn star_less(text: &str, indentation: &str) -> Option<String> {
    text.strip_prefix("/**")?.strip_suffix("*/")?;
    let mut lines = text.split(['\r', '\n']);
    lines.next();
    if !lines.any(|line| !line.trim().is_empty() && !line.trim_start().starts_with('*')) {
        return None;
    }
    let mut form = String::new();
    for (index, line) in text.split(['\r', '\n']).enumerate() {
        let line = if index == 0 {
            line.trim()
        } else {
            let line = line.trim_end();
            line.strip_prefix(indentation).unwrap_or(line)
        };
        if !line.is_empty() {
            form.push_str(line);
            form.push('\n');
        }
    }
    Some(form)
}

/// The items of a `/**` form being built.
#[derive(Default)]
struct Form {
    items: Vec<String>,
    word: String,
}

impl Form {
    /// Ends the current word.
    fn boundary(&mut self) {
        if !self.word.is_empty() {
            let word = std::mem::take(&mut self.word);
            self.items.push(format!("{word:?}"));
        }
    }
}

/// The lowercase name of the HTML tag `tag` (`<…>`), and whether it closes
/// its element.
fn element_name(tag: &str) -> Option<(&'static str, bool)> {
    let after = tag.strip_prefix('<')?;
    let closing = after.starts_with('/');
    let name = &after[usize::from(closing)..];
    let len = name
        .find(|ch: char| !ch.is_ascii_alphanumeric())
        .unwrap_or(name.len());
    if !name[len..].starts_with(|ch: char| ch == '>' || ch == '/' || ch.is_whitespace()) {
        return None;
    }
    BLOCK_ELEMENTS
        .iter()
        .find(|element| element.eq_ignore_ascii_case(&name[..len]))
        .map(|&element| (element, closing))
}

/// The line and byte after the `}` that balances the `{` at byte `at` of
/// line `index`.
fn inline_tag_end(lines: &[Line<'_>], index: usize, at: usize) -> Option<(usize, usize)> {
    let mut depth = 0usize;
    for (line, text) in lines.iter().enumerate().skip(index) {
        let from = if line == index { at } else { 0 };
        for (offset, byte) in text.text.bytes().enumerate().skip(from) {
            match byte {
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some((line, offset + 1));
                    }
                }
                _ => {}
            }
        }
    }
    None
}

/// The line and byte after the `</pre>` that follows byte `from` of line
/// `index`, whose `>` is on its line.
fn closing_pre(lines: &[Line<'_>], index: usize, from: usize) -> Option<(usize, usize)> {
    for (line, text) in lines.iter().enumerate().skip(index) {
        let start = if line == index { from } else { 0 };
        let text = &text.text[start..];
        let mut at = 0;
        while let Some(found) = text[at..].find('<') {
            at += found;
            // A tag's end: its `>` outside quotes, on its line.
            let mut quote = None;
            let len = text[at + 1..].char_indices().find_map(|(index, ch)| {
                match (quote, ch) {
                    (None, '"' | '\'') => quote = Some(ch),
                    (Some(open), ch) if ch == open => quote = None,
                    (None, '>') => return Some(index + 2),
                    _ => {}
                }
                None
            });
            let tag = text[at + 1..].starts_with(|ch: char| ch.is_ascii_alphabetic() || ch == '/');
            match len {
                Some(len) if element_name(&text[at..at + len]) == Some(("pre", true)) => {
                    return Some((line, start + at + len));
                }
                Some(len) if tag => at += len,
                _ => at += 1,
            }
        }
    }
    None
}

/// The form of a run of `///` lines (each with its `///`) that Jolt may
/// reflow, or `None` when the oracle compares its lines exactly. As JEP 467
/// reads it: the text after `///` without the indentation that all lines
/// with text share and without trailing whitespace, split at block tags
/// (lines that start with `@` and a letter), each part as its Markdown
/// events ([`markdown_events`]). The precondition: no `{@snippet`, and no
/// line with text that ends in two spaces or `\` before a line with text,
/// which may be a hard break. In a source with syntax errors (not `clean`),
/// trailing spaces don't count: javadoc reads no doc comment there, and Jolt
/// may print a run in a malformed node without its trailing whitespace.
pub(crate) fn markdown(run: &[&str], clean: bool) -> Option<String> {
    let lines: Vec<&str> = run
        .iter()
        .map(|line| line.strip_prefix("///").unwrap_or(line))
        .collect();
    for (index, line) in lines.iter().enumerate() {
        let has_text = |line: &str| !line.trim().is_empty();
        let ends_in_break = (clean && line.ends_with("  ")) || line.trim_end().ends_with('\\');
        if line.contains("{@snippet")
            || (has_text(line)
                && ends_in_break
                && lines.get(index + 1).is_some_and(|next| has_text(next)))
        {
            return None;
        }
    }
    let shared = lines
        .iter()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.len() - line.trim_start_matches([' ', '\t']).len())
        .min()
        .unwrap_or(0);
    let mut parts = vec![String::new()];
    for line in &lines {
        let line = line.get(shared..).unwrap_or("").trim_end();
        let trimmed = line.trim_start();
        if trimmed.starts_with('@') && trimmed[1..].starts_with(|ch: char| ch.is_ascii_alphabetic())
        {
            parts.push(String::new());
        }
        let part = parts.last_mut().expect("a part");
        part.push_str(line);
        part.push('\n');
    }
    Some(
        parts
            .iter()
            .map(|part| tag_part_events(part))
            .collect::<Vec<_>>()
            .join("\n@@\n"),
    )
}

/// The events of one part of a run: the description, or a block tag. javadoc
/// reads a block tag's description as Markdown of its own, after the tag and,
/// for `@param`, `@throws` and `@exception`, the name: so `@return # Title`
/// is a heading, and an indented line after `@return` alone is code.
fn tag_part_events(part: &str) -> String {
    let trimmed = part.trim_start_matches([' ', '\t']);
    if !(trimmed.starts_with('@') && trimmed[1..].starts_with(|ch: char| ch.is_ascii_alphabetic()))
    {
        return markdown_events(part);
    }
    let word_end = |text: &str| text.find(char::is_whitespace).unwrap_or(text.len());
    let tag_len = word_end(trimmed);
    let tag = &trimmed[..tag_len];
    let mut rest = &trimmed[tag_len..];
    let mut head = tag.to_owned();
    if matches!(tag, "@param" | "@throws" | "@exception") {
        let after = rest.trim_start_matches([' ', '\t']);
        let name_len = word_end(after);
        head.push(' ');
        head.push_str(&after[..name_len]);
        rest = &after[name_len..];
    }
    // The separator after the tag (or name) is not part of the description.
    let description = rest.trim_start_matches([' ', '\t']);
    format!("Tag({head:?})\n{}", markdown_events(description))
}

/// The events of `markdown`, one per line. Adjacent texts are joined and
/// their runs of whitespace collapse, since a rewrapped paragraph renders the
/// same; code is kept exactly. A soft line break is a space.
fn markdown_events(markdown: &str) -> String {
    let mut out = String::new();
    let mut text = String::new();
    // Inside a code block or an HTML block, where whitespace is content.
    let mut literal = 0usize;
    let flush = |out: &mut String, text: &mut String| {
        if !text.is_empty() {
            let _ = writeln!(out, "Text({:?})", collapse(text));
            text.clear();
        }
    };
    for event in Parser::new_ext(markdown, Options::ENABLE_TABLES) {
        let event = borrowed(&event);
        match event {
            Event::Text(value) if literal > 0 => {
                flush(&mut out, &mut text);
                let _ = writeln!(out, "Literal({value:?})");
            }
            Event::Text(value) => text.push_str(&value),
            Event::SoftBreak => text.push(' '),
            event => {
                flush(&mut out, &mut text);
                match &event {
                    Event::Start(Tag::CodeBlock(_) | Tag::HtmlBlock) => literal += 1,
                    Event::End(TagEnd::CodeBlock | TagEnd::HtmlBlock) => literal -= 1,
                    _ => {}
                }
                let _ = writeln!(out, "{event:?}");
            }
        }
    }
    flush(&mut out, &mut text);
    out
}

/// `event` with its strings borrowed, so that its debug form does not show
/// how pulldown-cmark stored them (`Borrowed`, `Boxed` or `Inlined`), which
/// depends on where the text was split into lines.
fn borrowed<'e>(event: &'e Event<'_>) -> Event<'e> {
    let text = |value: &'e CowStr<'_>| CowStr::Borrowed(value);
    match event {
        Event::Text(value) => Event::Text(text(value)),
        Event::Code(value) => Event::Code(text(value)),
        Event::Html(value) => Event::Html(text(value)),
        Event::InlineHtml(value) => Event::InlineHtml(text(value)),
        Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(info))) => {
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(text(info))))
        }
        Event::Start(Tag::Link {
            link_type,
            dest_url,
            title,
            id,
        }) => Event::Start(Tag::Link {
            link_type: *link_type,
            dest_url: text(dest_url),
            title: text(title),
            id: text(id),
        }),
        Event::Start(Tag::Image {
            link_type,
            dest_url,
            title,
            id,
        }) => Event::Start(Tag::Image {
            link_type: *link_type,
            dest_url: text(dest_url),
            title: text(title),
            id: text(id),
        }),
        event => event.clone(),
    }
}

/// An inline tag's form: its text with each run of whitespace made one
/// space, except in `{@code}` and `{@literal}`, whose whitespace at the start
/// and end of their content is text in javadoc's reading (after the one
/// separator javadoc drops after the name) and is kept exactly.
fn inline_tag_form(tag: &str) -> String {
    let name_end = tag[2..]
        .find(|ch: char| ch.is_whitespace() || ch == '}')
        .map_or(tag.len(), |len| 2 + len);
    let name = &tag[..name_end];
    if !matches!(name, "{@code" | "{@literal") {
        return collapse(tag).trim_end().to_owned();
    }
    let mut body = tag[name_end..]
        .strip_suffix('}')
        .unwrap_or(&tag[name_end..]);
    let mut chars = body.chars();
    if chars.next().is_some_and(char::is_whitespace) {
        body = chars.as_str();
    }
    let inner = body.trim();
    let lead = &body[..body.len() - body.trim_start().len()];
    let trail = if inner.is_empty() {
        ""
    } else {
        &body[body.trim_end().len()..]
    };
    format!("{name} {lead:?}{}{trail:?}}}", collapse(inner))
}

/// `text` with each run of whitespace made one space.
fn collapse(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut space = false;
    for ch in text.chars() {
        if ch.is_whitespace() {
            space = true;
        } else {
            if space {
                out.push(' ');
                space = false;
            }
            out.push(ch);
        }
    }
    if space {
        out.push(' ');
    }
    out
}
