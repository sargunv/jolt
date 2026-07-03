use std::fmt;

use jolt_syntax::{
    SyntaxElement, SyntaxNode, SyntaxToken, TriviaKind as SyntaxTriviaKind, green_text,
};
use jolt_text::TextRange;

use crate::{JavaSyntaxKind, language::JavaLanguage};

pub(crate) type JavaSyntaxNode = SyntaxNode<JavaLanguage>;
type JavaRawSyntaxToken = SyntaxToken<JavaLanguage>;

/// A comment attached as token trivia in the Java syntax tree.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JavaComment {
    kind: JavaCommentKind,
    text: String,
    text_range: TextRange,
}

impl JavaComment {
    /// Returns the comment kind.
    #[must_use]
    pub const fn kind(&self) -> JavaCommentKind {
        self.kind
    }

    /// Returns the raw comment text.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Returns the raw source range covered by the comment.
    #[must_use]
    pub const fn text_range(&self) -> TextRange {
        self.text_range
    }
}

/// A Java comment kind exposed from syntax trivia.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JavaCommentKind {
    /// A `//` line comment.
    Line,
    /// A non-Javadoc block comment.
    Block,
    /// A Javadoc comment.
    Doc,
}

#[derive(Clone, Eq, PartialEq)]
pub struct JavaSyntaxToken {
    syntax: JavaRawSyntaxToken,
}

impl JavaSyntaxToken {
    #[must_use]
    pub fn kind(&self) -> JavaSyntaxKind {
        self.syntax.kind()
    }

    #[must_use]
    pub fn text(&self) -> &str {
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

    /// Returns comments attached before this token.
    #[must_use]
    pub fn leading_comments(&self) -> Vec<JavaComment> {
        comments_from_trivia(self.syntax.leading(), self.syntax.offset())
    }

    /// Returns comments attached after this token.
    #[must_use]
    pub fn trailing_comments(&self) -> Vec<JavaComment> {
        comments_from_trivia(self.syntax.trailing(), self.syntax.token_text_range().end())
    }

    /// Returns true when the token's leading trivia contains an intentional
    /// blank line.
    #[must_use]
    pub fn has_leading_blank_line(&self) -> bool {
        trivia_has_blank_line(self.syntax.leading())
    }
}

impl fmt::Debug for JavaSyntaxToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.syntax.fmt(f)
    }
}

fn comments_from_trivia(
    trivia: &[jolt_syntax::GreenTrivia],
    start: jolt_text::TextSize,
) -> Vec<JavaComment> {
    let mut offset = start;
    trivia
        .iter()
        .filter_map(|trivia| {
            let text_range = TextRange::new(offset, offset + trivia.text_len());
            offset = text_range.end();
            let kind = match trivia.kind() {
                SyntaxTriviaKind::LineComment => JavaCommentKind::Line,
                SyntaxTriviaKind::BlockComment => JavaCommentKind::Block,
                SyntaxTriviaKind::DocComment => JavaCommentKind::Doc,
                SyntaxTriviaKind::Whitespace
                | SyntaxTriviaKind::Newline
                | SyntaxTriviaKind::Ignored => {
                    return None;
                }
            };
            Some(JavaComment {
                kind,
                text: trivia.text().to_owned(),
                text_range,
            })
        })
        .collect()
}

fn trivia_has_blank_line(trivia: &[jolt_syntax::GreenTrivia]) -> bool {
    let mut line_breaks_since_content = 0;
    for trivia in trivia {
        match trivia.kind() {
            SyntaxTriviaKind::Newline => {
                line_breaks_since_content += 1;
                if line_breaks_since_content >= 2 {
                    return true;
                }
            }
            SyntaxTriviaKind::Whitespace | SyntaxTriviaKind::Ignored => {}
            SyntaxTriviaKind::LineComment
            | SyntaxTriviaKind::BlockComment
            | SyntaxTriviaKind::DocComment => {
                line_breaks_since_content = 0;
            }
        }
    }

    false
}

mod private {
    pub trait Sealed {}
}

pub(crate) trait JavaNode: Clone + private::Sealed {
    fn cast(syntax: JavaSyntaxNode) -> Option<Self>;
}

