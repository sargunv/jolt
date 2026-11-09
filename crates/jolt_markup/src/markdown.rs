//! Markdown doc comments (`///`, JEP 467): javadoc's reading of a run
//! (`parser`), whose Markdown text is read again with pulldown-cmark, and
//! its layout.
//!
//! javadoc reads the description and each block tag's text as a Markdown
//! document of its own, in which each inline tag is one character (U+FFFC,
//! as `HtmlDocletWriter.MarkdownHandler` builds it for commonmark-java).
//! This module builds the same text for each part, copying the run's
//! Markdown text into one `String`, parses it with pulldown-cmark, and maps
//! each top-level block back to its source lines:
//!
//! - a paragraph whose lines hold no other block is filled; its atoms (code
//!   spans, links, autolinks, inline HTML, a `<pre>` element, inline tags)
//!   never split, and its hard breaks stay;
//! - every other block (a list, heading, code block, quote, table, HTML
//!   block), every line that no block covers (a blank line, a link
//!   reference definition), and lines that two blocks share are printed as
//!   written.
//!
//! A block tag's head (`@param name`) is joined with single spaces; its text
//! starts after it on the tag's line, and its wrapped lines are flush with
//! the frame (an indented line could start code or continue a list).

use std::ops::Range;

use pulldown_cmark::{BrokenLink, CodeBlockKind, CowStr, Event, Options, Parser, Tag, TagEnd};

use crate::frame::Line;
use crate::html::Raw;
use crate::layout::{Join, Layout};
use crate::parser::{BlockTag, Comment, Inline, Node, Pos, Span, to_u32};
use crate::words;

/// Where a reader of a part's HTML is, between two pieces of it: what
/// pulldown-cmark reports as HTML (inline HTML, or a line of an HTML block)
/// may end inside a tag, an attribute value or a comment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Lex {
    Text,
    /// In a tag of `element` (`None`: another element), after `=` when `eq`.
    Tag {
        element: Option<Element>,
        closing: bool,
        eq: bool,
    },
    /// In a tag's attribute value quoted by `quote`.
    Value {
        element: Option<Element>,
        closing: bool,
        quote: u8,
    },
    /// In a comment, to `-->`.
    Comment,
    /// In a declaration or processing instruction, to `>`.
    Declaration,
}

/// An element whose content keeps its whitespace: `<pre>`, whose content
/// HTML doesn't collapse, `<code>`, whose content javadoc's oracle reads
/// exactly (the HTML that javadoc renders from Markdown holds a code span as
/// `<code>`, which raw HTML can't be told apart from), and a raw text
/// element.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Element {
    Pre,
    Raw(Raw),
}

/// HTML's reading of a part's HTML pieces, as far as whitespace goes: the
/// open `<pre>` and `<code>` elements and the open raw text element. A raw text
/// element's content is text up to its own end tag (`</name` and then
/// whitespace, `/` or `>`, in any case); elsewhere a tag ends at the first
/// `>` outside a quoted attribute value.
#[derive(Clone, Copy, Debug)]
struct Reader {
    pre: u32,
    raw: Option<Raw>,
    lex: Lex,
}

impl Reader {
    const fn new() -> Self {
        Self {
            pre: 0,
            raw: None,
            lex: Lex::Text,
        }
    }

    /// Whether the text that follows keeps its whitespace: in an open
    /// `<pre>`, `<code>` or raw text element, and inside a tag or a
    /// declaration (javadoc's oracle reads them as written), a quoted
    /// attribute value or a comment, wherever they are.
    const fn exact(&self) -> bool {
        self.pre > 0
            || self.raw.is_some()
            || matches!(
                self.lex,
                Lex::Tag { .. } | Lex::Value { .. } | Lex::Comment | Lex::Declaration
            )
    }

    /// Reads the next piece of HTML, which starts at byte `start` of a
    /// part's `text`, line by line: a line that ends inside a tag, a comment
    /// or an element that keeps whitespace keeps its end (in `kept`), though
    /// the piece closes it on a later line.
    fn read_lines(&mut self, html: &str, start: usize, text: &Text, kept: &mut Vec<Range<u32>>) {
        let mut offset = start;
        for piece in html.split_inclusive('\n') {
            self.read(piece);
            offset += piece.len();
            if piece.ends_with('\n') && self.exact() && offset <= text.text.len() {
                let line = text.pos(offset - 1).line;
                kept.push(line..line + 1);
            }
        }
    }

    /// Reads the next piece of HTML.
    fn read(&mut self, html: &str) {
        let bytes = html.as_bytes();
        let mut at = 0;
        while at < bytes.len() {
            match self.lex {
                Lex::Text => {
                    let Some(found) = html[at..].find('<') else {
                        return;
                    };
                    at = self.after_lt(html, at + found + 1);
                }
                Lex::Tag {
                    element,
                    closing,
                    eq,
                } => {
                    let byte = bytes[at];
                    at += 1;
                    match byte {
                        b'>' => {
                            self.lex = Lex::Text;
                            self.close_tag(element, closing);
                        }
                        b'"' | b'\'' if eq => {
                            self.lex = Lex::Value {
                                element,
                                closing,
                                quote: byte,
                            };
                        }
                        b' ' | b'\t' | b'\n' | b'\r' | 0x0c => {}
                        _ => {
                            self.lex = Lex::Tag {
                                element,
                                closing,
                                eq: byte == b'=',
                            };
                        }
                    }
                }
                Lex::Value {
                    element,
                    closing,
                    quote,
                } => match bytes[at..].iter().position(|&byte| byte == quote) {
                    Some(found) => {
                        at += found + 1;
                        self.lex = Lex::Tag {
                            element,
                            closing,
                            eq: false,
                        };
                    }
                    None => return,
                },
                Lex::Comment => match html[at..].find("-->") {
                    Some(found) => {
                        at += found + 3;
                        self.lex = Lex::Text;
                    }
                    None => return,
                },
                Lex::Declaration => match html[at..].find('>') {
                    Some(found) => {
                        at += found + 1;
                        self.lex = Lex::Text;
                    }
                    None => return,
                },
            }
        }
    }

