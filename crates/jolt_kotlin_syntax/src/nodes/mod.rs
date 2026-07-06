use std::{fmt, slice};

use jolt_syntax::{SyntaxNode, SyntaxToken, SyntaxTrivia, TriviaKind as SyntaxTriviaKind};
use jolt_text::TextRange;

use crate::{KotlinSyntaxKind, language::KotlinLanguage};

mod accessors;

pub(crate) type KotlinSyntaxNode<'source> = SyntaxNode<'source, KotlinLanguage>;
type KotlinRawSyntaxToken<'source> = SyntaxToken<'source, KotlinLanguage>;

/// A comment attached as token trivia in the Kotlin syntax tree.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KotlinComment<'source> {
    kind: KotlinCommentKind,
    source: &'source str,
    text_range: TextRange,
}

/// Borrowed comments attached to syntax token trivia.
#[derive(Clone)]
pub struct KotlinComments<'source> {
    source: &'source str,
    trivia: slice::Iter<'source, SyntaxTrivia>,
    offset: jolt_text::TextSize,
}

impl<'source> KotlinComments<'source> {
    fn new(
        source: &'source str,
        trivia: &'source [SyntaxTrivia],
        offset: jolt_text::TextSize,
    ) -> Self {
        Self {
            source,
            trivia: trivia.iter(),
            offset,
        }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.trivia.as_slice().iter().all(|trivia| {
            !matches!(
                trivia.kind(),
                SyntaxTriviaKind::LineComment
                    | SyntaxTriviaKind::ShebangComment
                    | SyntaxTriviaKind::BlockComment
                    | SyntaxTriviaKind::DocComment
            )
        })
    }
}

impl<'source> Iterator for KotlinComments<'source> {
    type Item = KotlinComment<'source>;

    fn next(&mut self) -> Option<Self::Item> {
        for trivia in self.trivia.by_ref() {
            let text_range = TextRange::new(self.offset, self.offset + trivia.text_len());
            self.offset = text_range.end();
            let kind = match trivia.kind() {
                SyntaxTriviaKind::LineComment | SyntaxTriviaKind::ShebangComment => {
                    KotlinCommentKind::Line
                }
                SyntaxTriviaKind::BlockComment => KotlinCommentKind::Block,
                SyntaxTriviaKind::DocComment => KotlinCommentKind::Doc,
                SyntaxTriviaKind::Whitespace
                | SyntaxTriviaKind::Newline
                | SyntaxTriviaKind::Ignored => continue,
            };
            return Some(KotlinComment {
                kind,
                source: self.source,
                text_range,
            });
        }

        None
    }
}

impl<'source> KotlinComment<'source> {
    #[must_use]
    pub const fn kind(&self) -> KotlinCommentKind {
        self.kind
    }

    #[must_use]
    pub fn text(&self) -> &'source str {
        &self.source[self.text_range.start().get()..self.text_range.end().get()]
    }

    #[must_use]
    pub fn text_range(&self) -> TextRange {
        self.text_range
    }
}

/// A Kotlin comment kind exposed from syntax trivia.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KotlinCommentKind {
    /// A `//` line comment.
    Line,
    /// A non-`KDoc` block comment.
    Block,
    /// A `KDoc` comment.
    Doc,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct KotlinSyntaxToken<'source> {
    syntax: KotlinRawSyntaxToken<'source>,
}

impl<'source> KotlinSyntaxToken<'source> {
    #[must_use]
    pub fn kind(&self) -> KotlinSyntaxKind {
        self.syntax.kind()
    }

    #[must_use]
    pub fn text(&self) -> &'source str {
        self.syntax.text()
    }

    #[must_use]
    pub fn text_range(&self) -> TextRange {
        self.syntax.text_range()
    }

    #[must_use]
    pub fn token_text_range(&self) -> TextRange {
        self.syntax.token_text_range()
    }

    #[must_use]
    pub fn leading_comments(&self) -> KotlinComments<'source> {
        KotlinComments::new(
            self.syntax.source(),
            self.syntax.leading(),
            self.syntax.offset(),
        )
    }

    #[must_use]
    pub fn trailing_comments(&self) -> KotlinComments<'source> {
        KotlinComments::new(
            self.syntax.source(),
            self.syntax.trailing(),
            self.syntax.token_text_range().end(),
        )
    }
}

impl fmt::Debug for KotlinSyntaxToken<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.syntax.fmt(f)
    }
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
    syntax.tokens().map(|syntax| KotlinSyntaxToken { syntax })
}

fn first_token<'source>(syntax: &KotlinSyntaxNode<'source>) -> Option<KotlinSyntaxToken<'source>> {
    syntax
        .first_token()
        .map(|syntax| KotlinSyntaxToken { syntax })
}

fn last_token<'source>(syntax: &KotlinSyntaxNode<'source>) -> Option<KotlinSyntaxToken<'source>> {
    syntax
        .last_token()
        .map(|syntax| KotlinSyntaxToken { syntax })
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
    syntax
        .child_tokens()
        .map(|syntax| KotlinSyntaxToken { syntax })
        .find(|token| token.kind() == kind)
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
                    first_token(&self.syntax)
                }

                #[must_use]
                pub fn last_token(&self) -> Option<KotlinSyntaxToken<'source>> {
                    last_token(&self.syntax)
                }

                #[allow(dead_code)]
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
    }
}
