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
    max_line_length: usize,
    indent_width: u16,
}

impl JavaFormatPolicy {
    pub(crate) const fn with_render_options(
        profile: JavaFormatProfile,
        max_line_length: usize,
        indent_width: u16,
    ) -> Self {
        Self {
            profile,
            max_line_length,
            indent_width,
        }
    }

    pub(crate) const fn continuation_indent_levels(self) -> u16 {
        2
    }

    pub(crate) const fn continuation_indent_columns(self) -> usize {
        self.continuation_indent_levels() as usize * self.indent_width as usize
    }

    /// Minimum accumulated receiver length before breaking before a selector.
    /// Matches google-java-format's `indentationMultiplier * 4`.
    pub(crate) const fn selector_chain_min_receiver_length_before_break(self) -> usize {
        self.indent_width as usize * 4
    }

    pub(crate) const fn type_argument_indent_levels(self) -> u16 {
        4
    }

    /// Nested generic type arguments are already inside an enclosing generic
    /// continuation. Google/AOSP step inner `<...>` bodies by one continuation
    /// level instead of repeating the top-level plus-four indent.
    pub(crate) const fn nested_type_argument_indent_levels(self) -> u16 {
        self.continuation_indent_levels()
    }

    pub(crate) const fn type_arguments_break_nested_generic_items(self) -> bool {
        true
    }

    /// Selector invocation type arguments are emitted inside the selector chain
    /// continuation, so google-java-format's plusIndent maps to one Java
    /// continuation level here rather than the wider nested-type indent.
    pub(crate) const fn selector_type_argument_indent_levels(self) -> u16 {
        self.continuation_indent_levels()
    }

    pub(crate) const fn selector_invocation_head_indent_levels(self) -> u16 {
        self.selector_type_argument_indent_levels()
    }

    /// Class header clauses (`extends Foo<...>`) already sit under the header
    /// continuation indent in google-java-format, so their type argument lists
    /// use one continuation level instead of the normal generic-list indent.
    pub(crate) const fn type_clause_type_argument_indent_levels(
        self,
        has_multiple_clause_types: bool,
    ) -> u16 {
        if has_multiple_clause_types {
            self.type_argument_indent_levels()
        } else {
            self.continuation_indent_levels()
        }
    }

    /// In long type clauses with sibling clause types, Google/AOSP keep generic
    /// type arguments vertical once the list breaks instead of filling adjacent
    /// long type names on the same continuation line.
    pub(crate) const fn type_clause_type_arguments_one_per_line(
        self,
        has_multiple_clause_types: bool,
    ) -> bool {
        has_multiple_clause_types
            && matches!(
                self.profile,
                JavaFormatProfile::Google | JavaFormatProfile::Aosp
            )
    }

    pub(crate) const fn declaration_type_parameter_indent_levels(
        self,
        has_following_type_clauses: bool,
    ) -> u16 {
        if has_following_type_clauses {
            self.type_argument_indent_levels()
        } else {
            self.continuation_indent_levels()
        }
    }

    /// Google/AOSP declaration headers fill short type-parameter lists before
    /// falling back to one-per-line behavior for long declaration headers.
    pub(crate) const fn declaration_type_parameters_fill(self) -> bool {
        matches!(
            self.profile,
            JavaFormatProfile::Google | JavaFormatProfile::Aosp
        )
    }

    pub(crate) const fn declaration_type_parameters_fill_max_items(self) -> usize {
        4
    }

    /// GJF emits declaration modifiers with fill-style breaks before a simple
    /// non-generic type header, allowing `static class Short` to stay flat while
    /// `static` breaks before an overlong `class VeryLongName...` header.
    pub(crate) const fn type_declaration_modifiers_fill_before_simple_header(self) -> bool {
        true
    }

    pub(crate) const fn callable_type_parameter_indent_levels(self) -> u16 {
        self.type_argument_indent_levels()
    }

    /// Callable headers scan return types inside the header's zero-indent scope
    /// while generic lists still break vertically. Google/AOSP therefore step
    /// outer return-type `<...>` bodies by one continuation level instead of the
    /// wider field/local generic-list indent.
    pub(crate) const fn callable_leading_return_type_type_argument_indent_levels(self) -> u16 {
        self.continuation_indent_levels()
    }

    /// Google/AOSP declaration headers keep short generic leading types with
    /// the name, but once the leading type itself is too wide to fit as a header
    /// unit the name moves to the plus-four continuation line.
    pub(crate) const fn declaration_leading_type_forces_name_break(
        self,
        has_type_arguments: bool,
        rendered_leading_type_source_width: usize,
        rendered_declaration_head_source_width: usize,
    ) -> bool {
        (has_type_arguments && rendered_leading_type_source_width > self.max_line_length)
            || rendered_declaration_head_source_width
                > self
                    .max_line_length
                    .saturating_sub(self.continuation_indent_columns())
    }

    /// Field declarations use GJF's `declareOne` type/name break.
    pub(crate) const fn field_leading_type_forces_name_break(
        self,
        rendered_leading_type_source_width: usize,
        rendered_declaration_head_source_width: usize,
    ) -> bool {
        self.declaration_leading_type_forces_name_break(
            false,
            rendered_leading_type_source_width,
            rendered_declaration_head_source_width,
        )
    }

