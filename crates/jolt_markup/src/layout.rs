//! The layout: turns the lines and words that a flavour's layout places into
//! `jolt_fmt` docs.
//!
//! Words are filled: between two words on a line is a wrap point, a group
//! that is a space when the next word fits on the line, and else a line break
//! and the frame. The printer's fit check for a wrap point measures the space,
//! the next word and the words glued to it, and stops at the next wrap point
//! (which is broken in the rest of the command stack, so its line break ends
//! the check). Each comment shares one wrap point doc per indentation, so a
//! word costs one text and two child slots.
//!
//! A word may span lines (an atom such as a multi-line `{@code …}`): its
//! first line is placed like any word, each later line is printed as written
//! after a line break (see [`Layout::continuation`]), and the next word
//! follows its last line.
//!
//! The flavour sets the frame: ` * ` inside `/**` and `*/` for Javadoc, and
//! the run's own `///` frame for Markdown, whose last text ends the line
//! ([`DocBuilder::line_comment`]).

use jolt_fmt::{Doc, DocBuilder, cat};

use crate::frame::Line;
use crate::parser::{Pos, Span};

/// How a word joins the word before it on its line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Join {
    /// A wrap point: a space, or a line break when the word doesn't fit.
    Wrap,
    /// A space.
    Space,
    /// Nothing.
    None,
}

/// The whitespace that HTML and Markdown collapse at the end of a line:
/// Java's space, tab and form feed. Any other character, U+00A0 included,
/// is text.
pub(crate) const BLANK: [char; 3] = [' ', '\t', '\u{c}'];

/// The frame of a doc block comment line and the spaces of its indentation.
const FRAME: &str = " *                                                                ";

/// The comment's delimiters and the frame of its lines.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Flavour<'a> {
    /// `/**` and ` */` around lines framed by ` * `.
    Javadoc,
    /// Lines framed by `frame`: `///` and the run's shared indentation.
    Markdown { frame: &'a str },
}

/// A line that has been asked for but not started: it starts before the
/// next word or verbatim line.
#[derive(Clone, Copy, Debug)]
struct Start {
    blank: bool,
    indent: u16,
}

/// Builds the docs of one doc comment.
pub(crate) struct Layout<'d, 'l, 'a> {
    docs: &'d mut DocBuilder<'a>,
    flavour: Flavour<'a>,
    lines: &'l [Line<'a>],
    /// Everything after the first line's start.
    parts: Vec<Doc>,
    pending: Option<Start>,
    /// The indentation of the first line, when it starts with a word.
    first: Option<u16>,
    /// Whether anything has been laid out.
    started: bool,
    /// Whether the comment has more than one line, or can't be on one line.
    multiline: bool,
    /// The indentation of the lines a wrap starts.
    hang: u16,
    /// The wrap point for each indentation used so far.
    wraps: Vec<(u16, Doc)>,
    /// The last text and its index in `parts`.
    last: Option<(usize, &'a str)>,
    /// The lines whose trailing whitespace stays, by index (empty when
    /// none): their ends lie inside a node whose whitespace javadoc reads.
    kept_ends: Vec<bool>,
    /// The lines a Markdown block tag starts, in order.
    tag_lines: Vec<u32>,
}