    /// Reads what follows a `<` at byte `at` of `html` in text: a tag's
    /// name, the start of a comment or declaration, or nothing (in a raw
    /// text element, anything but its end tag). Returns where reading goes
    /// on.
    fn after_lt(&mut self, html: &str, mut at: usize) -> usize {
        let bytes = html.as_bytes();
        let rest = &bytes[at..];
        if let Some(raw) = self.raw {
            let name = raw.name().as_bytes();
            if rest.first() == Some(&b'/')
                && rest.len() > name.len() + 1
                && rest[1..=name.len()].eq_ignore_ascii_case(name)
                && matches!(
                    rest[name.len() + 1],
                    b' ' | b'\t' | b'\n' | 0x0c | b'\r' | b'/' | b'>'
                )
            {
                self.lex = Lex::Tag {
                    element: Some(Element::Raw(raw)),
                    closing: true,
                    eq: false,
                };
                at += name.len() + 1;
            }
        } else if rest.starts_with(b"!--") {
            // HTML closes `<!-->` and `<!--->` at once (an abruptly closed
            // empty comment), as CommonMark does.
            if rest[3..].starts_with(b">") {
                at += 4;
            } else if rest[3..].starts_with(b"->") {
                at += 5;
            } else {
                self.lex = Lex::Comment;
                at += 3;
            }
        } else if rest
            .first()
            .is_some_and(|&byte| byte == b'!' || byte == b'?')
        {
            self.lex = Lex::Declaration;
        } else {
            let closing = rest.first() == Some(&b'/');
            let from = at + usize::from(closing);
            if bytes.get(from).is_some_and(u8::is_ascii_alphabetic) {
                let len = bytes[from..]
                    .iter()
                    .take_while(|&&byte| byte.is_ascii_alphanumeric() || byte == b'-')
                    .count();
                let name = &html[from..from + len];
                let element =
                    if name.eq_ignore_ascii_case("pre") || name.eq_ignore_ascii_case("code") {
                        Some(Element::Pre)
                    } else {
                        Raw::of(name).map(Element::Raw)
                    };
                self.lex = Lex::Tag {
                    element,
                    closing,
                    eq: false,
                };
                at = from + len;
            }
        }
        at
    }

    /// A tag of `element` ended.
    fn close_tag(&mut self, element: Option<Element>, closing: bool) {
        match (element, closing) {
            (Some(Element::Pre), false) => self.pre += 1,
            (Some(Element::Pre), true) => self.pre = self.pre.saturating_sub(1),
            (Some(Element::Raw(raw)), false) => self.raw = Some(raw),
            (Some(Element::Raw(_)), true) => self.raw = None,
            (None, _) => {}
        }
    }
}

/// Keeps the line ends inside the nodes whose whitespace javadoc reads:
/// inline tags without rich text, escapes, erroneous nodes, and a rich
/// tag's head.
fn keep_ends(nodes: &[Node], layout: &mut Layout<'_, '_, '_>) {
    for node in nodes {
        match node {
            Node::Text(_) => {}
            Node::Atom(span) | Node::Erroneous(span) | Node::Element { span, .. } => {
                layout.keep_ends(*span);
            }
            Node::Inline(inline) => match &inline.rich {
                None => layout.keep_ends(inline.span),
                Some(rich) => {
                    if let Some(head) = rich.head {
                        layout.keep_ends(head);
                    }
                    keep_ends(&rich.content, layout);
                }
            },
        }
    }
}

/// The Object Replacement Character, for an inline tag.
const PLACEHOLDER: char = '\u{fffc}';

/// Lays out a Markdown comment read by `parser`; `None` keeps the run as
/// written: when a block tag doesn't start its line (javac reads `- @tag`
/// as a list marker and a block tag).
pub(crate) fn lay_out<'l, 'a>(
    comment: &Comment,
    lines: &'l [Line<'a>],
    layout: &mut Layout<'_, 'l, 'a>,
) -> Option<()> {
    for tag in &comment.tags {
        let line = lines[tag.span.start.line as usize].text;
        if !line[..tag.span.start.off as usize]
            .bytes()
            .all(|byte| byte == b' ' || byte == b'\t')
        {
            return None;
        }
    }
    keep_ends(&comment.body, layout);
    for tag in &comment.tags {
        layout.tag_line(tag.span.start.line);
        match &tag.body {
            Some(body) => keep_ends(body, layout),
            None => layout.keep_ends(tag.span),
        }
    }
    if let Some(tail) = comment.tail {
        layout.keep_ends(tail);
    }
    let mut markdown = Markdown {
        lines,
        layout,
        next: 0,
        first_text: to_u32(
            lines
                .iter()
                .position(|line| !line.is_blank())
                .unwrap_or(lines.len()),
        ),
        first_sentence: true,
        kept_as_written: false,
    };
    markdown.section(&comment.body, None, !comment.tags.is_empty());
    for (index, tag) in comment.tags.iter().enumerate() {
        markdown.block_tag(tag, index == 0, index + 1 < comment.tags.len());
    }
    if markdown.kept_as_written {
        return None;
    }
    let last = to_u32(lines.len() - 1);
    if let Some(tail) = comment.tail {
        markdown.verbatim_until(tail.start.line);
    }
    markdown.verbatim_until(last + 1);
    Some(())
}

