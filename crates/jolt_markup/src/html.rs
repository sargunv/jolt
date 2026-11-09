//! The body parser of Javadoc comments (`/**`): HTML-lite, in the style of
//! google-java-format's Javadoc lexer.
//!
//! It walks the content lines once, recognizing block tags at line starts,
//! inline tags (brace-balanced, first), block-level HTML elements anywhere
//! outside inline tags, and verbatim regions, and hands lines and words to
//! the [`Layout`]. HTML collapses whitespace outside `<pre>`, so the line
//! breaks and blank lines it chooses don't change the rendered page.

use std::cell::Cell;

use crate::frame::Line;
use crate::layout::{Join, Layout};
use crate::tags::{self, Arg, InlineTags, SPACES, is_space};

/// The block-level elements, matched without regard to case.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Name {
    P,
    /// `<ul>`, `<ol>`, `<dl>`.
    List,
    /// `<li>`, `<dt>`, `<dd>`.
    Item,
    Pre,
    Table,
    Blockquote,
    /// `<h1>` to `<h6>`.
    Heading,
    Br,
}

/// A block-level element's tag.
#[derive(Clone, Copy, Debug)]
struct Element {
    name: Name,
    closing: bool,
    /// The line and byte after the tag's `>`, which may be on a later line;
    /// the end of the tag's line when a verbatim region's tag has no `>`.
    end: (usize, usize),
    /// A quoted attribute value holds a line break of the tag.
    literal: bool,
}

/// The block-level element name at the start of `name`, and its length.
fn element_name(name: &str) -> Option<(Name, usize)> {
    let bytes = name.as_bytes();
    let len = bytes
        .iter()
        .take_while(|byte| byte.is_ascii_alphanumeric())
        .count();
    let after = bytes.get(len);
    if !after.is_none_or(|&byte| matches!(byte, b'>' | b'/') || byte.is_ascii_whitespace()) {
        return None;
    }
    let name = &name[..len];
    let is = |candidate: &str| name.eq_ignore_ascii_case(candidate);
    let name = if is("p") {
        Name::P
    } else if is("ul") || is("ol") || is("dl") {
        Name::List
    } else if is("li") || is("dt") || is("dd") {
        Name::Item
    } else if is("pre") {
        Name::Pre
    } else if is("table") {
        Name::Table
    } else if is("blockquote") {
        Name::Blockquote
    } else if is("br") {
        Name::Br
    } else if name.len() == 2
        && name.as_bytes()[0].eq_ignore_ascii_case(&b'h')
        && matches!(name.as_bytes()[1], b'1'..=b'6')
    {
        Name::Heading
    } else {
        return None;
    };
    Some((name, len))
}

/// How a unit of lines starts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Lead {
    Text,
    /// A block tag.
    Tag,
    /// A `<p>`.
    Paragraph,
}

/// The byte offset of `needle` (lowercase ASCII) in `text`, ignoring ASCII
/// case.
fn find_ignoring_case(text: &str, needle: &str) -> Option<usize> {
    let needle = needle.as_bytes();
    text.as_bytes()
        .windows(needle.len())
        .position(|window| window.eq_ignore_ascii_case(needle))
}

/// How a word was separated from the one before it in the source.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Sep {
    /// Nothing: a word split at an element or inline tag.
    None,
    /// Spaces on the same line.
    Space,
    /// A line break.
    Line,
}

/// Why the next word starts a line, from weakest to strongest.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Kind {
    /// A line break inside a paragraph (`<br>`, a list-like line, a ruler).
    Break,
    /// A new paragraph of text (after a blank line or a block).
    Text,
    /// A block element.
    Block,
}

#[derive(Clone, Copy, Debug)]
struct NewLine {
    kind: Kind,
    /// Asked for in the middle of a source line (after an element or a
    /// region on it).
    mid_line: bool,
}

/// What lies between the last word and the next, from weakest to strongest.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Gap {
    None,
    /// A block ended: a list, a heading or a verbatim region.
    Block,
    /// A blank line of the source.
    Blank,
}

/// An open block that indents the lines inside it.
#[derive(Clone, Copy, Debug)]
enum Frame {
    /// Block tag text: its lines are indented 4.
    Tag,
    /// A list: its `<ul>` line at `open`, its items at `column`.
    List { open: u16, column: u16 },
    /// A list item: its tag at `line`, its other lines at `column`.
    Item { line: u16, column: u16 },
}

