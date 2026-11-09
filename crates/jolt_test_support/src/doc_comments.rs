//! The oracle's canonical forms of doc comments: what a doc comment means to
//! javadoc, so that formatting may rewrap its text but not change it.
//!
//! - A `/**` comment is its description's words, its block tags with their
//!   words, and its verbatim regions as exact text ([`javadoc`]).
//! - A run of `///` lines is the Markdown events of its description and of
//!   each block tag ([`markdown`]).

use std::fmt::Write as _;

use pulldown_cmark::{CodeBlockKind, CowStr, Event, Options, Parser, Tag, TagEnd};

/// HTML elements that start or end a block. Javadoc's HTML collapses the
/// whitespace around them, so the canonical form splits them off as words
/// of their own: gluing `<p>` to the next word, or moving it to a line of its
/// own, does not change the form.
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

/// Elements whose content is kept byte for byte.
const VERBATIM_ELEMENTS: &[&str] = &["pre", "table"];

/// The canonical form of a `/**` comment, or `None` when `text` is not a
/// terminated `/**` comment with literal delimiters.
///
/// Each line of a multi-line comment loses its indentation, then its run of
/// `*`s and one space after them, as javadoc reads it, and its trailing
/// whitespace; a comment on one line loses only its delimiters and the
/// whitespace around its text. Then the form is, in order:
///
/// - the whitespace-separated words of the description, with block-level
///   HTML tags split off as words of their own;
/// - each block tag (a line that starts with `@` and a letter) on a line of
///   its own, with its words;
/// - each verbatim region (`<pre>`, `<table>`, `{@snippet}`) as its exact
///   lines.
///
/// Inline tags (`{@…}`, matched by brace balance) are lexed first: nothing
/// inside one starts a block tag, a block element or a verbatim region. A
/// `<pre>` or `<table>` without its end tag runs to the end of the comment.
///
/// A comment with a content line that does not start with `*` is printed
/// with its lines as written (javadoc keeps the indentation of such a line),
/// re-indented with the code: its form is the first line trimmed, then each
/// other non-blank line without trailing whitespace and without
/// `indentation`, the indentation of the source line the comment starts on,
/// when it starts with it.
pub(crate) fn javadoc(text: &str, indentation: &str) -> Option<String> {
    let body = text.strip_prefix("/**")?.strip_suffix("*/")?;
    let lines: Vec<&str> = body.split(['\r', '\n']).collect();
    let multiline = lines.len() > 1;
    let star_less = lines
        .iter()
        .skip(1)
        .any(|line| !line.trim().is_empty() && !line.trim_start().starts_with('*'));
    if star_less {
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
        return Some(form);
    }
    let content: Vec<&str> = lines
        .iter()
        .map(|line| {
            let line = line.trim_start();
            match line.strip_prefix('*') {
                Some(rest) if multiline => {
                    let rest = rest.trim_start_matches('*').trim_end();
                    rest.strip_prefix(' ').unwrap_or(rest)
                }
                _ => line.trim(),
            }
        })
        .collect();
    Some(javadoc_form(&content.join("\n")))
}

/// The form of a `/**` comment's content lines, joined with `\n`.
fn javadoc_form(content: &str) -> String {
    let mut form = Form::default();
    let bytes = content.as_bytes();
    let mut index = 0;
    let mut line_start = true;
    while index < bytes.len() {
        let rest = &content[index..];
        let byte = bytes[index];
        if line_start {
            let indentation = rest.len() - rest.trim_start_matches([' ', '\t']).len();
            let after = &rest[indentation..];
            if after.starts_with('@') && after[1..].starts_with(|ch: char| ch.is_ascii_alphabetic())
            {
                form.block_tag();
            }
            line_start = false;
        }
        if byte == b'\n' {
            form.boundary();
            line_start = true;
            index += 1;
            continue;
        }
        if rest.starts_with("{@")
            && let Some(len) = inline_tag_len(rest)
        {
            let tag = &rest[..len];
            if tag.starts_with("{@snippet") {
                form.verbatim(tag);
            } else {
                for ch in tag.chars() {
                    form.char(ch);
                }
            }
            index += len;
            continue;
        }
        if byte == b'<'
            && let Some((name, len)) = block_element(rest)
        {
            let closing = rest.as_bytes()[1] == b'/';
            if !closing && VERBATIM_ELEMENTS.contains(&name.as_str()) {
                let end = find_ascii_ci(rest, &format!("</{name}"))
                    .and_then(|start| rest[start..].find('>').map(|end| start + end + 1))
                    .unwrap_or(rest.len());
                form.verbatim(&rest[..end]);
                index += end;
                continue;
            }
            form.boundary();
            for word in rest[..len].split_whitespace() {
                form.boundary();
                form.word.push_str(word);
            }
            form.boundary();
            index += len;
            continue;
        }
        let ch = rest.chars().next().expect("a char at a boundary");
        form.char(ch);
        index += ch.len_utf8();
    }
    form.finish()
}

/// The canonical form being built: words joined by spaces, a line break
/// before each block tag, and verbatim regions between marker lines.
#[derive(Default)]
struct Form {
    out: String,
    word: String,
}

impl Form {
    /// Ends the current word.
    fn boundary(&mut self) {
        if self.word.is_empty() {
            return;
        }
        if !self.out.is_empty() && !self.out.ends_with('\n') {
            self.out.push(' ');
        }
        self.out.push_str(&self.word);
        self.word.clear();
    }

    fn char(&mut self, ch: char) {
        if ch.is_whitespace() {
            self.boundary();
        } else {
            self.word.push(ch);
        }
    }

    fn block_tag(&mut self) {
        self.boundary();
        self.out.push('\n');
    }