/// The end of the word of a paragraph's `text` that starts at byte `at`:
/// the next whitespace outside an atom, or, while a span of `unclosed` is
/// open, outside it. A link, a tag or a code span that this reader didn't
/// take for one (javadoc's may) stays whole, with its whitespace, up to where
/// it closes: a line break or a run of spaces in it can decide what it is.
fn end_of_word(
    text: &Text,
    mut at: usize,
    end: usize,
    atoms: &mut std::iter::Peekable<std::slice::Iter<'_, Range<usize>>>,
    unclosed: &mut LinkScan,
) -> usize {
    let source = text.text.as_bytes();
    let space = |byte: &u8| matches!(byte, b' ' | b'\t' | b'\n' | b'\r');
    loop {
        let run = at;
        loop {
            while atoms.peek().is_some_and(|atom| atom.end <= at) {
                atoms.next();
            }
            if let Some(atom) = atoms.peek().filter(|atom| atom.start <= at) {
                at = atom.end.max(at + 1);
                continue;
            }
            if at >= end || space(&source[at]) {
                break;
            }
            at += text.text[at..].chars().next().map_or(1, char::len_utf8);
        }
        unclosed.scan(&text.text[run..at]);
        let next = at
            + source[at..end]
                .iter()
                .take_while(|byte| space(byte))
                .count();
        if !unclosed.open() || next >= end {
            return at;
        }
        at = next;
    }
}

/// What a paragraph's words have opened of a link, a tag or a code span,
/// word by word: the `[`s not closed yet, whether an inline link's
/// destination is open, in `<…>` or not, whether a tag is (a `<` and a
/// letter, `/`, `!` or `?`, up to its `>`), and the length of the run of
/// `` ` `` that opened a code span not closed yet.
#[derive(Clone, Copy, Default)]
struct LinkScan {
    brackets: u32,
    destination: Option<Destination>,
    tag: bool,
    code: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Destination {
    Angle,
    Plain,
}

impl LinkScan {
    const fn open(self) -> bool {
        self.brackets > 0 || self.destination.is_some() || self.tag || self.code > 0
    }

    fn scan(&mut self, word: &str) {
        let bytes = word.as_bytes();
        let mut at = 0;
        while at < bytes.len() {
            if bytes[at] == b'`' && !self.tag && self.destination.is_none() {
                let run = bytes[at..].iter().take_while(|&&byte| byte == b'`').count();
                if self.code == 0 {
                    self.code = run;
                } else if self.code == run {
                    self.code = 0;
                }
                at += run;
                continue;
            }
            if self.code > 0 {
                at += 1;
                continue;
            }
            if self.tag {
                self.tag = bytes[at] != b'>';
                at += 1;
                continue;
            }
            match (self.destination, bytes[at]) {
                (Some(Destination::Angle), b'>') => self.destination = Some(Destination::Plain),
                (Some(Destination::Plain), b')') => self.destination = None,
                (None, b'\\') => at += 1,
                (None, b'<') => {
                    self.tag = bytes
                        .get(at + 1)
                        .is_some_and(|&next| next.is_ascii_alphabetic() || b"/!?".contains(&next));
                }
                (None, b'[') => self.brackets += 1,
                (None, b']') => {
                    self.brackets = self.brackets.saturating_sub(1);
                    if bytes.get(at + 1) == Some(&b'(') {
                        at += 1;
                        self.destination = Some(if bytes.get(at + 1) == Some(&b'<') {
                            at += 1;
                            Destination::Angle
                        } else {
                            Destination::Plain
                        });
                    }
                }
                _ => {}
            }
            at += 1;
        }
    }
}

/// The columns of a line's indentation, a tab moving to the next multiple
/// of four, as Markdown counts them.
fn columns(indentation: &str) -> u16 {
    indentation.chars().fold(0, |column, ch| {
        if ch == '\t' {
            column.saturating_add(4 - column % 4)
        } else {
            column.saturating_add(1)
        }
    })
}

/// Whether a fenced code block's text ends with a line that closes it: as
/// many of its fence's characters or more, and nothing else but
/// indentation. One this doesn't recognize (a closing fence in a quote) is
/// taken for unclosed.
fn closes_fence(block: &str) -> bool {
    let mut lines = block.trim_end().lines();
    let opener = lines.next().unwrap_or("").trim_start();
    let Some(fence) = opener.chars().next() else {
        return false;
    };
    let length = opener.chars().take_while(|&ch| ch == fence).count();
    lines.next_back().is_some_and(|last| {
        let last = last.trim();
        last.chars().count() >= length && last.chars().all(|ch| ch == fence)
    })
}

/// A piece of a part's Markdown text: the text of a source line from `pos`
/// on, or a placeholder for an inline tag at `span`.
#[derive(Clone, Copy, Debug)]
enum Segment<'n> {
    Text {
        pos: Pos,
    },
    /// An inline tag (or an escape or erroneous node, `inline` `None`).
    Placeholder {
        span: Span,
        inline: Option<&'n Inline>,
        erroneous: bool,
    },
}

/// One part's Markdown text, as javadoc builds it, and where each piece
/// comes from.
#[allow(clippy::struct_field_names, reason = "a part's text and its pieces")]
struct Text<'n> {
    text: String,
    /// The start of each piece in `text`, in order.
    segments: Vec<(usize, Segment<'n>)>,
    /// The offset of each `\n` in `text`, in order.
    newlines: Vec<usize>,
    /// The text ends before the end of its last source line: a block tag
    /// follows it there.
    ends_mid_line: bool,
}

impl<'n> Text<'n> {
    /// The source position of byte `offset` of the text (the start of a
    /// placeholder for an offset inside it).
    fn pos(&self, offset: usize) -> Pos {
        let index = self
            .segments
            .partition_point(|&(start, _)| start <= offset)
            .saturating_sub(1);
        let (start, segment) = self.segments[index];
        match segment {
            Segment::Text { pos } => Pos {
                line: pos.line,
                off: pos.off + to_u32(offset - start),
            },
            Segment::Placeholder { span, .. } => span.start,
        }
    }