impl Frame {
    const fn column(self) -> u16 {
        match self {
            Self::Tag => 4,
            Self::List { column, .. } | Self::Item { column, .. } => column,
        }
    }
}

/// Lays out a Javadoc comment's content lines.
pub(crate) fn parse<'a>(lines: &[Line<'a>], layout: &mut Layout<'_, 'a>) {
    let mut parser = Parser {
        lines,
        tags: InlineTags::new(lines),
        layout,
        frames: Vec::new(),
        in_tags: false,
        tag_end: 0,
        inline_arg: Arg::None,
        block_arg: Arg::None,
        next_line: None,
        gap: Gap::None,
        attach: false,
        no_quote: Cell::new([usize::MAX; 2]),
    };
    parser.run();
}

struct Parser<'p, 'l, 'a> {
    lines: &'p [Line<'a>],
    tags: InlineTags,
    layout: &'p mut Layout<'l, 'a>,
    frames: Vec<Frame>,
    /// Whether a block tag has started.
    in_tags: bool,
    /// The end of the inline tag being lexed, or 0.
    tag_end: usize,
    inline_arg: Arg,
    block_arg: Arg,
    /// The next word starts a line.
    next_line: Option<NewLine>,
    /// What lies between the last word and the next.
    gap: Gap,
    /// The next word follows the last with nothing between (after `<p>` or
    /// `<li>`).
    attach: bool,
    /// For `"` and `'`: the comment offset from which a search for one
    /// found none, so that no later search looks again.
    no_quote: Cell<[usize; 2]>,
}

impl<'a> Parser<'_, '_, 'a> {
    fn run(&mut self) {
        let (mut index, mut pos) = (0, 0);
        let mut unit_start = true;
        while index < self.lines.len() {
            let line = self.lines[index];
            if pos == 0 && self.tag_end <= line.offset {
                if line.text.is_empty() {
                    self.request(Kind::Text, false);
                    self.gap = Gap::Blank;
                    unit_start = true;
                    index += 1;
                    continue;
                }
                let is_tag = tags::is_block_tag(line.text);
                let lead = if is_tag {
                    Lead::Tag
                } else if self.paragraph_len(index).is_some() {
                    Lead::Paragraph
                } else {
                    Lead::Text
                };
                if (unit_start || lead != Lead::Text)
                    && let Some(end) = self.as_written_unit(index, lead)
                {
                    self.as_written(index, end, lead);
                    index = end;
                    unit_start = true;
                    continue;
                }
                unit_start = false;
                if is_tag {
                    self.start_tag();
                } else if index > 0 {
                    if tags::is_ruler(line.text) {
                        self.request(Kind::Break, false);
                        self.emit(line.text, Sep::Line, false);
                        self.request(Kind::Break, false);
                        index += 1;
                        continue;
                    }
                    let first = line.text.split(|ch: char| ch.is_ascii_whitespace()).next();
                    if first.is_some_and(tags::is_list_marker) {
                        self.request(Kind::Break, false);
                    }
                }
            }
            let (next_index, next_pos, region) = self.lex(index, pos);
            if region && next_pos == 0 {
                unit_start = true;
            }
            (index, pos) = (next_index, next_pos);
        }
    }

    /// The block-level element whose tag starts at byte `at` of line `index`,
    /// which is a `<`.
    fn block_element(&self, index: usize, at: usize) -> Option<Element> {
        let text = &self.lines[index].text[at..];
        let closing = text.as_bytes().get(1) == Some(&b'/');
        let start = 1 + usize::from(closing);
        let (name, len) = element_name(&text[start..])?;
        let region = matches!(name, Name::Pre | Name::Table | Name::Blockquote) && !closing;
        let (end, literal) = match self.tag_end(index, at + start + len) {
            Some(end) => end,
            None if region => ((index, self.lines[index].text.len()), false),
            None => return None,
        };
        Some(Element {
            name,
            closing,
            end,
            literal,
        })
    }

