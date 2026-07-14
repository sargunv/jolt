# Formatter Recovery Architecture

Status: PROPOSED. This document defines the lossless Java and Kotlin formatter
architecture that replaces the recovery strategy attempted by the original
formatter-debt roadmap.

The design intentionally follows Biome's broad shape: a lossless CST with
grammar-owned fields, structured formatting for valid nodes, syntax-owned
malformed/bogus nodes formatted verbatim, and debug/test accounting that proves
tokens and comments were handled exactly once.

## Decision

Jolt will not build a second formatter-facing tree or materialize an ordered
`SyntaxPart` collection for every node.

The syntax tree remains the only structural representation. Its child entries
become generated grammar slots in the same flat arena. The parser creates narrow
category-bogus children for consumed invalid spans; the syntax factory inserts
empty slots and validates each requested node. Generated constant-time accessors
read those slots. Formatter rules structurally format valid nodes and emit only
syntax-owned malformed/bogus subtrees verbatim.

Jolt already has the typed-view half of this architecture. `SyntaxNode` is a
small parent-aware borrowed cursor over the parse-owned arena, and wrappers such
as `BinaryExpression`, `Block`, and `Expression` are the existing typed
red-style views. Phase 7 makes the declarative grammar and generated
declarations authoritative without changing runtime representation. Phases 8 and
9 preserve those cursor and wrapper types while replacing hand-written child
searches with generated fixed-slot access for Java and Kotlin respectively. They
do not add a second typed tree, universal `FooFields` wrapper layer, or
persistent decoded representation.

Verbatim is not an error-handling fallback. A formatter failure, missing
accessor, or unimplemented valid-node rule must return an internal error in
debug/test or fail its gate; it must never cause valid syntax to be replayed.

## Product Problem

Formatting runs in CLI, dprint, editor, and future lint/autofix workflows. These
workflows routinely observe incomplete source. The product contract is:

- valid Java and Kotlin receive canonical Jolt layout;
- valid syntax never silently falls back to source replay;
- malformed source is not lost, repaired, retokenized, or rejected merely
  because the parser recovered a represented tree;
- valid ancestors and siblings surrounding a malformed subtree are still
  formatted canonically;
- repeated formatting is stable; and
- production algorithms remain linear or explicitly bounded.

The formatter is not a validator. Structural parser diagnostics have explicit
syntax owners; semantic/version diagnostics do not select verbatim output. The
formatter neither discovers syntax errors nor attempts to repair them.

## Reference Architecture

Biome is the closest reference because its rowan tree is full fidelity, a
generated syntax factory maps parsed children into exhaustive grammar slots,
typed nodes are views over those slots, category-compatible bogus nodes retain
their source, and the formatter tracks printed tokens in debug builds. Jolt
adopts that syntax architecture.

Jolt does not initially copy Biome's physical green-tree representation. This is
a narrow, provisional storage decision rather than a different recovery
architecture. Ruff and Oxfmt are useful formatter and performance references,
but their normal text-in formatters reject parse errors, so they do not define
Jolt's recovery contract.

Primary implementation references, pinned to the reviewed revisions:

- Biome's syntax node and lazy direct-child iterator:
  <https://github.com/biomejs/biome/blob/01bba129afefced1c04aa69592b1b7f337a7b609/crates/biome_rowan/src/syntax/node.rs>.
- Biome's syntax factory and missing/bogus slot construction:
  <https://github.com/biomejs/biome/blob/01bba129afefced1c04aa69592b1b7f337a7b609/crates/biome_rowan/src/syntax_factory.rs>.
- Biome's tracked verbatim/bogus formatting:
  <https://github.com/biomejs/biome/blob/01bba129afefced1c04aa69592b1b7f337a7b609/crates/biome_js_formatter/src/verbatim.rs>.
- Biome's debug-only printed-token accounting:
  <https://github.com/biomejs/biome/blob/01bba129afefced1c04aa69592b1b7f337a7b609/crates/biome_formatter/src/lib.rs#L2268-L2352>.
- Ruff's parse-before-format entrypoint:
  <https://github.com/astral-sh/ruff/blob/04ff791a198844b1a897b765713b30e9cd78f003/crates/ruff_python_formatter/src/lib.rs#L135-L190>.
- Oxfmt's parse-error boundary and arena-allocated formatter wrappers:
  <https://github.com/oxc-project/oxc/blob/8a4f028a5a6853d182f901027bf20bbbd1bc3f46/crates/oxc_formatter/src/lib.rs#L75-L225>
  and
  <https://github.com/oxc-project/oxc/blob/8a4f028a5a6853d182f901027bf20bbbd1bc3f46/crates/oxc_formatter/src/ast_nodes/node.rs#L13-L48>.

