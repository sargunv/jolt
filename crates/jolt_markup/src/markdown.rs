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

use pulldown_cmark::{BrokenLink, CowStr, Event, Options, Parser, Tag, TagEnd};

use crate::frame::Line;
use crate::layout::{Join, Layout};
use crate::parser::{BlockTag, Comment, Inline, Node, Pos, Span, to_u32};
use crate::words;

/// Whether an element that keeps whitespace (`<pre>`, `<script>`, `<style>`,
/// `<textarea>`) is open after the HTML `html`, when `open` before it.
fn keeps_whitespace_after(html: &str, mut open: bool) -> bool {
    let lower = html.to_ascii_lowercase();
    let mut rest = lower.as_str();
    while let Some(at) = rest.find('<') {
        rest = &rest[at + 1..];
        let closing = rest.starts_with('/');
        let name = rest.trim_start_matches('/');
        let named = ["pre", "script", "style", "textarea"].iter().any(|tag| {
            name.strip_prefix(tag).is_some_and(|after| {
                !after.starts_with(|ch: char| ch.is_ascii_alphanumeric() || ch == '-')
            })
        });
        if named {
            open = !closing;
        }
    }
    open
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
    };
    markdown.section(&comment.body, None);
    for (index, tag) in comment.tags.iter().enumerate() {
        markdown.block_tag(tag, index == 0);
    }
    let last = to_u32(lines.len() - 1);
    if let Some(tail) = comment.tail {
        markdown.verbatim_until(tail.start.line);
    }
    markdown.verbatim_until(last + 1);
    Some(())
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
struct Text<'n> {
    text: String,
    /// The start of each piece in `text`, in order.
    segments: Vec<(usize, Segment<'n>)>,
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
    /// the description's first block. (javadoc ends the sentence at a `.` and
    /// whitespace outside code spans, links and brackets, or at a line that
    /// starts with one of [`words::ends_first_paragraph`].)
    first_sentence: bool,
}

impl Markdown<'_, '_, '_, '_> {
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

