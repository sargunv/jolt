//! The document IR: an append-only arena of layout commands whose texts are
//! borrowed from the source or `'static`.

use std::cell::Cell;

use unicode_width::UnicodeWidthChar;

/// A handle to a document node in a [`DocBuilder`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Doc(u32);

impl Doc {
    /// Nothing.
    pub const NIL: Self = Self(0);
    pub(crate) const SPACE: Self = Self(1);
    pub(crate) const LINE: Self = Self(2);
    const SOFT_LINE: Self = Self(3);
    pub(crate) const HARD_LINE: Self = Self(4);
    pub(crate) const EMPTY_LINE: Self = Self(5);
    pub(crate) const BREAK_PARENT: Self = Self(6);

    #[must_use]
    pub const fn is_nil(self) -> bool {
        self.0 == 0
    }

    const fn index(self) -> usize {
        self.0 as usize
    }
}

/// How the printer rewrites the continuation lines of a multi-line text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Reindent<'a> {
    /// A multi-line block comment: star blocks are re-aligned under the
    /// opening delimiter, other lines are trimmed (see [`comment_lines`]).
    /// The indentation of the source line the comment starts on, which a
    /// doc comment kept as written re-indents against.
    Comment(&'a str),
    /// Verbatim source: `prefix` is removed from each continuation line that
    /// starts with it.
    Strip(&'a str),
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum Kind {
    Nil,
    Text(u32),
    Space,
    Line,
    SoftLine,
    HardLine,
    EmptyLine,
    BreakParent,
    Concat(u32, u32),
    Group(Doc),
    Indent(Doc),
    IfBreak(Doc, Doc),
    LineSuffix(Doc),
    /// `line` (a `line` or `soft_line`), then the value.
    Hang(Doc, Doc),
    /// The hugged and the broken layout of a list.
    Hug(Doc, Doc),
    /// A doc comment laid out, and as written (see
    /// [`DocBuilder::doc_comment`]).
    DocComment(Doc, Doc),
}

const _: () = assert!(size_of::<Kind>() == 12);

/// Node flag: contains a forced break.
const BREAKS: u8 = 1;
/// Node flag: contains a line, a hang or an if-break outside any group
/// inside it, so its layout depends on the mode of a group around it.
const LINES: u8 = 2;

pub(crate) const STICKY: u8 = 1;
pub(crate) const LINE_COMMENT: u8 = 2;
pub(crate) const MULTILINE: u8 = 4;
/// The first text of a verbatim node: the printer records the indentation of
/// the current output line as the base its `Strip` pieces re-indent against.
pub(crate) const VERBATIM_START: u8 = 8;
/// The last text of a run of doc line comments: a doc line comment that
/// starts the next line belongs to another run, so the printer writes a blank
/// line before it.
pub(crate) const RUN_END: u8 = 16;
/// Written right after the text before it, without the space that
/// [`FormatLanguage::joins`](crate::FormatLanguage::joins) would put between
/// them.
pub(crate) const GLUED: u8 = 32;

#[derive(Clone, Copy, Debug)]
#[allow(
    clippy::struct_field_names,
    reason = "a piece of text and its properties"
)]
pub(crate) struct Text<'a> {
    pub(crate) text: &'a str,
    /// Display width of the first output line.
    pub(crate) width: u32,
    pub(crate) flags: u8,
    pub(crate) reindent: Option<Reindent<'a>>,
}

/// Builds documents. One builder lives for one format run.
pub struct DocBuilder<'a> {
    pub(crate) nodes: Vec<Kind>,
    /// Per node: [`BREAKS`] and [`LINES`].
    flags: Vec<u8>,
    pub(crate) children: Vec<Doc>,
    pub(crate) texts: Vec<Text<'a>>,
}

