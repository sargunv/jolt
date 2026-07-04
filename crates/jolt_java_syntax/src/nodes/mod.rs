use std::fmt;

use jolt_syntax::{SyntaxNode, SyntaxToken, TriviaKind as SyntaxTriviaKind, green_text};
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
    pub(crate) fn has_leading_blank_line(&self) -> bool {
        trivia_has_blank_line(self.syntax.leading())
    }
}

impl fmt::Debug for JavaSyntaxToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.syntax.fmt(f)
    }
}

/// A Java operator, which may span multiple syntax tokens in ambiguous `>` forms.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JavaOperator {
    kind: JavaOperatorKind,
    first_token: JavaSyntaxToken,
    last_token: Option<JavaSyntaxToken>,
}

impl JavaOperator {
    pub(crate) fn single(kind: JavaOperatorKind, token: JavaSyntaxToken) -> Self {
        Self {
            kind,
            first_token: token,
            last_token: None,
        }
    }

    pub(crate) fn composite(
        kind: JavaOperatorKind,
        first_token: JavaSyntaxToken,
        last_token: JavaSyntaxToken,
    ) -> Self {
        Self {
            kind,
            first_token,
            last_token: Some(last_token),
        }
    }

    #[must_use]
    pub fn text(&self) -> &'static str {
        self.kind.text()
    }

    #[must_use]
    pub fn leading_comments(&self) -> Vec<JavaComment> {
        self.first_token.leading_comments()
    }

    #[must_use]
    pub fn trailing_comments(&self) -> Vec<JavaComment> {
        self.last_token().trailing_comments()
    }

    #[must_use]
    pub fn as_single_token(&self) -> Option<&JavaSyntaxToken> {
        if self.last_token.is_none() {
            Some(&self.first_token)
        } else {
            None
        }
    }

    fn last_token(&self) -> &JavaSyntaxToken {
        self.last_token.as_ref().unwrap_or(&self.first_token)
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
    pub const fn text(self) -> &'static str {
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

                pub fn token_iter(&self) -> impl Iterator<Item = JavaSyntaxToken> + '_ {
                    token_iter(&self.syntax)
                }

                #[must_use]
                pub fn first_token(&self) -> Option<JavaSyntaxToken> {
                    first_token(&self.syntax)
                }

                #[must_use]
                pub fn last_token(&self) -> Option<JavaSyntaxToken> {
                    last_token(&self.syntax)
                }

            }

            impl private::Sealed for $node {}

            impl JavaNode for $node {
                fn cast(syntax: JavaSyntaxNode) -> Option<Self> {
                    matches!(syntax.kind(), JavaSyntaxKind::$kind).then_some(Self { syntax })
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

                pub fn token_iter(&self) -> impl Iterator<Item = JavaSyntaxToken> + '_ {
                    token_iter(self.syntax())
                }

                #[must_use]
                pub fn first_token(&self) -> Option<JavaSyntaxToken> {
                    first_token(self.syntax())
                }

                #[must_use]
                pub fn last_token(&self) -> Option<JavaSyntaxToken> {
                    last_token(self.syntax())
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

impl CompilationUnitItem {
    #[must_use]
    pub fn first_token(&self) -> Option<JavaSyntaxToken> {
        match self {
            Self::Package(item) => item.first_token(),
            Self::Import(item) => item.first_token(),
            Self::Module(item) => item.first_token(),
            Self::Type(item) => item.first_token(),
            Self::EmptyDeclaration(item) => item.first_token(),
        }
    }

    #[must_use]
    pub fn last_token(&self) -> Option<JavaSyntaxToken> {
        match self {
            Self::Package(item) => item.last_token(),
            Self::Import(item) => item.last_token(),
            Self::Module(item) => item.last_token(),
            Self::Type(item) => item.last_token(),
            Self::EmptyDeclaration(item) => item.last_token(),
        }
    }
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
    tokens: [Option<JavaSyntaxToken>; 3],
    len: usize,
}

impl ModifierEntry {
    pub(crate) fn single(token: JavaSyntaxToken) -> Self {
        Self {
            tokens: [Some(token), None, None],
            len: 1,
        }
    }

    pub(crate) fn non_sealed(
        non: JavaSyntaxToken,
        minus: JavaSyntaxToken,
        sealed: JavaSyntaxToken,
    ) -> Self {
        Self {
            tokens: [Some(non), Some(minus), Some(sealed)],
            len: 3,
        }
    }

    pub fn tokens(&self) -> impl Iterator<Item = &JavaSyntaxToken> {
        self.tokens[..self.len].iter().filter_map(Option::as_ref)
    }

    pub fn into_tokens(self) -> impl Iterator<Item = JavaSyntaxToken> {
        self.tokens.into_iter().take(self.len).flatten()
    }

    pub fn first_token(&self) -> Option<&JavaSyntaxToken> {
        self.tokens.first().and_then(Option::as_ref)
    }
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

fn token_iter(syntax: &JavaSyntaxNode) -> impl Iterator<Item = JavaSyntaxToken> + '_ {
    syntax.tokens().map(|syntax| JavaSyntaxToken { syntax })
}

fn first_token(syntax: &JavaSyntaxNode) -> Option<JavaSyntaxToken> {
    syntax
        .first_token()
        .map(|syntax| JavaSyntaxToken { syntax })
}

fn last_token(syntax: &JavaSyntaxNode) -> Option<JavaSyntaxToken> {
    syntax.last_token().map(|syntax| JavaSyntaxToken { syntax })
}

fn starts_after_blank_line(syntax: &JavaSyntaxNode) -> bool {
    first_token(syntax).is_some_and(|token| token.has_leading_blank_line())
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
mod tests;
