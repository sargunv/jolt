//! Doc comment layout: fills the text of doc comments to the line width and
//! lays out their paragraphs, lists, block tags and code in one fixed style.
//!
//! Each flavour reads a comment in two stages:
//!
//! 1. A tokenizer (`javadoc` for `/**`, `markdown` for `///`, JEP 467),
//!    the only code that reads the comment's characters. It frames the
//!    comment (`frame`), finds its tags (`tags`) and blocks, and returns a
//!    flat list of borrowed tokens (`token`), or `None` when the comment has
//!    a construct outside the subset that Jolt lays out. Then the comment is
//!    printed as written, as N9 prints any other comment.
//! 2. A layout (`html` for `/**`, `markdown` for `///`) that places the
//!    tokens in lines and blocks, and `layout`, which turns those into
//!    `jolt_fmt` docs. Words are filled by the printer's fit check, so the
//!    layout doesn't know the width or the comment's column.
//!
//! What javadoc renders stays the same, and every character is kept: the
//! oracle in `jolt_test_support` checks it.

mod frame;
mod html;
mod javadoc;
mod layout;
mod markdown;
mod tags;
mod token;

use jolt_fmt::{Doc, DocBuilder};

/// Lays out a Javadoc comment (`/** … */`, delimiters included) that starts
/// its output line, or `None` to print it as written (see `javadoc` for
/// which comments those are). The layout starts at the comment's column.
pub fn javadoc<'a>(docs: &mut DocBuilder<'a>, text: &'a str) -> Option<Doc> {
    let tokens = javadoc::tokenize(text)?;
    let mut layout = layout::Layout::new(docs, layout::Flavour::Javadoc);
    html::lay_out(&tokens, &mut layout);
    Some(layout.finish())
}

/// Lays out a Markdown doc comment (JEP 467): a run of `///` lines, each with
/// its `///` and without its line break, that starts its output line, or
/// `None` to keep the run as written (see `markdown` for which runs those
/// are). The layout starts at the comment's column and ends the line after
/// the run.
pub fn markdown<'a>(docs: &mut DocBuilder<'a>, run: &[&'a str]) -> Option<Doc> {
    let (lines, frame) = frame::markdown(run);
    let tokens = markdown::tokenize(&lines)?;
    let mut layout = layout::Layout::new(docs, layout::Flavour::Markdown { frame });
    markdown::lay_out(&tokens, &mut layout);
    Some(layout.finish())
}
