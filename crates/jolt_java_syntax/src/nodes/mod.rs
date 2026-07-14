use std::fmt;

pub use jolt_syntax::{
    Comment as JavaComment, CommentKind as JavaCommentKind, Comments as JavaComments,
};
use jolt_syntax::{SyntaxNode, SyntaxToken};
use jolt_text::TextRange;

use crate::{JavaSyntaxKind, language::JavaLanguage};

pub(crate) type JavaSyntaxNode<'source> = SyntaxNode<'source, JavaLanguage>;
pub type JavaSyntaxToken<'source> = SyntaxToken<'source, JavaLanguage>;

/// A Java operator, which may span multiple syntax tokens in ambiguous `>` forms.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct JavaOperator<'source> {
    kind: JavaOperatorKind,
    tokens: [Option<JavaSyntaxToken<'source>>; 4],
    len: usize,
}

impl<'source> JavaOperator<'source> {
    pub(crate) fn single(kind: JavaOperatorKind, token: JavaSyntaxToken<'source>) -> Self {
        Self {
            kind,
            tokens: [Some(token), None, None, None],
            len: 1,
        }
    }

    pub(crate) fn composite(
        kind: JavaOperatorKind,
        tokens: [Option<JavaSyntaxToken<'source>>; 4],
        len: usize,
    ) -> Self {
        Self { kind, tokens, len }
    }

    #[must_use]
    pub fn text(&self) -> &'static str {
        self.kind.text()
    }

    #[must_use]
    pub fn as_single_token(&self) -> Option<&JavaSyntaxToken<'source>> {
        if self.len == 1 {
            self.tokens[0].as_ref()
        } else {
            None
        }
    }

    pub fn tokens(&self) -> impl Iterator<Item = JavaSyntaxToken<'source>> + '_ {
        self.tokens.iter().take(self.len).flatten().copied()
    }
}

/// Logical Java operator kinds used to reconstruct composite operator text.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum JavaOperatorKind {
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

pub(crate) fn binary_operator_precedence(kind: JavaOperatorKind) -> Option<u8> {
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

mod private {
    pub(crate) trait Sealed {}
}

pub(crate) trait JavaNode<'source>: Clone + private::Sealed {
    fn cast(syntax: JavaSyntaxNode<'source>) -> Option<Self>;
}

pub(crate) trait JavaFamily<'source>: Clone {
    fn cast(syntax: JavaSyntaxNode<'source>) -> Option<Self>;
}

fn syntax_source_text<'source>(syntax: &JavaSyntaxNode<'source>) -> &'source str {
    let range = syntax.text_range();
    &syntax.source()[range.start().get()..range.end().get()]
}