impl<'a> DocBuilder<'a> {
    /// A builder sized for a tree of `tokens` tokens, so the arena rarely
    /// grows while a file is formatted.
    pub(crate) fn with_capacity(tokens: usize) -> Self {
        let fixed = [
            (Kind::Nil, 0),
            (Kind::Space, 0),
            (Kind::Line, LINES),
            (Kind::SoftLine, LINES),
            (Kind::HardLine, BREAKS),
            (Kind::EmptyLine, BREAKS),
            (Kind::BreakParent, BREAKS),
        ];
        let mut nodes = Vec::with_capacity(fixed.len() + tokens * 3);
        let mut flags = Vec::with_capacity(nodes.capacity());
        for (kind, flag) in fixed {
            nodes.push(kind);
            flags.push(flag);
        }
        Self {
            nodes,
            flags,
            children: Vec::with_capacity(tokens * 3),
            texts: Vec::with_capacity(tokens + tokens / 4),
        }
    }

    pub(crate) fn kind(&self, doc: Doc) -> Kind {
        self.nodes[doc.index()]
    }

    pub(crate) fn breaks(&self, doc: Doc) -> bool {
        self.flags[doc.index()] & BREAKS != 0
    }

    fn flags(&self, doc: Doc) -> u8 {
        self.flags[doc.index()]
    }

    /// Whether `doc` contains a forced break (a hard line, a blank line, a
    /// break-parent or a multi-line text), so no group around it is flat.
    #[must_use]
    pub fn will_break(&self, doc: Doc) -> bool {
        self.breaks(doc)
    }

    fn push(&mut self, kind: Kind, flags: u8) -> Doc {
        let id = u32::try_from(self.nodes.len()).expect("document arena fits in u32");
        self.nodes.push(kind);
        self.flags.push(flags);
        Doc(id)
    }

    fn push_text(&mut self, text: Text<'a>) -> Doc {
        if text.text.is_empty() {
            return Doc::NIL;
        }
        let index = u32::try_from(self.texts.len()).expect("text table fits in u32");
        let flags = if text.flags & MULTILINE == 0 {
            0
        } else {
            BREAKS
        };
        self.texts.push(text);
        self.push(Kind::Text(index), flags)
    }

    /// Literal text. A text containing a line break is written raw and cannot
    /// be flat.
    pub fn text(&mut self, text: &'a str) -> Doc {
        self.text_with(text, None, 0)
    }

    /// A synthesized separator that prints before a pending line suffix
    /// instead of flushing it.
    pub fn sticky(&mut self, text: &'a str) -> Doc {
        self.push_text(Text {
            text,
            width: measure(text).0,
            flags: STICKY,
            reindent: None,
        })
    }

    /// A multi-line source slice whose continuation lines the printer rewrites
    /// (see [`Reindent`]). Cannot be flat when it spans lines.
    pub(crate) fn reindented(&mut self, text: &'a str, reindent: Reindent<'a>) -> Doc {
        self.text_with(text, Some(reindent), 0)
    }

    pub(crate) fn text_with(
        &mut self,
        text: &'a str,
        reindent: Option<Reindent<'a>>,
        flags: u8,
    ) -> Doc {
        let (width, multiline) = match reindent {
            None | Some(Reindent::Strip(_)) => measure(text),
            Some(Reindent::Comment(indentation)) => {
                // The first output line of a multi-line doc comment is `/**`
                // when its source's is, star block or not, so most doc
                // comments need no scan here.
                let bare = text.starts_with("/**")
                    && !text.starts_with("/**/")
                    && first_line(text).trim_end() == "/**";
                let width = if bare {
                    3
                } else {
                    let kept = Cell::new(false);
                    comment_lines(text, indentation, keeps_lines(text), &kept)
                        .next()
                        .map_or(0, |parts| {
                            parts.iter().map(|part| display_width(part)).sum()
                        })
                };
                (width, has_line_break(text))
            }
        };
        self.push_text(Text {
            text,
            width,
            flags: flags | if multiline { MULTILINE } else { 0 },
            reindent,
        })
    }

    /// The last text of a run of doc line comments (`///` lines): the line
    /// ends after it, as after any line comment, and a doc line comment that
    /// starts the next line is kept apart from the run by a blank line, so
    /// the two never read as one run. `glued` as for [`glued`](Self::glued).
    pub fn line_comment(&mut self, text: &'a str, glued: bool) -> Doc {
        let glue = if glued { GLUED } else { 0 };
        self.comment_text(text, LINE_COMMENT | RUN_END | glue)
    }

