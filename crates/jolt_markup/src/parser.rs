//! A port of javac's doc comment parser (JDK 25,
//! `com.sun.tools.javac.parser.DocCommentParser`), rule for rule, over the
//! comment's lines as javac reads them (see `frame`).
//!
//! It reads a comment as javadoc does: text, entities, HTML, inline tags and
//! block tags for a `/**` comment, and Markdown text, inline tags and block
//! tags for a `///` comment, with javac's own recognition of Markdown code
//! spans and code blocks (its nested `Markdown` class), inside which it reads
//! no tags. Erroneous constructs become erroneous nodes, never failures.
//!
//! The tree keeps only what the layout needs: each node's span, and for HTML
//! elements and tags their names and parts. Deviations from javac:
//!
//! - A reference (`{@link Map#get}`, `@throws IOException`) is not parsed:
//!   javac's `ReferenceParser` runs the Java parser on it and turns a tag with
//!   a malformed reference into an erroneous node. Here it is always a
//!   reference. The layout prints references, labels and erroneous nodes with
//!   the same rules (a reference is an atom, label text is filled, and every
//!   word that starts with `@` keeps whether it starts a line), so both
//!   readings stay the same.
//! - `<pre>` content is not normalized (`normalizePreContent`): the layout
//!   prints a `<pre>` region as written.
//! - Where javac's Markdown scanner would index past a line (a list marker
//!   followed by a tab at the end of a line), this reads the rest of the line
//!   as empty.

use std::cell::OnceCell;

use crate::frame::Line;
use crate::java;

/// The end of input, as javac marks it (`LayoutCharacters.EOI`).
const EOI: char = '\u{1a}';

/// A position in a comment as javadoc reads it: a line, and a byte offset in
/// that line's text. An offset equal to the line's length is the line break
/// after it (or the end of the comment, after the last line).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Pos {
    pub(crate) line: u32,
    pub(crate) off: u32,
}

/// A byte offset or line index as a `u32` (a comment longer than 4 GiB
/// saturates, and is then read wrongly but safely).
pub(crate) fn to_u32(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Span {
    pub(crate) start: Pos,
    pub(crate) end: Pos,
}

#[derive(Debug)]
pub(crate) enum Node {
    /// Text (`/**`) or Markdown text (`///`).
    Text(Span),
    /// A node printed as written: an entity, an escape (`@@`, `@*`, `@/`),
    /// an HTML comment, a doctype or CDATA.
    Atom(Span),
    /// An erroneous node, printed as written. What follows it decided that
    /// it is erroneous (`{@snippet :` needs a line break after `:`), so the
    /// separator after it keeps its kind.
    Erroneous(Span),
    /// An HTML start or end tag. `name` is the element's name.
    Element {
        span: Span,
        name: Span,
        closing: bool,
    },
    Inline(Inline),
}

#[derive(Debug)]
pub(crate) struct Inline {
    /// From `{` to after the closing `}`.
    pub(crate) span: Span,
    /// `{@` and the tag's name.
    pub(crate) name: Span,
    /// The tag's text: `None` for a tag printed as written; otherwise the
    /// head after the name (a reference, an index term), and the rich-text
    /// content (a label, a description), and the closing `}`, which a label
    /// that runs to the end of the comment lacks.
    pub(crate) rich: Option<Rich>,
}

#[derive(Debug)]
pub(crate) struct Rich {
    pub(crate) head: Option<Span>,
    pub(crate) content: Vec<Node>,
    pub(crate) close: Option<Span>,
}

#[derive(Debug)]
pub(crate) struct BlockTag {
    /// From `@` to the end of the tag's text.
    pub(crate) span: Span,
    /// The name (`@param`) and arguments (a parameter name, a reference, a
    /// quoted string), which the layout joins with single spaces.
    pub(crate) head: Vec<Span>,
    /// The tag's text, or `None` for an erroneous tag, printed as written.
    pub(crate) body: Option<Vec<Node>>,
}

#[derive(Debug)]
pub(crate) struct Comment {
    pub(crate) body: Vec<Node>,
    pub(crate) tags: Vec<BlockTag>,
    /// Text after a tag that javac reads no further than (an inline-only tag
    /// such as `@code` used as a block tag ends its block tags).
    pub(crate) tail: Option<Span>,
}

/// Reads a comment's lines as javadoc does; `markdown` for a `///` comment.
/// `None` when the reading takes more than [`STEPS_PER_BYTE`] steps per
/// byte, nests inline tags deeper than [`MAX_INLINE_DEPTH`], or depends on
/// whether javac's `ReferenceParser` accepts a reference (see
/// [`Parser::give_up`]): javac rescans text after a malformed construct (an
/// HTML tag with an unclosed quote, an unclosed Markdown code span), which
/// some inputs make quadratic, and its Markdown scanner never ends on some
/// last lines; such a comment is printed as written.
pub(crate) fn parse(lines: &[Line<'_>], markdown: bool) -> Option<Comment> {
    let size: usize = lines.iter().map(|line| line.text.len() + 1).sum();
    let mut parser = Parser {
        steps: 0,
        budget: size * STEPS_PER_BYTE + 64,
        lines,
        cur: Cursor {
            pos: Pos { line: 0, off: 0 },
            ch: EOI,
            width: 0,
            prev: EOI,
        },
        newline: true,
        text_start: None,
        last_non_white: None,
        markdown: markdown.then(Markdown::default),
        end: Pos {
            line: to_u32(lines.len() - 1),
            off: to_u32(lines[lines.len() - 1].text.len()),
        },
        inline_depth: 0,
    };
    (parser.cur.ch, parser.cur.width) = parser.decode(parser.cur.pos);
    let (body, _) = parser.content(Phase::Body);
    if !markdown
        && let Some(&Node::Text(span)) = body.first()
        && parser.is_blank(span)
    {
        // Text of only whitespace (a form feed, which javac doesn't skip at
        // the start) makes a `<p>` after it a sentence break, which it isn't
        // as the first tree; the layout drops that text.
        parser.give_up();
    }
    let mut tags = Vec::new();
    while !parser.at_end() && parser.cur.ch == '@' {
        tags.push(parser.block_tag());
    }
    // The whitespace around the tail is the frame's, not text.
    parser.skip_whitespace();
    let tail = (!parser.at_end()).then(|| {
        let mut end = parser.end();
        while let Some((pos, ch)) = parser.before(end)
            && is_ws(ch)
            && pos > parser.cur.pos
        {
            end = pos;
        }
        Span {
            start: parser.cur.pos,
            end,
        }
    });
    (parser.steps <= parser.budget).then_some(Comment { body, tags, tail })
}

/// The reading's step budget per byte of the comment. A comment is read in
/// about one step per character, a few for the lines of a Markdown comment.
const STEPS_PER_BYTE: usize = 8;

/// The deepest nesting of inline tags that a comment is read with (`{@link
/// A {@code b}}` nests one in another); a deeper comment is printed as
/// written.
const MAX_INLINE_DEPTH: u32 = 32;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Body,
    Inline,
}

/// The reader's place: `pos` and the character there (`ch`), and the one
/// before it, for javac's `buf[bp - 1]`.
#[derive(Clone, Copy)]
struct Cursor {
    pos: Pos,
    ch: char,
    /// The width of `ch` in bytes (none for a line break or the end).
    width: u8,
    prev: char,
}

/// javac's `ParseException`: the tag in progress becomes an erroneous node.
struct ParseError;

type Parsed<T> = Result<T, ParseError>;

struct Parser<'l, 'a> {
    lines: &'l [Line<'a>],
    /// Characters read, and the most the reading may take.
    steps: usize,
    budget: usize,
    cur: Cursor,
    /// javac's `newline`: no character but spaces and tabs since the last
    /// line break.
    newline: bool,
    text_start: Option<Pos>,
    /// The end (exclusive) of javac's `lastNonWhite`.
    last_non_white: Option<Pos>,
    markdown: Option<Markdown>,
    /// The end of the comment.
    end: Pos,
    /// The inline tags the reader is in.
    inline_depth: u32,
}