    /// The inline tag whose placeholder starts at byte `offset`.
    fn inline_at(&self, offset: usize) -> Option<&'n Inline> {
        let index = self.segments.partition_point(|&(start, _)| start < offset);
        match self.segments.get(index) {
            Some(&(start, Segment::Placeholder { inline, .. })) if start == offset => inline,
            _ => None,
        }
    }

    /// Whether the text before byte `offset` ends with an erroneous node.
    fn erroneous_before(&self, offset: usize) -> bool {
        let index = self.segments.partition_point(|&(start, _)| start < offset);
        index > 0
            && matches!(
                self.segments[index - 1],
                (start, Segment::Placeholder { erroneous: true, .. })
                    if start + PLACEHOLDER.len_utf8() == offset
            )
    }

    /// The offset of the last `\n` before byte `offset`.
    fn newline_before(&self, offset: usize) -> Option<usize> {
        let index = self.newlines.partition_point(|&found| found < offset);
        index.checked_sub(1).map(|index| self.newlines[index])
    }

    /// The offset of the first `\n` at or after byte `offset`.
    fn newline_after(&self, offset: usize) -> Option<usize> {
        let index = self.newlines.partition_point(|&found| found < offset);
        self.newlines.get(index).copied()
    }

    /// The end position of a range that ends at `offset`.
    fn end(&self, offset: usize) -> Pos {
        if offset == 0 {
            return self.pos(0);
        }
        let index = self
            .segments
            .partition_point(|&(start, _)| start < offset)
            .saturating_sub(1);
        match self.segments[index] {
            (_, Segment::Placeholder { span, .. }) => span.end,
            _ => self.pos(offset),
        }
    }
}

/// What a part leaves open at its end: a fenced code block that reaches
/// it unclosed, or a tag that an HTML block ends inside.
#[derive(Default)]
struct Open {
    fence: bool,
    tag: bool,
}

/// A top-level block of a part: its range in the part's text, its source
/// lines, and the paragraph's layout when it is one.
struct Block {
    lines: Range<u32>,
    paragraph: Option<Paragraph>,
}

/// A paragraph's text range, its atoms (ranges no wrap splits), and where
/// its hard breaks end lines.
struct Paragraph {
    range: Range<usize>,
    atoms: Vec<Range<usize>>,
    breaks: Vec<usize>,
}

struct Markdown<'m, 'd, 'l, 'a> {
    lines: &'l [Line<'a>],
    layout: &'m mut Layout<'d, 'l, 'a>,
    /// The first source line not yet printed.
    next: u32,
    /// The first line with text.
    first_text: u32,
    /// Whether the next word may still be in javadoc's first sentence: in
    /// the description, before a blank line or a later line that starts
    /// with one of [`words::ends_first_paragraph`] (`DocTreeMaker`'s
    /// `endPara`, which reads the text, not its blocks: a heading or a
    /// thematic break on the first line doesn't end the sentence). javadoc
    /// also ends it at a `.` and whitespace, which this doesn't follow: a
    /// word it takes for one keeps its line break when it needn't.
    first_sentence: bool,
    /// The comment can't be laid out: a part that a block tag follows ends
    /// in an unclosed fenced code block (javadoc renders each part as a
    /// document of its own, so the block holds the part's last lines up to
    /// the tag, with the whitespace before the tag on its line and the blank
    /// line a layout puts there), or an HTML block ends inside a tag.
    kept_as_written: bool,
}

impl Markdown<'_, '_, '_, '_> {
    /// Ends the first sentence at a line from `checked` up to `until` that
    /// ends javadoc's first paragraph: a blank line, or one that starts with
    /// up to three spaces and one of [`words::ends_first_paragraph`].
    fn end_first_sentence(&mut self, checked: &mut u32, until: u32) {
        let until = until.min(to_u32(self.lines.len()));
        while *checked < until {
            let line = self.lines[*checked as usize].text;
            let text = line.trim_start_matches(' ');
            if text.trim_start_matches(['\t', ' ']).is_empty()
                || line.len() - text.len() <= 3 && words::ends_first_paragraph(text)
            {
                self.first_sentence = false;
            }
            *checked += 1;
        }
    }

    /// Starts a paragraph's first word, after the tag's head when
    /// `on_tag_line`. A word that would end javadoc's first sentence
    /// (`ends_sentence`) or start a block at the start of a line keeps the
    /// indentation that keeps it from doing so (after a link reference
    /// definition, which no paragraph continues).
    fn first_word(
        &mut self,
        word: &str,
        on_tag_line: bool,
        ends_sentence: bool,
        source_indent: u16,
    ) -> (Join, bool) {
        if on_tag_line {
            return (Join::Space, false);
        }
        let deep = source_indent > 3;
        let indent = if deep && (ends_sentence || words::starts_structure(word, true)) {
            source_indent
        } else {
            0
        };
        self.layout.line(false, indent, 0);
        (Join::Wrap, false)
    }

    /// Whether the whitespace at the end of `line`, after `word` (the
    /// paragraph's `last` word, or not), stays: after a `\\` it keeps the
    /// `\\` from ending the line, a hard break; in a link, tag or code span
    /// still open, javadoc's reader may read it in one. After the paragraph's
    /// last word, two spaces before a line with text are no hard break in the
    /// paragraph, but one in javadoc's first sentence, which it reads on past
    /// the paragraph's end; and that sentence ends at the paragraph's end after
    /// an inline tag or a reference link (`]`) only when whitespace comes
    /// between.
    fn keeps_end(&self, word: &str, line: u32, last: bool, unclosed: LinkScan) -> bool {
        word.ends_with('\\')
            || unclosed.open()
            || last
                && (self.first_sentence && word.ends_with([PLACEHOLDER, ']'])
                    || self.hard_break_at_end(line))
    }

    /// Whether line `line` ends with two or more spaces after text, before a
    /// line with text that starts no block tag.
    fn hard_break_at_end(&self, line: u32) -> bool {
        let source = self.lines[line as usize].text;
        source.trim_end_matches(['\t', ' ']).len() + 2 <= source.len()
            && source[source.trim_end_matches(['\t', ' ']).len()..].starts_with("  ")
            && self
                .lines
                .get(line as usize + 1)
                .is_some_and(|next| !next.is_blank())
            && !self.layout.is_tag_line(line + 1)
    }

    /// Prints the lines before `line` that are not printed yet, as written.
    fn verbatim_until(&mut self, line: u32) {
        while self.next < line && (self.next as usize) < self.lines.len() {
            // The blank lines that start a run keep their whitespace: javadoc
            // ends the first sentence at the first blank line, and keeps what
            // comes before it.
            if self.next < self.first_text {
                self.layout.exact_line(self.next);
            } else {
                self.layout.verbatim_lines(self.next, self.next);
            }
            self.next += 1;
        }
    }

