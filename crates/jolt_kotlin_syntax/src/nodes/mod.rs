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

macro_rules! define_kotlin_cst_from_schema {
    (
        tokens { $($token:ident,)* }
        categories { $($family:ident => $bogus:ident { $($member:ident,)* })* }
        nodes { $($kind:ident => $wrapper:ident [$module:ident $class:ident] { $($fields:tt)* })* }
    ) => {
        fn kotlin_kind_is_category_bogus(kind: KotlinSyntaxKind) -> bool {
            matches!(kind, $(KotlinSyntaxKind::$bogus)|*)
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
    ($($schema:tt)*) => {
        jolt_syntax::define_typed_cst_accessors! {
            names {
                syntax_token: KotlinSyntaxToken,
                syntax_field: KotlinSyntaxField,
                role_element: KotlinRoleElement,
                list_part: KotlinSyntaxListPart,
                typed_node: KotlinTypedNode,
                family: KotlinFamily,
                list_item: KotlinListItem,
                generic_vis: pub,
            }
            schema { $($schema)* }
        }
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
        matches!(self.open_brace(), KotlinSyntaxField::Present(_))
            && matches!(self.close_brace(), KotlinSyntaxField::Present(_))
            && matches!(
                self.items(),
                KotlinSyntaxField::Present(items) if items.first_token().is_none()
            )
    }
}

pub(crate) fn cast_kotlin_file(syntax: KotlinSyntaxNode<'_>) -> Option<KotlinFile<'_>> {
    <KotlinFile<'_> as KotlinNode<'_>>::cast(syntax)
}
