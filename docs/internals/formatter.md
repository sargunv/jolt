# Formatter architecture

This page is the design contract for Jolt's formatter. It describes the
architecture: the crates, their public APIs, the core data structures and
algorithms with their costs, and what the rewrite deleted. The Java output it
must produce is specified separately in [Java style](./java-style.md).

Where this page and an implementation disagree, the implementation is wrong or
this page must be amended in the same change. Where this page explicitly
supersedes a project invariant in `AGENTS.md` (which `CLAUDE.md` links to), the
section says so. Recommended defaults for decisions this page leaves open are
under [Decisions](#decisions); an implementer follows them unless the page is
amended.

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
printer's fit check (at most `FIT_BUDGET` commands per group) and the parser's
speculative lookahead (each scan capped by `MAX_DEPTH` nesting, balanced
brackets answered by a table built once in linear time; see Parser rules).

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
slot range, its token range, its parent and its index in the parent. A token
stores its parent node, so `SyntaxToken::parent` is `O(1)`. Slots are fixed per
node kind (see the factory below); an empty slot is a valid absent optional or a
missing required element.

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

// Equality and hashing are identity: (tree pointer, id), written by hand
// because the borrowed source a cursor also carries must not take part.
#[derive(Clone, Copy)]
pub struct SyntaxNode<'a> {
    tree: TreeRef<'a>,
    source: &'a str,
    id: NodeId,
}
#[derive(Clone, Copy)]
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