    pub(crate) const fn switch_record_pattern_component_indent_levels(self) -> u16 {
        match self.profile {
            JavaFormatProfile::Google | JavaFormatProfile::Aosp => 4,
            JavaFormatProfile::Palantir => 2,
        }
    }

    pub(crate) const fn field_annotations_prefer_horizontal(self) -> bool {
        true
    }

    /// After vertical parameter annotations, Google/AOSP keep the type and name
    /// as a grouped pair; the name only moves to its own continuation line when
    /// the pair does not fit.
    pub(crate) const fn annotated_parameter_groups_type_and_name(self) -> bool {
        true
    }

    pub(crate) const fn separates_static_import_section(self) -> bool {
        matches!(self.profile, JavaFormatProfile::Aosp)
    }

    pub(crate) const fn selector_chain_breaks_before_first_selector(self) -> bool {
        true
    }

    pub(crate) const fn selector_chain_breaks_before_first_selector_for_role(
        self,
        role: ChainRole,
    ) -> bool {
        let _ = role;
        self.selector_chain_breaks_before_first_selector()
    }

    pub(crate) const fn selector_chain_preserves_nested_argument_head(
        self,
        role: ChainRole,
    ) -> bool {
        let _ = role;
        false
    }

    /// Google/AOSP regular dot chains keep simple receiver + zero-arg call
    /// runs together until the width-driven break loop chooses a later dot.
    pub(crate) const fn selector_chain_coalesces_simple_receiver_call_run(
        self,
        role: ChainRole,
    ) -> bool {
        matches!(role, ChainRole::Default) && self.max_line_length >= 100
    }

    pub(crate) const fn selector_chain_simple_receiver_call_run_max_base_width(self) -> usize {
        8
    }

    pub(crate) const fn selector_chain_primary_selector_indent_levels(
        self,
        base_kind: ChainBaseKind,
    ) -> u16 {
        match (self.profile, base_kind) {
            (
                JavaFormatProfile::Google | JavaFormatProfile::Aosp,
                ChainBaseKind::CastPrimaryExpression,
            ) => self.continuation_indent_levels() * 2,
            _ => self.continuation_indent_levels(),
        }
    }

    pub(crate) const fn selector_chain_receiver_argument_indent_levels(self) -> u16 {
        self.continuation_indent_levels() * 2
    }

    pub(crate) const fn array_access_index_indent_levels(self) -> u16 {
        self.continuation_indent_levels()
    }

    pub(crate) const fn method_reference_type_qualifier_uses_selector_chain(self) -> bool {
        true
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
            ChainRole::Default => false,
            ChainRole::LambdaBody => matches!(base_kind, ChainBaseKind::Call),
            ChainRole::NestedArgument | ChainRole::NestedArgumentFit => false,
        }
    }

    pub(crate) const fn selector_chain_long_receiver_width(self) -> usize {
        28
    }

    pub(crate) const fn normalizes_text_block_indentation(self) -> bool {
        true
    }

    pub(crate) const fn reflows_string_literals(self) -> bool {
        matches!(self.profile, JavaFormatProfile::Google)
    }

    /// Matches google-java-format's `MAX_ITEM_LENGTH_FOR_FILLING`.
    pub(crate) const fn max_line_length(self) -> usize {
        self.max_line_length
    }

    pub(crate) const fn argument_list_max_item_length_for_filling(self) -> usize {
        10
    }

    /// Once a receiver-call argument list is nested inside another argument,
    /// Google/AOSP stop filling larger short-item lists and let the broken
    /// continuation shape keep one argument per line.
    pub(crate) const fn argument_list_nested_fill_max_items(self) -> usize {
        4
    }

    pub(crate) const fn argument_list_single_nested_invocation_head_min_width(self) -> usize {
        24
    }

    /// Parameter-commented arguments stay flat when the whole call fits, but
    /// broken multi-argument calls use one item per line.
    pub(crate) const fn argument_list_breaks_inline_commented_items_one_per_line(self) -> bool {
        true
    }

    /// The first nested argument layer may keep receiver/call heads cohesive;
    /// deeper nested arguments are emitted in the fully broken nested role.
    pub(crate) const fn nested_argument_selector_chain_fit_depth_limit(self) -> usize {
        1
    }

    /// Dense scalar array initializers in the Google/AOSP oracle prefer breaking
    /// before an exact-width row would consume the final column.
    pub(crate) const fn array_initializer_tight_fit_min_items(self) -> usize {
        20
    }

    /// Google/AOSP expression lambdas keep short bodies on the arrow line, but
    /// when the body does not fit or owns leading comments the body starts on a
    /// continuation line after `->`.
    pub(crate) const fn lambda_expression_body_breaks_after_arrow(self) -> bool {
        true
    }

    pub(crate) const fn lambda_expression_body_indent_levels(self) -> u16 {
        self.continuation_indent_levels()
    }

    /// Google/AOSP expression lambda bodies break long binary chains one
    /// operator per continuation line once the body moves after `->`.
    pub(crate) const fn lambda_body_binary_chain_breaks_one_per_line(self) -> bool {
        true
    }
}
