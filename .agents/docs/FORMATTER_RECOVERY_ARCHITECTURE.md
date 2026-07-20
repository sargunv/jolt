# Formatter Recovery Architecture

Status: IMPLEMENTED FOR JAVA IN UNCOMMITTED PHASE 8; Kotlin migration remains
Phase 9. This document defines the lossless Java and Kotlin formatter
architecture that replaces both the original formatter-debt roadmap and the
rejected virtual-span Phase 8 prototype.

## Decision

Jolt will use one compact, uniform, parse-owned lossless syntax tree.

Every grammar construct that owns tokens or child nodes is an ordinary physical
syntax node. This includes lists and bounded constructed values such as split
shift operators. Categories, unions, and aliases own nothing and are generated
typed views over ordinary nodes. There is no second formatter tree and no second
class of virtual structural element.

The parser emits compact structural events. A single generated production syntax
factory consumes each represented child once, writes exhaustive slots, and
constructs the flat tree once. Diagnostics are stored separately from the
structural event stream. Parent links and recovery aggregates are assigned while
nodes are constructed; no whole-event index, recursive layout pass, or
formatter-time reconstruction is permitted.

The formatter sees only generated typed accessors over that tree. Valid and
representable recovered syntax is formatted structurally. Only parser- or
factory-owned malformed/bogus nodes and formatter-ignore ranges may select
tracked verbatim output.

The earlier virtual-span Phase 8 implementation is rejected. Its tests,
snapshots, recovery corpus, exact fast-versus-generic comparison, and benchmark
results are useful oracles, but its `TreeSpan`, `SyntaxRole`, `FinishRole`, role
index, dual factory, and recursive layout machinery are not production
architecture. The replacement uniform-tree Phase 8 is implemented for Java in
the current uncommitted worktree and passes its gates; it remains subject to
review before acceptance.

## Product Contract

Formatting runs in CLI, dprint, editor, and future lint/autofix workflows, which
routinely observe incomplete source. The contract is:

- valid Java and Kotlin receive canonical Jolt layout;
- valid syntax never silently falls back to source replay;
- malformed represented source is not lost, repaired, retokenized, or rejected;
- valid ancestors and siblings around malformed syntax remain canonical;
- every represented token and trivia item is emitted exactly once;
- repeated formatting is stable; and
- production algorithms remain linear or explicitly bounded.

The formatter is not a validator. Structural parser diagnostics have explicit
syntax owners. Semantic and version diagnostics do not select verbatim output.
The formatter neither discovers syntax errors nor repairs them.

## Scope

The contract covers every represented tree returned by a production Java or
Kotlin parse entrypoint. Constructor-valid synthetic trees may test factory,
field, and formatter invariants. Arbitrary combinations that bypass the syntax
factory do not broaden the production contract. If parsing produces no root,
formatting returns an explicit diagnostic and no output.

## Reference Architecture

Jolt follows Biome's logical architecture:

- a full-fidelity tree with exhaustive grammar slots;
- generated construction-time shape validation;
- generated typed views over fixed slots;
- category-compatible bogus nodes for malformed ownership; and
- debug/test accounting proving that tokens and comments are handled once.

Jolt deliberately differs only in storage needed for its current product. Biome
uses reference-counted immutable green subtrees to support structural sharing,
incremental replacement, and caching. Jolt batch-parses borrowed source and has
no incremental-editing requirement, so it uses dense flat arenas, compact IDs,
and a parent-navigation overlay. It does not copy token text or pay reference
counting costs. Logical grammar ownership remains Biome-shaped: list and
constructed nodes are physical nodes, not virtual spans.

Ruff and Oxfmt remain useful formatter and performance references, but their
normal text-in formatters reject parse errors and therefore do not define Jolt's
recovery contract.

Primary reviewed references:

- Biome syntax nodes and lazy direct-child iteration:
  <https://github.com/biomejs/biome/blob/01bba129afefced1c04aa69592b1b7f337a7b609/crates/biome_rowan/src/syntax/node.rs>.
- Biome syntax factory and missing/bogus slot construction:
  <https://github.com/biomejs/biome/blob/01bba129afefced1c04aa69592b1b7f337a7b609/crates/biome_rowan/src/syntax_factory.rs>.
- Biome tracked verbatim/bogus formatting:
  <https://github.com/biomejs/biome/blob/01bba129afefced1c04aa69592b1b7f337a7b609/crates/biome_js_formatter/src/verbatim.rs>.