pub(crate) trait JavaFamily: Clone {
    fn cast(syntax: JavaSyntaxNode) -> Option<Self>;
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
            #[derive(Clone, Eq, PartialEq)]
            pub struct $node {
                syntax: JavaSyntaxNode,
            }

            impl $node {
                #[must_use]
                pub fn can_cast(kind: JavaSyntaxKind) -> bool {
                    matches!(kind, JavaSyntaxKind::$kind)
                }

                #[must_use]
                pub fn kind(&self) -> JavaSyntaxKind {
                    self.syntax.kind()
                }

                #[must_use]
                pub fn text_range(&self) -> TextRange {
                    self.syntax.text_range()
                }

                #[must_use]
                pub fn source_text(&self) -> String {
                    green_text(self.syntax.green())
                }

                #[must_use]
                pub fn tokens(&self) -> Vec<JavaSyntaxToken> {
                    tokens(&self.syntax)
                }

            }

            impl private::Sealed for $node {}

            impl JavaNode for $node {
                fn cast(syntax: JavaSyntaxNode) -> Option<Self> {
                    Self::can_cast(syntax.kind()).then_some(Self { syntax })
                }
            }

            impl fmt::Debug for $node {
                fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                    self.syntax.fmt(f)
                }
            }
        )*

        #[derive(Clone, Debug, Eq, PartialEq)]
        pub enum AnyJavaNode {
            $($node($node),)*
        }

        impl AnyJavaNode {
            /// Returns true if `kind` is any Java CST node kind.
            #[must_use]
            pub fn can_cast(kind: JavaSyntaxKind) -> bool {
                matches!(kind, $(JavaSyntaxKind::$kind)|*)
            }

            #[must_use]
            pub fn kind(&self) -> JavaSyntaxKind {
                self.syntax().kind()
            }

            #[must_use]
            pub fn text_range(&self) -> TextRange {
                self.syntax().text_range()
            }

            #[must_use]
            pub fn source_text(&self) -> String {
                green_text(self.syntax().green())
            }

            #[must_use]
            pub fn tokens(&self) -> Vec<JavaSyntaxToken> {
                tokens(self.syntax())
            }

            pub(crate) fn cast(syntax: JavaSyntaxNode) -> Option<Self> {
                match syntax.kind() {
                    $(
                        JavaSyntaxKind::$kind => {
                            <$node as JavaNode>::cast(syntax).map(Self::$node)
                        }
                    )*
                    _ => None,
                }
            }

            fn syntax(&self) -> &JavaSyntaxNode {
                match self {
                    $(Self::$node(node) => &node.syntax,)*
                }
            }
        }

        $(
            impl From<$node> for AnyJavaNode {
                fn from(node: $node) -> Self {
                    Self::$node(node)
                }
            }
        )*

        $(
            #[derive(Clone, Debug, Eq, PartialEq)]
            pub enum $family {
                $($variant($variant),)+
            }

            impl $family {
                #[must_use]
                pub fn can_cast(kind: JavaSyntaxKind) -> bool {
                    matches!(kind, $(JavaSyntaxKind::$variant)|+)
                }

                #[must_use]
                pub fn kind(&self) -> JavaSyntaxKind {
                    self.syntax().kind()
                }

                #[must_use]
                pub fn text_range(&self) -> TextRange {
                    self.syntax().text_range()
                }

                #[must_use]
                pub fn source_text(&self) -> String {
                    green_text(self.syntax().green())
                }

                #[must_use]
                pub fn tokens(&self) -> Vec<JavaSyntaxToken> {
                    tokens(self.syntax())
                }

                pub(crate) fn syntax(&self) -> &JavaSyntaxNode {
                    match self {
                        $(Self::$variant(node) => &node.syntax,)+
                    }
                }
            }

            impl JavaFamily for $family {
                fn cast(syntax: JavaSyntaxNode) -> Option<Self> {
                    match syntax.kind() {
                        $(
                            JavaSyntaxKind::$variant => {
                                <$variant as JavaNode>::cast(syntax).map(Self::$variant)
                            }
                        )+
                        _ => None,
                    }
                }
            }

            $(
                impl From<$variant> for $family {
                    fn from(node: $variant) -> Self {
                        Self::$variant(node)
                    }
                }
            )+
        )*

        #[cfg(test)]
        const ALL_NODE_KINDS: &[JavaSyntaxKind] = &[
            $(JavaSyntaxKind::$kind,)*
        ];

        #[cfg(test)]
        fn node_casts_for_kind(kind: JavaSyntaxKind, syntax: JavaSyntaxNode) -> Vec<&'static str> {
            let mut casts = Vec::new();
            $(
                if <$node as JavaNode>::cast(syntax.clone()).is_some() {
                    casts.push(stringify!($node));
                }
            )*
            let _ = kind;
            casts
        }

        #[cfg(test)]
        fn assert_node_wrappers_cast_their_declared_kind() {
            $(
                {
                    let syntax = test_syntax(JavaSyntaxKind::$kind);
                    let node = <$node as JavaNode>::cast(syntax)
                        .expect(concat!(stringify!($node), " should cast its declared kind"));

                    assert_eq!(node.kind(), JavaSyntaxKind::$kind);
                    assert_eq!(
                        wrapper_expected_kind_name(stringify!($node)),
                        stringify!($kind),
                        concat!(stringify!($node), " is mapped to the wrong JavaSyntaxKind")
                    );
                }
            )*
        }

        #[cfg(test)]
        fn family_casts_for_kind(kind: JavaSyntaxKind, syntax: JavaSyntaxNode) -> Vec<&'static str> {
            let mut casts = Vec::new();
            $(
                if <$family as JavaFamily>::cast(syntax.clone()).is_some() {
                    casts.push(stringify!($family));
                }
            )*
            let _ = kind;
            casts
        }

        #[cfg(test)]
        fn family_variant_kinds() -> Vec<(&'static str, &'static [JavaSyntaxKind])> {
            vec![
                $(
                    (stringify!($family), &[$(JavaSyntaxKind::$variant,)+]),
                )*
            ]
        }

        #[cfg(test)]
        fn assert_family_conversions_compile_and_preserve_kind() {
            $(
                $(
                    {
                        let syntax = test_syntax(JavaSyntaxKind::$variant);
                        let node = <$variant as JavaNode>::cast(syntax)
                            .expect("variant wrapper should cast");
                        let family: $family = node.into();
                        assert_eq!(family.kind(), JavaSyntaxKind::$variant);
                    }
                )+
            )*
        }
    };
}