/// The kinds of tag that javac knows.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Tag {
    Author,
    Code,
    Deprecated,
    DocRoot,
    Exception,
    Hidden,
    Index,
    InheritDoc,
    Link,
    LinkPlain,
    Literal,
    Param,
    Provides,
    Return,
    See,
    SerialData,
    SerialField,
    Serial,
    Since,
    Snippet,
    Spec,
    Summary,
    SystemProperty,
    Throws,
    Uses,
    Value,
    Version,
}

impl Tag {
    fn named(name: &str) -> Option<Self> {
        Some(match name {
            "author" => Self::Author,
            "code" => Self::Code,
            "deprecated" => Self::Deprecated,
            "docRoot" => Self::DocRoot,
            "exception" => Self::Exception,
            "hidden" => Self::Hidden,
            "index" => Self::Index,
            "inheritDoc" => Self::InheritDoc,
            "link" => Self::Link,
            "linkplain" => Self::LinkPlain,
            "literal" => Self::Literal,
            "param" => Self::Param,
            "provides" => Self::Provides,
            "return" => Self::Return,
            "see" => Self::See,
            "serialData" => Self::SerialData,
            "serialField" => Self::SerialField,
            "serial" => Self::Serial,
            "since" => Self::Since,
            "snippet" => Self::Snippet,
            "spec" => Self::Spec,
            "summary" => Self::Summary,
            "systemProperty" => Self::SystemProperty,
            "throws" => Self::Throws,
            "uses" => Self::Uses,
            "value" => Self::Value,
            "version" => Self::Version,
            _ => return None,
        })
    }

    const fn allows_inline(self) -> bool {
        matches!(
            self,
            Self::Code
                | Self::DocRoot
                | Self::Index
                | Self::InheritDoc
                | Self::Link
                | Self::LinkPlain
                | Self::Literal
                | Self::Return
                | Self::Snippet
                | Self::Summary
                | Self::SystemProperty
                | Self::Value
        )
    }

    const fn allows_block(self) -> bool {
        matches!(self, Self::Return) || !self.allows_inline()
    }