## Scope

The product recovery contract covers every tree returned by a production Java or
Kotlin parse entrypoint for source text.

Constructor-valid synthetic trees may test syntax-factory, field, or formatter
invariants. Arbitrary red/green combinations do not broaden the product contract
unless a public formatting API explicitly accepts them.

If parsing produces no represented root, formatting returns an explicit
diagnostic and no output.

## Single Lossless Syntax Representation

### Exact alignment boundary

Biome and Jolt currently store the same logical information differently:

| Concern             | Biome                                                                               | Current Jolt                                                              | Decision                                                                              |
| ------------------- | ----------------------------------------------------------------------------------- | ------------------------------------------------------------------------- | ------------------------------------------------------------------------------------- |
| Node storage        | Immutable `ThinArc` green nodes with red cursors                                    | One parse-owned flat arena with dense node/token IDs and borrowed cursors | Keep the flat arena provisionally                                                     |
| Direct shape        | One physical slot per grammar field, including `Empty`                              | A compact slice containing only represented children                      | Align: replace child entries with grammar slots, including `Empty`                    |
| Shape authority     | Generated grammar and syntax factory                                                | Hand-written parser shape plus hand-written accessors                     | Align: introduce one declarative grammar-shape source and generated factory/accessors |
| Invalid shape       | Factory fills missing slots or changes the node to a category-compatible bogus kind | Error events add diagnostics but do not affect tree shape                 | Align: construction must establish slots and narrow bogus ownership                   |
| Typed access        | Generated typed views over known slots                                              | Repeated child-family and source-range searches                           | Align: generated total fields for valid nodes; delete search-based accessors          |
| Incremental editing | Green nodes support sharing, caching, and cheap replacement                         | Tree is built once for a batch parse                                      | Do not add sharing or reference counting without a product need and benchmark         |

The flat arena is worth preserving only if measurements confirm its intended
benefits: contiguous storage, dense identity, low allocation count, and no
per-node reference counting. Jolt does not currently require incremental tree
editing, structural sharing, or long-lived green-node replacement. Those are the
capabilities forgone by not adopting Biome's green tree. Grammar slots replace
the existing child entries; they are not a second tree or an additional
formatter-facing structure.

The absence of a syntax factory, exhaustive grammar slots, category-compatible
bogus construction, or generated typed views is not an intentional Jolt
difference. Those are the architectural gaps this roadmap removes. If the
prototype shows that preserving the flat arena makes that alignment more
complex, larger, or slower than adopting Biome's representation, Jolt aligns
further instead of protecting the existing tree by default.

Jolt retains three explicit policy differences from the reviewed Biome
implementation:

- Biome can catch a formatter `SyntaxError` and retry a typed node or list as
  verbatim. Jolt forbids formatter-error fallback because that mechanism
  previously admitted valid syntax. Only construction-established bogus kinds
  and required empty slots select verbatim.
- Biome's formatter maintains release comment-attachment state. Jolt initially
  uses the parser's token-owned trivia directly and adds no release comment map;
  a map requires a separately benchmarked architecture amendment.
- Biome's debug printed-token check uses offset identities in an `IndexSet`.
  Jolt uses its existing dense `TokenId` plus derived `(TokenId, side, ordinal)`
  trivia identities in debug/test builds, which compiles out of release builds.

### One parse-owned representation is authoritative

`SyntaxTree`, its dense `NodeId` and `TokenId` identities, slot arrays, and
source-backed trivia remain authoritative. `SyntaxNode`, `SyntaxToken`, and
`SyntaxElement` remain small borrowed cursors over that storage.

There is no persistent `SyntaxPart`, role array, formatter wrapper tree, copied
token buffer, or per-node recovery collection. Direct elements are traversed
with the existing borrowed iterators.

### One grammar-shape source and syntax factory

Java and Kotlin each gain one crate-private declarative Rust macro schema. Small
consumers expand the same schema into:

- node kinds, category unions, and category-compatible bogus kinds;
- the language syntax factory that validates parsed direct children, inserts
  empty slots, and converts an unrepresentable requested node to its bogus kind;
- the existing typed node wrappers, category unions, and their direct-slot
  accessors; and
- the sealed formatter dispatch input for each node kind.

There is no hand-written second description of field order in accessors or
formatter recovery code. There is also no TOML, Python generator, build script,
procedural macro, checked-in generated Rust, or code-generation command. Macro
expansion is compiler work rather than a second source artifact; schema and
consumer source lines count toward the completion budget, and compiled static
metadata counts toward binary-size and performance gates.

