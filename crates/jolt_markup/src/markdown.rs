//! The body parser of Markdown doc comments (`///`, JEP 467).
//!
//! Only paragraphs are filled: the description's and those of block tags.
//! Every other block (lists, headings, code blocks, block quotes, tables,
//! HTML blocks, link reference definitions and thematic breaks) and every
//! blank line is printed as written, from its `///`. So the recognizer only
//! has to find the paragraphs outside every container. It follows
//! `CommonMark`'s block algorithm with one line of lookahead: a stack of open
//! block quotes and list items, the open leaf block, lazy continuation lines
//! (which belong to the open quote or item), and the start and end
//! conditions of fences, indented code, HTML blocks and tables. A setext
//! underline or a table delimiter row turns the paragraph above it into a
//! heading or a table, which is then kept as written too.
//!
//! In a paragraph, a word is never split inside an atom: a code span, a link
//! (`[…]`, `[…](…)`, `[…][…]`), an autolink or HTML tag (`<…>`), or the pair
//! `{@tag reference`. Emphasis is not parsed: a line break and a space are
//! the same to it. A source line that starts with a word that could start a
//! block keeps its line break, and so does a hard break, which is written as
//! a trailing `\`.

use crate::frame::Line;
use crate::layout::{Join, Layout};
use crate::tags::{self, Arg, InlineTags, is_space};

/// Lays out the content lines of a `///` run: the description and each
/// block tag. A block tag starts at a line that starts with `@` and a letter,
/// outside a fence (any line that starts with three or more `` ` `` or `~`
/// opens one, and one of the same character and at least the same length
/// closes it) and outside an inline tag, as `jolt_test_support`'s doc
/// comment oracle reads it. Inline tags are found outside fences.
pub(crate) fn parse<'a>(lines: &[Line<'a>], layout: &mut Layout<'_, 'a>) {
    let mut tags = InlineTags::new(lines);
    let mut parser = Parser {
        layout,
        blank: false,
        last_blank: false,
        tagged: false,
        snippets: Vec::new(),
    };
    let mut fence: Option<(u8, usize)> = None;
    let mut tag_end = 0;
    let mut start = 0;
    let mut tag = false;
    for (index, line) in lines.iter().enumerate() {
        if line.offset >= tag_end {
            let trimmed = line.text.trim_start();
            match trimmed.as_bytes().first() {
                Some(&marker @ (b'`' | b'~')) => {
                    let len = trimmed.bytes().take_while(|&byte| byte == marker).count();
                    match fence {
                        None if len >= 3 => fence = Some((marker, len)),
                        Some((open, open_len)) if open == marker && len >= open_len => {
                            fence = None;
                        }
                        _ => {}
                    }
                    continue;
                }
                _ if fence.is_some() => continue,
                _ if tags::is_block_tag(trimmed) => {
                    if index > start {
                        parser.part(&lines[start..index], tag);
                    }
                    start = index;
                    tag = true;
                }
                _ => {}
            }
        }
        let text = line.text;
        let mut from = tag_end.saturating_sub(line.offset);
        while let Some(found) = text.get(from..).and_then(|rest| rest.find("{@")) {
            let at = from + found;
            from = at + 2;
            if let Some(end) = tags.end(line.offset + at) {
                tag_end = end;
                if tags::inline_tag_name(&text[at..]) == "snippet" {
                    parser.snippets.push((line.offset + at, end));
                }
                from = end - line.offset;
            }
        }
    }
    parser.part(&lines[start..], tag);
}

struct Parser<'p, 'l, 'a> {
    layout: &'p mut Layout<'l, 'a>,
    /// A blank line goes before the next line (the first block tag's).
    blank: bool,
    /// The last line printed is blank.
    last_blank: bool,
    /// A block tag has started.
    tagged: bool,
    /// The `{@snippet}` tags, `(start, end)` offsets sorted by start.
    snippets: Vec<(usize, usize)>,
}