    /// Comment text written right after the text before it, even where the
    /// language's [`joins`](crate::FormatLanguage::joins) would put a space
    /// between them: the `\` of a Markdown hard break after a word, or a word
    /// after the `///` that frames it (`///**`).
    pub fn glued(&mut self, text: &'a str) -> Doc {
        self.comment_text(text, GLUED)
    }

    pub(crate) fn comment_text(&mut self, text: &'a str, flags: u8) -> Doc {
        self.push_text(Text {
            text,
            width: measure(text).0,
            flags,
            reindent: None,
        })
    }

    /// One space; dropped at line start and before a newline; consecutive
    /// spaces collapse.
    #[must_use]
    pub const fn space(&self) -> Doc {
        Doc::SPACE
    }

    /// A space when flat, a newline when broken.
    #[must_use]
    pub const fn line(&self) -> Doc {
        Doc::LINE
    }

    /// Nothing when flat, a newline when broken.
    #[must_use]
    pub const fn soft_line(&self) -> Doc {
        Doc::SOFT_LINE
    }

    /// A newline that forces every enclosing group to break.
    #[must_use]
    pub const fn hard_line(&self) -> Doc {
        Doc::HARD_LINE
    }

    /// A blank line; forces enclosing groups to break.
    #[must_use]
    pub const fn empty_line(&self) -> Doc {
        Doc::EMPTY_LINE
    }

    /// A blank line when `blank`, else a hard line.
    #[must_use]
    pub const fn line_break(&self, blank: bool) -> Doc {
        if blank {
            Doc::EMPTY_LINE
        } else {
            Doc::HARD_LINE
        }
    }

    /// Forces every enclosing group to break.
    #[must_use]
    pub const fn break_parent(&self) -> Doc {
        Doc::BREAK_PARENT
    }

    /// A sequence. NIL elements are dropped.
    ///
    /// # Panics
    ///
    /// When the arena outgrows `u32` indices.
    pub fn concat(&mut self, docs: impl IntoIterator<Item = Doc>) -> Doc {
        let start = self.children.len();
        let mut flags = 0;
        for doc in docs {
            if !doc.is_nil() {
                flags |= self.flags(doc);
                self.children.push(doc);
            }
        }
        match self.children.len() - start {
            0 => Doc::NIL,
            1 => self.children.pop().expect("one child"),
            len => {
                let start = u32::try_from(start).expect("children fit in u32");
                let len = u32::try_from(len).expect("children fit in u32");
                self.push(Kind::Concat(start, len), flags)
            }
        }
    }

    /// A node whose content is set later by [`fill`](Self::fill) (NIL until
    /// then), for a choice that depends on what is built after it. It counts
    /// as a forced break, so it must only hold docs that end their line.
    pub(crate) fn slot(&mut self) -> Doc {
        let start = u32::try_from(self.children.len()).expect("children fit in u32");
        self.children.push(Doc::NIL);
        self.push(Kind::Concat(start, 1), BREAKS)
    }

    /// Sets the content of `slot`, made by [`slot`](Self::slot).
    pub(crate) fn fill(&mut self, slot: Doc, doc: Doc) {
        let Kind::Concat(start, 1) = self.kind(slot) else {
            unreachable!("not a slot");
        };
        self.children[start as usize] = doc;
    }

    /// Flat if the contents fit, else broken. Contents without a line of
    /// their own (outside a nested group) are returned as they are: their
    /// layout does not depend on a group's mode.
    pub fn group(&mut self, doc: Doc) -> Doc {
        let flags = self.flags(doc);
        if flags & LINES == 0 {
            return doc;
        }
        self.push(Kind::Group(doc), flags & BREAKS)
    }

    /// One more indentation level for newlines inside.
    pub fn indent(&mut self, doc: Doc) -> Doc {
        if doc.is_nil() {
            return doc;
        }
        let flags = self.flags(doc);
        self.push(Kind::Indent(doc), flags)
    }