Trivia attachment is unchanged from today's lexer: a token's trailing trivia is
the horizontal whitespace, line comments and single-line block comments that
follow it up to (not including) the next line break; a block comment that
contains a line break ends the trailing trivia and, like everything after it, is
the following token's leading trivia. The final `Eof` token carries the file's
trailing trivia. Comment placement never depends on which token's trivia holds a
comment; it uses only the tokens around the comment and the line breaks between
them (see [Classification](#classification)).

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
    /// running it when `MAX_DEPTH` (128) levels are active. `parser` is the
    /// parser or a language's wrapper around it (Java's has a `nested`
    /// method that forwards here).
    pub fn nested<P: AsMut<Self>, T>(parser: &mut P, parse: impl FnOnce(&mut P) -> T) -> Option<T>;
    /// Bumps tokens up to EOF or, outside the brackets the skipped tokens
    /// open, a token `stop` accepts. Expression recovery stops at `}` this
    /// way, so it never swallows the closer of an enclosing block.
    pub fn skip_balanced(&mut self, stop: impl Fn(L::Kind) -> bool);
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
calibrated for the 1 MiB WASM stack. A block that is the body of a control or
labeled statement takes no level of its own (the statement took one), so brace
promotion (N4) never makes the output nest deeper than the input, and an
`else if` chain is parsed in a loop and takes one level however long it is.

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
   Separated lists record missing items and separators as empty slots. A list
   child that fits neither (a constructor in an interface body, `[]` in an
   argument list) is wrapped in a new bogus node at an item position, so a list
   node is never malformed and its other items keep their structured layout.
4. In a fixed node, any child that fits no slot makes the whole node `MALFORMED`
   (raw children, kind kept so the node still fits its parent's slot).
5. Every mismatch of rules 3 and 4 that no parser diagnostic overlaps gets an
   `unexpected` diagnostic from the builder, so a tree with a bogus or malformed
   node always has a diagnostic and the CLI refuses it by default.

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

One schema is the single source of truth for kinds and tree shape. It is plain
Rust data in `crates/jolt_java_syntax/tests/codegen/schema.rs` (struct literals
or small builder functions), so there is no schema language and no schema parser
to maintain. It keeps today's vocabulary (tokens; categories; nodes with
`required`/`optional`/`many`/`one_or_more` fields over `token`, `token_set`,
`contextual`, `node`, `node_set`, `category`, `list`, `choice`; separated lists
with separator, minimum and trailing policy). The 28 per-category bogus kinds
collapse into one `Bogus` kind accepted wherever rule 2 or 3 applies; schema
fields that named a specific bogus kind (`BogusImportSuffix`, `BogusModifier`,
`BogusSwitchLabelItem`) name `Bogus`.

`crates/jolt_java_syntax/tests/codegen/main.rs` renders the schema to
`crates/jolt_java_syntax/src/generated.rs` (checked in, `// @generated`). The
test fails with a diff when the file is stale and rewrites it when
`INSTA_UPDATE=always` (what `mise run test --update` sets). The generator is
plain Rust (target under 500 lines), not macros. It emits:

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

/// Separated lists expose items and elements. A plain list whose item type
/// holds `Bogus` (a category or choice) exposes items; one whose item type
/// cannot (a single node kind or tokens) exposes only elements, so no rule can
/// skip a bogus item by accident.
impl<'a> ClassBodyMemberList<'a> {
    pub fn items(self) -> impl Iterator<Item = ClassBodyMember<'a>> + 'a;
}
impl<'a> BlockStatementList<'a> {
    pub fn elements(self) -> impl Iterator<Item = ListElement<'a, BlockStatement<'a>>> + 'a;
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
    /// A `Bogus` node at an item position whose type cannot hold it.
    Bogus(SyntaxNode<'a>),
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
  queries). Whether a formal parameter is a receiver (`X<T, U> this`) is a
  forward scan of that parameter, capped at 512 tokens.
- Every other speculative lookahead (type arguments, casts, lambda heads,
  modifiers) scans at most one type, annotation or header whose nesting is
  capped at `MAX_DEPTH`, or answers balanced-bracket questions from the table,
  so the total lookahead cost is `O(tokens × MAX_DEPTH)`. Review checklist item
  for the syntax step: list each lookahead entry point and its bound.

### Public API

```rust
pub fn parse(source: &str) -> JavaParse<'_>;
// The tree always refers to the raw source: Unicode escapes are translated
// into a temporary buffer for lexing, and token and trivia ranges are mapped
// back to the raw text, as today. So the source is a plain borrow.
pub struct JavaParse<'s> {/* &'s str source, SyntaxTree, Vec<Diagnostic> */}
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
/// emission check, and printing, in that order. A `ByteOrderMark` trivia piece
/// at the start of the source is written before everything else, and a
/// `TrailingSubstitute` piece after the final newline; no rule sees either.
pub fn format<'a, L: FormatLanguage, S: Sink + ?Sized>(
    root: SyntaxNode<'a>,
    options: &FormatOptions,
    sink: &mut S,
) -> std::ops::ControlFlow<()>;

/// What a language provides to the shared formatter. Implemented by a local
/// unit type in the language's formatter crate (`jolt_java_fmt::JavaFormat`).
/// The token tables take a token, not a kind, because the same kind can play
/// two roles (Java's `<`/`>` are type-argument delimiters or operators).
pub trait FormatLanguage: Sized + 'static {
    type Syntax: Language;
    /// Formats a node that is not malformed. Called only by `Formatter::node`.
    fn format_node<'a>(f: &mut Formatter<'a, Self>, node: SyntaxNode<'a>) -> Doc;
    /// Separators and terminators that no layout ever puts on a new line:
    /// an end-of-line comment directly before one moves after it (rule S1),
    /// and an inline comment before one trails the previous element.
    /// Java: `,` and `;`.
    fn is_separator(token: SyntaxToken<'_>) -> bool;
    /// If `token` opens a delimited region of its parent (`( [ {`, and Java's
    /// type-argument and type-parameter `<`), the kind of its closer.
    fn closer_for(token: SyntaxToken<'_>) -> Option<RawSyntaxKind>;
    /// `) ] }` and Java's type-argument and type-parameter `>`.
    fn is_closer(token: SyntaxToken<'_>) -> bool;
    /// Whether text ending in `left` directly followed by text starting with
    /// `right` would lex differently (`- -x`, `a b`, `/ /*c*/`, `< <` in
    /// recovered type arguments). The printer writes a space between such
    /// texts, so no rule has to; Java never joins `>` `>`.
    fn joins(left: char, right: char) -> bool;
    /// Tokens after which an inline comment is printed with no space:
    /// openers (`closer_for` is `Some`), and Java's `.`, `::` and `@`.
    fn hugs_next(token: SyntaxToken<'_>) -> bool;
    /// Language exceptions to comment placement, tried before the generic
    /// rules. `None` falls through.
    fn place_comment<'a>(ctx: &CommentContext<'a>) -> Option<Placement<'a>>;
    /// The item-list owner a formatter-ignore marker belongs to, given the
    /// enclosing node of the marker: the owner when the marker sits between,
    /// before or after the items of the owner's list, `None` when it is inside
    /// an item. Java: compilation unit, type bodies, block, constructor body,
    /// switch block statement group, module declaration (and their item list
    /// nodes, which map to the owner).
    fn ignore_scope(enclosing: SyntaxNode<'_>) -> Option<SyntaxNode<'_>>;
    /// Expand an own-line `/** text */` into a multi-line star block.
    const EXPAND_DOC_COMMENTS: bool;
}
```

### Document IR

A `Doc` is a `u32` handle into one append-only arena per format run. Text is
always borrowed (`&'a str`: source slices and `&'static` punctuation), so no
string is allocated during layout. Node size: 12 bytes; texts live in a side
table with their precomputed display width (ASCII fast path) and flags.

| Constructor                     | Meaning                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| ------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `text(&'a str)`                 | Literal text. A text containing a line break (a text block literal) is written raw (line breaks as `\n`) and cannot be flat.                                                                                                                                                                                                                                                                                                                                       |
| `sticky(&'a str)`               | A synthesized separator (`,` `;`) that is printed before a pending line suffix instead of flushing it (see printer).                                                                                                                                                                                                                                                                                                                                               |
| `reindented(&'a str, Reindent)` | A multi-line source slice (a block comment, or verbatim source) whose continuation lines the printer rewrites while writing them: `Reindent::Comment` and `Reindent::ExpandedDoc` lay out a block comment's lines (see Printing); `Reindent::Strip(prefix)` removes `prefix` from a line that starts with it. Each continuation line is then written after the indentation of the output line the text started on; blank lines get no indentation. Cannot be flat. |
| `space()`                       | One space; dropped at line start and before a newline; consecutive spaces collapse into one.                                                                                                                                                                                                                                                                                                                                                                       |
| `line()`                        | Space when flat, newline when broken.                                                                                                                                                                                                                                                                                                                                                                                                                              |
| `soft_line()`                   | Nothing when flat, newline when broken.                                                                                                                                                                                                                                                                                                                                                                                                                            |
| `hard_line()`                   | Newline; forces every enclosing group to break.                                                                                                                                                                                                                                                                                                                                                                                                                    |
| `empty_line()`                  | Blank line (two newlines); forces enclosing groups to break.                                                                                                                                                                                                                                                                                                                                                                                                       |
| `concat([Doc])`                 | Sequence.                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| `group(Doc)`                    | Flat if the contents fit, else broken.                                                                                                                                                                                                                                                                                                                                                                                                                             |
| `indent(Doc)`                   | One more indentation level for newlines inside.                                                                                                                                                                                                                                                                                                                                                                                                                    |
| `if_break(broken, flat)`        | Chooses by the mode of the enclosing group.                                                                                                                                                                                                                                                                                                                                                                                                                        |
| `line_suffix(Doc)`              | Deferred until the next newline (or a non-sticky text; see printer).                                                                                                                                                                                                                                                                                                                                                                                               |
| `break_parent()`                | Forces every enclosing group to break.                                                                                                                                                                                                                                                                                                                                                                                                                             |
| `will_break(Doc)` (query)       | Whether the doc contains a forced break; a rule may choose a layout from it (qualified names break every segment when a comment forces a break inside).                                                                                                                                                                                                                                                                                                            |
| `Doc::NIL`                      | Nothing.                                                                                                                                                                                                                                                                                                                                                                                                                                                           |

There is no `fill`, `align`, `force_flat`, `comment_prefix`, `literal_text`,
source claim, or any of the nine `LineMode`s: newline coalescing (below) makes
`*_boundary` and `hard_line_suffix` variants unnecessary, and the reference
style uses neither fill nor alignment (all continuation is one indent level).

Each node stores a `forces_break` bit computed at construction from its children
(`hard_line`, `empty_line`, `break_parent`, multi-line text). A group whose bit
is set is broken without a fit check; this is Prettier's `propagateBreaks`, done
in `O(1)` per constructed node.

Text side-table flags: `sticky`, `line_comment` (set by the comment printer for
line comments; see the printer's "line comments end their line"), and the
multi-line mode (`raw` for `text`, the `Reindent` mode for `reindented`).
Display width is the sum of `unicode-width` widths of the characters with a tab
counting 1, as in the reference (`jolt_fmt_ir/src/width.rs` at `53efc726`), with
an ASCII fast path. The width of a multi-line text for fitting is its first
line's; after printing it, the column is the width of its last line (plus the
indentation written before it, for `reindented`).

Ergonomics: `Formatter` derefs to `DocBuilder`. Because building a doc needs
`&mut`, `jolt_fmt` exports three macros that evaluate their arguments before
borrowing the builder: `cat!`, `group!` and `indent!`, each written
`name![f; doc, doc, …]`. These are the only macros in the formatter crates.

### Printer

The printer is iterative: a command stack of `(indent, mode, doc)`, a line
suffix buffer, and output buffered into chunks for the sink. State that replaces
the old line modes:

- **Newline coalescing.** A newline request (broken `line`/`soft_line`,
  `hard_line`) sets `pending = max(pending, 1)`; `empty_line` sets
  `pending = 2`. Nothing is written until the next non-space text, which first
  writes `pending` newlines and the indentation of the most recent request.
  Consequently a newline at line start is a no-op, two blank-line requests never
  stack, a newline before the first text of the file is dropped, and trailing
  whitespace is never produced (pending spaces are discarded at a newline).
- **Line suffixes.** A broken newline request with pending suffixes first prints
  the suffixes (Prettier semantics: the request is pushed back and the suffix
  contents are printed first, so a suffix may itself contain line breaks).
  Output reaches the sink in chunks of about 1 KiB, so a comparing sink (check
  mode) stops the printer soon after the first difference. Additionally, any
  text that is not `sticky` with pending suffixes first prints the suffixes and
  then a newline at that text's indent. This is what keeps an end-of-line
  comment from being carried past a token: `} // c` followed by `else` prints
  `} // c` / `else`. Only synthesized separators are `sticky` (`b // c` followed
  by a synthesized trailing `,` prints `b, // c`); a represented `,` or `;`
  never follows a pending suffix, because placement rule S1 already attached the
  comment after it.
- **Line comments end their line.** After a text flagged `line_comment`, the
  next text on the same output line (including the next flushed suffix) is
  preceded by a newline at the current indent. Two trailing line comments can
  therefore never be merged into one comment, whatever the rules produce.
- **End of input.** Flush suffixes, then exactly one `\n` (also for an empty
  file).

Spaces collapse in the fit check as they do when printing (a run of `space` and
flat `line` counts one column before the next text, and none at the start of a
line).

Fit check for a group not already forced to break: Prettier's `fits` over the
group's contents in flat mode followed by the rest of the stack in its recorded
modes, with remaining width `line_width - column`. The contents of a line suffix
(an end-of-line comment) count toward the width, as in the reference. It returns
`false` on width overflow and on a forced newline inside the group. When, inside
the group's contents, a non-`sticky` text is reached while a line suffix is
pending (from before the group or from inside it), that text will be preceded by
a forced newline: if a flat `line` or `soft_line` was measured in the group
before it, the group returns `false` (breaking it moves that text); otherwise
breaking cannot change where the text starts, so the check continues from a
fresh line at the text's indentation. It returns `true` at the first newline of
the rest, at a non-`sticky` text of the rest while a suffix is pending, or at
the end. It visits at most `FIT_BUDGET = 4096` commands; exhausting the budget
returns `false` (break). Cost: `O(doc nodes × FIT_BUDGET)` worst case, in
practice `O(width)` per group because the check stops at the line end.

The pending-suffix rule is what makes `x = // c` / `value` and `return // c` /
`value` lay out like the reference: the value's group sees the pending comment,
breaks, and the value continues on the next line at its own indent, where it may
break further by its own rule. If `fits` treated that text as fitting, the group
would print flat and a long value could never break. The fresh-line rule is what
keeps `a ||  // c` / `(b && c)` from breaking the parenthesized operand: its
group starts with text, so it is measured on the line the comment pushes it to.

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
    pub layout: CommentLayout, // from the source; the effective layout is the placement's
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
`Eof` token). The **enclosing node** is the deepest node containing both,
computed lazily (only hooks and debug assertions need it).

The layout is decided from line breaks alone: "newline before" means a line
break between the end of `prev` (or of the previous comment) and the comment's
start, or no `prev`; "newline after" means a line break between the comment's
end and the start of `next` (or of the next comment), or `next` is `Eof`. A
multi-line block comment is classified by its start and end lines the same way.
`lines_before` and `lines_after` count line breaks in the same gaps.

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
    root: SyntaxNode<'a>,
}
impl<'a> CommentContext<'a> {
    pub fn enclosing(&self) -> SyntaxNode<'a>; // computed on demand
}
```

Targets. `leading_target(next)` is the outermost node other than the root whose
first token is `next`, or the token itself when no such node exists. If that
node is a list, the target is instead its child on the path to `next` (repeated
while that child is a list too). `trailing_target(prev)` is the same with last
tokens. So a comment attaches to the outermost element that begins (or ends) at
the adjacent token, never to a list (always to its item, so a member's Javadoc
leads the member, not its `ModifierList`), and never past an adjacent token.
Comments therefore never move across a token, except by S1 (past `,`/`;`) and
hook H1 (into a control statement's body).

Rules, first match wins:

1. `L::place_comment(ctx)`.
2. **G1, empty delimited region.** `prev` opens a region (`closer_for(prev)` is
   `Some`) and `next` is its matching closer (same parent, kind
   `closer_for(prev)`): `Dangling(prev.parent())`. An opener whose closer is
   missing gets no special treatment: the comment falls through to the rules
   below, and attaches to `next` (often `Eof`) outside the unclosed node, which
   is stable and never hides a comment inside a node printed verbatim.
3. **G2, end-of-line after an opener.** `prev` opens a region, the layout is
   `EndOfLine`, and no other comment lies between `prev` and this comment:
   `Leading(leading_target(next), OwnLine)`.
4. **G3, own-line before a closer.** `next` is a closer (`is_closer`) and the
   layout is `OwnLine` or `StartOfLine`: `Dangling(next.parent())`.
5. **S1, end-of-line before a separator.** The comment is a line comment, the
   layout is `EndOfLine`, `next` is a separator (`is_separator`), `prev` is not
   one, and no other comment lies between this comment and `next`:
   `Trailing(trailing_target(next),
   EndOfLine)`. This is the one placement
   that moves a comment across a token (`a // c` `,` prints `a, // c`); doing it
   in placement rather than in the printer keeps the comment's `break_parent` at
   the same depth in the document on every pass, which idempotence needs. (An
   earlier draft extended `next` to a run of separators; the corpus sweep showed
   that moves comments onto removed `;` items after an import or statement, so
   S1 moves past one separator only, and not at all when the comment already
   follows one.)
6. Defaults: `OwnLine`/`StartOfLine` → `Leading(leading_target(next), layout)`;
   `EndOfLine` → `Trailing(trailing_target(prev), EndOfLine)`; `Inline` →
   `Trailing(trailing_target(prev), Inline)` if `next` is a closer or a
   separator and `prev` is not an opener, else
   `Leading(leading_target(next), Inline)`. (`EndOfLine` and `Inline` imply a
   `prev`.) The opener exception keeps `{/* c */;}` stable: the `;` may be
   removed, and a comment trailing the `{` would then be alone in the region.

Two rules then keep the comments of one gap in order, whatever the rules above
chose:

- **Runs.** Once a comment of a gap is placed `Leading` or `Dangling`, a later
  comment of the same gap that would be `Trailing` follows it instead: the same
  target and side, with its own layout (so `Leading` can carry `EndOfLine` here:
  `/* a */ /* b */` alone on a line before code keeps both on that line).
- **After a trailing line comment.** S1 can put a comment after a trailing line
  comment of the same element (`f // a` `;` `/* b */`). Nothing can follow a
  line comment on its line, so such a second trailing comment is placed as if it
  were `OwnLine`, where the next pass finds it.

Hook constraints (checked by debug assertions in the pass): a hook may only
target elements inside the enclosing node; `Leading` layouts are `OwnLine`,
`StartOfLine` or `Inline`; `Trailing` layouts are `EndOfLine` or `Inline`.

**Who can have dangling comments.** Only G1, G3 and hook H1 produce `Dangling`,
always on the parent of an opener or closer token. So the nodes that can carry
dangling comments are exactly the kinds with a delimiter token slot: bodies and
blocks, argument, parameter, type-argument and type-parameter lists, and also
nodes that hold their own parentheses or brackets (Java: method, constructor,
record and annotation-element declarations; `if`, `while`, `do`, `for`,
`switch`, `synchronized`, `catch` headers; `Guard`; `ResourceSpecification`;
`ParenthesizedExpression`, `CastExpression`, `LambdaExpression`,
`ArrayAccessExpression`, `DimExpression`, `ArrayDimension`, `ArrayInitializer`,
`AnnotationArgumentList`, `AnnotationArrayInitializer`, `RecordPattern`). The
rule of every such kind prints `f.dangling(node)` just before its closer, either
directly or by passing the node as `owner` to `delimited` or `braced_body`. A
rule that forgets fails the emission check on the first probe that reaches it.

Storage: comments in a `Vec` in source order; attachments in a `Vec` sorted by
`(target, side, source order)`; two bitsets (`NodeId`, `TokenId`) answer "has
attached comments" in `O(1)`, so elements without comments never search.
Placement is `O(tokens + trivia + Σ_comments a(c))`, where `a(c)` is the number
of nodes that begin or end at the comment's adjacent tokens (the target walk
stops as soon as a parent starts or ends elsewhere). That is bounded by tree
depth and is large only at the head of a deep left-nested spine.