impl<'a> Parser<'_, '_, 'a> {
    /// Lays out the description, or a block tag (`tag`) and its text.
    fn part(&mut self, lines: &[Line<'a>], tag: bool) {
        if tag && !std::mem::replace(&mut self.tagged, true) {
            self.blank = !self.layout.at_start() && !self.last_blank;
        }
        let mut blocks = Blocks::default();
        let mut paragraph: Option<usize> = None;
        for (index, line) in lines.iter().enumerate() {
            let class = blocks.line(line.text);
            if class == Class::Paragraph {
                paragraph.get_or_insert(index);
                continue;
            }
            if let Some(start) = paragraph.take() {
                let lines = &lines[start..index];
                self.paragraph(lines, tag && start == 0, class == Class::Converts);
            }
            self.verbatim(line);
        }
        if let Some(start) = paragraph {
            self.paragraph(&lines[start..], tag && start == 0, false);
        }
    }

    fn verbatim(&mut self, line: &Line<'a>) {
        if std::mem::take(&mut self.blank) {
            self.layout.line(true, 0, 0);
        }
        self.layout.verbatim(line.raw);
        self.last_blank = line.text.is_empty();
    }

    /// A paragraph outside every container: filled, unless the line after it
    /// turns it into a heading or a table (`converted`), it may start with
    /// link reference definitions (which a fill could join to the text after
    /// them), or a `{@snippet}` (whose code keeps its lines) covers a part of
    /// it.
    fn paragraph(&mut self, lines: &[Line<'a>], tag: bool, converted: bool) {
        let definitions = lines[0].text.trim_start().starts_with('[')
            && lines.iter().any(|line| line.text.contains("]:"));
        let last = lines[lines.len() - 1];
        let (start, end) = (lines[0].offset, last.offset + last.text.len());
        let next = self.snippets.partition_point(|snippet| snippet.1 <= start);
        let snippet = self
            .snippets
            .get(next)
            .is_some_and(|snippet| snippet.0 < end);
        if converted || definitions || snippet {
            for line in lines {
                self.verbatim(line);
            }
            return;
        }
        self.last_blank = false;
        let blank = std::mem::take(&mut self.blank);
        self.layout.line(blank, 0, 0);
        Fill::new(lines, tag).run(self.layout);
    }
}

/// What a line is to the parser.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Class {
    /// A line of a paragraph outside every container.
    Paragraph,
    /// A setext underline or a table delimiter row: the paragraph above it is
    /// a heading or a table.
    Converts,
    /// Any other line.
    Other,
}

/// An open container block.
#[derive(Clone, Copy, Debug)]
enum Container {
    Quote,
    /// A list item whose content starts `width` columns after the column
    /// its marker's line starts at, inside the containers around it.
    Item {
        width: usize,
    },
}

/// The open leaf block.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Leaf {
    #[default]
    None,
    Paragraph,
    Fence {
        marker: u8,
        len: usize,
    },
    /// Indented code.
    Code,
    Html(HtmlEnd),
    Table,
}

/// The end condition of an HTML block.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HtmlEnd {
    /// Type 1: a line with `</pre>`, `</script>`, `</style>` or
    /// `</textarea>`, in any case.
    Raw,
    /// Types 2 to 5: a line with this text.
    Text(&'static str),
    /// Types 6 and 7: a blank line.
    Blank,
}

/// `CommonMark`'s block structure, a line at a time.
#[derive(Debug, Default)]
struct Blocks {
    containers: Vec<Container>,
    leaf: Leaf,
}

impl Blocks {
    /// Reads the next line (its text after the frame).
    fn line(&mut self, text: &str) -> Class {
        let mut cursor = Cursor::new(text);
        let mut matched = 0;
        for &container in &self.containers {
            let (indent, at) = cursor.indent();
            let blank = at == text.len();
            match container {
                Container::Quote if indent <= 3 && text.as_bytes().get(at) == Some(&b'>') => {
                    cursor.advance(indent);
                    cursor.quote_marker();
                }
                Container::Item { .. } if blank => {}
                Container::Item { width } if indent >= width => cursor.advance(width),
                _ => break,
            }
            matched += 1;
        }
        let (indent, at) = cursor.indent();
        let rest = &text.as_bytes()[at..];
        let blank = rest.is_empty();
        if matched == self.containers.len() {
            match self.leaf {
                Leaf::Fence { marker, len } => {
                    if indent <= 3 && closes_fence(rest, marker, len) {
                        self.leaf = Leaf::None;
                    }
                    return Class::Other;
                }
                Leaf::Code if blank || indent >= 4 => return Class::Other,
                Leaf::Html(end) => {
                    if html_ends(end, &text[cursor.pos..]) {
                        self.leaf = Leaf::None;
                    }
                    return Class::Other;
                }
                Leaf::Table if !blank && !opens(rest, indent, false) => return Class::Other,
                Leaf::Paragraph if !blank => {
                    if indent <= 3 && is_setext_underline(rest) {
                        self.leaf = Leaf::None;
                        return Class::Converts;
                    }
                    if indent <= 3 && is_delimiter_row(rest) {
                        self.leaf = Leaf::Table;
                        return Class::Converts;
                    }
                    if !opens(rest, indent, true) {
                        return self.paragraph_line();
                    }
                }
                _ => {}
            }
        } else if self.leaf == Leaf::Paragraph && !blank && !opens(rest, indent, false) {
            // A lazy continuation line.
            return Class::Other;
        }
        self.containers.truncate(matched);
        self.leaf = Leaf::None;
        if blank {
            return Class::Other;
        }
        self.open(cursor, text)
    }