Every ordinary node declaration is physically representable: each named field is
required or optional and lowers to exactly one `TreeSlot`. Repeated roles are
explicit variable-length syntax-list node kinds, and their parent field is one
required node slot even when the list is empty. Bounded compound forms that are
one semantic value, such as split shift operators, are explicit fixed-field
syntax nodes rather than multi-element pseudo-fields. Only list nodes and
malformed nodes may own variable slot ranges.

Phase 7 must audit the current compact parser tree without installing the new
runtime representation. A `list` or `constructed` field therefore names its
target node and expands that same node's declared children in place during the
audit. Phases 8 and 9 replace that temporary expansion by constructing the named
node and storing it in the parent's one fixed slot. This is migration behavior
derived from the target declaration, not a second grammar or a persistent range
decoder.

The sealed classification reuses the existing typed wrapper as its valid view:

```rust
enum FormatShape<N> {
    Valid(N),
    Bogus(MalformedOwner),
    InvariantError(SyntaxInvariantError),
}
```

Generated required-field accessors remain non-panicking and read their fixed
slot directly. Optional accessors distinguish `Empty` from an invariant type
mismatch. A stack-local fields value is permitted only when an owning formatter
rule needs it to encode a real multi-field invariant; macro expansion does not
produce universal fields records as a convenience API.

The target physical representation is Biome-style stored slots in Jolt's flat
arena:

```rust
enum TreeSlot {
    Node(NodeId),
    Token(TokenId),
    Empty,
}

struct TreeNode {
    kind: RawSyntaxKind,
    slots: Range<usize>,
    tokens: Range<usize>,
    // Existing parent, offset, length, and index fields remain.
}
```

`TreeSlot` replaces `TreeElement`; no role array, decoder result, or wrapper
tree is stored beside it. A node kind determines the meaning of each slot index.
Ordinary nodes have a fixed slot count. Explicit syntax-list nodes alone have a
variable slot range and use macro-defined borrowed views over their alternating
items and separators; a parent still reaches the entire list through one fixed
node slot.

Phase 7 validates every current compact direct-child sequence against the
generated shape matcher in audit mode but makes no runtime storage change. Phase
8 enables the representation for all Java nodes atomically; Phase 9 does the
same for Kotlin and removes the mixed compact/slot runtime. If either
language-complete pivot misses the performance or total-size gates,
implementation stops. An alternative compact encoding may be proposed only if it
preserves the same logical slots, factory behavior, generated APIs, and
constant-time field access; it is an architecture amendment, not an
implementation detail selected during a later canonical-layout phase.

The selected implementation must be:

- deterministic and idempotent;
- borrowed from existing tree storage;
- stack-local and allocation-free;
- exhaustive over direct represented elements;
- generated in the syntax crate, so formatter code cannot fabricate malformed
  classification; and
- the replacement for hand-written recovery accessors, not an additional API
  beside them.

Phase 7 records macro-schema, consumer, audit, and ordinary implementation-line
measurements and the projected factory/slot cost. Phases 8 and 9 measure the
actual language-complete factory, stored empty slots, and generated views using
parse-only CPU, allocation, memory, and tree-byte metrics.

### Syntax-owned malformed classification

The parser and generated syntax factory have distinct responsibilities:

- parser recovery consumes an unexpected span and wraps it in the narrowest
  category-compatible bogus node permitted by the current grammar position;
- for missing syntax, the parser emits a diagnostic and no source element;
- the factory maps parsed direct elements to the current node's generated slots
  and inserts `Empty` for missing optional or required slots; and
- if parsed elements remain after slot matching, the factory converts only the
  current requested node to its category-compatible bogus kind. It does not
  repartition children or invent a narrower bogus child.

A node is directly malformed when it has a category-compatible bogus kind or a
required `Empty` slot. An optional `Empty` slot is valid. These semantics do not
depend on physical encoding. Direct malformed cases include:

- an explicit parser error/bogus node;
- an unexpected direct child or token;
- a missing required slot;
- an invalid list element/separator sequence; or
- skipped/error trivia owned by that node.

A structurally complete container may contain a malformed child. The container
remains structurally formattable and dispatches that child verbatim. A node does
not become a verbatim boundary merely because a descendant is malformed.

The parser chooses the smallest subtree that completely owns the malformed
pieces. It must not mark the whole file or declaration malformed when a smaller
child boundary preserves complete ownership.

### Category-compatible bogus nodes

