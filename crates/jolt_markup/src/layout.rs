//! The layout: turns a body parser's lines and words into `jolt_fmt` docs.
//!
//! Words are filled: between two words on a line is a wrap point, a group
//! that is a space when the next word fits on the line, and else a line break
//! and the frame. The printer's fit check for a wrap point measures the space,
//! the next word and the words glued to it, and stops at the next wrap point
//! (which is broken in the rest of the command stack, so its line break ends
//! the check). Each comment shares one wrap point doc per indentation, so a
//! word costs one text and two child slots.

use jolt_fmt::{Doc, DocBuilder, cat};

use crate::tags;

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

/// The frame of a doc block comment line and the spaces of its indentation.
const FRAME: &str = " *                                                                ";

/// A line that has been asked for but not started: it starts before the
/// next word or verbatim line.
#[derive(Clone, Copy, Debug)]
struct Start {
    blank: bool,
    indent: u16,
}

/// Builds the docs of one doc comment.
pub(crate) struct Layout<'d, 'a> {
    docs: &'d mut DocBuilder<'a>,
    /// Everything after the first line's start.
    parts: Vec<Doc>,
    pending: Option<Start>,
    /// The indentation of the first line, when it starts with a word.
    first: Option<u16>,
    /// Whether anything has been laid out.
    started: bool,
    /// Whether the comment has more than one line.
    multiline: bool,
    /// The indentation of the lines a wrap starts.
    hang: u16,
    /// The wrap point for each indentation used so far.
    wraps: Vec<(u16, Doc)>,
}

impl<'d, 'a> Layout<'d, 'a> {
    pub(crate) const fn new(docs: &'d mut DocBuilder<'a>) -> Self {
        Self {
            docs,
            parts: Vec::new(),
            pending: None,
            first: None,
            started: false,
            multiline: false,
            hang: 0,
            wraps: Vec::new(),
        }
    }

    /// Starts a new line before the next word, after a blank line when
    /// `blank` (but never at the start of the comment). Its text starts
    /// `indent` columns after the frame, and the lines a wrap starts in it
    /// `hang` columns. A later call before the next word replaces this one.
    pub(crate) fn line(&mut self, blank: bool, indent: u16, hang: u16) {
        self.pending = Some(Start { blank, indent });
        self.hang = hang;
    }

    /// Keeps the comment off the one-line form (it has block tags).
    pub(crate) const fn multiline(&mut self) {
        self.multiline = true;
    }

    /// Whether the next word starts the comment.
    pub(crate) const fn at_start(&self) -> bool {
        !self.started
    }

    /// Adds a word, joined to the word before it by `join` unless it starts
    /// a line. A word that would start new structure at the start of a line
    /// ([`tags::starts_structure`]) never follows a wrap point: it is glued
    /// to the word before it with a space.
    pub(crate) fn word(&mut self, text: &'a str, join: Join) {
        if let Some(start) = self.pending.take() {
            self.start_line(start);
        } else if !self.started {
            self.first = Some(0);
        } else {
            match join {
                Join::Wrap if !tags::starts_structure(text) => {
                    let wrap = self.wrap(self.hang);
                    self.parts.push(wrap);
                }
                Join::Wrap | Join::Space => self.parts.push(self.docs.space()),
                Join::None => {}
            }
        }
        self.started = true;
        let text = self.docs.text(text);
        self.parts.push(text);
    }

    /// Adds a line printed as written, `raw` from its frame on: only the
    /// comment's indentation changes, and no list or tag indentation is
    /// added. A blank line asked for before it is kept.
    pub(crate) fn verbatim(&mut self, raw: &'a str) {
        if let Some(start) = self.pending.take()
            && start.blank
            && self.started
        {
            self.blank_line();
        }
        self.started = true;
        self.multiline = true;
        let raw = if raw.is_empty() { "*" } else { raw };
        let space = self.docs.text(&FRAME[..1]);
        let text = self.docs.text(raw);
        let line = self.docs.hard_line();
        self.parts.extend([line, space, text]);
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
        let blank = self.docs.text(&FRAME[..2]);
        self.parts.push(self.docs.hard_line());
        self.parts.push(blank);
    }

    /// `" * "` and `indent` spaces.
    fn frame(&mut self, indent: u16) -> Doc {
        let mut width = 3 + usize::from(indent);
        let first = self.docs.text(&FRAME[..width.min(FRAME.len())]);
        if width <= FRAME.len() {
            return first;
        }
        let mut docs = vec![first];
        width -= FRAME.len();
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

    /// The comment: `/** text */` on one line when it is one line of words
    /// that fits, else the opener and closer on lines of their own.
    pub(crate) fn finish(mut self) -> Doc {
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