    /// Opens the blocks that start at `cursor`, after the containers that
    /// continue: new containers, then a leaf block.
    fn open(&mut self, mut cursor: Cursor<'_>, text: &str) -> Class {
        loop {
            let (indent, at) = cursor.indent();
            let rest = &text.as_bytes()[at..];
            if indent >= 4 || is_thematic_break(rest) {
                break;
            }
            if rest.first() == Some(&b'>') {
                cursor.advance(indent);
                cursor.quote_marker();
                self.containers.push(Container::Quote);
            } else if let Some(marker) = list_marker(rest) {
                let start = cursor.col;
                cursor.advance(indent);
                cursor.skip(marker);
                // The content starts after the marker and its spaces; after
                // one space when it is blank or starts with indented code.
                let (spaces, at) = cursor.indent();
                let width = if at == text.len() {
                    cursor.col - start + 1
                } else {
                    cursor.advance(if spaces >= 5 { 1 } else { spaces });
                    cursor.col - start
                };
                self.containers.push(Container::Item { width });
            } else {
                break;
            }
        }
        let (indent, at) = cursor.indent();
        let rest = &text.as_bytes()[at..];
        if rest.is_empty() || is_thematic_break(rest) || is_heading(rest) {
            return Class::Other;
        }
        if indent >= 4 {
            self.leaf = Leaf::Code;
        } else if let Some((marker, len)) = opens_fence(rest) {
            self.leaf = Leaf::Fence { marker, len };
        } else if let Some(end) = html_start(rest, true) {
            if !html_ends(end, &text[at..]) {
                self.leaf = Leaf::Html(end);
            }
        } else {
            self.leaf = Leaf::Paragraph;
            return self.paragraph_line();
        }
        Class::Other
    }

    fn paragraph_line(&self) -> Class {
        if self.containers.is_empty() {
            Class::Paragraph
        } else {
            Class::Other
        }
    }
}

/// A position in a line, in bytes and in columns (a tab advances to the
/// next multiple of 4).
struct Cursor<'s> {
    bytes: &'s [u8],
    pos: usize,
    col: usize,
    /// Columns of the tab before `pos` that are not used yet: a container
    /// marker may take one column of a tab and leave the rest as indentation.
    virt: usize,
}

impl<'s> Cursor<'s> {
    const fn new(text: &'s str) -> Self {
        Self {
            bytes: text.as_bytes(),
            pos: 0,
            col: 0,
            virt: 0,
        }
    }

    /// The columns of whitespace from here, and the byte after it.
    fn indent(&self) -> (usize, usize) {
        let mut col = self.col + self.virt;
        let mut at = self.pos;
        while let Some(&byte) = self.bytes.get(at) {
            match byte {
                b' ' => col += 1,
                b'\t' => col += 4 - col % 4,
                _ => break,
            }
            at += 1;
        }
        (col - self.col, at)
    }

    /// Takes `columns` columns of whitespace.
    fn advance(&mut self, mut columns: usize) {
        let take = columns.min(self.virt);
        self.virt -= take;
        self.col += take;
        columns -= take;
        while columns > 0 {
            match self.bytes.get(self.pos) {
                Some(b' ') => {
                    self.col += 1;
                    columns -= 1;
                }
                Some(b'\t') => {
                    let width = 4 - self.col % 4;
                    let take = width.min(columns);
                    self.col += take;
                    self.virt = width - take;
                    columns -= take;
                }
                _ => break,
            }
            self.pos += 1;
        }
    }

    /// Takes `len` bytes of non-whitespace.
    const fn skip(&mut self, len: usize) {
        self.pos += len;
        self.col += len;
    }

    /// Takes a block quote's `>` and the one space after it.
    fn quote_marker(&mut self) {
        self.skip(1);
        if matches!(self.bytes.get(self.pos), Some(b' ' | b'\t')) {
            self.advance(1);
        }
    }
}

/// Whether `rest` (after `indent` columns) starts a block other than a
/// paragraph, where it could end the open paragraph: when `interrupting`, a
/// list item needs text and an ordered one must start at 1, and an HTML
/// block of type 7 can't start.
fn opens(rest: &[u8], indent: usize, interrupting: bool) -> bool {
    if indent >= 4 || rest.is_empty() {
        return false;
    }
    rest[0] == b'>'
        || is_thematic_break(rest)
        || is_heading(rest)
        || opens_fence(rest).is_some()
        || html_start(rest, !interrupting).is_some()
        || list_marker(rest).is_some_and(|len| {
            let number = &rest[..len - 1];
            !interrupting
                || (rest[len..].iter().any(|&byte| !is_space(byte))
                    && (number.is_empty()
                        || number.iter().skip_while(|&&digit| digit == b'0').eq(b"1")))
        })
}

