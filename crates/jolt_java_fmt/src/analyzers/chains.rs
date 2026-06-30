use jolt_diagnostics::TextRange;
use jolt_fmt_ir::{Doc, text};

#[derive(Clone)]
pub(crate) struct Chain {
    pub(crate) base: Doc,
    pub(crate) base_trailing_comments: Vec<Doc>,
    pub(crate) members: Vec<ChainMember>,
    pub(crate) metadata: ChainMetadata,
    tail_range: Option<TextRange>,
}

impl Chain {
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

    pub(crate) fn simple_base(base: Doc, source_width: usize, simple_name: Option<String>) -> Self {
        Self::with_base_metadata(
            base,
            Vec::new(),
            BaseMetadata::simple(source_width, simple_name),
        )
    }

    pub(crate) fn complex_base(base: Doc, source_width: usize) -> Self {
        Self::with_base_metadata(base, Vec::new(), BaseMetadata::complex(source_width))
    }

    pub(crate) fn primary_expression_base(base: Doc, source_width: usize) -> Self {
        Self::with_base_metadata(
            base,
            Vec::new(),
            BaseMetadata::primary_expression(source_width),
        )
    }

    pub(crate) fn cast_primary_expression_base(base: Doc, source_width: usize) -> Self {
        Self::with_base_metadata(
            base,
            Vec::new(),
            BaseMetadata::cast_primary_expression(source_width),
        )
    }

    pub(crate) fn call_base(base: Doc, source_width: usize) -> Self {
        Self::with_base_metadata(base, Vec::new(), BaseMetadata::call(source_width))
    }

    pub(crate) fn object_creation_base(base: Doc, source_width: usize) -> Self {
        Self::with_base_metadata(
            base,
            Vec::new(),
            BaseMetadata::object_creation(source_width),
        )
    }

