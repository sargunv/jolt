use std::fmt;

pub use jolt_syntax::{
    Comment as JavaComment, CommentKind as JavaCommentKind, Comments as JavaComments,
};
use jolt_syntax::{SyntaxElement, SyntaxNode, SyntaxSlot, SyntaxToken, SyntaxVerbatimCore};
use jolt_text::TextRange;

use crate::{JavaSyntaxKind, language::JavaLanguage};

pub type JavaSyntaxNode<'source> = SyntaxNode<'source, JavaLanguage>;
pub type JavaSyntaxToken<'source> = SyntaxToken<'source, JavaLanguage>;
pub type JavaSyntaxVerbatimCore<'source> = SyntaxVerbatimCore<'source, JavaLanguage>;

jolt_syntax::define_typed_cst_fields! {
    language: JavaLanguage,
    syntax_kind: JavaSyntaxKind,
    syntax_node: JavaSyntaxNode,
    invariant_error: JavaSyntaxInvariantError,
    syntax_result: JavaSyntaxResult,
    syntax_field: JavaSyntaxField,
    malformed_syntax: JavaMalformedSyntax,
    missing_syntax: JavaMissingSyntax,
    fixed_syntax: JavaFixedSyntax,
}

/// A Java operator, which may span multiple syntax tokens in ambiguous `>` forms.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct JavaOperator<'source> {
    kind: JavaOperatorKind,
    components: [Option<JavaSyntaxField<'source, JavaSyntaxToken<'source>>>; 4],
    len: usize,
}

impl<'source> JavaOperator<'source> {
    pub(crate) fn single(kind: JavaOperatorKind, token: JavaSyntaxToken<'source>) -> Self {
        Self {
            kind,
            components: [Some(JavaSyntaxField::Present(token)), None, None, None],
            len: 1,
        }
    }

    pub(crate) fn composite(
        kind: JavaOperatorKind,
        components: [Option<JavaSyntaxField<'source, JavaSyntaxToken<'source>>>; 4],
        len: usize,
    ) -> Self {
        Self {
            kind,
            components,
            len,
        }
    }

    #[must_use]
    pub const fn kind(&self) -> JavaOperatorKind {
        self.kind
    }

    #[must_use]
    pub fn text(&self) -> &'static str {
        self.kind.text()
    }

    #[must_use]
    pub fn as_single_token(&self) -> Option<&JavaSyntaxToken<'source>> {
        if self.len == 1 {
            match self.components[0].as_ref() {
                Some(JavaSyntaxField::Present(token)) => Some(token),
                Some(JavaSyntaxField::Missing(_) | JavaSyntaxField::Malformed(_)) | None => None,
            }
        } else {
            None
        }
    }

    pub fn components(
        &self,
    ) -> impl Iterator<Item = JavaSyntaxField<'source, JavaSyntaxToken<'source>>> + '_ {
        self.components.iter().take(self.len).flatten().copied()
    }
}

/// Logical Java operator kinds used to reconstruct composite operator text.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum JavaOperatorKind {
    Assign,
    PlusEq,
    MinusEq,
    StarEq,
    SlashEq,
    AmpEq,
    BarEq,
    CaretEq,
    PercentEq,
    LShiftEq,
    RShiftEq,
    UnsignedRShiftEq,
    Instanceof,
    OrOr,
    AndAnd,
    Bar,
    Caret,
    Amp,
    EqEq,
    BangEq,
    Lt,
    Gt,
    LtEq,
    GtEq,
    LShift,
    RShift,
    UnsignedRShift,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
}

impl JavaOperatorKind {
    #[must_use]
    const fn text(self) -> &'static str {
        match self {
            Self::Assign => "=",
            Self::PlusEq => "+=",
            Self::MinusEq => "-=",
            Self::StarEq => "*=",
            Self::SlashEq => "/=",
            Self::AmpEq => "&=",
            Self::BarEq => "|=",
            Self::CaretEq => "^=",
            Self::PercentEq => "%=",
            Self::LShiftEq => "<<=",
            Self::RShiftEq => ">>=",
            Self::UnsignedRShiftEq => ">>>=",
            Self::Instanceof => "instanceof",
            Self::OrOr => "||",
            Self::AndAnd => "&&",
            Self::Bar => "|",
            Self::Caret => "^",
            Self::Amp => "&",
            Self::EqEq => "==",
            Self::BangEq => "!=",
            Self::Lt => "<",
            Self::Gt => ">",
            Self::LtEq => "<=",
            Self::GtEq => ">=",
            Self::LShift => "<<",
            Self::RShift => ">>",
            Self::UnsignedRShift => ">>>",
            Self::Plus => "+",
            Self::Minus => "-",
            Self::Star => "*",
            Self::Slash => "/",
            Self::Percent => "%",
        }
    }
}

pub(crate) struct JavaOperatorPattern {
    pub(crate) kind: JavaOperatorKind,
    pub(crate) tokens: &'static [JavaSyntaxKind],
}

