# Formatter architecture

This page is the design contract for Jolt's formatter. It describes the target
architecture the rewrite builds toward: the crates, their public APIs, the core
data structures and algorithms with their costs, what is deleted, and how the
work is staged. The Java output it must produce is specified separately in
[Java style](./java-style.md).

Where this page and an implementation disagree, the implementation is wrong or
this page must be amended in the same change. Where this page explicitly
supersedes a project invariant in `AGENTS.md`, the section says so.

## Goals

- Format Java correctly: reparse cleanly, preserve structure (modulo the listed
  normalizations), emit every comment exactly once, and reach a fixpoint in one
  pass. The corpus sweep (`mise run hunt`) runs with an empty `KNOWN_OPEN`.
- Produce the current Java style byte for byte, except for the changes listed
  under "Deliberate changes" in [Java style](./java-style.md).
- Dramatically less code and machinery. The target is about half of today's ~48k
  lines of hand-written Rust, with the formatter core under 2.5k lines.
- Be no slower than the reference binary (see [Performance](#performance)).
- Stay multi-language-capable: tree, parser infrastructure, document IR,
  printer, comment placement and formatter-ignore are language-agnostic crates.
  Only Java is built.

Non-goals: proving source conservation at runtime, normalization "authority"
types, diagnostic ownership, configurable style.

## Overview

```text
source ──lex──▶ tokens+trivia ──parse──▶ events ──build──▶ SyntaxTree (fixed slots)
                                                              │
                                     comment placement pass ◀─┤
                                     formatter-ignore regions ◀┘
                                                              │
                     Formatter::node (one chokepoint) ──rules──▶ Doc arena
                                                              │
                                              printer ──chunks──▶ Sink
```

Every phase is a single linear pass, except two explicitly bounded searches: the
printer's fit check (at most `FIT_BUDGET` commands per group) and Java's
parenthesis/annotation lookahead table (built once, linear).

## Crates

| Crate               | Kind     | Owns                                                                                                                                                                     |
| ------------------- | -------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `jolt_syntax`       | agnostic | Text sizes and ranges, line index, `Diagnostic`, the lossless arena tree and its builder, borrowed cursors, trivia, parser infrastructure (markers, guards, token sets). |
| `jolt_fmt`          | agnostic | `FormatOptions`, `Sink`, the document IR, the printer, comment placement and printing, verbatim emission, item lists with formatter-ignore, the `Formatter` context.     |
| `jolt_java_syntax`  | Java     | Lexer, recursive-descent parser, the Java schema and the code generated from it (kinds, factory, typed nodes, accessors).                                                |
| `jolt_java_fmt`     | Java     | The Java `FormatLanguage` (tables and comment hooks), node dispatch, helpers, and the layout rules.                                                                      |
| `jolt_formatter`    | facade   | Language detection and the single API the CLI and dprint plugin call.                                                                                                    |
| `jolt_test_support` | tests    | Fixture collection, the oracle (structure fingerprint, comment multiset, idempotence, reparse), and the corpus sweep with comment probes.                                |
| `jolt_cli`          | app      | The `jolt` binary (periphery).                                                                                                                                           |
| `jolt_fmt_dprint`   | app      | The dprint WASM plugin (periphery).                                                                                                                                      |

Dependency graph (arrows point at dependencies):

```text
jolt_cli ─┐
          ├─▶ jolt_formatter ─▶ jolt_java_fmt ─▶ jolt_fmt ─▶ jolt_syntax
jolt_fmt_dprint ┘                   │                            ▲
                                    └────▶ jolt_java_syntax ─────┘
jolt_test_support ─▶ jolt_fmt, jolt_syntax
```

Decisions:

- `jolt_text` and `jolt_diagnostics` are folded into `jolt_syntax` (`text.rs`,
  `diagnostic.rs`). Both were under 200 lines and every consumer already
  depended on `jolt_syntax`.
- `jolt_fmt_ir` is replaced by `jolt_fmt`. The new crate holds the whole
  language-agnostic formatter core, not only an IR, and none of its old API
  survives.
- `jolt_formatter` stays as a tiny facade. It is the only place that knows the
  set of languages, and the stable boundary for the CLI and dprint, so folding
  it into a language crate would invert the dependency direction.
- The Java schema produces generated Rust source (checked in) instead of layered
  `macro_rules!`. Generated code is greppable, has real rustdoc, and replaces
  ~3k lines of projection/accessor/factory macros plus every hand-written
  `classify`/`as_*`/`partitioned_items` function.

## `jolt_syntax`

### Text and diagnostics

```rust
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TextSize(u32); // byte offset; sources are < 4 GiB
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct TextRange {
    start: TextSize,
    end: TextSize,
}
pub struct LineIndex {/* line starts */}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LineCol {
    pub line: u32,
    pub col: u32,
} // zero-based, col in bytes

impl LineIndex {
    pub fn new(source: &str) -> Self;
    pub fn line_col(&self, offset: TextSize) -> LineCol;
}

/// A lexer or parser error. There is no severity, stage, or owner: every
/// diagnostic is a syntax error at a source range.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: &'static str, // stable, e.g. "java.parse.expected_syntax"
    pub message: String,
    pub range: TextRange,
}
```

### Kinds and languages

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RawSyntaxKind(pub u16);

/// Static facts about one language's tree. Implemented by a unit struct
/// (`JavaLanguage`) that derives `Clone, Copy, Debug, PartialEq, Eq, Hash`.
pub trait Language: Copy + Eq + std::fmt::Debug + 'static {
    type Kind: Copy + Eq + std::fmt::Debug;
    const EOF: Self::Kind;
    /// The single bogus node kind, accepted at every category slot and list
    /// item position.
    const BOGUS: Self::Kind;
    fn to_raw(kind: Self::Kind) -> RawSyntaxKind;
    fn from_raw(raw: RawSyntaxKind) -> Self::Kind;
    /// List nodes (variable-length children). Comment placement never
    /// attaches to a list node, only to its items.
    fn is_list(raw: RawSyntaxKind) -> bool;
    fn kind_name(raw: RawSyntaxKind) -> &'static str;
}
```

### Tree storage

The tree is one flat arena per parse, as today: nodes, a shared slot vector,
tokens, and trivia, all with `u32` indices. A node stores its kind, flags, its
slot range, its token range, its parent and its index in the parent. Slots are
fixed per node kind (see the factory below); an empty slot is a valid absent
optional or a missing required element.

Flags, two bits, identical in debug and release builds:

- `MALFORMED`: the node is the bogus kind, or its children did not fit its
  kind's shape and are stored raw.
- `HAS_RECOVERY`: the node or any descendant is malformed or has a missing
  required slot. Propagated to ancestors during the build.

There are no debug-only fields anywhere in the tree, cursors, comments, or
document arena. A struct has one layout in every profile.

### Cursors

Cursors borrow the tree and the source. They are not generic over the language;
language crates wrap them in typed nodes.

```rust
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(u32);
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TokenId(u32); // source order

// Equality is identity: (tree pointer, id). Implemented once on a private
// `TreeRef<'a>(&'a SyntaxTree)` newtype so the cursors themselves can derive.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct SyntaxNode<'a> {
    tree: TreeRef<'a>,
    source: &'a str,
    id: NodeId,
}
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct SyntaxToken<'a> {
    tree: TreeRef<'a>,
    source: &'a str,
    id: TokenId,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SyntaxElement<'a> {
    Node(SyntaxNode<'a>),
    Token(SyntaxToken<'a>),
}

impl<'a> SyntaxNode<'a> {
    pub fn kind(self) -> RawSyntaxKind;
    pub fn id(self) -> NodeId;
    pub fn parent(self) -> Option<SyntaxNode<'a>>;
    /// Typed-accessor slot read. Returns `None` for an empty slot, and for
    /// every slot of a malformed node (whose children are raw).
    pub fn slot(self, index: usize) -> Option<SyntaxElement<'a>>;
    /// Raw non-empty children in order, malformed or not.
    pub fn children(self) -> impl Iterator<Item = SyntaxElement<'a>> + 'a;
    pub fn first_token(self) -> Option<SyntaxToken<'a>>; // O(1): token range
    pub fn last_token(self) -> Option<SyntaxToken<'a>>; // O(1)
    pub fn tokens(self) -> impl Iterator<Item = SyntaxToken<'a>> + 'a;
    /// From the first token's text start to the last token's text end
    /// (outer trivia excluded). Empty nodes give an empty range at the next
    /// token's text start.
    pub fn text_range(self) -> TextRange;
    pub fn is_malformed(self) -> bool;
    pub fn is_recovery_free(self) -> bool;
    pub fn source(self) -> &'a str;
    /// Iterative preorder walk (no recursion; deep trees are fine).
    pub fn descendants(self) -> impl Iterator<Item = SyntaxElement<'a>> + 'a;
}

impl<'a> SyntaxToken<'a> {
    pub fn kind(self) -> RawSyntaxKind;
    pub fn id(self) -> TokenId;
    pub fn parent(self) -> SyntaxNode<'a>;
    pub fn text(self) -> &'a str;
    pub fn text_range(self) -> TextRange;
    pub fn leading(self) -> impl Iterator<Item = TriviaPiece<'a>> + 'a;
    pub fn trailing(self) -> impl Iterator<Item = TriviaPiece<'a>> + 'a;
    pub fn prev(self) -> Option<SyntaxToken<'a>>; // O(1): flat token vector
    pub fn next(self) -> Option<SyntaxToken<'a>>;
}

/// Hand-written once: prints `Kind@start..end`.
impl std::fmt::Debug for SyntaxNode<'_> {
    /* … */
}
impl std::fmt::Debug for SyntaxToken<'_> {
    /* … */
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TriviaKind {
    Whitespace,
    Newline,
    LineComment,
    BlockComment,
    DocComment,
    UnterminatedBlockComment,
    ByteOrderMark,
    TrailingSubstitute,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TriviaPiece<'a> {
    pub kind: TriviaKind,
    pub text: &'a str,
    pub range: TextRange,
}