    /// The end of the HTML tag whose name ends at byte `at` of line `index`:
    /// the line and byte after its `>`, and whether a quoted attribute value
    /// (a `"` or `'` after `=`) holds a line break. A quoted value may hold
    /// `>`; any other line break inside the tag is whitespace. `None` when an
    /// unquoted `<` or the comment's end comes first.
    fn tag_end(&self, mut index: usize, mut at: usize) -> Option<((usize, usize), bool)> {
        // After `=`, where a quote starts a value.
        let mut value = false;
        let mut literal = false;
        loop {
            let text = self.lines.get(index)?.text.as_bytes();
            let Some(&byte) = text.get(at) else {
                (index, at) = (index + 1, 0);
                continue;
            };
            match byte {
                b'>' => return Some(((index, at + 1), literal)),
                b'<' => return None,
                b'=' => value = true,
                quote @ (b'"' | b'\'') if value => {
                    let open = index;
                    (index, at) = self.closing_quote(quote, index, at + 1)?;
                    literal |= index != open;
                    value = false;
                }
                byte if is_space(byte) => {}
                _ => value = false,
            }
            at += 1;
        }
    }

    /// The line and byte of the first `quote` at or after byte `at` of line
    /// `index`.
    fn closing_quote(&self, quote: u8, mut index: usize, mut at: usize) -> Option<(usize, usize)> {
        let kind = usize::from(quote == b'\'');
        let from = self.lines[index].offset + at;
        let mut no_quote = self.no_quote.get();
        if from >= no_quote[kind] {
            return None;
        }
        while let Some(line) = self.lines.get(index) {
            let rest = line.text.as_bytes().get(at..).unwrap_or_default();
            if let Some(found) = rest.iter().position(|&byte| byte == quote) {
                return Some((index, at + found));
            }
            (index, at) = (index + 1, 0);
        }
        no_quote[kind] = from;
        self.no_quote.set(no_quote);
        None
    }

    /// Whether a block-level element's tag or an HTML comment starts at byte
    /// `at` of line `index`.
    fn starts_block(&self, index: usize, at: usize) -> bool {
        let rest = &self.lines[index].text[at..];
        rest.starts_with("<!--")
            || (rest.starts_with('<') && self.block_element(index, at).is_some())
    }

    /// The length up to the end of the `<p>` that starts line `index` after
    /// spaces and tabs, if one does: the line's length when the tag ends on
    /// a later line.
    fn paragraph_len(&self, index: usize) -> Option<usize> {
        let text = self.lines[index].text;
        let rest = text.trim_start_matches([' ', '\t']);
        if !rest.starts_with('<') {
            return None;
        }
        self.block_element(index, text.len() - rest.len())
            .filter(|element| element.name == Name::P && !element.closing)
            .map(|element| {
                if element.end.0 == index {
                    element.end.1
                } else {
                    text.len()
                }
            })
    }

    /// The column of a new line in the innermost open block.
    fn column(&self) -> u16 {
        self.frames.last().map_or(0, |frame| frame.column())
    }

    /// Asks for a new line before the next word; the strongest reason wins.
    fn request(&mut self, kind: Kind, mid_line: bool) {
        self.next_line = Some(match self.next_line {
            Some(next) if next.kind >= kind => next,
            _ => NewLine { kind, mid_line },
        });
    }

    /// Starts a line at `indent`, with wraps at `hang`. A blank line goes
    /// before it in the description, outside lists, before a block or after
    /// a blank line or a block; elsewhere (in a list or in tag text) only a
    /// blank line of the source is kept.
    fn start(&mut self, kind: Kind, indent: u16, hang: u16) {
        let blank = match kind {
            Kind::Break => false,
            _ if self.frames.is_empty() => kind == Kind::Block || self.gap != Gap::None,
            _ => self.gap == Gap::Blank,
        };
        self.layout.line(blank, indent, hang);
    }

    /// Adds a word. `glue` keeps it on the line of the word before.
    fn emit(&mut self, text: &'a str, sep: Sep, glue: bool) {
        if let Some(next) = self.next_line.take() {
            // A word that a line break would turn into structure stays on
            // the line where the source has it.
            if next.mid_line
                && sep != Sep::Line
                && !self.layout.at_start()
                && tags::starts_structure(text, false)
            {
                self.layout.word(text, Join::Space);
            } else {
                let column = self.column();
                self.start(next.kind, column, column);
                self.layout.word(text, Join::Wrap);
            }
        } else {
            let join = match sep {
                _ if self.attach => Join::None,
                Sep::None => Join::None,
                Sep::Space | Sep::Line if glue => Join::Space,
                Sep::Space | Sep::Line => Join::Wrap,
            };
            self.layout.word(text, join);
        }
        self.attach = false;
        self.gap = Gap::None;
    }

