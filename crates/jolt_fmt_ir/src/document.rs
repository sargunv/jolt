use std::borrow::Cow;
use std::marker::PhantomData;

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
    children: Vec<DocChild<'source>>,
}

impl<'source> DocArena<'source> {
    pub(crate) fn node(&self, doc: Doc<'source>) -> Option<&DocNode<'source>> {
        if doc.is_nil() {
            return None;
        }
        self.nodes.get(usize::try_from(doc.id().0).ok()?)
    }

    pub(crate) fn child(&self, id: ChildId) -> &DocChild<'source> {
        &self.children[usize::try_from(id.0).expect("doc child id fits usize")]
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

    fn push_child(&mut self, doc: Doc<'source>, next: Option<ChildId>) -> ChildId {
        let id = ChildId(
            self.children
                .len()
                .try_into()
                .expect("doc arena child count fits u32"),
        );
        self.children.push(DocChild { doc, next });
        id
    }
}

#[derive(Default)]
pub struct DocBuilder<'source> {
    arena: DocArena<'source>,
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
        let mut concat = ConcatBuilder::new();
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
        let mut concat = ConcatBuilder::new();
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

    #[must_use]
    pub const fn list(&self) -> DocList<'source> {
        DocList {
            concat: ConcatBuilder::new(),
        }
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

    fn push_child(&mut self, doc: Doc<'source>, next: Option<ChildId>) -> ChildId {
        self.arena.push_child(doc, next)
    }
}

pub struct DocList<'source> {
    concat: ConcatBuilder<'source>,
}

impl<'source> DocList<'source> {
    pub fn push(&mut self, doc: Doc<'source>, builder: &mut DocBuilder<'source>) {
        self.concat.push(doc, builder);
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.concat.is_empty()
    }

    #[must_use]
    pub fn finish(self, builder: &mut DocBuilder<'source>) -> Doc<'source> {
        self.concat.finish(builder)
    }
}

struct ConcatBuilder<'source> {
    first: Option<Doc<'source>>,
    head: Option<ChildId>,
    len: u32,
}

impl<'source> ConcatBuilder<'source> {
    const fn new() -> Self {
        Self {
            first: None,
            head: None,
            len: 0,
        }
    }

    const fn is_empty(&self) -> bool {
        self.len == 0
    }

    fn push(&mut self, doc: Doc<'source>, builder: &mut DocBuilder<'source>) {
        if doc.is_nil() {
            return;
        }

        match self.len {
            0 => {
                self.first = Some(doc);
                self.len = 1;
            }
            1 => {
                let first = self.first.take().expect("first concat doc exists");
                let first = builder.push_child(first, None);
                self.head = Some(builder.push_child(doc, Some(first)));
                self.len = 2;
            }
            _ => {
                self.head = Some(builder.push_child(doc, self.head));
                self.len = self
                    .len
                    .checked_add(1)
                    .expect("concat child count fits u32");
            }
        }
    }

    fn finish(self, builder: &mut DocBuilder<'source>) -> Doc<'source> {
        match self.len {
            0 => builder.nil(),
            1 => self.first.unwrap_or_else(|| builder.nil()),
            _ => builder.push_node(DocNode::Concat {
                head: self.head.expect("concat head exists"),
            }),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum DocNode<'source> {
    Text(Text<'source>),
    LiteralText(LiteralText<'source>),
    Concat {
        head: ChildId,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ChildId(u32);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct DocChild<'source> {
    pub(crate) doc: Doc<'source>,
    pub(crate) next: Option<ChildId>,
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