pub trait AstNode<'a>: Copy {
    fn can_cast(kind: RawSyntaxKind) -> bool;
    fn cast(node: SyntaxNode<'a>) -> Option<Self>;
    fn syntax(self) -> SyntaxNode<'a>;
}
```

Trivia attachment is unchanged: a token's trailing trivia runs to (not
including) the next line break; everything else is the following token's leading
trivia. The final `Eof` token carries the file's trailing trivia.

### Parser infrastructure

The parser API keeps the event/marker design (`start`, `complete`, `precede`,
`abandon`, forward parents) and drops everything related to diagnostic
ownership: there are no `NodeAnchor`/slot arguments, `PendingDiagnostic`,
`missing_required_slot`, `complete_recovery`, or owner resolution. Missing
required slots are discovered by the factory from the children, exactly as
today; the parser only reports the diagnostic.

```rust
pub struct TokenSet(u128); // token kinds must fit in 0..128 (asserted by codegen)
impl TokenSet {
    pub const fn new(kinds: &[RawSyntaxKind]) -> Self;
    pub const fn contains(self, k: RawSyntaxKind) -> bool;
}

pub struct Parser<'s, L: Language> {/* token buffer, events, diagnostics, depth */}
impl<'s, L: Language> Parser<'s, L> {
    pub fn new(source: &'s str, lexer: impl Lexer<'s, L>) -> Self;
    pub fn current(&mut self) -> L::Kind;
    pub fn nth(&mut self, n: usize) -> L::Kind;
    pub fn nth_text(&mut self, n: usize) -> &'s str;
    pub fn at(&mut self, kind: L::Kind) -> bool;
    pub fn at_set(&mut self, set: TokenSet) -> bool;
    pub fn bump(&mut self);
    pub fn eat(&mut self, kind: L::Kind) -> bool;
    /// Eats `kind` or records `message` as an `expected` diagnostic at the
    /// current token. The factory turns the absent token into a missing slot.
    pub fn expect(&mut self, kind: L::Kind, message: &'static str) -> bool;
    pub fn error(&mut self, code: &'static str, message: impl Into<String>);
    pub fn start(&mut self) -> Marker;
    /// Replaces the current token in the buffer by a one-byte `first` token
    /// followed by `rest` covering the remaining bytes. Leading trivia stays
    /// on `first`, trailing trivia moves to `rest`. Used only by consuming
    /// grammar (never by lookahead) for Java's `>` family.
    pub fn split_current(&mut self, first: L::Kind, rest: L::Kind);
    pub fn loop_guard(&mut self) -> LoopGuard;
    /// Runs `parse` one recursion level deeper, or returns `None` without
    /// running it when `MAX_DEPTH` (128) levels are active.
    pub fn nested<T>(&mut self, parse: impl FnOnce(&mut Self) -> T) -> Option<T>;
    /// The one deep-nesting recovery. Records a "too deeply nested"
    /// diagnostic and wraps tokens into a single `BOGUS` node until, at
    /// bracket depth zero, a token in `stops` or an unmatched closing
    /// bracket, or EOF. Brackets are `()`, `[]`, `{}`; it never splits tokens.
    pub fn recover_too_deep(&mut self, stops: TokenSet) -> CompletedMarker;
    /// A forked read-only cursor for speculative lookahead.
    pub fn lookahead(&mut self) -> Lookahead<'_, 's, L>;
    pub fn finish(self) -> ParseOutput;
}

/// Release-active progress guard. Every grammar loop that calls back into
/// grammar functions holds one and calls `check` at the top of each
/// iteration.
pub struct LoopGuard {
    last: u32,
}
impl LoopGuard {
    /// Returns `false` at EOF (the loop must exit). If no token was consumed
    /// since the previous `check`, wraps the current token in a `BOGUS` node
    /// with an "unexpected token" diagnostic so the loop always advances.
    pub fn check<L: Language>(&mut self, p: &mut Parser<'_, L>) -> bool;
}

pub struct ParseOutput {
    pub events: Vec<Event>,
    pub tokens: Vec<TokenData>,
    pub trivia: Vec<Trivia>,
    pub diagnostics: Vec<Diagnostic>,
}
```

`Marker` keeps its drop check (a debug assertion that a marker was completed or
abandoned; it guards programmer error, not input).

Cost: the guard makes every loop advance by at least one token per iteration, so
a parse is `O(tokens)` iterations in release builds regardless of grammar bugs.
Recursion is bounded by `MAX_DEPTH` active `nested` calls; the value stays 128,
calibrated for the 1 MiB WASM stack.

### Tree building and the factory

```rust
#[doc(hidden)]
pub trait SyntaxFactory {
    /// Places `children` into the fixed slots of `kind`, or stores them raw and
    /// marks the node `MALFORMED` when they do not fit. Infallible.
    fn make(
        &self,
        kind: RawSyntaxKind,
        children: ParsedChildren<'_>,
        sink: &mut NodeSink<'_>,
    ) -> NodeId;
}
pub fn build_tree(
    source: &str,
    output: ParseOutput,
    factory: &impl SyntaxFactory,
) -> (SyntaxTree, Vec<Diagnostic>);
```

The builder is infallible: the marker API cannot produce an unbalanced stream,
so a malformed event stream is a parser bug and panics with a message.
`BuildSyntaxTreeError`, `FactoryMismatch`, and the "no syntax tree" formatter
diagnostic are deleted.

Factory rules (generated, see the Java schema):

1. A fixed node matches each child against its slots in order. A required slot
   with no matching child is left empty and the node gets `HAS_RECOVERY`.
2. A category slot accepts the category's members and the bogus kind.
3. A list accepts its item matcher and the bogus kind at every item position.
   Separated lists record missing items and separators as empty slots, exactly
   as today.
4. Any child that fits no slot makes the whole node `MALFORMED` (raw children,
   kind kept so the node still fits its parent's slot).

Because of rule 4, a typed accessor never needs a "malformed" variant: a field
is `Option<T>`, where `None` is absent-or-missing and `Some` is always a node of
the declared type (which may itself be malformed and is then formatted
verbatim).

## `jolt_java_syntax`

### Lexer

The lexer, including Unicode escape translation and range remapping, stays. One
change: `>` is lexed by maximal munch into `Gt`, `GtEq`, `RShift`, `RShiftEq`,
`UnsignedRShift`, `UnsignedRShiftEq` and is **never split eagerly**. Binary and
assignment operators are therefore single tokens, and the five composite
operator node kinds (`GreaterThanOrEqualOperator`, `RightShiftOperator`,
`UnsignedRightShiftOperator`, `RightShiftAssignmentOperator`,
`UnsignedRightShiftAssignmentOperator`), `JavaOperator`, `JavaOperatorKind`
components, and the `COMPOSITE_*` tables are deleted.

Where a type argument or type parameter list expects its closing `>` and the
current token is one of the compound kinds, the consuming parser calls
`split_current(Gt, remainder)` (`>>` → `>` `>`; `>>>` → `>` `>>`; `>=` → `>`
`=`; `>>=` → `>` `>=`; `>>>=` → `>` `>>=`) and eats the `>`. Speculative generic
lookahead runs on a forked cursor and counts the `>` characters of a compound
token without splitting. The consuming parser never rewinds, so a split is never
undone.

### Schema and generated code

One schema file, `crates/jolt_java_syntax/java.schema`, is the single source of
truth for kinds and tree shape. It keeps today's vocabulary (tokens; categories;
nodes with `required`/`optional`/`many`/`one_or_more` fields over `token`,
`token_set`, `contextual`, `node`, `node_set`, `category`, `list`, `choice`;
separated lists with separator, minimum and trailing policy). The 28
per-category bogus kinds collapse into one `Bogus` kind accepted wherever rule 2
or 3 applies; schema fields that named a specific bogus kind
(`BogusImportSuffix`, `BogusModifier`, `BogusSwitchLabelItem`) name `Bogus`.

`crates/jolt_java_syntax/tests/codegen.rs` parses the schema and renders
`crates/jolt_java_syntax/src/generated.rs` (checked in, `// @generated`). The
test fails with a diff when the file is stale and rewrites it when
`INSTA_UPDATE=always` (what `mise run test --update` sets). The generator is
plain Rust (~600 lines), not macros. It emits:

```rust
#[repr(u16)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum JavaSyntaxKind {
    Eof,
    Unknown,
    Identifier,
    /* …tokens… */ CompilationUnit,
    /* …nodes… */ Bogus,
}
impl JavaSyntaxKind {
    pub const fn is_token(self) -> bool;
    pub const fn is_list(self) -> bool;
    pub const fn name(self) -> &'static str;
    pub const fn to_raw(self) -> RawSyntaxKind;
    pub fn from_raw(raw: RawSyntaxKind) -> Self;
}

/// One newtype per node kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ClassDeclaration<'a>(SyntaxNode<'a>);
impl<'a> AstNode<'a> for ClassDeclaration<'a> {
    /* can_cast / cast / syntax */
}
impl<'a> ClassDeclaration<'a> {
    pub fn modifiers(self) -> Option<ModifierList<'a>>; // node / list slot
    pub fn class_keyword(self) -> Option<SyntaxToken<'a>>; // token slot
    pub fn name(self) -> Option<SyntaxToken<'a>>;
    pub fn type_parameters(self) -> Option<TypeParameterList<'a>>;
    pub fn extends(self) -> Option<ExtendsClause<'a>>;
    pub fn implements(self) -> Option<ImplementsClause<'a>>;
    pub fn permits(self) -> Option<PermitsClause<'a>>;
    pub fn body(self) -> Option<ClassBody<'a>>;
    pub fn missing_body_semicolon(self) -> Option<SyntaxToken<'a>>;
}

/// One enum per category; every category has a `Bogus` variant.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Expression<'a> {
    LiteralExpression(LiteralExpression<'a>),
    /* … */ Bogus(Bogus<'a>),
}
impl<'a> AstNode<'a> for Expression<'a> {
    /* … */
}

/// One enum per `choice` field (replaces hand-written `classify`/`as_*`).
/// Every node alternative is a variant; all token alternatives share `Token`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ModifierElement<'a> {
    Annotation(Annotation<'a>),
    NonSealedModifier(NonSealedModifier<'a>),
    Bogus(Bogus<'a>),
    Token(SyntaxToken<'a>),
}

/// Plain lists iterate present items; separated lists also expose elements.
impl<'a> BlockStatementList<'a> {
    pub fn items(self) -> impl Iterator<Item = BlockStatement<'a>> + 'a;
}
impl<'a> ExpressionList<'a> {
    pub fn items(self) -> impl Iterator<Item = Expression<'a>> + 'a;
    pub fn elements(self) -> impl Iterator<Item = ListElement<'a, Expression<'a>>> + 'a;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ListElement<'a, T> {
    Item(T),
    Separator(SyntaxToken<'a>),
    Missing,
}

pub(crate) struct JavaSyntaxFactory; // impl SyntaxFactory, one match arm per kind
```

Semantic helpers that are not schema projections stay hand-written in
`jolt_java_syntax/src/semantics.rs` and must carry real behavior: operator
precedence by token kind, contextual keyword tests, modifier partitioning
(declaration vs type-use annotations). Nothing in it re-derives a schema choice.

### Parser rules

- Every `while`/`loop` in `parser/grammar/**` that calls a grammar function
  holds a `LoopGuard`. Review checklist item for the syntax step: search the
  parser for every `loop` and `while` and justify each loop without a guard
  (pure token scans that always bump).
- Recursive grammar owners use `nested`; on `None` they call `recover_too_deep`
  with the stop set of their position (expression: `; , ) ] }`; statement: `}`;
  type: `> , ) ] {` plus the `>` compounds; body member: `}`). This single
  function replaces all 13 `parse_excessive_*` functions and their bespoke
  shapes; the result is always one `Bogus` node at a category or list position.
- Recovery has exactly three representations: an empty required slot (missing),
  a `Bogus` node at a category or list position (unexpected tokens), and a
  `MALFORMED` node whose children did not fit its shape. There is no other.
- The parenthesis/annotation lookahead table stays (linear build, indexed
  queries).

### Public API

```rust
pub fn parse(source: &str) -> JavaParse<'_>;
pub struct JavaParse<'s> {/* Cow<'s, str> source, SyntaxTree, Vec<Diagnostic> */}
impl<'s> JavaParse<'s> {
    pub fn root(&self) -> CompilationUnit<'_>; // always present
    pub fn syntax(&self) -> SyntaxNode<'_>;
    pub fn diagnostics(&self) -> &[Diagnostic];
    pub fn source(&self) -> &str;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct JavaLanguage; // impl Language
```

Parser snapshots use `jolt_syntax::debug_tree::<L>`, an iterative,
language-agnostic tree dump that names kinds with `L::kind_name`.

`parse_compilation_unit`, `structural_diagnostic_owners`, `benchmark_metrics`,
and the `bench` feature are deleted.

## `jolt_fmt`

### Public API

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct FormatOptions {
    pub line_width: u16,
    pub indent_width: u8,
    pub use_tabs: bool,
} // default 80, 2, false

/// Receives rendered output in chunks; `Break` stops rendering early.
pub trait Sink {
    fn write(&mut self, chunk: &str) -> std::ops::ControlFlow<()>;
}
impl Sink for String {
    /* push, Continue */
}

/// Formats a tree. Placement, ignore regions, doc construction, the debug
/// emission check, and printing, in that order.
pub fn format<'a, L: FormatLanguage, S: Sink + ?Sized>(
    root: SyntaxNode<'a>,
    options: &FormatOptions,
    sink: &mut S,
) -> std::ops::ControlFlow<()>;

/// What a language provides to the shared formatter. Implemented by a local
/// unit type in the language's formatter crate (`jolt_java_fmt::JavaFormat`).
pub trait FormatLanguage: Sized + 'static {
    type Syntax: Language;
    /// Formats a node that is not malformed. Called only by `Formatter::node`.
    fn format_node<'a>(f: &mut Formatter<'a, Self>, node: SyntaxNode<'a>) -> Doc;
    /// Tokens that hug the left: an inline comment before one trails the
    /// previous element, and they may be printed before a pending line
    /// suffix. Java: `, ; ) ] } >`.
    fn is_sticky(kind: RawSyntaxKind) -> bool;
    /// If `token` opens a delimited region of its parent (`( [ {`, and Java's
    /// type-argument `<`), the closing kind; used by the generic placement
    /// hooks.
    fn closer_for(token: SyntaxToken<'_>) -> Option<RawSyntaxKind>;
    fn is_closer(token: SyntaxToken<'_>) -> bool;
    /// Language exceptions to comment placement, tried before the generic
    /// rules. `None` falls through.
    fn place_comment<'a>(ctx: &CommentContext<'a>) -> Option<Placement<'a>>;
    /// Expand an own-line `/** text */` into a multi-line star block.
    const EXPAND_DOC_COMMENTS: bool;
}
```

### Document IR

A `Doc` is a `u32` handle into one append-only arena per format run. Text is
always borrowed (`&'a str`: source slices and `&'static` punctuation), so no
string is allocated during layout. Target node size: 16 bytes; texts live in a
side table with their precomputed display width (ASCII fast path) and flags.

| Constructor                   | Meaning                                                                                                                             |
| ----------------------------- | ----------------------------------------------------------------------------------------------------------------------------------- |
| `text(&'a str)`               | Literal text. A text containing `\n` (text blocks) is written raw, gets no indentation inside, and cannot be flat.                  |
| `sticky(&'a str)`             | Text that may precede a pending line suffix (see printer). `Formatter::token` sets it from `is_sticky`; synthesized `,`/`;` use it. |
| `space()`                     | One space; dropped at line start and before a newline.                                                                              |
| `line()`                      | Space when flat, newline when broken.                                                                                               |
| `soft_line()`                 | Nothing when flat, newline when broken.                                                                                             |
| `hard_line()`                 | Newline; forces every enclosing group to break.                                                                                     |
| `empty_line()`                | Blank line (two newlines); forces enclosing groups to break.                                                                        |
| `concat([Doc])`               | Sequence.                                                                                                                           |
| `group(Doc)`                  | Flat if the contents fit, else broken.                                                                                              |
| `group_with_id(Doc, GroupId)` | Same, and records the decision under `GroupId` (allocated by `group_id()`).                                                         |
| `indent(Doc)`                 | One more indentation level for newlines inside.                                                                                     |
| `if_break(broken, flat, id)`  | Chooses by the mode of group `id`, or of the enclosing group when `None`.                                                           |
| `line_suffix(Doc)`            | Deferred until the next newline (or a non-sticky text; see printer).                                                                |
| `break_parent()`              | Forces every enclosing group to break.                                                                                              |
| `Doc::NIL`                    | Nothing.                                                                                                                            |

There is no `fill`, `align`, `force_flat`, `comment_prefix`, `literal_text`,
source claim, or any of the nine `LineMode`s: newline coalescing (below) makes
`*_boundary` and `hard_line_suffix` variants unnecessary, and the reference
style uses neither fill nor alignment (all continuation is one indent level).

Each node stores a `forces_break` bit computed at construction from its children
(`hard_line`, `empty_line`, `break_parent`, multi-line text). A group whose bit
is set is broken without a fit check; this is Prettier's `propagateBreaks`, done
in `O(1)` per constructed node.

Ergonomics: `Formatter` derefs to `DocBuilder`. Because building a doc needs
`&mut`, `jolt_fmt` exports three macros that evaluate their arguments before
borrowing the builder: `cat!`, `group!` and `indent!`, each written
`name![f; doc, doc, …]`. These are the only macros in the formatter crates.

### Printer

The printer is iterative: a command stack of `(indent, mode, doc)`, a line
suffix buffer, a `Vec<Mode>` indexed by `GroupId`, and output buffered into
chunks for the sink. State that replaces the old line modes:

- **Newline coalescing.** A newline request (broken `line`/`soft_line`,
  `hard_line`) sets `pending = max(pending, 1)`; `empty_line` sets
  `pending = 2`. Nothing is written until the next non-space text, which first
  writes `pending` newlines and the indentation of the most recent request.
  Consequently a newline at line start is a no-op, two blank-line requests never
  stack, a newline before the first text of the file is dropped, and trailing
  whitespace is never produced (pending spaces are discarded at a newline).
- **Line suffixes.** A broken newline request with pending suffixes first prints
  the suffixes (Prettier semantics). Additionally, a **non-sticky** text with
  pending suffixes first prints the suffixes and a newline at the text's indent.
  This is what keeps an end-of-line comment from being carried past a token:
  `} // c` followed by `else` prints `} // c` / `else`, while `a // c` followed
  by `,` prints `a, // c`.
- **End of input.** Flush suffixes, then exactly one `\n` (also for an empty
  file).

Fit check for a group not already forced to break: Prettier's `fits` over the
group's contents in flat mode followed by the rest of the stack in its recorded
modes, with remaining width `line_width - column`. It returns `false` on width
overflow or a forced newline inside the group, and `true` at the first newline
of the rest, at a non-sticky text while a suffix is pending, or at the end. It
visits at most `FIT_BUDGET = 4096` commands; exhausting the budget returns
`false` (break). Cost: `O(doc nodes × FIT_BUDGET)` worst case, in practice
`O(width)` per group because the check stops at the line end.

Indentation is `indent_width` spaces per level, or one tab per level with
`use_tabs` (a tab counts `indent_width` columns for fitting).

### Comments

#### Classification

The placement pass walks every token's trivia once and records each comment:

```rust
#[derive(Clone, Copy, Debug)]
pub struct Comment<'a> {
    pub text: &'a str,
    pub range: TextRange,
    pub kind: CommentKind,     // Line | Block | Doc (unterminated: Block)
    pub lines_before: u8,      // newlines since previous token/comment, capped at 2
    pub lines_after: u8,       // newlines until next comment/token, capped at 2
    pub layout: CommentLayout, // effective, after hooks
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommentLayout {
    OwnLine,     // newline before and after (file start / EOF count as newlines)
    StartOfLine, // newline before, code after on the same line (block comments)
    EndOfLine,   // code before, newline after (every line comment that is not OwnLine)
    Inline,      // code before and after on the same line (block comments)
}
```

`prev` and `next` are the tokens around a comment (`next` always exists: the
`Eof` token). The **enclosing node** is the deepest node containing both.

#### Placement

Every comment is attached exactly once:

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Placement<'a> {
    Leading(SyntaxElement<'a>, CommentLayout),
    Trailing(SyntaxElement<'a>, CommentLayout),
    Dangling(SyntaxNode<'a>),
}
pub struct CommentContext<'a> {
    pub comment: Comment<'a>,
    pub prev: Option<SyntaxToken<'a>>,
    pub next: SyntaxToken<'a>,
    pub enclosing: SyntaxNode<'a>,
}
```

Targets. `leading_target(next)` is the outermost node other than the root whose
first token is `next`, or the token itself when no such node exists. If that
node is a list, the target is instead its child on the path to `next` (repeated
while that child is a list too). `trailing_target(prev)` is the same with last
tokens. So a comment attaches to the outermost element that begins (or ends) at
the adjacent token, never to a list (always to its item, so a member's Javadoc
leads the member, not its `ModifierList`), and never past an adjacent token.
Comments therefore never move across a token.

Rules, first match wins:

1. `L::place_comment(ctx)`.
2. **G1, empty or unclosed delimited region.** `prev` opens a region
   (`closer_for(prev)` is `Some`) and `next` is not inside `prev`'s parent
   before its closer (it is the matching closer, or lies beyond a missing one):
   `Dangling(prev.parent())`.
3. **G2, end-of-line after an opener.** `prev` opens a region and the layout is
   `EndOfLine`: `Leading(leading_target(next), OwnLine)`.
4. **G3, own-line before a closer.** `next` is a closer and the layout is
   `OwnLine` or `StartOfLine`: `Dangling(next.parent())`.
5. Defaults: `OwnLine`/`StartOfLine` → `Leading(leading_target(next), layout)`;
   `EndOfLine` → `Trailing(trailing_target(prev), EndOfLine)`; `Inline` →
   `Trailing(trailing_target(prev), Inline)` if `next` is sticky, else
   `Leading(leading_target(next), Inline)`.

Hook constraints (checked by debug assertions in the pass): a hook may only
target elements inside the enclosing node; `Leading` layouts are `OwnLine`,
`StartOfLine` or `Inline`; `Trailing` layouts are `EndOfLine` or `Inline`.

Storage: comments in a `Vec` in source order; attachments in a `Vec` sorted by
`(target, side, source order)`; two bitsets (`NodeId`, `TokenId`) answer "has
attached comments" in `O(1)`, so elements without comments never search.
Placement is `O(tokens + trivia + comments × depth)`.

#### Printing

The formatter, not the rules, prints leading and trailing comments, from the
effective layout only:

| Attachment | Layout        | Printed as                                                     |
| ---------- | ------------- | -------------------------------------------------------------- |
| Leading    | `OwnLine`     | `hard_line, C, (empty_line if lines_after ≥ 2 else hard_line)` |
| Leading    | `StartOfLine` | `hard_line, C, space`                                          |
| Leading    | `Inline`      | `C, space`                                                     |
| Trailing   | `EndOfLine`   | `line_suffix(space, C), break_parent`                          |
| Trailing   | `Inline`      | `space, C`                                                     |
| Dangling   | any           | by the owning rule via `Formatter::dangling`: one per line     |

`C` is the comment doc: a line comment's text with trailing whitespace removed;
a single-line block comment's text; a multi-line block comment re-indented line
by line (star blocks aligned to `*`, plain continuation lines left-trimmed);
with `EXPAND_DOC_COMMENTS`, an own-line single-line `/** text */` is expanded to
`/**`, `* text`, `*/`. See [Java style](./java-style.md#comments).

Why this is idempotent: printing preserves each comment's neighbours (no comment
crosses a token) and its newline-before/newline-after facts, except where the
layout itself put a newline next to an `Inline` or `EndOfLine` comment. In that
case every enclosing group was already broken, so on the next pass the comment
(now `StartOfLine` or `EndOfLine`) forces exactly the breaks that already
happened and the output is unchanged. Sticky tokens may move ahead of an
end-of-line comment once (`a // c` `,` → `a, // c`); the result is a fixpoint.

### The `Formatter` context

```rust
pub struct Formatter<'a, L: FormatLanguage> {/* DocBuilder, Comments, ignore regions, emission counts */}
impl<'a, L: FormatLanguage> std::ops::DerefMut for Formatter<'a, L> {
    /* Target = DocBuilder<'a> */
}

impl<'a, L: FormatLanguage> Formatter<'a, L> {
    // ── The chokepoints. Rules emit source only through these. ──
    /// Leading comments, then (malformed ? verbatim : L::format_node), then
    /// trailing comments. The only caller of `L::format_node`.
    pub fn node(&mut self, node: SyntaxNode<'a>) -> Doc;
    pub fn fmt<N: AstNode<'a>>(&mut self, node: N) -> Doc; // node(n.syntax())
    pub fn fmt_opt<N: AstNode<'a>>(&mut self, node: Option<N>) -> Doc; // NIL for None
    /// Leading comments, the token text (sticky per `is_sticky`), trailing
    /// comments. Also emits a `ByteOrderMark` or `TrailingSubstitute` trivia
    /// piece of the token in place (only the first token and `Eof` have them).
    pub fn token(&mut self, token: SyntaxToken<'a>) -> Doc;
    pub fn token_opt(&mut self, token: Option<SyntaxToken<'a>>) -> Doc;
    /// `node`'s leading/trailing comments around a doc the caller built from
    /// the node's children. For iterative spine flattening and normalization
    /// wrappers; `node` must not be malformed.
    pub fn node_with(&mut self, node: SyntaxNode<'a>, body: Doc) -> Doc;
    /// Every comment attached to or inside `element`, in source order and in
    /// their own layouts, without the element's text. For tokens or nodes a
    /// normalization removes. Returns NIL when there are none.
    pub fn removed(&mut self, element: SyntaxElement<'a>) -> Doc;
    /// The node's source from its first token's text to its last token's
    /// text, interior comments included and marked emitted. Lines after the
    /// first lose the indentation prefix of the source line the node starts on
    /// (when they share it) and are re-indented by the printer.
    pub fn verbatim(&mut self, node: SyntaxNode<'a>) -> Doc;
    /// The node's dangling comments, each on its own line (empty_line between
    /// two when the source had a blank line), with no line before the first
    /// or after the last. NIL when none.
    pub fn dangling(&mut self, node: SyntaxNode<'a>) -> Doc;

    // ── Queries; they emit nothing. ──
    pub fn has_leading_comments(&self, element: SyntaxElement<'a>) -> bool;
    pub fn has_trailing_comments(&self, element: SyntaxElement<'a>) -> bool;
    pub fn has_dangling_comments(&self, node: SyntaxNode<'a>) -> bool;
    /// Any comment attached to the node or located inside its range.
    pub fn has_comments_within(&self, node: SyntaxNode<'a>) -> bool; // O(log comments)
    /// The source has a blank line before the node's first leading comment
    /// (or first token when it has none).
    pub fn blank_line_before(&self, node: SyntaxNode<'a>) -> bool;

    // ── Item lists ──
    pub fn item_list(
        &mut self,
        items: impl IntoIterator<Item = ListItem<'a>>,
        separator: impl FnMut(&mut Self, Separator<'a>) -> Doc,
    ) -> Doc;
}
pub enum ListItem<'a> {
    Format(SyntaxNode<'a>),
    Elide(SyntaxNode<'a>),
}
pub struct Separator<'a> {
    pub prev: SyntaxNode<'a>,
    pub next: SyntaxNode<'a>,
    pub blank_line_in_source: bool,
}
```

`node()` cost: two bitset probes on the fast path. There is no relocation state,
no `LeadingTrivia`/`TrailingTrivia` mode, and no `*_with_leading` variants: a
rule says _what_ to format, never how its comments print.

Rules emit tokens only with `token`, nodes only with `node`/`fmt`, and
synthesized punctuation (normalizations) with `text`/`sticky`. A rule formats
each element at most once; to use the same element in two `if_break` branches,
build its `Doc` once and reference the handle twice.

### Item lists, blank lines, formatter-ignore

`item_list` is the one place that implements:

- **Separators.** Between consecutive formatted items it calls `separator`; the
  language decides `hard_line` vs `empty_line` from the kinds and
  `blank_line_in_source`. Nothing is emitted before the first item or after the
  last, so bodies never start or end with a blank line.
- **Elision.** An `Elide` item prints `removed(item)`; if that is NIL the item
  vanishes (no separator), otherwise its comments stand in its place.
- **Formatter-ignore.** An item (`Format` or `Elide`) whose first token's text
  starts inside an ignore region prints `node_with(item, verbatim(item))`: its
  own leading and trailing comments (including the markers) are formatted
  normally, its interior is verbatim, and it is never elided. This check comes
  before elision.

Ignore regions are computed once after placement. A marker is a comment whose
body, after stripping `//` or `/* */` and surrounding whitespace, is exactly
`@formatter:off` or `@formatter:on`. Scanning markers in source order, the first
`off` opens a region at its end; the next `on` closes it at its start (later
`off`s inside a region are ignored). An `off` with no later `on` ends at the end
of its comment's enclosing node — the innermost container, or end of file at top
level. Regions are sorted and disjoint; membership is a binary search,
`O(log regions)` per item. Nested lists inside a verbatim item are never
visited; nested lists inside a formatted item apply the same test to their own
items. Only item lists honour ignore regions (Java: compilation unit items, type
body members, block statements, switch group statements, module directives).

Malformed nodes use the same `verbatim`: `node()` on a malformed node is
`node_with(node, verbatim(node))`. Verbatim output covers exactly the comments
strictly between the node's first and last token; placement never attaches those
to anything outside the node, so nothing is lost or duplicated at the boundary
(#224).

### Emission check

`Formatter` keeps one emission counter per comment in every build. After the
document is built, `format` asserts every counter is exactly 1 when
`cfg!(debug_assertions)` is true, panicking with each offending comment's text
and position. Release builds skip the check but keep the same layout. This
replaces `SyntaxConservationTracker`, source claims, and the debug double
render. Tokens need no tracking: the oracle's structure fingerprint covers them.

## `jolt_java_fmt`

### Module layout and ownership

```text
crates/jolt_java_fmt/
  src/lib.rs                    shared   pub fn format, JavaFormat, type JavaFormatter
  src/language.rs               shared   impl FormatLanguage: sticky/opener tables, comment hooks
  src/dispatch.rs               shared   total match: node kind → rule function
  src/helpers/mod.rs            shared
  src/helpers/lists.rs          shared   delimited comma lists
  src/helpers/bodies.rs         shared   braced bodies, member/statement item lists, blank-line policy
  src/helpers/assignment.rs     shared   `left = right` layout
  src/helpers/tokens.rs         shared   join spacing between adjacent tokens, keyword helpers
  src/helpers/placeholder.rs    shared   TEMPORARY: deleted at integration
  src/rules/mod.rs              shared   declares the four area modules
  src/rules/declarations/**     Area A
  src/rules/statements/**       Area B
  src/rules/expressions/**      Area C
  src/rules/types/**            Area D
  examples/jhunt.rs jcheck.rs jref.rs              shared
  tests/corpus.rs recovery.rs probes.rs common/    shared
```

```rust
pub struct JavaFormat; // impl FormatLanguage<Syntax = JavaLanguage>
pub type JavaFormatter<'a> = jolt_fmt::Formatter<'a, JavaFormat>;
pub fn format<S: Sink + ?Sized>(
    parse: &JavaParse<'_>,
    options: &FormatOptions,
    sink: &mut S,
) -> ControlFlow<()>;
```

Every rule function has the signature

```rust
pub(crate) fn class_declaration<'a>(f: &mut JavaFormatter<'a>, node: ClassDeclaration<'a>) -> Doc;
```

named after the kind in `snake_case`, and is re-exported from its area's
`mod.rs`. `dispatch.rs` calls exactly these names, so an area can restructure
its files freely as long as the entry points keep their names and signatures.
List kinds with a layout of their own (`ModifierList`, `ParameterModifierList`,
`VariableDeclaratorList`, `ArrayDimensions`, `AnnotationList`, …) have rule
functions like any node. List kinds whose parent always iterates them through a
helper (`FormalParameterList`, `ExpressionList`, `BlockStatementList`,
`ClassBodyMemberList`, …) dispatch to `helpers::lists::unexpected_list`, which
formats the children separated by spaces and `debug_assert!`s; the foundation
fixes this split in `dispatch.rs`.

### Java comment hooks

`JavaFormat::place_comment` implements only these exceptions to the generic
rules:

- **H1, comment after a control header.** An `EndOfLine` comment whose `prev` is
  the `)` closing the header of `if`, `while`, `for`, `switch`, `synchronized`,
  `catch`, or try-with-resources, or the keyword `else`, `do`, `try`, or
  `finally`, and whose `next` starts the body: a `Leading` placement with layout
  `OwnLine` on the first element inside the body when it has one (for a
  non-block body that brace promotion will wrap, the body statement itself),
  else `Dangling` on the body block.
- **H2, type-argument angles.** `<`/`>` of `TypeArgumentList` and
  `TypeParameterList` are openers/closers (via `closer_for`); `<` and `>` as
  operators are not.

Anything else that looks like it needs a hook should first be checked against
the generic rules; a new hook requires a fixture and an entry here.

### Helpers (foundation-provided)

```rust
// helpers/lists.rs
pub(crate) enum TrailingComma {
    Never,
    IfBroken,
    Preserve,
}
pub(crate) struct ListLayout {
    pub padded: bool, /* "{ a, b }" when flat */
    pub trailing_comma: TrailingComma,
}
/// `open item, item close` with the reference's break-all layout: flat if it
/// fits, else every element on its own line at one more indent and `close` on
/// its own line. Prints dangling comments of `owner` inside. Represented
/// separators go through `token`, synthesized ones through `sticky`.
pub(crate) fn delimited<'a, T: AstNode<'a>>(
    f: &mut JavaFormatter<'a>,
    owner: SyntaxNode<'a>,
    open: Option<SyntaxToken<'a>>,
    elements: impl Iterator<Item = ListElement<'a, T>>,
    close: Option<SyntaxToken<'a>>,
    layout: ListLayout,
) -> Doc;

// helpers/bodies.rs
pub(crate) enum BodyKind {
    Members,
    Statements,
    SwitchEntries,
    Directives,
}
/// `{`, indented items separated per the body policy, dangling comments,
/// `}`. Empty bodies print per java-style.md ("Empty bodies").
pub(crate) fn braced_body<'a>(
    f: &mut JavaFormatter<'a>,
    owner: SyntaxNode<'a>,
    open: Option<SyntaxToken<'a>>,
    items: impl IntoIterator<Item = ListItem<'a>>,
    close: Option<SyntaxToken<'a>>,
    kind: BodyKind,
) -> Doc;
pub(crate) fn member_separator<'a>(f: &mut JavaFormatter<'a>, sep: Separator<'a>) -> Doc; // blank unless field–field
pub(crate) fn statement_separator<'a>(f: &mut JavaFormatter<'a>, sep: Separator<'a>) -> Doc; // blank iff source had one

// helpers/assignment.rs
/// `left op` then `group(indent(line, right))`.
pub(crate) fn assignment_like<'a>(
    f: &mut JavaFormatter<'a>,
    left: Doc,
    op: Option<SyntaxToken<'a>>,
    right: Doc,
) -> Doc;

// helpers/tokens.rs
/// Whether two adjacent tokens printed with no separator would lex as
/// different tokens (`- -x`, `+ +x`, `a instanceof`…). Rules that join
/// elements with nothing between them consult it.
pub(crate) fn needs_space(left: SyntaxToken<'_>, right: SyntaxToken<'_>) -> bool;

// helpers/placeholder.rs  (TEMPORARY)
/// Verbatim stand-in for an unported rule; records the kind for `jref`.
pub(crate) fn placeholder<'a>(f: &mut JavaFormatter<'a>, node: SyntaxNode<'a>) -> Doc;
```

### Cross-area layout contracts

What a node's `Doc` contains when another area formats it with `f.fmt`. Every
doc excludes surrounding spaces unless stated.

| Kind (owner)                                      | Doc contract                                                                                                                                                                  |
| ------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `ModifierList`, `ParameterModifierList` (D)       | Annotations and keywords **including** the trailing separator: `hard_line` after declaration annotations, `space` after the last keyword/type-use annotation. NIL when empty. |
| `Block` (B)                                       | `{ … }` from `braced_body(Statements)`; caller adds the space before.                                                                                                         |
| `ClassBody`, `InterfaceBody`, `EnumBody`, … (A)   | `{ … }`; used by C for anonymous classes and enum constant bodies.                                                                                                            |
| `ArgumentList` (C)                                | `( … )` via `delimited`; used by A (enum constants, explicit constructor invocations).                                                                                        |
| `FormalParameter`, `ReceiverParameter` (D)        | One parameter. The parens and break-all list are A's: the declaration rule passes `parameters().elements()` to `delimited`.                                                   |
| `TypeParameterList`, `TypeArgumentList` (D)       | `< … >` group; used by A, C.                                                                                                                                                  |
| `VariableDeclaratorList` (D)                      | `name = init, …` with `assignment_like`; continuation declarators on new lines at one indent when broken.                                                                     |
| `Expression` category (C), `SwitchExpression` (B) | A complete expression; never includes a trailing `;`.                                                                                                                         |
| Patterns (D)                                      | Used by C (`instanceof`) and B (case labels).                                                                                                                                 |
| Statements (B)                                    | Include their `;`. Bodies of control statements are formatted by B only.                                                                                                      |

If an area needs a different contract, it requests it in its result instead of
working around another area's output.

### Normalizations

Normalizations apply only when the owning node `is_recovery_free()`. Removed
tokens print their comments through `removed`. The structure fingerprint
(`jolt_java_fmt/tests/common`) ignores exactly these changes.

| #   | Normalization                                  | Where                                                                                                                                                                | Extra condition                                                              |
| --- | ---------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------- |
| N1  | Import grouping and sorting                    | Imports of a compilation unit                                                                                                                                        | Per import: recovery-free, name without interior comments. See Java style.   |
| N2  | Modifier ordering                              | Declaration `ModifierList`                                                                                                                                           | Keyword runs between comment-carrying or bogus modifiers sort independently. |
| N3  | Module directive and `requires` modifier order | `ModuleDeclaration`                                                                                                                                                  | Directive recovery-free; comment-carrying directives anchor like imports.    |
| N4  | Brace promotion                                | Non-block bodies of `if`, `else`, `while`, `do`, `for`, enhanced `for`                                                                                               | Body is not malformed.                                                       |
| N5  | Redundant `;` removal                          | `EmptyStatement` in blocks and switch groups, `EmptyDeclaration` in compilation units and type bodies, trailing `;` of try resources, redundant enum body separators | —                                                                            |
| N6  | Redundant parenthesis removal                  | Parenthesized left operand of a same-precedence binary chain; switch guard `when (x)`; single untyped lambda parameter `(x) ->`                                      | Parentheses carry no comments.                                               |
| N7  | Readability parentheses                        | Binary operands of `&`, `\|`, `^`, `<<`, `>>`, `>>>` that are themselves binary expressions                                                                          | —                                                                            |
| N8  | Trailing commas                                | Array and annotation array initializers (iff broken); enum constants (per Java style)                                                                                | —                                                                            |
| N9  | Comment text                                   | Trailing whitespace, re-indentation, star alignment, doc comment expansion                                                                                           | Comment content otherwise unchanged.                                         |
| N10 | Whitespace                                     | Line endings become `\n`; one final newline; BOM and trailing SUB preserved                                                                                          | —                                                                            |

There is no import deduplication; the reference does not deduplicate.

### Deep spines

The parser builds left-deep binary/`instanceof` chains and receiver chains
(field access, calls, array access, postfix) iteratively, so their depth is
unbounded. Their rules (Area C) must not recurse through `f.node` along the
spine: they walk the spine with an explicit loop, format the leaves with
`f.node`, and emit each interior spine node's comments with `node_with` (or by
`removed` for parentheses removed by N6). A malformed spine node stops the walk
and is formatted with `f.node`. Every other recursion is bounded by the parser's
`MAX_DEPTH`. A test formats a 20 000-term `+` chain and a 20 000-call chain on a
256 KiB thread, and the deepest accepted nesting on a 1 MiB thread.

## `jolt_formatter` (facade)

This API is fixed by the formatter foundation; the CLI and dprint plugin use
nothing else.

```rust
pub use jolt_fmt::{FormatOptions, Sink};
pub use jolt_syntax::{Diagnostic, LineCol, LineIndex, TextRange, TextSize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Language {
    Java,
}
impl Language {
    pub fn from_path(path: &std::path::Path) -> Option<Language>;
} // ".java"

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OnSyntaxError {
    Refuse,
    Format,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FormatResult {
    /// Output was written to the sink; `halted` when the sink broke early.
    Formatted { halted: bool },
    /// `Refuse` and the source has syntax errors; nothing was written.
    Refused(Vec<Diagnostic>),
}

pub fn format<S: Sink + ?Sized>(
    source: &str,
    language: Language,
    options: &FormatOptions,
    on_error: OnSyntaxError,
    sink: &mut S,
) -> FormatResult;
```

Feature `serde` forwards to `jolt_fmt/serde`. `RenderSink`, `RenderControl`,
`FormatSinkResult`, `ParsedSource`, `parse_source`,
`format_parsed_source_to_sink`, and `format_source_to_sink` are deleted. `check`
mode is a sink that compares chunks against the source and breaks at the first
difference.

## Verification

### Oracle

For an input `S` and output `F = format(S)`:

1. **Reparse.** If `S` parses without diagnostics, `F` does too. Otherwise the
   multiset of diagnostic codes of `F` equals that of `S`.
2. **Structure.** For clean `S`, the structure fingerprint of `F` equals that of
   `S` modulo N1–N8: imports, module directives, requires modifiers and
   declaration modifier keywords compared as multisets; `EmptyStatement`,
   `EmptyDeclaration`, `ParenthesizedExpression`, promoted `Block`s and
   `BlockStatementList`/`BlockStatement` wrappers transparent; `, ; { } ( )`
   ignored. (Today's `STRUCTURE_POLICY`, minus composite operators.)
3. **Comments.** The multiset of comments (kind and canonical text: per-line
   trimmed, star prefixes stripped, doc expansion undone) of `F` equals that of
   `S`.
4. **Idempotence.** `format(F) == F`.

### Tests

- `jolt_java_syntax/tests/parser_fixtures.rs`: `debug_tree` and diagnostics
  snapshots for `fixtures/java/syntax/**` (regenerated once by the syntax step;
  the diff must show only owner removal, `Bogus` unification, and `>` tokens).
- `jolt_java_syntax/tests/codegen.rs`: generated code is current.
- `jolt_java_fmt/tests/corpus.rs`: every fixture outside `syntax/` and
  `recovery/` is formatted, checked with the oracle at widths 40, 80 and 120,
  and snapshotted at 80. Snapshot names stay `corpus__<dir>__<file>.snap`.
- `jolt_java_fmt/tests/recovery.rs`: `fixtures/java/syntax/recovery/**` with the
  oracle and input/output/diagnostics snapshots.
- `jolt_java_fmt/tests/probes.rs`: every comment probe shape at every token
  boundary of every fixture under 400 tokens, oracle checks 1, 3, 4.
- Stack-depth tests (see Deep spines), and a fit-budget test in `jolt_fmt`.
- The old `structure_fingerprint.rs`, `trivia_conservation.rs`,
  `comment_positions.rs`, `layout_fit_boundaries.rs`,
  `jolt_java_syntax/tests/normalization.rs`, diagnostic-ownership audits, and
  schema physical audits are folded into the above or deleted.

### Corpus tools

- `mise run hunt` (`examples/jhunt.rs`): the sweep over `tools/import/.imports`
  with the oracle at three widths and the four comment probe shapes.
  `KNOWN_OPEN` must be empty from the formatter foundation onward.
- `examples/jcheck.rs`: replays one file or one probe (`File.java:ownline12`).
- `examples/jref.rs` (new): `jref <dir>` checks `format(F) == F` for every
  `.java` file (or the `formatted:` section of every `.snap`) under `<dir>`;
  `jref --input <in> --expected <out> [--line-width N]` checks
  `format(in) == out` file by file. It prints totals, writes diffs to
  `target/jref/`, and for every mismatching file lists the placeholder kinds the
  run hit, so a mismatch with no placeholders is a real bug. The reference set
  is `/tmp/jolt-ref-ScpJkX` (`input/`, `old-output/`, `old-output-w100/`) and
  the snapshots at commit `53efc726`.

### Performance

Budget: `jolt format --no-config --check` over
`/tmp/jolt-ref-ScpJkX/bench/spring-framework` must not be slower than the
reference binary: median wall **≤ 109.5 ms** with default threads and **≤ 1141.8
ms** with `--threads 1` (hyperfine, as in `/tmp/jolt-ref-ScpJkX/README.md`;
`rerun.sh` reruns everything). Measured at integration, and by any area that
changes a hot path.

Design levers already in this contract: no per-token comment work without a
bitset hit; borrowed text only; 16-byte doc nodes with precomputed widths and
break bits; early sink halt in check mode; no debug-only work in release. If the
budget is missed, the next levers are reusing the token, node, and doc vectors
across files on a thread, and profile-guided builds (`tools/pgo`).

## Deleted

Crates: `jolt_text`, `jolt_diagnostics`, `jolt_fmt_ir`.

`jolt_syntax`: `conservation.rs` (tracker, source identities and claims,
`SyntaxVerbatimCore`), `normalization.rs` (all claim types), `projection.rs`,
`accessors.rs`, `field.rs` (`SyntaxField`, `SyntaxListPart`, `MalformedSyntax`,
`MissingSyntax`, `SyntaxInvariantError`), `schema.rs` (macros), `comment.rs`
(replaced by `jolt_fmt` placement), `parse_debug.rs` (replaced by `debug_tree`),
diagnostic owners in `event.rs`, `parser/source.rs` and `syntax_tree.rs`,
`BuildSyntaxTreeError`, `SyntaxTreeMetrics`, the `bench` feature, the `Language`
type parameter on cursors and every hand-written
`Clone`/`Copy`/`Debug`/`PartialEq` impl for marker types.

`jolt_java_syntax`: `normalization.rs`, `shape.rs`, `schema_audit.rs`,
`schema.rs` macros (replaced by `java.schema` + `generated.rs`), the
hand-written projections, `classify`, `as_*`, `partitioned_items` and
`define_*_projection` macros in `nodes/mod.rs`, `JavaOperator` and
`JavaOperatorKind` composites, the five composite operator kinds, 27 of the 28
bogus kinds, `split_token`, all `parse_excessive_*` functions, the diagnostic
owner/slot arguments at every parser call site, `JavaParseDiagnosticCode`
`InvalidEventStream`, `structural_diagnostic_owners`, `tests/normalization.rs`.

`jolt_java_fmt`: all of `src/` (rewritten), `tests/structure_fingerprint.rs`,
`tests/trivia_conservation.rs`, `tests/comment_positions.rs`,
`tests/layout_fit_boundaries.rs`, the `bench` feature.

`jolt_test_support`: `diagnostic_ownership.rs`, `schema_audit.rs`, `StringSink`
(use `String`), trivia markers, `assert_trivia_markers_conserved`.

Concepts: comment ownership by tokens, `LeadingTrivia`/`TrailingTrivia` modes,
relocation state on `DocBuilder`, the nine line modes, `CommentPrefix`,
`ForceFlat`, `LexicalSafety` join tables for verbatim fragments, the
formatter-ignore planner and splice sites, recovery `FormatField`/`LayoutDoc`
plumbing, `RenderError`, invariant blocking (`block_on_invariant`).

## Build plan

All stages work in their own worktree, commit locally with conventional commits,
and report honestly. Gates use the current task names.

### (a) Syntax foundation

Scope: `jolt_syntax`, `jolt_java_syntax`, the deletions above for those crates,
and folding `jolt_text`/`jolt_diagnostics`. The old `jolt_fmt_ir` and
`jolt_java_fmt` cannot follow the new syntax API: delete both, and temporarily
remove `jolt_formatter`, `jolt_java_fmt`, `jolt_cli`, `jolt_fmt_dprint` and
`tools/bench/driver` from the workspace `members` (marked `TEMPORARY`; the
formatter foundation restores them). Cut `jolt_test_support` down to what the
syntax tests use (fixture collection, snapshot sections, diagnostics rendering);
the formatter foundation restores the oracle and sweep from `53efc726`, adapted.
The old rules stay readable at commit `53efc726` (`git show 53efc726:<path>`),
which is how later stages consult them.

Gates, run directly (the `mise` check tasks still list the removed crates):

- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo nextest run --workspace`
- the parser snapshot diff, reviewed and explained in the result
- every corpus file in `tools/import/.imports` parses in release with the same
  diagnostic count as before

### (b) Formatter foundation

Scope: `jolt_fmt` complete (IR, printer, placement, rules G1–G3, `Formatter`,
item lists, formatter-ignore, emission check); `jolt_java_fmt` shared files
(`lib.rs`, `language.rs` with H1–H2, `dispatch.rs`, `helpers/**`,
`rules/mod.rs`, tests, examples including `jref`); one file per area with
**every** rule entry point, each a call to `placeholder`, except these
exemplars, which must match the reference: `compilation_unit` (item list without
import sorting), `class_declaration` (without clauses), `class_body`,
`field_declaration`, `method_declaration` (simple header), `block`,
`expression_statement`, `return_statement`, `method_invocation_expression`
(unqualified and simple qualified), `name_expression`, `literal_expression`,
`argument_list`, `class_type`, `primitive_type`, `modifier_list` (no sorting),
`formal_parameter`; the `jolt_formatter` facade; minimal call-site updates in
`jolt_cli`/`jolt_fmt_dprint` and the crate lists in `hk.pkl`; `AGENTS.md`
invariants updated to this design. Formatter snapshots are **not** accepted in
this stage; they remain the reference.

This stage lands before the periphery agent branches, so its edits to
periphery-owned files cannot conflict.

Gates:

- `cargo clippy --workspace --all-targets -- -D warnings`
- `INSTA_FORCE_PASS=1 mise run test` green: every oracle check passes, snapshot
  mismatches are expected
- `mise run hunt` green with an empty `KNOWN_OPEN`
- the stack-depth tests pass
- `jref` totals recorded in the result as the baseline for the areas

### (c) Rule areas (parallel)

Four agents branch from the formatter-foundation commit, one worktree each. Each
owns exactly the files below and may create new files only inside its own
directories. They implement their kinds to match the reference (snapshots and
`/tmp/jolt-ref-ScpJkX`) plus the comment behaviour this page defines, deleting
each `placeholder` call they port.

| Area               | Owned files                                                                                                                                                       | Kinds                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| ------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| A, declarations    | `crates/jolt_java_fmt/src/rules/declarations/**`; new fixtures under `fixtures/java/style/{declarations,program,imports,modules,compact-compilation-units}/`      | `CompilationUnit`, `PackageDeclaration`, `ImportDeclaration`, `ModuleDeclaration`, the five directives, `ModuleTargetClause`, `ModuleImplementationClause`, class/record/enum/interface/annotation-interface declarations, `ExtendsClause`, `ImplementsClause`, `PermitsClause`, all type bodies, `EmptyDeclaration`, `EnumConstant`, `RecordComponent`, `FieldDeclaration`, `MethodDeclaration`, `ConstructorDeclaration`, `ConstructorBody`, `ConstructorInvocation`, `CompactConstructorDeclaration`, static/instance initializers, `ThrowsClause`, `AnnotationElementDeclaration`, `DefaultValue`, and their lists |
| B, statements      | `crates/jolt_java_fmt/src/rules/statements/**`; new fixtures under `fixtures/java/style/statements/`                                                              | `Block`, `BlockStatement`, `LocalVariableDeclaration`, `LocalClassOrInterfaceDeclaration`, every statement kind, `SwitchStatement`, `SwitchExpression`, `SwitchBlock`, `SwitchBlockStatementGroup`, `SwitchRule`, `SwitchLabel`, `CaseConstant`, `CasePattern`, `Guard`, for/enhanced-for parts, try/catch/finally/resources, `VariableAccess`, and their lists                                                                                                                                                                                                                                                        |
| C, expressions     | `crates/jolt_java_fmt/src/rules/expressions/**`; new fixtures under `fixtures/java/style/{expressions,layout-fit-boundaries,lists}/`, `fixtures/java/properties/` | Every `Expression` member except `SwitchExpression`, `QualifiedMethodInvocation`, `UnqualifiedMethodInvocation`, `ArgumentList`, `DimExpression`, `ArrayInitializer`, `LambdaParameterList`, `LambdaParameter`, and their lists                                                                                                                                                                                                                                                                                                                                                                                        |
| D, types and parts | `crates/jolt_java_fmt/src/rules/types/**`; new fixtures under `fixtures/java/style/types/`                                                                        | `ModifierList`, `NonSealedModifier`, annotations and element values, type parameters and bounds, every type kind, type arguments and wildcards, array dimensions, names, `TypeList`, `FormalParameter`, `ReceiverParameter`, `ParameterModifierList`, `VariableDeclaratorList`, `VariableDeclarator`, `VariableInitializer`, every pattern kind, and their lists                                                                                                                                                                                                                                                       |

Rules of engagement:

- Do not edit any file you do not own: no shared helper, hook, dispatch, test,
  snapshot, syntax crate, or another area's rules. If you need a shared helper
  or hook, write a private version in your own files marked with a
  `REQUEST(shared)` comment saying what it is, and list it in your result;
  integration consolidates.
- The old rule for a kind is at commit `53efc726` under
  `crates/jolt_java_fmt/src/rules/`; read it for the reference's decisions, but
  do not port its comment or recovery plumbing.
- Never accept or edit existing snapshots. Inspect pending diffs for your
  fixture directories with `cargo insta test -p jolt_java_fmt`, and run
  `cargo insta reject` before committing. Snapshots for fixtures you add are
  created at integration.

Gates:

- `cargo clippy --workspace --all-targets -- -D warnings`
- `INSTA_FORCE_PASS=1 mise run test` green
- `mise run hunt` green
- `jref` over the four corpora (`--input`/`--expected`, widths 80 and 100) and
  over the snapshot directory, reported against the foundation baseline;
  mismatches with no placeholder hits are reported as bugs, and mismatches you
  believe belong to other areas are reported as such

### Periphery (concurrent with (c))

A separate agent owns `crates/jolt_cli`, `crates/jolt_fmt_dprint`, `tools/`,
`docs/` except `docs/internals/`, `README.md`, `mise.toml`, `hk.pkl` and
`.github`, branching from the formatter-foundation commit. It consumes only the
facade above. It should also add [Java style](./java-style.md) to the docs
sidebar.

### (d) Integration

Merge A–D and the periphery; consolidate `REQUEST(shared)` helpers; delete
`helpers/placeholder.rs` and its `jref` reporting; review every snapshot diff
against the reference and accept only differences listed in Java style's
"Deliberate changes"; then apply D1 (empty bodies) as its own commit with its
snapshot updates; run every gate including the performance budget; update
`AGENTS.md` and this page to what was built.

## Changes to project invariants

This design keeps every invariant in `AGENTS.md` and makes these precise:

- "Formatter rules must preserve represented source tokens and their trivia":
  rules emit tokens only through `Formatter::token`, and comments only through
  the chokepoints and `dangling`. Rules never print leading or trailing comments
  themselves.
- "Verbatim formatting must track every contained token and comment": verbatim
  covers exactly the comments strictly inside the node, and the emission check
  verifies every comment is emitted once.
- Normalizations are the closed list N1–N10, gated on `is_recovery_free()`.

It temporarily supersedes "Valid syntax must always use structured formatting"
for `helpers::placeholder` only, from the formatter foundation until integration
deletes it.