Java and Kotlin define bogus categories accepted by their corresponding typed
unions, including expression, statement, declaration, type, pattern, list item,
and any additional grammar category proven necessary. Parser recovery wraps an
unexpected represented span in the narrowest category-compatible bogus node, so
its valid parent can keep a typed slot and remain structurally formattable.

Malformed separated-list content is wrapped as a bogus item when one item owns
the error. The list itself becomes directly malformed only when no narrower item
can own an invalid separator/delimiter sequence. Converting a larger enclosing
node to bogus is a last resort, not routine recovery.

### Diagnostic ownership

Every structural parser diagnostic resolves to a reachable category-bogus node
or required empty slot. A missing token consumes no bytes; its required empty
slot makes the smallest containing typed node the malformed owner. Diagnostics
for semantic restrictions, unsupported language versions, or other
non-structural conditions do not select verbatim.

The ownership invariant is bidirectional for production parses:

- every structural diagnostic has exactly one reachable malformed owner; and
- every reachable category-bogus/malformed owner is backed by a structural
  diagnostic.

A valid-kind node whose generated slots are missing, duplicated, or unexpected
without a syntax-owned malformed marker returns `InvariantError`. It blocks
formatting as a parser/decoder bug and may not select verbatim. Therefore a
parse with no structural diagnostics implies that every reachable classification
result is `Valid(node)`.

Parser/factory tests prove both directions. Formatter dispatch uses only the
sealed classification result; it never scans diagnostics or descendants on the
hot path.

Parser list and delimiter recovery use shared bounded combinators. Orphan or
early closing delimiters are assigned to a concrete malformed owner during
parsing rather than rediscovered by formatter lookahead.

## Formatter Dispatch

### Concrete syntax tree before and after

A Kotlin binary expression has three logical slots:

```text
BinaryExpression = left: Expression, operator: BinaryOperator, right: Expression
```

Today the flat tree stores only represented children. For clean `a + b`, that
happens to resemble the grammar:

```text
BinaryExpression
  NameExpression
    Identifier "a"
  Plus "+"
  NameExpression
    Identifier "b"
```

For missing `a +` at a closing brace or EOF, the current parser creates a
generic zero-width `ErrorNode` as the right child. For an invalid token such as
the unknown Kotlin character in `a + §`, it creates a generic `ErrorNode`
containing that token:

```text
BinaryExpression                 BinaryExpression
  NameExpression                   NameExpression
    Identifier "a"                   Identifier "a"
  Plus "+"                        Plus "+"
  ErrorNode                       ErrorNode
                                    Unknown "§"
```

`ErrorNode` does not implement the expression category. Consequently the typed
API loses the right-hand role, and formatter recovery has to reinterpret the
ordered children.

After the pivot, the generated factory materializes the three grammar slots.
Clean `a + b` becomes:

```text
BinaryExpression slots=[
  Node(NameExpression("a")),
  Token(Plus("+")),
  Node(NameExpression("b")),
]
```

For `a + §`, parser recovery consumes `§` as a `BogusExpression`. The factory
accepts it because `BogusExpression` is a member of `Expression`, so the parent
remains structurally complete:

```text
BinaryExpression slots=[
  Node(NameExpression("a")),
  Token(Plus("+")),
  Node(BogusExpression(Unknown("§"))),
]
```

The binary expression formats structurally; only `BogusExpression("§")` uses
tracked verbatim output. The parser, not the factory, chose that narrow recovery
boundary.

For missing `a +`, parser recovery emits the missing-expression diagnostic but
does not create a generic zero-width child. The factory inserts the required
empty slot:

```text
BinaryExpression slots=[
  Node(NameExpression("a")),
  Token(Plus("+")),
  Empty, // required right expression
]
```

A required `Empty` makes `BinaryExpression` itself directly malformed. Its
tracked verbatim core is exactly `a +`; a following `}` remains owned and
structured by the enclosing block. Whether an empty slot consumes zero bytes is
not confused with whether it has a malformed owner: the containing binary node
is that owner.

If the factory receives children that cannot fit the generated slots, such as a
fourth direct element, it converts the requested `BinaryExpression` itself to
`BogusExpression` while preserving all of its children. It does not scan for a
smaller span or invent a nested bogus node.

### Concrete generated factory and existing typed accessors

The grammar-shape source contains one definition equivalent to:

```text
BinaryExpression {
    left: Expression,
    operator: token(BinaryOperator),
    right: Expression,
}

Expression = ... | BinaryExpression | BogusExpression
```

It generates construction matching equivalent to:

```rust
fn make_binary_expression(children: ParsedChildren) -> RawSyntaxNode {
    let slots = match_slots::<3>(children)
        .required_node::<Expression>(0)
        .required_token::<BinaryOperator>(1)
        .required_node::<Expression>(2);

    if slots.has_unconsumed_children() {
        RawSyntaxNode::bogus(KotlinSyntaxKind::BogusExpression, children)
    } else {
        slots.into_node(KotlinSyntaxKind::BinaryExpression)
    }
}
```

The pseudocode describes generated behavior, not a proposed builder API. Missing
fields become `Empty`; mismatched elements are left unconsumed and make the
current node bogus.

The same definition keeps the existing typed wrapper and generates its
constant-time slot accessors with their compatible public signatures:

```rust
impl<'source> BinaryExpression<'source> {
    pub fn left(&self) -> Option<Expression<'source>> {
        node_at_slot(self.syntax(), 0)
    }

    pub fn operator(&self) -> Option<KotlinSyntaxToken<'source>> {
        token_at_slot(self.syntax(), 1)
    }

    pub fn right(&self) -> Option<Expression<'source>> {
        node_at_slot(self.syntax(), 2)
    }
}
```

The accessors return `None` for an `Empty` slot and report a stored variant or
kind contradicting the schema through the sealed classifier. `format_shape()`
returns `Bogus(self)` for a required `Empty`, `Valid(self)` for a conforming
node, and reserves `InvariantError` for a factory/schema contradiction. A later
owning formatter phase may create a private stack-local input value after
classification when that rule needs to carry several proven required values;
that value is behavior of the rule, not a second generated typed syntax layer.

This replaces current-main accessors that find all expression-family children
and then locate an operator by comparing source ranges. The rejected P16
`BinaryExpressionPart::{Expression, Type, Operator, Token, Error, Node}` stream
is shown only as an approach that must never be reintroduced; it is not present
on the replacement branch.

Formatter classification uses the same slots, once. The owning vertical phase
writes structured formatting against the existing wrapper:

```rust
match expression.format_shape() {
    Valid(binary) => format_binary(binary),
    Bogus(owner) => format_tracked_verbatim(owner),
    InvariantError(error) => block_with_internal_diagnostic(error),
}
```

The results for the three examples are exact:

| Input   | Binary result                                      | Child result               | Verbatim boundary |
| ------- | -------------------------------------------------- | -------------------------- | ----------------- |
| `a + b` | `Valid(binary)`                                    | all valid                  | none              |
| `a + §` | `Valid(binary)`                                    | right is `BogusExpression` | `§` only          |
| `a +`   | `Bogus(binary)` because required slot 2 is `Empty` | none                       | `a +`             |

The intended code movement is deletion, not wrapping: generated grammar/factory
code replaces main's hand-written field searches and formatter fallbacks. P16's
`*Part`, `parts_with_recovered`, and family-specific recovered-token loops are
forbidden patterns that must never enter a replacement branch.

### Valid structured path

Every valid node kind has a structured rule over the existing typed wrapper and
its generated direct-slot accessors. Rules borrow source tokens, child nodes,
and trivia. They may choose canonical whitespace, line breaks, indentation, and
documented semantic normalizations.

Required-slot access on a node classified as valid is total. A required empty
slot classifies its containing node as malformed before its structured rule is
entered. Any other absent required field is an invariant violation.

Clean-corpus gates record verbatim tags and fail if any valid node or token is
covered by one.

### Malformed verbatim path

`format_or_verbatim(node)` performs exactly one slot classification. The
classifier is not called again by the structured rule:

```text
match node.format_shape():
    Bogus(owner) => format_tracked_verbatim(owner)
    Valid(node) => format_structured(node)
    InvariantError(error) => block_with_internal_diagnostic(error)
```

No structured rule catches a formatting error and retries verbatim.

Tracked verbatim output:

- emits the malformed subtree's verbatim core bytes in their original order;
- marks every descendant source token and every conserved trivia identity inside
  the final verbatim core as handled exactly once;
- preserves comment ownership at the subtree boundary;
- reports its first and last lexical atoms to the surrounding formatter;
- applies no syntax repair or canonical token normalization; and
- is linear in the subtree's represented elements and source length.

Phase 6 installs the primitive without pretending that category-bogus
classification already exists. Only the current generic parser error node can
produce an opaque borrowed malformed core. Formatter IR accepts that core,
records its identities, resolves its bounded lexical joins, and requires the
consuming tracked render entrypoint:

```rust
let core = error.syntax().malformed_verbatim_core()?;
let fragment = docs.malformed_verbatim(&core, boundary);
let document = docs.resolve_exceptional(
    fragment,
    previous_source_token,
    next_source_token,
    &mut lexical_safety,
);
let outcome = render_to_tracked(
    &arena,
    document,
    options,
    sink,
    RenderProof::new(root.conservation_tracker()),
)?;
```

