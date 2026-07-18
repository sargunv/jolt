use std::fmt;

pub use jolt_syntax::{
    Comment as KotlinComment, CommentKind as KotlinCommentKind, Comments as KotlinComments,
};
use jolt_syntax::{SyntaxElement, SyntaxNode, SyntaxSlot, SyntaxToken, SyntaxVerbatimCore};
use jolt_text::TextRange;

use crate::{KotlinSyntaxKind, language::KotlinLanguage};

pub type KotlinSyntaxNode<'source> = SyntaxNode<'source, KotlinLanguage>;
pub type KotlinSyntaxToken<'source> = SyntaxToken<'source, KotlinLanguage>;
pub type KotlinSyntaxVerbatimCore<'source> = SyntaxVerbatimCore<'source, KotlinLanguage>;

jolt_syntax::define_typed_cst_fields! {
    language: KotlinLanguage,
    syntax_kind: KotlinSyntaxKind,
    syntax_node: KotlinSyntaxNode,
    invariant_error: KotlinSyntaxInvariantError,
    syntax_result: KotlinSyntaxResult,
    syntax_field: KotlinSyntaxField,
    malformed_syntax: KotlinMalformedSyntax,
    missing_syntax: KotlinMissingSyntax,
    fixed_syntax: KotlinFixedSyntax,
}

fn kotlin_kind_is_category_bogus(_: KotlinSyntaxKind) -> bool {
    false
}

jolt_syntax::define_typed_cst_access! {
    language: KotlinLanguage,
    syntax_kind: KotlinSyntaxKind,
    syntax_node: KotlinSyntaxNode,
    syntax_token: KotlinSyntaxToken,
    invariant_error: KotlinSyntaxInvariantError,
    syntax_result: KotlinSyntaxResult,
    syntax_field: KotlinSyntaxField,
    malformed_syntax: KotlinMalformedSyntax,
    missing_syntax: KotlinMissingSyntax,
    fixed_syntax: KotlinFixedSyntax,
    syntax_view: KotlinSyntaxView,
    typed_node: KotlinTypedNode,
    node: KotlinNode,
    family: KotlinFamily,
    role_element: KotlinRoleElement,
    list_item: KotlinListItem,
    list_part: KotlinSyntaxListPart,
    category_bogus: kotlin_kind_is_category_bogus,
}