/// The length of the list marker that starts `rest`: `-`, `+` or `*`, or
/// one to nine digits and `.` or `)`, before whitespace or the line's end.
fn list_marker(rest: &[u8]) -> Option<usize> {
    let digits = rest.iter().take_while(|byte| byte.is_ascii_digit()).count();
    let len = match rest.get(digits) {
        Some(b'-' | b'+' | b'*') if digits == 0 => 1,
        Some(b'.' | b')') if (1..=9).contains(&digits) => digits + 1,
        _ => return None,
    };
    rest.get(len)
        .is_none_or(|&byte| matches!(byte, b' ' | b'\t'))
        .then_some(len)
}

/// A thematic break: three or more `*`, `-` or `_`, and spaces or tabs.
fn is_thematic_break(rest: &[u8]) -> bool {
    let Some(&mark @ (b'*' | b'-' | b'_')) = rest.first() else {
        return false;
    };
    let mut count = 0;
    for &byte in rest {
        if byte == mark {
            count += 1;
        } else if !matches!(byte, b' ' | b'\t') {
            return false;
        }
    }
    count >= 3
}

/// An ATX heading: one to six `#` before whitespace or the line's end.
fn is_heading(rest: &[u8]) -> bool {
    let hashes = rest.iter().take_while(|&&byte| byte == b'#').count();
    (1..=6).contains(&hashes)
        && rest
            .get(hashes)
            .is_none_or(|&byte| matches!(byte, b' ' | b'\t'))
}

/// A setext heading underline: `=`s or `-`s, then spaces or tabs.
fn is_setext_underline(rest: &[u8]) -> bool {
    let Some(&mark @ (b'=' | b'-')) = rest.first() else {
        return false;
    };
    let len = rest.iter().take_while(|&&byte| byte == mark).count();
    rest[len..].iter().all(|&byte| matches!(byte, b' ' | b'\t'))
}

/// Whether `rest` may be a table's delimiter row (`|---|:--:|`): only `|`,
/// `:`, `-` and whitespace, with a `-` and a `|` or `:`.
fn is_delimiter_row(rest: &[u8]) -> bool {
    rest.iter()
        .all(|&byte| matches!(byte, b'|' | b':' | b'-' | b' ' | b'\t'))
        && rest.contains(&b'-')
        && (rest.contains(&b'|') || rest.contains(&b':'))
}

/// The fence that `rest` opens: three or more `` ` `` (with no `` ` `` in
/// the info string after them) or `~`.
fn opens_fence(rest: &[u8]) -> Option<(u8, usize)> {
    let &marker @ (b'`' | b'~') = rest.first()? else {
        return None;
    };
    let len = rest.iter().take_while(|&&byte| byte == marker).count();
    (len >= 3 && !(marker == b'`' && rest[len..].contains(&b'`'))).then_some((marker, len))
}

/// Whether `rest` closes a fence of `len` `marker`s.
fn closes_fence(rest: &[u8], marker: u8, len: usize) -> bool {
    let run = rest.iter().take_while(|&&byte| byte == marker).count();
    run >= len && rest[run..].iter().all(|&byte| matches!(byte, b' ' | b'\t'))
}

/// The block-level HTML tag names of HTML block type 6, separated by spaces.
const BLOCK_NAMES: &str = "address article aside base basefont blockquote body caption center col \
     colgroup dd details dialog dir div dl dt fieldset figcaption figure \
     footer form frame frameset h1 h2 h3 h4 h5 h6 head header hr html \
     iframe legend li link main menu menuitem nav noframes ol optgroup \
     option p param search section summary table tbody td tfoot th thead \
     title tr track ul";

/// The tag names of HTML block type 1, whose content keeps blank lines.
const RAW_NAMES: &str = "pre script style textarea";

