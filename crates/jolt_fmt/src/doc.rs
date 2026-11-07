//! The document IR: an append-only arena of layout commands whose texts are
//! borrowed from the source or `'static`.

use unicode_width::UnicodeWidthChar;

/// A handle to a document node in a [`DocBuilder`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Doc(u32);

impl Doc {
    /// Nothing.
    pub const NIL: Self = Self(0);
    pub(crate) const SPACE: Self = Self(1);
    const LINE: Self = Self(2);
    const SOFT_LINE: Self = Self(3);
    pub(crate) const HARD_LINE: Self = Self(4);
    const EMPTY_LINE: Self = Self(5);
    pub(crate) const BREAK_PARENT: Self = Self(6);

    #[must_use]
    pub const fn is_nil(self) -> bool {
        self.0 == 0
    }

    const fn index(self) -> usize {
        self.0 as usize
    }
}

/// Identifies a group whose break decision an [`DocBuilder::if_break`] reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GroupId(u32);

/// How the printer rewrites the continuation lines of a multi-line text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reindent<'a> {
    /// A multi-line block comment: star blocks are re-aligned under the
    /// opening delimiter, other lines are trimmed (see [`comment_lines`]).
    Comment,
    /// A single-line doc comment expanded into a three-line star block.
    ExpandedDoc,
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
    Group(Doc, u32),
    Indent(Doc),
    IfBreak(Doc, Doc, u32),
    LineSuffix(Doc),
}

const _: () = assert!(size_of::<Kind>() == 16);

pub(crate) const STICKY: u8 = 1;
pub(crate) const LINE_COMMENT: u8 = 2;
pub(crate) const MULTILINE: u8 = 4;
/// The first text of a verbatim node: the printer records the indentation of
/// the current output line as the base its `Strip` pieces re-indent against.
pub(crate) const VERBATIM_START: u8 = 8;
/// An inline trailing comment: its width counts for fitting only when more
/// text follows it on the line, as for the end-of-line comment it becomes when
/// a line break follows it.
pub(crate) const TRAILING_COMMENT: u8 = 16;

#[derive(Clone, Copy, Debug)]
#[allow(clippy::struct_field_names)]
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
    breaks: Vec<bool>,
    pub(crate) children: Vec<Doc>,
    pub(crate) texts: Vec<Text<'a>>,
    groups: u32,
}

