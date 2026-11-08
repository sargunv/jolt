//! The iterative printer: a command stack, newline coalescing, line suffixes,
//! and a bounded fit check.

use std::ops::ControlFlow;

use crate::comments::is_doc_line;
use crate::doc::{
    Kind, LINE_COMMENT, MULTILINE, RUN_END, Reindent, STICKY, Text, VERBATIM_START, comment_lines,
    display_width, lines,
};
use crate::{Doc, DocBuilder, FormatOptions, Sink};

/// The most commands one fit check visits before answering "does not fit".
pub(crate) const FIT_BUDGET: usize = 4096;

/// The whitespace verbatim lines lose at their ends: what separates tokens.
/// (Other Unicode whitespace is a token of its own.)
const BLANK: [char; 3] = [' ', '\t', '\x0c'];

/// Output is handed to the sink in chunks of about this size, so a sink that
/// compares (check mode) can stop the printer soon after a difference.
const CHUNK: usize = 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Flat,
    Break,
}

#[derive(Clone, Copy, Debug)]
struct Command {
    indent: u32,
    mode: Mode,
    doc: Doc,
}

/// What the current output line holds, for the text that comes next.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Line {
    /// Nothing has been written yet: the newlines and spaces before the first
    /// text are dropped.
    FileStart,
    /// A text.
    Text,
    /// A text that ends with a line comment: the next text starts a new line.
    Commented,
}

/// Whitespace requested but not yet written. The next text writes it first.
#[derive(Clone, Copy, Debug)]
enum Pending {
    None,
    Space,
    /// `count` newlines (2 make a blank line), then `indent` levels of
    /// indentation: those of the most recent request.
    Newlines {
        count: u8,
        indent: u32,
    },
}

/// What the printer needs from the language.
pub(crate) struct Language<J: Fn(char, char) -> bool> {
    /// [`FormatLanguage::joins`](crate::FormatLanguage::joins).
    pub(crate) joins: J,
    /// [`FormatLanguage::DOC_LINE_COMMENT`](crate::FormatLanguage::DOC_LINE_COMMENT).
    pub(crate) doc_line: Option<&'static str>,
}

struct Printer<'p, 'a, S: Sink + ?Sized, J: Fn(char, char) -> bool> {
    docs: &'p DocBuilder<'a>,
    options: FormatOptions,
    sink: &'p mut S,
    out: String,
    /// Display column of the end of the output.
    column: u32,
    /// Leading whitespace of the current output line.
    line_ws: String,
    /// The indent of the last line the printer started (a verbatim line's
    /// whitespace is the source's, not an indent).
    line_indent: u32,
    /// The base indentation of the verbatim node being printed.
    verbatim_base: String,
    pending: Pending,
    line: Line,
    /// The last character written on the current line, if it is not a space.
    last: Option<char>,
    /// The last text written ends a run of doc line comments.
    run_ended: bool,
    language: Language<J>,
    suffixes: Vec<Command>,
    /// The fit check's stack, kept to reuse its allocation.
    fit_stack: Vec<Fit>,
    stack: Vec<Command>,
}

pub(crate) fn print<S: Sink + ?Sized, J: Fn(char, char) -> bool>(
    docs: &DocBuilder<'_>,
    root: Doc,
    options: FormatOptions,
    language: Language<J>,
    final_newline: bool,
    sink: &mut S,
) -> ControlFlow<()> {
    let mut printer = Printer {
        docs,
        options,
        sink,
        out: String::new(),
        column: 0,
        line_ws: String::new(),
        line_indent: 0,
        verbatim_base: String::new(),
        pending: Pending::None,
        line: Line::FileStart,
        last: None,
        run_ended: false,
        language,
        suffixes: Vec::new(),
        fit_stack: Vec::new(),
        stack: vec![Command {
            indent: 0,
            mode: Mode::Break,
            doc: root,
        }],
    };
    printer.run()?;
    if final_newline {
        printer.out.push('\n');
    }
    printer.sink.write(&printer.out)
}