java_cst! {
    nodes {
        ErrorNode => ErrorNode,
        CompilationUnit => CompilationUnit,
        PackageDeclaration => PackageDeclaration,
        ImportDeclaration => ImportDeclaration,
        ModuleDeclaration => ModuleDeclaration,
        ModuleDirectiveNode => ModuleDirective,
        RequiresDirective => RequiresDirective,
        ExportsDirective => ExportsDirective,
        OpensDirective => OpensDirective,
        UsesDirective => UsesDirective,
        ProvidesDirective => ProvidesDirective,
        ModifierList => ModifierList,
        Annotation => Annotation,
        AnnotationArgumentList => AnnotationArgumentList,
        AnnotationElementDeclaration => AnnotationElementDeclaration,
        AnnotationElementValue => AnnotationElementValue,
        AnnotationElementValuePair => AnnotationElementValuePair,
        AnnotationElementList => AnnotationElementList,
        AnnotationArrayInitializer => AnnotationArrayInitializer,
        DefaultValue => DefaultValue,
        ClassDeclaration => ClassDeclaration,
        RecordDeclaration => RecordDeclaration,
        EnumDeclaration => EnumDeclaration,
        InterfaceDeclaration => InterfaceDeclaration,
        AnnotationInterfaceDeclaration => AnnotationInterfaceDeclaration,
        TypeParameterList => TypeParameterList,
        TypeParameter => TypeParameter,
        TypeBoundList => TypeBoundList,
        ExtendsClause => ExtendsClause,
        ImplementsClause => ImplementsClause,
        PermitsClause => PermitsClause,
        ClassBody => ClassBody,
        ClassBodyDeclaration => ClassBodyDeclaration,
        EmptyDeclaration => EmptyDeclaration,
        RecordBody => RecordBody,
        InterfaceBody => InterfaceBody,
        AnnotationInterfaceBody => AnnotationInterfaceBody,
        EnumBody => EnumBody,
        EnumConstantList => EnumConstantList,
        EnumConstant => EnumConstant,
        RecordComponentList => RecordComponentList,
        RecordComponent => RecordComponent,
        FieldDeclaration => FieldDeclaration,
        MethodDeclaration => MethodDeclaration,
        ConstructorDeclaration => ConstructorDeclaration,
        ConstructorBody => ConstructorBody,
        ConstructorInvocation => ConstructorInvocation,
        CompactConstructorDeclaration => CompactConstructorDeclaration,
        StaticInitializer => StaticInitializer,
        InstanceInitializer => InstanceInitializer,
        FormalParameterList => FormalParameterList,
        FormalParameter => FormalParameter,
        ReceiverParameter => ReceiverParameter,
        ThrowsClause => ThrowsClause,
        VariableDeclaratorList => VariableDeclaratorList,
        VariableDeclarator => VariableDeclarator,
        VariableInitializer => VariableInitializer,
        Block => Block,
        BlockStatement => BlockStatement,
        LocalVariableDeclaration => LocalVariableDeclaration,
        LocalClassOrInterfaceDeclaration => LocalClassOrInterfaceDeclaration,
        EmptyStatement => EmptyStatement,
        LabeledStatement => LabeledStatement,
        ExpressionStatement => ExpressionStatement,
        IfStatement => IfStatement,
        AssertStatement => AssertStatement,
        SwitchStatement => SwitchStatement,
        SwitchBlock => SwitchBlock,
        SwitchBlockStatementGroup => SwitchBlockStatementGroup,
        SwitchRule => SwitchRule,
        SwitchLabel => SwitchLabel,
        CaseConstant => CaseConstant,
        CasePattern => CasePattern,
        Guard => Guard,
        WhileStatement => WhileStatement,
        DoStatement => DoStatement,
        ForStatement => ForStatement,
        BasicForStatement => BasicForStatement,
        EnhancedForStatement => EnhancedForStatement,
        ForInitializer => ForInitializer,
        ForUpdate => ForUpdate,
        StatementExpressionList => StatementExpressionList,
        BreakStatement => BreakStatement,
        YieldStatement => YieldStatement,
        ContinueStatement => ContinueStatement,
        ReturnStatement => ReturnStatement,
        ThrowStatement => ThrowStatement,
        SynchronizedStatement => SynchronizedStatement,
        TryStatement => TryStatement,
        TryWithResourcesStatement => TryWithResourcesStatement,
        CatchClause => CatchClause,
        CatchParameter => CatchParameter,
        CatchTypeList => CatchTypeList,
        FinallyClause => FinallyClause,
        ResourceSpecification => ResourceSpecification,
        ResourceList => ResourceList,
        Resource => Resource,
        VariableAccess => VariableAccess,
        PrimitiveType => PrimitiveType,
        VoidType => VoidType,
        ClassType => ClassType,
        ArrayType => ArrayType,
        IntersectionType => IntersectionType,
        UnionType => UnionType,
        TypeArgumentList => TypeArgumentList,
        TypeArgument => TypeArgument,
        WildcardType => WildcardType,
        ArrayDimensions => ArrayDimensions,
        ArrayDimension => ArrayDimension,
        Name => Name,
        QualifiedName => QualifiedName,
        LiteralExpression => LiteralExpression,
        NameExpression => NameExpression,
        ThisExpression => ThisExpression,
        SuperExpression => SuperExpression,
        ParenthesizedExpression => ParenthesizedExpression,
        ClassLiteralExpression => ClassLiteralExpression,
        FieldAccessExpression => FieldAccessExpression,
        ArrayAccessExpression => ArrayAccessExpression,
        MethodInvocationExpression => MethodInvocationExpression,
        MethodReferenceExpression => MethodReferenceExpression,
        ObjectCreationExpression => ObjectCreationExpression,
        ArrayCreationExpression => ArrayCreationExpression,
        DimExpression => DimExpression,
        ArrayInitializer => ArrayInitializer,
        AssignmentExpression => AssignmentExpression,
        ConditionalExpression => ConditionalExpression,
        InstanceofExpression => InstanceofExpression,
        BinaryExpression => BinaryExpression,
        UnaryExpression => UnaryExpression,
        PostfixExpression => PostfixExpression,
        CastExpression => CastExpression,
        LambdaExpression => LambdaExpression,
        LambdaParameterList => LambdaParameterList,
        LambdaParameter => LambdaParameter,
        SwitchExpression => SwitchExpression,
        ArgumentList => ArgumentList,
        TypePattern => TypePattern,
        RecordPattern => RecordPattern,
        ComponentPattern => ComponentPattern,
        MatchAllPattern => MatchAllPattern,
    }
    enums {
        TypeDeclaration =
            ClassDeclaration |
            RecordDeclaration |
            EnumDeclaration |
            InterfaceDeclaration |
            AnnotationInterfaceDeclaration;

        Statement =
            Block |
            EmptyStatement |
            LabeledStatement |
            ExpressionStatement |
            IfStatement |
            AssertStatement |
            SwitchStatement |
            WhileStatement |
            DoStatement |
            ForStatement |
            BreakStatement |
            YieldStatement |
            ContinueStatement |
            ReturnStatement |
            ThrowStatement |
            SynchronizedStatement |
            TryStatement |
            TryWithResourcesStatement;

        Expression =
            LiteralExpression |
            NameExpression |
            ThisExpression |
            SuperExpression |
            ParenthesizedExpression |
            ClassLiteralExpression |
            FieldAccessExpression |
            ArrayAccessExpression |
            MethodInvocationExpression |
            MethodReferenceExpression |
            ObjectCreationExpression |
            ArrayCreationExpression |
            AssignmentExpression |
            ConditionalExpression |
            InstanceofExpression |
            BinaryExpression |
            UnaryExpression |
            PostfixExpression |
            CastExpression |
            LambdaExpression |
            SwitchExpression;

        Type =
            PrimitiveType |
            VoidType |
            ClassType |
            ArrayType |
            IntersectionType |
            UnionType |
            WildcardType;

        Pattern =
            TypePattern |
            RecordPattern |
            ComponentPattern |
            MatchAllPattern;

        NameSyntax =
            Name |
            QualifiedName;

        ModuleDirective =
            RequiresDirective |
            ExportsDirective |
            OpensDirective |
            UsesDirective |
            ProvidesDirective;

        BlockItem =
            LocalVariableDeclaration |
            LocalClassOrInterfaceDeclaration |
            Block |
            EmptyStatement |
            LabeledStatement |
            ExpressionStatement |
            IfStatement |
            AssertStatement |
            SwitchStatement |
            WhileStatement |
            DoStatement |
            ForStatement |
            BreakStatement |
            YieldStatement |
            ContinueStatement |
            ReturnStatement |
            ThrowStatement |
            SynchronizedStatement |
            TryStatement |
            TryWithResourcesStatement;

        ClassBodyMember =
            EmptyDeclaration |
            ClassDeclaration |
            RecordDeclaration |
            EnumDeclaration |
            InterfaceDeclaration |
            AnnotationInterfaceDeclaration |
            FieldDeclaration |
            MethodDeclaration |
            ConstructorDeclaration |
            CompactConstructorDeclaration |
            StaticInitializer |
            InstanceInitializer;

        InterfaceBodyMember =
            EmptyDeclaration |
            ClassDeclaration |
            RecordDeclaration |
            EnumDeclaration |
            InterfaceDeclaration |
            AnnotationInterfaceDeclaration |
            FieldDeclaration |
            MethodDeclaration;

        AnnotationInterfaceBodyMember =
            EmptyDeclaration |
            ClassDeclaration |
            RecordDeclaration |
            EnumDeclaration |
            InterfaceDeclaration |
            AnnotationInterfaceDeclaration |
            FieldDeclaration |
            MethodDeclaration |
            AnnotationElementDeclaration;

        VariableInitializerValue =
            LiteralExpression |
            NameExpression |
            ThisExpression |
            SuperExpression |
            ParenthesizedExpression |
            ClassLiteralExpression |
            FieldAccessExpression |
            ArrayAccessExpression |
            MethodInvocationExpression |
            MethodReferenceExpression |
            ObjectCreationExpression |
            ArrayCreationExpression |
            AssignmentExpression |
            ConditionalExpression |
            InstanceofExpression |
            BinaryExpression |
            UnaryExpression |
            PostfixExpression |
            CastExpression |
            LambdaExpression |
            SwitchExpression |
            ArrayInitializer;
    }
}