/// The HTML block that `rest` starts, by its end condition. Type 7 (any
/// other tag alone on its line) only when `any_tag`, and read loosely: a
/// line that starts with a tag and ends with `>`, which keeps more lines as
/// written than `CommonMark` would.
fn html_start(rest: &[u8], any_tag: bool) -> Option<HtmlEnd> {
    let after = rest.strip_prefix(b"<")?;
    if after.starts_with(b"!--") {
        return Some(HtmlEnd::Text("-->"));
    }
    if after.starts_with(b"?") {
        return Some(HtmlEnd::Text("?>"));
    }
    if after.starts_with(b"![CDATA[") {
        return Some(HtmlEnd::Text("]]>"));
    }
    if after.first() == Some(&b'!') && after.get(1).is_some_and(u8::is_ascii_alphabetic) {
        return Some(HtmlEnd::Text(">"));
    }
    let closing = after.first() == Some(&b'/');
    let name = &after[usize::from(closing)..];
    if !name.first().is_some_and(u8::is_ascii_alphabetic) {
        return None;
    }
    let len = name
        .iter()
        .take_while(|&&byte| byte.is_ascii_alphanumeric() || byte == b'-')
        .count();
    let (name, next) = (&name[..len], &name[len..]);
    let is = |names: &str| {
        names
            .split(' ')
            .any(|candidate| candidate.as_bytes().eq_ignore_ascii_case(name))
    };
    let ends_name = |slash: bool| match next.first() {
        None | Some(b' ' | b'\t' | b'>') => true,
        Some(b'/') => slash && next.get(1) == Some(&b'>'),
        _ => false,
    };
    if !closing && is(RAW_NAMES) && ends_name(false) {
        return Some(HtmlEnd::Raw);
    }
    if is(BLOCK_NAMES) && ends_name(true) {
        return Some(HtmlEnd::Blank);
    }
    (any_tag && rest.trim_ascii_end().ends_with(b">")).then_some(HtmlEnd::Blank)
}

/// Whether `text`, a line inside an HTML block, ends it.
fn html_ends(end: HtmlEnd, text: &str) -> bool {
    match end {
        HtmlEnd::Raw => ["</pre>", "</script>", "</style>", "</textarea>"]
            .iter()
            .any(|closer| find_ignoring_case(text, closer)),
        HtmlEnd::Text(closer) => text.contains(closer),
        HtmlEnd::Blank => text.trim().is_empty(),
    }
}

/// Whether `text` contains `needle` (lowercase ASCII), ignoring ASCII case.
fn find_ignoring_case(text: &str, needle: &str) -> bool {
    let needle = needle.as_bytes();
    text.as_bytes()
        .windows(needle.len())
        .any(|window| window.eq_ignore_ascii_case(needle))
}

/// Fills a paragraph's words into the layout.
struct Fill<'p, 'a> {
    lines: &'p [Line<'a>],
    /// The atoms, `(start, end)` offsets sorted by start and disjoint.
    atoms: Vec<(usize, usize)>,
    /// The first atom that may start at or after the scan position.
    atom: usize,
    tags: InlineTags,
    /// The end of the inline tag the scan is in, or 0.
    tag_end: usize,
    inline_arg: Arg,
    block_arg: Arg,
}