macro_rules! java_cst {
    (
        nodes {
            $($node:ident => $kind:ident,)*
        }
        enums {
            $(
                $family:ident =
                    $($variant:ident)|+;
            )*
        }
    ) => {
        $(
            #[derive(Clone, Copy, Eq, PartialEq)]
            pub struct $node<'source> {
                syntax: JavaSyntaxNode<'source>,
            }

            impl<'source> $node<'source> {
                #[must_use]
                pub fn kind(&self) -> JavaSyntaxKind {
                    self.syntax.kind()
                }

                #[must_use]
                pub fn text_range(&self) -> TextRange {
                    self.syntax.text_range()
                }

                #[must_use]
                pub fn source_text(&self) -> &'source str {
                    syntax_source_text(&self.syntax)
                }

                pub fn token_iter(&self) -> impl Iterator<Item = JavaSyntaxToken<'source>> + '_ {
                    token_iter(&self.syntax)
                }

                #[must_use]
                pub fn first_token(&self) -> Option<JavaSyntaxToken<'source>> {
                    first_token(&self.syntax)
                }

                #[must_use]
                pub fn last_token(&self) -> Option<JavaSyntaxToken<'source>> {
                    last_token(&self.syntax)
                }

            }

            impl private::Sealed for $node<'_> {}

            impl<'source> JavaNode<'source> for $node<'source> {
                fn cast(syntax: JavaSyntaxNode<'source>) -> Option<Self> {
                    matches!(syntax.kind(), JavaSyntaxKind::$kind).then_some(Self { syntax })
                }
            }

            impl fmt::Debug for $node<'_> {
                fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                    self.syntax.fmt(f)
                }
            }
        )*

        #[derive(Clone, Copy, Debug, Eq, PartialEq)]
        pub(crate) enum AnyJavaNode<'source> {
            $($node($node<'source>),)*
        }

        impl<'source> AnyJavaNode<'source> {
            pub(crate) fn cast(syntax: JavaSyntaxNode<'source>) -> Option<Self> {
                match syntax.kind() {
                    $(
                        JavaSyntaxKind::$kind => {
                            <$node<'source> as JavaNode<'source>>::cast(syntax).map(Self::$node)
                        }
                    )*
                    _ => None,
                }
            }
        }

        $(
            impl<'source> From<$node<'source>> for AnyJavaNode<'source> {
                fn from(node: $node<'source>) -> Self {
                    Self::$node(node)
                }
            }
        )*

        $(
            #[derive(Clone, Copy, Debug, Eq, PartialEq)]
            pub enum $family<'source> {
                $($variant($variant<'source>),)+
            }

            impl<'source> $family<'source> {
                #[must_use]
                pub fn kind(&self) -> JavaSyntaxKind {
                    self.syntax().kind()
                }

                #[must_use]
                pub fn text_range(&self) -> TextRange {
                    self.syntax().text_range()
                }

                #[must_use]
                pub fn source_text(&self) -> &'source str {
                    syntax_source_text(self.syntax())
                }

                pub fn token_iter(&self) -> impl Iterator<Item = JavaSyntaxToken<'source>> + '_ {
                    token_iter(self.syntax())
                }

                #[must_use]
                pub fn first_token(&self) -> Option<JavaSyntaxToken<'source>> {
                    first_token(self.syntax())
                }

                #[must_use]
                pub fn last_token(&self) -> Option<JavaSyntaxToken<'source>> {
                    last_token(self.syntax())
                }

                pub(crate) fn syntax(&self) -> &JavaSyntaxNode<'source> {
                    match self {
                        $(Self::$variant(node) => &node.syntax,)+
                    }
                }
            }

            impl<'source> JavaFamily<'source> for $family<'source> {
                fn cast(syntax: JavaSyntaxNode<'source>) -> Option<Self> {
                    match syntax.kind() {
                        $(
                            JavaSyntaxKind::$variant => {
                                <$variant<'source> as JavaNode<'source>>::cast(syntax).map(Self::$variant)
                            }
                        )+
                        _ => None,
                    }
                }
            }

            $(
                impl<'source> From<$variant<'source>> for $family<'source> {
                    fn from(node: $variant<'source>) -> Self {
                        Self::$variant(node)
                    }
                }
            )+
        )*

    };
}

macro_rules! define_java_cst_from_schema {
    (
        tokens { $($token:ident,)* }
        categories { $($family:ident => $bogus:ident { $($member:ident,)* })* }
        nodes { $($kind:ident => $wrapper:ident [$module:ident $class:ident] { $($fields:tt)* })* }
    ) => {
        java_cst! {
            nodes {
                $($wrapper => $kind,)*
                $($bogus => $bogus,)*
            }
            enums {
                $($family = $($member)|+;)*
            }
        }
    };
}

java_syntax_schema!(define_java_cst_from_schema);

#[cfg(test)]
impl<'source> CompilationUnit<'source> {
    pub(crate) fn syntax(&self) -> &JavaSyntaxNode<'source> {
        &self.syntax
    }
}