    /// `line` (a [`line`](Self::line) or [`soft_line`](Self::soft_line)),
    /// then `value`, laid out by the first that applies: flat, if it fits;
    /// `value` on the next line one indent deeper, if it fits there flat;
    /// `value` on this line, broken, if it fits up to its first line break
    /// that is indented deeper than here; else `value` on the next line one
    /// indent deeper, broken. A pending line suffix (`x = // c`) selects the
    /// last layout. See the printer for the cost of the choice.
    pub fn hang(&mut self, line: Doc, value: Doc) -> Doc {
        debug_assert!(
            matches!(line, Doc::LINE | Doc::SOFT_LINE),
            "hang after {line:?}"
        );
        let flags = self.flags(value) & BREAKS | LINES;
        self.push(Kind::Hang(line, value), flags)
    }

    /// `hugged` if its first line fits up to its first required line break,
    /// and that break is indented deeper than here and outside every group
    /// in `hugged`; else `broken`. `hugged` has no line break of its own and
    /// prints broken, so each of its groups breaks only if it does not fit.
    /// `broken` contains a forced break. One fit check, like a hang's (see
    /// the printer).
    pub fn hug(&mut self, hugged: Doc, broken: Doc) -> Doc {
        debug_assert!(self.breaks(broken), "a hug whose broken layout can be flat");
        let flags = self.flags(broken) & BREAKS;
        self.push(Kind::Hug(hugged, broken), flags)
    }

    /// A doc comment that the printer prints `laid_out` when it starts its
    /// output line and the line ends after it, and `as_written` otherwise.
    /// The next pass reads the comment where the output puts it, so laying
    /// it out exactly where it ends up alone on its lines is what keeps the
    /// output a fixpoint: a laid-out comment there stays there, and one
    /// that shares a line with code keeps sharing it. A fit check measures
    /// `as_written`: a comment laid out has lines of its own, which no
    /// measured line holds.
    pub(crate) fn doc_comment(&mut self, laid_out: Doc, as_written: Doc) -> Doc {
        let flags = self.flags(as_written) | LINES;
        self.push(Kind::DocComment(laid_out, as_written), flags)
    }

    /// `broken` or `flat` by the mode of the enclosing group.
    pub fn if_break(&mut self, broken: Doc, flat: Doc) -> Doc {
        let flags = self.flags(flat) & BREAKS | LINES;
        self.push(Kind::IfBreak(broken, flat), flags)
    }

    /// Deferred until the next newline, or the next text that is not sticky.
    pub fn line_suffix(&mut self, doc: Doc) -> Doc {
        if doc.is_nil() {
            return doc;
        }
        let flags = self.flags(doc) & LINES;
        self.push(Kind::LineSuffix(doc), flags)
    }
}

/// Builds a concatenation, evaluating every argument before borrowing the
/// builder: `cat![f; a, b, c]`.
#[macro_export]
macro_rules! cat {
    ($f:expr; $($doc:expr),* $(,)?) => {{
        let docs = [$($doc),*];
        ($f).concat(docs)
    }};
}

/// Builds a group of a concatenation: `group![f; a, b, c]`.
#[macro_export]
macro_rules! group {
    ($f:expr; $($doc:expr),* $(,)?) => {{
        let docs = [$($doc),*];
        let doc = ($f).concat(docs);
        ($f).group(doc)
    }};
}

/// Builds an indented concatenation: `indent![f; a, b, c]`.
#[macro_export]
macro_rules! indent {
    ($f:expr; $($doc:expr),* $(,)?) => {{
        let docs = [$($doc),*];
        let doc = ($f).concat(docs);
        ($f).indent(doc)
    }};
}

pub(crate) fn has_line_break(text: &str) -> bool {
    text.bytes().any(|byte| matches!(byte, b'\n' | b'\r'))
}

fn first_line(text: &str) -> &str {
    text.bytes()
        .position(|byte| matches!(byte, b'\n' | b'\r'))
        .map_or(text, |end| &text[..end])
}

/// Splits on `\r\n`, `\r` and `\n`.
pub(crate) fn lines(text: &str) -> impl Iterator<Item = &str> + Clone {
    let mut rest = Some(text);
    std::iter::from_fn(move || {
        let text = rest?;
        if let Some(end) = text.find(['\n', '\r']) {
            let skip = if text[end..].starts_with("\r\n") {
                2
            } else {
                1
            };
            rest = Some(&text[end + skip..]);
            Some(&text[..end])
        } else {
            rest = None;
            Some(text)
        }
    })
}