impl<'p, 'a> Fill<'p, 'a> {
    /// `tag`: the paragraph starts a block tag, whose name and first
    /// argument stay together.
    fn new(lines: &'p [Line<'a>], tag: bool) -> Self {
        Self {
            lines,
            atoms: atoms(lines),
            atom: 0,
            tags: InlineTags::new(lines),
            tag_end: 0,
            inline_arg: Arg::None,
            block_arg: if tag { Arg::Name } else { Arg::None },
        }
    }

    fn run(mut self, layout: &mut Layout<'_, 'a>) {
        // The end of an atom that continues on the next line.
        let mut carry: Option<usize> = None;
        // The next line starts an output line: after a hard break, or
        // inside an atom.
        let mut break_before = false;
        for (index, line) in self.lines.iter().enumerate() {
            let text = line.text;
            let bytes = text.as_bytes();
            // A `<` that may start an HTML tag and is in no atom.
            let mut open_tag = false;
            // The whitespace that starts a line is the paragraph's
            // indentation, except inside an atom, whose text it is.
            let mut pos = if carry.is_some() {
                0
            } else {
                bytes.iter().take_while(|&&byte| is_space(byte)).count()
            };
            let indent = columns(&bytes[..pos]);
            let mut first = true;
            while pos < bytes.len() {
                let start = pos;
                let atom = carry.take();
                // Before the scan, which may start an inline tag.
                let in_tag = line.offset + start < self.tag_end;
                let (end, next) = match atom {
                    Some(end) if end > line.offset + bytes.len() => (bytes.len(), Some(end)),
                    Some(end) => self.word_end(line, start, end - line.offset, &mut open_tag),
                    None => self.word_end(line, start, start, &mut open_tag),
                };
                let word = &text[start..end];
                let glue = match atom {
                    Some(_) => true,
                    None if in_tag => self.inline_arg.glue(word),
                    None => self.block_arg.glue(word),
                };
                if first && index > 0 {
                    // A line that starts with a word that could start a block
                    // keeps its line break, unless it is indented as code: at
                    // the frame it would start the block.
                    let structure = tags::starts_structure(word, true);
                    let deep = indent >= 4;
                    if atom.is_some() || break_before {
                        let indent = if structure && deep { indent } else { 0 };
                        layout.line(false, u16::try_from(indent).unwrap_or(u16::MAX), 0);
                    } else if structure && !deep {
                        layout.line(false, 0, 0);
                    }
                }
                layout.word(word, if glue { Join::Space } else { Join::Wrap });
                self.after_word(line.offset + end);
                first = false;
                carry = next;
                pos = end;
                if carry.is_some() {
                    break;
                }
                while pos < bytes.len() && is_space(bytes[pos]) {
                    pos += 1;
                }
            }
            break_before = carry.is_some();
            if carry.is_some() || index + 1 == self.lines.len() {
                continue;
            }
            // A line that ends in two spaces ends in a hard break (outside
            // tags), and a `\` before them is literal; else an odd run of
            // `\` is a hard break.
            if line.hard_break {
                if !open_tag && line.offset + bytes.len() >= self.tag_end {
                    layout.hard_break();
                    break_before = true;
                }
            } else if bytes
                .iter()
                .rev()
                .take_while(|&&byte| byte == b'\\')
                .count()
                % 2
                == 1
            {
                break_before = true;
            }
        }
    }

    /// The end of the word at `pos` on `line`: at whitespace outside atoms,
    /// scanning from `from`. When an atom runs past the line's end, the word
    /// ends there, with the atom's end.
    fn word_end(
        &mut self,
        line: &Line<'a>,
        pos: usize,
        from: usize,
        open_tag: &mut bool,
    ) -> (usize, Option<usize>) {
        let bytes = line.text.as_bytes();
        let mut at = from.max(pos);
        while at < bytes.len() {
            let offset = line.offset + at;
            while self
                .atoms
                .get(self.atom)
                .is_some_and(|atom| atom.0 < offset)
            {
                self.atom += 1;
            }
            if let Some(&(start, end)) = self.atoms.get(self.atom)
                && start == offset
            {
                if end > line.offset + bytes.len() {
                    return (bytes.len(), Some(end));
                }
                at = end - line.offset;
                continue;
            }
            match bytes[at] {
                byte if is_space(byte) => break,
                b'\\' => {
                    at += 1 + usize::from(bytes.get(at + 1).is_some_and(u8::is_ascii_punctuation));
                }
                b'{' if bytes.get(at + 1) == Some(&b'@') && offset >= self.tag_end => {
                    if let Some(end) = self.tags.end(offset) {
                        self.tag_end = end;
                        self.inline_arg = Arg::inline(tags::inline_tag_name(&line.text[at..]));
                    }
                    at += 2;
                }
                b'<' => {
                    *open_tag |= bytes.get(at + 1).is_some_and(|&next| {
                        next.is_ascii_alphabetic() || matches!(next, b'/' | b'!' | b'?')
                    });
                    at += 1;
                }
                _ => at += 1,
            }
        }
        (at, None)
    }

    /// Leaves an inline tag that ended before `offset`.
    fn after_word(&mut self, offset: usize) {
        if self.tag_end != 0 && offset >= self.tag_end {
            self.tag_end = 0;
            self.inline_arg = Arg::None;
        }
    }
}

