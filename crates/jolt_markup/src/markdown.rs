//! The body parser of Markdown doc comments (`///`, JEP 467). Not written
//! yet: Jolt prints `///` comments as written. It will recognize the
//! `CommonMark` blocks, fill paragraphs and block tag text with the shared
//! [`Layout`], and keep every other block as written.
//!
//! [`Layout`]: crate::layout::Layout