/// The display width of the first line of `text`, and whether `text` spans
/// lines, in one pass over its bytes. ASCII is measured as [`display_width`]
/// measures it; the rest of a text from its first non-ASCII byte goes to
/// [`display_width`].
fn measure(text: &str) -> (u32, bool) {
    let mut width = 0;
    for (index, byte) in text.bytes().enumerate() {
        match byte {
            b'\t' | b' '..=b'~' => width += 1,
            b'\n' | b'\r' => return (width, true),
            0x80.. => {
                // Every byte before `index` is ASCII, so it is a char boundary.
                let rest = &text[index..];
                return (
                    width + display_width(first_line(rest)),
                    has_line_break(rest),
                );
            }
            _ => {}
        }
    }
    (width, false)
}

/// Display width, with a tab counting one column.
pub(crate) fn display_width(text: &str) -> u32 {
    if text.is_ascii() {
        let printable = text
            .bytes()
            .filter(|byte| matches!(byte, b'\t' | b' '..=b'~'))
            .count();
        return u32::try_from(printable).expect("width fits in u32");
    }
    text.chars()
        .map(|ch| {
            if ch == '\t' {
                1
            } else {
                u32::try_from(ch.width().unwrap_or(0)).expect("char width fits in u32")
            }
        })
        .sum()
}

/// One output line of a block comment, as parts to concatenate.
pub(crate) type CommentLine<'t> = [&'t str; 3];

