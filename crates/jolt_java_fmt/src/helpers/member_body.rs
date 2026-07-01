use jolt_fmt_ir::{Doc, concat, hard_line};

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum MemberBodyCategory {
    Field,
    Constructor,
    Method,
    Initializer,
    Type,
}

pub(crate) struct MemberBodyItem {
    pub(crate) category: Option<MemberBodyCategory>,
    pub(crate) starts_after_blank_line: bool,
    pub(crate) doc: Doc,
}

impl MemberBodyItem {
    pub(crate) fn comment(doc: Doc) -> Self {
        Self {
            category: None,
            starts_after_blank_line: false,
            doc,
        }
    }

    pub(crate) fn ignored(doc: Doc, category: MemberBodyCategory) -> Self {
        Self {
            category: Some(category),
            starts_after_blank_line: false,
            doc,
        }
    }

    pub(crate) fn without_blank_line_before(self) -> Self {
        Self {
            starts_after_blank_line: false,
            ..self
        }
    }
}

pub(crate) fn join_member_body(members: Vec<MemberBodyItem>) -> Doc {
    let mut joined = Vec::new();
    let mut previous_category = None;
    let mut previous_was_neutral = false;

    for member in members {
        if !joined.is_empty() {
            joined.push(member_separator(
                previous_category,
                member.category,
                member.starts_after_blank_line,
                previous_was_neutral,
            ));
        }
        previous_was_neutral = member.category.is_none();
        if let Some(category) = member.category {
            previous_category = Some(category);
        }
        joined.push(member.doc);
    }

    concat(joined)
}

fn member_separator(
    previous_category: Option<MemberBodyCategory>,
    current_category: Option<MemberBodyCategory>,
    starts_after_blank_line: bool,
    previous_was_neutral: bool,
) -> Doc {
    if previous_was_neutral {
        return hard_line();
    }
    if starts_after_blank_line {
        return jolt_fmt_ir::empty_line();
    }

    match (previous_category, current_category) {
        (Some(MemberBodyCategory::Field), Some(MemberBodyCategory::Field))
        | (None, Some(_))
        | (_, None) => hard_line(),
        _ => jolt_fmt_ir::empty_line(),
    }
}
