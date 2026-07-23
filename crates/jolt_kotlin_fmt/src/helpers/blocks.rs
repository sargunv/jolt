use jolt_fmt_ir::{Doc, DocBuilder, LayoutDoc};
pub(crate) struct BodyItem<'source> {
    doc: Doc<'source>,
    separator: BodyItemSeparator,
    visible: bool,
}

#[derive(Clone, Copy)]
pub(crate) enum BodyItemSeparator {
    None,
    Line,
    EmptyLine,
}

impl BodyItemSeparator {
    pub(crate) fn doc<'source>(self, doc: &mut DocBuilder<'source>) -> Doc<'source> {
        match self {
            Self::None => doc.nil(),
            Self::Line => doc.hard_line(),
            Self::EmptyLine => doc.empty_line(),
        }
    }
}

impl<'source> BodyItem<'source> {
    pub(crate) fn new(doc: Doc<'source>, separator: BodyItemSeparator) -> Self {
        Self {
            doc,
            separator,
            visible: true,
        }
    }

    pub(crate) fn invisible(doc: Doc<'source>) -> Self {
        Self {
            doc,
            separator: BodyItemSeparator::None,
            visible: false,
        }
    }

    pub(crate) fn without_blank_line_before(self) -> Self {
        Self {
            separator: BodyItemSeparator::Line,
            ..self
        }
    }
}

pub(crate) fn join_hard_lines<'source>(
    doc: &mut DocBuilder<'source>,
    docs: impl IntoIterator<Item = Doc<'source>>,
) -> Doc<'source> {
    let separator = doc.hard_line();
    doc.join(separator, docs)
}

pub(crate) fn join_body_items<'source>(
    doc: &mut DocBuilder<'source>,
    items: Vec<BodyItem<'source>>,
) -> LayoutDoc<'source> {
    let mut has_visible_item = false;
    let joined = doc.concat_list(|joined| {
        for item in items {
            if item.visible && has_visible_item {
                let separator = item.separator.doc(joined);
                joined.push(separator);
            }
            joined.push(item.doc);
            has_visible_item |= item.visible;
        }
    });
    LayoutDoc::from_visibility(joined, has_visible_item)
}