- Biome debug-only printed-token accounting:
  <https://github.com/biomejs/biome/blob/01bba129afefced1c04aa69592b1b7f337a7b609/crates/biome_formatter/src/lib.rs#L2268-L2352>.
- Ruff parse-before-format entrypoint:
  <https://github.com/astral-sh/ruff/blob/04ff791a198844b1a897b765713b30e9cd78f003/crates/ruff_python_formatter/src/lib.rs#L135-L190>.
- Oxfmt parse-error boundary and arena-allocated formatter wrappers:
  <https://github.com/oxc-project/oxc/blob/8a4f028a5a6853d182f901027bf20bbbd1bc3f46/crates/oxc_formatter/src/lib.rs#L75-L225>.

## Before And After

The rejected virtual-span prototype had two structural models:

```rust
enum TreeSlot {
    Node(NodeId),
    Token(TokenId),
    Span(SpanId),
    Empty,
}

struct TreeSpan {
    kind: RawSyntaxKind,
    children: CompactRange,
    tokens: CompactRange,
    owner: Option<NodeId>,
    text_len: TextSize,
    contains_recovery: bool,
    depth: u8,
}
```

Lists and constructed values were `TreeSpan`s, exposed through `SyntaxRole`.
Their event intervals were recovered from `FinishRole` events, indexed before
construction, and laid out after construction. A generated clean factory
returned masks while a generic factory remained as recovery fallback.

The selected model has one structural element:

```rust
#[repr(transparent)]
struct PackedSlot(u32); // NodeId, TokenId, or Empty

struct TreeNode {
    kind: RawSyntaxKind,
    flags: u16,
    children: CompactRange,
    tokens: CompactRange,
    parent: Option<NodeId>,
    index: u32,
}

struct SyntaxTree {
    nodes: Vec<TreeNode>,
    slots: Vec<PackedSlot>,
    tokens: Vec<SyntaxTokenData>,
    trivia: Vec<SyntaxTrivia>,
}
```

Exact packing may change after measurement, but the semantic constraints may
not: slots encode only node, token, or empty; all token/child-owning grammar
constructs use `TreeNode`; and navigation metadata is co-located with its node
rather than a second structural hierarchy.

Node ranges derive in constant time from the first and last owned token. Empty
nodes retain a construction anchor. Tokens retain lexer-owned absolute source
ranges and borrowed source text. Nodes do not store propagated offsets or
duplicated text lengths, and construction performs no recursive layout pass.

The typed view remains small and borrowed:

```rust
pub struct SyntaxNode<'tree> {
    tree: &'tree SyntaxTree,
    id: NodeId,
}

pub struct ArgumentList<'tree> {
    syntax: SyntaxNode<'tree>,
}

pub enum Expression<'tree> {
    Binary(BinaryExpression<'tree>),
    Invocation(InvocationExpression<'tree>),
    Bogus(BogusExpression<'tree>),
    // Exhaustive generated variants.
}
```

There is no persistent `SyntaxPart`, decoded fields record, formatter wrapper
tree, copied token buffer, `SyntaxRole`, or virtual span.

## Parser Events

Structural events express exactly physical syntax construction:

```rust
enum Event {
    Start {
        kind: RawSyntaxKind,
        forward_parent: u32,
    },
    Token,
    Finish,
    Tombstone,
}
```

Diagnostics live in a separate vector and identify their owning node or exact
missing-slot anchor. Separating diagnostics prevents every structural event from
carrying the size of diagnostic strings and payloads.

Lists and constructed values use the same `Start`/`Finish` events as every other
node. There is no `RoleMarker`, `FinishRole`, zero-event checkpoint, role-start
index, interval reconstruction, or special empty-list event.

Parser recovery consumes unexpected represented elements inside the narrowest
physical node already delimited by the grammar. Where the grammar position is a
category, it may wrap them in the category's compatible bogus kind. Where an
exact node or list owns the boundary, the factory retains that kind and marks
that node directly malformed. Missing syntax emits a diagnostic and no synthetic
source token.

## One Generated Production Factory

Java and Kotlin each have one declarative Rust schema. The same declaration
generates:

- syntax kinds, list kinds, category unions, and compatible bogus kinds;
- the complete production syntax factory;
- typed node/list wrappers and direct-slot accessors;
- exhaustive formatter dispatch; and
- test-only audit metadata.

There is no TOML or Python generator, build script, procedural macro, checked-in
generated Rust, runtime schema interpreter, or second hand-written field-order
description.

