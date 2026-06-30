use crate::analyzers::chains::{ChainBaseKind, ChainRole};
use crate::options::JavaFormatProfile;

/// Centralized profile policy access for Java formatting decisions.
///
/// Profiles are compatibility targets, not independent style knobs. Rule
/// modules should ask this layer for named policies instead of matching on
/// `JavaFormatProfile` near syntax formatting code.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct JavaFormatPolicy {
    profile: JavaFormatProfile,
}

impl JavaFormatPolicy {
    pub(crate) const fn new(profile: JavaFormatProfile) -> Self {
        Self { profile }
    }

    pub(crate) const fn continuation_indent_levels(self) -> u16 {
        2
    }

    pub(crate) const fn type_argument_indent_levels(self) -> u16 {
        4
    }

    pub(crate) const fn separates_static_import_section(self) -> bool {
        matches!(self.profile, JavaFormatProfile::Aosp)
    }

    pub(crate) const fn selector_chain_breaks_before_first_selector(self) -> bool {
        !matches!(self.profile, JavaFormatProfile::Palantir)
    }

    pub(crate) const fn selector_chain_breaks_before_first_selector_for_role(
        self,
        role: ChainRole,
    ) -> bool {
        match (self.profile, role) {
            (JavaFormatProfile::Palantir, _) => false,
            _ => self.selector_chain_breaks_before_first_selector(),
        }
    }

    pub(crate) const fn selector_chain_preserves_nested_argument_head(
        self,
        role: ChainRole,
    ) -> bool {
        matches!(
            (self.profile, role),
            (JavaFormatProfile::Palantir, ChainRole::NestedArgument)
        )
    }

    pub(crate) const fn selector_chain_role_breaks_before_first_selector(
        self,
        role: ChainRole,
        base_kind: ChainBaseKind,
        first_member_is_call: bool,
    ) -> bool {
        if !first_member_is_call {
            return false;
        }

        match role {
            ChainRole::Default => matches!(base_kind, ChainBaseKind::ObjectCreation),
            ChainRole::LambdaBody => {
                matches!(
                    base_kind,
                    ChainBaseKind::Call | ChainBaseKind::ObjectCreation
                )
            }
            ChainRole::NestedArgument => false,
        }
    }

    pub(crate) const fn selector_chain_long_receiver_width(self) -> usize {
        match self.profile {
            JavaFormatProfile::Google | JavaFormatProfile::Aosp => 28,
            JavaFormatProfile::Palantir => usize::MAX,
        }
    }

    pub(crate) const fn normalizes_text_block_indentation(self) -> bool {
        matches!(
            self.profile,
            JavaFormatProfile::Google | JavaFormatProfile::Aosp
        )
    }
}