impl<'d, 'l, 'a> Layout<'d, 'l, 'a> {
    pub(crate) const fn new(
        docs: &'d mut DocBuilder<'a>,
        flavour: Flavour<'a>,
        lines: &'l [Line<'a>],
    ) -> Self {
        Self {
            docs,
            flavour,
            lines,
            parts: Vec::new(),
            pending: None,
            first: None,
            started: false,
            multiline: false,
            hang: 0,
            wraps: Vec::new(),
            last: None,
            kept_ends: Vec::new(),
            tag_lines: Vec::new(),
        }
    }

    /// Keeps the trailing whitespace of each line of `span` but its last:
    /// the line ends inside a node whose whitespace javadoc reads (an inline
    /// tag's body, an attribute value, an HTML comment, a `<pre>` element, a
    /// raw text element).
    pub(crate) fn keep_ends(&mut self, span: Span) {
        self.keep_line_ends(span.start.line..span.end.line);
    }

    /// Keeps the trailing whitespace of the lines `lines` (a Markdown code
    /// block's, whose last line may end inside it).
    pub(crate) fn keep_line_ends(&mut self, lines: std::ops::Range<u32>) {
        if lines.is_empty() {
            return;
        }
        if self.kept_ends.is_empty() {
            self.kept_ends = vec![false; self.lines.len()];
        }
        for line in lines {
            self.kept_ends[line as usize] = true;
        }
    }

    /// Notes that a Markdown block tag starts on `line`, after the lines of
    /// the tags before it.
    pub(crate) fn tag_line(&mut self, line: u32) {
        self.tag_lines.push(line);
    }

    /// Whether a Markdown block tag starts on `line`.
    pub(crate) fn is_tag_line(&self, line: u32) -> bool {
        self.tag_lines.binary_search(&line).is_ok()
    }

    /// Whether line `line` keeps its trailing whitespace.
    fn kept_end(&self, line: u32) -> bool {
        self.kept_ends.get(line as usize).copied().unwrap_or(false)
    }

    const fn markdown(&self) -> bool {
        matches!(self.flavour, Flavour::Markdown { .. })
    }

    /// Starts a new line before the next word, after a blank line when
    /// `blank` (but never at the start of the comment). Its text starts
    /// `indent` columns after the frame, and the lines a wrap starts in it
    /// `hang` columns. A later call before the next word replaces this one.
    pub(crate) fn line(&mut self, blank: bool, indent: u16, hang: u16) {
        self.pending = Some(Start { blank, indent });
        self.hang = hang;
    }

    /// Makes the lines a wrap starts from here on start `hang` columns after
    /// the frame, as [`line`](Self::line) does, when the line break asked for
    /// before the next word is not taken.
    pub(crate) const fn hang(&mut self, hang: u16) {
        self.hang = hang;
    }

    /// Keeps the comment off the one-line form (it has block tags, or its
    /// first word would lose a leading `*` on the opener's line).
    pub(crate) const fn multiline(&mut self) {
        self.multiline = true;
    }

    /// Whether the next word starts the comment.
    pub(crate) const fn at_start(&self) -> bool {
        !self.started
    }

    /// The text of line `line` from byte `from` of its text, to byte `to`
    /// (or its end), as javadoc reads it.
    pub(crate) fn segment(&self, line: u32, from: u32, to: Option<u32>) -> &'a str {
        let text = self.lines[line as usize].text;
        let to = to.map_or(text.len(), |to| to as usize);
        &text[from as usize..to]
    }

    /// The text of `pos`'s line from `pos` on.
    pub(crate) fn text_at(&self, pos: Pos) -> &'a str {
        self.segment(pos.line, pos.off, None)
    }

    /// Adds a word or an atom that may span lines, joined to the word before
    /// it by `join` unless it starts a line. A `sticky` word, which would
    /// start new structure at the start of a line, never follows a wrap
    /// point: it is glued to the word before it with a space.
    pub(crate) fn word(&mut self, span: Span, join: Join, sticky: bool) {
        let first_end = (span.end.line == span.start.line).then_some(span.end.off);
        let mut first = self.segment(span.start.line, span.start.off, first_end);
        if first_end.is_none() && !self.markdown() && !self.kept_end(span.start.line) {
            first = first.trim_end_matches(BLANK);
        }
        self.push_word(first, join, sticky);
        for line in span.start.line + 1..=span.end.line {
            let to = (line == span.end.line).then_some(span.end.off);
            self.continuation(Pos { line, off: 0 }, to);
        }
    }

    /// Adds a word with this text (see [`word`](Self::word)).
    pub(crate) fn push_word(&mut self, text: &'a str, join: Join, sticky: bool) {
        if let Some(start) = self.pending.take() {
            self.start_line(start);
        } else if !self.started {
            self.first = Some(0);
        } else {
            match join {
                Join::Wrap if !sticky => {
                    let wrap = self.wrap(self.hang);
                    self.parts.push(wrap);
                }
                Join::Wrap | Join::Space => self.parts.push(self.docs.space()),
                Join::None => {}
            }
        }
        self.started = true;
        self.push_text(text);
    }

    /// Adds a text, glued to the text before it: the printer's join check
    /// would put a space between `&` and `&` (two erroneous entities), or
    /// between `///` and `*` (`///**` is not `/// **`). Not a Markdown text
    /// that starts with `/`, where that space keeps the line from starting
    /// with `////`, which ends a run.
    fn push_text(&mut self, text: &'a str) {
        if text.is_empty() {
            return;
        }
        self.last = Some((self.parts.len(), text));
        let doc = if self.markdown() && text.starts_with('/') {
            self.docs.text(text)
        } else {
            self.docs.glued(text)
        };
        self.parts.push(doc);
    }

    /// Prints line `pos.line` as written, from byte `pos.off` of its text to
    /// byte `to` (or its end), after a line break: only the comment's
    /// indentation changes, and no list or tag indentation is added. In a
    /// Javadoc comment the line gets the frame `" * "` (`" *"` and its tab
    /// when javadoc removed a tab of indentation from it, so that the tab
    /// stays), and in Markdown its own source frame, from its `///`.
    pub(crate) fn continuation(&mut self, pos: Pos, to: Option<u32>) {
        let started = std::mem::replace(&mut self.started, true);
        self.multiline = true;
        let line = self.lines[pos.line as usize];
        let text = self.segment(pos.line, pos.off, to);
        match self.flavour {
            Flavour::Javadoc => {
                let hard = self.docs.hard_line();
                self.parts.push(hard);
                // A span's end is its content's end: whitespace before it
                // is content (a form feed at the end of an unclosed `<pre>`).
                let text = if to.is_some() || self.kept_end(pos.line) {
                    text
                } else {
                    text.trim_end_matches(BLANK)
                };
                if text.is_empty() {
                    self.push_text(&FRAME[..2]);
                    return;
                }
                let lead = line.raw.len() - line.text.len();
                let tab = pos.off == 0 && line.raw[..lead].ends_with('\t');
                if tab {
                    self.push_text(&FRAME[..2]);
                    self.push_text(&line.raw[lead - 1..lead + text.len()]);
                } else {
                    self.push_text(&FRAME[..3]);
                    self.push_text(text);
                }
            }
            Flavour::Markdown { .. } => {
                if started {
                    self.parts.push(self.docs.hard_line());
                }
                let lead = line.raw.len() - line.text.len();
                let end = lead + pos.off as usize + text.len();
                self.push_text(&line.raw[..end]);
            }
        }
    }

    /// Prints whole lines as written (see [`continuation`](Self::continuation)),
    /// after the blank line asked for before them.
    pub(crate) fn verbatim_lines(&mut self, first: u32, last: u32) {
        if let Some(start) = self.pending.take()
            && start.blank
            && self.started
        {
            self.blank_line();
        }
        for line in first..=last {
            self.verbatim_line(line);
        }
    }

    /// Prints a Markdown line exactly as written, trailing whitespace
    /// included.
    pub(crate) fn exact_line(&mut self, line: u32) {
        let started = std::mem::replace(&mut self.started, true);
        self.multiline = true;
        if started {
            self.parts.push(self.docs.hard_line());
        }
        self.push_text(self.lines[line as usize].raw);
    }

    /// Prints one whole line as written. A Markdown line keeps two or more
    /// trailing spaces after text before a line with text that starts no
    /// block tag (a hard break, in a paragraph of a list or a quote; before
    /// a tag, the part ends there), and a line whose text ends with `\` or
    /// that [keeps its end](Self::keep_ends) all of them.
    fn verbatim_line(&mut self, line: u32) {
        if self.markdown() {
            let started = std::mem::replace(&mut self.started, true);
            self.multiline = true;
            if started {
                self.parts.push(self.docs.hard_line());
            }
            let source = self.lines[line as usize];
            let trimmed = source.raw.trim_end_matches(BLANK);
            let hard_break = source.raw[trimmed.len()..].starts_with("  ")
                && !source.is_blank()
                && self
                    .lines
                    .get(line as usize + 1)
                    .is_some_and(|next| !next.is_blank())
                && !self.is_tag_line(line + 1);
            // A `\` before the trailing whitespace would end the line, a
            // hard break.
            let kept = hard_break || trimmed.ends_with('\\') || self.kept_end(line);
            self.push_text(if kept { source.raw } else { trimmed });
        } else {
            self.continuation(Pos { line, off: 0 }, None);
        }
    }

    /// A blank Markdown line, printed as its frame.
    pub(crate) fn blank(&mut self) {
        self.pending = None;
        if self.started {
            self.blank_line();
        }
    }

    fn start_line(&mut self, start: Start) {
        if self.started {
            self.multiline = true;
            if start.blank {
                self.blank_line();
            }
            let frame = self.frame(start.indent);
            self.parts.push(self.docs.hard_line());
            self.parts.push(frame);
        } else {
            self.first = Some(start.indent);
        }
    }

    fn blank_line(&mut self) {
        let blank = match self.flavour {
            Flavour::Javadoc => &FRAME[..2],
            Flavour::Markdown { frame } => frame.trim_end_matches(crate::java::is_whitespace),
        };
        self.parts.push(self.docs.hard_line());
        self.push_text(blank);
    }

    /// The frame (`" * "`, or the Markdown frame) and `indent` spaces.
    fn frame(&mut self, indent: u16) -> Doc {
        if let Flavour::Markdown { frame } = self.flavour {
            let frame = self.docs.text(frame);
            if indent == 0 {
                return frame;
            }
            let spaces = self.spaces(indent);
            return self.docs.concat([frame, spaces]);
        }
        let width = 3 + usize::from(indent);
        let first = self.docs.text(&FRAME[..width.min(FRAME.len())]);
        if width <= FRAME.len() {
            return first;
        }
        let rest = u16::try_from(width - FRAME.len()).unwrap_or(u16::MAX);
        let rest = self.spaces(rest);
        self.docs.concat([first, rest])
    }

    /// `width` spaces.
    fn spaces(&mut self, width: u16) -> Doc {
        let mut width = usize::from(width);
        let mut docs = Vec::new();
        while width > 0 {
            let chunk = width.min(FRAME.len() - 2);
            docs.push(self.docs.text(&FRAME[2..2 + chunk]));
            width -= chunk;
        }
        self.docs.concat(docs)
    }

    /// The wrap point whose line break starts a line at `indent`.
    fn wrap(&mut self, indent: u16) -> Doc {
        if let Some(&(_, wrap)) = self.wraps.iter().find(|(at, _)| *at == indent) {
            return wrap;
        }
        let frame = self.frame(indent);
        let broken = cat![self.docs; self.docs.soft_line(), frame];
        let line = self.docs.line();
        let choice = self.docs.if_break(broken, line);
        let wrap = self.docs.group(choice);
        self.wraps.push((indent, wrap));
        wrap
    }

    /// Adds a line break inside a Markdown paragraph after the last word (a
    /// hard break, or another line break kept as written): the next word
    /// starts a line at `indent`, its source indentation, or the paragraph's
    /// when that is deeper.
    pub(crate) fn hard_break(&mut self, indent: u16) {
        let hang = self.hang;
        self.line(false, indent.max(hang), hang);
    }

    /// The comment. For Javadoc, `/** text */` on one line when it is one
    /// line of words that fits, else the opener and closer on lines of their
    /// own. For Markdown, the lines, the last ending the line.
    pub(crate) fn finish(mut self) -> Doc {
        if let Flavour::Markdown { .. } = self.flavour {
            if let Some((index, text)) = self.last {
                self.parts[index] = self.docs.line_comment(text, !text.starts_with('/'));
            }
            let mut all = Vec::with_capacity(self.parts.len() + 1);
            if let Some(indent) = self.first {
                all.push(self.frame(indent));
            }
            all.append(&mut self.parts);
            return self.docs.concat(all);
        }
        let docs = &mut *self.docs;
        let open = docs.text("/**");
        let close = docs.text(" */");
        if !self.started {
            return docs.concat([open, close]);
        }
        match self.first {
            Some(indent) if !self.multiline => {
                let frame = self.frame(indent);
                let docs = &mut *self.docs;
                let broken_open = cat![docs; docs.soft_line(), frame];
                let space = docs.text(&FRAME[..1]);
                let open_break = docs.if_break(broken_open, space);
                let broken_close = cat![docs; docs.soft_line(), close];
                let close_break = docs.if_break(broken_close, close);
                let mut all = Vec::with_capacity(self.parts.len() + 3);
                all.extend([open, open_break]);
                all.append(&mut self.parts);
                all.push(close_break);
                let body = docs.concat(all);
                docs.group(body)
            }
            first => {
                let mut all = Vec::with_capacity(self.parts.len() + 5);
                all.push(open);
                if let Some(indent) = first {
                    let frame = self.frame(indent);
                    all.extend([self.docs.hard_line(), frame]);
                }
                all.append(&mut self.parts);
                all.extend([self.docs.hard_line(), close]);
                self.docs.concat(all)
            }
        }
    }
}