An ordinary `render_to` call rejects a visited exceptional fragment. Debug and
test renders record a `MalformedVerbatim` tag even for an empty core. Valid
nodes cannot construct a malformed core. The tracked entrypoint completes the
root proof before it can return a completed outcome; an intentional sink halt
returns an incomplete halted outcome with no proof, and callers cannot forget a
separate `finish()` step. Ordinary structured token and trivia documents carry
source claims without exceptional tags, so mixed valid/bogus output can complete
one root proof. Phase 7 macro-expands the category-bogus and shape declarations
without enabling them at runtime. Phase 8 wires the complete Java dispatch and
tracked root; Phase 9 does the same for Kotlin. A root switches only after that
language's token, comment, ignore, removal, normalization, malformed-boundary,
and recovered-container paths carry exact claims. Optimized builds keep the same
required API and output behavior while the dense tracker, claim arena, and
provenance ledger compile out.

The only permitted byte change inside the verbatim core is an explicitly
approved global text policy, such as line-ending normalization. Any such policy
is reason-tagged and tested separately.

Formatter-ignore remains a separate verbatim feature. It is selected by an
ignore directive, not malformed classification, and retains its existing
normalized indentation and line-ending contract. Each language-complete pivot
adapts that existing range/run path to claim every skipped token and conserved
trivia identity before its root rendering becomes tracked; it does not introduce
another ignore path.

### Exact verbatim range and boundary trivia

Jolt node ranges include boundary trivia, so `text_range()` is not automatically
a safe non-root verbatim range. Tracked verbatim follows this algorithm:

1. A non-root core starts at the first owned token's token-text start and ends
   at the last owned token's token-text end. This includes all inter-token
   trivia.
2. Syntax-owned skipped/error trivia before the first or after the last token
   expands the core only to cover those specific pieces.
3. The verbatim wrapper partitions leading/trailing comments outside the core,
   emits them through the normal comment formatter, and consumes their
   identities exactly once. The structured parent does not format them again.
   Comments inside the core are consumed by verbatim tracking.
4. Boundary whitespace outside the core belongs to the structured parent join
   and may be canonicalized.
5. A malformed root preserves its full outer range because it has no structured
   parent.
6. A line comment at the end of a core forces a line boundary before the next
   structured fragment.

A missing-token boundary with no represented token has an empty verbatim core
and no lexical atoms. It still records malformed dispatch, while the valid
parent formats the surrounding represented siblings normally.

The debug/test gate compares the verbatim core slice byte-for-byte, modulo an
approved global newline policy. “Malformed bytes are conserved” refers to this
core plus separately tracked outside comments, not canonicalized boundary
whitespace.

A lexical atom is a source token, an authorized synthetic token, or a conserved
comment/control item. Boundary metadata is attached only to exceptional emitted
fragments: malformed verbatim, formatter-ignore, replacement, or synthesized
text. Removal has conservation claims but no emitted boundary. Metadata is not
added to every `Doc` or valid structured join.

Joins involving one of those exceptional fragments pass through a bounded
language-aware lexical boundary service. Ordinary valid structured rules remain
responsible for their explicit separators. The exceptional service prevents
identifier, numeric, comment-delimiter, compound-token, and line-comment fusion
without inspecting raw source gaps.

## Canonical Normalization

Valid structured rules may perform narrowly documented semantic-preserving
normalizations. Each normalization is a closed enum case with one permitted
spelling and exact preconditions. Phase 6 exposes temporary opaque replacement,
removal, and synthesis claim carriers but no public constructors, so formatter
rules cannot pair an arbitrary source identity with a normalization case before
the grammar schema exists. Phase 7 macro-expands the closed operation
declarations and owning slot roles. Phases 8 and 9 move the carriers upstream
into `jolt_syntax`: the shared `Language` contract receives a closed
normalization operation and the generated language implementation validates the
owning slot, source kind, and valid-syntax precondition. A `SyntaxToken` then
returns an opaque, tree-branded permit only when that language hook accepts the
operation, and formatter IR consumes the permit without creating a syntax-to-IR
dependency. Phase 10 deletes the temporary IR-owned carriers after both language
pivots. Synthesized tokens require a same-tree source-token anchor. Removal
reasons are separately closed.

Source-token replacement or reordering records the source identities it consumes
and the permitted spelling or bounded permutation. A synthesized token records
its spelling, anchor, reason, and valid-syntax precondition. Synthetic tokens
are never used to repair malformed syntax.

