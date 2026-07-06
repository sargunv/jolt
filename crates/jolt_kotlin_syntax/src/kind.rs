use jolt_syntax::RawSyntaxKind;
use num_enum::{IntoPrimitive, TryFromPrimitive};

/// A Kotlin token or syntax node kind.
///
/// The token inventory follows the Kotlin compiler's `KtTokens` split between
/// hard keywords, soft keywords, modifier keywords, string-template tokens, and
/// punctuation/operator tokens. Parser-created CST nodes will share this same
/// kind space once the grammar is implemented.
#[repr(u16)]
#[derive(Clone, Copy, Debug, Eq, Hash, IntoPrimitive, PartialEq, TryFromPrimitive)]
#[allow(clippy::enum_variant_names)]
pub enum KotlinSyntaxKind {
    /// End-of-file marker.
    Eof,
    /// Unknown token emitted for lexer recovery.
    Unknown,
    /// Reserved token spelling, such as `...`.
    Reserved,

    // Literals and identifiers.
    Identifier,
    FieldIdentifier,
    IntegerLiteral,
    FloatLiteral,
    CharacterLiteral,

    // String-template tokens.
    InterpolationPrefix,
    OpenQuote,
    ClosingQuote,
    RegularStringPart,
    EscapeSequence,
    ShortTemplateEntryStart,
    LongTemplateEntryStart,
    LongTemplateEntryEnd,
    DanglingNewline,

    // Hard keywords.
    PackageKw,
    AsKw,
    TypeAliasKw,
    ClassKw,
    ThisKw,
    SuperKw,
    ValKw,
    VarKw,
    FunKw,
    ForKw,
    NullKw,
    TrueKw,
    FalseKw,
    IsKw,
    InKw,
    ThrowKw,
    ReturnKw,
    BreakKw,
    ContinueKw,
    ObjectKw,
    IfKw,
    TryKw,
    ElseKw,
    WhileKw,
    DoKw,
    WhenKw,
    InterfaceKw,
    TypeOfKw,
    AsSafe,

    // Soft keywords.
    AllKw,
    FileKw,
    FieldKw,
    PropertyKw,
    ReceiverKw,
    ParamKw,
    SetParamKw,
    DelegateKw,
    ImportKw,
    WhereKw,
    ByKw,
    GetKw,
    SetKw,
    ConstructorKw,
    InitKw,
    ContextKw,
    CatchKw,
    DynamicKw,
    FinallyKw,

    // Modifier keywords.
    AbstractKw,
    EnumKw,
    ContractKw,
    OpenKw,
    InnerKw,
    OverrideKw,
    PrivateKw,
    PublicKw,
    InternalKw,
    ProtectedKw,
    OutKw,
    VarargKw,
    ReifiedKw,
    CompanionKw,
    SealedKw,
    FinalKw,
    LateinitKw,
    DataKw,
    ValueKw,
    InlineKw,
    NoinlineKw,
    TailrecKw,
    ExternalKw,
    AnnotationKw,
    CrossinlineKw,
    OperatorKw,
    InfixKw,
    ConstKw,
    SuspendKw,
    ExpectKw,
    ActualKw,

    // Delimiters and punctuation.
    LBracket,
    RBracket,
    LBrace,
    RBrace,
    LParen,
    RParen,
    Dot,
    Question,
    ColonColon,
    Colon,
    Semicolon,
    DoubleSemicolon,
    Range,
    RangeUntil,
    Assign,
    Hash,
    At,
    Comma,
    EolOrSemicolon,

    // Operators.
    PlusPlus,
    MinusMinus,
    Star,
    Plus,
    Minus,
    Bang,
    Slash,
    Percent,
    Lt,
    Gt,
    LtEq,
    GtEq,
    EqEqEq,
    Arrow,
    DoubleArrow,
    BangEqEqEq,
    EqEq,
    BangEq,
    BangBang,
    AndAnd,
    Amp,
    OrOr,
    SafeAccess,
    Elvis,
    StarEq,
    SlashEq,
    PercentEq,
    PlusEq,
    MinusEq,
    NotIn,
    NotIs,

    // Nodes.
    ErrorNode,
    KotlinFile,
}

impl KotlinSyntaxKind {
    /// Converts this kind into the raw representation used by shared syntax data.
    #[must_use]
    pub fn to_raw(self) -> RawSyntaxKind {
        RawSyntaxKind::new(u16::from(self))
    }

    /// Converts a raw kind back into a Kotlin syntax kind.
    #[must_use]
    pub fn from_raw(raw: RawSyntaxKind) -> Option<Self> {
        Self::try_from(raw.get()).ok()
    }
}
