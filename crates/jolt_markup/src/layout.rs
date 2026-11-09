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
//! The flavour sets the frame: ` * ` inside `/**` and `*/` for Javadoc, and
//! the run's own `///` frame for Markdown, whose last text ends the line
//! ([`DocBuilder::line_comment`]).

use jolt_fmt::{Doc, DocBuilder, cat};

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
pub(crate) struct Layout<'d, 'a> {
    docs: &'d mut DocBuilder<'a>,
    flavour: Flavour<'a>,
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
    /// The last text and its index in `parts`.
    last: Option<(usize, &'a str)>,
}

impl<'d, 'a> Layout<'d, 'a> {
    pub(crate) const fn new(docs: &'d mut DocBuilder<'a>, flavour: Flavour<'a>) -> Self {
        Self {
            docs,
            flavour,
            parts: Vec::new(),
            pending: None,
            first: None,
            started: false,
            multiline: false,
            hang: 0,
            wraps: Vec::new(),
            last: None,
        }
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

    /// Keeps the comment off the one-line form (it has block tags).
    pub(crate) const fn multiline(&mut self) {
        self.multiline = true;
    }

    /// Whether the next word starts the comment.
    pub(crate) const fn at_start(&self) -> bool {
        !self.started
    }

    /// Adds a word, joined to the word before it by `join` unless it starts
    /// a line. A `sticky` word, which would start new structure at the start
    /// of a line, never follows a wrap point: it is glued to the word before
    /// it with a space.
    pub(crate) fn word(&mut self, text: &'a str, join: Join, sticky: bool) {
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

    /// Adds a text. In Markdown it is glued to a frame before it: the
    /// printer's join check would put a space between `///` and `*` or `=`
    /// (`///**` is not `/// **`). Not a text that starts with `/`, where
    /// that space keeps the line from starting with `////`, which ends a run.
    fn push_text(&mut self, text: &'a str) {
        self.last = Some((self.parts.len(), text));
        let doc = if self.markdown() && !text.starts_with('/') {
            self.docs.glued(text)
        } else {
            self.docs.text(text)
        };
        self.parts.push(doc);
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
        let started = std::mem::replace(&mut self.started, true);
        self.multiline = true;
        match self.flavour {
            Flavour::Javadoc => {
                let raw = if raw.is_empty() { "*" } else { raw };
                let line = self.docs.hard_line();
                let space = self.docs.text(&FRAME[..1]);
                self.parts.extend([line, space]);
                self.push_text(raw);
            }
            Flavour::Markdown { .. } => {
                if started {
                    self.parts.push(self.docs.hard_line());
                }
                self.push_text(raw);
            }
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
            Flavour::Markdown { frame } => frame.trim_end(),
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