Conceptually, generated construction looks like:

```rust
fn make_syntax(kind: JavaSyntaxKind, parsed: ParsedChildren<'_>, tree: &mut TreeBuilder) -> NodeId {
    match kind {
        JavaSyntaxKind::BinaryExpression => fixed_slots!(
            tree,
            kind,
            parsed,
            [node(Expression), node(BinaryOperator), node(Expression)]
        ),
        JavaSyntaxKind::ArgumentList => {
            separated_list!(tree, kind, parsed, item(Expression), separator(Comma))
        } // Exhaustive generated arms.
    }
}
```

The factory advances once through direct represented children, using only the
statically bounded matchers for the current declared slot:

1. classify the child at the cursor against the statically declared field;
2. write every grammar slot exactly once;
3. insert `Empty` for absent required or optional fields;
4. validate list item/separator order and trailing-separator policy;
5. retain parser-created category-compatible bogus children and mark an
   explicitly delimited exact node directly malformed when its represented
   children cannot fill that node's declared slots;
6. append the completed physical node once; and
7. assign direct-child parent links and aggregate recovery flags while writing.

A child already classified directly malformed by syntax construction occupies
the current declared field or list-item slot without having to cast to that
field's valid type. Generated accessors expose that slot as `Malformed`, so the
child remains the narrow verbatim owner and the valid enclosing list or node
continues structured formatting.

Unexpected represented input must already be inside an explicit parser node
boundary. The factory may align missing fields and classify that same boundary
as directly malformed, but it may not infer a new boundary, move leftovers to a
different owner, or silently manufacture a different tree. A kind unknown to the
schema is a factory invariant failure; it is visible in debug/test and never
selects valid source replay.

There is exactly one production path. A reference interpreter may exist only in
tests during the atomic pivot and is deleted before Phase 8 acceptance. There is
no production clean fast path plus generic recovery fallback.

## Exhaustive Syntax Ownership

Every ordinary node has one slot for each declared grammar field. Required and
optional fields differ in accessor type, not storage. A missing required field
is an `Empty` slot at an exact construction anchor and remains structurally
formattable recovery.

Repeated fields are explicit physical list nodes. Bounded compound semantic
values are explicit physical constructed nodes. Categories and aliases are typed
unions/views and allocate no node of their own.

Examples:

```text
InvocationExpression
  receiver: Expression | Empty
  type_arguments: TypeArgumentList | Empty
  name: Identifier
  arguments: ArgumentList

ArgumentList
  l_paren: '('
  elements: [Expression, ',']*
  r_paren: ')' | Empty

UnsignedRightShiftOperator
  first: '>'
  second: '>'
  third: '>'
```

These are all ordinary nodes with ordinary parent links and ordinary traversal.
`Expression` is a category view; it owns no slots.

Generated accessors read known slots directly:

```rust
impl<'tree> BinaryExpression<'tree> {
    pub fn left(&self) -> SyntaxResult<Expression<'tree>>;
    pub fn operator(&self) -> SyntaxResult<BinaryOperator<'tree>>;
    pub fn right(&self) -> SyntaxResult<Expression<'tree>>;
}

impl<'tree> ArgumentList<'tree> {
    pub fn elements(&self) -> SeparatedElements<'tree, Expression<'tree>>;
}
```

Formatter rules do not search children, walk token cursors, inspect source gaps,
or parse token streams to rediscover these roles.

## Malformed Ownership And Verbatim

A node is directly malformed only when construction gives it a
category-compatible bogus kind or explicit malformed flag. A structurally
complete ancestor may contain a malformed child and remains structurally
formattable. Missing required fields remain `Empty`; they do not make their
owner verbatim.

Only a directly malformed/bogus physical subtree selects tracked verbatim. The
verbatim renderer claims every contained token and trivia identity so the
surrounding structured formatter cannot lose or duplicate content. A formatter
failure, missing accessor, missing valid rule, or invariant mismatch is never a
verbatim condition.

The parser and factory do not synthesize source tokens to repair invalid input.
Formatter normalization may synthesize separators or delimiters only where a
syntax-owned permit proves semantics and trivia are preserved.

Formatter-ignore remains a separate, syntax-delimited verbatim feature and uses
the same tracking guarantees.

## Recovery Splice Execution Plan

Malformed output is an opaque child, not an opaque remainder of its parent.
Before changing formatter layout, audit the parser shapes for every regression:

