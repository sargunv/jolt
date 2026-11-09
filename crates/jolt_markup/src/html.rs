//! The layout of Javadoc comments (`/**`): places the tokens of `javadoc`
//! in lines, blocks and lists. HTML collapses whitespace outside `<pre>`, so
//! the line breaks and blank lines it chooses don't change the rendered
//! page.

use crate::layout::{Join, Layout};
use crate::token::{Kind, Name, Sep, Token};

/// Why the next word starts a line, from weakest to strongest.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Why {
    /// A line break inside a paragraph (`<br>`, a list-like line, a ruler).
    Break,
    /// A new paragraph of text (after a blank line or a block).
    Text,
    /// A block element.
    Block,
}

/// What lies between the last word and the next, from weakest to strongest.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Gap {
    None,
    /// A block ended: a list, a heading or a `<pre>` region.
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

/// Lays out a Javadoc comment's tokens.
pub(crate) fn lay_out<'a>(tokens: &[Token<'a>], layout: &mut Layout<'_, 'a>) {
    let mut parser = Parser {
        layout,
        frames: Vec::new(),
        in_tags: false,
        next_line: None,
        gap: Gap::None,
        attach: false,
    };
    for token in tokens {
        parser.token(token);
    }
}

struct Parser<'p, 'l, 'a> {
    layout: &'p mut Layout<'l, 'a>,
    frames: Vec<Frame>,
    /// Whether a block tag has started.
    in_tags: bool,
    /// The next word starts a line.
    next_line: Option<Why>,
    /// What lies between the last word and the next.
    gap: Gap,
    /// The next word follows the last with nothing between (after `<p>` or
    /// `<li>`).
    attach: bool,
}

impl<'a> Parser<'_, '_, 'a> {
    fn token(&mut self, token: &Token<'a>) {
        match token.kind {
            Kind::Blank => {
                self.request(Why::Text);
                self.gap = Gap::Blank;
            }
            Kind::BlockTag => self.start_tag(),
            Kind::Break => self.request(Why::Break),
            Kind::Word => self.emit(token, token.glue),
            Kind::Element { name, closing } => self.element(token, name, closing),
            Kind::Pre => {
                self.request(Why::Block);
                self.emit(token, false);
            }
            Kind::Verbatim => self.layout.verbatim(token.text),
            Kind::PreEnd => {
                self.gap = self.gap.max(Gap::Block);
                self.request(Why::Text);
            }
        }
    }

    /// The column of a new line in the innermost open block.
    fn column(&self) -> u16 {
        self.frames.last().map_or(0, |frame| frame.column())
    }

    /// Asks for a new line before the next word; the strongest reason wins.
    fn request(&mut self, why: Why) {
        self.next_line = Some(self.next_line.map_or(why, |next| next.max(why)));
    }

    /// Starts a line at `indent`, with wraps at `hang`. A blank line goes
    /// before it in the description, outside lists, before a block or after
    /// a blank line or a block; elsewhere (in a list or in tag text) only a
    /// blank line of the source is kept.
    fn start(&mut self, why: Why, indent: u16, hang: u16) {
        let blank = match why {
            Why::Break => false,
            _ if self.frames.is_empty() => why == Why::Block || self.gap != Gap::None,
            _ => self.gap == Gap::Blank,
        };
        self.layout.line(blank, indent, hang);
    }

    /// Adds a word. `glue` keeps it on the line of the word before.
    fn emit(&mut self, token: &Token<'a>, glue: bool) {
        if let Some(why) = self.next_line.take() {
            // A word that a line break would turn into structure stays on
            // the line where the source has it.
            if token.sep != Sep::Line && !self.layout.at_start() && token.sticky {
                self.layout.word(token.text, Join::Space, true);
            } else {
                let column = self.column();
                self.start(why, column, column);
                self.layout.word(token.text, Join::Wrap, token.sticky);
            }
        } else {
            let join = match token.sep {
                _ if self.attach => Join::None,
                Sep::None => Join::None,
                Sep::Space | Sep::Line if glue => Join::Space,
                Sep::Space | Sep::Line => Join::Wrap,
            };
            self.layout.word(token.text, join, token.sticky);
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
        self.attach = false;
        self.gap = Gap::None;
    }

    /// Lays out a block-level element's tag.
    fn element(&mut self, token: &Token<'a>, name: Name, closing: bool) {
        match (name, closing) {
            (Name::P | Name::Heading, false) => {
                // `<li><p>` stays together.
                if !(self.attach && name == Name::P) {
                    self.request(Why::Block);
                }
                self.emit(token, false);
                self.attach = true;
            }
            (Name::P | Name::Heading, true) => {
                self.attach |= self.next_line.is_none();
                self.emit(token, true);
                if name == Name::Heading {
                    self.gap = self.gap.max(Gap::Block);
                    self.request(Why::Text);
                }
            }
            (Name::List, false) => {
                self.request(Why::Block);
                let open = self.column();
                self.emit(token, false);
                self.frames.push(Frame::List {
                    open,
                    column: open + 2,
                });
                self.request(Why::Text);
            }
            (Name::List, true) => {
                let Some(list) = self
                    .frames
                    .iter()
                    .rposition(|frame| matches!(frame, Frame::List { .. }))
                else {
                    self.emit(token, false);
                    return;
                };
                let Frame::List { open, .. } = self.frames[list] else {
                    unreachable!("a list frame");
                };
                self.frames.truncate(list);
                self.next_line = None;
                self.layout.line(self.gap == Gap::Blank, open, open);
                self.emit(token, false);
                self.gap = self.gap.max(Gap::Block);
                self.request(Why::Text);
            }
            (Name::Item, false) => {
                if matches!(self.frames.last(), Some(Frame::Item { .. })) {
                    self.frames.pop();
                }
                let line = self.column();
                self.next_line = None;
                self.start(Why::Block, line, line + 4);
                self.emit(token, false);
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
                    self.emit(token, true);
                    self.frames.pop();
                } else {
                    self.emit(token, true);
                }
            }
            (Name::Br, _) => {
                self.emit(token, true);
                self.request(Why::Break);
            }
            (Name::Pre, _) => self.emit(token, false),
        }
    }
}