mod accessors;

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

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AnnotationArgument<'source> {
    Value(AnnotationElementValue<'source>),
    Pair(AnnotationElementValuePair<'source>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnnotationArrayInitializerEntry<'source> {
    pub value: AnnotationElementValue<'source>,
    pub comma: Option<JavaSyntaxToken<'source>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnnotationArgumentListEntry<'source> {
    pub argument: AnnotationArgument<'source>,
    pub comma: Option<JavaSyntaxToken<'source>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CompilationUnitItem<'source> {
    Package(PackageDeclaration<'source>),
    Import(ImportDeclaration<'source>),
    Module(ModuleDeclaration<'source>),
    Type(TypeDeclaration<'source>),
    Field(FieldDeclaration<'source>),
    Method(MethodDeclaration<'source>),
    EmptyDeclaration(EmptyDeclaration<'source>),
}

impl<'source> CompilationUnitItem<'source> {
    #[must_use]
    pub fn first_token(&self) -> Option<JavaSyntaxToken<'source>> {
        match self {
            Self::Package(item) => item.first_token(),
            Self::Import(item) => item.first_token(),
            Self::Module(item) => item.first_token(),
            Self::Type(item) => item.first_token(),
            Self::Field(item) => item.first_token(),
            Self::Method(item) => item.first_token(),
            Self::EmptyDeclaration(item) => item.first_token(),
        }
    }

    #[must_use]
    pub fn last_token(&self) -> Option<JavaSyntaxToken<'source>> {
        match self {
            Self::Package(item) => item.last_token(),
            Self::Import(item) => item.last_token(),
            Self::Module(item) => item.last_token(),
            Self::Type(item) => item.last_token(),
            Self::Field(item) => item.last_token(),
            Self::Method(item) => item.last_token(),
            Self::EmptyDeclaration(item) => item.last_token(),
        }
    }
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArgumentListEntry<'source> {
    pub argument: Expression<'source>,
    pub comma: Option<JavaSyntaxToken<'source>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StatementExpressionEntry<'source> {
    pub expression: Expression<'source>,
    pub comma: Option<JavaSyntaxToken<'source>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceListEntry<'source> {
    pub resource: Resource<'source>,
    pub separator: Option<JavaSyntaxToken<'source>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArrayInitializerEntry<'source> {
    pub value: VariableInitializerValue<'source>,
    pub comma: Option<JavaSyntaxToken<'source>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnionTypeEntry<'source> {
    pub ty: Type<'source>,
    pub separator: Option<JavaSyntaxToken<'source>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IntersectionTypeEntry<'source> {
    pub ty: Type<'source>,
    pub separator: Option<JavaSyntaxToken<'source>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypeArgumentListEntry<'source> {
    pub argument: TypeArgument<'source>,
    pub comma: Option<JavaSyntaxToken<'source>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypeParameterListEntry<'source> {
    pub parameter: TypeParameter<'source>,
    pub comma: Option<JavaSyntaxToken<'source>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FormalParameterListEntry<'source> {
    pub item: FormalParameterListItem<'source>,
    pub comma: Option<JavaSyntaxToken<'source>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FormalParameterListItem<'source> {
    ReceiverParameter(ReceiverParameter<'source>),
    FormalParameter(FormalParameter<'source>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecordComponentListEntry<'source> {
    pub component: RecordComponent<'source>,
    pub comma: Option<JavaSyntaxToken<'source>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnumConstantListEntry<'source> {
    pub constant: EnumConstant<'source>,
    pub comma: Option<JavaSyntaxToken<'source>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LambdaParameterListEntry<'source> {
    pub parameter: LambdaParameter<'source>,
    pub comma: Option<JavaSyntaxToken<'source>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecordPatternComponentEntry<'source> {
    pub component: ComponentPattern<'source>,
    pub comma: Option<JavaSyntaxToken<'source>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ThrowsClauseEntry<'source> {
    pub exception: Type<'source>,
    pub comma: Option<JavaSyntaxToken<'source>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypeClauseEntry<'source> {
    pub ty: Type<'source>,
    pub comma: Option<JavaSyntaxToken<'source>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PermitsClauseEntry<'source> {
    pub name: NameSyntax<'source>,
    pub comma: Option<JavaSyntaxToken<'source>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClassTypeSegment<'source> {
    pub annotations: Vec<Annotation<'source>>,
    pub dot_before: Option<JavaSyntaxToken<'source>>,
    pub name: NameSyntax<'source>,
    pub type_arguments: Option<TypeArgumentList<'source>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NameSegment<'source> {
    pub annotations: Vec<Annotation<'source>>,
    pub dot_before: Option<JavaSyntaxToken<'source>>,
    pub identifier: JavaSyntaxToken<'source>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModifierEntry<'source> {
    tokens: [Option<JavaSyntaxToken<'source>>; 3],
    len: usize,
}

impl<'source> ModifierEntry<'source> {
    pub(crate) fn single(token: JavaSyntaxToken<'source>) -> Self {
        Self {
            tokens: [Some(token), None, None],
            len: 1,
        }
    }

    pub(crate) fn non_sealed(
        non: JavaSyntaxToken<'source>,
        minus: JavaSyntaxToken<'source>,
        sealed: JavaSyntaxToken<'source>,
    ) -> Self {
        Self {
            tokens: [Some(non), Some(minus), Some(sealed)],
            len: 3,
        }
    }

    pub fn tokens(&self) -> impl Iterator<Item = &JavaSyntaxToken<'source>> {
        self.tokens[..self.len].iter().filter_map(Option::as_ref)
    }

    /// Returns true if this entry spells the `sealed` modifier (single Identifier token
    /// with text "sealed" — `sealed` is a contextual keyword in Java, not a reserved word).
    #[must_use]
    pub fn is_sealed(&self) -> bool {
        self.len == 1
            && self.tokens[0].as_ref().is_some_and(|token| {
                token.kind() == JavaSyntaxKind::Identifier && token.text() == "sealed"
            })
    }

    /// Returns true if this entry spells the `non-sealed` modifier (three tokens:
    /// Identifier "non", Minus, Identifier "sealed").
    #[must_use]
    pub fn is_non_sealed(&self) -> bool {
        self.len == 3
            && self.tokens[0]
                .as_ref()
                .is_some_and(|t| t.kind() == JavaSyntaxKind::Identifier && t.text() == "non")
            && self.tokens[1]
                .as_ref()
                .is_some_and(|t| t.kind() == JavaSyntaxKind::Minus)
            && self.tokens[2]
                .as_ref()
                .is_some_and(|t| t.kind() == JavaSyntaxKind::Identifier && t.text() == "sealed")
    }

    fn into_tokens(self) -> impl Iterator<Item = JavaSyntaxToken<'source>> {
        self.tokens.into_iter().take(self.len).flatten()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VariableDeclaratorEntry<'source> {
    pub declarator: VariableDeclarator<'source>,
    pub comma: Option<JavaSyntaxToken<'source>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoveredNode<'source> {
    syntax: JavaSyntaxNode<'source>,
}

impl<'source> RecoveredNode<'source> {
    pub(crate) fn new(syntax: JavaSyntaxNode<'source>) -> Self {
        Self { syntax }
    }

    pub fn token_iter(&self) -> impl Iterator<Item = JavaSyntaxToken<'source>> + '_ {
        token_iter(&self.syntax)
    }

    #[must_use]
    pub fn first_token(&self) -> Option<JavaSyntaxToken<'source>> {
        first_token(&self.syntax)
    }

    #[must_use]
    pub fn last_token(&self) -> Option<JavaSyntaxToken<'source>> {
        last_token(&self.syntax)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RecoveredSeparatedListEntry<'source, Entry> {
    Entry(Entry),
    Token(JavaSyntaxToken<'source>),
    Error(ErrorNode<'source>),
    Node(RecoveredNode<'source>),
}

impl<'source> AnnotationArgument<'source> {
    fn cast(syntax: JavaSyntaxNode<'source>) -> Option<Self> {
        match syntax.kind() {
            JavaSyntaxKind::AnnotationElementValue => {
                AnnotationElementValue::cast(syntax).map(Self::Value)
            }
            JavaSyntaxKind::AnnotationElementValuePair => {
                AnnotationElementValuePair::cast(syntax).map(Self::Pair)
            }
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SwitchBlockEntry<'source> {
    StatementGroup(SwitchBlockStatementGroup<'source>),
    Rule(SwitchRule<'source>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConstructorBodyEntry<'source> {
    Invocation(ConstructorInvocation<'source>),
    BlockStatement(BlockStatement<'source>),
}

pub(crate) fn cast_compilation_unit(syntax: JavaSyntaxNode<'_>) -> Option<CompilationUnit<'_>> {
    <CompilationUnit<'_> as JavaNode<'_>>::cast(syntax)
}

fn child<'source, N: JavaNode<'source>>(syntax: &JavaSyntaxNode<'source>) -> Option<N> {
    syntax.children().find_map(N::cast)
}

pub(crate) fn recovered_child<'source, N: JavaNode<'source>>(
    syntax: &JavaSyntaxNode<'source>,
) -> Option<N> {
    syntax.children().find_map(|node| {
        (node.kind() == JavaSyntaxKind::ErrorNode)
            .then(|| child(&node))
            .flatten()
    })
}

fn children<'source, N: JavaNode<'source>>(
    syntax: &JavaSyntaxNode<'source>,
) -> impl Iterator<Item = N> + use<'source, N> {
    syntax.children().filter_map(N::cast)
}

fn token_iter<'source>(
    syntax: &JavaSyntaxNode<'source>,
) -> impl Iterator<Item = JavaSyntaxToken<'source>> + use<'source> {
    syntax.tokens()
}

fn first_token<'source>(syntax: &JavaSyntaxNode<'source>) -> Option<JavaSyntaxToken<'source>> {
    syntax.first_token()
}

fn last_token<'source>(syntax: &JavaSyntaxNode<'source>) -> Option<JavaSyntaxToken<'source>> {
    syntax.last_token()
}

fn starts_after_blank_line(syntax: &JavaSyntaxNode<'_>) -> bool {
    first_token(syntax).is_some_and(|token| token.has_leading_blank_line())
}

fn child_family<'source, F: JavaFamily<'source>>(syntax: &JavaSyntaxNode<'source>) -> Option<F> {
    syntax.children().find_map(F::cast)
}

pub(crate) fn recovered_child_family<'source, F: JavaFamily<'source>>(
    syntax: &JavaSyntaxNode<'source>,
) -> Option<F> {
    syntax.children().find_map(|node| {
        (node.kind() == JavaSyntaxKind::ErrorNode)
            .then(|| child_family(&node))
            .flatten()
    })
}

pub(crate) fn recovered_error_tokens<'source>(
    syntax: &JavaSyntaxNode<'source>,
    kind: JavaSyntaxKind,
) -> impl Iterator<Item = JavaSyntaxToken<'source>> + use<'source> {
    syntax.children().filter_map(move |node| {
        (node.kind() == JavaSyntaxKind::ErrorNode)
            .then(|| node.child_tokens().find(|token| token.kind() == kind))
            .flatten()
    })
}

pub(crate) fn direct_error_tokens<'source>(
    syntax: &JavaSyntaxNode<'source>,
) -> impl Iterator<Item = JavaSyntaxToken<'source>> + use<'source> {
    syntax
        .children()
        .filter(|node| node.kind() == JavaSyntaxKind::ErrorNode)
        .flat_map(|node| node.tokens())
}

fn nth_child_family<'source, F: JavaFamily<'source>>(
    syntax: &JavaSyntaxNode<'source>,
    index: usize,
) -> Option<F> {
    syntax.children().filter_map(F::cast).nth(index)
}

fn children_family<'source, F: JavaFamily<'source>>(
    syntax: &JavaSyntaxNode<'source>,
) -> impl Iterator<Item = F> + use<'source, F> {
    syntax.children().filter_map(F::cast)
}

fn child_token<'source>(
    syntax: &JavaSyntaxNode<'source>,
    kind: JavaSyntaxKind,
) -> Option<JavaSyntaxToken<'source>> {
    nth_child_token(syntax, kind, 0)
}

fn nth_child_token<'source>(
    syntax: &JavaSyntaxNode<'source>,
    kind: JavaSyntaxKind,
    index: usize,
) -> Option<JavaSyntaxToken<'source>> {
    syntax
        .child_tokens()
        .filter(|token| token.kind() == kind)
        .nth(index)
}

fn child_token_in<'source>(
    syntax: &JavaSyntaxNode<'source>,
    kinds: &[JavaSyntaxKind],
) -> Option<JavaSyntaxToken<'source>> {
    syntax
        .child_tokens()
        .find(|token| kinds.contains(&token.kind()))
}

fn children_tokens_matching<'source, P>(
    syntax: &JavaSyntaxNode<'source>,
    predicate: P,
) -> impl Iterator<Item = JavaSyntaxToken<'source>> + use<'source, P>
where
    P: Fn(JavaSyntaxKind) -> bool + Copy,
{
    syntax
        .child_tokens()
        .filter(move |token| predicate(token.kind()))
}
