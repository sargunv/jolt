use std::fmt;

pub use jolt_syntax::{
    Comment as KotlinComment, CommentKind as KotlinCommentKind, Comments as KotlinComments,
};
use jolt_syntax::{SyntaxNode, SyntaxToken};
use jolt_text::TextRange;

use crate::{KotlinSyntaxKind, language::KotlinLanguage};

mod accessors;

pub use accessors::{
    ClassMemberDeclarationEntry, ContextFunctionTypeParameterEntry, ContextParameterClauseEntry,
    DelegationSpecifierListEntry, LambdaParameterListEntry, NavigationOperatorTokens,
    QualifiedNameSegment, TokenGap, ValueArgumentEntry, WhenEntryRecoveryPart,
    operators_equivalent, token_gap,
};

pub(crate) type KotlinSyntaxNode<'source> = SyntaxNode<'source, KotlinLanguage>;
pub type KotlinSyntaxToken<'source> = SyntaxToken<'source, KotlinLanguage>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExpressionParentRole {
    NavigationReceiver,
    CallCallee,
    IndexReceiver,
    IndexArgument,
}

mod private {
    pub(crate) trait Sealed {}
}

pub(crate) trait KotlinNode<'source>: Clone + private::Sealed {
    fn cast(syntax: KotlinSyntaxNode<'source>) -> Option<Self>;
}

pub(crate) trait KotlinFamily<'source>: Clone {
    fn cast(syntax: KotlinSyntaxNode<'source>) -> Option<Self>;
}

fn syntax_source_text<'source>(syntax: &KotlinSyntaxNode<'source>) -> &'source str {
    let range = syntax.text_range();
    &syntax.source()[range.start().get()..range.end().get()]
}

fn token_iter<'source, 'node>(
    syntax: &'node KotlinSyntaxNode<'source>,
) -> impl Iterator<Item = KotlinSyntaxToken<'source>> + 'node
where
    'source: 'node,
{
    syntax.tokens()
}

fn first_token<'source>(syntax: &KotlinSyntaxNode<'source>) -> Option<KotlinSyntaxToken<'source>> {
    syntax.first_token()
}

fn last_token<'source>(syntax: &KotlinSyntaxNode<'source>) -> Option<KotlinSyntaxToken<'source>> {
    syntax.last_token()
}

fn child<'source, N>(syntax: &KotlinSyntaxNode<'source>) -> Option<N>
where
    N: KotlinNode<'source>,
{
    syntax.children().find_map(N::cast)
}

fn children<'source, N>(
    syntax: &KotlinSyntaxNode<'source>,
) -> impl Iterator<Item = N> + use<'source, N>
where
    N: KotlinNode<'source>,
{
    syntax.children().filter_map(N::cast)
}

fn child_family<'source, F>(syntax: &KotlinSyntaxNode<'source>) -> Option<F>
where
    F: KotlinFamily<'source>,
{
    syntax.children().find_map(F::cast)
}

fn children_family<'source, F>(
    syntax: &KotlinSyntaxNode<'source>,
) -> impl Iterator<Item = F> + use<'source, F>
where
    F: KotlinFamily<'source>,
{
    syntax.children().filter_map(F::cast)
}

fn child_token<'source>(
    syntax: &KotlinSyntaxNode<'source>,
    kind: KotlinSyntaxKind,
) -> Option<KotlinSyntaxToken<'source>> {
    syntax.child_tokens().find(|token| token.kind() == kind)
}

fn child_tokens<'source>(
    syntax: &KotlinSyntaxNode<'source>,
) -> impl Iterator<Item = KotlinSyntaxToken<'source>> + use<'source> {
    syntax.child_tokens()
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct KotlinFile<'source> {
    syntax: KotlinSyntaxNode<'source>,
}

impl<'source> KotlinFile<'source> {
    #[must_use]
    pub fn kind(&self) -> KotlinSyntaxKind {
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

    pub fn token_iter(&self) -> impl Iterator<Item = KotlinSyntaxToken<'source>> + '_ {
        token_iter(&self.syntax)
    }

    #[must_use]
    pub fn first_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        first_token(&self.syntax)
    }

