//! Doc comment layout: fills the text of doc comments to the line width and
//! lays out their paragraphs, lists, block tags and code in one fixed style.
//!
//! A comment is read as javadoc reads it, by a port of javac's reader
//! (JDK 25):
//!
//! 1. `frame`: the comment's text as javac frames it, line by line, borrowed
//!    from the source (`JavaTokenizer` and `JavadocTokenizer`).
//! 2. `parser`: javac's `DocCommentParser`, which turns that text into a
//!    tree of text, entities, HTML, inline tags and block tags (`/**`), or
//!    Markdown text, inline tags and block tags (`///`).
//!
//! Then each flavour lays out the tree: `html` for `/**`, and `markdown` for
//! `///`, which reads the Markdown text with pulldown-cmark. Both place
//! words in lines and blocks through `layout`, which builds `jolt_fmt` docs.
//! Words are filled by the printer's fit check, so the layouts don't know
//! the width or the comment's column. Whatever javadoc reads with its
//! whitespace (code, `<pre>`, attribute values, references) is printed as
//! written, the whitespace at the ends of its lines included; only
//! whitespace that HTML or Markdown collapses changes, and only Java's
//! whitespace (space, tab, form feed) counts. A comment that isn't laid out
//! (`None`) is printed as written by `jolt_fmt`, all its trailing whitespace
//! included, since where that whitespace is content wasn't read.
//!
//! The hunt checks what javadoc reads before and after against the JDK
//! itself (`jolt_test_support`'s `doc_oracle`).

mod frame;
mod html;
mod java;
mod layout;
mod markdown;
mod parser;
mod words;

use jolt_fmt::{Doc, DocBuilder};

/// Lays out a Javadoc comment (`/** … */`, delimiters included) that starts
/// its output line, or `None` to print it as written: a banner (`/***`, or a
/// line made only of `*`s), `/**/`, a comment with a Unicode escape (which
/// javac decodes before it reads the comment), and one whose reading would
/// take more than linear time (see `parser::parse`). The layout starts at
/// the comment's column.
pub fn javadoc<'a>(docs: &mut DocBuilder<'a>, text: &'a str) -> Option<Doc> {
    let (lines, opener_text) = frame::javadoc(text)?;
    let comment = parser::parse(&lines, false)?;
    let mut layout = layout::Layout::new(docs, layout::Flavour::Javadoc, &lines);
    html::lay_out(&comment, &lines, opener_text, &mut layout);
    Some(layout.finish())
}

/// Lays out a Markdown doc comment (JEP 467): a run of `///` lines, each with
/// its `///` and without its line break, that starts its output line, or
/// `None` to keep the run as written (a run with a Unicode escape, or one
/// whose reading would take more than linear time). The layout starts at the
/// comment's column and ends the line after the run.
pub fn markdown<'a>(docs: &mut DocBuilder<'a>, run: &[&'a str]) -> Option<Doc> {
    let (lines, frame) = frame::markdown(run)?;
    let comment = parser::parse(&lines, true)?;
    let mut layout = layout::Layout::new(docs, layout::Flavour::Markdown { frame }, &lines);
    markdown::lay_out(&comment, &lines, &mut layout)?;
    Some(layout.finish())
}