    /// A block tag starts on this line: it closes the open blocks, and a
    /// blank line goes before the first.
    fn start_tag(&mut self) {
        self.frames.clear();
        let blank = !self.in_tags;
        self.in_tags = true;
        self.layout.multiline();
        self.next_line = None;
        self.layout.line(blank, 0, Frame::Tag.column());
        self.frames.push(Frame::Tag);
        self.block_arg = Arg::Name;
        self.attach = false;
        self.gap = Gap::None;
    }

    /// The end of the unit of lines from `index` when it is kept as written,
    /// that is when its lines' indentation differs by two columns or more,
    /// as in ASCII art, a grammar or aligned text (a difference of one is
    /// taken for a typo). A unit is a run of non-blank lines up to a line
    /// that starts a block tag or a block element. Only a unit of plain
    /// lines qualifies, after a block tag or `<p>` that starts it: one with
    /// an inline tag, a block element or a verbatim region is laid out as
    /// usual. The first line of the comment, and a first line that starts
    /// with a block tag or `<p>`, don't count.
    fn as_written_unit(&self, index: usize, lead: Lead) -> Option<usize> {
        let (mut least, mut most) = (usize::MAX, 0);
        let mut end = index;
        while let Some(line) = self.lines.get(end) {
            let text = line.text;
            let rest = text.trim_start_matches([' ', '\t']);
            if rest.is_empty() {
                break;
            }
            let block = self.starts_block(end, text.len() - rest.len());
            if end == index && block && lead == Lead::Text {
                return None;
            }
            if end > index && (block || tags::is_block_tag(rest)) {
                break;
            }
            if !(end == index && (lead != Lead::Text || index == 0)) {
                least = least.min(text.len() - rest.len());
                most = most.max(text.len() - rest.len());
            }
            end += 1;
        }
        if most < least.saturating_add(2) {
            return None;
        }
        let plain = (index..end).all(|line| {
            let text = self.lines[line].text;
            let from = match lead {
                Lead::Paragraph if line == index => self.paragraph_len(index).unwrap_or(0),
                _ => 0,
            };
            !text.contains("{@")
                && !text
                    .bytes()
                    .enumerate()
                    .skip(from)
                    .any(|(at, byte)| byte == b'<' && self.starts_block(line, at))
        });
        plain.then_some(end)
    }

    /// Prints the lines from `index` to `end` as written.
    fn as_written(&mut self, index: usize, end: usize, lead: Lead) {
        if lead == Lead::Tag {
            self.start_tag();
        } else {
            let kind = if lead == Lead::Paragraph {
                Kind::Block
            } else {
                Kind::Text
            };
            self.request(kind, false);
            let column = self.column();
            if let Some(next) = self.next_line.take() {
                self.start(next.kind, column, column);
            }
        }
        for line in &self.lines[index..end] {
            if line.raw.is_empty() {
                self.layout.word(line.text, Join::Wrap);
            } else {
                self.layout.verbatim(line.raw);
            }
        }
        self.request(Kind::Text, false);
        self.block_arg = Arg::None;
        self.gap = Gap::None;
    }