impl<S: Sink + ?Sized, J: Fn(char, char) -> bool> Printer<'_, '_, S, J> {
    fn run(&mut self) -> ControlFlow<()> {
        loop {
            let Some(command) = self.stack.pop() else {
                if self.suffixes.is_empty() {
                    return ControlFlow::Continue(());
                }
                self.flush_suffixes();
                continue;
            };
            let Command { indent, mode, doc } = command;
            match self.docs.kind(doc) {
                Kind::Nil | Kind::BreakParent => {}
                Kind::Text(index) => self.text(index, command)?,
                Kind::Space => self.space(),
                Kind::Line if mode == Mode::Flat => self.space(),
                Kind::SoftLine if mode == Mode::Flat => {}
                Kind::Line | Kind::SoftLine | Kind::HardLine => self.newline(command, 1),
                Kind::EmptyLine => self.newline(command, 2),
                Kind::Concat(start, len) => {
                    let children = &self.docs.children[start as usize..(start + len) as usize];
                    self.stack.extend(children.iter().rev().map(|&doc| Command {
                        indent,
                        mode,
                        doc,
                    }));
                }
                Kind::Indent(child) => self.stack.push(Command {
                    indent: indent + 1,
                    mode,
                    doc: child,
                }),
                Kind::Group(child) => {
                    let mode = if self.docs.breaks(doc) {
                        Mode::Break
                    } else if mode == Mode::Flat || self.fits(child, indent) {
                        Mode::Flat
                    } else {
                        Mode::Break
                    };
                    self.stack.push(Command {
                        indent,
                        mode,
                        doc: child,
                    });
                }
                Kind::IfBreak(broken, flat) => self.stack.push(Command {
                    indent,
                    mode,
                    doc: if mode == Mode::Break { broken } else { flat },
                }),
                Kind::LineSuffix(child) => self.suffixes.push(Command {
                    indent,
                    mode,
                    doc: child,
                }),
                Kind::Hug(hugged, broken) => {
                    let hugged = Fit::new(hugged, indent, Mode::Flat);
                    let doc = if mode == Mode::Break && self.measure(&[hugged], Probe::Hung(indent))
                    {
                        hugged.doc
                    } else {
                        broken
                    };
                    self.stack.push(Command { indent, mode, doc });
                }
                Kind::Hang(line, value) => {
                    let layout = if mode == Mode::Flat {
                        Hang::Flat
                    } else {
                        self.hang(doc, value, indent)
                    };
                    let (indent, line_mode, value_mode) = match layout {
                        Hang::Flat => (indent, Mode::Flat, Mode::Flat),
                        Hang::Here => (indent, Mode::Flat, Mode::Break),
                        Hang::Next => (indent + 1, Mode::Break, Mode::Break),
                    };
                    self.stack.push(Command {
                        indent,
                        mode: value_mode,
                        doc: value,
                    });
                    self.stack.push(Command {
                        indent,
                        mode: line_mode,
                        doc: line,
                    });
                }
            }
            if self.out.len() >= CHUNK {
                self.sink.write(&self.out)?;
                self.out.clear();
            }
        }
    }

    /// Prints the pending line suffixes before whatever is on the stack.
    fn flush_suffixes(&mut self) {
        let suffixes = std::mem::take(&mut self.suffixes);
        self.stack.extend(suffixes.into_iter().rev());
    }

    /// A space is dropped before a pending newline.
    fn space(&mut self) {
        if matches!(self.pending, Pending::None) {
            self.pending = Pending::Space;
        }
    }

    fn newline(&mut self, command: Command, count: u8) {
        if !self.suffixes.is_empty() {
            self.stack.push(command);
            self.flush_suffixes();
            return;
        }
        let count = match self.pending {
            Pending::Newlines { count: pending, .. } => pending.max(count),
            Pending::None | Pending::Space => count,
        };
        self.pending = Pending::Newlines {
            count,
            indent: command.indent,
        };
    }