#### Printing

The formatter, not the rules, prints leading and trailing comments, from the
effective layout only:

| Attachment | Layout                              | Printed as                                                                                                          |
| ---------- | ----------------------------------- | ------------------------------------------------------------------------------------------------------------------- |
| Leading    | `OwnLine`                           | `hard_line, C, (empty_line if lines_after ≥ 2 else hard_line)`                                                      |
| Leading    | `StartOfLine`                       | `hard_line, C`, then `space` unless `next` is a closer or separator                                                 |
| Leading    | `Inline`                            | `space` unless `L::hugs_next(prev)`, then `C`, then `space` unless `next` is a closer or separator                  |
| Leading    | `EndOfLine`                         | `space` unless `L::hugs_next(prev)`, then `C, (empty_line if lines_after ≥ 2 else hard_line)` (Runs)                |
| Trailing   | `EndOfLine`, line comment           | `line_suffix(space, C), break_parent`                                                                               |
| Trailing   | `EndOfLine` block comment, `Inline` | `space` unless `L::hugs_next(prev)`, then `C`, then `space` unless `next` is a closer or separator                  |
| Dangling   | any                                 | by the owning rule via `Formatter::dangling`: one per line, except that comments that shared a source line share it |

A block comment that ends its line trails its code like an inline comment: it
does not force the line to break, so `foo(a, /* c */` / `b)` prints
`foo(a, /* c */ b)` when it fits, as in the reference. On the next pass the
comment is `Inline` and leads `b`, which prints the same text.