Every output token is either source-backed or covered by one of these claims.

## Conservation And Completion Proofs

### Debug and test tracking

Debug/test formatters maintain a root-level token/comment tracker keyed directly
by dense `TokenId`. A conserved trivia identity is derived without stored
metadata as `(TokenId, leading-or-trailing, trivia ordinal)`. It covers
comments, ignored/skipped pieces, formatter-control trivia, and the line
terminator needed to terminate a line comment; replaceable ordinary whitespace
is not conserved.

Formatting a source token, replacing/removing it through an authorized
normalization, or covering it with a tracked verbatim range consumes the
corresponding identities.

The tracker rejects missing, duplicate, foreign, or unauthorized identities and
provenance-free output tokens. It uses dense storage or a bitset, not a hash map
per node.

Like Biome's printed-token checks, full accounting and normalization reason tags
compile out of optimized release builds unless a performance phase explicitly
approves them. Release correctness comes from the same structured/verbatim
dispatch APIs exercised by the debug, test, mutation, and corpus gates.

Jolt initially retains comments on token trivia. It does not add Biome's
root-level release comment-attachment map. Any future release comment map or
cheap release conservation tracker requires its own allocation/memory benchmark
and explicit approval.

### Reparse and idempotence

Identity is local to one parse. The first render is checked against the original
tree's identities. The output is then reparsed, compared by token/comment
spelling and order modulo authorized normalizations, and formatted with a fresh
identity proof. The first and second formatted bytes must match.

### Valid-path and malformed-path proofs

The clean corpus must prove:

- every node is classified valid;
- no verbatim tag is emitted;
- every token/comment is handled once; and
- structured output reparses and is idempotent.

The diagnostic and mutation corpora must prove:

- every verbatim tag corresponds to a directly malformed syntax node;
- emitted verbatim boundaries form an antichain: when a directly malformed node
  is encountered from a valid structured parent, it emits one outermost boundary
  and formatting does not recurse into its malformed descendants;
- every directly malformed node is covered by exactly one emitted outermost
  boundary, either its own or a necessary directly malformed ancestor's;
- no valid ancestor or sibling is unnecessarily covered;
- malformed subtree bytes and identities are conserved; and
- surrounding structured output reparses to the same token/comment sequence and
  is idempotent.

Snapshot approval cannot bless a classification, conservation, or completion
failure.

## Performance Contract

Lossless recovery must not add a second structural layer to the clean hot path.

Hard constraints:

- no heap allocation per syntax node during field decoding or formatter
  dispatch;
- no formatter-side node wrapper tree;
- no copied source, token, node, or trivia buffers;
- no `Vec`, map, or sorted recovery collection constructed per formatted node;
- generated field access and malformed dispatch allocate nothing and visit each
  direct represented element at most once per structured formatter rule;
- direct-child and verbatim traversal are linear and explicitly bounded;
- debug/test accounting is absent from optimized builds unless separately
  benchmark-approved; and
- added syntax metadata is compact, measured, and justified against tree bytes.

Before migration, a benchmark phase records the existing release baseline for:

- parse-only wall time;
- parse plus format wall time;
- format-only wall time over already parsed trees;
- allocation count and allocated bytes;
- peak resident memory;
- syntax-tree bytes per token/node; and
- formatter document nodes per input token.

The baseline is the first committed report produced by the Phase 3 measurement
harness in the locked release profile on a given machine. Earlier commits have
no allocation or stage-specific harness; applying new instrumentation to them
would modify the measured subject and create a false baseline. The architecture
gate uses the Spring Framework Java and MapLibre Compose Kotlin realistic
corpora. Each result records the source identity, corpus digest, command,
toolchain, hardware-derived machine identifier, sample count, warmup count,
median, dispersion, and raw samples.

Running the benchmark overwrites one tracked report per derived machine
identifier. The code and report are reviewed and committed together; Git is the
history and acceptance mechanism. Comparisons across different machines,
toolchains, profiles, harness generations, or corpus digests are invalid.

Every measured production phase reviews its report diff against both the
previous committed report and the Phase 3 report in Git history; earlier
speedups cannot buy permission for a later regression. No broad migration starts
if a slice introduces a per-node allocation or if either comparison exceeds any
default budget:

- three percent median regression for parse-only, format-only, or parse-plus-
  format wall time;
- one percent increase in allocation count or allocated bytes;
- five percent increase in peak resident memory; or
- five percent increase in syntax-tree bytes per source token or syntax node.

A budget may change only through an explicit architecture amendment with the
measured product justification. It is not absorbed silently into a later phase.