1. keep a recognized declaration, directive, list, or statement structurally
   valid when only one of its declared slots is malformed;
2. place unexpected represented source in the narrowest category-compatible
   bogus child or directly malformed exact child already delimited by the
   grammar;
3. keep following valid siblings and enclosing delimiters outside that child;
   and
4. aggregate only adjacent source for which the grammar cannot represent a
   narrower structured owner.

The parser audit determines the splice boundary completely. A directly malformed
child is formatted through the existing typed `ExceptionalFragment`, which owns
its tracked verbatim source claim, exact source range, first and last lexical
atoms, and required line-comment termination. The shared exceptional resolver
distinguishes source-preserved joins from formatter-created joins before
returning an ordinary `Doc`. Its valid parent remains authoritative for spaces,
lines, list separators, indentation, and delimiters.

```text
structured parent rule(
    structured left,
    exceptional malformed child,
    structured right,
)
```

There is deliberately no recovery-specific layout state. Represented whitespace
outside the malformed child's exact source claim is ordinary input trivia and
does not override the valid parent's canonical rule. Where no structured parent
exists, the parser aggregates adjacent unrecoverable source into one malformed
owner, so no formatter list separator is inserted inside that source run.

This division keeps valid ancestors on their ordinary rules. A malformed module
name does not suppress module-directive separation, a malformed type parameter
does not become a top-level program item, and a malformed statement does not
change an unrelated catch parameter after comments are relocated. Formatter
rules do not add recovery-specific separator exceptions.

Implementation order:

1. correct Java and Kotlin parser ownership exposed by the regression fixtures;
2. aggregate adjacent top-level source only until the next recognized program
   item;
3. retain the shared exceptional-fragment lexical-safety path in both language
   formatters;
4. delete recovery boundary-spacing metadata, renderer locks, flat-fit recovery
   state, and boundary normalization workarounds; and
5. prove the reported examples, comment-relocation idempotence, conservation,
   recovery mutation corpora, and full project checks.

The existing token/trivia conservation proof remains the source-identity gate.
This change does not add a general proof over every emitted whitespace
character. Parser shape snapshots prove narrow malformed ownership; recovery
snapshots and idempotence corpora remain the behavioral proof for layout.

## Lexical Safety

All token emission passes through one lexical-boundary service. Exact
exceptional source preserves an edge that was already adjacent to its
represented neighbor; only a join created by formatting is checked for
retokenization, comment merging, or formation of a different operator. The
service inserts the minimum safe separation. Formatter rules may request layout
but may not implement their own raw-text adjacency logic.

The service operates on borrowed token kinds/text, represented token ranges, and
explicit normalization permits. It compares range endpoints but does not inspect
raw source gaps, retokenize fragment text, or infer grammar structure.

## Cost Contract

Tree construction is:

```text
O(events + represented slots + tokens + trivia)
```

with the stronger operational guarantees:

- each event is visited once;
- the direct-child cursor advances once and per-slot matcher work is statically
  bounded by the schema;
- each final slot is written once;
- each parent link is assigned once;
- each recovery aggregate is updated during construction;
- matcher alternatives are statically bounded by the grammar; and
- there are no whole-event, whole-tree, or recursive layout postpasses.

Formatting remains linear in represented tree and emitted document size, except
for separately documented bounded operations such as import/member sorting.
There is no unbounded best-fit or conditional-group search.

Performance gates verify the design rather than guide piecemeal deletion.
Passing by shaving isolated helpers while retaining two structural models is not
acceptance.

## Conservation And Completion Proofs

Debug/test builds assign dense identities to every source token and to each
leading/trailing trivia item. Rendering records exactly one disposition:

- emitted by a structured token helper;
- emitted inside a tracked malformed/bogus subtree;
- emitted inside a formatter-ignore range; or
- removed/replaced by a syntax-owned normalization permit.

Completion requires:

1. every represented token and trivia identity is claimed exactly once;
2. no synthetic token is emitted without an explicit permit;
3. valid syntax records zero malformed-verbatim claims;
4. diagnostics with structural consequences have an exact node or empty-slot
   owner, and every direct malformed owner has a corresponding structural cause;
5. every schema kind has generated construction, access, and dispatch;
6. formatting malformed source is lossless under the defined conservation
   comparison; and
7. formatting is idempotent.

The release build does not retain claim maps or test-only schema metadata.

## Phase 8 Atomic Pivot

Implementation starts from approved Phase 7 production code, not by refining the
virtual-span prototype. The prototype is retained only long enough to extract
its behavioral oracle.