impl Default for DocBuilder<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> DocBuilder<'a> {
    #[must_use]
    pub fn new() -> Self {
        let fixed = [
            (Kind::Nil, false),
            (Kind::Space, false),
            (Kind::Line, false),
            (Kind::SoftLine, false),
            (Kind::HardLine, true),
            (Kind::EmptyLine, true),
            (Kind::BreakParent, true),
        ];
        Self {
            nodes: fixed.iter().map(|&(kind, _)| kind).collect(),
            breaks: fixed.iter().map(|&(_, breaks)| breaks).collect(),
            children: Vec::new(),
            texts: Vec::new(),
            groups: 0,
        }
    }

    pub(crate) fn kind(&self, doc: Doc) -> Kind {
        self.nodes[doc.index()]
    }

    pub(crate) fn breaks(&self, doc: Doc) -> bool {
        self.breaks[doc.index()]
    }

    /// Whether `doc` contains a forced break (a hard line, a blank line, a
    /// break-parent or a multi-line text), so no group around it is flat.
    #[must_use]
    pub fn will_break(&self, doc: Doc) -> bool {
        self.breaks(doc)
    }

    pub(crate) const fn group_count(&self) -> usize {
        self.groups as usize
    }

    fn push(&mut self, kind: Kind, breaks: bool) -> Doc {
        let id = u32::try_from(self.nodes.len()).expect("document arena fits in u32");
        self.nodes.push(kind);
        self.breaks.push(breaks);
        Doc(id)
    }

    fn push_text(&mut self, text: Text<'a>) -> Doc {
        if text.text.is_empty() {
            return Doc::NIL;
        }
        let index = u32::try_from(self.texts.len()).expect("text table fits in u32");
        let breaks = text.flags & MULTILINE != 0;
        self.texts.push(text);
        self.push(Kind::Text(index), breaks)
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
            width: display_width(text),
            flags: STICKY,
            reindent: None,
        })
    }

    /// A multi-line source slice whose continuation lines the printer rewrites
    /// (see [`Reindent`]). Cannot be flat when it spans lines.
    pub fn reindented(&mut self, text: &'a str, reindent: Reindent<'a>) -> Doc {
        self.text_with(text, Some(reindent), 0)
    }

    pub(crate) fn text_with(
        &mut self,
        text: &'a str,
        reindent: Option<Reindent<'a>>,
        flags: u8,
    ) -> Doc {
        let width = match reindent {
            None | Some(Reindent::Strip(_)) => display_width(first_line(text)),
            Some(Reindent::Comment | Reindent::ExpandedDoc) => {
                let mut width = None;
                comment_lines(text, reindent == Some(Reindent::ExpandedDoc), |parts| {
                    width.get_or_insert_with(|| parts.iter().map(|part| display_width(part)).sum());
                });
                width.unwrap_or(0)
            }
        };
        let multiline = reindent == Some(Reindent::ExpandedDoc) || has_line_break(text);
        self.push_text(Text {
            text,
            width,
            flags: flags | if multiline { MULTILINE } else { 0 },
            reindent,
        })
    }

    pub(crate) fn comment_text(&mut self, text: &'a str, flags: u8) -> Doc {
        self.push_text(Text {
            text,
            width: display_width(text),
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
        let mut breaks = false;
        for doc in docs {
            if !doc.is_nil() {
                breaks |= self.breaks(doc);
                self.children.push(doc);
            }
        }
        match self.children.len() - start {
            0 => Doc::NIL,
            1 => self.children.pop().expect("one child"),
            len => {
                let start = u32::try_from(start).expect("children fit in u32");
                let len = u32::try_from(len).expect("children fit in u32");
                self.push(Kind::Concat(start, len), breaks)
            }
        }
    }

    /// Flat if the contents fit, else broken.
    pub fn group(&mut self, doc: Doc) -> Doc {
        if doc.is_nil() {
            return doc;
        }
        let breaks = self.breaks(doc);
        self.push(Kind::Group(doc, 0), breaks)
    }

    /// A new group id for [`Self::group_with_id`].
    pub fn group_id(&mut self) -> GroupId {
        self.groups += 1;
        GroupId(self.groups)
    }

    /// A group whose decision is recorded under `id`.
    pub fn group_with_id(&mut self, doc: Doc, id: GroupId) -> Doc {
        let breaks = self.breaks(doc);
        self.push(Kind::Group(doc, id.0), breaks)
    }

    /// One more indentation level for newlines inside.
    pub fn indent(&mut self, doc: Doc) -> Doc {
        if doc.is_nil() {
            return doc;
        }
        let breaks = self.breaks(doc);
        self.push(Kind::Indent(doc), breaks)
    }

    /// `broken` or `flat` by the mode of group `id`, or of the enclosing
    /// group when `None`. A group not printed yet counts as flat.
    pub fn if_break(&mut self, broken: Doc, flat: Doc, id: Option<GroupId>) -> Doc {
        let breaks = self.breaks(flat);
        self.push(Kind::IfBreak(broken, flat, id.map_or(0, |id| id.0)), breaks)
    }

    /// Deferred until the next newline, or the next text that is not sticky.
    pub fn line_suffix(&mut self, doc: Doc) -> Doc {
        if doc.is_nil() {
            return doc;
        }
        self.push(Kind::LineSuffix(doc), false)
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
    text.find(['\n', '\r']).map_or(text, |end| &text[..end])
}

/// Splits on `\r\n`, `\r` and `\n`.
pub(crate) fn lines(text: &str) -> impl Iterator<Item = &str> {
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

/// Display width with a tab counting one column, as in the reference.
pub(crate) fn display_width(text: &str) -> u32 {
    if text.is_ascii() {
        return u32::try_from(
            text.bytes()
                .filter(|byte| matches!(byte, b'\t' | b' '..=b'~'))
                .count(),
        )
        .expect("width fits in u32");
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

/// The output lines of a block comment, each given as parts to concatenate:
/// a multi-line doc comment (`/**`), or any comment with `expand_doc`, is a
/// star block whose opener text moves to its own line; a multi-line block
/// comment whose continuation starts with `*` is a star block that keeps its
/// opener text; any other comment has each line trimmed.
pub(crate) fn comment_lines<'t>(text: &'t str, expand_doc: bool, mut emit: impl FnMut(&[&'t str])) {
    let doc = text.starts_with("/**") && !text.starts_with("/**/");
    let content = strip_delimiters(text);
    let multiline = has_line_break(content);
    if !(doc && (multiline || expand_doc)) && !is_star_block(content) {
        for line in lines(text.trim()) {
            emit(&[line.trim()]);
        }
        return;
    }
    let mut lines = lines(content);
    let mut first = [if doc { "/**" } else { "/*" }, "", ""];
    let mut has_content = false;
    if !doc && multiline {
        let opener = lines.next().unwrap_or("").trim();
        if !opener.is_empty() {
            has_content = true;
            first[1] = " ";
            first[2] = opener;
        }
    }
    emit(&first);
    let mut blank_lines = 0;
    for line in lines {
        let line = if multiline {
            line.trim_start()
                .strip_prefix('*')
                .map_or_else(|| line.trim(), str::trim)
        } else {
            line.trim()
        };
        if line.is_empty() {
            blank_lines += usize::from(has_content);
            continue;
        }
        has_content = true;
        for _ in 0..blank_lines {
            emit(&[" *"]);
        }
        blank_lines = 0;
        emit(&[" * ", line]);
    }
    emit(&[" */"]);
}

fn strip_delimiters(text: &str) -> &str {
    let text = text.trim();
    let body = text.strip_suffix("*/").unwrap_or(text);
    body.strip_prefix("/**")
        .or_else(|| body.strip_prefix("/*"))
        .unwrap_or(body)
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
