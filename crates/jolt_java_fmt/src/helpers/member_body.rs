use jolt_fmt_ir::{Doc, DocBuilder};

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum MemberBodyCategory {
    Field,
    Constructor,
    Method,
    Initializer,
    Type,
}

pub(crate) struct MemberBodyItem<'source> {
    pub(crate) category: Option<MemberBodyCategory>,
    pub(crate) starts_after_blank_line: bool,
    pub(crate) hard_line_before: bool,
    pub(crate) doc: Doc<'source>,
    pub(crate) visible: bool,
}

impl<'source> MemberBodyItem<'source> {
    pub(crate) fn comment(doc: Doc<'source>) -> Self {
        Self {
            category: None,
            starts_after_blank_line: false,
            hard_line_before: false,
            doc,
            visible: true,
        }
    }

    pub(crate) fn ignored(doc: Doc<'source>, category: MemberBodyCategory) -> Self {
        Self {
            category: Some(category),
            starts_after_blank_line: false,
            hard_line_before: false,
            doc,
            visible: true,
        }
    }

    pub(crate) fn invisible(doc: Doc<'source>) -> Self {
        Self {
            category: None,
            starts_after_blank_line: false,
            hard_line_before: false,
            doc,
            visible: false,
        }
    }

    pub(crate) fn without_blank_line_before(self) -> Self {
        Self {
            starts_after_blank_line: false,
            hard_line_before: true,
            ..self
        }
    }
}

pub(crate) fn join_member_body<'source>(
    doc: &mut DocBuilder<'source>,
    members: Vec<MemberBodyItem<'source>>,
) -> Doc<'source> {
    let mut previous_category = None;
    let mut previous_was_neutral = false;
    let mut saw_visible = false;

    doc.concat_list(|joined| {
        for member in members {
            if !member.visible {
                joined.push(member.doc);
                continue;
            }
            if saw_visible {
                let separator = member_separator(
                    joined,
                    previous_category,
                    member.category,
                    member.starts_after_blank_line,
                    member.hard_line_before,
                    previous_was_neutral,
                );
                joined.push(separator);
            }
            saw_visible = true;
            previous_was_neutral = member.category.is_none();
            if let Some(category) = member.category {
                previous_category = Some(category);
            }
            joined.push(member.doc);
        }
    })
}

fn member_separator<'source>(
    doc: &mut DocBuilder<'source>,
    previous_category: Option<MemberBodyCategory>,
    current_category: Option<MemberBodyCategory>,
    starts_after_blank_line: bool,
    hard_line_before: bool,
    previous_was_neutral: bool,
) -> Doc<'source> {
    if previous_was_neutral || hard_line_before {
        return doc.hard_line();
    }
    if starts_after_blank_line {
        return doc.empty_line();
    }

    match (previous_category, current_category) {
        (Some(MemberBodyCategory::Field), Some(MemberBodyCategory::Field))
        | (None, Some(_))
        | (_, None) => doc.hard_line(),
        _ => doc.empty_line(),
    }
}