    fn text(&mut self, index: u32, command: Command) -> ControlFlow<()> {
        let text = self.docs.texts[index as usize];
        if text.flags & STICKY == 0 && !self.suffixes.is_empty() {
            // A suffix is never carried past a token: print it, then start
            // this text's line.
            self.stack.push(command);
            self.stack.push(Command {
                doc: Doc::HARD_LINE,
                ..command
            });
            self.flush_suffixes();
            return ControlFlow::Continue(());
        }
        if self.line == Line::Commented && !matches!(self.pending, Pending::Newlines { .. }) {
            self.pending = Pending::Newlines {
                count: 1,
                indent: command.indent,
            };
        }
        // A doc line comment right after the end of a run starts another run:
        // a blank line keeps the two apart.
        if self.run_ended
            && let Pending::Newlines { count: 1, indent } = self.pending
            && self
                .language
                .doc_line
                .is_some_and(|opener| is_doc_line(text.text, opener))
        {
            self.pending = Pending::Newlines { count: 2, indent };
        }
        self.run_ended = text.flags & RUN_END != 0;
        self.flush_pending();
        let bytes = text.text.as_bytes();
        if let Some(last) = self.last {
            // Texts start with an ASCII byte in all but rare cases.
            let first = match bytes.first() {
                Some(&byte) if byte.is_ascii() => Some(char::from(byte)),
                _ => text.text.chars().next(),
            };
            if first.is_some_and(|first| (self.language.joins)(last, first)) {
                self.out.push(' ');
                self.column += 1;
            }
        }
        self.last = match bytes.last() {
            Some(b' ') | None => None,
            Some(&byte) if byte.is_ascii() => Some(char::from(byte)),
            _ => text.text.chars().next_back(),
        };
        if text.flags & VERBATIM_START != 0 {
            self.verbatim_base.clone_from(&self.line_ws);
        }
        self.write(&text);
        self.line = if text.flags & LINE_COMMENT == 0 {
            Line::Text
        } else {
            Line::Commented
        };
        ControlFlow::Continue(())
    }

    fn flush_pending(&mut self) {
        match std::mem::replace(&mut self.pending, Pending::None) {
            Pending::None => {}
            Pending::Space => {
                if self.line != Line::FileStart {
                    self.out.push(' ');
                    self.column += 1;
                    self.last = None;
                }
            }
            Pending::Newlines { count, indent } => {
                if self.line != Line::FileStart {
                    for _ in 0..count {
                        self.out.push('\n');
                    }
                }
                self.line_ws.clear();
                for _ in 0..indent {
                    if self.options.use_tabs {
                        self.line_ws.push('\t');
                    } else {
                        for _ in 0..self.options.indent_width {
                            self.line_ws.push(' ');
                        }
                    }
                }
                self.out.push_str(&self.line_ws);
                self.line_indent = indent;
                self.column = self.indent_width(indent);
                self.last = None;
            }
        }
    }

    fn indent_width(&self, indent: u32) -> u32 {
        let per_level = u32::from(self.options.indent_width);
        indent * per_level
    }

    fn write(&mut self, text: &Text<'_>) {
        if text.flags & MULTILINE == 0 {
            self.out.push_str(text.text);
            self.column += text.width;
            return;
        }
        match text.reindent {
            None => {
                let mut lines = lines(text.text);
                let first = lines.next().unwrap_or("");
                self.out.push_str(first);
                self.column += display_width(first);
                for line in lines {
                    self.out.push('\n');
                    self.out.push_str(line);
                    self.line_ws.clear();
                    self.line_ws.push_str(leading_ws(line));
                    self.column = display_width(line);
                }
            }
            Some(Reindent::Strip(prefix)) => {
                let base = std::mem::take(&mut self.verbatim_base);
                let mut lines = lines(text.text).peekable();
                let first = lines.next().unwrap_or("");
                let first = if lines.peek().is_some() {
                    first.trim_end_matches(BLANK)
                } else {
                    first
                };
                self.out.push_str(first);
                self.column += display_width(first);
                while let Some(line) = lines.next() {
                    let line = if lines.peek().is_some() {
                        line.trim_end_matches(BLANK)
                    } else {
                        line
                    };
                    self.out.push('\n');
                    // A blank line gets no indentation; the last line is never
                    // blank, since a text block follows it on the same line.
                    if line.trim_matches(BLANK).is_empty() && lines.peek().is_some() {
                        self.line_ws.clear();
                        self.column = 0;
                        continue;
                    }
                    let rest = line.strip_prefix(prefix).unwrap_or(line);
                    self.out.push_str(&base);
                    self.out.push_str(rest);
                    self.line_ws.clear();
                    self.line_ws.push_str(&base);
                    self.line_ws.push_str(leading_ws(rest));
                    self.column = display_width(&base) + display_width(rest);
                }
                self.verbatim_base = base;
            }
            Some(Reindent::Comment(indentation)) => {
                for (index, parts) in comment_lines(text.text, indentation).enumerate() {
                    if index > 0 {
                        self.out.push('\n');
                        self.column = 0;
                        if !parts.iter().all(|part| part.is_empty()) {
                            self.out.push_str(&self.line_ws);
                            self.column = display_width(&self.line_ws);
                        }
                    }
                    for part in parts {
                        self.out.push_str(part);
                        self.column += display_width(part);
                    }
                }
            }
        }
    }