    fn block_tag(&mut self, tag: &BlockTag, first: bool, before_tag: bool) {
        self.first_sentence = false;
        let line = tag.span.start.line;
        if first {
            // One blank line before the first block tag, and the source's
            // blank lines. Not before a tag printed as written with code's
            // indentation, which javadoc reads as a tag only when no blank
            // line comes before it.
            let blank_before = line > 0 && self.lines[line as usize - 1].is_blank();
            let indented = tag.body.is_none()
                && columns(&self.lines[line as usize].text[..tag.span.start.off as usize]) >= 4;
            self.verbatim_until(line);
            if !blank_before && !indented {
                self.layout.blank();
            }
        } else {
            self.verbatim_until(line);
        }
        let Some(body) = &tag.body else {
            self.verbatim_until(tag.span.end.line + 1);
            return;
        };
        self.layout.line(false, 0, 0);
        for (index, &head) in tag.head.iter().enumerate() {
            for (part, span) in crate::html::head_parts(self.lines, head)
                .into_iter()
                .enumerate()
            {
                let join = if index == 0 && part == 0 {
                    Join::Wrap
                } else {
                    Join::Space
                };
                self.layout.word(span, join, false);
            }
        }
        let head_end = tag.head.last().map_or(tag.span.start, |head| head.end);
        self.next = head_end.line + 1;
        self.section(body, Some(head_end.line), before_tag);
    }

    /// Lays out a part: the description (`tag_line` `None`), or a block
    /// tag's text, whose head is printed on `tag_line`; `before_tag` when a
    /// block tag follows it.
    fn section(&mut self, nodes: &[Node], tag_line: Option<u32>, before_tag: bool) {
        let Some(text) = self.text(nodes) else {
            return;
        };
        let mut kept = Vec::new();
        let mut open = Open::default();
        let blocks = Self::blocks(&text, &mut kept, &mut open);
        // Lines of the part that no block holds (link reference
        // definitions, whose titles may span lines) keep their ends.
        let mut gap = text.pos(0).line;
        let gaps = blocks
            .iter()
            .map(|block| block.lines.clone())
            .chain(std::iter::once({
                let end = text.end(text.text.len()).line + 1;
                end..end
            }));
        for lines in gaps {
            for line in gap..lines.start {
                if !self.lines[line as usize].is_blank() {
                    kept.push(line..line + 1);
                }
            }
            gap = gap.max(lines.end);
        }
        self.kept_as_written |= open.fence && before_tag || open.tag;
        for lines in kept {
            self.layout.keep_line_ends(lines);
        }
        // Text on the tag's line that no block holds (a link reference
        // definition) follows the head as written.
        let start = text.pos(0);
        if tag_line == Some(start.line)
            && blocks
                .first()
                .is_none_or(|block| block.lines.start != start.line)
        {
            let line = self.lines[start.line as usize].text;
            let end = Pos {
                line: start.line,
                off: to_u32(line.len()),
            };
            self.layout.word(Span { start, end }, Join::Space, false);
        }
        // The lines read for the end of the first sentence: javadoc's
        // pattern looks for a line break, so not the description's first.
        let mut checked = start.line + 1;
        let mut index = 0;
        while index < blocks.len() {
            // Blocks that share a line are printed as written together.
            let mut lines = blocks[index].lines.clone();
            let mut end = index + 1;
            while end < blocks.len() && blocks[end].lines.start < lines.end {
                lines.end = lines.end.max(blocks[end].lines.end);
                end += 1;
            }
            let on_tag_line = tag_line == Some(lines.start);
            if !on_tag_line {
                self.verbatim_until(lines.start);
            }
            if tag_line.is_none() {
                self.end_first_sentence(&mut checked, lines.start + 1);
            }
            match &blocks[index].paragraph {
                Some(paragraph) if end == index + 1 && Self::fills(&text, paragraph) => {
                    self.paragraph(&text, paragraph, on_tag_line);
                }
                _ if on_tag_line => {
                    // The tag's text, from its start to the end of its
                    // first lines, as written after the head.
                    let start = text.pos(0);
                    let last = lines.end - 1;
                    let end_pos = Pos {
                        line: last,
                        off: u32::try_from(self.lines[last as usize].text.len())
                            .unwrap_or(u32::MAX),
                    };
                    self.layout.word(
                        Span {
                            start,
                            end: end_pos,
                        },
                        Join::Space,
                        false,
                    );
                }
                _ => {
                    if self.first_sentence {
                        // javadoc's first sentence ends at the line after
                        // an inline tag only when whitespace comes between.
                        self.layout.keep_line_ends(lines.clone());
                    }
                    self.layout.verbatim_lines(lines.start, lines.end - 1);
                }
            }
            if tag_line.is_none() {
                self.end_first_sentence(&mut checked, lines.end);
            }
            self.next = self.next.max(lines.end);
            index = end;
        }
    }