Because spaces collapse and are dropped at line start, these are correct
whatever separator the rule itself put at that position: `a /* c */ + b`,
`foo(/* c */ a)`, `a /* c */)`, `this /* c */ .field`.

`C` is the comment doc (multi-line block comments follow the reference: a
multi-line doc comment is a star block whose opener text moves to its own `*`
line; a block comment whose continuation lines start with `*` is a star block
that keeps its opener text; every star block ends with `*/` on its own line and
keeps each line's text after the `*` and one space; other multi-line block
comments lose only the indentation their continuation lines share (Java style
C13); an unterminated comment is written raw; dangling doc comments are always
expanded, since they always print on their own lines): a line comment's text
with trailing whitespace removed, flagged `line_comment`; a single-line block
comment's text; a multi-line block comment as
`reindented(text, Reindent::Comment)` (so continuation lines follow the
indentation of the line the comment starts on, star lines get one space before
the `*`); with `EXPAND_DOC_COMMENTS`, a single-line `/** text */` that is
printed on its own line (a `Leading` `OwnLine` comment, or a dangling comment
that was `OwnLine` in the source) is expanded to `/**`, `* text`, `*/`. See
[Java style](./java-style.md#comments).

Why this is idempotent: printing preserves each comment's neighbours (no comment
crosses a token, except across separators by S1, which is decided the same way
on every pass) and its newline-before/newline-after facts, except where the
layout itself put a newline next to an `Inline` or `EndOfLine` comment, or
removed one after an end-of-line block comment (which prints the same as the
`Inline` comment it becomes, and a rule that chooses a layout from leading
comments asks whether they start a line, not whether there are any). In that
case every enclosing group was already broken (by `break_parent`, a multi-line
text, or the pending-suffix fit rule), so on the next pass the comment (now
`StartOfLine` or `EndOfLine`) forces exactly the breaks that already happened
and the output is unchanged. A synthesized `sticky` separator may move ahead of
an end-of-line comment once (`b // c` + synthesized `,` → `b, // c`); on the
next pass the separator is represented and S1 attaches the comment after it,
inside the same groups, so the result is a fixpoint. These arguments are checked
by the probes test and the corpus sweep, not by proof; a failure there is a
design bug to fix here, not a case to special-case in a rule.

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
    /// Leading comments, the token text, trailing comments. Never sticky.
    pub fn token(&mut self, token: SyntaxToken<'a>) -> Doc;
    pub fn token_opt(&mut self, token: Option<SyntaxToken<'a>>) -> Doc;
    /// A node's leading and trailing comments, for a rule that builds the
    /// node's doc from its children itself instead of through `node`: the
    /// iterative spines and the `else if` chain, where an interior node's
    /// leading comments go before the head and its trailing comments after
    /// the segment that ends it (see Deep spines); the readability
    /// parentheses of N7, which go between an operand and its comments; and
    /// the declaration header, whose last clause's trailing comments print
    /// outside the header group. The node must not be malformed.
    pub fn leading_comments(&mut self, node: SyntaxNode<'a>) -> Doc;
    pub fn trailing_comments(&mut self, node: SyntaxNode<'a>) -> Doc;
    /// Every comment attached to or inside `element`, in source order and in
    /// their own layouts, without the element's text. For tokens or nodes a
    /// normalization removes. Returns NIL when there are none.
    pub fn removed(&mut self, element: SyntaxElement<'a>) -> Doc;
    /// The node's source from its first token's text to its last token's
    /// text, interior comments included and marked emitted, as
    /// `reindented(slice, Reindent::Strip(p))` where `p` is the indentation of
    /// the source line the node starts on. Text block literals inside are
    /// emitted as raw `text` pieces between the reindented pieces, so their
    /// content never changes. Re-indenting relative to the output line (not
    /// the document indent) is what makes verbatim output a fixpoint even when
    /// the node starts mid-line inside an `indent`.
    pub fn verbatim(&mut self, node: SyntaxNode<'a>) -> Doc;
    /// The node's dangling comments, each on its own line (`empty_line`
    /// between two when the source had a blank line, `space` when they shared
    /// a source line), with no line before the first or after the last. NIL
    /// when none.
    pub fn dangling(&mut self, node: SyntaxNode<'a>) -> Doc;

    // ── Queries; they emit nothing. ──
    pub fn has_leading_comments(&self, element: SyntaxElement<'a>) -> bool;
    pub fn has_trailing_comments(&self, element: SyntaxElement<'a>) -> bool;
    pub fn has_dangling_comments(&self, node: SyntaxNode<'a>) -> bool;
    /// A trailing line comment (no trailing comma is synthesized after it).
    pub fn has_trailing_line_comment(&self, element: SyntaxElement<'a>) -> bool;
    /// Some leading comment prints a line break between the code before the
    /// element and the element (its effective layout is not `Inline`).
    pub fn leading_comments_break(&self, element: SyntaxElement<'a>) -> bool;
    /// Some trailing comment ends its line.
    pub fn trailing_comments_break(&self, element: SyntaxElement<'a>) -> bool;
    /// Any comment attached to the node or located inside its range.
    pub fn has_comments_within(&self, node: SyntaxNode<'a>) -> bool; // O(log comments)
    /// The node is an item inside a formatter-ignore region, so a
    /// normalization (N1, N3) must leave it in place.
    pub fn in_ignore_region(&self, node: SyntaxNode<'a>) -> bool;
    /// The source has a blank line before the element's first leading comment
    /// (or its first token when it has none), counted from the previous token
    /// or comment. Used for `Separator::blank_line_in_source` and before the
    /// leading comments of `Eof`.
    pub fn blank_line_before(&self, element: SyntaxElement<'a>) -> bool;
    /// The source has a blank line before the node's first dangling comment.
    /// `delimited` and `braced_body` put `empty_line` instead of `hard_line`
    /// between the last item and the dangling comments when this holds.
    pub fn blank_line_before_dangling(&self, node: SyntaxNode<'a>) -> bool;

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
  `blank_line_in_source`. Between two items that are both inside one ignore
  region, and between the last item of a region and the item after it,
  `item_list` does not call `separator` and uses the source's choice
  (`empty_line` iff `blank_line_in_source`), so a region prints as written.
  Nothing is emitted before the first item or after the last, so bodies never
  start or end with a blank line.
- **Elision.** An `Elide` item prints `removed(item)`; if that is NIL the item
  vanishes (no separator), otherwise its comments stand in its place. Its
  trailing comments that lie before it in the source (moved onto it by S1, as in
  `} // c` then a stray `;`) end the previous item's line: they are printed
  before the separator, unless an item vanished in between. The last comment of
  an elided item, when it is an own-line leading comment, is not followed by its
  blank line itself; instead a blank line after it in the source counts as a
  blank line before the next item (`Separator::blank_line_in_source`), so every
  body keeps the source's blank line there.
- **Formatter-ignore.** An item (`Format` or `Elide`) whose first token's text
  starts inside an ignore region prints `node_with(item, verbatim(item))`: its
  own leading and trailing comments (including the markers) are formatted
  normally, its interior is verbatim, and it is never elided. This check comes
  before elision.

Ignore regions are computed once after placement. A marker is a comment whose
body, after stripping `//` or `/* */` and surrounding whitespace, is exactly
`@formatter:off` or `@formatter:on`. A marker's **list** is the innermost
item-list owner (Java: compilation unit, type body, block, constructor body,
switch block statement group, module body) whose range contains the marker while
no item of that list contains it, i.e. the marker sits between items, or before
the first or after the last item; a marker inside an item's interior (say, in an
argument list) is in no list. Scanning markers in source order, an `off` that is
in a list opens a region at its end; the next `on` closes it at its start (later
`off`s inside a region are ignored). The region is kept only if that `on` is in
the same list; otherwise it is dropped and has no effect, which is the reference
behaviour for an `off`/`on` pair that crosses a body boundary (fixture
`formatter-ignore-nested-member-body`). An `off` with no later `on` at all ends
at the start of its list's closing token, or at end of file at top level. Kept
regions are sorted and disjoint; membership is a binary search, `O(log regions)`
per item, and finding a marker's list walks its ancestors once. Nested lists
inside a verbatim item are never visited; nested lists inside a formatted item
apply the same test to their own items.

Two refinements make verbatim robust for formatter-ignore items and malformed
nodes. First, `verbatim` also prints the comments attached to the descendants
that begin or end at the node's first or last token (a list's first item's
leading comments lie before the list), so a verbatim list never loses them.
Second, a comment inside the slice can be attached outside the node by S1
(`return x // c` `;` with a verbatim `return`): verbatim marks the comments it
prints, and attachment printing skips a comment verbatim already printed. Every
verbatim piece of one node re-indents against the indentation of the output line
the node starts on (recorded at its first text), also after a text block.

Malformed nodes use the same `verbatim`: `node()` on a malformed node is
`node_with(node, verbatim(node))`. Verbatim output covers exactly the comments
strictly between the node's first and last token; placement never attaches those
to anything outside the node (every target rule picks a node that begins or ends
at an adjacent token, and G1 needs the closer to be present), and attaches a
comment outside that range to something inside it only through H1, which
requires the body it targets not to be malformed. So nothing is lost or
duplicated at the boundary (#224).

### Emission check

`Formatter` keeps one emission counter per comment in every build. After the
document is built, `format` asserts every counter is exactly 1 when
`cfg!(debug_assertions)` is true, panicking with each offending comment's text
and position. Release builds skip the check but keep the same layout. This
replaces `SyntaxConservationTracker`, source claims, and the debug double
render. Tokens need no tracking: the oracle's structure fingerprint covers them.
The count is taken at construction, so it catches duplicates and comments never
reached, but not a doc that was built and then dropped by a rule; the oracle's
comment multiset catches that.

## `jolt_java_fmt`

### Module layout and ownership

```text
crates/jolt_java_fmt/
  src/lib.rs                    shared   pub fn format, JavaFormat, type JavaFormatter
  src/language.rs               shared   impl FormatLanguage: separator/delimiter tables, hooks H1–H3, H5 (H4 is in rules/statements/control.rs)
  src/dispatch.rs               shared   total match: node kind → rule function
  src/helpers/mod.rs            shared
  src/helpers/lists.rs          shared   delimited comma lists, one-element delimited nodes, list items
  src/helpers/bodies.rs         shared   braced bodies, member/statement item lists, blank-line policy
  src/helpers/assignment.rs     shared   `left = right` layout
  src/rules/mod.rs              shared   declares the four area modules
  src/rules/declarations/**     Area A
  src/rules/statements/**       Area B
  src/rules/expressions/**      Area C
  src/rules/types/**            Area D
  examples/jhunt.rs jcheck.rs jref.rs                                      shared
  tests/corpus.rs recovery_snapshots.rs corpus_fixtures.rs probes.rs
        deep.rs common/                                                    shared
```

Test file names `corpus.rs` and `recovery_snapshots.rs` are kept because insta
derives snapshot names from them (`corpus__…`, `recovery_snapshots__…`);
renaming either would orphan the reference snapshots.

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
`mod.rs`. `dispatch.rs` maps each kind to a small function that calls exactly
these names (a table of function pointers rather than one large `match` frame,
so the depth tests' 128 nesting levels fit the 1 MiB stack in debug builds), so
an area can restructure its files freely as long as the entry points keep their
names and signatures. `Bogus` has no rule: it is always `MALFORMED`, so `node()`
prints it verbatim before dispatch, and its dispatch arm is `unreachable!`.

List kinds never carry comments (placement attaches to their items) and are
never malformed (factory rule 3), so a parent may iterate a list's elements
directly. A list kind therefore has a rule function only when some other rule
formats it as a unit with `f.fmt`; those are exactly the lists in the
[cross-area contracts](#cross-area-layout-contracts) (`ModifierList`,
`ParameterModifierList`, `VariableDeclaratorList`, `ArrayDimensions`,
`AnnotationList`). Every other list kind is laid out by its parent's rule and
dispatches to `helpers::lists::unexpected_list`, which `debug_assert!`s and
formats the children separated by spaces (`dispatch.rs`).

### Java comment hooks

`JavaFormat::place_comment` implements only these exceptions to the generic
rules. "Header end" below means the `)` closing the header of `if`, `while`,
`for` (basic or enhanced), `switch` (statement or expression), `synchronized`,
`catch`, or the resource specification of a `try`, or the keyword `else`, `do`,
`try`, or `finally`, and "the body" is the statement or block that follows it.

- **H1, comment between a control header and its body.** `prev` is a header end,
  `next` is the first token of the body, the body is not malformed, and the
  comment is an `EndOfLine` comment or an `OwnLine`/`StartOfLine` line comment:
  `Leading(t, OwnLine)`, where `t` is `leading_target` of the first token inside
  the body block when the block has one, or the body statement itself when it is
  not a block (brace promotion will wrap it); if the body block is empty,
  `Dangling(body)`. Exception: after `else` whose body is an `if` (an `else if`
  chain), H1 does not apply and the generic rules give `} else // c` / `if (…)`.
  This moves the comment across the body's `{` (C7); it is stable because on the
  next pass the comment is own-line inside the body. All comments between a
  header end and its body go the same way, so their order is kept.
- **H2, type-argument angles.** `<`/`>` of `TypeArgumentList` and
  `TypeParameterList` are openers/closers (via `closer_for`/`is_closer`); `<`
  and `>` as operators are not.
- **H3, comment on its own line before a body's `{`.** `next` is the `{` of a
  body or block that follows a header in the same parent (a type body, method,
  constructor or lambda body, an initializer block, an anonymous class or enum
  constant body, a switch block, a control statement body block) and the layout
  is `OwnLine` or `StartOfLine`, and H1 did not apply: a block comment becomes
  `Trailing(trailing_target(prev), Inline)` and a line comment
  `Trailing(trailing_target(prev), EndOfLine)`. This reproduces the reference
  (`class X` / `/* c */` / `{` prints `class X /* c */ {`; `class X` / `// c` /
  `{` prints `class X // c` / `{`; fixture
  `leading-comments-before-body-braces`) and is stable: on the next pass the
  block comment is `Inline` before `{` and leads the body inline, and the line
  comment is `EndOfLine` after the header. A `{` that starts a statement (a
  nested block, `case 1: {`) has no header before it in the same parent, so H3
  does not apply and the comment leads the statement on its own line.

- **H4, end-of-line comment after a promoted body.** The comment is `EndOfLine`
  and `prev` is the last token of the final non-block body of an `if`, `else`,
  `while`, `for` or enhanced `for` (not an `else if` chain):
  `Trailing(body, EndOfLine)`. The generic rules would make the comment trail
  the whole statement and print after the synthesized `}` of brace promotion; H4
  keeps `if (a) return; // c` inside the block, as in the reference (fixture
  `brace-promotion-comments`).
- **H5, block comment at the end of a line after `(`, `[`, `<` or an assignment
  operator.** A block comment with layout `EndOfLine` right after one of these
  openers (not before its closer, where G1 applies) or after `=`, `+=`, … is
  `Leading(leading_target(next), Inline)`: it stays inline before the next
  token, as in the reference (`run( /* d */` / `a)` prints `run(/* d */ a)`),
  and is where the next pass, reading it as inline, puts it (after `=` it is
  part of the value, so a long declaration breaks the same way on both passes;
  Java style C14). G2 still moves it to its own line after `{`.

Anything else that looks like it needs a hook should first be checked against
the generic rules; a new hook requires a fixture and an entry here.

### Helpers (foundation-provided)

```rust
// helpers/lists.rs
pub(crate) enum TrailingComma {
    /// Argument, parameter and type-argument lists (Java forbids one there).
    Never,
    /// Array and annotation-array initializers: a synthesized `,` when broken,
    /// except after a last element with a trailing line comment (`last // c`
    /// / `}`, as in the reference); a trailing `,` in the source forces the
    /// broken layout and is printed with `token`.
    IfBroken,
}
pub(crate) struct ListLayout {
    pub padded: bool, /* "{ a, b }" when flat */
    pub trailing_comma: TrailingComma,
}
/// `open item, item close` with the reference's break-all layout: flat if it
/// fits, else every element on its own line at one more indent and `close` on
/// its own line. `owner` is `open.parent()` (also when `open` is missing, the
/// node that would own it); its dangling comments are printed after the last
/// element, on their own lines, preceded by `empty_line` when
/// `blank_line_before_dangling(owner)`. Represented separators go through
/// `token`, synthesized ones through `sticky`; `Missing` elements print
/// nothing. Comma-separated lists only: a resource specification (`;`,
/// always broken) is laid out by Area B itself.
pub(crate) fn delimited<'a, T: AstNode<'a>>(
    f: &mut JavaFormatter<'a>,
    owner: SyntaxNode<'a>,
    open: Option<SyntaxToken<'a>>,
    elements: impl Iterator<Item = ListElement<'a, T>>,
    close: Option<SyntaxToken<'a>>,
    layout: ListLayout,
) -> Doc;
/// `open inner close` for a node with one inner element (array access,
/// dimension expressions, casts): no breaks of its own; `inner` is indented
/// when asked, and the owner's dangling comments print on their own lines
/// before `close`.
pub(crate) fn enclosed<'a>(
    f: &mut JavaFormatter<'a>,
    owner: SyntaxNode<'a>,
    delimiters: (Option<SyntaxToken<'a>>, Option<SyntaxToken<'a>>),
    inner: Doc,
    indent_inner: bool,
) -> Doc;
/// One list element: the item with `fmt`, a bogus node with `node`.
pub(crate) fn list_item<'a, T: AstNode<'a>>(
    f: &mut JavaFormatter<'a>,
    element: ListElement<'a, T>,
) -> Doc;

// helpers/bodies.rs
/// `{`, the items at one more indent joined by `item_list` with `separator`,
/// the owner's dangling comments, `}` (via `braced`).
pub(crate) fn braced_body<'a>(
    f: &mut JavaFormatter<'a>,
    owner: SyntaxNode<'a>,
    open: Option<SyntaxToken<'a>>,
    items: impl IntoIterator<Item = ListItem<'a>>,
    close: Option<SyntaxToken<'a>>,
    separator: impl FnMut(&mut JavaFormatter<'a>, Separator<'a>) -> Doc,
) -> Doc;
/// `open`, already formatted `content` and the owner's dangling comments one
/// indent deeper on their own lines, `close`; `{}` when both are empty (D1).
/// Used by `braced_body`, enum bodies (constants then members) and brace
/// promotion (synthesized braces, no owner).
pub(crate) fn braced<'a>(
    f: &mut JavaFormatter<'a>,
    owner: Option<SyntaxNode<'a>>,
    open: Doc,
    content: Doc,
    close: Doc,
) -> Doc;
pub(crate) fn member_separator<'a>(f: &mut JavaFormatter<'a>, sep: Separator<'a>) -> Doc; // blank unless field–field
pub(crate) fn statement_separator<'a>(f: &mut JavaFormatter<'a>, sep: Separator<'a>) -> Doc; // blank iff source had one

// helpers/assignment.rs
/// `left space op` then `group(indent(line, right))`. With a trailing comment
/// on `op` (`x = // c`), the pending-suffix fit rule breaks the group, so the
/// value starts the next line one indent deeper, as in the reference.
pub(crate) fn assignment_like<'a>(
    f: &mut JavaFormatter<'a>,
    left: Doc,
    op: Option<SyntaxToken<'a>>,
    right: Doc,
) -> Doc;
```

### Cross-area layout contracts

What a node's `Doc` contains when another area formats it with `f.fmt`. Every
doc excludes surrounding spaces unless stated.

| Kind (owner)                                            | Doc contract                                                                                                                                                                                                                                                                                                                                                                       |
| ------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `ModifierList`, `ParameterModifierList` (D)             | Annotations and keywords **including** the trailing separator: `hard_line` after declaration annotations (those before the first keyword), `space` after the last keyword/type-use annotation. NIL when empty. Via `f.fmt` this is the declaration layout, used for fields, methods, types, local variables, enhanced-for variables and resources.                                 |
| `types::inline_modifiers(f, ParameterModifierList)` (D) | The inline layout: every annotation and keyword on one line, separated by spaces, followed by `space`; NIL when empty. A plain `pub(crate)` function (lists carry no comments, so no `f.fmt` is needed), used for `FormalParameter` (D), `RecordComponent` (A) and `CatchParameter` (B), as in the reference (`void m(@Nonnull String p)`, `catch (@Nonnull final Exception ex)`). |
| `AnnotationList` (D)                                    | Type-use annotations inline: separated by spaces, followed by `space`; NIL when empty. `types::annotation_lines(f, Option<AnnotationList>)` lays out package, module and enum-constant annotations one per line.                                                                                                                                                                   |
| `ArrayDimensions` (D)                                   | `[]` per dimension, an annotated one preceded by its annotations and a space (`@A []`); no space before an unannotated `[`.                                                                                                                                                                                                                                                        |
| `Name`, `QualifiedName` (D)                             | The dotted name on one line; never breaks.                                                                                                                                                                                                                                                                                                                                         |
| `Block` (B)                                             | `{ … }` from `braced_body` with `statement_separator`; caller adds the space before.                                                                                                                                                                                                                                                                                               |
| `ClassBody`, `InterfaceBody`, `EnumBody`, … (A)         | `{ … }`; `ClassBody` is used by C for anonymous classes.                                                                                                                                                                                                                                                                                                                           |
| `ArgumentList` (C)                                      | `( … )` via `delimited`; used by A (enum constants, explicit constructor invocations).                                                                                                                                                                                                                                                                                             |
| `FormalParameter`, `ReceiverParameter` (D)              | One parameter. The parens and break-all list are A's: the declaration rule passes `parameters().elements()` to `delimited`.                                                                                                                                                                                                                                                        |
| `TypeParameterList`, `TypeArgumentList` (D)             | `< … >` group; used by A, C.                                                                                                                                                                                                                                                                                                                                                       |
| `VariableDeclaratorList` (D)                            | `name = init, …` with `assignment_like`; continuation declarators on new lines at one indent when broken.                                                                                                                                                                                                                                                                          |
| `Expression` category (C), `SwitchExpression` (B)       | A complete expression; never includes a trailing `;`.                                                                                                                                                                                                                                                                                                                              |
| Patterns (D)                                            | Used by C (`instanceof`) and B (case labels).                                                                                                                                                                                                                                                                                                                                      |
| Statements (B)                                          | Include their `;`. Bodies of control statements are formatted by B only.                                                                                                                                                                                                                                                                                                           |

If an area needs a different contract, it requests it in its result instead of
working around another area's output.

Comment-driven layout rules that a single area owns:

- **Keyword values (B).** `return`, `throw`, `yield`, `break`, `continue` and
  `assert` print `keyword space value` normally. When the keyword itself has
  trailing comments (`return // c` / `y + 1;`), they print
  `keyword group(indent(line, value))` instead: the pending-suffix fit rule
  breaks the group, so the value continues one indent deeper and may break
  further by its own rule (reference fixture
  `comments-between-statement-keyword-and-value`).
- **Promoted empty bodies (B).** A non-block body that is an `EmptyStatement`
  (`if (a);`) promotes to an empty block: the `;` is elided through `removed`,
  exactly as an `EmptyStatement` inside a block is, so the first output is the
  fixpoint.

### Normalizations

Normalizations apply only when the owning node `is_recovery_free()`. Removed
tokens print their comments through `removed`. The structure fingerprint
(`jolt_java_fmt/tests/common`) ignores exactly these changes.

| #   | Normalization                                  | Where                                                                                                                                                                                       | Extra condition                                                                                                                                                                                                     |
| --- | ---------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| N1  | Import grouping and sorting                    | Imports of a compilation unit                                                                                                                                                               | Per import: recovery-free, name without interior comments. An elided item whose comments survive (a stray `;` carrying a comment) is a barrier like a non-sortable import. See Java style.                          |
| N2  | Modifier ordering                              | Declaration `ModifierList`                                                                                                                                                                  | Keyword runs between comment-carrying or bogus modifiers sort independently.                                                                                                                                        |
| N3  | Module directive and `requires` modifier order | `ModuleDeclaration`                                                                                                                                                                         | Directive recovery-free; comment-carrying directives anchor like imports.                                                                                                                                           |
| N4  | Brace promotion                                | Non-block bodies of `if`, `else`, `while`, `do`, `for`, enhanced `for`                                                                                                                      | Body is not malformed.                                                                                                                                                                                              |
| N5  | Redundant `;` removal                          | `EmptyStatement` in blocks and switch groups and as a promoted body, `EmptyDeclaration` in compilation units and type bodies, trailing `;` of try resources, redundant enum body separators | When an enum's constants end with `;`, a removed trailing comma's comments, and the `;`'s own leading comments, print after a blank line following `B;` (`B, // c` `;` → `B;`, blank, `// c`), as in the reference. |
| N6  | Redundant parenthesis removal                  | Parenthesized left operand of a same-precedence binary chain; switch guard `when (x)`; single untyped lambda parameter `(x) ->`                                                             | `has_comments_within` is false for the parenthesized node (or, for the lambda, the parameter list's parentheses carry no comments).                                                                                 |
| N7  | Readability parentheses                        | Binary operands of `&`, `\|`, `^`, `<<`, `>>`, `>>>` that are themselves binary expressions                                                                                                 | —                                                                                                                                                                                                                   |
| N8  | Trailing commas                                | Array and annotation array initializers (iff broken); enum constants (per Java style)                                                                                                       | —                                                                                                                                                                                                                   |
| N9  | Comment text                                   | Trailing whitespace, re-indentation, star alignment, doc comment expansion                                                                                                                  | Comment content otherwise unchanged.                                                                                                                                                                                |
| N10 | Whitespace                                     | Line endings become `\n`; one final newline; BOM and trailing SUB preserved                                                                                                                 | —                                                                                                                                                                                                                   |

There is no import deduplication; the reference does not deduplicate.

### Deep spines

The parser builds left-deep binary/`instanceof` chains and receiver chains
iteratively, so their depth is unbounded. The receiver spine is every kind the
postfix loop wraps around its receiver: `MethodInvocationExpression` with
`QualifiedMethodInvocation`, `FieldAccessExpression`, `ArrayAccessExpression`,
`PostfixExpression`, `MethodReferenceExpression`, `ClassLiteralExpression`, and
the qualified forms of `ObjectCreationExpression`, `ThisExpression` and
`SuperExpression`. Their rules (Area C) must not recurse through `f.node` along
the spine: they walk the spine with an explicit loop, format the leaves with
`f.node`, and emit each interior spine node's comments with `leading_comments`
(before the head) and `trailing_comments` (after the segment the node ends
with), its dangling comments in its segment, and `removed` for parentheses
removed by N6. A malformed spine node stops the walk and is formatted with
`f.node`. The `else if` chain is walked the same way by `if_statement`. Every
other recursion is bounded by the parser's `MAX_DEPTH`. `tests/deep.rs` formats
a 20 000-long `else if` chain, a 20 000-term `+` chain, a 20 000-call chain, a
20 000-deep `a[0][0]…` and a 20 000-deep `a.new B().new B()…` on a 256 KiB
thread, and the deepest accepted nesting of each recursive construct on a 1 MiB
thread (the WASM stack).

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
difference (`jolt_cli::format::Compare`); after `Formatted { halted: false }`
the CLI also checks that the whole source was consumed (the output may be a
strict prefix of the source).

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
3. **Comments.** The multiset of comments (kind and canonical text: blank lines
   and trailing whitespace dropped, star prefixes and one space after them
   stripped, the indentation all continuation lines share removed, doc expansion
   undone) of `F` equals that of `S`. Relative indentation inside a comment is
   compared.
4. **Idempotence.** `format(F) == F`.
5. **Tokens.** For dirty `S` (whose recovered tree the structure check cannot
   compare), the multiset of token texts of `F` equals that of `S`, ignoring
   `, ; { } ( )` and counting the `>` family by character.

### Tests

- `jolt_java_syntax/tests/parser_fixtures.rs`: `debug_tree` and diagnostics
  snapshots for `fixtures/java/syntax/**` (regenerated once by the syntax step;
  the diff must show only owner removal, `Bogus` unification, and `>` tokens).
- `jolt_java_syntax/tests/codegen/`: generated code is current.
- `jolt_java_fmt/tests/corpus.rs`: every fixture under `fixtures/java/` except
  `syntax/lexer/` and `syntax/recovery/` (the same set as today, including
  `syntax/parser/`, `trivia/` and `upstream-docs/`) is formatted, checked with
  the oracle at widths 40, 80 and 120, and snapshotted at 80. Snapshot names
  stay `corpus__<dir>__<file>.snap`. A fixture expected to carry diagnostics
  gets the oracle without a snapshot, as today.
- `jolt_java_fmt/tests/recovery_snapshots.rs`:
  `fixtures/java/syntax/recovery/**` with the oracle and
  input/output/diagnostics snapshots. These snapshots are not part of the
  normative reference; their diffs are reviewed for loss, duplication and
  stability only.
- `jolt_java_fmt/tests/corpus_fixtures.rs`: the oracle at width 80 over the
  imported upstream fixture suites
  (`tools/import/.imports/{google-java-format,
  palantir-java-format,prettier-java}/input`),
  as today, adapted to the new API. This is the only imported-corpus check CI
  runs; the sweep is too slow.
- `jolt_java_fmt/tests/probes.rs`: every comment probe shape at every token
  boundary of the fixtures under `fixtures/java/style/comments/` and
  `fixtures/java/trivia/`, plus the construct list of today's
  `comment_positions.rs`, with oracle checks 1, 3 and 4. It must stay under ten
  seconds in a debug build; the sweep covers everything else.
- `jolt_java_fmt/tests/deep.rs`: the stack-depth tests (see Deep spines), and a
  fit-budget test in `jolt_fmt` (`printer.rs`).
- The old `structure_fingerprint.rs`, `trivia_conservation.rs`,
  `comment_positions.rs`, `layout_fit_boundaries.rs`,
  `jolt_java_syntax/tests/normalization.rs`, diagnostic-ownership audits, and
  schema physical audits are folded into the above or deleted.

### Corpus tools

- `mise run hunt` (`examples/jhunt.rs`): the sweep over the repository fixtures
  and `tools/import/.imports` with the oracle at three widths and the four
  comment probe shapes: every boundary at every width for the fixtures and the
  gjf, palantir and prettier inputs up to 4 KiB, sampled boundaries at width 80
  for larger files and Spring. `KNOWN_OPEN` must be empty. `--mutate` also
  deletes each token of those small files in turn (the oracle's token check
  catches code loss); that recovery stress test is outside the gate.
- `examples/jcheck.rs`: replays one file or one variant (`File.java:ownline12`,
  `File.java:comment3@40`, `File.java:delete7`).
- `examples/jref.rs` (new): `jref <dir>` checks `format(F) == F` for every
  `.java` file (or the `formatted:` section of every `.snap`) under `<dir>`;
  `jref --input <in> --expected <out> [--line-width N]` checks
  `format(in) == out` file by file. It prints totals, writes diffs to
  `target/jref/` with each mismatch's first differing line. Input files with
  syntax errors are skipped and counted separately (the reference CLI left them
  untouched, and the new CLI refuses them too). The reference set is
  `tools/import/.imports/.jolt-ref/` (`input/`, `old-output/`,
  `old-output-w100/`, `jolt-old`, `bench/`), which `tools/jolt-ref.sh build`
  recreates from commit `de12e2cb`, and the snapshots at commit `53efc726`.
  `tools/jolt-ref.sh compare <binary>` counts the files another binary formats
  differently. `import.py` keeps dot-named entries of `.imports` when it
  re-imports.

### Performance

Budget: `jolt format --no-config --check` over
`tools/import/.imports/.jolt-ref/bench/spring-framework` must not be slower than
the reference binary `jolt-old` beside it, with default threads and with
`--threads 1`
(`hyperfine -N -i --warmup 3 --runs 10 '<jolt> format --no-config
--check bench/spring-framework'`;
the recorded baseline was a median wall of 109.5 ms and 1141.8 ms on a 32-core
machine). Compare against `jolt-old` measured back to back on a quiet machine
rather than against the recorded numbers.

Design levers already in this contract: no per-token comment work without a
bitset hit; borrowed text only; 12-byte doc nodes with precomputed widths and
break bits; early sink halt in check mode; no debug-only work in release. If the
budget is missed, the next levers are reusing the token, node, and doc vectors
across files on a thread, and profile-guided builds (`tools/pgo.py`).

Measured at integration (back to back): median wall 107.0 ms against
`jolt-old`'s 121.4 ms with default threads, and 1.132 s against 1.160 s with
`--threads 1` (non-PGO release builds of both; the dprint plugin formats the
corpus in 0.34 s). Two fixes got there: syntax cursors compare and hash by tree
and id (the derived impls compared the whole source string, which made comment
placement scan the file per comparison), and check mode halts at the first
difference again (the printer hands the sink 1 KiB chunks). In a single-threaded
run over Spring, parsing is about 0.47 s, comment placement 0.04 s, document
construction 0.48 s and printing 0.20 s; document construction (many small
allocations in the rules) is the next target.

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
`schema.rs` macros (replaced by the schema data in `tests/codegen/schema.rs` and
the `generated.rs` it produces), the hand-written projections, `classify`,
`as_*`, `partitioned_items` and `define_*_projection` macros in `nodes/mod.rs`,
`JavaOperator` and `JavaOperatorKind` composites, the five composite operator
kinds, 27 of the 28 bogus kinds, `split_token`, all `parse_excessive_*`
functions, the diagnostic owner/slot arguments at every parser call site,
`JavaParseDiagnosticCode` `InvalidEventStream`, `structural_diagnostic_owners`,
`tests/normalization.rs`.

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

## History

The rewrite was built in stages, each gated on `mise run test`,
`mise run
check`, `mise run hunt` and the reference diff: the syntax foundation
(arena tree, factory, parser infrastructure, schema and generated code), the
formatter foundation (document IR, printer, comment placement, item lists,
formatter-ignore, verbatim, the oracle and the sweep), the Java rules in four
areas (declarations, statements, expressions, types and comments), and
integration (merging the areas, reviewing every snapshot diff against the
reference, applying D1). Every remaining difference from the reference is
classified under Java style's "Deliberate changes".

## Changes to project invariants

This design keeps every invariant in `AGENTS.md` and makes these precise:

- "Formatter rules must preserve represented source tokens and their trivia":
  rules emit tokens only through `Formatter::token`, and comments only through
  the chokepoints (`node`, `token`, `verbatim`, `removed`, `dangling`, and
  `leading_comments`/`trailing_comments` for the rules that build a node's doc
  themselves: spines, the `else if` chain, N7 parentheses and the declaration
  header). Rules never choose how a comment prints.
- "Verbatim formatting must track every contained token and comment": verbatim
  covers exactly the comments strictly inside the node, and the emission check
  verifies every comment is emitted once.
- Normalizations are the closed list N1–N10, gated on `is_recovery_free()`.

## Decisions

Recommended defaults for questions this page would otherwise leave open. Follow
them unless this page is amended.

1. **Schema form.** The Java schema is Rust data in the codegen test, not a text
   DSL, so no schema parser exists (see [Schema](#schema-and-generated-code)).
2. **Reference location.** The normative reference outputs and `jolt-old` live
   in `tools/import/.imports/.jolt-ref/`, which `tools/jolt-ref.sh build`
   recreates.
3. **Unclosed delimiters.** G1 needs the matching closer. A comment after an
   opener whose closer is missing attaches by the generic rules, usually to
   `Eof`, and prints at the outer indentation. This fixes #222 (the first pass
   is the fixpoint) without letting a comment attach inside a node that may be
   printed verbatim. Java style's C2 shows the output.
4. **Separators and comments.** Only `,` and `;` pull an end-of-line line
   comment past themselves (S1; a block comment stays, `1 /* c */;`, as in the
   reference), and only synthesized ones are `sticky` in the printer. Closers do
   not: every layout that may break before a closer already puts a line there,
   and the reference keeps `a[i // c` / `]`.
5. **Inline comment spacing.** One space on each side, none after `L::hugs_next`
   tokens, none before a closer or separator. `this /* c */.x` becomes
   `this /* c */ .x` (listed under C4).
6. **Comments before a body brace.** H3 keeps the reference's hoisting of an
   own-line comment before `{` onto the header line; H1 keeps C7. A block
   comment before a control statement's `{` is hoisted like a declaration's
   (reference: `if (a) /* c */ {`); a line comment there moves into the body.
7. **Formatter-ignore scope.** Regions pair markers within one item list only;
   pairs that cross a body boundary are dropped, matching the reference. Items
   in a region keep their source separation (blank line or not) but each starts
   its own line (Java style F1).
8. **Verbatim and text blocks.** Verbatim re-indents relative to the output line
   it starts on and never touches text block lines.
9. **Recovery snapshots** are not normative; their diffs are reviewed only for
   loss, duplication and stability. The normative set is the `corpus__*`
   snapshots and the `.jolt-ref` outputs.
10. **Test scope in CI.** `corpus.rs`, `recovery_snapshots.rs`,
    `corpus_fixtures.rs`, `probes.rs` (comment and trivia fixtures only) and
    `deep.rs` run in `mise run test`; the full probe sweep over the imported
    corpora is `mise run hunt` only.
11. **Empty switch blocks and module bodies.** D1 applies to every
    `braced_body`, including switch blocks, module bodies and enum bodies, so
    the helper has no per-kind exception.
12. **Unicode escapes.** `JavaParse` holds `&str`: the tree always refers to the
    raw source, so the output keeps escapes exactly as written.