macro_rules! kotlin_field_accessor {
    ($module:ident $field:ident required $matcher:tt => $role:ident) => {
        #[inline]
        #[allow(clippy::missing_errors_doc)]
        pub fn $field(&self) -> KotlinSyntaxResult<KotlinSyntaxField<'source, $role<'source>>> {
            required_role_element(self.fixed_syntax(), crate::shape::$module::Slot::$field as usize).map(|slot| {
                slot.map(|element| $role { element })
            })
        }
    };
    ($module:ident $field:ident optional $matcher:tt => $role:ident) => {
        #[inline]
        #[allow(clippy::missing_errors_doc)]
        pub fn $field(&self) -> KotlinSyntaxResult<KotlinSyntaxField<'source, $role<'source>>> {
            required_role_element(self.fixed_syntax(), crate::shape::$module::Slot::$field as usize)
                .map(|slot| slot.map(|element| $role { element }))
        }
    };
    ($module:ident $field:ident required (token $kind:ident)) => {
        #[inline]
        #[allow(clippy::missing_errors_doc)]
        pub fn $field(&self) -> KotlinSyntaxResult<KotlinSyntaxField<'source, KotlinSyntaxToken<'source>>> {
            required_token(self.fixed_syntax(), crate::shape::$module::Slot::$field as usize)
        }
    };
    ($module:ident $field:ident required (token_set $kinds:tt)) => {
        kotlin_field_accessor!($module $field required (token __schema_token_set));
    };
    ($module:ident $field:ident required (contextual $text:literal)) => {
        kotlin_field_accessor!($module $field required (token __schema_contextual));
    };
    ($module:ident $field:ident optional (token $kind:ident)) => {
        #[inline]
        #[allow(clippy::missing_errors_doc)]
        pub fn $field(&self) -> KotlinSyntaxResult<KotlinSyntaxField<'source, KotlinSyntaxToken<'source>>> {
            optional_token(self.fixed_syntax(), crate::shape::$module::Slot::$field as usize)
        }
    };
    ($module:ident $field:ident optional (token_set $kinds:tt)) => {
        kotlin_field_accessor!($module $field optional (token __schema_token_set));
    };
    ($module:ident $field:ident optional (contextual $text:literal)) => {
        kotlin_field_accessor!($module $field optional (token __schema_contextual));
    };

    ($module:ident $field:ident required (node ModuleDirective)) => {
        #[inline]
        #[allow(clippy::missing_errors_doc)]
        pub fn $field(&self) -> KotlinSyntaxResult<KotlinSyntaxField<'source, ModuleDirectiveNode<'source>>> {
            required_node(self.fixed_syntax(), crate::shape::$module::Slot::$field as usize)
        }
    };
    ($module:ident $field:ident optional (node ModuleDirective)) => {
        #[inline]
        #[allow(clippy::missing_errors_doc)]
        pub fn $field(&self) -> KotlinSyntaxResult<KotlinSyntaxField<'source, ModuleDirectiveNode<'source>>> {
            optional_node(self.fixed_syntax(), crate::shape::$module::Slot::$field as usize)
        }
    };
    ($module:ident $field:ident required (node $target:ident)) => {
        #[inline]
        #[allow(clippy::missing_errors_doc)]
        pub fn $field(&self) -> KotlinSyntaxResult<KotlinSyntaxField<'source, $target<'source>>> {
            required_node(self.fixed_syntax(), crate::shape::$module::Slot::$field as usize)
        }
    };
    ($module:ident $field:ident optional (node $target:ident)) => {
        #[inline]
        #[allow(clippy::missing_errors_doc)]
        pub fn $field(&self) -> KotlinSyntaxResult<KotlinSyntaxField<'source, $target<'source>>> {
            optional_node(self.fixed_syntax(), crate::shape::$module::Slot::$field as usize)
        }
    };
    ($module:ident $field:ident required (constructed $target:ident)) => {
        #[inline]
        #[allow(clippy::missing_errors_doc)]
        pub fn $field(&self) -> KotlinSyntaxResult<KotlinSyntaxField<'source, $target<'source>>> {
            required_node(self.fixed_syntax(), crate::shape::$module::Slot::$field as usize)
        }
    };
    ($module:ident $field:ident optional (constructed $target:ident)) => {
        #[inline]
        #[allow(clippy::missing_errors_doc)]
        pub fn $field(&self) -> KotlinSyntaxResult<KotlinSyntaxField<'source, $target<'source>>> {
            optional_node(self.fixed_syntax(), crate::shape::$module::Slot::$field as usize)
        }
    };
    ($module:ident $field:ident required (list $target:ident)) => {
        #[inline]
        #[allow(clippy::missing_errors_doc)]
        pub fn $field(&self) -> KotlinSyntaxResult<KotlinSyntaxField<'source, $target<'source>>> {
            required_node(self.fixed_syntax(), crate::shape::$module::Slot::$field as usize)
        }
    };
    ($module:ident $field:ident optional (list $target:ident)) => {
        #[inline]
        #[allow(clippy::missing_errors_doc)]
        pub fn $field(&self) -> KotlinSyntaxResult<KotlinSyntaxField<'source, $target<'source>>> {
            optional_node(self.fixed_syntax(), crate::shape::$module::Slot::$field as usize)
        }
    };
    ($module:ident $field:ident required (category $target:ident)) => {
        #[inline]
        #[allow(clippy::missing_errors_doc)]
        pub fn $field(&self) -> KotlinSyntaxResult<KotlinSyntaxField<'source, $target<'source>>> {
            required_family(self.fixed_syntax(), crate::shape::$module::Slot::$field as usize)
        }
    };
    ($module:ident $field:ident optional (category $target:ident)) => {
        #[inline]
        #[allow(clippy::missing_errors_doc)]
        pub fn $field(&self) -> KotlinSyntaxResult<KotlinSyntaxField<'source, $target<'source>>> {
            optional_family(self.fixed_syntax(), crate::shape::$module::Slot::$field as usize)
        }
    };

    // Heterogeneous roles are wrapped by the semantic adapters below. This
    // primitive still reads exactly one declared slot and never searches.
    ($module:ident $field:ident required $matcher:tt) => {
        #[inline]
        #[allow(clippy::missing_errors_doc)]
        pub fn $field(&self) -> KotlinSyntaxResult<KotlinSyntaxField<'source, KotlinRoleElement<'source>>> {
            required_role_element(self.fixed_syntax(), crate::shape::$module::Slot::$field as usize)
        }
    };
    ($module:ident $field:ident optional $matcher:tt) => {
        #[inline]
        #[allow(clippy::missing_errors_doc)]
        pub fn $field(&self) -> KotlinSyntaxResult<KotlinSyntaxField<'source, KotlinRoleElement<'source>>> {
            required_role_element(self.fixed_syntax(), crate::shape::$module::Slot::$field as usize)
        }
    };
    ($module:ident $field:ident many $matcher:tt $(=> $role:ident)?) => {};
    ($module:ident $field:ident one_or_more $matcher:tt $(=> $role:ident)?) => {};
}