    fn verbatim(&mut self, text: &str) {
        self.boundary();
        let _ = write!(self.out, "\n[verbatim]\n{text}\n[/verbatim]\n");
    }

    fn finish(mut self) -> String {
        self.boundary();
        self.out.trim().to_owned()
    }
}

/// The length of the brace-balanced inline tag at the start of `text`, or
/// `None` when its braces don't balance.
fn inline_tag_len(text: &str) -> Option<usize> {
    let mut depth = 0usize;
    for (index, byte) in text.bytes().enumerate() {
        match byte {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(index + 1);
                }
            }
            _ => {}
        }
    }
    None
}

/// The lowercase name and length of the block-level HTML tag (opening or
/// closing, attributes included) at the start of `text`.
fn block_element(text: &str) -> Option<(String, usize)> {
    let after = text.strip_prefix('<')?;
    let after = after.strip_prefix('/').unwrap_or(after);
    let name_len = after
        .find(|ch: char| !ch.is_ascii_alphanumeric())
        .unwrap_or(after.len());
    let name = after[..name_len].to_ascii_lowercase();
    if !BLOCK_ELEMENTS.contains(&name.as_str()) {
        return None;
    }
    let next = after[name_len..].chars().next()?;
    if !(next == '>' || next == '/' || next.is_whitespace()) {
        return None;
    }
    let end = text.find('>')?;
    Some((name, end + 1))
}

/// The byte offset of `needle` (ASCII) in `text`, ignoring ASCII case.
fn find_ascii_ci(text: &str, needle: &str) -> Option<usize> {
    let needle = needle.as_bytes();
    text.as_bytes()
        .windows(needle.len())
        .position(|window| window.eq_ignore_ascii_case(needle))
}

/// The canonical form of a run of `///` lines (each the comment's text,
/// `///` included), as JEP 467 reads it: the text after `///` without the
/// indentation all non-blank lines share and without trailing whitespace,
/// split at block tags (lines that start with `@` and a letter, outside
/// fenced code blocks and inline tags), each part as its Markdown events.
///
/// Inline tags (`{@…}`, matched by brace balance, outside fenced code
/// blocks) are lexed before block tags. A `{@snippet}` is replaced by U+FFFC,
/// as javadoc does before it parses the Markdown, and its exact lines follow
/// the part's events.
///
/// Adjacent texts are joined and their runs of whitespace collapse, since a
/// rewrapped paragraph renders the same; code is kept exactly. Line breaks
/// are spaces, hard ones included: Jolt never writes trailing whitespace,
/// so a two-space hard break can only be kept in another spelling (`\`).
pub(crate) fn markdown(run: &[&str]) -> String {
    let lines: Vec<&str> = run
        .iter()
        .map(|line| line.strip_prefix("///").unwrap_or(line).trim_end())
        .collect();
    let shared = lines
        .iter()
        .filter(|line| !line.is_empty())
        .map(|line| line.len() - line.trim_start_matches([' ', '\t']).len())
        .min()
        .unwrap_or(0);
    let mut text = String::new();
    for line in &lines {
        text.push_str(line.get(shared..).unwrap_or(""));
        text.push('\n');
    }
    // Each part: its Markdown, and its snippets.
    let mut parts: Vec<(String, Vec<String>)> = vec![(String::new(), Vec::new())];
    let mut copied = 0;
    let mut fence: Option<(u8, usize)> = None;
    let mut tag_end = 0;
    let mut start = 0;
    for line in text.split_inclusive('\n') {
        let line_start = start;
        start += line.len();
        if line_start < tag_end {
            // Inside an inline tag: no fence or block tag starts here.
        } else {
            let trimmed = line.trim_start();
            let marker = trimmed.as_bytes().first().copied();
            if let Some(byte @ (b'`' | b'~')) = marker {
                let len = trimmed.bytes().take_while(|&b| b == byte).count();
                match fence {
                    None if len >= 3 => fence = Some((byte, len)),
                    Some((open, open_len)) if open == byte && len >= open_len => fence = None,
                    _ => {}
                }
                continue;
            }
            if fence.is_some() {
                continue;
            }
            if trimmed.starts_with('@')
                && trimmed[1..].starts_with(|ch: char| ch.is_ascii_alphabetic())
            {
                let part = &mut parts.last_mut().expect("a part").0;
                part.push_str(&text[copied..line_start]);
                copied = line_start;
                parts.push((String::new(), Vec::new()));
            }
        }
        let mut from = tag_end.max(line_start) - line_start;
        while let Some(found) = line.get(from..).and_then(|rest| rest.find("{@")) {
            let at = line_start + from + found;
            let Some(len) = inline_tag_len(&text[at..]) else {
                from += found + 2;
                continue;
            };
            tag_end = at + len;
            if text[at..].starts_with("{@snippet") {
                let (part, snippets) = parts.last_mut().expect("a part");
                part.push_str(&text[copied..at]);
                part.push('\u{fffc}');
                copied = tag_end;
                let lines: Vec<&str> = text[at..tag_end].lines().map(str::trim_end).collect();
                snippets.push(lines.join("\n"));
            }
            if tag_end >= start {
                break;
            }
            from = tag_end - line_start;
        }
    }
    parts
        .last_mut()
        .expect("a part")
        .0
        .push_str(&text[copied..]);
    parts
        .iter()
        .map(|(part, snippets)| {
            let mut form = markdown_events(part);
            for snippet in snippets {
                let _ = writeln!(form, "Snippet({snippet:?})");
            }
            form
        })
        .collect::<Vec<_>>()
        .join("\n@@\n")
}

/// The events of `markdown`, one per line, with texts joined.
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
            Event::SoftBreak | Event::HardBreak => text.push(' '),
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
