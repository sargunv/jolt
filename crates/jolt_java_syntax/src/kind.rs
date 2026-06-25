/// A Java token or syntax node kind.
///
/// This enum is language-wide: lexer tokens and parser-created CST nodes share
/// the same kind space, while the green tree stores whether an element is a
/// token or node structurally.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[allow(clippy::enum_variant_names)]
pub enum JavaSyntaxKind {
    // Tokens.
    //
    // Token variants come from JLS Chapter 3 lexical structure: identifiers,
    // literals, reserved keywords, separators, and operators. Contextual
    // keywords remain Identifier tokens until parser context interprets them.
    Eof,
    Unknown,
    Identifier,

    IntegerLiteral,
    FloatingPointLiteral,
    BooleanLiteral,
    CharacterLiteral,
    StringLiteral,
    TextBlockLiteral,
    NullLiteral,

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
}