#[allow(unused_macros)]
macro_rules! define_kotlin_role {
    ($role:ident) => {
        #[derive(Clone, Copy, Debug)]
        pub struct $role<'source> {
            element: KotlinRoleElement<'source>,
        }

        impl<'source> $role<'source> {
            #[must_use]
            pub fn token(self) -> Option<KotlinSyntaxToken<'source>> {
                match self.element {
                    KotlinRoleElement::Token(token) => Some(token),
                    KotlinRoleElement::Node(_) => None,
                }
            }

            #[must_use]
            pub fn first_token(self) -> Option<KotlinSyntaxToken<'source>> {
                match self.element {
                    KotlinRoleElement::Node(node) => node.first_token(),
                    KotlinRoleElement::Token(token) => Some(token),
                }
            }

            #[must_use]
            pub fn last_token(self) -> Option<KotlinSyntaxToken<'source>> {
                match self.element {
                    KotlinRoleElement::Node(node) => node.last_token(),
                    KotlinRoleElement::Token(token) => Some(token),
                }
            }

            #[must_use]
            pub fn cast_node<N: KotlinTypedNode<'source>>(self) -> Option<N> {
                N::cast_element(self.element)
            }

            #[must_use]
            pub fn cast_family<F: KotlinFamily<'source>>(self) -> Option<F> {
                match self.element {
                    KotlinRoleElement::Node(node) => F::cast(node),
                    KotlinRoleElement::Token(_) => None,
                }
            }
        }

        impl<'source> KotlinListItem<'source> for $role<'source> {
            const IS_FAMILY: bool = false;

            fn cast_element(element: KotlinRoleElement<'source>) -> Option<Self> {
                Some(Self { element })
            }
        }
    };
}

macro_rules! kotlin_list_item_type {
    ($source:lifetime; $matcher:tt => $role:ident) => { $role<$source> };
    ($source:lifetime; (node ModuleDirective)) => { ModuleDirectiveNode<$source> };
    ($source:lifetime; (node $target:ident)) => { $target<$source> };
    ($source:lifetime; (constructed $target:ident)) => { $target<$source> };
    ($source:lifetime; (category $target:ident)) => { $target<$source> };
    ($source:lifetime; $matcher:tt) => { KotlinRoleElement<$source> };
}