The single uncommitted review point must:

1. preserve the Phase 8 fixture, snapshot, recovery, conservation, idempotence,
   CLI, dprint, and benchmark oracles;
2. replace the syntax tree with compact uniform physical nodes and packed slots;
3. replace parser role events with ordinary node events and separate
   diagnostics;
4. generate the one production factory and typed accessors from the Rust schema;
5. convert all Java list and constructed roles to ordinary physical nodes;
6. convert Java formatting to generated structural access only;
7. retain tracked verbatim only for syntax-owned bogus nodes and ignore ranges;
8. delete the legacy child-search/recovery accessors and all virtual-span,
   role-index, dual-factory, and layout-postpass code; and
9. pass correctness, conservation, idempotence, WASM, allocation, memory,
   timing, and implementation-size gates.

### Uncommitted Java implementation record

The Java pivot now implements the selected model. Its production construction
path has one physical node arena, packed four-byte slots, compact parser events,
one generated Java factory, and generated typed slot views. Parent navigation is
stored beside each flat node rather than in a separately allocated arena; this
is the same navigation overlay described above, not another tree. Direct parser
children use one packed eight-byte construction record and are passed straight
to the factory without copying. Event capacity is reserved once from the
measured realistic-corpus density, and forward-parent construction reuses the
ordinary node stack rather than allocating or recursing.

The 398-file Java audit records 24,656 physical nodes: 24,360 exact valid shapes
and 201 syntax-owned malformed nodes. Clean fixtures have zero missing required
shapes and zero unexpected shapes. Direct malformed ownership is limited to
single invalid productions; malformed children occupy their declared fields/list
slots so valid enclosing containers remain structured. Statement recovery stops
at grammar-owned delimiters, preventing an invalid expression from swallowing
following statements or braces.

The final same-machine Phase 3 versus uniform-tree report is:

| Realistic corpus metric                   |    Phase 3 | Uniform Phase 8 |   Delta |
| ----------------------------------------- | ---------: | --------------: | ------: |
| Spring Java parse                         | 412.952 ms |      393.998 ms |  -4.59% |
| Spring Java format                        | 572.324 ms |      580.238 ms |  +1.38% |
| Spring Java end-to-end                    | 973.780 ms |      977.340 ms |  +0.37% |
| Spring parse allocations                  |    237,499 |         167,176 | -29.61% |
| Spring format allocations                 |  1,535,738 |       1,534,888 |  -0.06% |
| Spring tree reserved bytes/token          |     251.47 |          164.18 | -34.71% |
| MapLibre Kotlin parse                     |   9.199 ms |        7.613 ms | -17.24% |
| MapLibre Kotlin format                    |  13.044 ms |       12.804 ms |  -1.84% |
| MapLibre Kotlin end-to-end                |  21.455 ms |       20.293 ms |  -5.42% |
| MapLibre Kotlin tree reserved bytes/token |     262.37 |          146.81 | -44.04% |

Phase 8 removes 2,175 implementation Rust lines relative to Phase 7, excluding
fixtures, snapshots, reports, and documentation. The roadmap as a whole remains
3,893 implementation lines above the pre-roadmap baseline because Kotlin still
retains its Phase 7 accessors/audit path and the shared proof migration is now
present. The final net-negative gate is therefore not claimed by Java Phase 8;
Phase 9 and the transitional-deletion phase must remove that remaining balance.

No intermediate construction architecture is accepted as a phase or commit. If
the uniform model cannot meet the gates, the result is evidence for revising the
storage representation—not permission to restore virtual roles or layer a third
construction path beside it.

Phase 9 applies the settled uniform architecture atomically to Kotlin. It may
begin only after Java Phase 8 is accepted.

The implementation now exists and validates the architectural model: Kotlin uses
physical generated fields and lists throughout, all clean audited shapes are
exact, malformed output is syntax-owned and tracked, canonical valid output is
retained, realistic corpora are idempotent, and the handwritten accessor layer
is deleted. The phase is not yet accepted because the recorded Kotlin parse and
tree-size deltas exceed the roadmap's incremental budgets. Relative to Phase 8,
parse is +27.1%, end-to-end is +17.0%, parse allocated bytes are +15.9%, and
reserved tree bytes/token are +13.5%. Unchanged Java timing drifted about 6-8%
in the original run; the final report's Java parse drift is 3.6%, which still
does not account for the Kotlin-specific parse and tree growth. The next
decision is therefore a storage/construction optimization or an explicit gate
amendment, not formatter-local recovery code.