pub(crate) const COMPOSITE_ASSIGNMENT_OPERATORS: &[JavaOperatorPattern] = &[
    JavaOperatorPattern {
        kind: JavaOperatorKind::UnsignedRShiftEq,
        tokens: &[
            JavaSyntaxKind::Gt,
            JavaSyntaxKind::Gt,
            JavaSyntaxKind::Gt,
            JavaSyntaxKind::Assign,
        ],
    },
    JavaOperatorPattern {
        kind: JavaOperatorKind::RShiftEq,
        tokens: &[
            JavaSyntaxKind::Gt,
            JavaSyntaxKind::Gt,
            JavaSyntaxKind::Assign,
        ],
    },
];

pub(crate) const COMPOSITE_BINARY_OPERATORS: &[JavaOperatorPattern] = &[
    JavaOperatorPattern {
        kind: JavaOperatorKind::GtEq,
        tokens: &[JavaSyntaxKind::Gt, JavaSyntaxKind::Assign],
    },
    JavaOperatorPattern {
        kind: JavaOperatorKind::UnsignedRShift,
        tokens: &[JavaSyntaxKind::Gt, JavaSyntaxKind::Gt, JavaSyntaxKind::Gt],
    },
    JavaOperatorPattern {
        kind: JavaOperatorKind::RShift,
        tokens: &[JavaSyntaxKind::Gt, JavaSyntaxKind::Gt],
    },
];

pub(crate) fn assignment_operator_kind(kind: JavaSyntaxKind) -> Option<JavaOperatorKind> {
    Some(match kind {
        JavaSyntaxKind::Assign => JavaOperatorKind::Assign,
        JavaSyntaxKind::PlusEq => JavaOperatorKind::PlusEq,
        JavaSyntaxKind::MinusEq => JavaOperatorKind::MinusEq,
        JavaSyntaxKind::StarEq => JavaOperatorKind::StarEq,
        JavaSyntaxKind::SlashEq => JavaOperatorKind::SlashEq,
        JavaSyntaxKind::AmpEq => JavaOperatorKind::AmpEq,
        JavaSyntaxKind::BarEq => JavaOperatorKind::BarEq,
        JavaSyntaxKind::CaretEq => JavaOperatorKind::CaretEq,
        JavaSyntaxKind::PercentEq => JavaOperatorKind::PercentEq,
        JavaSyntaxKind::LShiftEq => JavaOperatorKind::LShiftEq,
        _ => return None,
    })
}

pub(crate) fn binary_operator_kind(kind: JavaSyntaxKind) -> Option<JavaOperatorKind> {
    Some(match kind {
        JavaSyntaxKind::OrOr => JavaOperatorKind::OrOr,
        JavaSyntaxKind::AndAnd => JavaOperatorKind::AndAnd,
        JavaSyntaxKind::Bar => JavaOperatorKind::Bar,
        JavaSyntaxKind::Caret => JavaOperatorKind::Caret,
        JavaSyntaxKind::Amp => JavaOperatorKind::Amp,
        JavaSyntaxKind::EqEq => JavaOperatorKind::EqEq,
        JavaSyntaxKind::BangEq => JavaOperatorKind::BangEq,
        JavaSyntaxKind::Lt => JavaOperatorKind::Lt,
        JavaSyntaxKind::Gt => JavaOperatorKind::Gt,
        JavaSyntaxKind::LtEq => JavaOperatorKind::LtEq,
        JavaSyntaxKind::LShift => JavaOperatorKind::LShift,
        JavaSyntaxKind::Plus => JavaOperatorKind::Plus,
        JavaSyntaxKind::Minus => JavaOperatorKind::Minus,
        JavaSyntaxKind::Star => JavaOperatorKind::Star,
        JavaSyntaxKind::Slash => JavaOperatorKind::Slash,
        JavaSyntaxKind::Percent => JavaOperatorKind::Percent,
        JavaSyntaxKind::InstanceofKw => JavaOperatorKind::Instanceof,
        _ => return None,
    })
}

#[must_use]
pub const fn binary_operator_precedence(kind: JavaOperatorKind) -> Option<u8> {
    Some(match kind {
        JavaOperatorKind::OrOr => 1,
        JavaOperatorKind::AndAnd => 2,
        JavaOperatorKind::Bar => 3,
        JavaOperatorKind::Caret => 4,
        JavaOperatorKind::Amp => 5,
        JavaOperatorKind::EqEq | JavaOperatorKind::BangEq => 6,
        JavaOperatorKind::Lt
        | JavaOperatorKind::Gt
        | JavaOperatorKind::LtEq
        | JavaOperatorKind::GtEq
        | JavaOperatorKind::Instanceof => 7,
        JavaOperatorKind::LShift | JavaOperatorKind::RShift | JavaOperatorKind::UnsignedRShift => 8,
        JavaOperatorKind::Plus | JavaOperatorKind::Minus => 9,
        JavaOperatorKind::Star | JavaOperatorKind::Slash | JavaOperatorKind::Percent => 10,
        JavaOperatorKind::Assign
        | JavaOperatorKind::PlusEq
        | JavaOperatorKind::MinusEq
        | JavaOperatorKind::StarEq
        | JavaOperatorKind::SlashEq
        | JavaOperatorKind::AmpEq
        | JavaOperatorKind::BarEq
        | JavaOperatorKind::CaretEq
        | JavaOperatorKind::PercentEq
        | JavaOperatorKind::LShiftEq
        | JavaOperatorKind::RShiftEq
        | JavaOperatorKind::UnsignedRShiftEq => return None,
    })
}