macro_rules! kotlin_list_item_type_optional_role {
    ($source:lifetime; $matcher:tt; $role:ident) => {
        kotlin_list_item_type!($source; $matcher => $role)
    };
    ($source:lifetime; $matcher:tt;) => {
        kotlin_list_item_type!($source; $matcher)
    };
}

macro_rules! kotlin_list_is_separated {
    ([separated $($policy:tt)*]) => {
        true
    };
    ([$($policy:tt)*]) => {
        false
    };
    () => {
        false
    };
}

macro_rules! kotlin_variable_slot_view {
    (list; $field:ident: $cardinality:ident $matcher:tt $(=> $role:ident)? $([$($policy:tt)*])?;) => {
        /// Returns this list's represented elements and separators in source order.
        pub fn parts(
            &self,
        ) -> impl Iterator<
            Item = KotlinSyntaxResult<KotlinSyntaxListPart<
                'source,
                kotlin_list_item_type_optional_role!('source; $matcher; $($role)?),
            >>,
        > + '_ {
            list_parts::<kotlin_list_item_type_optional_role!('source; $matcher; $($role)?)>(
                self.syntax,
                kotlin_list_is_separated!($([$($policy)*])?),
            )
        }
    };
    ($class:ident; $($fields:tt)*) => {};
}

macro_rules! define_kotlin_cst_from_schema {
    (
        tokens { $($token:ident,)* }
        categories { $($family:ident => $bogus:ident { $($member:ident,)* })* }
        nodes { $($kind:ident => $wrapper:ident [$module:ident $class:ident] { $($fields:tt)* })* }
    ) => {
        macro_rules! ordinary_list_item {
            () => {
                const IS_FAMILY: bool = false;
            };
        }
        macro_rules! family_list_item {
            () => {
                const IS_FAMILY: bool = true;
            };
        }

        jolt_syntax::define_typed_cst_projection! {
            syntax_node: KotlinSyntaxNode,
            syntax_token: KotlinSyntaxToken,
            syntax_kind: KotlinSyntaxKind,
            fixed_syntax: KotlinFixedSyntax,
            role_element: KotlinRoleElement,
            node_trait: KotlinNode,
            typed_node_trait: KotlinTypedNode,
            family_trait: KotlinFamily,
            list_item_trait: KotlinListItem,
            syntax_view_trait: KotlinSyntaxView,
            any_node: { AnyKotlinNode },
            node_list_item_marker: ordinary_list_item,
            family_list_item_marker: family_list_item,
            nodes {
                $($wrapper => $kind [$class],)*
                $($bogus => $bogus [malformed],)*
            }
            enums {
                $($family = $($member)|+|$bogus;)*
            }
        }
    };
}

kotlin_syntax_schema!(define_kotlin_cst_from_schema);

macro_rules! define_kotlin_accessors_from_schema {
    (
        tokens { $($token:ident,)* }
        categories { $($family:ident => $bogus:ident { $($member:ident,)* })* }
        nodes {
            $(
                $kind:ident => $wrapper:ident [$module:ident $class:ident] {
                    $(
                        $field:ident: $cardinality:ident $matcher:tt
                        $(=> $role:ident)?
                        $([$($policy:tt)*])?;
                    )*
                }
            )*
        }
    ) => {
        $($( $(define_kotlin_role!($role);)? )*)*
        $(
            impl<'source> $wrapper<'source> {
                $(kotlin_field_accessor!($module $field $cardinality $matcher $(=> $role)?);)*
                kotlin_variable_slot_view!(
                    $class;
                    $($field: $cardinality $matcher $(=> $role)? $([$($policy)*])?;)*
                );
            }
        )*
    };
}

kotlin_syntax_schema!(define_kotlin_accessors_from_schema);

fn invalid_role_projection(node: KotlinSyntaxKind) -> KotlinSyntaxInvariantError {
    KotlinSyntaxInvariantError { node, slot: 0 }
}