    /// Lexes line `index` from byte `pos`. Returns where to go on, and
    /// whether a verbatim region ended there.
    fn lex(&mut self, index: usize, mut pos: usize) -> (usize, usize, bool) {
        let line = self.lines[index];
        let text = line.text;
        let bytes = text.as_bytes();
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
                return (index + 1, 0, false);
            }
            let offset = line.offset + pos;
            if offset < self.tag_end {
                let end = (pos..bytes.len())
                    .find(|&at| is_space(bytes[at]))
                    .unwrap_or(bytes.len())
                    .min(self.tag_end - line.offset);
                let piece = &text[pos..end];
                let glue = self.inline_arg.glue(piece);
                self.emit(piece, sep, glue);
                pos = end;
                sep = Sep::None;
                if line.offset + pos >= self.tag_end {
                    self.tag_end = 0;
                    self.inline_arg = Arg::None;
                }
                continue;
            }
            if text[pos..].starts_with("{@")
                && let Some(end) = self.tags.end(offset)
            {
                let name = tags::inline_tag_name(&text[pos..]);
                if name == "snippet" {
                    return self.region(index, pos, Closer::Offset(end), sep);
                }
                self.tag_end = end;
                self.inline_arg = Arg::inline(name);
                let piece_end = (pos..bytes.len())
                    .find(|&at| is_space(bytes[at]))
                    .unwrap_or(bytes.len())
                    .min(end - line.offset);
                let glue = self.block_arg.glue(&text[pos..piece_end]);
                self.emit(&text[pos..piece_end], sep, glue);
                pos = piece_end;
                sep = Sep::None;
                if line.offset + pos >= self.tag_end {
                    self.tag_end = 0;
                    self.inline_arg = Arg::None;
                }
                continue;
            }
            if bytes[pos] == b'<' {
                if let Some(element) = self.block_element(index, pos) {
                    if let Some(next) = self.element(index, pos, element, sep) {
                        return next;
                    }
                    let (end_line, end) = element.end;
                    if end_line != index {
                        return self.after(end_line, end);
                    }
                    pos = end;
                    sep = Sep::None;
                    continue;
                }
                if text[pos..].starts_with("<!--") {
                    match text[pos..].find("-->") {
                        Some(end) => {
                            let end = pos + end + 3;
                            self.emit(&text[pos..end], sep, false);
                            pos = end;
                            sep = Sep::None;
                            continue;
                        }
                        None => return self.region(index, pos, Closer::Text("-->"), sep),
                    }
                }
            }
            let ((end_line, end), literal) = self.word_end(index, pos);
            if end_line != index {
                let glue = self.block_arg.glue(&text[pos..]);
                self.emit_lines(index, pos, (end_line, end), literal, sep, glue);
                return self.after(end_line, end);
            }
            let piece = &text[pos..end];
            let glue = self.block_arg.glue(piece);
            self.emit(piece, sep, glue);
            pos = end;
            sep = Sep::None;
        }
    }

    /// Where to go on after a word or tag that ends at byte `end` of line
    /// `index`, a later line than the one it starts on.
    fn after(&self, index: usize, end: usize) -> (usize, usize, bool) {
        if end >= self.lines[index].text.len() {
            (index + 1, 0, false)
        } else {
            (index, end, false)
        }
    }

    /// The line and byte where the word at `pos` on line `index` ends: at a
    /// space, or where a block element or an inline tag starts. An inline
    /// HTML tag (`<a href="…">`) is part of the word with the spaces and line
    /// breaks inside it; the flag tells whether a quoted attribute value
    /// holds a line break, as [`tag_end`](Self::tag_end) does.
    fn word_end(&mut self, index: usize, pos: usize) -> ((usize, usize), bool) {
        let Line { text, offset, .. } = self.lines[index];
        let bytes = text.as_bytes();
        let mut at = pos;
        while at < bytes.len() {
            match bytes[at] {
                byte if is_space(byte) => break,
                b'{' if at > pos
                    && bytes.get(at + 1) == Some(&b'@')
                    && self.tags.end(offset + at).is_some() =>
                {
                    break;
                }
                b'<' if at > pos && self.starts_block(index, at) => break,
                b'<' if bytes
                    .get(at + 1)
                    .is_some_and(|&next| next.is_ascii_alphabetic() || next == b'/') =>
                {
                    match self.tag_end(index, at + 1) {
                        Some(((line, end), _)) if line == index => at = end,
                        Some(end) => return end,
                        None => at += 1,
                    }
                }
                _ => at += 1,
            }
        }
        ((index, at), false)
    }

    /// Lays out the block-level element at `pos` on line `index`. Returns
    /// where to go on when it starts a verbatim region.
    fn element(
        &mut self,
        index: usize,
        pos: usize,
        element: Element,
        sep: Sep,
    ) -> Option<(usize, usize, bool)> {
        let mid_line = true;
        self.block_arg = Arg::None;
        match (element.name, element.closing) {
            (Name::Pre, false) => return Some(self.region(index, pos, Closer::Text("</pre"), sep)),
            (Name::Table, false) => {
                return Some(self.region(index, pos, Closer::Text("</table"), sep));
            }
            (Name::Blockquote, false) => {
                return Some(self.region(index, pos, Closer::Text("</blockquote"), sep));
            }
            (Name::P | Name::Heading, false) => {
                // `<li><p>` stays together.
                if !(self.attach && element.name == Name::P) {
                    self.request(Kind::Block, mid_line);
                }
                self.emit_tag(index, pos, element, sep, false);
                self.attach = true;
            }
            (Name::P | Name::Heading, true) => {
                self.attach |= self.next_line.is_none();
                self.emit_tag(index, pos, element, sep, true);
                if element.name == Name::Heading {
                    self.gap = self.gap.max(Gap::Block);
                    self.request(Kind::Text, mid_line);
                }
            }
            (Name::List, false) => {
                self.request(Kind::Block, mid_line);
                let open = self.column();
                self.emit_tag(index, pos, element, sep, false);
                self.frames.push(Frame::List {
                    open,
                    column: open + 2,
                });
                self.request(Kind::Text, mid_line);
            }
            (Name::List, true) => {
                let Some(list) = self
                    .frames
                    .iter()
                    .rposition(|frame| matches!(frame, Frame::List { .. }))
                else {
                    self.emit_tag(index, pos, element, sep, false);
                    return None;
                };
                let Frame::List { open, .. } = self.frames[list] else {
                    unreachable!("a list frame");
                };
                self.frames.truncate(list);
                self.next_line = None;
                self.layout.line(self.gap == Gap::Blank, open, open);
                self.emit_tag(index, pos, element, sep, false);
                self.gap = self.gap.max(Gap::Block);
                self.request(Kind::Text, mid_line);
            }
            (Name::Item, false) => {
                if matches!(self.frames.last(), Some(Frame::Item { .. })) {
                    self.frames.pop();
                }
                let line = self.column();
                self.next_line = None;
                self.start(Kind::Block, line, line + 4);
                self.emit_tag(index, pos, element, sep, false);
                self.frames.push(Frame::Item {
                    line,
                    column: line + 4,
                });
                self.attach = true;
            }
            (Name::Item, true) => {
                if let Some(&Frame::Item { line, .. }) = self.frames.last() {
                    if self.next_line.take().is_some() {
                        self.layout.line(self.gap == Gap::Blank, line, line);
                    } else {
                        self.attach = true;
                    }
                    self.emit_tag(index, pos, element, sep, true);
                    self.frames.pop();
                } else {
                    self.emit_tag(index, pos, element, sep, true);
                }
            }
            (Name::Br, _) => {
                self.emit_tag(index, pos, element, sep, true);
                self.request(Kind::Break, mid_line);
            }
            (Name::Pre | Name::Table | Name::Blockquote, true) => {
                self.emit_tag(index, pos, element, sep, false);
            }
        }
        None
    }

    /// Adds the tag of `element`, which starts at `pos` on line `index`, as
    /// [`emit`](Self::emit) adds a word.
    fn emit_tag(&mut self, index: usize, pos: usize, element: Element, sep: Sep, glue: bool) {
        let (end_line, end) = element.end;
        if end_line == index {
            self.emit(&self.lines[index].text[pos..end], sep, glue);
        } else {
            self.emit_lines(index, pos, element.end, element.literal, sep, glue);
        }
    }

    /// Adds the text from `pos` on line `index` to `end` on a later line,
    /// which ends an HTML tag that runs onto later lines, as words: the tag
    /// may wrap at its whitespace outside quoted attribute values, where a
    /// line break renders nothing, but not inside one, where javadoc would
    /// keep it. When a quoted attribute value holds a line break of the
    /// source (`literal`), the lines after the first are printed as written.
    fn emit_lines(
        &mut self,
        index: usize,
        pos: usize,
        end: (usize, usize),
        literal: bool,
        sep: Sep,
        glue: bool,
    ) {
        let first = &self.lines[index].text[pos..];
        if literal {
            self.emit(first, sep, glue);
            self.as_written_to(index, end);
            return;
        }
        let (end_line, end) = end;
        // After `=`, where a quote starts a value.
        let mut value = false;
        let mut started = false;
        for line in index..=end_line {
            let text = self.lines[line].text;
            let text = if line == end_line { &text[..end] } else { text };
            let text = if line == index { first } else { text };
            let bytes = text.as_bytes();
            let mut start = None;
            let mut at = 0;
            while at <= bytes.len() {
                let byte = bytes.get(at).copied();
                if byte.is_none_or(is_space) {
                    if let Some(from) = start.take() {
                        if std::mem::replace(&mut started, true) {
                            self.layout.word(&text[from..at], Join::Wrap);
                        } else {
                            self.emit(&text[from..at], sep, glue);
                        }
                    }
                    at += 1;
                    continue;
                }
                start.get_or_insert(at);
                match byte {
                    Some(b'=') => value = true,
                    Some(quote @ (b'"' | b'\'')) if value => {
                        at = text[at + 1..]
                            .find(char::from(quote))
                            .map_or(bytes.len() - 1, |len| at + 1 + len);
                        value = false;
                    }
                    _ => value = false,
                }
                at += 1;
            }
        }
    }

    /// Prints the lines after line `index` up to byte `end` of line
    /// `end_line` as written.
    fn as_written_to(&mut self, index: usize, (end_line, end): (usize, usize)) {
        for line in &self.lines[index + 1..end_line] {
            self.layout.verbatim(line.raw);
        }
        let last = self.lines[end_line];
        let frame = last.raw.len() - last.text.len();
        self.layout.verbatim(&last.raw[..frame + end]);
    }

    /// A verbatim region from `pos` on line `index` to its closer: its first
    /// line from `pos` starts a block, the lines after it are printed as
    /// written, and the last up to the closer. A region that isn't closed
    /// runs to the end of the comment. An HTML comment (`<!--`) is a region
    /// too, but it starts no block: it may be glued to the words around it,
    /// and starts a line only where the source starts one with it.
    fn region(
        &mut self,
        index: usize,
        pos: usize,
        closer: Closer,
        sep: Sep,
    ) -> (usize, usize, bool) {
        let (end_line, end_pos) = self.region_end(index, pos, closer);
        let block = !matches!(closer, Closer::Text("-->"));
        self.block_arg = Arg::None;
        if block {
            self.request(Kind::Block, true);
        } else if sep == Sep::Line {
            self.request(Kind::Break, false);
        }
        let first = self.lines[index].text;
        if end_line == index {
            self.emit(&first[pos..end_pos], sep, false);
        } else {
            self.emit(&first[pos..], sep, false);
            self.as_written_to(index, (end_line, end_pos));
        }
        let text = self.lines[end_line].text;
        if block {
            self.gap = self.gap.max(Gap::Block);
            let rest = text[end_pos..].trim_start_matches(SPACES);
            self.request(Kind::Text, !rest.is_empty());
        }
        if end_pos >= text.len() {
            (end_line + 1, 0, true)
        } else {
            (end_line, end_pos, true)
        }
    }

    /// The line and byte where a region that starts at `pos` on line
    /// `index` ends.
    fn region_end(&self, index: usize, pos: usize, closer: Closer) -> (usize, usize) {
        // A region that isn't closed ends at the comment's last line of text.
        let last = (index..self.lines.len())
            .rev()
            .find(|&at| !self.lines[at].text.is_empty())
            .unwrap_or(index);
        let end_of_comment = (last, self.lines[last].text.len());
        match closer {
            Closer::Offset(end) => {
                let line = (index..=last)
                    .find(|&at| {
                        let line = self.lines[at];
                        end <= line.offset + line.text.len()
                    })
                    .unwrap_or(last);
                (line, end - self.lines[line].offset)
            }
            Closer::Text(closer) => {
                let tag = closer.starts_with("</");
                for at in index..=last {
                    let text = self.lines[at].text;
                    let from = if at == index { pos + 1 } else { 0 };
                    let Some(found) = find_ignoring_case(&text[from.min(text.len())..], closer)
                    else {
                        continue;
                    };
                    let after = from + found + closer.len();
                    if !tag {
                        return (at, after);
                    }
                    for end in at..=last {
                        let text = self.lines[end].text;
                        let from = if end == at { after } else { 0 };
                        if let Some(gt) = text[from..].find('>') {
                            return (end, from + gt + 1);
                        }
                    }
                    return end_of_comment;
                }
                end_of_comment
            }
        }
    }
}

/// What ends a verbatim region.
#[derive(Clone, Copy, Debug)]
enum Closer {
    /// The end offset of a `{@snippet}` in the comment.
    Offset(usize),
    /// The text that ends it, lowercase: `</pre`, then the next `>`, or
    /// `-->`.
    Text(&'static str),
}