    fn block_tag(&mut self, tag: &BlockTag, first: bool) {
        self.first_sentence = false;
        let line = tag.span.start.line;
        if first {
            // One blank line before the first block tag, and the source's
            // blank lines.
            let blank_before = line > 0 && self.lines[line as usize - 1].is_blank();
            self.verbatim_until(line);
            if !blank_before {
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
            let join = if index == 0 { Join::Wrap } else { Join::Space };
            self.layout.word(head, join, false);
        }
        let head_end = tag.head.last().map_or(tag.span.start, |head| head.end);
        self.next = head_end.line + 1;
        self.section(body, Some(head_end.line));
    }

    /// Lays out a part: the description (`tag_line` `None`), or a block
    /// tag's text, whose head is printed on `tag_line`.
    fn section(&mut self, nodes: &[Node], tag_line: Option<u32>) {
        let Some(text) = self.text(nodes) else {
            return;
        };
        let blocks = Self::blocks(&text);
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
            match &blocks[index].paragraph {
                Some(paragraph) if end == index + 1 => {
                    self.paragraph(&text, paragraph, on_tag_line);
                    self.first_sentence = false;
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
                _ => self.layout.verbatim_lines(lines.start, lines.end - 1),
            }
            self.first_sentence = false;
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
        };
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
    /// no paragraph to fill.
    fn blocks(text: &Text) -> Vec<Block> {
        let mut blocks: Vec<Block> = Vec::new();
        let mut depth = 0usize;
        let mut paragraph: Option<Paragraph> = None;
        // Whether an element that keeps whitespace is open, and where it
        // opened in the paragraph being read.
        let mut exact = false;
        let mut exact_start: Option<usize> = None;
        // javadoc reads `[text][ref]` and `[ref]` as links to program
        // elements even without a definition: keep them whole.
        let links = |_: BrokenLink<'_>| Some((CowStr::from(""), CowStr::from("")));
        let parser =
            Parser::new_with_broken_link_callback(&text.text, Options::ENABLE_TABLES, Some(links))
                .into_offset_iter();
        for (event, range) in parser {
            match event {
                Event::Start(tag) => {
                    if depth == 0 {
                        if matches!(tag, Tag::Paragraph) && !exact {
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
                    if depth == 0
                        && end == TagEnd::Paragraph
                        && let Some(mut done) = paragraph.take()
                    {
                        if let Some(start) = exact_start.take() {
                            done.atoms.push(start..done.range.end);
                        }
                        done.atoms.sort_by_key(|atom| atom.start);
                        if let Some(block) = blocks.last_mut() {
                            block.paragraph = Some(done);
                        }
                    }
                }
                Event::Code(_) | Event::InlineMath(_) | Event::DisplayMath(_) => {
                    if let Some(paragraph) = &mut paragraph
                        && exact_start.is_none()
                    {
                        paragraph.atoms.push(range);
                    }
                }
                Event::Html(html) | Event::InlineHtml(html) => {
                    let before = exact;
                    exact = keeps_whitespace_after(&html, exact);
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
        blocks
    }

    /// A block's source lines.
    fn block(text: &Text, range: &Range<usize>) -> Block {
        let mut end = range.end;
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
        while at < end {
            while at < end && matches!(source[at], b' ' | b'\t' | b'\n' | b'\r') {
                at += 1;
            }
            if at >= end {
                break;
            }
            let start = at;
            loop {
                while atoms.peek().is_some_and(|atom| atom.end <= at) {
                    atoms.next();
                }
                if let Some(atom) = atoms.peek().filter(|atom| atom.start <= at) {
                    at = atom.end.max(at + 1);
                    continue;
                }
                if at >= end || matches!(source[at], b' ' | b'\t' | b'\n' | b'\r') {
                    break;
                }
                at += text.text[at..].chars().next().map_or(1, char::len_utf8);
            }
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
            let line_end = text.text[at..].find('\n').map_or(end, |found| at + found);
            let last_on_line = text.text[at..line_end]
                .bytes()
                .all(|byte| byte == b' ' || byte == b'\t');
            if last_on_line && breaks.peek().is_some_and(|&found| found <= line_end) {
                breaks.next();
                hard = true;
                let line = self.lines[span.end.line as usize].text;
                span.end.off = to_u32(line.len());
            }
            // The spaces before the word on its line, when it starts one.
            let line_start =
                text.text[..start]
                    .rfind('\n')
                    .map_or(start == paragraph.range.start, |found| {
                        text.text[found + 1..start]
                            .bytes()
                            .all(|byte| byte == b' ' || byte == b'\t')
                    });
            let indent = text.text[..start]
                .rfind('\n')
                .map_or(0, |found| start - found - 1);
            // In the first sentence, javadoc ends the sentence at a line that
            // starts with one of these; an `@` that starts a line is a block
            // tag or an escape: such a word keeps the line break before it.
            let at_sign = self.layout.text_at(span.start).starts_with('@');
            let ends_sentence = self.first_sentence && words::ends_first_paragraph(word);
            let keeps_break = !first
                && line_start
                && (ends_sentence
                    && indent <= 3
                    && !text.text[start - indent..start].contains('\t')
                    || at_sign);
            let sticky = previous_backslash
                || at_sign
                || ends_sentence
                || words::starts_structure(word, true);
            // The separator after an erroneous node keeps its kind.
            let keeps_break = keeps_break || after_error && line_start;
            let sticky = sticky || after_error;
            let (join, sticky) = if first {
                if on_tag_line {
                    (Join::Space, false)
                } else {
                    self.layout.line(false, 0, 0);
                    (Join::Wrap, false)
                }
            } else if break_pending || keeps_break {
                self.layout.hard_break();
                (Join::Wrap, false)
            } else {
                (Join::Wrap, sticky)
            };
            match text.inline_at(start).filter(|_| word == "\u{fffc}") {
                Some(inline) if self.fills_label(inline) => self.label(inline, join, sticky),
                _ => self.layout.word(span, join, sticky),
            }
            after_error = text.erroneous_before(at);
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
    /// move (code spans, links, HTML, escapes) and without a line that
    /// starts with a word that starts structure.
    fn fills_label(&self, inline: &Inline) -> bool {
        let Some(rich) = &inline.rich else {
            return false;
        };
        rich.close.is_some()
            && rich
                .content
                .iter()
                .all(|node| matches!(node, Node::Text(_)))
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

    /// Lays out an inline tag whose label is filled: its name and head
    /// glued, the label's words, and its `}`.
    fn label(&mut self, inline: &Inline, join: Join, sticky: bool) {
        let Some(rich) = &inline.rich else { return };
        self.layout.word(inline.name, join, sticky);
        if let Some(head) = rich.head {
            self.layout.word(head, Join::Space, false);
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
