use jolt_diagnostics::TextRange;
use jolt_fmt_ir::Doc;

#[derive(Clone)]
pub(crate) struct Chain {
    pub(crate) base: Doc,
    pub(crate) base_trailing_comments: Vec<Doc>,
    pub(crate) members: Vec<ChainMember>,
    pub(crate) metadata: ChainMetadata,
    tail_range: Option<TextRange>,
}

impl Chain {
    pub(crate) fn new(base: Doc, members: Vec<ChainMember>) -> Self {
        Self::with_base_metadata(base, members, BaseMetadata::simple(0))
    }

    pub(crate) fn with_base_metadata(
        base: Doc,
        members: Vec<ChainMember>,
        base_metadata: BaseMetadata,
    ) -> Self {
        let metadata = ChainMetadata::from_parts(base_metadata, &members);
        Self {
            base,
            base_trailing_comments: Vec::new(),
            members,
            metadata,
            tail_range: None,
        }
    }

    pub(crate) fn base(base: Doc) -> Self {
        Self::new(base, Vec::new())
    }

    pub(crate) fn simple_base(base: Doc, source_width: usize) -> Self {
        Self::with_base_metadata(base, Vec::new(), BaseMetadata::simple(source_width))
    }

    pub(crate) fn complex_base(base: Doc, source_width: usize) -> Self {
        Self::with_base_metadata(base, Vec::new(), BaseMetadata::complex(source_width))
    }

    pub(crate) fn call_base(base: Doc, source_width: usize) -> Self {
        Self::with_base_metadata(base, Vec::new(), BaseMetadata::call(source_width))
    }

    pub(crate) fn push(&mut self, member: ChainMember) {
        self.members.push(member);
        self.metadata = ChainMetadata::from_parts(self.metadata.base, &self.members);
    }

    pub(crate) fn with_tail_range(mut self, range: Option<TextRange>) -> Self {
        self.tail_range = range;
        self
    }

    pub(crate) fn tail_range(&self) -> Option<TextRange> {
        self.tail_range
    }

    pub(crate) fn set_tail_range(&mut self, range: Option<TextRange>) {
        self.tail_range = range;
    }

    pub(crate) fn push_trailing_comments_to_tail(&mut self, comments: Vec<Doc>) {
        if comments.is_empty() {
            return;
        }

        if let Some(member) = self.members.last_mut() {
            member.trailing_comments.extend(comments);
        } else {
            self.base_trailing_comments.extend(comments);
        }
    }

    pub(crate) fn groups(&self) -> ChainGroups {
        ChainGroups::from_members(&self.members)
    }
}

#[derive(Clone, Copy)]
pub(crate) struct BaseMetadata {
    pub(crate) source_width: usize,
    pub(crate) is_complex: bool,
    pub(crate) call_count: usize,
}

impl BaseMetadata {
    pub(crate) const fn simple(source_width: usize) -> Self {
        Self {
            source_width,
            is_complex: false,
            call_count: 0,
        }
    }

    pub(crate) const fn complex(source_width: usize) -> Self {
        Self {
            source_width,
            is_complex: true,
            call_count: 0,
        }
    }

    pub(crate) const fn call(source_width: usize) -> Self {
        Self {
            source_width,
            is_complex: false,
            call_count: 1,
        }
    }
}

#[derive(Clone)]
pub(crate) struct ChainMember {
    pub(crate) kind: ChainMemberKind,
    pub(crate) doc: Doc,
    pub(crate) trailing_comments: Vec<Doc>,
    pub(crate) source_width: usize,
    pub(crate) has_type_arguments: bool,
}

impl ChainMember {
    pub(crate) fn field(doc: Doc, source_width: usize) -> Self {
        Self {
            kind: ChainMemberKind::Field,
            doc,
            trailing_comments: Vec::new(),
            source_width,
            has_type_arguments: false,
        }
    }

