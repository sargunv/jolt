use std::borrow::Cow;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

use crate::width::{TextWidth, display_width, literal_text_metrics};

/// Copyable formatter document handle.
///
/// Documents are allocated into a [`DocBuilder`] for one formatting run. This
/// handle indexes that builder's arena and does not own recursive child data.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Doc<'source> {
    id: DocId,
    source: PhantomData<&'source str>,
}

impl Doc<'_> {
    const NIL_ID: DocId = DocId(u32::MAX);

    #[must_use]
    pub const fn nil() -> Self {
        Self::new(Self::NIL_ID)
    }

    const fn new(id: DocId) -> Self {
        Self {
            id,
            source: PhantomData,
        }
    }

    pub(crate) const fn id(self) -> DocId {
        self.id
    }

    pub(crate) const fn is_nil(self) -> bool {
        self.id.0 == Self::NIL_ID.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DocId(u32);

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DocArena<'source> {
    nodes: Vec<DocNode<'source>>,
    children: Vec<Doc<'source>>,
}

impl<'source> DocArena<'source> {
    pub(crate) fn node(&self, doc: Doc<'source>) -> Option<&DocNode<'source>> {
        if doc.is_nil() {
            return None;
        }
        self.nodes.get(usize::try_from(doc.id().0).ok()?)
    }

    pub(crate) fn child(&self, index: u32) -> Doc<'source> {
        self.children[usize::try_from(index).expect("doc child index fits usize")]
    }

    fn child_count(&self) -> u32 {
        self.children
            .len()
            .try_into()
            .expect("doc arena child count fits u32")
    }

    fn push_node(&mut self, node: DocNode<'source>) -> Doc<'source> {
        let id = DocId(
            self.nodes
                .len()
                .try_into()
                .expect("doc arena node count fits u32"),
        );
        self.nodes.push(node);
        Doc::new(id)
    }

    fn push_child(&mut self, doc: Doc<'source>) {
        self.children.push(doc);
    }
}

#[derive(Default)]
pub struct DocBuilder<'source> {
    arena: DocArena<'source>,
    list_scratch: Vec<Doc<'source>>,
}

impl<'source> DocBuilder<'source> {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub const fn nil(&self) -> Doc<'source> {
        Doc::new(Doc::NIL_ID)
    }

    #[must_use]
    pub fn text(&mut self, value: impl Into<Cow<'source, str>>) -> Doc<'source> {
        let text = value.into();
        let width = display_width(&text);
        self.push_node(DocNode::Text(Text { text, width }))
    }

    #[must_use]
    pub fn space(&mut self) -> Doc<'source> {
        self.text(" ")
    }

    #[must_use]
    pub fn literal_text(&mut self, value: impl Into<Cow<'source, str>>) -> Doc<'source> {
        let text = value.into();
        let metrics = literal_text_metrics(&text);
        self.push_node(DocNode::LiteralText(LiteralText {
            text,
            final_width: metrics.final_width,
            line_count: metrics.line_count,
        }))
    }

    #[must_use]
    pub fn concat(&mut self, docs: impl IntoIterator<Item = Doc<'source>>) -> Doc<'source> {
        let mut concat = ConcatAppender::new();
        for doc in docs {
            concat.push(doc, self);
        }
        concat.finish(self)
    }

    #[must_use]
    pub fn join(
        &mut self,
        separator: Doc<'source>,
        docs: impl IntoIterator<Item = Doc<'source>>,
    ) -> Doc<'source> {
        let mut concat = ConcatAppender::new();
        let mut needs_separator = false;
        for doc in docs {
            if needs_separator {
                concat.push(separator, self);
            } else {
                needs_separator = true;
            }
            concat.push(doc, self);
        }
        concat.finish(self)
    }

    /// Builds a concatenation using reusable builder scratch storage.
    ///
    /// # Panics
    ///
    /// Panics if the list exceeds the supported document size.
    #[must_use]
    pub fn concat_list(
        &mut self,
        build: impl FnOnce(&mut ConcatBuilder<'_, 'source>),
    ) -> Doc<'source> {
        let start = self.list_scratch.len();
        let mut list = ConcatBuilder {
            builder: self,
            start,
            active: true,
        };
        build(&mut list);
        list.finish()
    }

    #[must_use]
    pub fn group(&mut self, contents: Doc<'source>) -> Doc<'source> {
        self.group_with_break(contents, false)
    }

    #[must_use]
    pub fn force_group(&mut self, contents: Doc<'source>) -> Doc<'source> {
        self.group_with_break(contents, true)
    }

    #[must_use]
    pub fn indent(&mut self, contents: Doc<'source>) -> Doc<'source> {
        if contents.is_nil() {
            return contents;
        }

        self.push_node(DocNode::Indent {
            contents,
            levels: 1,
        })
    }

    #[must_use]
    pub fn line(&mut self) -> Doc<'source> {
        self.push_node(DocNode::Line(Line {
            mode: LineMode::SoftOrSpace,
            flat: FlatLine::Space,
            indent_delta: 0,
        }))
    }

    #[must_use]
    pub fn soft_line(&mut self) -> Doc<'source> {
        self.push_node(DocNode::Line(Line {
            mode: LineMode::Soft,
            flat: FlatLine::Empty,
            indent_delta: 0,
        }))
    }

    #[must_use]
    pub fn hard_line(&mut self) -> Doc<'source> {
        self.push_node(DocNode::Line(Line {
            mode: LineMode::Hard,
            flat: FlatLine::Empty,
            indent_delta: 0,
        }))
    }

    #[must_use]
    pub fn empty_line(&mut self) -> Doc<'source> {
        self.push_node(DocNode::Line(Line {
            mode: LineMode::Empty,
            flat: FlatLine::Empty,
            indent_delta: 0,
        }))
    }

    #[must_use]
    pub fn if_break(&mut self, breaks: Doc<'source>, flat: Doc<'source>) -> Doc<'source> {
        self.push_node(DocNode::IfBreak { breaks, flat })
    }

    #[must_use]
    pub fn into_arena(self) -> DocArena<'source> {
        self.arena
    }

    fn group_with_break(&mut self, contents: Doc<'source>, should_break: bool) -> Doc<'source> {
        if contents.is_nil() {
            return contents;
        }

        self.push_node(DocNode::Group {
            contents,
            should_break,
        })
    }

    fn push_node(&mut self, node: DocNode<'source>) -> Doc<'source> {
        self.arena.push_node(node)
    }

    fn child_count(&self) -> u32 {
        self.arena.child_count()
    }

    fn push_child(&mut self, doc: Doc<'source>) {
        self.arena.push_child(doc);
    }
}

/// Scoped dynamic concatenation backed by reusable [`DocBuilder`] scratch.
pub struct ConcatBuilder<'builder, 'source> {
    builder: &'builder mut DocBuilder<'source>,
    start: usize,
    active: bool,
}

impl<'source> ConcatBuilder<'_, 'source> {
    /// Appends a document to this concatenation.
    pub fn push(&mut self, doc: Doc<'source>) {
        if !doc.is_nil() {
            self.builder.list_scratch.push(doc);
        }
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.builder.list_scratch.len() == self.start
    }

    fn finish(mut self) -> Doc<'source> {
        let len = self.builder.list_scratch.len() - self.start;
        let doc = match len {
            0 => self.builder.nil(),
            1 => self
                .builder
                .list_scratch
                .pop()
                .expect("concat list item exists"),
            _ => {
                let len = u32::try_from(len).expect("concat list length fits u32");
                let child_start = self.builder.child_count();
                child_start
                    .checked_add(len)
                    .expect("doc arena child count fits u32");
                for index in self.start..self.builder.list_scratch.len() {
                    self.builder
                        .arena
                        .push_child(self.builder.list_scratch[index]);
                }
                self.builder.list_scratch.truncate(self.start);
                self.builder.push_node(DocNode::Concat {
                    start: child_start,
                    len,
                })
            }
        };
        self.active = false;
        doc
    }
}