macro_rules! define_kotlin_role_projection {
    (
        $role:ident => $value:ident {
            families { $($family_variant:ident => $family:ident),* $(,)? }
            nodes { $($node_variant:ident => $node:ident),* $(,)? }
        }
    ) => {
        #[derive(Clone, Copy, Debug, Eq, PartialEq)]
        pub enum $value<'source> {
            $($family_variant($family<'source>),)*
            $($node_variant($node<'source>),)*
        }

        impl<'source> $role<'source> {
            #[allow(clippy::missing_errors_doc)]
            pub fn classify(self) -> KotlinSyntaxResult<$value<'source>> {
                $(if let Some(value) = self.cast_family::<$family<'source>>() {
                    return Ok($value::$family_variant(value));
                })*
                $(if let Some(value) = self.cast_node::<$node<'source>>() {
                    return Ok($value::$node_variant(value));
                })*
                Err(invalid_role_projection(self.element.kind()))
            }
        }
    };
}

define_kotlin_role_projection! {
    StatementContentValue => StatementContentSyntax {
        families {
            Statement => StatementSyntax,
            Expression => Expression,
            Declaration => Declaration,
        }
        nodes {}
    }
}

define_kotlin_role_projection! {
    IfThenBranchValue => IfThenBranchSyntax {
        families { Expression => Expression }
        nodes { Block => Block, EmptyStatement => EmptyStatement }
    }
}

define_kotlin_role_projection! {
    IfElseBranchValue => IfElseBranchSyntax {
        families { Expression => Expression }
        nodes { Block => Block, EmptyStatement => EmptyStatement }
    }
}

define_kotlin_role_projection! {
    WhenEntryBodyValue => WhenEntryBodySyntax {
        families { Expression => Expression }
        nodes { Block => Block }
    }
}

define_kotlin_role_projection! {
    WhenConditionValue => WhenConditionValueSyntax {
        families { Expression => Expression }
        nodes { TypeReference => TypeReference }
    }
}

define_kotlin_role_projection! {
    ForVariableBindingValue => ForVariableSyntax {
        families {}
        nodes { Name => Name, Destructuring => DestructuringDeclaration }
    }
}

define_kotlin_role_projection! {
    ForBodyValue => ForBodySyntax {
        families { Expression => Expression }
        nodes { Block => Block, EmptyStatement => EmptyStatement }
    }
}

define_kotlin_role_projection! {
    WhileBodyValue => WhileBodySyntax {
        families { Expression => Expression }
        nodes { Block => Block, EmptyStatement => EmptyStatement }
    }
}