    /// The layout of a hang (`line`, then `value`) in a broken context: at
    /// most three fit checks, each ending at the first line end it reaches.
    fn hang(&mut self, hang: Doc, value: Doc, indent: u32) -> Hang {
        let forced = self.docs.breaks(value);
        if !forced && self.fits(hang, indent) {
            return Hang::Flat;
        }
        // The pending comment ends this line.
        if !self.suffixes.is_empty() {
            return Hang::Next;
        }
        let next = Fit::new(value, indent + 1, Mode::Flat);
        if !forced && self.measure(&[next], Probe::NextLine) {
            return Hang::Next;
        }
        let Kind::Hang(line, _) = self.docs.kind(hang) else {
            unreachable!("a hang");
        };
        let here = [
            Fit::new(line, indent, Mode::Flat),
            Fit::new(value, indent, Mode::Break),
        ];
        if self.measure(&here, Probe::Hung(indent)) {
            Hang::Here
        } else {
            Hang::Next
        }
    }

    /// Whether `doc` fits flat on the rest of the current line, followed by
    /// the rest of the stack in its recorded modes. `indent` is the group's.
    fn fits(&mut self, doc: Doc, indent: u32) -> bool {
        self.measure(&[Fit::new(doc, indent, Mode::Flat)], Probe::Here)
    }

    /// Whether `entries`, then the rest of the stack in its recorded modes,
    /// fit as `probe` says.
    fn measure(&mut self, entries: &[Fit], probe: Probe) -> bool {
        let mut stack = std::mem::take(&mut self.fit_stack);
        stack.clear();
        stack.extend(entries.iter().rev());
        let fits = self.measure_stack(probe, &mut stack);
        self.fit_stack = stack;
        fits
    }