    /// The Markdown text of a part, as javadoc builds it, or `None` when it
    /// is empty.
    fn text<'n>(&self, nodes: &'n [Node]) -> Option<Text<'n>> {
        let mut text = Text {
            text: String::new(),
            segments: Vec::new(),
            newlines: Vec::new(),
            ends_mid_line: false,
        };
        if let Some(Node::Text(span)) = nodes.last() {
            text.ends_mid_line =
                (span.end.off as usize) < self.lines[span.end.line as usize].text.len();
        }
        for node in nodes {
            match node {
                Node::Text(span) => {
                    for line in span.start.line..=span.end.line {
                        let source = self.lines[line as usize].text;
                        let from = if line == span.start.line {
                            span.start.off as usize
                        } else {
                            0
                        };
                        let to = if line == span.end.line {
                            span.end.off as usize
                        } else {
                            source.len()
                        };
                        text.segments.push((
                            text.text.len(),
                            Segment::Text {
                                pos: Pos {
                                    line,
                                    off: to_u32(from),
                                },
                            },
                        ));
                        text.text.push_str(&source[from..to]);
                        if line < span.end.line {
                            text.newlines.push(text.text.len());
                            text.text.push('\n');
                        }
                    }
                }
                Node::Atom(span) | Node::Erroneous(span) | Node::Element { span, .. } => {
                    text.segments.push((
                        text.text.len(),
                        Segment::Placeholder {
                            span: *span,
                            inline: None,
                            erroneous: matches!(node, Node::Erroneous(_)),
                        },
                    ));
                    text.text.push(PLACEHOLDER);
                }
                Node::Inline(inline) => {
                    let name = &self.lines[inline.name.start.line as usize].text
                        [inline.name.start.off as usize..inline.name.end.off as usize];
                    let start = text.text.len();
                    text.segments.push((
                        start,
                        Segment::Placeholder {
                            span: inline.span,
                            inline: Some(inline),
                            erroneous: false,
                        },
                    ));
                    if name == "{@snippet" {
                        // A block placeholder (`<!--￼-->`), on a line of its own.
                        if !text.text.is_empty() && !text.text.ends_with('\n') {
                            text.newlines.push(text.text.len());
                            text.text.push('\n');
                        }
                        text.text.push_str("<!--\u{fffc}-->");
                    } else {
                        text.text.push(PLACEHOLDER);
                    }
                }
            }
        }
        (!text.text.is_empty()).then_some(text)
    }

    /// The top-level blocks of a part's text. A paragraph inside an element
    /// that keeps whitespace (`<pre>`, opened by an HTML block before it) is
    /// no paragraph to fill. Adds to `kept` the lines whose trailing
    /// whitespace is content: those that end inside a `<pre>` or raw text
    /// element, and those of a code block.
    fn blocks(text: &Text, kept: &mut Vec<Range<u32>>, open: &mut Open) -> Vec<Block> {
        let mut blocks: Vec<Block> = Vec::new();
        let mut depth = 0usize;
        let mut paragraph: Option<Paragraph> = None;
        // HTML's reading so far, where the element that keeps whitespace
        // opened in the paragraph being read, and where it opened in the
        // part.
        let mut reader = Reader::new();
        let mut exact_start: Option<usize> = None;
        let mut kept_start: Option<usize> = None;
        // javadoc reads `[text][ref]` and `[ref]` as links to program
        // elements even without a definition: keep them whole.
        let links = |_: BrokenLink<'_>| Some((CowStr::from(""), CowStr::from("")));
        let parser =
            Parser::new_with_broken_link_callback(&text.text, Options::ENABLE_TABLES, Some(links))
                .into_offset_iter();
        for (event, range) in parser {
            match event {
                Event::Start(tag) => {
                    if let Tag::CodeBlock(kind) = &tag {
                        kept.push(Self::block(text, &range).lines);
                        open.fence |= matches!(kind, CodeBlockKind::Fenced(_))
                            && range.end == text.text.len()
                            && !closes_fence(&text.text[range.clone()]);
                    }
                    if depth == 0 {
                        if matches!(tag, Tag::Paragraph) && !reader.exact() {
                            paragraph = Some(Paragraph {
                                range: range.clone(),
                                atoms: Vec::new(),
                                breaks: Vec::new(),
                            });
                        }
                        blocks.push(Self::block(text, &range));
                    } else if let Some(paragraph) = &mut paragraph
                        && matches!(tag, Tag::Link { .. } | Tag::Image { .. })
                        && exact_start.is_none()
                    {
                        paragraph.atoms.push(range);
                    }
                    depth += 1;
                }
                Event::End(end) => {
                    depth -= 1;
                    // An HTML block that ends inside a tag (at a blank line):
                    // javadoc's HTML reads the tag on into what follows, as
                    // rendered, which this reading doesn't see.
                    open.tag |= end == TagEnd::HtmlBlock && !matches!(reader.lex, Lex::Text);
                    if depth == 0
                        && end == TagEnd::Paragraph
                        && let Some(mut done) = paragraph.take()
                    {
                        if let Some(start) = exact_start.take() {
                            // Up to the paragraph's last character: the
                            // line break after it is no part of a word.
                            let end = text.text[..done.range.end].trim_end().len();
                            done.atoms.push(start..end.max(start));
                        }
                        done.atoms.sort_by_key(|atom| atom.start);
                        if let Some(block) = blocks.last_mut() {
                            block.paragraph = Some(done);
                        }
                    }
                }
                Event::Code(_) | Event::InlineMath(_) | Event::DisplayMath(_) => {
                    // A code span's whitespace is content, at the ends of its
                    // lines too.
                    kept.push(text.pos(range.start).line..text.end(range.end).line);
                    if let Some(paragraph) = &mut paragraph
                        && exact_start.is_none()
                    {
                        paragraph.atoms.push(range);
                    }
                }
                Event::Html(html) | Event::InlineHtml(html) => {
                    let before = reader.exact();
                    reader.read_lines(&html, range.start, text, kept);
                    let exact = reader.exact();
                    if !before && exact {
                        kept_start = Some(range.start);
                    } else if before
                        && !exact
                        && let Some(start) = kept_start.take()
                    {
                        kept.push(text.pos(start).line..text.end(range.end).line);
                    }
                    if depth == 0 {
                        blocks.push(Self::block(text, &range));
                    } else if let Some(paragraph) = &mut paragraph {
                        // An element that keeps whitespace is an atom up to
                        // its end tag (or the paragraph's end).
                        if !before && exact {
                            exact_start = Some(range.start);
                        } else if before && !exact {
                            if let Some(start) = exact_start.take() {
                                paragraph.atoms.push(start..range.end);
                            }
                        } else if !exact {
                            paragraph.atoms.push(range);
                        }
                    }
                }
                Event::HardBreak => {
                    if let Some(paragraph) = &mut paragraph {
                        paragraph.breaks.push(range.start);
                    }
                }
                Event::Rule if depth == 0 => blocks.push(Self::block(text, &range)),
                _ => {}
            }
        }
        if let Some(start) = kept_start {
            kept.push(text.pos(start).line..text.end(text.text.len()).line + 1);
        }
        blocks
    }

    /// A block's source lines. The indentation of a block tag that ends the
    /// part's text, after its last line break, is on the tag's line, which
    /// is no line of the block.
    fn block(text: &Text, range: &Range<usize>) -> Block {
        let mut end = range.end;
        if end == text.text.len()
            && text.ends_mid_line
            && let Some(newline) = text.newline_before(end)
            && newline >= range.start
            && text.text[newline + 1..end]
                .bytes()
                .all(|byte| byte == b' ' || byte == b'\t')
        {
            end = newline;
        }
        while end > range.start && text.text.as_bytes()[end - 1] == b'\n' {
            end -= 1;
        }
        let first = text.pos(range.start).line;
        let last = text.end(end.max(range.start + 1)).line;
        Block {
            lines: first..last.max(first) + 1,
            paragraph: None,
        }
    }

    /// Whether a paragraph can be filled: no hard break comes before a word
    /// that could start a block. The word after a hard break starts a line,
    /// where such a word would end the paragraph; its source indentation
    /// (four spaces before `***`) is what keeps it text.
    ///
    /// Nor does a paragraph that starts like a link reference definition
    /// (`[foo]: /url "title"` with text after the title): a line break
    /// after its destination or title could make it one. Nor one with a hard
    /// break and an entity: after a hard break, javadoc reads the line break
    /// after an entity that ends its line as another hard break. Nor one
    /// whose first word is made only of `*`, `-` or `_`: its first line
    /// could fill with such words alone, a thematic break (`** *`). Nor one
    /// with a line that starts like a table's delimiter row (`|---|---|`),
    /// which a Markdown reader that doesn't count cells as this one does
    /// reads as a table.
    fn fills(text: &Text, paragraph: &Paragraph) -> bool {
        let body = &text.text[paragraph.range.clone()];
        let first_word = body.split_whitespace().next().unwrap_or("");
        let delimiter_row = body
            .lines()
            .skip(1)
            .any(|line| words::is_delimiter_row(line.split_whitespace().next().unwrap_or("")));
        if body.trim_start().starts_with('[') && body.contains("]:")
            || !paragraph.breaks.is_empty() && body.contains('&')
            || first_word.bytes().all(|byte| b"*-_".contains(&byte))
            || delimiter_row
        {
            return false;
        }
        paragraph.breaks.iter().all(|&found| {
            let Some(newline) = text.newline_after(found) else {
                return true;
            };
            let rest = &text.text[newline + 1..paragraph.range.end];
            let word = rest
                .trim_start()
                .split([' ', '\t', '\n', '\r'])
                .next()
                .unwrap_or("");
            !words::starts_structure(word, true)
        })
    }

    /// Fills a paragraph: its words, split at whitespace outside atoms, on
    /// lines flush with the frame (after the head when `on_tag_line`).
    fn paragraph(&mut self, text: &Text, paragraph: &Paragraph, on_tag_line: bool) {
        let source = text.text.as_bytes();
        let mut at = paragraph.range.start;
        let end = paragraph.range.end;
        let mut atoms = paragraph.atoms.iter().peekable();
        let mut first = true;
        let mut previous_backslash = false;
        let mut breaks = paragraph.breaks.iter().copied().peekable();
        let mut break_pending = false;
        let mut after_error = false;
        // A link's text or an inline link's destination and title that a
        // word opened (a `[` or `](` not closed yet).
        let mut unclosed = LinkScan::default();
        // The output line starts with a word that could start a block there
        // (`*` alone, which can't interrupt the paragraph): a word from a
        // later source line joined onto it could make it one (`* foo`), or
        // make javac's reading of its list markers run away.
        let mut after_structure = false;
        // The end of the last word.
        let mut word_end = 0;
        while at < end {
            while at < end && matches!(source[at], b' ' | b'\t' | b'\n' | b'\r') {
                at += 1;
            }
            if at >= end {
                break;
            }
            let start = at;
            at = end_of_word(text, at, end, &mut atoms, &mut unclosed);
            let word = &text.text[start..at];
            let mut span = Span {
                start: text.pos(start),
                end: text.end(at),
            };
            // A hard break after the word: its trailing spaces or `\` stay,
            // and the next word starts a line.
            let mut hard = false;
            while breaks.peek().is_some_and(|&found| found < start) {
                breaks.next();
            }
            let line_end = text.newline_after(at).unwrap_or(end);
            let last_on_line = text.text[at..line_end]
                .bytes()
                .all(|byte| byte == b' ' || byte == b'\t');
            if last_on_line && breaks.peek().is_some_and(|&found| found <= line_end) {
                breaks.next();
                hard = true;
            }
            let last = text.text[at..end].trim().is_empty();
            if last_on_line && (hard || self.keeps_end(word, span.end.line, last, unclosed)) {
                span.end.off = to_u32(self.lines[span.end.line as usize].text.len());
            }
            // The spaces before the word on its line, when it starts one (no
            // word since the line's start, and only spaces and tabs).
            let newline = text.newline_before(start);
            let line_start = newline.map_or(start == paragraph.range.start, |found| {
                word_end <= found
                    && text.text[found + 1..start]
                        .bytes()
                        .all(|byte| byte == b' ' || byte == b'\t')
            });
            let indent = newline.map_or(0, |found| start - found - 1);
            // In the first sentence, javadoc ends the sentence at a line that
            // starts with one of these; an `@` that starts a line is a block
            // tag or an escape: such a word keeps the line break before it.
            let at_sign = self.layout.text_at(span.start).starts_with('@');
            // javadoc reads a `.` inside `<…>`, `[…]`, `(…)` or a code span
            // as a `-` when it looks for the sentence's end.
            let ends_sentence =
                self.first_sentence && (words::ends_first_paragraph(word) || word.starts_with('.'));
            // A word that starts a line keeps its line break when it ends
            // javadoc's first sentence there, is a block tag or an escape
            // (`@`), follows a `\` (which javac reads with the line break),
            // or is one that Markdown readers don't agree on there.
            let keeps_break = !first
                && line_start
                && (ends_sentence
                    && indent <= 3
                    && !text.text[start - indent..start].contains('\t')
                    || at_sign
                    || previous_backslash
                    || words::keeps_line_start(word));
            let sticky = previous_backslash
                || at_sign
                || ends_sentence
                || words::starts_structure(word, true);
            // The separator after an erroneous node keeps its kind.
            let keeps_break = keeps_break || (after_error || after_structure) && line_start;
            let sticky = sticky || after_error || after_structure;
            // The columns of the word's indentation, when it starts a line.
            let source_indent = if line_start {
                columns(&text.text[start - indent..start])
            } else {
                0
            };
            let (join, sticky) = if first {
                self.first_word(word, on_tag_line, ends_sentence, source_indent)
            } else if break_pending || keeps_break {
                // A line break kept from the source keeps the word's
                // indentation after it: there it can keep the word from
                // starting a block (`    ***`) or ending javadoc's first
                // sentence, and after a `\` that ends a line, javac reads the
                // next line's first character with the escape.
                self.layout.hard_break(source_indent);
                (Join::Wrap, false)
            } else {
                (Join::Wrap, sticky)
            };
            match text.inline_at(start).filter(|_| word == "\u{fffc}") {
                Some(inline) if self.fills_label(inline) => self.label(inline, join, sticky),
                _ => self.layout.word(span, join, sticky),
            }
            after_error = text.erroneous_before(at);
            if first || keeps_break || break_pending {
                after_structure = line_start && words::starts_structure(word, true);
            }
            word_end = at;
            break_pending = hard;
            first = false;
            previous_backslash = word.ends_with('\\');
        }
    }

    /// The words of the plain text of `nodes`, each with whether it starts
    /// a source line.
    fn label_words(&self, nodes: &[Node]) -> Vec<(Span, bool)> {
        let mut words = Vec::new();
        for node in nodes {
            let Node::Text(span) = node else { continue };
            for line in span.start.line..=span.end.line {
                let text = self.lines[line as usize].text;
                let from = if line == span.start.line {
                    span.start.off as usize
                } else {
                    0
                };
                let to = if line == span.end.line {
                    span.end.off as usize
                } else {
                    text.len()
                };
                let mut at = from;
                let bytes = text.as_bytes();
                let mut line_start = line != span.start.line;
                while at < to {
                    while at < to && matches!(bytes[at], b' ' | b'\t') {
                        at += 1;
                    }
                    let start = at;
                    while at < to && !matches!(bytes[at], b' ' | b'\t') {
                        at += 1;
                    }
                    if at > start {
                        words.push((
                            Span {
                                start: Pos {
                                    line,
                                    off: to_u32(start),
                                },
                                end: Pos {
                                    line,
                                    off: to_u32(at),
                                },
                            },
                            line_start,
                        ));
                        line_start = false;
                    }
                }
            }
        }
        words
    }

    /// Whether the label (or description) of an inline tag is filled: plain
    /// text, without the Markdown that could change when its line breaks
    /// move (code spans, links, HTML, escapes, a hard break, a blank line)
    /// and without a line that starts with a word that starts structure.
    fn fills_label(&self, inline: &Inline) -> bool {
        let Some(rich) = &inline.rich else {
            return false;
        };
        rich.close.is_some()
            && rich.content.iter().all(|node| match node {
                Node::Text(span) => !self.breaks_lines(*span),
                _ => false,
            })
            && self
                .label_words(&rich.content)
                .iter()
                .all(|&(span, line_start)| {
                    let word = self.layout.text_at(span.start);
                    let word = &word[..(span.end.off - span.start.off) as usize];
                    !word.contains(['`', '<', '[', '\\', '{', '&', '*', '_'])
                        && !(line_start
                            && (words::starts_structure(word, true)
                                || words::ends_first_paragraph(word)
                                || word.starts_with('@')))
                })
    }

    /// Whether Markdown text has a line break that a fill would lose: a
    /// line that ends with two spaces (a hard break) or a blank line (which
    /// ends a paragraph) before its last line.
    fn breaks_lines(&self, span: Span) -> bool {
        (span.start.line..span.end.line).any(|line| {
            let text = self.lines[line as usize].text;
            let from = if line == span.start.line {
                span.start.off as usize
            } else {
                0
            };
            let part = &text[from..];
            part.ends_with("  ")
                || line > span.start.line && part.trim_matches(crate::layout::BLANK).is_empty()
        })
    }

    /// Lays out an inline tag whose label is filled: its name and head
    /// glued, the label's words, and its `}`.
    fn label(&mut self, inline: &Inline, join: Join, sticky: bool) {
        let Some(rich) = &inline.rich else { return };
        self.layout.word(inline.name, join, sticky);
        if let Some(head) = rich.head {
            // A reference may be joined onto one line; an `{@index}` term is
            // text that keeps its line breaks, as in the Javadoc layout.
            let index = self
                .layout
                .text_at(inline.name.start)
                .starts_with("{@index");
            let joined = (!index)
                .then(|| words::joined_pieces(self.lines, head))
                .flatten();
            for span in joined.unwrap_or_else(|| vec![head]) {
                self.layout.word(span, Join::Space, false);
            }
        }
        let words = self.label_words(&rich.content);
        let mut last = rich.head.map_or(inline.name.end, |head| head.end);
        for &(span, _) in &words {
            let word = self.layout.text_at(span.start);
            let word = &word[..(span.end.off - span.start.off) as usize];
            let sticky = words::starts_structure(word, true) || word.starts_with('@');
            self.layout.word(span, Join::Wrap, sticky);
            last = span.end;
        }
        if let Some(close) = rich.close {
            let join = if close.start == last {
                Join::None
            } else {
                Join::Space
            };
            self.layout.word(close, join, false);
        }
    }
}