    #[must_use]
    pub fn last_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        last_token(&self.syntax)
    }

    pub(crate) fn syntax(&self) -> &KotlinSyntaxNode<'source> {
        &self.syntax
    }
}

impl private::Sealed for KotlinFile<'_> {}

impl<'source> KotlinNode<'source> for KotlinFile<'source> {
    fn cast(syntax: KotlinSyntaxNode<'source>) -> Option<Self> {
        matches!(syntax.kind(), KotlinSyntaxKind::KotlinFile).then_some(Self { syntax })
    }
}

impl fmt::Debug for KotlinFile<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.syntax.fmt(f)
    }
}

pub(crate) fn cast_kotlin_file(syntax: KotlinSyntaxNode<'_>) -> Option<KotlinFile<'_>> {
    <KotlinFile<'_> as KotlinNode<'_>>::cast(syntax)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoveredNode<'source> {
    syntax: KotlinSyntaxNode<'source>,
}

impl<'source> RecoveredNode<'source> {
    pub(crate) fn new(syntax: KotlinSyntaxNode<'source>) -> Self {
        Self { syntax }
    }

    pub fn token_iter(&self) -> impl Iterator<Item = KotlinSyntaxToken<'source>> + '_ {
        token_iter(&self.syntax)
    }

    #[must_use]
    pub fn first_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        first_token(&self.syntax)
    }

    #[must_use]
    pub fn last_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        last_token(&self.syntax)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RecoveredSeparatedListEntry<'source, Entry> {
    Entry(Entry),
    Token(KotlinSyntaxToken<'source>),
    Error(ErrorNode<'source>),
    Node(RecoveredNode<'source>),
}

macro_rules! kotlin_cst {
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
                syntax: KotlinSyntaxNode<'source>,
            }

            impl<'source> $node<'source> {
                #[must_use]
                pub fn kind(&self) -> KotlinSyntaxKind {
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

                pub fn token_iter(&self) -> impl Iterator<Item = KotlinSyntaxToken<'source>> + '_ {
                    token_iter(&self.syntax)
                }

                #[must_use]
                pub fn first_token(&self) -> Option<KotlinSyntaxToken<'source>> {
                    first_token(self.syntax())
                }

                #[must_use]
                pub fn last_token(&self) -> Option<KotlinSyntaxToken<'source>> {
                    last_token(self.syntax())
                }

                #[must_use]
                pub(crate) fn syntax(&self) -> &KotlinSyntaxNode<'source> {
                    &self.syntax
                }
            }

            impl private::Sealed for $node<'_> {}

            impl<'source> KotlinNode<'source> for $node<'source> {
                fn cast(syntax: KotlinSyntaxNode<'source>) -> Option<Self> {
                    matches!(syntax.kind(), KotlinSyntaxKind::$kind).then_some(Self { syntax })
                }
            }

            impl fmt::Debug for $node<'_> {
                fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                    self.syntax.fmt(f)
                }
            }
        )*

        #[derive(Clone, Copy, Debug, Eq, PartialEq)]
        pub enum AnyKotlinNode<'source> {
            KotlinFile(KotlinFile<'source>),
            $($node($node<'source>),)*
        }

        impl<'source> AnyKotlinNode<'source> {
            pub(crate) fn cast(syntax: KotlinSyntaxNode<'source>) -> Option<Self> {
                match syntax.kind() {
                    KotlinSyntaxKind::KotlinFile => {
                        <KotlinFile<'source> as KotlinNode<'source>>::cast(syntax)
                            .map(Self::KotlinFile)
                    }
                    $(
                        KotlinSyntaxKind::$kind => {
                            <$node<'source> as KotlinNode<'source>>::cast(syntax).map(Self::$node)
                        }
                    )*
                    _ => None,
                }
            }
        }

        $(
            impl<'source> From<$node<'source>> for AnyKotlinNode<'source> {
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
                pub fn kind(&self) -> KotlinSyntaxKind {
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

                pub fn token_iter(&self) -> impl Iterator<Item = KotlinSyntaxToken<'source>> + '_ {
                    token_iter(self.syntax())
                }

                #[must_use]
                pub fn first_token(&self) -> Option<KotlinSyntaxToken<'source>> {
                    first_token(self.syntax())
                }

                #[must_use]
                pub fn last_token(&self) -> Option<KotlinSyntaxToken<'source>> {
                    last_token(self.syntax())
                }

                pub(crate) fn syntax(&self) -> &KotlinSyntaxNode<'source> {
                    match self {
                        $(Self::$variant(node) => node.syntax(),)+
                    }
                }
            }

            impl<'source> KotlinFamily<'source> for $family<'source> {
                fn cast(syntax: KotlinSyntaxNode<'source>) -> Option<Self> {
                    match syntax.kind() {
                        $(
                            KotlinSyntaxKind::$variant => {
                                <$variant<'source> as KotlinNode<'source>>::cast(syntax)
                                    .map(Self::$variant)
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

kotlin_cst! {
    nodes {
        ErrorNode => ErrorNode,
        PackageHeader => PackageHeader,
        ImportList => ImportList,
        ImportDirective => ImportDirective,
        ImportAlias => ImportAlias,
        ModifierList => ModifierList,
        Annotation => Annotation,
        AnnotationUseSiteTarget => AnnotationUseSiteTarget,
        AnnotationArgumentList => AnnotationArgumentList,
        ValueArgumentList => ValueArgumentList,
        ValueArgument => ValueArgument,
        Name => Name,
        QualifiedName => QualifiedName,
        CallableName => CallableName,
        TypeArgumentList => TypeArgumentList,
        TypeArgument => TypeArgument,
        ClassDeclaration => ClassDeclaration,
        InterfaceDeclaration => InterfaceDeclaration,
        ObjectDeclaration => ObjectDeclaration,
        CompanionObject => CompanionObject,
        EnumEntry => EnumEntry,
        ClassBody => ClassBody,
        ClassMemberDeclaration => ClassMemberDeclaration,
        PrimaryConstructor => PrimaryConstructor,
        SecondaryConstructor => SecondaryConstructor,
        ConstructorDelegationCall => ConstructorDelegationCall,
        InitializerBlock => InitializerBlock,
        FunctionDeclaration => FunctionDeclaration,
        PropertyDeclaration => PropertyDeclaration,
        PropertyAccessor => PropertyAccessor,
        ExplicitBackingField => ExplicitBackingField,
        TypeAliasDeclaration => TypeAliasDeclaration,
        TypeParameterList => TypeParameterList,
        TypeParameter => TypeParameter,
        TypeConstraintList => TypeConstraintList,
        TypeConstraint => TypeConstraint,
        ContextParameterClause => ContextParameterClause,
        ContextParameter => ContextParameter,
        DelegationSpecifierList => DelegationSpecifierList,
        DelegationSpecifier => DelegationSpecifier,
        UserType => UserType,
        NullableType => NullableType,
        FunctionType => FunctionType,
        ContextFunctionType => ContextFunctionType,
        ReceiverType => ReceiverType,
        ParenthesizedType => ParenthesizedType,
        FunctionTypeParameter => FunctionTypeParameter,
        DefinitelyNonNullableType => DefinitelyNonNullableType,
        TypeProjection => TypeProjection,
        TypeProjectionList => TypeProjectionList,
        Block => Block,
        Statement => Statement,
        ExpressionStatement => ExpressionStatement,
        LocalDeclaration => LocalDeclaration,
        AssignmentExpression => AssignmentExpression,
        BinaryExpression => BinaryExpression,
        UnaryExpression => UnaryExpression,
        PostfixExpression => PostfixExpression,
        CallExpression => CallExpression,
        IndexExpression => IndexExpression,
        NavigationExpression => NavigationExpression,
        CallableReferenceExpression => CallableReferenceExpression,
        LiteralExpression => LiteralExpression,
        StringTemplateExpression => StringTemplateExpression,
        StringTemplateEntry => StringTemplateEntry,
        NameExpression => NameExpression,
        ThisExpression => ThisExpression,
        SuperExpression => SuperExpression,
        ParenthesizedExpression => ParenthesizedExpression,
        AnnotatedExpression => AnnotatedExpression,
        IfExpression => IfExpression,
        WhenExpression => WhenExpression,
        WhenSubject => WhenSubject,
        WhenEntry => WhenEntry,
        WhenCondition => WhenCondition,
        WhenGuard => WhenGuard,
        TryExpression => TryExpression,
        CatchClause => CatchClause,
        FinallyClause => FinallyClause,
        LoopExpression => LoopExpression,
        ForStatement => ForStatement,
        WhileStatement => WhileStatement,
        DoWhileStatement => DoWhileStatement,
        JumpExpression => JumpExpression,
        ThrowExpression => ThrowExpression,
        LambdaExpression => LambdaExpression,
        LambdaParameterList => LambdaParameterList,
        LambdaParameter => LambdaParameter,
        AnonymousFunctionExpression => AnonymousFunctionExpression,
        ObjectExpression => ObjectExpression,
        CollectionLiteralExpression => CollectionLiteralExpression,
        DestructuringDeclaration => DestructuringDeclaration,
        DestructuringEntry => DestructuringEntry,
        ValueParameterList => ValueParameterList,
        ValueParameter => ValueParameter,
        TypeReference => TypeReference,
    }
    enums {
        KotlinFileItem =
            PackageHeader |
            ImportList |
            ClassDeclaration |
            InterfaceDeclaration |
            ObjectDeclaration |
            CompanionObject |
            EnumEntry |
            FunctionDeclaration |
            PropertyDeclaration |
            TypeAliasDeclaration |
            SecondaryConstructor |
            InitializerBlock |
            Statement;

        Declaration =
            ClassDeclaration |
            InterfaceDeclaration |
            ObjectDeclaration |
            CompanionObject |
            EnumEntry |
            FunctionDeclaration |
            PropertyDeclaration |
            TypeAliasDeclaration |
            SecondaryConstructor |
            InitializerBlock;

        ClassMember =
            ClassMemberDeclaration |
            ClassDeclaration |
            InterfaceDeclaration |
            ObjectDeclaration |
            CompanionObject |
            EnumEntry |
            FunctionDeclaration |
            PropertyDeclaration |
            TypeAliasDeclaration |
            SecondaryConstructor |
            InitializerBlock |
            PropertyAccessor |
            ExplicitBackingField |
            Statement;

        TypeSyntax =
            UserType |
            NullableType |
            FunctionType |
            ContextFunctionType |
            ReceiverType |
            ParenthesizedType |
            DefinitelyNonNullableType;

        Expression =
            AssignmentExpression |
            BinaryExpression |
            UnaryExpression |
            PostfixExpression |
            CallExpression |
            IndexExpression |
            NavigationExpression |
            CallableReferenceExpression |
            LiteralExpression |
            StringTemplateExpression |
            NameExpression |
            ThisExpression |
            SuperExpression |
            ParenthesizedExpression |
            AnnotatedExpression |
            IfExpression |
            WhenExpression |
            TryExpression |
            LoopExpression |
            ForStatement |
            WhileStatement |
            DoWhileStatement |
            JumpExpression |
            ThrowExpression |
            LambdaExpression |
            AnonymousFunctionExpression |
            ObjectExpression |
            CollectionLiteralExpression;

        WhenConditionSyntax =
            WhenCondition |
            WhenGuard;

        StringTemplatePart =
            StringTemplateEntry;

        ValueArgumentListEntry =
            ValueArgument;

        TypeArgumentListEntry =
            TypeArgument |
            TypeProjection;

        DestructuringPatternEntry =
            DestructuringEntry;

        StatementSyntax =
            Statement |
            ExpressionStatement |
            LocalDeclaration |
            Block;

        BlockItem =
            Statement |
            ExpressionStatement |
            LocalDeclaration |
            Block |
            ClassDeclaration |
            InterfaceDeclaration |
            ObjectDeclaration |
            FunctionDeclaration |
            PropertyDeclaration |
            TypeAliasDeclaration |
            SecondaryConstructor |
            InitializerBlock;
    }
}