The immediate follow-up applies every straightforward storage/construction
optimization found by audit. Exact node reservation, language-owned buffer
capacities, compact token/trivia records, in-place event consumption, bounded
pending scratch, and removal of redundant Kotlin wrapper nodes reduce the
realistic Kotlin tree to 121.18 reserved bytes/token and parse allocations to
32.79 MB. Both improve past Phase 8. Parse timing remains about 9.6 ms versus
Phase 8's 7.613 ms; simultaneous unchanged Java runs vary around 7% slower,
leaving roughly 16-18% Kotlin-specific regression after drift adjustment.

The remaining candidates are deliberate architecture experiments: moving
parent/index navigation from green storage into red views, packing events under
an explicit bound, or generating exact slot-capacity metadata. They require a
separate design and measurement decision. Eliding intentional physical lists or
constructed syntax is not an optimization path.

Phase 23 resolves the remaining Kotlin storage regression without weakening
syntax ownership. A typed role does not require a one-child physical node:
binary operator/right roles, navigation operators and identifier selectors, and
string-template content are generated fields on their owning nodes. User-type
segments remain physical because they group annotations, a name, and type
arguments, but an absent annotation list has no node and the name is a typed
source token. Physical lists, multi-role constructed syntax, malformed/bogus
owners, and diagnostic-bearing recovery nodes remain unchanged.

The same performance audit found two shared construction costs independent of
the grammar model. Profiled token-kind queries crossed an uninlined
already-buffered fast path, so the small cursor chain is forced inline in
release builds. Parser-owned event streams also skip the low-level builder's
external-input scan for its construction-only `Consumed` sentinel; the public
builder retains the check, while the parser marker API cannot produce that
sentinel. These are execution optimizations of the single uniform tree, not a
second construction architecture.

Formatter width measurement takes the same direct path for ASCII source before
falling back to Unicode width decoding, and the compact-concat append hot path
is forced inline. With two warmups and twenty recorded samples, the final
same-machine report passes every incremental and Phase 3 cumulative time,
allocation, peak-memory, and tree-size budget. Kotlin parse and format improve
from Phase 3, end-to-end remains within the three-percent limit, and tree
bytes/token are reduced by more than half.

Phase 10 closes the shared migration layer rather than absorbing every syntax
family's recovery migration. It deletes the test-only raw tree constructor, the
Phase 7 dynamic schema matcher/static audit representation, and the red-tree
generic-error special case. Generated factories, slot indices, typed borrowed
views, and the compact parent overlay remain the sole production model. Existing
render proofs become a hard recovery-free zero-malformed-verbatim gate, and
source scans plus an exact baseline projection make forbidden patterns and
implementation size executable.

Remaining parser `ErrorNode` sites are frozen and then replaced by the
category-compatible bogus owners assigned to the vertical Java and Kotlin
phases. The public kind disappears with the transitional recovery architecture,
after the final owning phase. Exact bidirectional diagnostic ownership follows
the same migration: source-range overlap or a file-wide "has diagnostics" flag
is not ownership, so each vertical phase supplies structural node/missing-slot
identity and the final completion proof checks both directions. Phase 10 retains
the accepted Phase 9 timing exception as an explicit open gate and does not add
a second benchmark harness.

## Rejected Designs

- **Formatter-facing parts tree:** duplicates syntax and invites divergence.
- **Virtual list/constructed spans:** retain two structural models, special
  traversal, role ownership, event indexing, and layout machinery.
- **Eager 48-byte red headers for every grammar node:** measured too expensive;
  compact physical nodes solve storage rather than weakening grammar ownership.
- **Production fast and generic factories:** duplicate construction semantics
  and make recovery a second path.
- **Formatter token-stream parsing:** moves grammar and recovery ownership into
  layout code.
- **Valid-source verbatim fallback:** hides incomplete architecture and repeats
  the failure mode that motivated this project.
- **Copied source/token/node probes:** violate borrowed-storage and cost
  contracts.
- **Unbounded layout search:** violates the formatter's finite cost model.

## Completion Gate

The architecture is complete only when both languages use the uniform tree and
single generated factory, all valid formatting is structural, all malformed
verbatim is syntax-owned and tracked, conservation/idempotence proofs pass, the
realistic performance gates pass, and production Rust is net negative from the
pre-roadmap main baseline excluding fixtures, snapshots, benchmark reports, and
documentation.