    const fn retains_whitespace(self) -> bool {
        matches!(self, Self::Code | Self::Literal)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Whitespace {
    RetainAll,
    RemoveFirstSpace,
    RemoveAll,
}

const fn is_ws(ch: char) -> bool {
    matches!(ch, '\n' | '\r' | '\u{c}' | ' ' | '\t')
}

impl Parser<'_, '_> {
    // The reader.

    /// The character at `pos` and its width in bytes: the line break after
    /// a line and the end of input have none.
    fn decode(&self, pos: Pos) -> (char, u8) {
        let text = self.lines[pos.line as usize].text;
        let bytes = text.as_bytes();
        let off = pos.off as usize;
        match bytes.get(off) {
            Some(&byte) if byte.is_ascii() => (char::from(byte), 1),
            Some(_) => {
                let ch = text[off..].chars().next().unwrap_or(EOI);
                (ch, u8::try_from(ch.len_utf8()).unwrap_or(1))
            }
            None if (pos.line as usize) + 1 < self.lines.len() => ('\n', 0),
            None => (EOI, 0),
        }
    }

    fn char_at(&self, pos: Pos) -> char {
        self.decode(pos).0
    }

    const fn end(&self) -> Pos {
        self.end
    }

    fn at_end(&self) -> bool {
        self.cur.pos >= self.end
    }

    /// The position after the character at `pos`.
    fn after(&self, pos: Pos) -> Pos {
        let (_, width) = self.decode(pos);
        self.step(pos, width)
    }

    /// The position after a character of `width` bytes at `pos`.
    fn step(&self, pos: Pos, width: u8) -> Pos {
        if width > 0 {
            Pos {
                line: pos.line,
                off: pos.off + u32::from(width),
            }
        } else if pos < self.end {
            Pos {
                line: pos.line + 1,
                off: 0,
            }
        } else {
            pos
        }
    }

    /// The position after the reader's character.
    fn after_cur(&self) -> Pos {
        self.step(self.cur.pos, self.cur.width)
    }

    /// The position of the character before `pos`, and that character.
    fn before(&self, pos: Pos) -> Option<(Pos, char)> {
        if pos.off > 0 {
            let line = self.lines[pos.line as usize].text;
            let ch = line[..pos.off as usize].chars().next_back()?;
            let off = pos.off - u32::try_from(ch.len_utf8()).unwrap_or(1);
            Some((
                Pos {
                    line: pos.line,
                    off,
                },
                ch,
            ))
        } else if pos.line > 0 {
            let line = pos.line - 1;
            let off = to_u32(self.lines[line as usize].text.len());
            Some((Pos { line, off }, '\n'))
        } else {
            None
        }
    }

    /// Fails the reading where it can't follow javac's: the comment is
    /// printed as written. That is a description that starts with text of
    /// only whitespace (see [`parse`]); an escape (`@@`, `@*`) that doesn't
    /// start its line, which javac reads as one only after a line break
    /// before a tag's argument that the layout joins; an inline tag nested
    /// too deeply; a `{@value}` reference that no `}` follows on its line;
    /// and an `{@inheritDoc}` reference whose `}` is on a later line. javac
    /// stops at an invalid reference, which `ReferenceParser` (not read by
    /// this port) decides, and after a valid one reads on: through a `}`
    /// after which the line break before it still makes an `@` start a
    /// block tag, and for `{@value}` one more character, which can be the
    /// `{` of the next inline tag.
    fn give_up(&mut self) {
        self.charge(self.budget + 1);
    }

    /// Reads an escape (`@@`, `@*`) at the reader, which is at a line start;
    /// false when the reading gives up instead. An escape that doesn't start
    /// its line is one because of a line break before a tag's argument
    /// (`@param` and `x @@` on the next line), which the layout joins.
    fn escape(&mut self, trees: &mut Vec<Node>) -> bool {
        if !self.starts_line(self.cur.pos) {
            self.give_up();
            return false;
        }
        self.add_pending_text(trees, Some(self.cur.pos));
        let at = self.cur.pos;
        // A line break right after the escape keeps the reader at a line
        // start.
        self.newline = false;
        self.next_char();
        self.next_char();
        trees.push(Node::Atom(self.span(at)));
        self.text_start = Some(self.cur.pos);
        true
    }

    /// Whether `span` holds only whitespace.
    fn is_blank(&self, span: Span) -> bool {
        (span.start.line..=span.end.line).all(|line| {
            let from = if line == span.start.line {
                span.start.off
            } else {
                0
            };
            let to = (line == span.end.line).then_some(span.end.off);
            let text = self.lines[line as usize].text;
            text[from as usize..to.map_or(text.len(), |to| to as usize)]
                .chars()
                .all(is_ws)
        })
    }

    /// Whether only spaces and tabs come before `pos` on its line.
    fn starts_line(&self, pos: Pos) -> bool {
        self.lines[pos.line as usize].text[..pos.off as usize]
            .bytes()
            .all(|byte| matches!(byte, b' ' | b'\t'))
    }

    /// Counts `count` steps of the reading. Past the budget the reading
    /// fails: the reader moves to the end, and true tells the caller to stop.
    fn charge(&mut self, count: usize) -> bool {
        self.steps = self.steps.saturating_add(count);
        if self.steps > self.budget {
            self.cur.pos = self.end;
            self.cur.ch = EOI;
            self.cur.width = 0;
            return true;
        }
        false
    }

    /// javac's `nextChar`: a line break sets `newline`.
    fn next_char(&mut self) {
        if self.at_end() || self.charge(1) {
            return;
        }
        let next = self.after_cur();
        self.cur.prev = self.cur.ch;
        self.cur.pos = next;
        (self.cur.ch, self.cur.width) = self.decode(next);
        if self.cur.ch == '\n' {
            self.newline = true;
        }
    }

    fn peek_char(&self) -> char {
        if self.at_end() {
            return EOI;
        }
        self.char_at(self.after_cur())
    }

    /// Sets the reader to a saved place without touching `newline`, as
    /// javac's `bp = …; ch = buf[bp]` does.
    const fn seek(&mut self, cursor: Cursor) {
        self.cur = cursor;
    }

    fn span(&self, start: Pos) -> Span {
        Span {
            start,
            end: self.cur.pos,
        }
    }

    fn skip_whitespace(&mut self) {
        while !self.at_end() && java::is_whitespace(self.cur.ch) {
            self.next_char();
        }
    }

    fn is_identifier_start(&self) -> bool {
        java::is_unicode_identifier_start(self.cur.ch)
    }

    /// javac's `readIdentifier`, `readAttributeName` and `readTagName`: the
    /// first character, then the characters `part` accepts.
    fn read_name(&mut self, part: impl Fn(char) -> bool) -> Span {
        let start = self.cur.pos;
        self.next_char();
        while !self.at_end() && part(self.cur.ch) {
            self.next_char();
        }
        self.span(start)
    }

    fn read_identifier(&mut self) -> Span {
        self.read_name(java::is_unicode_identifier_part)
    }

    fn read_attribute_name(&mut self) -> Span {
        self.read_name(|ch| java::is_unicode_identifier_part(ch) || ch == '-')
    }

    fn read_tag_name(&mut self) -> Span {
        self.read_name(|ch| java::is_unicode_identifier_part(ch) || matches!(ch, '.' | '-' | ':'))
    }

    fn text(&self, span: Span) -> &str {
        // Names never span lines.
        let line = self.lines[span.start.line as usize].text;
        if span.end.line == span.start.line {
            &line[span.start.off as usize..span.end.off as usize]
        } else {
            &line[span.start.off as usize..]
        }
    }

    // Text.

    /// javac's `defaultContentCharacter`.
    fn default_content(&mut self) {
        self.newline = false;
        if self.text_start.is_none() {
            self.text_start = Some(self.cur.pos);
        }
        self.last_non_white = Some(self.after_cur());
        self.next_char();
    }

    /// `default_content` for the reader's character and the ordinary
    /// characters after it on its line, at once: the characters that no case
    /// of `content` reads differently.
    fn plain_run(&mut self) {
        self.default_content();
        let text = self.lines[self.cur.pos.line as usize].text;
        let bytes = text.as_bytes();
        let start = self.cur.pos.off as usize;
        let mut end = start;
        while end < bytes.len()
            && !matches!(
                bytes[end],
                b' ' | b'\t'
                    | b'&'
                    | b'<'
                    | b'{'
                    | b'}'
                    | b'@'
                    | b'\\'
                    | b'`'
                    | b'~'
                    | b'*'
                    | 0x0c
                    | 0x80..=0xff
            )
        {
            end += 1;
        }
        let count = end - start;
        if count == 0 {
            return;
        }
        if self.steps + count > self.budget {
            // Past the budget: the reading fails here rather than reading
            // the same run again one character at a time.
            self.charge(count);
            return;
        }
        self.steps += count;
        self.newline = false;
        self.cur.prev = char::from(bytes[end - 1]);
        self.cur.pos.off = to_u32(end);
        self.last_non_white = Some(self.cur.pos);
        (self.cur.ch, self.cur.width) = self.decode(self.cur.pos);
        if self.cur.ch == '\n' {
            self.newline = true;
        }
    }

    /// javac's `addPendingText`, with an exclusive end.
    fn add_pending_text(&mut self, list: &mut Vec<Node>, end: Option<Pos>) {
        if let Some(start) = self.text_start.take()
            && let Some(end) = end
            && start < end
        {
            list.push(Node::Text(Span { start, end }));
        }
    }

    /// javac's `erroneous`: from `start` to the last character before the
    /// reader that is not whitespace.
    fn erroneous(&mut self, start: Pos) -> Node {
        let mut end = self.cur.pos;
        while let Some((before, ch)) = self.before(end) {
            if before <= start || !is_ws(ch) {
                break;
            }
            if matches!(ch, '\u{c}' | '\n' | '\r') {
                self.newline = true;
            }
            end = before;
        }
        if end <= start {
            end = self.after(start);
        }
        self.text_start = None;
        Node::Erroneous(Span { start, end })
    }

    /// javac's `content`: rich text up to a block tag (`Body`) or the `}`
    /// that closes an inline tag (`Inline`, with the `}`'s span).
    fn content(&mut self, phase: Phase) -> (Vec<Node>, Option<Span>) {
        let mut trees = Vec::new();
        self.text_start = None;
        let mut depth = 1;
        let start = self.cur.pos;
        if self.markdown.is_some() {
            self.init_markdown_line();
        }
        'read: while !self.at_end() {
            let markdown = self.markdown.is_some();
            match self.cur.ch {
                '\n' | '\r' => {
                    self.next_char();
                    if markdown {
                        self.init_markdown_line();
                    }
                }
                ' ' | '\t' => {
                    if markdown && self.text_start.is_none() {
                        self.text_start = Some(self.cur.pos);
                    }
                    self.next_char();
                }
                '&' if !markdown => self.entity_into(&mut trees),
                '<' if !markdown => {
                    self.newline = false;
                    self.add_pending_text(&mut trees, Some(self.cur.pos));
                    let html = self.html();
                    trees.push(html);
                    if self.text_start.is_none() {
                        self.text_start = Some(self.cur.pos);
                        self.last_non_white = None;
                    }
                }
                '{' => {
                    let tag = self.inline_tag(&mut trees);
                    if phase == Phase::Inline && !tag {
                        depth += 1;
                    }
                }
                '}' if phase == Phase::Inline => {
                    self.newline = false;
                    depth -= 1;
                    if depth == 0 {
                        self.add_pending_text(&mut trees, Some(self.cur.pos));
                        let close = self.cur.pos;
                        self.next_char();
                        return (trees, Some(self.span(close)));
                    }
                    self.next_char();
                }
                '@' => {
                    if self.newline {
                        let peek = self.peek_char();
                        if peek == '@' || peek == '*' {
                            if !self.escape(&mut trees) {
                                break 'read;
                            }
                            continue;
                        } else if phase == Phase::Body {
                            let end = self.last_non_white;
                            self.add_pending_text(&mut trees, end);
                            break 'read;
                        }
                    } else if self.text_start.is_some()
                        && self.cur.prev == '*'
                        && self.peek_char() == '/'
                    {
                        self.add_pending_text(&mut trees, Some(self.cur.pos));
                        let at = self.cur.pos;
                        self.next_char();
                        self.next_char();
                        trees.push(Node::Atom(self.span(at)));
                        self.newline = false;
                        self.text_start = Some(self.cur.pos);
                        continue;
                    }
                    self.default_content();
                }
                '\\' if markdown => {
                    self.default_content();
                    self.next_char();
                    self.default_content();
                }
                '`' | '~' if markdown => self.markdown_code(),
                _ => self.plain_run(),
            }
        }
        if self.last_non_white.is_some() {
            let end = self.last_non_white;
            self.add_pending_text(&mut trees, end);
        }
        if phase == Phase::Inline {
            return (vec![self.erroneous(start)], None);
        }
        (trees, None)
    }

    fn entity_into(&mut self, list: &mut Vec<Node>) {
        self.newline = false;
        self.add_pending_text(list, Some(self.cur.pos));
        let entity = self.entity();
        list.push(entity);
        if self.text_start.is_none() {
            self.text_start = Some(self.cur.pos);
            self.last_non_white = None;
        }
    }

    /// javac's `entity`: `&name;`, `&#digits;`, `&#xhex;`.
    fn entity(&mut self) -> Node {
        let start = self.cur.pos;
        self.next_char();
        let mut named = false;
        if self.cur.ch == '#' {
            self.next_char();
            if self.cur.ch.is_ascii_digit() {
                self.next_char();
                while !self.at_end() && self.cur.ch.is_ascii_digit() {
                    self.next_char();
                }
                named = true;
            } else if matches!(self.cur.ch, 'x' | 'X') {
                self.next_char();
                if self.cur.ch.is_ascii_hexdigit() {
                    self.next_char();
                    while !self.at_end() && self.cur.ch.is_ascii_hexdigit() {
                        self.next_char();
                    }
                    named = true;
                }
            }
        } else if self.is_identifier_start() {
            self.read_identifier();
            named = true;
        }
        if !named || self.cur.ch != ';' {
            return self.erroneous(start);
        }
        self.next_char();
        Node::Atom(self.span(start))
    }

    // HTML.

    /// javac's `html`: a start or end tag, a comment, a doctype or CDATA,
    /// or an erroneous `<`.
    fn html(&mut self) -> Node {
        let start = self.cur.pos;
        self.next_char();
        let after_open = self.cur;
        if self.is_identifier_start() {
            let name = self.read_identifier();
            self.html_attrs();
            if self.cur.ch == '/' {
                self.next_char();
            }
            if self.cur.ch == '>' {
                self.next_char();
                return Node::Element {
                    span: self.span(start),
                    name,
                    closing: false,
                };
            }
        } else if self.cur.ch == '/' {
            self.next_char();
            if self.is_identifier_start() {
                let name = self.read_identifier();
                self.skip_whitespace();
                if self.cur.ch == '>' {
                    self.next_char();
                    return Node::Element {
                        span: self.span(start),
                        name,
                        closing: true,
                    };
                }
            }
        } else if self.cur.ch == '!' {
            self.next_char();
            if self.cur.ch == '-' {
                self.next_char();
                if self.cur.ch == '-' {
                    self.next_char();
                    while !self.at_end() {
                        let mut dashes = 0;
                        while !self.at_end() && self.cur.ch == '-' {
                            dashes += 1;
                            self.next_char();
                        }
                        if dashes >= 2 && self.cur.ch == '>' {
                            self.next_char();
                            return Node::Atom(self.span(start));
                        }
                        self.next_char();
                    }
                }
            } else if self.is_identifier_start() && self.peek_doctype() {
                self.read_identifier();
                self.next_char();
                self.skip_whitespace();
                while !self.at_end() {
                    if self.cur.ch == '>' {
                        self.next_char();
                        return Node::Atom(self.span(start));
                    }
                    self.next_char();
                }
            } else {
                for expected in "[CDATA[".chars() {
                    if self.cur.ch == expected {
                        self.next_char();
                    } else {
                        return self.erroneous(start);
                    }
                }
                while !self.at_end() {
                    if self.cur.ch == ']' {
                        let mut brackets = 0;
                        while !self.at_end() && self.cur.ch == ']' {
                            brackets += 1;
                            self.next_char();
                        }
                        if brackets >= 2 && self.cur.ch == '>' {
                            self.next_char();
                            return Node::Atom(self.span(start));
                        }
                    } else {
                        self.next_char();
                    }
                }
                return self.erroneous(start);
            }
        }
        self.seek(after_open);
        self.erroneous(start)
    }

    /// javac's `peek("doctype")`.
    fn peek_doctype(&mut self) -> bool {
        let saved = self.cur;
        let newline = self.newline;
        let name = self.read_identifier();
        let found = self.text(name).eq_ignore_ascii_case("doctype");
        self.seek(saved);
        self.newline = newline;
        found
    }

    /// javac's `htmlAttrs`: attributes up to the tag's end.
    fn html_attrs(&mut self) {
        let mut scratch = Vec::new();
        self.skip_whitespace();
        while !self.at_end() && self.is_identifier_start() {
            self.read_attribute_name();
            self.skip_whitespace();
            if self.cur.ch == '=' {
                self.next_char();
                self.skip_whitespace();
                if matches!(self.cur.ch, '\'' | '"') {
                    self.newline = false;
                    let quote = self.cur.ch;
                    self.next_char();
                    self.text_start = Some(self.cur.pos);
                    while !self.at_end() && self.cur.ch != quote {
                        self.attr_value_char(&mut scratch);
                    }
                    self.add_pending_text(&mut scratch, Some(self.cur.pos));
                    self.next_char();
                } else {
                    self.text_start = Some(self.cur.pos);
                    while !self.at_end() && !is_unquoted_attr_value_terminator(self.cur.ch) {
                        self.attr_value_char(&mut scratch);
                    }
                    self.add_pending_text(&mut scratch, Some(self.cur.pos));
                }
                self.skip_whitespace();
            }
            scratch.clear();
        }
    }

    fn attr_value_char(&mut self, list: &mut Vec<Node>) {
        match self.cur.ch {
            '&' => self.entity_into(list),
            '{' => {
                self.inline_tag(list);
            }
            _ => self.next_char(),
        }
    }

    // Inline tags.

    /// javac's `inlineTag(list)`, at a `{`: reads an inline tag into `list`
    /// and returns true, or reads the `{` (or the escape `{@@`) as text and
    /// returns false.
    fn inline_tag(&mut self, list: &mut Vec<Node>) -> bool {
        self.newline = false;
        let brace = self.cur.pos;
        self.next_char();
        if self.cur.ch == '@' {
            if self.peek_char() == '@' {
                if self.text_start.is_none() {
                    self.text_start = Some(brace);
                }
                self.add_pending_text(list, Some(self.cur.pos));
                let at = self.cur.pos;
                self.next_char();
                self.next_char();
                list.push(Node::Atom(self.span(at)));
                self.text_start = None;
                self.last_non_white = Some(self.after_cur());
            } else {
                self.add_pending_text(list, Some(brace));
                let tag = self.inline_tag_tree(brace);
                list.push(tag);
                self.text_start = Some(self.cur.pos);
                self.last_non_white = None;
                return true;
            }
        } else {
            if self.text_start.is_none() {
                self.text_start = Some(brace);
            }
            self.last_non_white = Some(self.after_cur());
        }
        false
    }

    /// javac's `inlineTag()`, at the `@` after the `{` at `brace`. An inline
    /// tag nested in more than [`MAX_INLINE_DEPTH`] others fails the reading
    /// (javac reads it by recursion, as this does).
    fn inline_tag_tree(&mut self, brace: Pos) -> Node {
        if self.inline_depth == MAX_INLINE_DEPTH {
            self.give_up();
            return self.erroneous(brace);
        }
        self.inline_depth += 1;
        let node = self.inline_tag_tree_at(brace);
        self.inline_depth -= 1;
        node
    }

    fn inline_tag_tree_at(&mut self, brace: Pos) -> Node {
        self.next_char();
        if !self.is_identifier_start() {
            return self.erroneous(brace);
        }
        let name_span = self.read_tag_name();
        let name = Span {
            start: brace,
            end: name_span.end,
        };
        let tag = Tag::named(self.text(name_span));
        let result = match tag {
            None => self.atomic_inline_tag(brace, name),
            Some(tag) => {
                if !tag.retains_whitespace() {
                    self.skip_whitespace();
                }
                if tag.allows_inline() {
                    self.inline_tag_body(tag, brace, name)
                } else {
                    self.atomic_inline_tag(brace, name)
                }
            }
        };
        result.unwrap_or_else(|ParseError| self.erroneous(brace))
    }

    /// An unknown inline tag, or a block tag used inline: its text up to the
    /// `}` that balances its `{`.
    fn atomic_inline_tag(&mut self, brace: Pos, name: Span) -> Parsed<Node> {
        self.skip_whitespace();
        self.inline_text(Whitespace::RemoveAll)?;
        self.next_char();
        Ok(self.atomic(brace, name))
    }

    fn atomic(&self, brace: Pos, name: Span) -> Node {
        Node::Inline(Inline {
            span: self.span(brace),
            name,
            rich: None,
        })
    }

    fn rich(
        &self,
        brace: Pos,
        name: Span,
        head: Option<Span>,
        content: (Vec<Node>, Option<Span>),
    ) -> Node {
        Node::Inline(Inline {
            span: self.span(brace),
            name,
            rich: Some(Rich {
                head,
                content: content.0,
                close: content.1,
            }),
        })
    }

    /// The inline tag parsers of javac's `createTagParsers`.
    #[allow(clippy::too_many_lines, reason = "one arm per tag, as in javac")]
    fn inline_tag_body(&mut self, tag: Tag, brace: Pos, name: Span) -> Parsed<Node> {
        match tag {
            Tag::Code | Tag::Literal => {
                self.inline_text(Whitespace::RemoveFirstSpace)?;
                self.next_char();
                Ok(self.atomic(brace, name))
            }
            Tag::DocRoot => {
                if self.cur.ch == '}' {
                    self.next_char();
                    return Ok(self.atomic(brace, name));
                }
                self.inline_text(Whitespace::RemoveAll)?;
                self.next_char();
                Err(ParseError)
            }
            Tag::Index => {
                self.skip_whitespace();
                if self.cur.ch == '}' {
                    return Err(ParseError);
                }
                let term = if self.cur.ch == '"' {
                    self.quoted_string()
                } else {
                    self.inline_word()
                };
                let term = term.ok_or(ParseError)?;
                self.skip_whitespace();
                let content = if self.cur.ch == '}' {
                    let close = self.cur.pos;
                    self.next_char();
                    (Vec::new(), Some(self.span(close)))
                } else {
                    self.inline_content()
                };
                Ok(self.rich(brace, name, Some(term), content))
            }
            Tag::InheritDoc => {
                let reference = self.reference()?;
                self.skip_whitespace();
                if self.cur.ch == '}' {
                    if self.newline && reference.is_some() {
                        self.give_up();
                        return Err(ParseError);
                    }
                    self.next_char();
                    return Ok(self.atomic(brace, name));
                }
                self.inline_text(Whitespace::RemoveAll)?;
                self.next_char();
                Err(ParseError)
            }
            Tag::Link | Tag::LinkPlain => {
                let reference = self.reference()?;
                let label = self.inline_content();
                Ok(self.rich(brace, name, reference, label))
            }
            Tag::Return | Tag::Summary => {
                let content = self.inline_content();
                Ok(self.rich(brace, name, None, content))
            }
            Tag::Snippet => self.snippet(brace, name),
            Tag::SystemProperty => {
                self.skip_whitespace();
                if self.cur.ch == '}' {
                    return Err(ParseError);
                }
                self.next_char();
                while !self.at_end() && java::is_unicode_identifier_part(self.cur.ch)
                    || self.cur.ch == '.'
                {
                    self.next_char();
                }
                self.skip_whitespace();
                let closed = self.cur.ch == '}';
                self.next_char();
                if closed {
                    Ok(self.atomic(brace, name))
                } else {
                    Err(ParseError)
                }
            }
            Tag::Value => {
                self.skip_whitespace();
                match self.cur.ch {
                    '%' => {
                        self.inline_word();
                        self.skip_whitespace();
                    }
                    '"' => {
                        self.quoted_string();
                        self.skip_whitespace();
                    }
                    _ => {}
                }
                let reference = self.reference()?;
                self.skip_whitespace();
                if self.cur.ch == '}' && !(self.newline && reference.is_some()) {
                    self.next_char();
                    Ok(self.atomic(brace, name))
                } else {
                    self.give_up();
                    Err(ParseError)
                }
            }
            _ => unreachable!("an inline tag"),
        }
    }

    fn snippet(&mut self, brace: Pos, name: Span) -> Parsed<Node> {
        self.skip_whitespace();
        self.snippet_attrs();
        if self.cur.ch == '}' {
            self.next_char();
            return Ok(self.atomic(brace, name));
        }
        if self.cur.ch == ':' {
            self.newline = false;
            self.next_char();
            while !self.at_end() && matches!(self.cur.ch, ' ' | '\t') {
                self.next_char();
            }
            if !self.newline {
                return Err(ParseError);
            }
            self.next_char();
            self.inline_text(Whitespace::RetainAll)?;
            self.next_char();
            return Ok(self.atomic(brace, name));
        }
        Err(ParseError)
    }

    /// The snippet tag's `tagAttrs`.
    fn snippet_attrs(&mut self) {
        let mut scratch = Vec::new();
        self.skip_whitespace();
        while !self.at_end() && self.is_identifier_start() {
            self.read_attribute_name();
            self.skip_whitespace();
            if self.cur.ch == '=' {
                self.next_char();
                self.skip_whitespace();
                if matches!(self.cur.ch, '\'' | '"') {
                    self.newline = false;
                    let quote = self.cur.ch;
                    self.next_char();
                    self.text_start = Some(self.cur.pos);
                    while !self.at_end() && self.cur.ch != quote {
                        self.next_char();
                    }
                    self.add_pending_text(&mut scratch, Some(self.cur.pos));
                    self.next_char();
                } else {
                    self.text_start = Some(self.cur.pos);
                    while !self.at_end()
                        && self.cur.ch != '}'
                        && self.cur.ch != ':'
                        && !is_unquoted_attr_value_terminator(self.cur.ch)
                    {
                        self.next_char();
                    }
                    self.add_pending_text(&mut scratch, Some(self.cur.pos));
                }
                self.skip_whitespace();
            }
            scratch.clear();
        }
    }

    /// javac's `inlineText`: plain text up to the `}` that balances the
    /// tag's `{`, which is left unread.
    fn inline_text(&mut self, whitespace: Whitespace) -> Parsed<Span> {
        match whitespace {
            Whitespace::RemoveAll => self.skip_whitespace(),
            Whitespace::RemoveFirstSpace => {
                if self.cur.ch == ' ' {
                    self.next_char();
                }
            }
            Whitespace::RetainAll => {}
        }
        let start = self.cur.pos;
        let mut depth = 1;
        while !self.at_end() {
            match self.cur.ch {
                ch if is_ws(ch) => {}
                '{' => {
                    self.newline = false;
                    self.last_non_white = Some(self.after_cur());
                    depth += 1;
                }
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        return Ok(self.span(start));
                    }
                    self.newline = false;
                    self.last_non_white = Some(self.after_cur());
                }
                _ => {
                    self.newline = false;
                    self.last_non_white = Some(self.after_cur());
                }
            }
            self.next_char();
        }
        Err(ParseError)
    }

    /// javac's `reference`: up to whitespace outside `()` and `<>`, or the
    /// tag's `}`; `None` when it is empty before a `}`.
    fn reference(&mut self) -> Parsed<Option<Span>> {
        let start = self.cur.pos;
        let mut depth = 0i32;
        while !self.at_end() {
            match self.cur.ch {
                ch if is_ws(ch) => {
                    if depth == 0 {
                        break;
                    }
                }
                '(' | '<' => {
                    self.newline = false;
                    depth += 1;
                }
                ')' | '>' => {
                    self.newline = false;
                    depth -= 1;
                }
                '}' => {
                    if self.cur.pos == start {
                        return Ok(None);
                    }
                    self.newline = false;
                    break;
                }
                '@' => {
                    if self.newline {
                        break;
                    }
                }
                _ => self.newline = false,
            }
            self.next_char();
        }
        // javac leaves a negative depth to `ReferenceParser`, which always
        // rejects it.
        if depth != 0 {
            return Err(ParseError);
        }
        Ok(Some(self.span(start)))
    }

    /// javac's `identifier`: a Java identifier after whitespace.
    fn identifier(&mut self) -> Parsed<Span> {
        self.skip_whitespace();
        if !java::is_java_identifier_start(self.cur.ch) {
            return Err(ParseError);
        }
        Ok(self.read_name(java::is_java_identifier_part))
    }

    /// javac's `quotedString`.
    fn quoted_string(&mut self) -> Option<Span> {
        self.newline = false;
        let start = self.cur.pos;
        self.next_char();
        while !self.at_end() {
            match self.cur.ch {
                ch if is_ws(ch) => {}
                '"' => {
                    self.next_char();
                    return Some(self.span(start));
                }
                '@' if self.newline => break,
                _ => {}
            }
            self.next_char();
        }
        None
    }

    /// javac's `inlineWord`.
    fn inline_word(&mut self) -> Option<Span> {
        let start = self.cur.pos;
        let mut depth = 0;
        while !self.at_end() {
            match self.cur.ch {
                ch if is_ws(ch) => return Some(self.span(start)),
                '@' if self.newline => break,
                '{' => depth += 1,
                '}' => {
                    if depth == 0 {
                        return Some(self.span(start));
                    }
                    depth -= 1;
                }
                _ => {}
            }
            self.newline = false;
            self.next_char();
        }
        None
    }

    /// javac's `inlineContent`.
    fn inline_content(&mut self) -> (Vec<Node>, Option<Span>) {
        self.skip_whitespace();
        self.content(Phase::Inline)
    }

    // Block tags.

    /// javac's `blockContent`.
    fn block_content(&mut self) -> Vec<Node> {
        while matches!(self.cur.ch, ' ' | '\t') && !self.at_end() {
            self.next_char();
        }
        self.content(Phase::Body).0
    }

    /// javac's `blockTag`, at an `@`.
    fn block_tag(&mut self) -> BlockTag {
        self.newline = false;
        let start = self.cur.pos;
        self.next_char();
        if !self.is_identifier_start() {
            self.block_content();
            return self.erroneous_tag(start);
        }
        let name = self.read_tag_name();
        let name_span = Span {
            start,
            end: name.end,
        };
        let parsed = match Tag::named(self.text(name)) {
            None => Ok((Vec::new(), Some(self.block_content()))),
            Some(tag) if tag.allows_block() => self.block_tag_body(tag),
            Some(_) => return self.erroneous_tag(start),
        };
        let Ok((args, body)) = parsed else {
            self.block_content();
            return self.erroneous_tag(start);
        };
        let mut head = vec![name_span];
        head.extend(args);
        BlockTag {
            span: self.span(start),
            head,
            body: Some(body.unwrap_or_default()),
        }
    }

    fn erroneous_tag(&mut self, start: Pos) -> BlockTag {
        let Node::Erroneous(span) = self.erroneous(start) else {
            unreachable!("an erroneous node");
        };
        BlockTag {
            span,
            head: vec![span],
            body: None,
        }
    }

    /// The block tag parsers of javac's `createTagParsers`: the tag's
    /// arguments, and its text.
    fn block_tag_body(&mut self, tag: Tag) -> Parsed<(Vec<Span>, Option<Vec<Node>>)> {
        match tag {
            Tag::Exception | Tag::Throws | Tag::Provides | Tag::Uses => {
                self.skip_whitespace();
                let reference = self.reference()?;
                Ok((reference.into_iter().collect(), Some(self.block_content())))
            }
            Tag::Param => {
                self.skip_whitespace();
                let start = self.cur.pos;
                let type_parameter = self.cur.ch == '<';
                if type_parameter {
                    self.next_char();
                }
                let identifier = self.identifier()?;
                if type_parameter {
                    if self.cur.ch != '>' {
                        return Err(ParseError);
                    }
                    self.next_char();
                }
                let name = if type_parameter {
                    self.span(start)
                } else {
                    identifier
                };
                self.skip_whitespace();
                Ok((vec![name], Some(self.block_content())))
            }
            Tag::See => {
                self.skip_whitespace();
                match self.cur.ch {
                    '"' => {
                        if let Some(string) = self.quoted_string() {
                            self.skip_whitespace();
                            if self.cur.ch == '@' || self.at_end() {
                                return Ok((vec![string], None));
                            }
                        }
                    }
                    '<' => return Ok((Vec::new(), Some(self.block_content()))),
                    ch if java::is_java_identifier_start(ch) || ch == '#' => {
                        let reference = self.reference()?;
                        return Ok((reference.into_iter().collect(), Some(self.block_content())));
                    }
                    _ => {}
                }
                Err(ParseError)
            }
            Tag::SerialField => {
                self.skip_whitespace();
                let name = self.identifier()?;
                self.skip_whitespace();
                let reference = self.reference()?;
                let mut args = vec![name];
                args.extend(reference);
                let body = if java::is_whitespace(self.cur.ch) {
                    self.skip_whitespace();
                    Some(self.block_content())
                } else {
                    None
                };
                Ok((args, body))
            }
            Tag::Spec => {
                self.skip_whitespace();
                let url = self.inline_word().ok_or(ParseError)?;
                if self.is_blank_span(url) {
                    return Err(ParseError);
                }
                self.skip_whitespace();
                let title = self.block_content();
                if title.iter().all(|node| match node {
                    Node::Text(span) => self.is_blank_span(*span),
                    _ => false,
                }) {
                    return Err(ParseError);
                }
                Ok((vec![url], Some(title)))
            }
            _ => Ok((Vec::new(), Some(self.block_content()))),
        }
    }

    /// Whether the text of `span` is only whitespace (`String.isBlank`).
    fn is_blank_span(&self, span: Span) -> bool {
        (span.start.line..=span.end.line).all(|line| {
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
            text[from..to].chars().all(java::is_whitespace)
        })
    }

    // Markdown: javac's `Markdown` class.

    fn markdown_state(&mut self) -> &mut Markdown {
        self.markdown.as_mut().expect("a Markdown comment")
    }

    /// javac's `initMarkdownLine`.
    fn init_markdown_line(&mut self) {
        if self.text_start.is_none() {
            self.text_start = Some(self.cur.pos);
        }
        self.markdown_update();
        if self.markdown_state().leaf == Leaf::IndentedCode {
            while !self.at_end() && self.cur.ch != '\n' {
                self.next_char();
            }
            self.last_non_white = Some(self.cur.pos);
        }
    }

    /// The `` ` `` and `~` case of javac's `content`: skips a code span or a
    /// fenced code block, in which no tag is read.
    fn markdown_code(&mut self) {
        self.newline = false;
        if self.text_start.is_none() {
            self.text_start = Some(self.cur.pos);
        }
        let opening = self.cur;
        self.last_non_white = Some(self.after_cur());
        let save_newline = self.newline;
        let fence = self.markdown_state().leaf == Leaf::FencedCode;
        if self.cur.ch == '`' || self.cur.ch == '~' && fence {
            match self.skip_code() {
                None if fence => self.last_non_white = Some(self.end()),
                None => {
                    self.seek(opening);
                    self.newline = save_newline;
                    self.next_char();
                }
                Some(end) => self.last_non_white = Some(end),
            }
        } else {
            self.next_char();
        }
    }

    /// `Markdown.update`: the containers and leaf block of the line that
    /// starts at the reader. Each pass of its loop costs a step, and so does
    /// each character that a line pattern looks at: javac's loop for a list
    /// item reads no character at the end of the comment (`/// + + ` never
    /// ends, in javac too), and a line of many quote or list markers is
    /// matched once for each.
    #[allow(clippy::too_many_lines, reason = "javac's method, rule for rule")]
    fn markdown_update(&mut self) {
        let previous = self.markdown_state().leaf;
        let indent = self.read_indent(0);
        let line_text = {
            let line = self.lines[self.cur.pos.line as usize].text;
            if self.cur.ch == '\n' || self.at_end() {
                ""
            } else {
                &line[self.cur.pos.off as usize..]
            }
        };
        let mut scan = LineScan::new(line_text);
        let mut peek = 0usize;
        let mut index = 0;
        loop {
            if self.charge(1) {
                return;
            }
            let containers = self.markdown_state().containers.len();
            if index == containers {
                if previous == Leaf::FencedCode {
                    return;
                }
                let code_indent = self
                    .markdown_state()
                    .containers
                    .last()
                    .map_or(0, |block| block.indent)
                    + 4;
                if indent >= code_indent && previous != Leaf::Paragraph {
                    let state = self.markdown_state();
                    state.leaf = Leaf::IndentedCode;
                    if state.leaf != previous {
                        state.block += 1;
                    }
                    return;
                }
                let (kind, cost) = scan.kind(peek);
                if self.charge(cost) {
                    return;
                }
                match kind {
                    LineKind::BulletedListItem | LineKind::OrderedListItem => {
                        let mut count = indent;
                        while self.cur.ch != ' ' && self.cur.ch != '\t' && !self.at_end() {
                            count += 1;
                            self.next_char();
                        }
                        let item_indent = self.read_indent(count);
                        let state = self.markdown_state();
                        state.containers.push(Container {
                            kind: ContainerKind::ListItem,
                            indent: item_indent,
                        });
                        index += 1;
                        peek = usize::try_from(item_indent - indent).unwrap_or(0);
                        state.block += 1;
                    }
                    LineKind::BlockQuote => {
                        let state = self.markdown_state();
                        state.containers.push(Container {
                            kind: ContainerKind::Quote,
                            indent: indent + 1,
                        });
                        index += 1;
                        peek += 1;
                        state.block += 1;
                    }
                    LineKind::Other => {
                        self.markdown_state().leaf = Leaf::Paragraph;
                        return;
                    }
                    LineKind::AtxHeader
                    | LineKind::SetextUnderline
                    | LineKind::ThematicBreak
                    | LineKind::Blank => {
                        let state = self.markdown_state();
                        state.leaf = Leaf::None;
                        state.block += 1;
                        return;
                    }
                    LineKind::CodeFence => {
                        let state = self.markdown_state();
                        state.leaf = Leaf::FencedCode;
                        state.block += 1;
                        return;
                    }
                }
            } else {
                let block = self.markdown_state().containers[index];
                let (kind, cost) = scan.kind(peek);
                if self.charge(cost) {
                    return;
                }
                match block.kind {
                    ContainerKind::ListItem => {
                        if indent >= block.indent || kind == LineKind::Blank {
                            index += 1;
                        } else if kind == LineKind::Other && previous == Leaf::Paragraph {
                            return;
                        } else {
                            let state = self.markdown_state();
                            state.containers.truncate(index);
                            state.block += 1;
                        }
                    }
                    ContainerKind::Quote => {
                        if kind == LineKind::BlockQuote {
                            index += 1;
                            peek += 1;
                        } else if kind == LineKind::Other && previous == Leaf::Paragraph {
                            return;
                        } else {
                            let state = self.markdown_state();
                            state.containers.truncate(index);
                            state.block += 1;
                        }
                    }
                }
            }
        }
    }

    /// `Markdown.skipCode`: from a run of `` ` `` or `~`, the position after
    /// the run that closes it, or `None` when there is none.
    fn skip_code(&mut self) -> Option<Pos> {
        let term = self.cur.ch;
        let count = self.count_run(term);
        let initial_leaf = self.markdown_state().leaf;
        let fenced = initial_leaf == Leaf::FencedCode;
        let initial_block = self.markdown_state().block;
        while !self.at_end() {
            if self.cur.ch == '\n' {
                self.next_char();
                self.markdown_update();
                let state = self.markdown_state();
                let (leaf, block) = (state.leaf, state.block);
                if fenced {
                    if leaf == Leaf::FencedCode
                        && self.cur.ch == term
                        && self.count_run(term) >= count
                        || self.markdown_state().block != initial_block
                    {
                        self.markdown_state().leaf = Leaf::None;
                        return Some(self.cur.pos);
                    }
                } else if block != initial_block {
                    return None;
                }
            } else {
                if self.cur.ch == term
                    && initial_leaf != Leaf::FencedCode
                    && self.count_run(term) == count
                {
                    return Some(self.cur.pos);
                }
                self.next_char();
            }
        }
        None
    }

    /// `Markdown.count`: the length of the run of `ch` at the reader, which
    /// moves past it.
    fn count_run(&mut self, ch: char) -> usize {
        let mut count = 1;
        self.next_char();
        while !self.at_end() && self.cur.ch == ch {
            count += 1;
            self.next_char();
        }
        count
    }

    /// `Markdown.readIndent`: spaces and tabs (to stops of 4) after
    /// `initial` columns.
    fn read_indent(&mut self, initial: i32) -> i32 {
        let mut indent = initial;
        while !self.at_end() {
            match self.cur.ch {
                ' ' => indent += 1,
                '\t' => indent += 4 - indent % 4,
                _ => return indent,
            }
            self.next_char();
        }
        indent
    }
}