impl Drop for ConcatBuilder<'_, '_> {
    fn drop(&mut self) {
        if self.active && self.builder.list_scratch.len() >= self.start {
            self.builder.list_scratch.truncate(self.start);
        }
    }
}

impl<'source> Deref for ConcatBuilder<'_, 'source> {
    type Target = DocBuilder<'source>;

    fn deref(&self) -> &Self::Target {
        self.builder
    }
}

impl DerefMut for ConcatBuilder<'_, '_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.builder
    }
}

struct ConcatAppender<'source> {
    first: Option<Doc<'source>>,
    start: Option<u32>,
    len: u32,
}

impl<'source> ConcatAppender<'source> {
    const fn new() -> Self {
        Self {
            first: None,
            start: None,
            len: 0,
        }
    }

    fn push(&mut self, doc: Doc<'source>, builder: &mut DocBuilder<'source>) {
        if doc.is_nil() {
            return;
        }

        match self.start {
            Some(_) => {
                builder.push_child(doc);
                self.len = self
                    .len
                    .checked_add(1)
                    .expect("concat child count fits u32");
            }
            None if self.first.is_none() => {
                self.first = Some(doc);
                self.len = 1;
            }
            None => {
                let first = self.first.take().expect("first concat doc exists");
                let start = builder.child_count();
                builder.push_child(first);
                builder.push_child(doc);
                self.start = Some(start);
                self.len = 2;
            }
        }
    }

    fn finish(self, builder: &mut DocBuilder<'source>) -> Doc<'source> {
        match self.start {
            Some(start) => {
                start
                    .checked_add(self.len)
                    .expect("doc arena child count fits u32");
                builder.push_node(DocNode::Concat {
                    start,
                    len: self.len,
                })
            }
            None => self.first.unwrap_or_else(|| builder.nil()),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum DocNode<'source> {
    Text(Text<'source>),
    LiteralText(LiteralText<'source>),
    Concat {
        start: u32,
        len: u32,
    },
    Group {
        contents: Doc<'source>,
        should_break: bool,
    },
    Indent {
        contents: Doc<'source>,
        levels: i16,
    },
    Line(Line),
    IfBreak {
        breaks: Doc<'source>,
        flat: Doc<'source>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Text<'source> {
    pub(crate) text: Cow<'source, str>,
    pub(crate) width: TextWidth,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct LiteralText<'source> {
    pub(crate) text: Cow<'source, str>,
    final_width: TextWidth,
    line_count: usize,
}

impl LiteralText<'_> {
    pub(crate) const fn final_width(&self) -> TextWidth {
        self.final_width
    }

    pub(crate) const fn is_multiline(&self) -> bool {
        self.line_count > 1
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Line {
    pub(crate) mode: LineMode,
    pub(crate) flat: FlatLine,
    pub(crate) indent_delta: i16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum LineMode {
    Soft,
    SoftOrSpace,
    Hard,
    Empty,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum FlatLine {
    Empty,
    Space,
}