    pub(crate) fn call(
        doc: Doc,
        source_width: usize,
        argument_count: usize,
        has_type_arguments: bool,
    ) -> Self {
        Self {
            kind: ChainMemberKind::Call { argument_count },
            doc,
            trailing_comments: Vec::new(),
            source_width,
            has_type_arguments,
        }
    }

    pub(crate) fn is_call(&self) -> bool {
        matches!(self.kind, ChainMemberKind::Call { .. })
    }
}

#[derive(Clone, Copy)]
pub(crate) struct ChainGroup {
    pub(crate) kind: ChainGroupKind,
    pub(crate) len: usize,
}

pub(crate) struct ChainGroups {
    groups: Vec<ChainGroup>,
}

impl ChainGroups {
    fn from_members(members: &[ChainMember]) -> Self {
        let mut groups = Vec::new();
        let mut index = 0;
        while index < members.len() {
            let kind = ChainGroupKind::for_member(&members[index], index);
            let mut len = 1;
            while index + len < members.len()
                && ChainGroupKind::for_member(&members[index + len], index + len) == kind
            {
                len += 1;
            }
            groups.push(ChainGroup { kind, len });
            index += len;
        }
        Self { groups }
    }

    pub(crate) fn all_fields(&self, member_count: usize) -> bool {
        self.groups.first().is_some_and(|group| {
            group.kind == ChainGroupKind::FieldRun && group.len == member_count
        })
    }

    pub(crate) fn field_prefix_len(&self) -> usize {
        self.groups
            .first()
            .filter(|group| group.kind == ChainGroupKind::FieldRun)
            .map(|group| group.len)
            .unwrap_or_default()
    }

    pub(crate) fn leading_type_argument_call_len(&self) -> usize {
        self.groups
            .first()
            .filter(|group| group.kind == ChainGroupKind::LeadingTypeArgumentCall)
            .map(|group| group.len)
            .unwrap_or_default()
    }

    pub(crate) fn starts_with_call_run(&self) -> bool {
        self.groups.first().is_some_and(|group| {
            matches!(
                group.kind,
                ChainGroupKind::LeadingTypeArgumentCall | ChainGroupKind::FluentCallRun
            )
        })
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum ChainGroupKind {
    FieldRun,
    LeadingTypeArgumentCall,
    FluentCallRun,
}

impl ChainGroupKind {
    fn for_member(member: &ChainMember, index: usize) -> Self {
        match member.kind {
            ChainMemberKind::Field => Self::FieldRun,
            ChainMemberKind::Call { .. } if index == 0 && member.has_type_arguments => {
                Self::LeadingTypeArgumentCall
            }
            ChainMemberKind::Call { .. } => Self::FluentCallRun,
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) enum ChainMemberKind {
    Field,
    Call { argument_count: usize },
}

#[derive(Clone, Copy)]
pub(crate) struct ChainMetadata {
    pub(crate) base: BaseMetadata,
    pub(crate) call_count: usize,
    pub(crate) total_call_count: usize,
    pub(crate) first_member_width: usize,
    pub(crate) first_member_is_call: bool,
    pub(crate) first_call_argument_count: usize,
}

impl ChainMetadata {
    fn from_parts(base: BaseMetadata, members: &[ChainMember]) -> Self {
        let call_count = members.iter().filter(|member| member.is_call()).count();
        let first_member_is_call = members.first().is_some_and(ChainMember::is_call);
        let first_member_width = members
            .first()
            .map(|member| member.source_width)
            .unwrap_or_default();
        let first_call_argument_count = members
            .first()
            .map(|member| match member.kind {
                ChainMemberKind::Field => 0,
                ChainMemberKind::Call { argument_count } => argument_count,
            })
            .unwrap_or_default();

        Self {
            base,
            call_count,
            total_call_count: base.call_count + call_count,
            first_member_width,
            first_member_is_call,
            first_call_argument_count,
        }
    }
}