/// `isUnquotedAttrValueTerminator`.
const fn is_unquoted_attr_value_terminator(ch: char) -> bool {
    matches!(
        ch,
        '\u{c}' | '\n' | '\r' | '\t' | ' ' | '"' | '\'' | '`' | '=' | '<' | '>'
    )
}

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
enum Leaf {
    #[default]
    None,
    Paragraph,
    FencedCode,
    IndentedCode,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum ContainerKind {
    ListItem,
    Quote,
}

#[derive(Clone, Copy, Debug)]
struct Container {
    kind: ContainerKind,
    indent: i32,
}

/// javac's `Markdown` state: the open container blocks, the leaf block, and
/// a serial number of blocks that ends a code span when it changes.
#[derive(Default)]
struct Markdown {
    containers: Vec<Container>,
    leaf: Leaf,
    block: u32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum LineKind {
    Blank,
    AtxHeader,
    SetextUnderline,
    ThematicBreak,
    CodeFence,
    BulletedListItem,
    OrderedListItem,
    BlockQuote,
    Other,
}

fn bytecount(bytes: &[u8], byte: u8) -> usize {
    bytes
        .iter()
        .fold(0, |count, &found| count + usize::from(found == byte))
}

/// The line that `Markdown.update` matches javac's line patterns against,
/// from a character index on (javac's `line.substring(peek)`), with what the
/// patterns ask of the rest of the line found once.
struct LineScan<'t> {
    text: &'t str,
    /// The character index and byte offset of the last suffix asked for.
    chars: usize,
    byte: usize,
    /// The start of the line's last line terminator other than `\n` and
    /// `\r` (which Java's `.` doesn't match), once looked for.
    terminator: OnceCell<Option<usize>>,
    /// The offset of the line's last `` ` ``, once looked for.
    backtick: OnceCell<Option<usize>>,
}

impl<'t> LineScan<'t> {
    const fn new(text: &'t str) -> Self {
        Self {
            text,
            chars: 0,
            byte: 0,
            terminator: OnceCell::new(),
            backtick: OnceCell::new(),
        }
    }