define_kotlin_role_projection! {
    DoWhileBodyValue => DoWhileBodySyntax {
        families { Expression => Expression }
        nodes { Block => Block, EmptyStatement => EmptyStatement }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BlockItemListElementSyntax<'source> {
    Item(BlockItem<'source>),
    Terminator(KotlinSyntaxToken<'source>),
}

impl<'source> BlockItemListElement<'source> {
    #[allow(clippy::missing_errors_doc)]
    pub fn classify(self) -> KotlinSyntaxResult<BlockItemListElementSyntax<'source>> {
        if let Some(value) = self.cast_family::<BlockItem<'source>>() {
            Ok(BlockItemListElementSyntax::Item(value))
        } else if let Some(token) = self.token() {
            Ok(BlockItemListElementSyntax::Terminator(token))
        } else {
            Err(invalid_role_projection(self.element.kind()))
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LambdaBodyItemSyntax<'source> {
    Item(BlockItem<'source>),
    Terminator(KotlinSyntaxToken<'source>),
}

impl<'source> LambdaBodyItem<'source> {
    #[allow(clippy::missing_errors_doc)]
    pub fn classify(self) -> KotlinSyntaxResult<LambdaBodyItemSyntax<'source>> {
        if let Some(value) = self.cast_family::<BlockItem<'source>>() {
            Ok(LambdaBodyItemSyntax::Item(value))
        } else if let Some(token) = self.token() {
            Ok(LambdaBodyItemSyntax::Terminator(token))
        } else {
            Err(invalid_role_projection(self.element.kind()))
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WhenEntryListElementSyntax<'source> {
    Entry(WhenEntry<'source>),
    Terminator(KotlinSyntaxToken<'source>),
}

impl<'source> WhenEntryListElement<'source> {
    #[allow(clippy::missing_errors_doc)]
    pub fn classify(self) -> KotlinSyntaxResult<WhenEntryListElementSyntax<'source>> {
        if let Some(value) = self.cast_node::<WhenEntry<'source>>() {
            Ok(WhenEntryListElementSyntax::Entry(value))
        } else if let Some(token) = self.token() {
            Ok(WhenEntryListElementSyntax::Terminator(token))
        } else {
            Err(invalid_role_projection(self.element.kind()))
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BinaryOperatorSyntax<'source> {
    Operator(KotlinSyntaxToken<'source>),
    InfixFunction(KotlinSyntaxToken<'source>),
}

impl<'source> BinaryOperatorValue<'source> {
    #[allow(clippy::missing_errors_doc)]
    pub fn classify(self) -> KotlinSyntaxResult<BinaryOperatorSyntax<'source>> {
        let token = self
            .token()
            .ok_or_else(|| invalid_role_projection(self.element.kind()))?;
        if token.kind() == KotlinSyntaxKind::Identifier {
            Ok(BinaryOperatorSyntax::InfixFunction(token))
        } else {
            Ok(BinaryOperatorSyntax::Operator(token))
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BinaryExpressionRightSyntax<'source> {
    Expression(Expression<'source>),
    TypeReference(TypeReference<'source>),
}

impl<'source> BinaryExpressionRightValue<'source> {
    #[allow(clippy::missing_errors_doc)]
    pub fn classify(self) -> KotlinSyntaxResult<BinaryExpressionRightSyntax<'source>> {
        if let Some(value) = self.cast_family::<Expression<'source>>() {
            Ok(BinaryExpressionRightSyntax::Expression(value))
        } else if let Some(value) = self.cast_node::<TypeReference<'source>>() {
            Ok(BinaryExpressionRightSyntax::TypeReference(value))
        } else {
            Err(invalid_role_projection(self.element.kind()))
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NavigationOperatorSyntax<'source> {
    Token(KotlinSyntaxToken<'source>),
    SplitSafe(SplitSafeNavigationOperator<'source>),
}

impl<'source> NavigationOperatorValue<'source> {
    #[allow(clippy::missing_errors_doc)]
    pub fn classify(self) -> KotlinSyntaxResult<NavigationOperatorSyntax<'source>> {
        if let Some(token) = self.token() {
            Ok(NavigationOperatorSyntax::Token(token))
        } else if let Some(value) = self.cast_node::<SplitSafeNavigationOperator<'source>>() {
            Ok(NavigationOperatorSyntax::SplitSafe(value))
        } else {
            Err(invalid_role_projection(self.element.kind()))
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NavigationSelectorSyntax<'source> {
    Name(KotlinSyntaxToken<'source>),
    This(ThisExpression<'source>),
    Super(SuperExpression<'source>),
    Bogus(BogusNavigationSelector<'source>),
}

impl<'source> NavigationSelectorValue<'source> {
    #[allow(clippy::missing_errors_doc)]
    pub fn classify(self) -> KotlinSyntaxResult<NavigationSelectorSyntax<'source>> {
        if let Some(token) = self.token() {
            Ok(NavigationSelectorSyntax::Name(token))
        } else if let Some(value) = self.cast_node::<ThisExpression<'source>>() {
            Ok(NavigationSelectorSyntax::This(value))
        } else if let Some(value) = self.cast_node::<SuperExpression<'source>>() {
            Ok(NavigationSelectorSyntax::Super(value))
        } else if let Some(value) = self.cast_node::<BogusNavigationSelector<'source>>() {
            Ok(NavigationSelectorSyntax::Bogus(value))
        } else {
            Err(invalid_role_projection(self.element.kind()))
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CallableReferenceReceiverSyntax<'source> {
    Expression(Expression<'source>),
    TypeReference(TypeReference<'source>),
}

impl<'source> CallableReferenceReceiverValue<'source> {
    #[allow(clippy::missing_errors_doc)]
    pub fn classify(self) -> KotlinSyntaxResult<CallableReferenceReceiverSyntax<'source>> {
        if let Some(value) = self.cast_family::<Expression<'source>>() {
            Ok(CallableReferenceReceiverSyntax::Expression(value))
        } else if let Some(value) = self.cast_node::<TypeReference<'source>>() {
            Ok(CallableReferenceReceiverSyntax::TypeReference(value))
        } else {
            Err(invalid_role_projection(self.element.kind()))
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StringTemplateContentSyntax<'source> {
    Token(KotlinSyntaxToken<'source>),
    Expression(Expression<'source>),
    LongEntry(LongStringTemplateEntry<'source>),
}

impl<'source> StringTemplateContentValue<'source> {
    #[allow(clippy::missing_errors_doc)]
    pub fn classify(self) -> KotlinSyntaxResult<StringTemplateContentSyntax<'source>> {
        if let Some(token) = self.token() {
            Ok(StringTemplateContentSyntax::Token(token))
        } else if let Some(value) = self.cast_family::<Expression<'source>>() {
            Ok(StringTemplateContentSyntax::Expression(value))
        } else if let Some(value) = self.cast_node::<LongStringTemplateEntry<'source>>() {
            Ok(StringTemplateContentSyntax::LongEntry(value))
        } else {
            Err(invalid_role_projection(self.element.kind()))
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LambdaParameterBindingSyntax<'source> {
    Name(Name<'source>),
    Destructuring(DestructuringDeclaration<'source>),
}

impl<'source> LambdaParameterBindingValue<'source> {
    #[allow(clippy::missing_errors_doc)]
    pub fn classify(self) -> KotlinSyntaxResult<LambdaParameterBindingSyntax<'source>> {
        if let Some(value) = self.cast_node::<Name<'source>>() {
            Ok(LambdaParameterBindingSyntax::Name(value))
        } else if let Some(value) = self.cast_node::<DestructuringDeclaration<'source>>() {
            Ok(LambdaParameterBindingSyntax::Destructuring(value))
        } else {
            Err(invalid_role_projection(self.element.kind()))
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValueArgumentPrefixSyntax<'source> {
    Spread(KotlinSyntaxToken<'source>),
    Annotation(Annotation<'source>),
}

impl<'source> ValueArgumentPrefixValue<'source> {
    #[allow(clippy::missing_errors_doc)]
    pub fn classify(self) -> KotlinSyntaxResult<ValueArgumentPrefixSyntax<'source>> {
        if let Some(token) = self.token() {
            Ok(ValueArgumentPrefixSyntax::Spread(token))
        } else if let Some(value) = self.cast_node::<Annotation<'source>>() {
            Ok(ValueArgumentPrefixSyntax::Annotation(value))
        } else {
            Err(invalid_role_projection(self.element.kind()))
        }
    }
}

impl Block<'_> {
    /// Returns whether the represented block interior contains only trivia.
    #[must_use]
    pub fn inner_is_whitespace(&self) -> bool {
        matches!(self.open_brace(), Ok(KotlinSyntaxField::Present(_)))
            && matches!(self.close_brace(), Ok(KotlinSyntaxField::Present(_)))
            && matches!(
                self.items(),
                Ok(KotlinSyntaxField::Present(items)) if items.first_token().is_none()
            )
    }
}

pub(crate) fn cast_kotlin_file(syntax: KotlinSyntaxNode<'_>) -> Option<KotlinFile<'_>> {
    <KotlinFile<'_> as KotlinNode<'_>>::cast(syntax)
}