The final clean gate reruns the same benchmark manifests on the same machine and
build profile.

## Implementation Size Contract

The discarded Phase 1–16 implementation added 10,021 and removed 3,076 lines of
non-test-tree Rust, a net increase of 6,945 lines. Of that, language syntax
source grew by 3,908 net lines and Java/Kotlin formatter source grew by 1,407
net lines. The P16 Java and Kotlin accessor files alone total 12,628 lines,
versus 9,256 lines before that stack.

The replacement is not complete if it merely adds macro-expanded slot accessors,
a factory, or bogus kinds beside that machinery. Completion requires:

- final implementation code, including Rust test support but excluding fixture
  data and snapshots, to be net negative relative to `2197128` on `main`, with
  the exact diff reported by crate;
- no external grammar source, generator, build script, or code-generation task;
- main's duplicated field-search and formatter fallback helpers to be removed as
  their macro-defined replacements land;
- P16-only ordered-part enums and `parts_with_recovered` APIs to remain absent,
  enforced by a forbidden-pattern scan;
- macro-schema, consumer, audit, and ordinary implementation lines to be
  reported separately, with every category included in the total; and
- comparison with P16 to be reported only as evidence that the discarded
  machinery was avoided, never as the completion baseline.

The checked-in counting command is:

```sh
git diff --numstat 2197128 -- ':(glob)**/*.rs'
```

For an uncommitted phase, append untracked Rust implementation files to that
report before summing it. Fixture sources, `*.snap`, and benchmark reports are
not implementation code.

Phase 7 records a by-crate projection against `2197128` before runtime
migration. Every later phase reports additions, deletions, and which main-branch
helpers it replaced. Phase 24 fails if implementation is not net negative
against `2197128` or if two independent grammar-shape descriptions remain.
Missing the target requires a user-approved architecture amendment; it cannot be
waived while declaring the checklist `CLEAN`.

## Testing Strategy

1. **Syntax-factory tests** compare parsed direct elements with generated slots,
   fields, and bogus ownership and prove that none is lost or duplicated.
2. **Classification tests** mutate valid inputs and snapshot the smallest direct
   malformed boundaries selected by the parser.
3. **Valid-path tests** fail on every verbatim tag.
4. **Malformed-path tests** prove exact tracked verbatim coverage and structured
   formatting around it.
5. **Lexical-pair tests** use a checked-in finite set of token and comment
   representatives.
6. **Mutation tests** use a checked-in bounded seed manifest and deterministic
   token deletion/replacement operations.
7. **In-repository and imported corpora** apply conservation, classification,
   reparse, and idempotence gates to clean and diagnostic inputs.
8. **CLI and dprint tests** prove public integrations never accept halted or
   partial output.
9. **Performance tests** compare the same release artifacts and manifests before
   and after each vertical slice and at completion.

Phase 5 installs the green, architecture-neutral corpus harness. Imported files
with parser diagnostics or syntax-reconstruction mismatches begin in an exact
deferred-path manifest, with reasons and owning replacement phases, rather than
as knowingly failing tests or snapshotted formatter-loss allowlists. Phases 8
through 19 activate those paths by owned syntax family only when all applicable
gates pass. Phase 23 requires the deferred manifest to be empty and applies the
full corpus contract above. Imported corpus identity remains the importer's
responsibility: it pins upstream commits, writes the generated file manifest,
and CI regenerates the imports.

## Migration Rules

- Build generated slots, direct accessors, and malformed ownership in syntax
  before changing a formatter family.
- Migrate vertically: parser/syntax factory, generated accessors, structured
  formatter, verbatim dispatch, fixtures, and benchmarks for one family in one
  commit.
- Delete that family's main-branch source-range field searches, filtered token
  fallback, and formatter-owned structural inference in the same commit.
- Reject P16-only `parts_with_recovered`, ordered recovery-part enums, and local
  recovered-token loops if they appear on any replacement branch.
- Never introduce a temporary formatter fallback from structured failure to
  verbatim.
- Preserve useful parser-reachability fixes and fixtures from the old branches.
- Restore each historical regression fixture in its owning vertical phase, not
  in the Phase 5 harness commit.
- A valid family is complete only when its tests prove zero verbatim coverage.

## Non-Goals

This architecture does not attempt to:

- make malformed syntax pretty;
- validate or repair malformed programs;
- preserve original whitespace in valid structured syntax;
- support arbitrary trees that bypass syntax-factory invariants;
- reproduce Biome's internal types or allocate an Oxfmt-style wrapper tree; or
- add unbounded layout search.