/// The atoms of a paragraph, in one pass each: code spans (a run of
/// backticks to the next run of the same length), then links and the
/// brackets and parentheses they are made of (outside code spans, and not
/// escaped with `\`), and raw HTML (`<…>`, `<!--…-->`, `<?…?>`,
/// `<![CDATA[…]]>`), which also covers autolinks. Atoms that overlap are
/// merged.
fn atoms(lines: &[Line<'_>]) -> Vec<(usize, usize)> {
    let spans = code_spans(lines);
    let mut atoms = Vec::new();
    let (mut brackets, mut parens) = (Vec::new(), Vec::new());
    let (mut open_brackets, mut open_parens) = (Vec::new(), Vec::new());
    let mut span = 0;
    let mut exhausted = [false; 6];
    let (mut index, mut at) = (0, 0);
    while let Some(line) = lines.get(index) {
        let bytes = line.text.as_bytes();
        if at >= bytes.len() {
            (index, at) = (index + 1, 0);
            continue;
        }
        let offset = line.offset + at;
        while spans
            .get(span)
            .is_some_and(|span: &(usize, usize)| span.1 <= offset)
        {
            span += 1;
        }
        if let Some(&(start, end)) = spans.get(span)
            && start <= offset
        {
            (index, at) = position(lines, index, end);
            continue;
        }
        match bytes[at] {
            b'\\' => at += 1,
            b'[' => open_brackets.push(offset),
            b']' => {
                if let Some(open) = open_brackets.pop() {
                    brackets.push((open, offset + 1));
                }
            }
            b'(' => open_parens.push(offset),
            b')' => {
                if let Some(open) = open_parens.pop() {
                    parens.push((open, offset + 1));
                }
            }
            b'<' => {
                let mut html = Html {
                    lines,
                    index,
                    at: at + 1,
                    exhausted: &mut exhausted,
                };
                if html.raw_html() {
                    atoms.push((offset, lines[html.index].offset + html.at));
                }
                // On failure, go on where the scan failed (never before
                // `at + 1`), so each byte is scanned once.
                (index, at) = (html.index, html.at);
                continue;
            }
            _ => {}
        }
        at += 1;
    }
    brackets.sort_unstable();
    parens.sort_unstable();
    let closer = |pairs: &[(usize, usize)], open: usize| {
        pairs
            .binary_search_by_key(&open, |pair| pair.0)
            .ok()
            .map(|index| pairs[index].1)
    };
    for &(open, end) in &brackets {
        let end = closer(&parens, end)
            .or_else(|| closer(&brackets, end))
            .unwrap_or(end);
        atoms.push((open, end));
    }
    atoms.extend(spans);
    atoms.sort_unstable();
    let mut merged: Vec<(usize, usize)> = Vec::with_capacity(atoms.len());
    for (start, end) in atoms {
        match merged.last_mut() {
            Some(last) if start < last.1 => last.1 = last.1.max(end),
            _ => merged.push((start, end)),
        }
    }
    merged
}

/// A scan of raw HTML (`CommonMark`'s inline HTML, and autolinks) over a
/// paragraph's lines; a line break is whitespace.
struct Html<'s, 'l> {
    lines: &'s [Line<'l>],
    index: usize,
    at: usize,
    /// For each text a scan looks for (`-->`, `?>`, `]]>`, `>`, `"`, `'`),
    /// whether none is left after the scan position.
    exhausted: &'s mut [bool; 6],
}

impl Html<'_, '_> {
    fn peek(&self) -> Option<u8> {
        let line = self.lines.get(self.index)?;
        Some(line.text.as_bytes().get(self.at).copied().unwrap_or(b'\n'))
    }

    fn bump(&mut self) {
        if self.at >= self.lines[self.index].text.len() {
            (self.index, self.at) = (self.index + 1, 0);
        } else {
            self.at += 1;
        }
    }

    fn eat(&mut self, byte: u8) -> bool {
        let eaten = self.peek() == Some(byte);
        if eaten {
            self.bump();
        }
        eaten
    }

    /// Takes bytes while `keep` holds; their count.
    fn take(&mut self, keep: impl Fn(u8) -> bool) -> usize {
        let mut count = 0;
        while self.peek().is_some_and(&keep) {
            self.bump();
            count += 1;
        }
        count
    }

    fn whitespace(&mut self) -> usize {
        self.take(|byte| matches!(byte, b' ' | b'\t' | b'\n'))
    }

    /// Moves past the next `needle` (`kind` in `exhausted`); false when there
    /// is none, leaving the scan where it was.
    fn past(&mut self, kind: usize, needle: &str) -> bool {
        if self.exhausted[kind] {
            return false;
        }
        if let Some((index, at)) = find_from(self.lines, self.index, self.at, needle) {
            (self.index, self.at) = (index, at + needle.len());
            true
        } else {
            self.exhausted[kind] = true;
            false
        }
    }