/// The output lines of a block comment: a multi-line doc comment (`/**`) is
/// a star block whose opener text moves to its own line; a multi-line block
/// comment whose continuation starts with `*` is a star block that keeps its
/// opener text. A star line keeps its text after the `*` (all of a doc
/// comment's leading `*`s, as javadoc reads it) and one space, so `<pre>`
/// code stays indented.
///
/// A doc comment that [`keeps_lines`] keeps its lines as written: each loses
/// `indentation`, the indentation of the source line the comment starts on,
/// when it starts with it. Any other comment keeps its continuation lines'
/// indentation relative to each other.
///
/// A doc comment keeps the trailing whitespace of its lines, and of the
/// blank lines between its text: javadoc reads it inside `<pre>`, code and
/// tag bodies, which a comment that isn't laid out was not read for. Its
/// whitespace is Java's (space, tab, form feed), so U+00A0 is text there.
/// Other comments lose it.
///
/// The caller says whether the doc comment is kept as written (`as_written`),
/// or finds out while laying it out as a star block: `kept` is set when a
/// continuation line without `*` or a ruler turns up, and the caller lays
/// the comment out again as written. Such comments are rare, so most doc
/// comments are scanned once.
pub(crate) fn comment_lines<'t>(
    text: &'t str,
    indentation: &'t str,
    as_written: bool,
    kept: &'t Cell<bool>,
) -> impl Iterator<Item = CommentLine<'t>> {
    let doc = text.starts_with("/**") && !text.starts_with("/**/");
    // A doc comment's whitespace, as javadoc reads it, and its trailing
    // whitespace, which it keeps.
    let ws: fn(char) -> bool = if doc { is_blank } else { char::is_whitespace };
    let trim_end = move |line: &'t str| if doc { line } else { line.trim_end() };
    let content = strip_delimiters(text);
    let multiline = has_line_break(content);
    let as_written = doc && multiline && (as_written || text.starts_with("/***"));
    let star_block = (doc && multiline && !as_written) || (!doc && is_star_block(content));
    // Exactly one of the two layouts is `Some`.
    let plain = (!star_block).then(|| {
        let mut lines = lines(text.trim());
        let first = trim_end(lines.next().unwrap_or("").trim_start());
        let shared = (!as_written).then(|| shared_indentation(lines.clone()));
        let rest = lines.map(move |line| {
            let line = trim_end(line);
            let line = match shared {
                Some(shared) => line.get(shared..).unwrap_or(""),
                None => line.strip_prefix(indentation).unwrap_or(line),
            };
            [line, "", ""]
        });
        std::iter::once([first, "", ""]).chain(rest)
    });
    let star = star_block.then(|| {
        let mut lines = lines(content);
        let mut first = [if doc { "/**" } else { "/*" }, "", ""];
        let mut has_content = false;
        if multiline {
            // The opener's text stays on its line. A doc comment's first line
            // has no indentation as javadoc reads it, so moving its text to a
            // line of its own would change the indentation all lines share,
            // which javadoc removes (inside `<pre>` and code too).
            let opener = lines.next().unwrap_or("");
            let opener = if doc {
                trim_end(opener.trim_start_matches(ws))
            } else {
                opener.trim()
            };
            if !opener.trim_matches(ws).is_empty() {
                has_content = true;
                first[1] = " ";
                first[2] = opener;
            }
        }
        // The blank lines since the last line with text, as their text.
        let mut blank_lines = Vec::new();
        let mut continuation = true;
        let body = lines
            .filter_map(move |line| {
                let (star, line) = match line.trim_start_matches(ws).strip_prefix('*') {
                    // A doc comment line loses its whole run of `*`s, as
                    // javadoc reads it, and keeps the text after them
                    // exactly (javadoc keeps it inside `<pre>`); a ruler
                    // keeps the comment as written.
                    Some(rest) if multiline && doc => {
                        if is_ruler(line) {
                            kept.set(true);
                        }
                        (" *", trim_end(rest.trim_start_matches('*')))
                    }
                    Some(rest) if multiline => {
                        let rest = rest.trim_end();
                        (" * ", rest.strip_prefix(' ').unwrap_or(rest))
                    }
                    _ => {
                        let line = trim_end(line.trim_start_matches(ws));
                        if doc && continuation && !line.is_empty() {
                            kept.set(true);
                        }
                        (" * ", line)
                    }
                };
                continuation = true;
                // javac skips spaces and tabs, but reads a form feed: a line
                // of one is text.
                let blank = if doc {
                    line.trim_start_matches([' ', '\t']).is_empty()
                } else {
                    line.trim_start_matches(ws).is_empty()
                };
                if blank {
                    if has_content {
                        blank_lines.push(line);
                    }
                    return None;
                }
                has_content = true;
                let blanks = std::mem::take(&mut blank_lines)
                    .into_iter()
                    .map(|line| [" *", line, ""]);
                Some(blanks.chain([[star, line, ""]]))
            })
            .flatten();
        // `*/` stays after the text of a last line that has some: a line
        // break there is one javadoc reads (it ends an unclosed `<pre>` with
        // one, and makes `{@snippet :` start a body).
        let close_after = doc && closes_after_text(content);
        let mut body = body.peekable();
        let body = std::iter::from_fn(move || {
            let mut line = body.next()?;
            if close_after && body.peek().is_none() {
                line[2] = "*/";
            }
            Some(line)
        });
        std::iter::once(first)
            .chain(body)
            .chain((!close_after).then_some([" */", "", ""]))
    });
    plain
        .into_iter()
        .flatten()
        .chain(star.into_iter().flatten())
}

/// The byte length of the leading spaces and tabs all non-blank `lines`
/// share.
fn shared_indentation<'t>(lines: impl Iterator<Item = &'t str>) -> usize {
    let mut shared: Option<&str> = None;
    for line in lines.filter(|line| !line.trim().is_empty()) {
        let indentation = &line[..line.len() - line.trim_start_matches([' ', '\t']).len()];
        let common = shared.map_or(indentation.len(), |shared| {
            shared
                .bytes()
                .zip(indentation.bytes())
                .take_while(|(left, right)| left == right)
                .count()
        });
        shared = Some(&indentation[..common]);
    }
    shared.map_or(0, str::len)
}

fn strip_delimiters(text: &str) -> &str {
    let text = text.trim();
    let body = text.strip_suffix("*/").unwrap_or(text);
    body.strip_prefix("/**")
        .or_else(|| body.strip_prefix("/*"))
        .unwrap_or(body)
}

