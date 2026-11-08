//! Javadoc's tag syntax, shared by both flavours: inline tags (`{@…}`),
//! block tags (`@param` at a line start), and the words a line must not start
//! with.

use crate::frame::Line;

/// The brace-balanced inline tags of a comment, found before anything else:
/// nothing inside one starts a block tag, a block element or a verbatim
/// region. An inline tag runs from its `{@` to the `}` that balances it,
/// counting every brace; a `{@` without one is an ordinary word.
pub(crate) struct InlineTags {
    /// `(start, end)` byte offsets in the comment, sorted by start.
    tags: Vec<(usize, usize)>,
    /// The first tag that may start at or after the offset asked last.
    cursor: usize,
}

impl InlineTags {
    /// Matches every brace of `lines` in one pass with a stack.
    pub(crate) fn new(lines: &[Line<'_>]) -> Self {
        let mut tags = Vec::new();
        let mut open: Vec<(usize, bool)> = Vec::new();
        for line in lines {
            let bytes = line.text.as_bytes();
            if !bytes.contains(&b'{') && (open.is_empty() || !bytes.contains(&b'}')) {
                continue;
            }
            for (index, &byte) in bytes.iter().enumerate() {
                match byte {
                    b'{' => open.push((line.offset + index, bytes.get(index + 1) == Some(&b'@'))),
                    b'}' => {
                        if let Some((start, true)) = open.pop() {
                            tags.push((start, line.offset + index + 1));
                        }
                    }
                    _ => {}
                }
            }
        }
        tags.sort_unstable();
        Self { tags, cursor: 0 }
    }

    /// The end of the inline tag that starts at `offset`, if one does.
    /// Offsets are asked in increasing order.
    pub(crate) fn end(&mut self, offset: usize) -> Option<usize> {
        while self.tags.get(self.cursor).is_some_and(|tag| tag.0 < offset) {
            self.cursor += 1;
        }
        self.tags
            .get(self.cursor)
            .filter(|tag| tag.0 == offset)
            .map(|tag| tag.1)
    }
}

/// The name of an inline tag whose text starts at `text` (`{@name`).
pub(crate) fn inline_tag_name(text: &str) -> &str {
    let name = &text[2..];
    let len = name
        .find(|ch: char| ch.is_ascii_whitespace() || ch == '}')
        .unwrap_or(name.len());
    &name[..len]
}

/// Whether a line whose text (after the frame) is `text` starts a block tag:
/// `@` and a letter after spaces and tabs.
pub(crate) fn is_block_tag(text: &str) -> bool {
    let text = text.trim_start_matches([' ', '\t']);
    text.strip_prefix('@')
        .is_some_and(|rest| rest.starts_with(|ch: char| ch.is_ascii_alphabetic()))
}

/// Whether `word` starts a list item in plain text: `-`, `*`, `+`, `•`, or
/// digits and `.` or `)`.
pub(crate) fn is_list_marker(word: &str) -> bool {
    match word {
        "-" | "*" | "+" | "•" => true,
        _ => word
            .strip_suffix(['.', ')'])
            .is_some_and(|digits| !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())),
    }
}

/// Whether `word` would start new structure at the start of a line: a block
/// tag (`@`), or a plain-text list item. Such a word is never put at the
/// start of a line by a wrap; it is glued to the word before it.
///
/// In Markdown (`markdown`) also every word that may start a block: `#`
/// (a heading), `>` (a block quote), three or more `` ` `` or `~` (a
/// fence), `|` (a table row), `<` and a letter, `/`, `!` or `?` (an HTML
/// block), and a word made only of `*`, `-`, `_`, `=`, `+` and `:` (a
/// thematic break, a setext underline, a table delimiter row).
pub(crate) fn starts_structure(word: &str, markdown: bool) -> bool {
    if word.starts_with('@') || is_list_marker(word) {
        return true;
    }
    if !markdown {
        return false;
    }
    let bytes = word.as_bytes();
    let Some(&first) = bytes.first() else {
        return false;
    };
    match first {
        b'#' | b'>' | b'|' => true,
        fence @ (b'`' | b'~') => bytes.len() >= 3 && bytes[..3].iter().all(|&byte| byte == fence),
        b'<' => bytes
            .get(1)
            .is_some_and(|&next| next.is_ascii_alphabetic() || matches!(next, b'/' | b'!' | b'?')),
        _ => {
            bytes.iter().all(|byte| b"*-_=+:".contains(byte))
                && bytes.iter().any(|&byte| byte != b':')
        }
    }
}

/// Whether `text` is a ruler: three or more of `-=_*+~#|.:/\`, possibly
/// with spaces between, such as `-------` or `|-----|`.
pub(crate) fn is_ruler(text: &str) -> bool {
    let mut count = 0;
    for byte in text.bytes() {
        match byte {
            b' ' | b'\t' => {}
            b'-' | b'=' | b'_' | b'*' | b'+' | b'~' | b'#' | b'|' | b'.' | b':' | b'/' | b'\\' => {
                count += 1;
            }
            _ => return false,
        }
    }
    count >= 3
}

/// Whether `byte` separates words.
pub(crate) const fn is_space(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\x0c')
}

/// What glues a word to the word before it, in a block tag's arguments and
/// inside an inline tag: the reference after `{@link` or `@see`, and the
/// name after `@param`, stay with the tag's name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Arg {
    None,
    /// The next word is a block tag's name.
    Name,
    /// The next word is glued: a tag's reference or name.
    First,
    /// Inside the parentheses of a reference (`#m(int, long)`), glued.
    Parens(i32),
    /// Inside `{@code}` or `{@literal}`: every word is glued.
    Atom,
}

impl Arg {
    /// The state at the start of an inline tag named `name`.
    pub(crate) fn inline(name: &str) -> Self {
        if name == "code" || name == "literal" {
            Self::Atom
        } else {
            Self::First
        }
    }

    /// Whether the word `piece` is glued to the word before, and the state
    /// after it.
    pub(crate) fn glue(&mut self, piece: &str) -> bool {
        let parens = |depth: i32| {
            let depth = piece.bytes().fold(depth, |depth, byte| match byte {
                b'(' => depth + 1,
                b')' => depth - 1,
                _ => depth,
            });
            if depth > 0 {
                Self::Parens(depth)
            } else {
                Self::None
            }
        };
        let (glue, next) = match *self {
            Self::None => (false, Self::None),
            Self::Name => (false, Self::First),
            Self::First => (true, parens(0)),
            Self::Parens(depth) => (true, parens(depth)),
            Self::Atom => (true, Self::Atom),
        };
        *self = next;
        glue
    }
}