    #[allow(clippy::too_many_lines)]
    fn measure_stack(&self, probe: Probe, stack: &mut Vec<Fit>) -> bool {
        // Spaces at the start of a line are dropped, as when printing.
        let file_start = self.line == Line::FileStart;
        let (start, mut space, mut line_start) = match (probe, self.pending) {
            (Probe::NextLine, _) => {
                let indent = stack.last().map_or(0, |entry| entry.indent);
                (self.indent_width(indent), false, true)
            }
            (_, Pending::Newlines { indent, .. }) => (self.indent_width(indent), false, true),
            (_, Pending::Space) => (self.column, !file_start, file_start),
            (_, Pending::None) => (self.column, false, file_start),
        };
        let mut width = i64::from(self.options.line_width) - i64::from(start);
        let mut suffix_pending = !self.suffixes.is_empty();
        let mut hung = match probe {
            Probe::Hung(indent) => {
                // The hang's line prints a space, or nothing.
                let line = i64::from(stack.last().is_some_and(|line| line.doc == Doc::LINE));
                // The line the value starts on may be a continuation line,
                // indented deeper than the hang.
                let line_indent = match self.pending {
                    Pending::Newlines { indent, .. } => indent,
                    Pending::None | Pending::Space => self.line_indent,
                };
                Some(Hung {
                    base: line_indent.max(indent),
                    value_start: width - line,
                    next_width: i64::from(self.options.line_width)
                        - i64::from(self.indent_width(indent + 1)),
                    parts: 0,
                    last: 0,
                    opening: 0,
                    hug: None,
                    fallback: false,
                    forced: false,
                    deciding: false,
                })
            }
            Probe::Here | Probe::NextLine => None,
        };
        // A flat line was measured inside the group, so breaking the group
        // would change where its text starts.
        let mut saw_line = false;
        let mut rest = self.stack.len();
        let mut budget = FIT_BUDGET;
        loop {
            let entry = if let Some(entry) = stack.pop() {
                entry
            } else {
                if rest == 0 {
                    return hung.is_none_or(|hung| !hung.deciding);
                }
                rest -= 1;
                let command = self.stack[rest];
                Fit {
                    in_rest: true,
                    ..Fit::new(command.doc, command.indent, command.mode)
                }
            };
            if budget == 0 {
                return false;
            }
            budget -= 1;
            let Fit {
                mode, doc, in_rest, ..
            } = entry;
            // A hung probe measures the value's own line breaks; the rest of
            // the stack is measured as a group's check measures it.
            let value = hung.as_mut().filter(|_| !in_rest);
            match self.docs.kind(doc) {
                Kind::Nil => {}
                Kind::Text(index) => {
                    let text = &self.docs.texts[index as usize];
                    if suffix_pending && !entry.in_suffix && text.flags & STICKY == 0 {
                        // The text starts a new line after the pending
                        // comment. If breaking the group cannot move it,
                        // measure the group on from that line.
                        if let Some(hung) = hung.as_ref() {
                            return hung.line_ends(entry.indent, in_rest);
                        }
                        if in_rest || saw_line {
                            return in_rest;
                        }
                        suffix_pending = false;
                        space = false;
                        width = i64::from(self.options.line_width)
                            - i64::from(self.indent_width(entry.indent));
                    }
                    width -= i64::from(text.width) + i64::from(space);
                    space = false;
                    line_start = false;
                    if let Some(hung) = hung.as_mut() {
                        if hung.deciding {
                            if hung.too_wide_for_next(width) {
                                return true;
                            }
                        } else if width < 0 {
                            // The line breaks at the last breakable part's
                            // first line break, if there is one.
                            if !hung.fallback {
                                return false;
                            }
                            hung.deciding = true;
                            if hung.too_wide_for_next(width) {
                                return true;
                            }
                        }
                        // A hug's `open`: its line break comes next, and is
                        // another place the line can break.
                        if hung.opening != 0 && hung.opening == entry.group && !in_rest {
                            if hung.deciding {
                                return false;
                            }
                            hung.last = hung.opening;
                            hung.fallback = entry.indent + 1 > hung.base;
                            hung.hug = Some(entry.indent);
                            hung.opening = 0;
                        }
                    } else if width < 0 {
                        return false;
                    }
                    // A text block or multi-line comment ends the line it
                    // starts.
                    if text.flags & MULTILINE != 0 {
                        return match &hung {
                            Some(hung) => hung.line_ends(hung.base + 1, in_rest),
                            None => in_rest,
                        };
                    }
                }
                Kind::Space => space = !line_start,
                Kind::Line | Kind::SoftLine if mode == Mode::Flat => {
                    space |= doc == Doc::LINE && !line_start;
                    saw_line |= !in_rest;
                }
                Kind::Line | Kind::SoftLine | Kind::HardLine | Kind::EmptyLine
                    if entry.group != 0 && value.is_some() =>
                {
                    let hung = value.expect("a hung probe");
                    let required = hung.forced
                        || entry.inner
                        || matches!(self.docs.kind(doc), Kind::HardLine | Kind::EmptyLine);
                    if hung.deciding {
                        // A required line break that the hug's body does not
                        // start with: the hug fails on the next line too,
                        // and its list breaks there as it does here.
                        if let Some(hug) = hung.hug.filter(|_| entry.group == hung.last)
                            && required
                            && (entry.inner || entry.indent <= hug)
                        {
                            return true;
                        }
                        // Another place the line can break: the line up to it
                        // fits on the next line.
                        if required || entry.group != hung.last {
                            return false;
                        }
                    } else if entry.inner {
                        // A forced break inside a breakable part, as a line
                        // too long there: that part breaks on the next line
                        // too.
                        return hung.last == entry.group && hung.fallback;
                    } else {
                        if entry.group != hung.last {
                            hung.last = entry.group;
                            hung.hug = None;
                            hung.fallback = entry.indent > hung.base;
                        }
                        if required {
                            return hung.fallback;
                        }
                    }
                    // Measured flat; the line could break here.
                    space |= doc == Doc::LINE && !line_start;
                }
                Kind::Line | Kind::SoftLine | Kind::HardLine | Kind::EmptyLine => {
                    return match &hung {
                        // A forced break inside a group of a hug's flat
                        // contents: a list item breaks before its body.
                        Some(_) if entry.inner && !in_rest => false,
                        Some(hung) => hung.line_ends(entry.indent, in_rest),
                        None => in_rest,
                    };
                }
                Kind::BreakParent => {
                    if !in_rest && mode == Mode::Flat {
                        return false;
                    }
                }
                Kind::Concat(start, len) => {
                    let children = &self.docs.children[start as usize..(start + len) as usize];
                    stack.extend(children.iter().rev().map(|&doc| Fit { doc, ..entry }));
                }
                Kind::Indent(child) => stack.push(Fit {
                    doc: child,
                    indent: entry.indent + 1,
                    ..entry
                }),
                Kind::Group(child) => {
                    let forced = self.docs.breaks(doc);
                    let (mode, group, inner) = match value {
                        // The value's outermost groups are its breakable
                        // parts; the groups inside them are flat.
                        Some(hung) if mode == Mode::Break && entry.group == 0 && !entry.inner => {
                            hung.parts += 1;
                            hung.forced = forced;
                            (Mode::Break, hung.parts, false)
                        }
                        Some(_) if !forced => (Mode::Flat, entry.group, true),
                        Some(_) => (Mode::Break, entry.group, true),
                        None if forced => (Mode::Break, entry.group, entry.inner),
                        None => (mode, entry.group, entry.inner),
                    };
                    stack.push(Fit {
                        mode,
                        doc: child,
                        group,
                        inner,
                        ..entry
                    });
                }
                Kind::IfBreak(broken, flat) => stack.push(Fit {
                    doc: if mode == Mode::Break { broken } else { flat },
                    ..entry
                }),
                // The suffix (an end-of-line comment) counts toward the width
                // of the line it ends.
                Kind::LineSuffix(child) => {
                    suffix_pending = true;
                    stack.push(Fit {
                        mode: Mode::Flat,
                        doc: child,
                        in_suffix: true,
                        ..entry
                    });
                }
                // In a hung probe, outside the groups of a breakable part:
                // measured as `hugged`, a breakable part (or a place where
                // the part it is in breaks) whose first line break is the
                // one `broken` has after its first text, `open`. Its body's
                // line break is required, so the line ends there unless it
                // was too long before.
                Kind::Hug(hugged, _) if mode == Mode::Break && !entry.inner && value.is_some() => {
                    let hung = value.expect("a hung probe");
                    if hung.deciding && (entry.group == 0 || entry.group != hung.last) {
                        return false;
                    }
                    let group = if entry.group == 0 {
                        hung.parts += 1;
                        hung.parts
                    } else {
                        entry.group
                    };
                    if group != hung.last {
                        hung.opening = group;
                    }
                    stack.push(Fit {
                        mode: Mode::Flat,
                        doc: hugged,
                        group,
                        ..entry
                    });
                }
                // Measured as its broken layout, which is always available.
                Kind::Hug(_, broken) => stack.push(Fit {
                    doc: broken,
                    ..entry
                }),
                Kind::Hang(line, hung_value) => {
                    let (indent, mode, group) = match (mode, value) {
                        (Mode::Flat, _) => (entry.indent, Mode::Flat, entry.group),
                        // Inside a breakable part: measured flat with it.
                        (Mode::Break, Some(_)) if entry.group != 0 => {
                            (entry.indent, Mode::Flat, entry.group)
                        }
                        // In a group with a forced break inside a hug's flat
                        // contents: a line break in a group, as below.
                        (Mode::Break, Some(_)) if entry.inner => return false,
                        // A breakable part whose line break comes first.
                        (Mode::Break, Some(hung)) => {
                            if hung.deciding {
                                return false;
                            }
                            // The pending comment ends the line, and the hang
                            // breaks its line (see `hang`).
                            if suffix_pending {
                                return hung.line_ends(entry.indent + 1, false);
                            }
                            hung.parts += 1;
                            hung.last = hung.parts;
                            hung.hug = None;
                            hung.fallback = entry.indent + 1 > hung.base;
                            // A value that cannot be flat breaks there.
                            if self.docs.breaks(hung_value) {
                                return hung.fallback;
                            }
                            (entry.indent, Mode::Flat, hung.parts)
                        }
                        // Undecided in a broken context: measured as its
                        // last layout, which is always available (see
                        // `hang`).
                        (Mode::Break, None) => (entry.indent + 1, Mode::Break, entry.group),
                    };
                    stack.push(Fit {
                        doc: hung_value,
                        indent,
                        mode,
                        group,
                        ..entry
                    });
                    stack.push(Fit {
                        doc: line,
                        indent,
                        mode,
                        group,
                        ..entry
                    });
                }
            }
        }
    }
}