    pub(crate) fn push(&mut self, member: ChainMember) {
        self.members.push(member);
        self.metadata = ChainMetadata::from_parts(self.metadata.base.clone(), &self.members);
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

#[derive(Clone)]
pub(crate) struct BaseMetadata {
    pub(crate) source_width: usize,
    pub(crate) is_complex: bool,
    pub(crate) call_count: usize,
    pub(crate) kind: ChainBaseKind,
    pub(crate) simple_name: Option<String>,
    pub(crate) forces_break_before_first_selector: bool,
    pub(crate) is_qualified_this_super_prefix: bool,
}

impl BaseMetadata {
    pub(crate) fn simple(source_width: usize, simple_name: Option<String>) -> Self {
        Self {
            source_width,
            is_complex: false,
            call_count: 0,
            kind: ChainBaseKind::Simple,
            simple_name,
            forces_break_before_first_selector: false,
            is_qualified_this_super_prefix: false,
        }
    }

    pub(crate) fn qualified_this_super_prefix(
        source_width: usize,
        simple_name: Option<String>,
    ) -> Self {
        Self {
            source_width,
            is_complex: false,
            call_count: 0,
            kind: ChainBaseKind::Simple,
            simple_name,
            forces_break_before_first_selector: false,
            is_qualified_this_super_prefix: true,
        }
    }

    pub(crate) const fn complex(source_width: usize) -> Self {
        Self {
            source_width,
            is_complex: true,
            call_count: 0,
            kind: ChainBaseKind::Complex,
            simple_name: None,
            forces_break_before_first_selector: false,
            is_qualified_this_super_prefix: false,
        }
    }

    pub(crate) const fn primary_expression(source_width: usize) -> Self {
        Self {
            source_width,
            is_complex: true,
            call_count: 0,
            kind: ChainBaseKind::PrimaryExpression,
            simple_name: None,
            forces_break_before_first_selector: true,
            is_qualified_this_super_prefix: false,
        }
    }

    pub(crate) const fn cast_primary_expression(source_width: usize) -> Self {
        Self {
            source_width,
            is_complex: true,
            call_count: 0,
            kind: ChainBaseKind::CastPrimaryExpression,
            simple_name: None,
            forces_break_before_first_selector: true,
            is_qualified_this_super_prefix: false,
        }
    }

    pub(crate) const fn call(source_width: usize) -> Self {
        Self {
            source_width,
            is_complex: false,
            call_count: 1,
            kind: ChainBaseKind::Call,
            simple_name: None,
            forces_break_before_first_selector: false,
            is_qualified_this_super_prefix: false,
        }
    }

    pub(crate) const fn object_creation(source_width: usize) -> Self {
        Self {
            source_width,
            is_complex: false,
            call_count: 0,
            kind: ChainBaseKind::ObjectCreation,
            simple_name: None,
            forces_break_before_first_selector: false,
            is_qualified_this_super_prefix: false,
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum ChainBaseKind {
    Simple,
    Complex,
    PrimaryExpression,
    CastPrimaryExpression,
    Call,
    ObjectCreation,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum ChainRole {
    Default,
    NestedArgument,
    NestedArgumentFit,
    LambdaBody,
}

#[derive(Clone)]
pub(crate) struct ChainMember {
    pub(crate) kind: ChainMemberKind,
    pub(crate) doc: Doc,
    pub(crate) doc_after_chain_break: Option<Doc>,
    pub(crate) doc_as_receiver_head_after_chain_break: Option<Doc>,
    pub(crate) selector_head_doc: Doc,
    pub(crate) selector_suffix_doc: Doc,
    pub(crate) trailing_comments: Vec<Doc>,
    pub(crate) source_width: usize,
    pub(crate) selector_head_width: usize,
    pub(crate) has_type_arguments: bool,
    pub(crate) simple_name: Option<String>,
}

impl ChainMember {
    pub(crate) fn field(doc: Doc, source_width: usize, simple_name: Option<String>) -> Self {
        Self {
            kind: ChainMemberKind::Field,
            selector_head_doc: doc.clone(),
            doc,
            doc_after_chain_break: None,
            doc_as_receiver_head_after_chain_break: None,
            selector_suffix_doc: text(""),
            trailing_comments: Vec::new(),
            source_width,
            selector_head_width: source_width,
            has_type_arguments: false,
            simple_name,
        }
    }

    pub(crate) fn call(
        doc: Doc,
        selector_head_doc: Doc,
        selector_suffix_doc: Doc,
        doc_after_chain_break: Option<Doc>,
        doc_as_receiver_head_after_chain_break: Option<Doc>,
        source_width: usize,
        selector_head_width: usize,
        argument_count: usize,
        has_type_arguments: bool,
        simple_name: Option<String>,
    ) -> Self {
        Self {
            kind: ChainMemberKind::Call { argument_count },
            selector_head_doc,
            doc,
            doc_after_chain_break,
            doc_as_receiver_head_after_chain_break,
            selector_suffix_doc,
            trailing_comments: Vec::new(),
            source_width,
            selector_head_width,
            has_type_arguments,
            simple_name,
        }
    }

    pub(crate) fn array_access(doc: Doc, source_width: usize) -> Self {
        Self {
            kind: ChainMemberKind::ArrayAccess,
            selector_head_doc: doc.clone(),
            doc,
            doc_after_chain_break: None,
            doc_as_receiver_head_after_chain_break: None,
            selector_suffix_doc: text(""),
            trailing_comments: Vec::new(),
            source_width,
            selector_head_width: source_width,
            has_type_arguments: false,
            simple_name: None,
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

/// When a chain contains exactly one call, google-java-format may treat a
/// leading field run plus that call as a single syntactic unit (e.g.
/// `System.err.println(...)` stays flat).
pub(crate) fn single_invocation_coalesced_prefix_len(members: &[ChainMember]) -> usize {
    let call_count = members.iter().filter(|member| member.is_call()).count();
    if call_count != 1 {
        return 0;
    }

    let call_index = members
        .iter()
        .position(ChainMember::is_call)
        .expect("single call checked above");
    if call_index == 0 {
        return 0;
    }

    call_index + 1
}

/// When a chain ends in `.stream()`, `.parallelStream()`, or `.toBuilder()`,
/// google-java-format may keep the prefix through that call flat.
pub(crate) fn stream_suffix_prefix_member_end_index(members: &[ChainMember]) -> Option<usize> {
    members.iter().position(|member| {
        member.is_call()
            && member
                .simple_name
                .as_deref()
                .is_some_and(|name| matches!(name, "stream" | "parallelStream" | "toBuilder"))
    })
}

/// google-java-format's `handleLogStatement` special case keeps fluent logger
/// calls as one prefix and lets the final `log(...)` argument list own its
/// continuation. The method names below are copied from the local GJF oracle.
pub(crate) fn is_gjf_log_statement_chain(base: &BaseMetadata, members: &[ChainMember]) -> bool {
    matches!(base.kind, ChainBaseKind::Simple)
        && base.simple_name.is_some()
        && members
            .last()
            .and_then(|member| member.simple_name.as_deref())
            == Some("log")
        && members[..members.len().saturating_sub(1)]
            .iter()
            .all(|member| member.simple_name.as_deref() != Some("log"))
        && members.iter().all(|member| {
            matches!(member.kind, ChainMemberKind::Call { .. })
                && member.simple_name.as_deref().is_some_and(is_gjf_log_method)
        })
}

fn is_gjf_log_method(name: &str) -> bool {
    matches!(
        name,
        "at" | "atConfig"
            | "atDebug"
            | "atFine"
            | "atFiner"
            | "atFinest"
            | "atInfo"
            | "atMostEvery"
            | "atSevere"
            | "atWarning"
            | "every"
            | "log"
            | "logVarargs"
            | "perUnique"
            | "withCause"
            | "withStackTrace"
    )
}

/// Longest inclusive member index that should stay grouped with the receiver.
///
/// Combines google-java-format's type-name prefix, single-invocation field prefix,
/// and `this`/`super` rules.
pub(crate) fn classified_prefix_member_end_index(
    base: &BaseMetadata,
    members: &[ChainMember],
) -> Option<usize> {
    let mut prefix_end = crate::analyzers::type_names::type_name_prefix_member_end_index(
        base.simple_name.as_deref(),
        members,
    );

    let coalesced = single_invocation_coalesced_prefix_len(members);
    if coalesced > 0 && coalesced == members.len() {
        prefix_end = Some(prefix_end.map_or(coalesced - 1, |end| end.max(coalesced - 1)));
    }

    if matches!(base.simple_name.as_deref(), Some("this" | "super")) && !members.is_empty() {
        prefix_end = Some(prefix_end.map_or(0, |end| end));
    }

    if let Some(stream_end) = stream_suffix_prefix_member_end_index(members) {
        prefix_end = Some(prefix_end.map_or(stream_end, |end| end.max(stream_end)));
    }

    prefix_end
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum ChainGroupKind {
    FieldRun,
    LeadingTypeArgumentCall,
    FluentCallRun,
    ArrayAccessRun,
}

impl ChainGroupKind {
    fn for_member(member: &ChainMember, index: usize) -> Self {
        match member.kind {
            ChainMemberKind::Field => Self::FieldRun,
            ChainMemberKind::Call { .. } if index == 0 && member.has_type_arguments => {
                Self::LeadingTypeArgumentCall
            }
            ChainMemberKind::Call { .. } => Self::FluentCallRun,
            ChainMemberKind::ArrayAccess => Self::ArrayAccessRun,
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) enum ChainMemberKind {
    Field,
    Call { argument_count: usize },
    ArrayAccess,
}

#[derive(Clone)]
pub(crate) struct ChainMetadata {
    pub(crate) base: BaseMetadata,
    pub(crate) call_count: usize,
    pub(crate) total_call_count: usize,
    pub(crate) first_member_width: usize,
    pub(crate) first_member_head_width: usize,
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
        let first_member_head_width = members
            .first()
            .map(|member| member.selector_head_width)
            .unwrap_or_default();
        let first_call_argument_count = members
            .first()
            .map(|member| match member.kind {
                ChainMemberKind::Field => 0,
                ChainMemberKind::Call { argument_count } => argument_count,
                ChainMemberKind::ArrayAccess => 0,
            })
            .unwrap_or_default();

        Self {
            base: base.clone(),
            call_count,
            total_call_count: base.call_count + call_count,
            first_member_width,
            first_member_head_width,
            first_member_is_call,
            first_call_argument_count,
        }
    }
}