mod accessors;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SwitchLabelCaseItem {
    Constant(CaseConstant),
    Pattern(CasePattern),
    Default(JavaSyntaxToken),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SwitchLabelCaseEntry {
    pub item: SwitchLabelCaseItem,
    pub comma: Option<JavaSyntaxToken>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SwitchBlockStatementGroupLabel {
    pub label: SwitchLabel,
    pub colon: Option<JavaSyntaxToken>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AnnotationArgument {
    Value(AnnotationElementValue),
    Pair(AnnotationElementValuePair),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnnotationArrayInitializerEntry {
    pub value: AnnotationElementValue,
    pub comma: Option<JavaSyntaxToken>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnnotationArgumentListEntry {
    pub argument: AnnotationArgument,
    pub comma: Option<JavaSyntaxToken>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CompilationUnitItem {
    Package(PackageDeclaration),
    Import(ImportDeclaration),
    Module(ModuleDeclaration),
    Type(TypeDeclaration),
    EmptyDeclaration(EmptyDeclaration),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ImportKind {
    SingleType(NameSyntax),
    TypeOnDemand(NameSyntax),
    SingleStatic(NameSyntax),
    StaticOnDemand(NameSyntax),
    SingleModule(NameSyntax),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ModuleDirectiveRole {
    Requires {
        module: NameSyntax,
        is_static: bool,
        is_transitive: bool,
    },
    Exports {
        package: NameSyntax,
        targets: Vec<NameSyntax>,
    },
    Opens {
        package: NameSyntax,
        targets: Vec<NameSyntax>,
    },
    Uses {
        service: NameSyntax,
    },
    Provides {
        service: NameSyntax,
        implementations: Vec<NameSyntax>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModuleNameListEntry {
    pub name: NameSyntax,
    pub comma: Option<JavaSyntaxToken>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StatementBody {
    Block(Block),
    Empty(EmptyStatement),
    Unbraced(Statement),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WildcardBound {
    Extends(Type),
    Super(Type),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemberChain {
    root: Expression,
    suffixes: Vec<MemberChainSuffix>,
}

impl MemberChain {
    #[must_use]
    pub fn root(&self) -> &Expression {
        &self.root
    }

    #[must_use]
    pub fn suffixes(&self) -> &[MemberChainSuffix] {
        &self.suffixes
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MemberChainSuffix {
    FieldAccess(FieldAccessExpression),
    MethodInvocation(MethodInvocationExpression),
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
pub struct ArgumentListEntry {
    pub argument: Expression,
    pub comma: Option<JavaSyntaxToken>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StatementExpressionEntry {
    pub expression: Expression,
    pub comma: Option<JavaSyntaxToken>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceListEntry {
    pub resource: Resource,
    pub separator: Option<JavaSyntaxToken>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArrayInitializerEntry {
    pub value: VariableInitializerValue,
    pub comma: Option<JavaSyntaxToken>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnionTypeEntry {
    pub ty: Type,
    pub separator: Option<JavaSyntaxToken>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IntersectionTypeEntry {
    pub ty: Type,
    pub separator: Option<JavaSyntaxToken>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypeArgumentListEntry {
    pub argument: TypeArgument,
    pub comma: Option<JavaSyntaxToken>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypeParameterListEntry {
    pub parameter: TypeParameter,
    pub comma: Option<JavaSyntaxToken>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FormalParameterListEntry {
    pub item: FormalParameterListItem,
    pub comma: Option<JavaSyntaxToken>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FormalParameterListItem {
    ReceiverParameter(ReceiverParameter),
    FormalParameter(FormalParameter),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecordComponentListEntry {
    pub component: RecordComponent,
    pub comma: Option<JavaSyntaxToken>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnumConstantListEntry {
    pub constant: EnumConstant,
    pub comma: Option<JavaSyntaxToken>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecordPatternComponentEntry {
    pub component: ComponentPattern,
    pub comma: Option<JavaSyntaxToken>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ThrowsClauseEntry {
    pub exception: Type,
    pub comma: Option<JavaSyntaxToken>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypeClauseEntry {
    pub ty: Type,
    pub comma: Option<JavaSyntaxToken>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PermitsClauseEntry {
    pub name: NameSyntax,
    pub comma: Option<JavaSyntaxToken>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClassTypeSegment {
    pub annotations: Vec<Annotation>,
    pub dot_before: Option<JavaSyntaxToken>,
    pub name: NameSyntax,
    pub type_arguments: Option<TypeArgumentList>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NameSegment {
    pub annotations: Vec<Annotation>,
    pub dot_before: Option<JavaSyntaxToken>,
    pub identifier: JavaSyntaxToken,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModifierEntry {
    pub tokens: Vec<JavaSyntaxToken>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VariableDeclaratorEntry {
    pub declarator: VariableDeclarator,
    pub comma: Option<JavaSyntaxToken>,
}

impl AnnotationArgument {
    fn cast(syntax: JavaSyntaxNode) -> Option<Self> {
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
pub enum SwitchBlockEntry {
    StatementGroup(SwitchBlockStatementGroup),
    Rule(SwitchRule),
}

pub(crate) fn cast_compilation_unit(syntax: JavaSyntaxNode) -> Option<CompilationUnit> {
    <CompilationUnit as JavaNode>::cast(syntax)
}

fn child<N: JavaNode>(syntax: &JavaSyntaxNode) -> Option<N> {
    syntax.children().find_map(N::cast)
}

fn children<'a, N: JavaNode + 'a>(syntax: &'a JavaSyntaxNode) -> impl Iterator<Item = N> + 'a {
    syntax.children().filter_map(N::cast)
}

fn tokens(syntax: &JavaSyntaxNode) -> Vec<JavaSyntaxToken> {
    let mut tokens = Vec::new();
    collect_tokens(syntax, &mut tokens);
    tokens
}

fn starts_after_blank_line(syntax: &JavaSyntaxNode) -> bool {
    tokens(syntax)
        .first()
        .is_some_and(JavaSyntaxToken::has_leading_blank_line)
}

fn collect_tokens(syntax: &JavaSyntaxNode, tokens: &mut Vec<JavaSyntaxToken>) {
    for element in syntax.children_with_tokens() {
        match element {
            SyntaxElement::Node(node) => collect_tokens(&node, tokens),
            SyntaxElement::Token(syntax) => tokens.push(JavaSyntaxToken { syntax }),
        }
    }
}

fn child_family<F: JavaFamily>(syntax: &JavaSyntaxNode) -> Option<F> {
    syntax.children().find_map(F::cast)
}

fn nth_child_family<F: JavaFamily>(syntax: &JavaSyntaxNode, index: usize) -> Option<F> {
    children_family(syntax).nth(index)
}

fn children_family<'a, F: JavaFamily + 'a>(
    syntax: &'a JavaSyntaxNode,
) -> impl Iterator<Item = F> + 'a {
    syntax.children().filter_map(F::cast)
}

fn child_token(syntax: &JavaSyntaxNode, kind: JavaSyntaxKind) -> Option<JavaSyntaxToken> {
    nth_child_token(syntax, kind, 0)
}

fn nth_child_token(
    syntax: &JavaSyntaxNode,
    kind: JavaSyntaxKind,
    index: usize,
) -> Option<JavaSyntaxToken> {
    syntax
        .child_tokens()
        .filter(|token| token.kind() == kind)
        .nth(index)
        .map(|syntax| JavaSyntaxToken { syntax })
}

fn child_token_in(syntax: &JavaSyntaxNode, kinds: &[JavaSyntaxKind]) -> Option<JavaSyntaxToken> {
    syntax
        .child_tokens()
        .find(|token| kinds.contains(&token.kind()))
        .map(|syntax| JavaSyntaxToken { syntax })
}

fn children_tokens_matching<'a>(
    syntax: &'a JavaSyntaxNode,
    predicate: impl Fn(JavaSyntaxKind) -> bool + Copy + 'a,
) -> impl Iterator<Item = JavaSyntaxToken> + 'a {
    syntax
        .child_tokens()
        .filter(move |token| predicate(token.kind()))
        .map(|syntax| JavaSyntaxToken { syntax })
}

#[cfg(test)]
fn test_syntax(kind: JavaSyntaxKind) -> JavaSyntaxNode {
    let green = jolt_syntax::GreenNode::new(kind.to_raw(), []);
    JavaSyntaxNode::new_root(green)
}

#[cfg(test)]
fn wrapper_expected_kind_name(wrapper: &str) -> &str {
    match wrapper {
        "ModuleDirectiveNode" => "ModuleDirective",
        _ => wrapper,
    }
}

#[cfg(test)]
mod tests;