    /// Whether raw HTML starts after the `<` the scan is past; the scan ends
    /// after it, or where it failed.
    fn raw_html(&mut self) -> bool {
        let rest = self.lines[self.index]
            .text
            .as_bytes()
            .get(self.at..)
            .unwrap_or_default();
        if rest.starts_with(b"!--") {
            return self.past(0, "-->");
        }
        if rest.starts_with(b"?") {
            return self.past(1, "?>");
        }
        if rest.starts_with(b"![CDATA[") {
            return self.past(2, "]]>");
        }
        if rest.first() == Some(&b'!') && rest.get(1).is_some_and(u8::is_ascii_alphabetic) {
            return self.past(3, ">");
        }
        let closing = self.eat(b'/');
        if !self.peek().is_some_and(|byte| byte.is_ascii_alphabetic()) {
            return false;
        }
        // An autolink (`<https://…>`) is not a tag, but has no whitespace.
        let name = |byte: u8| byte.is_ascii_alphanumeric() || byte == b'-';
        self.take(name);
        if self.peek() == Some(b':') || self.peek() == Some(b'@') {
            self.take(|byte| !matches!(byte, b'>' | b'<' | b' ' | b'\t' | b'\n'));
            return self.eat(b'>');
        }
        if closing {
            self.whitespace();
            return self.eat(b'>');
        }
        loop {
            let spaced = self.whitespace() > 0;
            if self.eat(b'>') {
                return true;
            }
            if self.eat(b'/') {
                return self.eat(b'>');
            }
            let attribute = |byte: u8| byte.is_ascii_alphabetic() || matches!(byte, b'_' | b':');
            if !spaced || !self.peek().is_some_and(attribute) {
                return false;
            }
            self.take(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b':' | b'-')
            });
            let before = (self.index, self.at);
            self.whitespace();
            if !self.eat(b'=') {
                (self.index, self.at) = before;
                continue;
            }
            self.whitespace();
            let valued = match self.peek() {
                Some(quote @ (b'"' | b'\'')) => {
                    self.bump();
                    let kind = if quote == b'"' { 4 } else { 5 };
                    self.past(kind, if quote == b'"' { "\"" } else { "'" })
                }
                _ => {
                    self.take(|byte| {
                        !matches!(
                            byte,
                            b'"' | b'\'' | b'=' | b'<' | b'>' | b'`' | b' ' | b'\t' | b'\n'
                        )
                    }) > 0
                }
            };
            if !valued {
                return false;
            }
        }
    }
}

/// The line and byte of the first `needle` at or after byte `at` of line
/// `index`.
fn find_from(lines: &[Line<'_>], index: usize, at: usize, needle: &str) -> Option<(usize, usize)> {
    let first = lines
        .get(index)?
        .text
        .get(at..)?
        .find(needle)
        .map(|found| (index, at + found));
    first.or_else(|| {
        lines[index + 1..]
            .iter()
            .enumerate()
            .find_map(|(later, line)| {
                line.text
                    .find(needle)
                    .map(|found| (index + 1 + later, found))
            })
    })
}

/// The line and byte of `offset`, which is on line `index` or a later one.
fn position(lines: &[Line<'_>], mut index: usize, offset: usize) -> (usize, usize) {
    while index + 1 < lines.len() && lines[index + 1].offset <= offset {
        index += 1;
    }
    (index, offset - lines[index].offset)
}

/// The columns of `indentation` (spaces and tabs, a tab to the next
/// multiple of 4).
fn columns(indentation: &[u8]) -> usize {
    indentation.iter().fold(0, |col, &byte| match byte {
        b'\t' => col + 4 - col % 4,
        _ => col + 1,
    })
}

/// The code spans of a paragraph, `(start, end)` offsets sorted by start: a
/// run of backticks that isn't escaped opens one, which the next run of the
/// same length closes (a run after `\` opens one backtick later). Linear: a
/// table of the next run of each length.
fn code_spans(lines: &[Line<'_>]) -> Vec<(usize, usize)> {
    // (start, len, escaped)
    let mut runs: Vec<(usize, usize, bool)> = Vec::new();
    for line in lines {
        let bytes = line.text.as_bytes();
        if !bytes.contains(&b'`') {
            continue;
        }
        let mut at = 0;
        while at < bytes.len() {
            if bytes[at] != b'`' {
                at += 1;
                continue;
            }
            let len = bytes[at..].iter().take_while(|&&byte| byte == b'`').count();
            let backslashes = bytes[..at]
                .iter()
                .rev()
                .take_while(|&&byte| byte == b'\\')
                .count();
            runs.push((line.offset + at, len, backslashes % 2 == 1));
            at += len;
        }
    }
    if runs.len() < 2 {
        return Vec::new();
    }
    // The next run of the same length, and of one less (for an escaped run).
    let longest = runs.iter().map(|run| run.1).max().unwrap_or(0);
    let mut later = vec![usize::MAX; longest + 1];
    let mut next = vec![(usize::MAX, usize::MAX); runs.len()];
    for (index, &(_, len, _)) in runs.iter().enumerate().rev() {
        next[index] = (later[len], later[len - 1]);
        later[len] = index;
    }
    let mut spans = Vec::new();
    let mut index = 0;
    while index < runs.len() {
        let (start, _, escaped) = runs[index];
        let (start, closer) = if escaped {
            (start + 1, next[index].1)
        } else {
            (start, next[index].0)
        };
        match runs.get(closer) {
            Some(&(close, close_len, _)) => {
                spans.push((start, close + close_len));
                index = closer + 1;
            }
            _ => index += 1,
        }
    }
    spans
}