/// The state of a hung probe (see the printer's section in
/// `docs/internals/formatter.md`). The value is measured with its outermost
/// groups and hangs, its breakable parts, flat, noting where each could
/// break. The probe fits when the value's first line breaks at a required
/// line break, or at the first line break of the last breakable part before
/// the line is too long, and that part would break on the next line too.
struct Hung {
    /// The indent the value's line breaks must be deeper than: the hang's,
    /// or that of the line the value starts on, if deeper.
    base: u32,
    /// The width left on the line where the value starts.
    value_start: i64,
    /// The width the value would have on the next line.
    next_width: i64,
    /// The breakable parts entered so far; each is numbered in turn.
    parts: u32,
    /// The breakable part whose first line break was passed last, or 0.
    last: u32,
    /// The breakable part of a hug whose `open` comes next, or 0.
    opening: u32,
    /// The indent of the hug whose `open` was passed last, if `last` is its
    /// part.
    hug: Option<u32>,
    /// Whether the line can break at that line break: it is indented deeper
    /// than `base`.
    fallback: bool,
    /// Whether the breakable part entered last contains a forced break, so
    /// that its first line break is required.
    forced: bool,
    /// The line is too long, so it breaks at that line break. The probe goes
    /// on to see whether the part would break on the next line too.
    deciding: bool,
}