/// Whether `text` is a multi-line doc comment (`/**`) kept as written (see
/// [`comment_lines`]) instead of being made a star block:
///
/// - a continuation line has text but no leading `*`: javadoc keeps the
///   indentation of such a line, so a star block could shift a code sample
///   in `<pre>`;
/// - it is a banner: it starts with `/***`, or a line is made only of two or
///   more `*`s (javadoc reads such a ruler as a blank line, which a star
///   block would print as one).
///
/// Most lines start with spaces and a `*`, which a scan for `\n` answers; any
/// other line is checked as text, only Java's whitespace (space, tab, form
/// feed) counting as blank.
pub(crate) fn keeps_lines(text: &str) -> bool {
    if !text.starts_with("/**") || text.starts_with("/**/") {
        return false;
    }
    let content = strip_delimiters(text);
    if !has_line_break(content) {
        return false;
    }
    if text.starts_with("/***") {
        return true;
    }
    let star_less = |line: &str| {
        let text = line.trim_start_matches(is_blank);
        !text.is_empty() && !text.starts_with('*')
    };
    let kept = |(index, line): (usize, &str)| (index > 0 && star_less(line)) || is_ruler(line);
    if content.as_bytes().contains(&b'\r') {
        return lines(content).enumerate().any(kept);
    }
    content.split('\n').enumerate().any(kept)
}

/// Whether `text` is a multi-line doc comment printed byte for byte: a later
/// line has text without a leading `*`, and another line has text, after
/// `/**` or after a leading `*`. javac removes the indentation that all of a
/// comment's lines with text share, counting a line's whole indentation when
/// it has no `*` and the indentation after the `*`s otherwise, so a new
/// indentation of the comment, which moves only the lines without `*`,
/// changes what javadoc reads. (When those are all the lines with text, they
/// move together and keep what javac reads.)
pub(crate) fn keeps_bytes(text: &str) -> bool {
    if !text.starts_with("/**") || text.starts_with("/**/") || text.starts_with("/***") {
        return false;
    }
    let content = strip_delimiters(text);
    let mut lines = lines(content);
    let mut starred_text = lines
        .next()
        .is_some_and(|first| !first.trim_matches(is_blank).is_empty());
    let mut star_less_text = false;
    for line in lines {
        let text = line.trim_start_matches(is_blank);
        match text.strip_prefix('*') {
            Some(rest) => {
                starred_text |= !rest
                    .trim_start_matches('*')
                    .trim_matches(is_blank)
                    .is_empty();
            }
            None => star_less_text |= !text.is_empty(),
        }
        if starred_text && star_less_text {
            return true;
        }
    }
    false
}

/// Whether the last line of a doc comment's `content` (between its
/// delimiters) has text before `*/`.
fn closes_after_text(content: &str) -> bool {
    !lines(content)
        .last()
        .unwrap_or("")
        .trim_start_matches(is_blank)
        .trim_start_matches('*')
        .trim_matches(is_blank)
        .is_empty()
}

/// Java's whitespace in a doc comment: a space, a tab or a form feed.
const fn is_blank(ch: char) -> bool {
    matches!(ch, ' ' | '\t' | '\u{c}')
}

/// Whether a doc comment line is a ruler made only of two or more `*`s.
fn is_ruler(line: &str) -> bool {
    let line = line.trim_matches(is_blank);
    line.len() >= 2 && line.bytes().all(|byte| byte == b'*')
}

fn is_star_block(content: &str) -> bool {
    let mut lines = lines(content);
    let opener = lines.next().unwrap_or("");
    let continuation = lines.find(|line| !line.trim().is_empty());
    let multiline = has_line_break(content);
    multiline
        && (opener.trim_start().starts_with('*')
            || continuation.is_some_and(|line| line.trim_start().starts_with('*'))
            || content.trim().is_empty())
}

#[cfg(test)]
mod tests {
    use super::{display_width, first_line, has_line_break, measure};

    /// The one-pass ASCII measure agrees with `display_width` on every ASCII
    /// byte, control bytes included, and around the non-ASCII fallback.
    #[test]
    fn measure_matches_display_width() {
        let mut texts: Vec<String> = (0..0x80u8)
            .map(|byte| format!("a{}b", char::from(byte)))
            .collect();
        texts
            .extend(["", "x\u{7}é\tz", "ab\u{4e2d}\u{301}c\r\nd", "é\n\u{4e2d}"].map(String::from));
        for text in &texts {
            let expected = (display_width(first_line(text)), has_line_break(text));
            assert_eq!(measure(text), expected, "{text:?}");
        }
    }
}
