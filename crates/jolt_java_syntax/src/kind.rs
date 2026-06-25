use jolt_syntax::RawSyntaxKind;
use num_enum::{IntoPrimitive, TryFromPrimitive};

/// A Java token or syntax node kind.
///
/// This enum is language-wide: lexer tokens and parser-created CST nodes share
/// the same kind space, while the green tree stores whether an element is a
/// token or node structurally.
#[repr(u16)]
#[derive(Clone, Copy, Debug, Eq, Hash, IntoPrimitive, PartialEq, TryFromPrimitive)]
#[allow(clippy::enum_variant_names)]
pub enum JavaSyntaxKind {
    /// End-of-file marker.
    Eof,
    /// Unknown token emitted for lexer recovery.
    Unknown,
    /// Identifier token, including contextual keyword spellings until parsed in context.
    Identifier,

    // Literals.
    IntegerLiteral,
    FloatingPointLiteral,
    BooleanLiteral,
    CharacterLiteral,
    StringLiteral,
    TextBlockLiteral,
    NullLiteral,

    // Reserved keywords.
    AbstractKw,
    AssertKw,
    BooleanKw,
    BreakKw,
    ByteKw,
    CaseKw,
    CatchKw,
    CharKw,
    ClassKw,
    ConstKw,
    ContinueKw,
    DefaultKw,
    DoKw,
    DoubleKw,
    ElseKw,
    EnumKw,
    ExtendsKw,
    FinalKw,
    FinallyKw,
    FloatKw,
    ForKw,
    GotoKw,
    IfKw,
    ImplementsKw,
    ImportKw,
    InstanceofKw,
    IntKw,
    InterfaceKw,
    LongKw,
    NativeKw,
    NewKw,
    PackageKw,
    PrivateKw,
    ProtectedKw,
    PublicKw,
    ReturnKw,
    ShortKw,
    StaticKw,
    StrictfpKw,
    SuperKw,
    SwitchKw,
    SynchronizedKw,
    ThisKw,
    ThrowKw,
    ThrowsKw,
    TransientKw,
    TryKw,
    VoidKw,
    VolatileKw,
    WhileKw,
    UnderscoreKw,

    // Separators.
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Semicolon,
    Comma,
    Dot,
    Ellipsis,
    At,
    Colon,
    DoubleColon,

    // Operators.
    Assign,
    Gt,
    GtEq,
    Lt,
    Bang,
    Tilde,
    Question,
    Arrow,
    EqEq,
    LtEq,
    BangEq,
    AndAnd,
    OrOr,
    PlusPlus,
    MinusMinus,
    Plus,
    Minus,
    Star,
    Slash,
    Amp,
    Bar,
    Caret,
    Percent,
    LShift,
    RShift,
    UnsignedRShift,
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

    // Nodes.
    //
    // Contributor note: derive node variants from the Java syntax grammar, but
    // do not add them as a blind one-to-one copy of every grammar production.
    //
    // - Add a node when formatting, recovery, traversal, or a parser ambiguity
    //   benefits from preserving that boundary.
    // - Fold grammar helpers into their parent when they only express EBNF
    //   shape and do not carry useful source structure on their own.
    // - Account for every JLS syntax production in the parser coverage matrix
    //   as represented by a node, folded into another node, or handled as token
    //   or contextual-keyword behavior.
    ErrorNode,

    CompilationUnit,
    PackageDeclaration,
    ImportDeclaration,

    ModuleDeclaration,
    ModuleDirective,
    RequiresDirective,
    ExportsDirective,
    OpensDirective,
    UsesDirective,
    ProvidesDirective,

    ModifierList,
    Annotation,
    AnnotationArgumentList,
    AnnotationElement,
    AnnotationElementList,
    AnnotationArrayInitializer,
    DefaultValue,

    ClassDeclaration,
    RecordDeclaration,
    EnumDeclaration,
    InterfaceDeclaration,
    AnnotationInterfaceDeclaration,
    TypeParameterList,
    TypeParameter,
    TypeBoundList,
    ExtendsClause,
    ImplementsClause,
    PermitsClause,
    ClassBody,
    ClassBodyDeclaration,
    EmptyDeclaration,
    RecordBody,
    InterfaceBody,
    AnnotationInterfaceBody,
    EnumBody,
    EnumConstantList,
    EnumConstant,
    RecordComponentList,
    RecordComponent,
    FieldDeclaration,
    MethodDeclaration,
    ConstructorDeclaration,
    ConstructorInvocation,
    CompactConstructorDeclaration,
    StaticInitializer,
    InstanceInitializer,

    FormalParameterList,
    FormalParameter,
    ReceiverParameter,
    ThrowsClause,
    VariableDeclaratorList,
    VariableDeclarator,
    VariableInitializer,

    Block,
    BlockStatement,
    LocalVariableDeclaration,
    LocalClassOrInterfaceDeclaration,
    EmptyStatement,
    LabeledStatement,
    ExpressionStatement,
    IfStatement,
    AssertStatement,
    SwitchStatement,
    SwitchBlock,
    SwitchBlockStatementGroup,
    SwitchRule,
    SwitchLabel,
    Guard,
    WhileStatement,
    DoStatement,
    ForStatement,
    BasicForStatement,
    EnhancedForStatement,
    ForInitializer,
    ForUpdate,
    StatementExpressionList,
    BreakStatement,
    YieldStatement,
    ContinueStatement,
    ReturnStatement,
    ThrowStatement,
    SynchronizedStatement,
    TryStatement,
    TryWithResourcesStatement,
    CatchClause,
    CatchTypeList,
    FinallyClause,
    ResourceList,
    Resource,
    VariableAccess,

    PrimitiveType,
    ClassType,
    ArrayType,
    TypeArgumentList,
    TypeArgument,
    WildcardType,
    ArrayDimensions,

    Name,
    QualifiedName,

    LiteralExpression,
    NameExpression,
    ThisExpression,
    SuperExpression,
    ParenthesizedExpression,
    ClassLiteralExpression,
    FieldAccessExpression,
    ArrayAccessExpression,
    MethodInvocationExpression,
    MethodReferenceExpression,
    ObjectCreationExpression,
    ArrayCreationExpression,
    DimExpression,
    ArrayInitializer,
    AssignmentExpression,
    ConditionalExpression,
    BinaryExpression,
    UnaryExpression,
    PostfixExpression,
    CastExpression,
    LambdaExpression,
    LambdaParameterList,
    LambdaParameter,
    SwitchExpression,
    ArgumentList,

    TypePattern,
    RecordPattern,
    ComponentPattern,
    MatchAllPattern,
}

impl JavaSyntaxKind {
    /// Converts this kind into the raw representation used by shared syntax data.
    #[must_use]
    pub fn to_raw(self) -> RawSyntaxKind {
        RawSyntaxKind::new(u16::from(self))
    }

    /// Converts a raw kind back into a Java syntax kind.
    #[must_use]
    pub fn from_raw(raw: RawSyntaxKind) -> Option<Self> {
        Self::try_from(raw.get()).ok()
    }
}