#[must_use]
pub const fn is_shift_operator(kind: JavaOperatorKind) -> bool {
    matches!(
        kind,
        JavaOperatorKind::LShift | JavaOperatorKind::RShift | JavaOperatorKind::UnsignedRShift
    )
}

#[must_use]
pub const fn is_bitwise_or_shift_operator(kind: JavaOperatorKind) -> bool {
    matches!(
        kind,
        JavaOperatorKind::Bar
            | JavaOperatorKind::Caret
            | JavaOperatorKind::Amp
            | JavaOperatorKind::LShift
            | JavaOperatorKind::RShift
            | JavaOperatorKind::UnsignedRShift
    )
}

#[must_use]
pub const fn is_multiplicative_operator(kind: JavaOperatorKind) -> bool {
    matches!(
        kind,
        JavaOperatorKind::Star | JavaOperatorKind::Slash | JavaOperatorKind::Percent
    )
}

jolt_syntax::define_typed_cst_access! {
    language: JavaLanguage,
    syntax_kind: JavaSyntaxKind,
    syntax_node: JavaSyntaxNode,
    syntax_token: JavaSyntaxToken,
    invariant_error: JavaSyntaxInvariantError,
    syntax_result: JavaSyntaxResult,
    syntax_field: JavaSyntaxField,
    malformed_syntax: JavaMalformedSyntax,
    missing_syntax: JavaMissingSyntax,
    fixed_syntax: JavaFixedSyntax,
    syntax_view: JavaSyntaxView,
    typed_node: JavaTypedNode,
    node: JavaNode,
    family: JavaFamily,
    role_element: JavaRoleElement,
    list_item: JavaListItem,
    list_part: JavaSyntaxListPart,
    category_bogus: java_kind_is_category_bogus,
}