    /// The byte offset of character `index` (the line's end past it), and
    /// the characters walked to find it: the scan moves on from the last
    /// index asked for.
    fn offset(&mut self, index: usize) -> (usize, usize) {
        if index < self.chars {
            (self.chars, self.byte) = (0, 0);
        }
        let skip = index - self.chars;
        let walked: usize = self.text[self.byte..]
            .chars()
            .take(skip)
            .map(char::len_utf8)
            .sum();
        self.byte += walked;
        self.chars = index;
        (self.byte, skip)
    }

    /// Whether Java's `.*` matches the line from byte `at`: no line
    /// terminator comes after it.
    fn dot_star(&self, at: usize, cost: &mut usize) -> bool {
        let text = self.text;
        let last = *self.terminator.get_or_init(|| {
            *cost += text.len();
            text.rfind(['\u{85}', '\u{2028}', '\u{2029}'])
        });
        last.is_none_or(|found| found < at)
    }

    /// Whether no `` ` `` comes after byte `at`.
    fn no_backtick(&self, at: usize, cost: &mut usize) -> bool {
        let text = self.text;
        let last = *self.backtick.get_or_init(|| {
            *cost += text.len();
            text.rfind('`')
        });
        last.is_none_or(|found| found < at)
    }

    /// `Markdown.getLineKind` of the line from character `index`: the first
    /// of javac's line patterns that matches, and the steps it took.
    fn kind(&mut self, index: usize) -> (LineKind, usize) {
        let (at, mut cost) = self.offset(index);
        let text = &self.text[at..];
        let bytes = text.as_bytes();
        let blank: usize = text
            .chars()
            .take_while(|&ch| java::is_whitespace(ch))
            .map(char::len_utf8)
            .sum();
        cost += blank + 1;
        if blank == text.len() {
            return (LineKind::Blank, cost);
        }
        let first = bytes[0];
        // The patterns below all start with one of these, which is not
        // whitespace, so the line is no `[ \t]*` either.
        if !matches!(
            first,
            b'#' | b'=' | b'-' | b'+' | b'*' | b'_' | b'`' | b'~' | b'>' | b'0'..=b'9'
        ) {
            return (LineKind::Other, cost);
        }
        let is_tab_space = |byte: u8| byte == b' ' || byte == b'\t';
        let run = |byte: u8| bytes.iter().take_while(|&&found| found == byte).count();
        // ATX_HEADER: `#{1,6}([ \t].*|$)`.
        let hashes = bytes
            .iter()
            .take(7)
            .take_while(|&&byte| byte == b'#')
            .count();
        cost += hashes;
        if (1..=6).contains(&hashes)
            && (hashes == bytes.len()
                || is_tab_space(bytes[hashes]) && self.dot_star(at + hashes, &mut cost))
        {
            return (LineKind::AtxHeader, cost);
        }
        // SETEXT_UNDERLINE: `(=+|-+)[ \t]*`.
        if matches!(first, b'=' | b'-') {
            let marks = run(first);
            let spaces = bytes[marks..]
                .iter()
                .take_while(|&&byte| is_tab_space(byte))
                .count();
            cost += marks + spaces;
            if marks + spaces == bytes.len() {
                return (LineKind::SetextUnderline, cost);
            }
        }
        // THEMATIC_BREAK: three or more of one of `*`, `-`, `_`, each followed
        // by spaces and tabs.
        if matches!(first, b'*' | b'-' | b'_') {
            let scanned = bytes
                .iter()
                .take_while(|&&byte| byte == first || is_tab_space(byte))
                .count();
            cost += scanned;
            if scanned == bytes.len() && bytecount(bytes, first) >= 3 {
                return (LineKind::ThematicBreak, cost + scanned);
            }
        }
        // CODE_FENCE: `` `{3,}[^`]*+ `` or `~{3,}.*+`.
        if matches!(first, b'`' | b'~') {
            let fence = run(first);
            cost += fence;
            if fence >= 3
                && if first == b'`' {
                    self.no_backtick(at + fence, &mut cost)
                } else {
                    self.dot_star(at, &mut cost)
                }
            {
                return (LineKind::CodeFence, cost);
            }
        }
        // BULLETED_LIST_ITEM: `[-+*][ \t].*`.
        if matches!(first, b'-' | b'+' | b'*')
            && bytes.get(1).is_some_and(|&byte| is_tab_space(byte))
            && self.dot_star(at, &mut cost)
        {
            return (LineKind::BulletedListItem, cost);
        }
        // ORDERED_LIST_ITEM: `[0-9]{1,9}[.)][ \t].*`.
        let digits = bytes
            .iter()
            .take(10)
            .take_while(|byte| byte.is_ascii_digit())
            .count();
        cost += digits;
        if (1..=9).contains(&digits)
            && bytes
                .get(digits)
                .is_some_and(|&byte| byte == b'.' || byte == b')')
            && bytes
                .get(digits + 1)
                .is_some_and(|&byte| is_tab_space(byte))
            && self.dot_star(at, &mut cost)
        {
            return (LineKind::OrderedListItem, cost);
        }
        // BLOCK_QUOTE: `>.*`.
        if first == b'>' && self.dot_star(at, &mut cost) {
            return (LineKind::BlockQuote, cost);
        }
        (LineKind::Other, cost)
    }
}

#[cfg(test)]
mod tests {
    use super::parse;
    use crate::frame;

    /// javac retries an unclosed run of k backticks k times, each time
    /// reading to the end of its paragraph; the step budget stops such a
    /// reading, which takes quadratic time, so the comment prints as written.
    #[test]
    fn rescanning_input_stops_at_the_budget() {
        let unclosed = format!("/// a {} {}", "`".repeat(3000), "b ".repeat(3000));
        let (lines, _) = frame::markdown(&[unclosed.as_str()]).expect("a run");
        assert!(parse(&lines, true).is_none());
        let closed = format!("/// a {} {}", "`b` ".repeat(1500), "b ".repeat(3000));
        let (lines, _) = frame::markdown(&[closed.as_str()]).expect("a run");
        assert!(parse(&lines, true).is_some());
    }
}