impl Hung {
    /// Whether the line fits when it ends at a required line break at
    /// `indent` (or at the next line break of the rest).
    const fn line_ends(&self, indent: u32, in_rest: bool) -> bool {
        !self.deciding && (in_rest || indent > self.base)
    }

    /// Whether the value up to here, with `width` left on its line, is
    /// wider than the next line.
    const fn too_wide_for_next(&self, width: i64) -> bool {
        self.value_start - width > self.next_width
    }
}

/// The three layouts of a hang in a broken context.
#[derive(Clone, Copy, Debug)]
enum Hang {
    /// `line` and the value flat.
    Flat,
    /// `line` flat, the value broken at the hang's indent.
    Here,
    /// `line` broken, the value broken one indent deeper.
    Next,
}

/// What a fit check measures.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Probe {
    /// The entries from the current position: a line break among them does
    /// not fit.
    Here,
    /// The entries from the start of a new line at their indent.
    NextLine,
    /// The entries from the current position, up to their first line break
    /// that is indented deeper than the given indent and the current line.
    Hung(u32),
}

/// One entry of the fit check's stack.
#[derive(Clone, Copy)]
struct Fit {
    mode: Mode,
    doc: Doc,
    indent: u32,
    /// In a hung probe: the breakable part the entry is in, or 0.
    group: u32,
    /// In a hung probe: the entry is in a group inside that part.
    inner: bool,
    in_rest: bool,
    in_suffix: bool,
}

impl Fit {
    const fn new(doc: Doc, indent: u32, mode: Mode) -> Self {
        Self {
            mode,
            doc,
            indent,
            group: 0,
            inner: false,
            in_rest: false,
            in_suffix: false,
        }
    }
}

fn leading_ws(line: &str) -> &str {
    let end = line
        .find(|ch: char| ch != ' ' && ch != '\t')
        .unwrap_or(line.len());
    &line[..end]
}

#[cfg(test)]
mod tests {
    use super::{FIT_BUDGET, Language, print};
    use crate::{DocBuilder, FormatOptions};

    /// The fit check is bounded: a group whose flat contents take more than
    /// `FIT_BUDGET` commands breaks, even though it would fit by width.
    #[test]
    fn fit_check_gives_up_after_its_budget() {
        let render = |lines: usize| {
            let mut docs = DocBuilder::with_capacity(0);
            let mut parts = vec![docs.text("a")];
            parts.extend((0..lines).map(|_| docs.soft_line()));
            parts.push(docs.text("b"));
            let contents = docs.concat(parts);
            let root = docs.group(contents);
            let mut out = String::new();
            let language = Language {
                joins: |_, _| false,
                doc_line: None,
            };
            let _ = print(
                &docs,
                root,
                FormatOptions::default(),
                language,
                true,
                &mut out,
            );
            out
        };
        assert_eq!(render(8), "ab\n");
        assert!(render(FIT_BUDGET).contains("a\n"));
    }
}