macro_rules! define_java_cst_from_schema {
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

        fn java_kind_is_category_bogus(kind: JavaSyntaxKind) -> bool {
            matches!(kind, $(JavaSyntaxKind::$bogus)|*)
        }

        jolt_syntax::define_typed_cst_projection! {
            syntax_node: JavaSyntaxNode,
            syntax_token: JavaSyntaxToken,
            syntax_kind: JavaSyntaxKind,
            fixed_syntax: JavaFixedSyntax,
            role_element: JavaRoleElement,
            node_trait: JavaNode,
            typed_node_trait: JavaTypedNode,
            family_trait: JavaFamily,
            list_item_trait: JavaListItem,
            syntax_view_trait: JavaSyntaxView,
            any_node: {},
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

java_syntax_schema!(define_java_cst_from_schema);

macro_rules! define_java_accessors_from_schema {
    ($($schema:tt)*) => {
        jolt_syntax::define_typed_cst_accessors! {
            names {
                syntax_token: JavaSyntaxToken,
                syntax_result: JavaSyntaxResult,
                syntax_field: JavaSyntaxField,
                role_element: JavaRoleElement,
                list_part: JavaSyntaxListPart,
                typed_node: JavaTypedNode,
                family: JavaFamily,
                list_item: JavaListItem,
                generic_vis: pub(crate),
            }
            schema { $($schema)* }
        }
    };
}

java_syntax_schema!(define_java_accessors_from_schema);

#[derive(Clone, Copy, Debug)]
pub enum ModifierItem<'source> {
    Annotation(Annotation<'source>),
    Bogus(BogusModifier<'source>),
    Sealed(JavaSyntaxToken<'source>),
    Token(JavaSyntaxToken<'source>),
    NonSealed(NonSealedModifier<'source>),
}

impl<'source> ModifierElement<'source> {
    /// Classifies one modifier-list element by its declared syntax role.
    ///
    /// # Errors
    ///
    /// Returns an invariant error if the represented element is not a declared modifier.
    pub fn classify(self) -> Result<ModifierItem<'source>, JavaSyntaxInvariantError> {
        if let Some(annotation) = self.cast_node::<Annotation<'source>>() {
            Ok(ModifierItem::Annotation(annotation))
        } else if let Some(bogus) = self.cast_node::<BogusModifier<'source>>() {
            Ok(ModifierItem::Bogus(bogus))
        } else if let Some(non_sealed) = self.cast_node::<NonSealedModifier<'source>>() {
            Ok(ModifierItem::NonSealed(non_sealed))
        } else if let Some(token) = self.token() {
            if token.kind() == JavaSyntaxKind::Identifier
                && crate::lexer::lexical_text_is(token.text(), "sealed")
            {
                Ok(ModifierItem::Sealed(token))
            } else {
                Ok(ModifierItem::Token(token))
            }
        } else {
            Err(JavaSyntaxInvariantError {
                node: JavaSyntaxKind::ModifierList,
                slot: 0,
            })
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum PartitionedModifierItem<'source> {
    DeclarationAnnotation(Annotation<'source>),
    TypeUseAnnotation(Annotation<'source>),
    Token(JavaSyntaxToken<'source>),
    Sealed(JavaSyntaxToken<'source>),
    NonSealed(NonSealedModifier<'source>),
    Bogus(BogusModifier<'source>),
    Missing(JavaMissingSyntax<'source>),
    Malformed(JavaMalformedSyntax<'source>),
}

impl<'source> ModifierList<'source> {
    pub fn partitioned_items(
        &self,
    ) -> impl Iterator<Item = Result<PartitionedModifierItem<'source>, JavaSyntaxInvariantError>> + '_
    {
        let mut saw_modifier = false;
        self.parts().map(move |part| match part {
            Ok(JavaSyntaxListPart::Item(item)) => match item.classify() {
                Ok(ModifierItem::Annotation(annotation)) if saw_modifier => {
                    Ok(PartitionedModifierItem::TypeUseAnnotation(annotation))
                }
                Ok(ModifierItem::Annotation(annotation)) => {
                    Ok(PartitionedModifierItem::DeclarationAnnotation(annotation))
                }
                Ok(ModifierItem::Token(token)) => {
                    saw_modifier = true;
                    Ok(PartitionedModifierItem::Token(token))
                }
                Ok(ModifierItem::Sealed(token)) => {
                    saw_modifier = true;
                    Ok(PartitionedModifierItem::Sealed(token))
                }
                Ok(ModifierItem::NonSealed(non_sealed)) => {
                    saw_modifier = true;
                    Ok(PartitionedModifierItem::NonSealed(non_sealed))
                }
                Ok(ModifierItem::Bogus(bogus)) => {
                    saw_modifier = true;
                    Ok(PartitionedModifierItem::Bogus(bogus))
                }
                Err(error) => Err(error),
            },
            Ok(JavaSyntaxListPart::Malformed(malformed)) => {
                Ok(PartitionedModifierItem::Malformed(malformed))
            }
            Ok(JavaSyntaxListPart::Missing(missing)) => {
                Ok(PartitionedModifierItem::Missing(missing))
            }
            Ok(JavaSyntaxListPart::Separator(_)) => Err(JavaSyntaxInvariantError {
                node: JavaSyntaxKind::ModifierList,
                slot: 0,
            }),
            Err(error) => Err(error),
        })
    }
}

impl<'source> ParameterModifierList<'source> {
    /// Partitions parameter modifiers by their grammar position. Annotations
    /// before `final` are declaration annotations; annotations after it apply
    /// to the type.
    pub fn partitioned_items(
        &self,
    ) -> impl Iterator<Item = Result<PartitionedModifierItem<'source>, JavaSyntaxInvariantError>> + '_
    {
        let mut saw_modifier = false;
        self.parts().map(move |part| match part {
            Ok(JavaSyntaxListPart::Item(item)) => {
                if let Some(annotation) = item.cast_node::<Annotation<'source>>() {
                    if saw_modifier {
                        Ok(PartitionedModifierItem::TypeUseAnnotation(annotation))
                    } else {
                        Ok(PartitionedModifierItem::DeclarationAnnotation(annotation))
                    }
                } else if let Some(bogus) = item.cast_node::<BogusModifier<'source>>() {
                    saw_modifier = true;
                    Ok(PartitionedModifierItem::Bogus(bogus))
                } else if let Some(token) = item.token() {
                    saw_modifier = true;
                    Ok(PartitionedModifierItem::Token(token))
                } else {
                    Err(JavaSyntaxInvariantError {
                        node: JavaSyntaxKind::ParameterModifierList,
                        slot: 0,
                    })
                }
            }
            Ok(JavaSyntaxListPart::Malformed(malformed)) => {
                Ok(PartitionedModifierItem::Malformed(malformed))
            }
            Ok(JavaSyntaxListPart::Missing(missing)) => {
                Ok(PartitionedModifierItem::Missing(missing))
            }
            Ok(JavaSyntaxListPart::Separator(_)) => Err(JavaSyntaxInvariantError {
                node: JavaSyntaxKind::ParameterModifierList,
                slot: 0,
            }),
            Err(error) => Err(error),
        })
    }
}

#[cfg(test)]
impl<'source> CompilationUnit<'source> {
    pub(crate) fn syntax(&self) -> &JavaSyntaxNode<'source> {
        &self.syntax
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SwitchLabelCaseItem<'source> {
    Constant(CaseConstant<'source>),
    Pattern(CasePattern<'source>),
    Default(JavaSyntaxToken<'source>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SwitchLabelCaseEntry<'source> {
    pub item: SwitchLabelCaseItem<'source>,
    pub comma: Option<JavaSyntaxToken<'source>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SwitchBlockStatementGroupLabel<'source> {
    pub label: SwitchLabel<'source>,
    pub colon: Option<JavaSyntaxToken<'source>>,
}

#[derive(Clone, Copy, Debug)]
pub enum AnnotationElementValueContentItem<'source> {
    Expression(Expression<'source>),
    Annotation(Annotation<'source>),
    ArrayInitializer(AnnotationArrayInitializer<'source>),
}

impl<'source> AnnotationElementValueContent<'source> {
    pub fn classify(self) -> Option<AnnotationElementValueContentItem<'source>> {
        self.cast_family::<Expression<'source>>()
            .map(AnnotationElementValueContentItem::Expression)
            .or_else(|| {
                self.cast_node::<Annotation<'source>>()
                    .map(AnnotationElementValueContentItem::Annotation)
            })
            .or_else(|| {
                self.cast_node::<AnnotationArrayInitializer<'source>>()
                    .map(AnnotationElementValueContentItem::ArrayInitializer)
            })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnnotationArrayInitializerEntry<'source> {
    pub value: AnnotationElementValue<'source>,
    pub comma: Option<JavaSyntaxToken<'source>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ImportKind<'source> {
    SingleType(NameSyntax<'source>),
    TypeOnDemand(NameSyntax<'source>),
    SingleStatic(NameSyntax<'source>),
    StaticOnDemand(NameSyntax<'source>),
    SingleModule(NameSyntax<'source>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ModuleDirectiveRole<'source> {
    Requires {
        module: NameSyntax<'source>,
        is_static: bool,
        is_transitive: bool,
    },
    Exports {
        package: NameSyntax<'source>,
    },
    Opens {
        package: NameSyntax<'source>,
    },
    Uses {
        service: NameSyntax<'source>,
    },
    Provides {
        service: NameSyntax<'source>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModuleNameListEntry<'source> {
    pub name: NameSyntax<'source>,
    pub comma: Option<JavaSyntaxToken<'source>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StatementBody<'source> {
    Block(Block<'source>),
    Empty(EmptyStatement<'source>),
    Unbraced(Statement<'source>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WildcardBound<'source> {
    Extends(Type<'source>),
    Super(Type<'source>),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExpressionParentRole {
    ParenthesizedExpression,
    AssignmentLeft,
    AssignmentRight,
    ConditionalCondition,
    ConditionalTrueExpression,
    ConditionalFalseExpression,
    BinaryLeft,
    BinaryRight,
    UnaryOperand,
    PostfixOperand,
    CastOperand,
    InstanceofOperand,
    FieldAccessReceiver,
    MethodInvocationQualifier,
    MethodInvocationCallee,
    MethodReferenceReceiver,
    ArrayAccessArray,
    ArrayAccessIndex,
    ObjectCreationQualifier,
    ArrayCreationDimension,
    ClassLiteralTarget,
    LambdaBody,
    SwitchExpressionSelector,
    Argument,
    AnnotationElementValue,
    VariableInitializer,
    ExpressionStatement,
    IfCondition,
    WhileCondition,
    DoCondition,
    BasicForCondition,
    EnhancedForIterable,
    SynchronizedExpression,
    AssertCondition,
    AssertDetail,
    ReturnValue,
    ThrowValue,
    YieldValue,
    SwitchStatementSelector,
}

pub(crate) fn cast_compilation_unit(syntax: JavaSyntaxNode<'_>) -> Option<CompilationUnit<'_>> {
    <CompilationUnit<'_> as JavaNode<'_>>::cast(syntax)
}

fn operator_from_element(
    element: JavaRoleElement<'_>,
    single: fn(JavaSyntaxKind) -> Option<JavaOperatorKind>,
) -> JavaSyntaxResult<JavaOperator<'_>> {
    match element {
        JavaRoleElement::Token(token) => single(token.kind())
            .map(|kind| JavaOperator::single(kind, token))
            .ok_or(JavaSyntaxInvariantError {
                node: token.kind(),
                slot: 0,
            }),
        JavaRoleElement::Node(node) => {
            let mut components = [None; 4];
            let (kind, len) = match node.kind() {
                JavaSyntaxKind::GreaterThanOrEqualOperator => {
                    let operator =
                        GreaterThanOrEqualOperator::cast(node).ok_or(JavaSyntaxInvariantError {
                            node: node.kind(),
                            slot: 0,
                        })?;
                    components[0] = Some(operator.greater_than()?);
                    components[1] = Some(operator.assign()?);
                    (JavaOperatorKind::GtEq, 2)
                }
                JavaSyntaxKind::RightShiftOperator => {
                    let operator =
                        RightShiftOperator::cast(node).ok_or(JavaSyntaxInvariantError {
                            node: node.kind(),
                            slot: 0,
                        })?;
                    components[0] = Some(operator.first_greater_than()?);
                    components[1] = Some(operator.second_greater_than()?);
                    (JavaOperatorKind::RShift, 2)
                }
                JavaSyntaxKind::UnsignedRightShiftOperator => {
                    let operator =
                        UnsignedRightShiftOperator::cast(node).ok_or(JavaSyntaxInvariantError {
                            node: node.kind(),
                            slot: 0,
                        })?;
                    components[0] = Some(operator.first_greater_than()?);
                    components[1] = Some(operator.second_greater_than()?);
                    components[2] = Some(operator.third_greater_than()?);
                    (JavaOperatorKind::UnsignedRShift, 3)
                }
                JavaSyntaxKind::RightShiftAssignmentOperator => {
                    let operator = RightShiftAssignmentOperator::cast(node).ok_or(
                        JavaSyntaxInvariantError {
                            node: node.kind(),
                            slot: 0,
                        },
                    )?;
                    components[0] = Some(operator.first_greater_than()?);
                    components[1] = Some(operator.second_greater_than()?);
                    components[2] = Some(operator.assign()?);
                    (JavaOperatorKind::RShiftEq, 3)
                }
                JavaSyntaxKind::UnsignedRightShiftAssignmentOperator => {
                    let operator = UnsignedRightShiftAssignmentOperator::cast(node).ok_or(
                        JavaSyntaxInvariantError {
                            node: node.kind(),
                            slot: 0,
                        },
                    )?;
                    components[0] = Some(operator.first_greater_than()?);
                    components[1] = Some(operator.second_greater_than()?);
                    components[2] = Some(operator.third_greater_than()?);
                    components[3] = Some(operator.assign()?);
                    (JavaOperatorKind::UnsignedRShiftEq, 4)
                }
                _ => {
                    return Err(JavaSyntaxInvariantError {
                        node: node.kind(),
                        slot: 0,
                    });
                }
            };
            Ok(JavaOperator::composite(kind, components, len))
        }
    }
}

impl<'source> AssignmentOperatorRole<'source> {
    /// Returns the logical operator represented by this declared role.
    ///
    /// # Errors
    ///
    /// Returns an invariant error if the role has an invalid token or node shape.
    pub fn as_operator(&self) -> JavaSyntaxResult<JavaOperator<'source>> {
        operator_from_element(self.element, assignment_operator_kind)
    }
}

impl<'source> BinaryOperatorRole<'source> {
    /// Returns the logical operator represented by this declared role.
    ///
    /// # Errors
    ///
    /// Returns an invariant error if the role has an invalid token or node shape.
    pub fn as_operator(&self) -> JavaSyntaxResult<JavaOperator<'source>> {
        operator_from_element(self.element, binary_operator_kind)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InvocationNameSyntax<'source> {
    NameExpression(NameExpression<'source>),
    Identifier(JavaSyntaxToken<'source>),
}

fn invocation_name_syntax(
    element: JavaRoleElement<'_>,
) -> JavaSyntaxResult<InvocationNameSyntax<'_>> {
    match element {
        JavaRoleElement::Token(token) if token.kind() == JavaSyntaxKind::Identifier => {
            Ok(InvocationNameSyntax::Identifier(token))
        }
        JavaRoleElement::Node(node) => NameExpression::cast(node)
            .map(InvocationNameSyntax::NameExpression)
            .ok_or(JavaSyntaxInvariantError {
                node: node.kind(),
                slot: 0,
            }),
        JavaRoleElement::Token(token) => Err(JavaSyntaxInvariantError {
            node: token.kind(),
            slot: 0,
        }),
    }
}

macro_rules! impl_invocation_name {
    ($($role:ident),+ $(,)?) => {$(
        impl<'source> $role<'source> {
            #[allow(clippy::missing_errors_doc)]
            pub fn classify(self) -> JavaSyntaxResult<InvocationNameSyntax<'source>> {
                invocation_name_syntax(self.element)
            }
        }
    )+};
}

impl_invocation_name!(QualifiedInvocationName, UnqualifiedInvocationName);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LambdaModifierSyntax<'source> {
    Annotation(Annotation<'source>),
    Final(JavaSyntaxToken<'source>),
    Var(JavaSyntaxToken<'source>),
}

impl<'source> LambdaModifier<'source> {
    #[allow(clippy::missing_errors_doc)]
    pub fn classify(self) -> JavaSyntaxResult<LambdaModifierSyntax<'source>> {
        match self.element {
            JavaRoleElement::Node(node) => Annotation::cast(node)
                .map(LambdaModifierSyntax::Annotation)
                .ok_or(JavaSyntaxInvariantError {
                    node: node.kind(),
                    slot: 0,
                }),
            JavaRoleElement::Token(token) if token.kind() == JavaSyntaxKind::FinalKw => {
                Ok(LambdaModifierSyntax::Final(token))
            }
            JavaRoleElement::Token(token)
                if token.kind() == JavaSyntaxKind::Identifier
                    && crate::lexer::lexical_text_is(token.text(), "var") =>
            {
                Ok(LambdaModifierSyntax::Var(token))
            }
            JavaRoleElement::Token(token) => Err(JavaSyntaxInvariantError {
                node: token.kind(),
                slot: 0,
            }),
        }
    }
}

macro_rules! define_family_projection {
    (
        $category:ident => $value:ident {
            special { $($special:ident => $special_value:ident),* $(,)? }
            families { $($family:ident => $family_value:ident),+ $(,)? }
        }
    ) => {
        #[derive(Clone, Copy, Debug, Eq, PartialEq)]
        pub enum $value<'source> {
            $($special_value($special<'source>),)*
            $($family_value($family<'source>),)+
        }

        impl<'source> $category<'source> {
            #[allow(clippy::missing_errors_doc)]
            pub fn classify(self) -> JavaSyntaxResult<$value<'source>> {
                match self {
                    $(Self::$special(value) => Ok($value::$special_value(value)),)*
                    value => {
                        let syntax = *value.syntax();
                        $(if let Some(value) = $family::cast(syntax) {
                            return Ok($value::$family_value(value));
                        })+
                        Err(JavaSyntaxInvariantError {
                            node: syntax.kind(),
                            slot: 0,
                        })
                    }
                }
            }
        }
    };
}

define_family_projection! {
    LambdaBodySyntax => LambdaBodyValue {
        special { Block => Block, BogusLambdaBody => Bogus }
        families { Expression => Expression }
    }
}

define_family_projection! {
    MethodReferenceReceiverSyntax => MethodReferenceReceiverValue {
        special { BogusMethodReferenceReceiver => Bogus }
        families { Expression => Expression, Type => Type }
    }
}

macro_rules! define_node_role_projection {
    ($role:ident => $value:ident { $($node:ident),+ $(,)? }) => {
        #[derive(Clone, Copy, Debug, Eq, PartialEq)]
        pub enum $value<'source> {
            $($node($node<'source>),)+
        }

        impl<'source> $role<'source> {
            #[allow(clippy::missing_errors_doc)]
            pub fn classify(self) -> JavaSyntaxResult<$value<'source>> {
                $(if let Some(value) = self.cast_node::<$node<'source>>() {
                    return Ok($value::$node(value));
                })+
                Err(JavaSyntaxInvariantError {
                    node: self.element.kind(),
                    slot: 0,
                })
            }
        }
    };
}

define_node_role_projection! {
    ForStatementForm => ForStatementFormSyntax {
        BasicForStatement,
        EnhancedForStatement,
    }
}

define_node_role_projection! {
    ForInitializerValue => ForInitializerSyntax {
        LocalVariableDeclaration,
        StatementExpressionList,
    }
}

define_node_role_projection! {
    LocalTypeDeclaration => LocalTypeDeclarationSyntax {
        ClassDeclaration,
        RecordDeclaration,
        EnumDeclaration,
        InterfaceDeclaration,
        AnnotationInterfaceDeclaration,
        BogusTypeDeclaration,
    }
}

define_node_role_projection! {
    VariableAccessExpression => VariableAccessSyntax {
        NameExpression,
        FieldAccessExpression,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SwitchRuleBodySyntax<'source> {
    Expression(Expression<'source>),
    Block(Block<'source>),
    ThrowStatement(ThrowStatement<'source>),
}

impl<'source> SwitchRuleBody<'source> {
    #[allow(clippy::missing_errors_doc)]
    pub fn classify(self) -> JavaSyntaxResult<SwitchRuleBodySyntax<'source>> {
        if let Some(value) = self.cast_family::<Expression<'source>>() {
            Ok(SwitchRuleBodySyntax::Expression(value))
        } else if let Some(value) = self.cast_node::<Block<'source>>() {
            Ok(SwitchRuleBodySyntax::Block(value))
        } else if let Some(value) = self.cast_node::<ThrowStatement<'source>>() {
            Ok(SwitchRuleBodySyntax::ThrowStatement(value))
        } else {
            Err(JavaSyntaxInvariantError {
                node: self.element.kind(),
                slot: 0,
            })
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SwitchLabelItemSyntax<'source> {
    CaseConstant(CaseConstant<'source>),
    CasePattern(CasePattern<'source>),
    BogusSwitchLabelItem(BogusSwitchLabelItem<'source>),
    Default(JavaSyntaxToken<'source>),
}

impl<'source> SwitchLabelItem<'source> {
    #[allow(clippy::missing_errors_doc)]
    pub fn classify(self) -> JavaSyntaxResult<SwitchLabelItemSyntax<'source>> {
        if let Some(value) = self.cast_node::<CaseConstant<'source>>() {
            Ok(SwitchLabelItemSyntax::CaseConstant(value))
        } else if let Some(value) = self.cast_node::<CasePattern<'source>>() {
            Ok(SwitchLabelItemSyntax::CasePattern(value))
        } else if let Some(value) = self.cast_node::<BogusSwitchLabelItem<'source>>() {
            Ok(SwitchLabelItemSyntax::BogusSwitchLabelItem(value))
        } else if let Some(token) = self.token()
            && token.kind() == JavaSyntaxKind::DefaultKw
        {
            Ok(SwitchLabelItemSyntax::Default(token))
        } else {
            Err(JavaSyntaxInvariantError {
                node: self.element.kind(),
                slot: 0,
            })
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VariableTypeSyntax<'source> {
    Type(Type<'source>),
    Var(JavaSyntaxToken<'source>),
}

fn classify_variable_type(
    element: JavaRoleElement<'_>,
) -> JavaSyntaxResult<VariableTypeSyntax<'_>> {
    if let Some(value) = element.cast_family::<Type<'_>>() {
        Ok(VariableTypeSyntax::Type(value))
    } else if let Some(token) = element.token() {
        Ok(VariableTypeSyntax::Var(token))
    } else {
        Err(JavaSyntaxInvariantError {
            node: element.kind(),
            slot: 0,
        })
    }
}

macro_rules! impl_variable_type {
    ($($role:ident),+ $(,)?) => {$(
        impl<'source> $role<'source> {
            #[allow(clippy::missing_errors_doc)]
            pub fn classify(self) -> JavaSyntaxResult<VariableTypeSyntax<'source>> {
                classify_variable_type(self.element)
            }
        }
    )+};
}

impl_variable_type!(
    LocalVariableType,
    EnhancedForVariableType,
    ResourceVariableType
);

impl<'source> CatchParameterTypes<'source> {
    #[allow(clippy::missing_errors_doc)]
    pub fn as_type(self) -> JavaSyntaxResult<Type<'source>> {
        self.cast_family::<Type<'source>>()
            .ok_or(JavaSyntaxInvariantError {
                node: self.element.kind(),
                slot: 0,
            })
    }
}

impl Expression<'_> {
    /// Returns this expression's grammar role from its parent's fixed slot.
    #[must_use]
    #[allow(clippy::too_many_lines)]
    pub fn parent_role(&self) -> Option<ExpressionParentRole> {
        let syntax = self.syntax();
        let parent = syntax.parent()?;
        let slot = syntax.index();

        macro_rules! role {
            ($module:ident, $field:ident, $role:ident) => {
                (slot == crate::shape::$module::Slot::$field as usize)
                    .then_some(ExpressionParentRole::$role)
            };
        }

        match parent.kind() {
            JavaSyntaxKind::ParenthesizedExpression => role!(
                parenthesized_expression,
                expression,
                ParenthesizedExpression
            ),
            JavaSyntaxKind::AssignmentExpression => {
                role!(assignment_expression, left, AssignmentLeft)
                    .or_else(|| role!(assignment_expression, right, AssignmentRight))
            }
            JavaSyntaxKind::ConditionalExpression => {
                role!(conditional_expression, condition, ConditionalCondition)
                    .or_else(|| {
                        role!(
                            conditional_expression,
                            then_expression,
                            ConditionalTrueExpression
                        )
                    })
                    .or_else(|| {
                        role!(
                            conditional_expression,
                            else_expression,
                            ConditionalFalseExpression
                        )
                    })
            }
            JavaSyntaxKind::BinaryExpression => role!(binary_expression, left, BinaryLeft)
                .or_else(|| role!(binary_expression, right, BinaryRight)),
            JavaSyntaxKind::UnaryExpression => role!(unary_expression, operand, UnaryOperand),
            JavaSyntaxKind::PostfixExpression => role!(postfix_expression, operand, PostfixOperand),
            JavaSyntaxKind::CastExpression => role!(cast_expression, expression, CastOperand),
            JavaSyntaxKind::InstanceofExpression => {
                role!(instanceof_expression, expression, InstanceofOperand)
            }
            JavaSyntaxKind::FieldAccessExpression => {
                role!(field_access_expression, receiver, FieldAccessReceiver)
            }
            JavaSyntaxKind::QualifiedMethodInvocation => role!(
                qualified_method_invocation,
                receiver,
                MethodInvocationQualifier
            )
            .or_else(|| role!(qualified_method_invocation, name, MethodInvocationCallee)),
            JavaSyntaxKind::UnqualifiedMethodInvocation => {
                role!(unqualified_method_invocation, name, MethodInvocationCallee)
            }
            JavaSyntaxKind::MethodReferenceExpression => role!(
                method_reference_expression,
                receiver,
                MethodReferenceReceiver
            ),
            JavaSyntaxKind::ArrayAccessExpression => {
                role!(array_access_expression, array, ArrayAccessArray)
                    .or_else(|| role!(array_access_expression, index, ArrayAccessIndex))
            }
            JavaSyntaxKind::ObjectCreationExpression => role!(
                object_creation_expression,
                qualifier,
                ObjectCreationQualifier
            ),
            JavaSyntaxKind::DimExpression => {
                role!(dim_expression, expression, ArrayCreationDimension)
            }
            JavaSyntaxKind::ClassLiteralExpression => {
                role!(class_literal_expression, target, ClassLiteralTarget)
            }
            JavaSyntaxKind::LambdaExpression => role!(lambda_expression, body, LambdaBody),
            JavaSyntaxKind::SwitchExpression => {
                role!(switch_expression, selector, SwitchExpressionSelector)
            }
            JavaSyntaxKind::ExpressionList => Some(ExpressionParentRole::Argument),
            JavaSyntaxKind::AnnotationElementValue => {
                role!(annotation_element_value, value, AnnotationElementValue)
            }
            JavaSyntaxKind::VariableInitializer => {
                role!(variable_initializer, value, VariableInitializer)
            }
            JavaSyntaxKind::ExpressionStatement => {
                role!(expression_statement, expression, ExpressionStatement)
            }
            JavaSyntaxKind::IfStatement => role!(if_statement, condition, IfCondition),
            JavaSyntaxKind::WhileStatement => role!(while_statement, condition, WhileCondition),
            JavaSyntaxKind::DoStatement => role!(do_statement, condition, DoCondition),
            JavaSyntaxKind::BasicForStatement => {
                role!(basic_for_statement, condition, BasicForCondition)
            }
            JavaSyntaxKind::EnhancedForStatement => {
                role!(enhanced_for_statement, iterable, EnhancedForIterable)
            }
            JavaSyntaxKind::SynchronizedStatement => {
                role!(synchronized_statement, expression, SynchronizedExpression)
            }
            JavaSyntaxKind::AssertStatement => role!(assert_statement, condition, AssertCondition)
                .or_else(|| role!(assert_statement, message, AssertDetail)),
            JavaSyntaxKind::ReturnStatement => role!(return_statement, expression, ReturnValue),
            JavaSyntaxKind::ThrowStatement => role!(throw_statement, expression, ThrowValue),
            JavaSyntaxKind::YieldStatement => role!(yield_statement, expression, YieldValue),
            JavaSyntaxKind::SwitchStatement => {
                role!(switch_statement, selector, SwitchStatementSelector)
            }
            _ => None,
        }
    }
}
