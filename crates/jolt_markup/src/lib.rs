//! Doc comment layout: fills the text of doc comments to the line width and
//! lays out their paragraphs, lists, block tags and code in one fixed style.
//!
//! A doc comment is read in stages that both flavours share, except for the
//! body parser:
//!
//! 1. `frame`: the comment's content lines, borrowed from its source.
//! 2. `tags`: inline tags (`{@…}`, brace-balanced, lexed first) and block
//!    tags (`@param` at a line start).
//! 3. A body parser: `html` for Javadoc (`/**`), and `markdown` for
//!    Markdown doc comments (`///`, JEP 467).
//! 4. `layout`: lines and words become `jolt_fmt` docs. Words are filled
//!    by the printer's fit check, so the layout doesn't know the width or the
//!    comment's column.
//!
//! What javadoc renders stays the same, and every word is kept: the oracle
//! in `jolt_test_support` checks it.

mod frame;
mod html;
mod layout;
mod markdown;
mod tags;

use jolt_fmt::{Doc, DocBuilder};

/// Lays out a Javadoc comment (`/** … */`, delimiters included) that starts
/// its output line, or `None` to print it as written (see `frame` for which
/// comments those are). The layout starts at the comment's column.
pub fn javadoc<'a>(docs: &mut DocBuilder<'a>, text: &'a str) -> Option<Doc> {
    let lines = frame::javadoc(text)?;
    let mut layout = layout::Layout::new(docs);
    html::parse(&lines, &mut layout);
    Some(layout.finish())
}
