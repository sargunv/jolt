# Formatter Architecture Debt Checklist

Status: OPEN. This is the canonical checklist for bringing the Java and Kotlin
formatters into full compliance with the formatter invariants in `AGENTS.md`. Do
not claim formatter cleanliness while any item here remains open.

## Clean Completion Gate

- [ ] The selected parse-owned lossless syntax tree is the only structural
      representation; formatter traversal adds no stored parts layer or wrapper
      tree.
- [ ] Generated grammar slots and category-compatible malformed/bogus ownership
      cover every direct represented element exactly once without source-range
      inference.
- [ ] Every valid node uses structured formatting, and clean-corpus gates fail
      on any valid syntax covered by verbatim output.
- [ ] Verbatim output is selected only by syntax-owned direct malformed/bogus
      classification, covers the smallest complete malformed subtree, and tracks
      every contained token and conserved comment exactly once.
- [ ] Structural diagnostics and reachable category-bogus/malformed owners map
      bidirectionally; a parse with no structural diagnostics decodes every node
      as valid, and an unmarked invalid shape is an internal error, not
      verbatim.
- [ ] Formatter failure, missing accessors, and unimplemented valid-node rules
      cannot select verbatim output.
- [ ] Every output token is source-backed or has an exact reason-tagged
      normalization/synthetic-token claim; malformed syntax is never repaired.
- [ ] Every parser-diagnostic fixture with a represented tree is formatted and
      checked for classification, conservation, lexical equivalence, and
      idempotence.
- [x] `FormatSinkResult::Halted` is rejected by every `StringSink` test path.
- [ ] Malformed/ignore/replaced/removed/synthesized fragments report exact
      lexical boundaries and use centralized lexical safety at exceptional
      joins; ordinary valid structured documents add no generic boundary layer.
- [ ] No formatter layout decision reads raw source gaps outside syntax-owned
      malformed/bogus verbatim output, formatter-ignore, or represented
      trivia/comment formatting.
- [ ] Every algorithm is linear or has an explicit, documented finite cost model
      and bound.
- [ ] Slot access and formatter dispatch allocate no heap storage per node; the
      release performance/allocation gates pass within the approved budget.
- [ ] No production formatter path can panic for a represented tree.
- [ ] No missing-child branch drops available siblings, delimiters, operators,
      comments, or recovered entries.
- [ ] No syntax repair token is synthesized for malformed represented syntax.
- [ ] Full Java/Kotlin syntax, formatter, CLI, dprint, formatting, whitespace,
      and snapshot-hygiene checks pass with no conservation allowlist entry that
      hides an unresolved formatter bug.

## Active Reproductions

Verified 2026-07-12 with the Java and Kotlin `recovery_snapshots` tests. All
committed reproductions now pass represented-token conservation, `JOLT-TRIVIA`
marker conservation, idempotence, and `StringSink` completion. These checked
reproductions do not by themselves close the broader architecture items below.

### Java

- [x] Preserve trailing annotated array dimensions:
      `fixtures/java/syntax/recovery/array-creation-trailing-dimensions.java`;
      formatter path at
      `crates/jolt_java_fmt/src/rules/expressions/arrays_objects.rs:79-103`.
- [x] Preserve module annotations:
      `fixtures/java/syntax/recovery/module-annotation.java`; formatter path at
      `crates/jolt_java_fmt/src/rules/modules.rs:22-59`.
- [x] Preserve malformed import suffixes:
      `fixtures/java/syntax/recovery/import-trailing-tokens.java`; structured
      import path at `crates/jolt_java_fmt/src/rules/imports.rs:99-123`.
- [x] Preserve a recovered missing-body semicolon:
      `fixtures/java/syntax/recovery/missing-type-body-token.java`; layout at
      `crates/jolt_java_fmt/src/rules/declarations/type_declarations.rs:20-48`.
- [x] Preserve restricted recovered declaration names and invalid modifiers:
      `fixtures/java/syntax/recovery/recovered-declaration-names-and-modifiers.java`;
      generated declaration slots are defined in
      `crates/jolt_java_syntax/src/schema.rs:530-628` and consumed structurally
      by `crates/jolt_java_fmt/src/rules/declarations/callables.rs` and
      `crates/jolt_java_fmt/src/rules/variables.rs`.
- [x] Preserve repeated `requires` modifiers:
      `fixtures/java/syntax/recovery/module-repeated-requires-modifiers.java`;
      physical list slots are defined in
      `crates/jolt_java_syntax/src/schema.rs:1123-1125` and consumed through the
      generated list view in `crates/jolt_java_fmt/src/rules/modules.rs`.

### Kotlin

- [x] Preserve invalid assignment targets and operators:
      `fixtures/kotlin/syntax/recovery/assignment-invalid-targets.kt`;
      recovery-aware operator accessors at
      `crates/jolt_kotlin_syntax/src/nodes/accessors.rs:2370-2401`.
- [x] Preserve comments after opening class/block braces:
      `fixtures/kotlin/syntax/recovery/braced-opening-comments.kt`; brace-trivia
      handling at `crates/jolt_kotlin_fmt/src/helpers/blocks.rs:75-109`.
- [x] Preserve pre-target callable-reference type arguments:
      `fixtures/kotlin/syntax/recovery/callable-reference-missing-target.kt`;
      callable-reference layout at
      `crates/jolt_kotlin_fmt/src/rules/expressions/references.rs:14-19,48-105`.
- [x] Preserve `!!` in represented definitely-non-nullable types:
      `fixtures/kotlin/syntax/recovery/definitely-non-nullable-bang.kt`;
      accessor/layout at
      `crates/jolt_kotlin_syntax/src/nodes/accessors.rs:1190-1198` and
      `crates/jolt_kotlin_fmt/src/rules/types.rs:616-643`.
- [x] Preserve and stabilize nested recovered `when` content:
      `fixtures/kotlin/syntax/recovery/nested-recovered-when-condition.kt`;
      entry formatting at
      `crates/jolt_kotlin_fmt/src/rules/expressions/control_flow.rs:717-769`.
- [x] Preserve property-body items after a recovered header gap:
      `fixtures/kotlin/syntax/recovery/property-body-recovered-gap.kt`;
      recovered fallback at
      `crates/jolt_kotlin_fmt/src/rules/declarations.rs:421-424`.
- [x] Preserve top-level orphan tokens:
      `fixtures/kotlin/syntax/recovery/top-level-orphan-token.kt`; file recovery
      at `crates/jolt_kotlin_fmt/src/rules/program.rs:31-64`.
- [x] Preserve trailing user-type dots:
      `fixtures/kotlin/syntax/recovery/trailing-user-type-dot.kt`; segment
      reconstruction at `crates/jolt_kotlin_fmt/src/rules/types.rs:225-278`.
- [x] Preserve the close brace previously lost by
      `fixtures/kotlin/syntax/recovery/type-constraints.kt`.

## Shared Test Debt

- [x] Recovery gates compare represented input/output token multisets outside
      snapshots, so `INSTA_UPDATE=always` cannot bless token loss.
- [x] Clean and diagnostic corpus fixtures also pass through represented-token,
      marker-conservation, and idempotence gates.
- [x] Recovery gates compare `JOLT-TRIVIA` marker multisets for recovered
      comment conservation.
- [x] Intentional Java token removals are exempted by exact fixture, spelling,
      and bounded count rather than global punctuation classes.
- [x] All Java/Kotlin formatter and dprint tests using `StringSink` reject
      `FormatSinkResult::Halted`.
- [ ] Add dense debug/test token and comment accounting over existing syntax
      identities so identical synthesized tokens cannot mask loss.
- [x] Extend recovery comment conservation from explicit markers to canonical
      inventories of every represented source comment.
- [x] Stop skipping parser-diagnostic fixtures in
      `crates/jolt_java_fmt/tests/corpus.rs:28-33` and
      `crates/jolt_kotlin_fmt/tests/corpus.rs:28-33`; route every represented
      tree through conservation and idempotence checks.
- [x] Replace aggregate diagnostic-import skip counts with an exact
      deferred-path manifest in Phase 5. Represented diagnostic trees now enter
      the hard malformed-input conservation lane; only absent trees or
      reconstruction mismatches may remain deferred. The manifest is empty.
- [x] Keep imported corpus identity in the importer: upstream commits and the
      generated file manifest are pinned, and CI regenerates imports. Formatter
      tests do not duplicate that contract by rehashing imported files.
- [ ] Make imported Java and Kotlin syntax reconstruction loss a hard failure
      instead of a summary count in
      `crates/jolt_java_syntax/tests/parser_fixtures.rs:38-42` and
      `crates/jolt_kotlin_syntax/tests/imported_fixtures.rs:25-38`.
- [ ] Give valid structured output and tracked malformed verbatim output exact
      debug/test accounting so duplicated source tokens cannot mask loss.
- [ ] Audit every Java and Kotlin token-sequence/raw replay call. Retain it only
      inside tracked syntax-owned malformed/bogus verbatim or formatter-ignore;
      valid syntax must use structured rules.
- [x] Return a diagnostic when either formatter receives no syntax tree instead
      of an unexplained empty blocked result:
      `crates/jolt_java_fmt/src/format.rs:33-36` and
      `crates/jolt_kotlin_fmt/src/format.rs:33-36`.

## Diagnostic Corpus Gate Findings

Verified 2026-07-12 with the Java and Kotlin `corpus` tests. These failures were
previously hidden by the formatter corpus diagnostic skip and now pass the hard
conservation and idempotence audit outside snapshots.

### Java

- [x] Preserve all represented pieces in
      `fixtures/java/syntax/parser/diagnoses-invalid-declaration-contexts.java`;
      previous losses included invalid modifiers, declarator suffixes, and
      initializer tokens.
- [x] Preserve and stabilize all represented pieces in
      `fixtures/java/syntax/parser/diagnoses-invalid-expression-forms.java`.
- [x] Preserve duplicate/recovered parameter names in
      `fixtures/java/syntax/parser/diagnoses-invalid-lambda-parameters.java`.
- [x] Preserve missing-body recovery semicolons in
      `fixtures/java/syntax/parser/diagnoses-missing-class-body.java`.
- [x] Preserve restricted recovered type names in
      `fixtures/java/syntax/parser/recovers-restricted-type-identifiers.java`.
- [x] Preserve annotated dimension expressions in
      `fixtures/java/syntax/parser/parses-annotated-dim-expression.java`.
- [x] Preserve module annotations in
      `fixtures/java/syntax/parser/parses-modular-compilation-unit-and-module-directives.java`.
- [x] Preserve trailing method/annotation-element dimensions and their
      annotations in
      `fixtures/java/syntax/parser/parses-trailing-dims-on-method-and-annotation-element-declarators.java`
      and
      `fixtures/java/syntax/parser/trailing-method-and-annotation-element-dims-have-per-dimension-nodes.java`.

### Kotlin

- [x] Preserve invalid assignment targets/operators in
      `fixtures/kotlin/syntax/parser/diagnoses-invalid-assignment-targets.kt`.
- [x] Preserve `?` and stabilize malformed type-argument calls in
      `fixtures/kotlin/syntax/parser/diagnoses-malformed-type-argument-call.kt`.
- [x] Preserve a dangling Elvis operator in
      `fixtures/kotlin/syntax/parser/recovers-missing-expression-after-elvis.kt`.
- [x] Preserve string-condition tokens and stabilize output in
      `fixtures/kotlin/syntax/parser/recovers-missing-when-arrow-and-body.kt`.
- [x] Preserve name-based destructuring defaults/modifiers in
      `fixtures/kotlin/syntax/parser/parses-destructuring-name-based-preview.kt`.

## Superseded Roadmap And Phase 17 Disposition

The original 17-phase roadmap attempted to establish recovery ownership through
node-specific `parts_with_recovered` accessors and formatter loops. Six reviews
of the proposed Phase 17 clean gate found new parser-reachable loss, ordering,
and lexical-fusion cases after all automated gates passed. That repetition is
architecture evidence: completeness remained conventional at each call site.

The Phase 17 commit `a78018c` and branch `codex/formatter-debt-p17-clean-gate`
are rejected from the accepted stack. The branch is retained temporarily as an
investigation record and regression source; none of its implementation or
checked-off clean status is accepted implicitly.

The replacement contract is `FORMATTER_RECOVERY_ARCHITECTURE.md`. The detailed
debt inventories below remain evidence and migration input, but the old phase
boundaries no longer prescribe the implementation order.

## Phase 1–16 Audit

The audit classifications mean:

- **Accept unchanged**: the commit's design is compatible and should be carried
  into the replacement stack without architectural changes.
- **Revise**: retain its goal and selected implementation, but re-extract or
  rewrite it on the replacement foundation before acceptance.
- **Supersede**: retain parser findings, fixtures, or vocabulary, but do not
  accept its recovery implementation as a foundation.
- **Drop**: retain no production or test work. No Phase 1–16 commit met this
  classification.

The classifications are semantic, not instructions to cherry-pick commits out of
their dependency chain. The replacement stack reconstructs the accepted pieces
in dependency order.

| Old phase                                | Commit    | Classification   | Disposition                                                                                                                                                                                                                                      |
| ---------------------------------------- | --------- | ---------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 1. Authoritative gates                   | `86b178b` | Revise           | Keep no-tree diagnostics, an exact deferred-path queue, diagnostic corpora, comment inventories, and renderer completion. Rework token accounting as dense debug/test tracking over existing IDs; do not add a production parts/provenance tree. |
| 2. Java recovery ownership               | `2aff426` | Supersede        | Keep parser reachability fixes, fixtures, and useful grammar-role vocabulary. Replace skip/range-derived recovery accessors with generated slot accessors and syntax-owned malformed/bogus boundaries.                                           |
| 3. Kotlin declaration ownership          | `d1783cf` | Supersede        | Keep duplicate-header parsing, fixtures, and parser findings. Replace ordered recovery streams, range-derived headers/bodies, and skip predicates with generated slot accessors and malformed ownership.                                         |
| 4. Kotlin expression/type ownership      | `d5d2fc8` | Supersede        | Keep its recovery corpus and delimiter/list findings. Move ownership into parser recovery and generated slot accessors with explicit malformed nodes; do not reconstruct expression roles in the formatter.                                      |
| 5. Kotlin types/parameters               | `bb3fa8d` | Supersede        | Keep valid canonical layout, parser fixes, list primitives, and fixtures. Replace per-node recovery state machines with structured valid rules plus tracked verbatim malformed dispatch.                                                         |
| 6. Kotlin declarations                   | `35e2e6e` | Supersede        | Keep parser fixes, canonical declaration documents, and fixtures. Remove prefix/header/tail recovery partitioning and format directly from valid decoded fields or a malformed boundary.                                                         |
| 7. Kotlin expressions/control flow       | `c2f7a66` | Supersede        | Keep parser-reachable fixtures and bounded recovery findings. Remove manual recovery loops, range inference, and parser-inexpressible completion claims.                                                                                         |
| 8. Kotlin formatter-owned parsing/replay | `598e535` | Supersede        | Keep the `fun interface` parser correction, canonical rules, and fixtures. Replace filtered token fallbacks with syntax-owned malformed verbatim; valid nodes must never replay.                                                                 |
| 9. Kotlin source-gap layout              | `ce91a28` | Accept unchanged | Retain represented-trivia classification, raw-gap removal on valid paths, formatter-ignore boundary, comment ownership, and linear matching.                                                                                                     |
| 10. Kotlin repair/panic paths            | `4d52772` | Revise           | Keep guarded valid-syntax normalization, malformed-token preservation, parser fixtures, and panic removal. Route malformed imports verbatim and keep valid import sorting behind malformed barriers.                                             |
| 11. Java programs/declarations           | `95d158c` | Supersede        | Keep duplicate-package parsing, explicit recovery nodes, singleton removal, valid canonical rules, and fixtures. Replace bespoke recovery streams and dispatch.                                                                                  |
| 12. Java expressions/statements          | `e4db005` | Supersede        | Keep parser fixes, fixtures, and grammar-role vocabulary. Replace optional-anchor/range recovery regions and formatter loops with generated slot accessors and malformed boundaries.                                                             |
| 13. Java formatter-owned parsing/replay  | `e47c982` | Supersede        | Keep borrowed operator identity, valid canonical rules, fixtures, and the finding that valid replay is unsafe. Replace local recovery formatting with tracked malformed verbatim.                                                                |
| 14. Java repair/panic paths              | `c272352` | Revise           | Re-extract missing-body parser boundaries, no-repair behavior, panic removal, and fixtures. Missing bodies must create syntax-owned malformed boundaries before formatter dispatch.                                                              |
| 15. Cross-language source reconstruction | `fa6055d` | Supersede        | Keep trivia/lexical-boundary findings and fixtures. Recovered source-gap reconstruction is unnecessary once malformed subtrees are tracked verbatim; valid layout remains structured.                                                            |
| 16. Cost model                           | `046eff8` | Accept unchanged | Carry forward bounded formatter-ignore lookup, finite sorting models, constant-time parent-role lookup, comment deduplication, and source-gap helper removal.                                                                                    |

### Audit Consequences

- Phases 9 and 16 remain compatible designs, although their commits are
  physically stacked on superseded work and must be re-extracted in dependency
  order.
- Parser fixes, canonical valid-node documents, and fixtures from every revised
  or superseded phase remain migration inputs.
- The old recovery accessor families—`parts_with_recovered`, filtered
  `unstructured_tokens`, skip predicates, source-range role inference, and
  formatter-local recovery loops—are not part of the new architecture.
- Raw/verbatim output is no longer globally prohibited. It is accepted only for
  a syntax-owned directly malformed/bogus subtree or formatter-ignore range.
- A clean valid corpus must have zero malformed-verbatim coverage. This is the
  gate that prevents the historical valid-source replay regression.
- No persistent parts layer or formatter wrapper tree is accepted.

## Replacement Roadmap

Each phase is one focused commit and one branch. Later phases stack only when
they consume an earlier phase's API. Every implementation phase receives an
independent architecture and performance review.

All replacement branches descend from `2197128` on `main`, directly or through
the minimum preceding replacement phases they consume. Old Phase 1–16 branches
are never ancestors. Parser fixes, fixtures, and accepted mechanisms are
reimplemented or cherry-picked individually only after review; the bloated stack
is reference material, not a base branch.

Every production-wired vertical phase from 8 through 19 reruns the relevant
Phase 3 parse-only, format-only, end-to-end, allocation, memory, tree-size, and
IR metrics. Review its per-machine report diff against the previously committed
report and the Phase 3 report in Git history; earlier improvements provide no
headroom for a later regression. Performance attribution may not be deferred to
the final gate.

### New Phase 1: Carry Forward Accepted Cost Controls

Re-extract old Phase 16's bounded formatter-ignore lookup, finite sorting
models, constant-time parent-role lookup, and comment deduplication. Run the
production complexity scan so the baseline includes these already-accepted
improvements. Do not copy Phase 16's `source_gap_is_trivia` deletion: on `main`
the helper still has four Kotlin consumers that old Phases 9 and 15 had already
removed. Record those consumers as Phase 2 inputs and forbid new call sites.

### New Phase 2: Carry Forward Represented-Trivia Layout

Re-extract old Phase 9's represented-trivia classification, valid-path raw-gap
removal, formatter-ignore boundary, comment ownership, and linear matching
without carrying its superseded recovery consumers. Replace the four remaining
`source_gap_is_trivia` consumers with represented structure/trivia and delete
the helper and export in this phase.

### New Phase 3: Establish Performance And Allocation Baselines

Replace the old tool-comparison benchmark with parse-only and format-only
execution, allocation count/bytes, peak memory, syntax-tree bytes per
node/token, and formatter-document nodes per input token. Running
`mise run benchmark` overwrites the automatically identified machine's report
for the Spring Framework Java and MapLibre Compose Kotlin realistic corpora,
including commands, toolchain, source identity, hardware-spec hash, corpus
digests, raw samples, and dispersion. The first committed report is the
architecture baseline; later committed report diffs are the accepted history.
Earlier commits cannot honestly supply allocation or stage-specific baselines
without modifying the measured subject. Never compare measurements across
machines. Review the architecture's three-percent time, one-percent allocation,
five-percent peak-memory, and five-percent tree-byte budgets against both the
preceding report and the Phase 3 report in Git history.

### New Phase 4: Retained Regression Inventory

Create a machine-readable inventory of every parser-reachable fixture and parser
fix from old Phases 1–15, mapped to its owning replacement phase. This commit
contains no formatter mechanism. Include `class Unexpected + (val value: Int)`:
the parser correctly ends the class before `+`, while the separately recovered
top-level parenthesized expression currently loses its error-node contents when
formatted. Assign that expression recovery to New Phase 18 rather than treating
it as a class-constructor gap.

Inventory: `formatter-retained-regressions.toml`. Historical fixture paths are
pinned to their old-phase commits; mixed-family fixtures carry a separate scope
for each owning replacement phase. The known active expression regression stays
inline until Phase 18 so Phase 4 does not add a knowingly failing corpus input.

### New Phase 5: Green Completion Harness And Corpus Baseline

Re-extract only architecture-neutral enforcement from old Phase 1: explicit
no-tree diagnostics, halted-render rejection, exact per-path reporting, and
represented-comment inventory helpers. Keep the existing in-repository
diagnostic corpus routed through hard token/comment conservation and idempotence
gates.

This phase restores no historical regression fixture, adds no knowingly failing
test, and changes no formatter recovery or layout rule. Record imported sources
that cannot produce an exactly reconstructing represented tree in an exact
deferred-path manifest with a reason and owning replacement phase. Represented
diagnostic trees belong in a malformed-input lane that preserves diagnostic
identity, tokens, comments, determinism, and idempotence rather than in the
deferred queue. That manifest is a migration queue, not a snapshot of accepted
formatter loss. The Phase 5 commit must pass `mise run test` with no token-loss,
comment-loss, or idempotence allowlist.

### New Phase 6: Tracked Verbatim Primitive

Add API-only structured source-token/trivia claims, tracked verbatim output,
debug/test dense token and derived-trivia accounting, mandatory render-time
completion, exceptional-fragment lexical joins, formatter-ignore separation,
malformed tags, and closed normalization claims. The current generic parser
error node is the only public malformed-core owner; valid nodes cannot construct
one. Replacement, removal, and synthesis claim carriers have no public Phase 6
constructor. Phase 7 replaces them with syntax-owned, tree-branded permits:
generated `Language` normalization hooks validate the exact role, source kind,
and valid-syntax precondition, while formatter IR only consumes the resulting
opaque permit. Test valid, bogus, and mixed structured/bogus constructor trees
without wiring a production formatter family. Keep the existing formatter-ignore
path unchanged rather than adding a parallel ignore API. Prove that optimized
builds compile accounting out and add no per-node tracker or comment-map
allocation. Defer throughput and allocation measurement until the language
pivots in Phases 8 and 9 exercise production paths that use the primitive; do
not add a synthetic or second benchmark harness for this API-only phase.

### New Phase 7: Declarative Rust Syntax Authority

Add one crate-private declarative Rust macro schema to each language syntax
crate. Expand the schema into the language kind inventory, typed node and
category declarations, category-compatible bogus kinds, typed static shape
metadata, and named slot indices. Delete the corresponding hand-written
kind/node/category duplication in the same commit; semantic accessor bodies may
remain temporarily, but macro-defined direct fields become authoritative.
Describe every requested node kind exactly once, including contextual-token
roles and list shapes. Do not add TOML, Python, a build script, a procedural
macro, checked-in generated Rust, or a code-generation task.

Every ordinary valid field must be one required or optional target slot. Model
repetition with an explicit syntax-list role stored in one parent slot, and
model a compound semantic value with an explicit fixed-field constructed role;
do not leave either shape as an unnamed child range. The Rust static audit
rejects any ordinary node that is not representable by this fixed-slot contract.
Audit-time expansion of list and constructed roles is only the migration bridge
from the current compact parser tree; the production pivot stores them as
compact ordinary physical nodes. Categories and aliases remain typed views and
allocate no structural node.

Add a Rust audit-only corpus gate that parses the existing fixture sources and
runs current compact direct-child sequences through the macro-defined matcher
without changing production tree construction. Traverse diagnostic and clean
trees alike; do not parse snapshot text or skip represented recovery nodes.
Resolve every unmatched or ambiguous valid shape in the declarative grammar;
diagnostic/recovery shapes remain reported separately rather than being accepted
as valid grammar variants. Record macro-schema/consumer and ordinary production
LOC plus the by-crate projection against `2197128`. This phase makes no arena,
parser, or formatter runtime change: do not expose `TreeSlot`, insert `Empty`,
switch root rendering, or add a second representation before one language can
pivot atomically.

Phase 7's checked-in size record is 53,632 lines of production Rust. Its two
macro schemas contain 2,199 lines, their language consumers contain 22, and the
shared production metadata and lowering macro contain 275. The Rust-only corpus
audit adds 1,133 lines outside snapshots. The phase adds 3,727 and removes 1,153
lines of implementation Rust, a net increase of 2,574. A one-time migration
comparison confirmed that every pre-Phase-7 raw kind kept its discriminant; new
structural and bogus kinds are appended. No historical compatibility table
remains in production.

The user's net-negative requirement counts all architecture implementation,
including audit and test-support code, while excluding fixture data and
snapshots. Against `2197128`, accepted Phases 1-7 stand at +7,856/-1,788, or
+6,068 net implementation lines:

| Implementation area            | Additions | Deletions | Current delta |  Completion budget |
| ------------------------------ | --------: | --------: | ------------: | -----------------: |
| `jolt_java_syntax`             |     1,348 |       619 |          +729 |     at most -1,900 |
| `jolt_kotlin_syntax`           |     1,164 |       587 |          +577 |     at most -1,250 |
| `jolt_syntax` + `jolt_fmt_ir`  |     2,968 |       103 |        +2,865 |       at most +450 |
| Java + Kotlin formatter crates |       453 |       458 |            -5 |     at most -2,350 |
| `jolt_test_support`            |     1,304 |         9 |        +1,295 |          at most 0 |
| Benchmark tools                |       598 |         0 |          +598 |       at most +598 |
| `jolt_cli`                     |        21 |        12 |            +9 |         at most +9 |
| **Implementation Rust total**  | **7,856** | **1,788** |    **+6,068** | **at most -4,443** |

The completion column is the maximum final delta from `2197128`, not credit
already earned. Phase 8 must replace Java's direct-search accessors instead of
wrapping them; Phase 9 must do the same for Kotlin; Phase 10 must delete the
temporary claim, audit-matcher, and compact-factory carriers once their proofs
move onto the production representation. Later layout phases may improve these
numbers but may not spend the reserved deletion. Recompute the table after each
pivot from the counting command in the implementation-size contract.

### New Phase 8: Java Whole-Language Construction And Formatting Pivot

Implementation status: **implemented and gate-green, uncommitted, awaiting
review**. The uniform physical-node pivot has been rebuilt from approved Phase
7. The earlier virtual-span rewrite described below remains rejected production
architecture; only its fixtures, snapshots, recovery behavior, exact
differential checks, and benchmark reports were retained as an oracle.

For every Java node in one atomic review point:

- use one compact physical node representation for every token/child-owning
  grammar construct, including all lists and constructed values;
- keep categories, unions, and aliases as generated typed views only;
- replace role events with ordinary compact node events and keep diagnostics in
  separate storage;
- generate one exhaustive production factory from the Phase 7 Rust schema;
- store one packed `Node`/`Token`/`Empty` slot for every declared field;
- assign parent links, ranges, and recovery aggregates during construction;
- generate constant-time node/list accessors, category bogus unions, sealed
  `Valid`/`Bogus`/`InvariantError` classification, syntax-derived malformed
  boundaries, lexical safety, and syntax-owned normalization permits; and
- wire Java token/comment/formatter-ignore claims and generic bogus dispatch,
  switching the root only after every output path is accounted.

Delete the old compact/fallback construction, direct child-search recovery
accessors, `TreeSpan`, `SyntaxRole`, role markers/events/indexes, dual
factories, and recursive layout postpass in the same change. Existing valid
family layout remains unchanged. The algorithm visits every event once, advances
once through each direct-child sequence with schema-bounded matching, writes
each final slot and parent link once, and performs no whole-tree pass.

No intermediate construction architecture is an accepted phase or commit. The
next review point is the complete uncommitted Java pivot. Phase 8 passes only
when correctness, conservation, idempotence, WASM, allocation, memory, timing,
and implementation-size gates all pass.

#### Rejected virtual-span prototype record

The remainder of this Phase 8 section records the rejected prototype as
attribution evidence. References to what that prototype “implements” or what a
smaller optimization could do are historical, not the selected roadmap.

Implementation status: virtual-span rewrite complete and correctness-green, but
not accepted because the parse-performance gate remains red. The rejected first
prototype emitted every nonempty list and constructed role as a physical node.
The rewrite instead converts all 40 Java list kinds and 10 constructed kinds to
syntax-owned virtual spans. A parser `RoleMarker` records a zero-event
checkpoint and completion emits one `FinishRole`; construction validates that
interval once and appends one compact `TreeSpan`. There is no paired start
event, physical boundary node, or formatter-time boundary reconstruction.

`TreeSlot::Span(SpanId)` keeps `TreeSlot` at eight bytes. A native `TreeSpan` is
at most 32 bytes versus a 48-byte `TreeNode`; compact IDs and ranges remain
`u32`. A packed eight-byte `ParentLink` identifies a physical parent or logical
span/index, while each span records its physical owner. Generated Java accessors
return borrowed `SyntaxRole` views over those slots. Generic syntax traversal
flattens nested roles to preserve the physical child/token stream; typed access
and debug snapshots retain the declared logical layers. Empty and nonempty roles
use the same representation, and required missing fields keep exact empty-slot
anchors.

Representable recovered roles remain structured. Only construction-established
physical malformed/error nodes—including physical overflow owners created when a
role cannot represent its input—select tracked verbatim. Java formatting
consumes generated typed roles, syntax-owned malformed classification,
normalization permits, and lexical safety; valid formatter fallback,
formatter-side token parsing, whole-list raw fallback, and the old Java
entry/segment recovery structs are gone.

The Java syntax and formatter corpora, recovery snapshots, parser losslessness
and progress tests, schema audit, conservation, idempotence, tracked completion,
CLI, dprint, and Kotlin compatibility tests pass. Two malformed CLI expectations
changed because missing class names no longer create doubled synthesized space.
The only valid-layout correction is `ArrayList::<@A String>new` becoming
`ArrayList<@A String>::new`.

The following table records the rejected physical-boundary prototype, not the
virtual-span result. It remains useful attribution evidence but must not be used
to accept or reject the rewrite:

| Spring Java metric        |     Phase 3 | Physical prototype |   Delta |
| ------------------------- | ----------: | -----------------: | ------: |
| Parse median              |  412.952 ms |         622.029 ms | +50.63% |
| Format median             |  572.324 ms |         695.989 ms | +21.61% |
| End-to-end median         |  973.780 ms |       1,318.134 ms | +35.36% |
| Parse allocation count    |     237,499 |            259,926 |  +9.44% |
| Format allocation count   |   1,535,738 |          1,589,776 |  +3.52% |
| Parse peak RSS            | 144,228,352 |        172,752,896 | +19.78% |
| End-to-end peak RSS       | 153,239,552 |        181,010,432 | +18.12% |
| Tree reserved bytes/token |      251.47 |             225.46 | -10.34% |

The physical prototype's tree storage density passed, but time, allocation, and
peak-RSS goals failed. Its shared changes also exceeded the Kotlin gate: parse,
format, and end-to-end medians move from 9.199, 13.044, and 21.455 ms to 9.859,
14.163, and 24.318 ms (+7.17%, +8.58%, and +13.34%), while Kotlin parse
allocation count rises from 10,629 to 11,114 (+4.56%). The dominant Java work is
approximately 2.26 million physical list and constructed boundaries; the Kotlin
regression shows that shared ID, range, tree, or IR changes also require
separate attribution.

The canonical optimized virtual-span report on the same Phase 3 machine and
corpora is:

| Spring Java metric        |     Phase 3 | Virtual spans |   Delta |
| ------------------------- | ----------: | ------------: | ------: |
| Parse median              |  412.952 ms |       ~567 ms |    ~37% |
| Format median             |  572.324 ms |       ~698 ms |    ~22% |
| End-to-end median         |  973.780 ms |     ~1,267 ms |    ~30% |
| Parse allocation count    |     237,499 |       285,631 | +20.27% |
| Parse allocated bytes     |      5.679G |        6.017G |  +5.96% |
| Parse peak RSS            | 144,228,352 |  about 155.9M |     ~8% |
| Format allocation count   |   1,535,738 |     1,589,776 |  +3.52% |
| Format allocated bytes    |      2.728G |        2.666G |  -2.27% |
| End-to-end peak RSS       | 153,239,552 |   121,733,120 | -20.56% |
| Tree reserved bytes/token |      251.47 |        218.65 | -13.05% |
| Tree reserved bytes/node  |      321.02 |        295.82 |  -7.85% |
| Document nodes/token      |        3.14 |          3.15 |  +0.29% |

The canonical format timing was affected by machine-frequency drift. A fair
alternating Rust 1.96 build of Phase 7 and the virtual implementation measured
format at 661.406 and 729.091 ms (+10.2%); a stabilized post-inlining run was
699.77 ms, effectively equal to the 695.99 ms physical prototype. That
optimization removed `JavaFixedSyntax::slot_at` from the profile without
changing its 1,589,776 allocations. It therefore resolves the
virtual-accessor-specific formatter cost, but it does not make the whole Phase 8
stack acceptable relative to Phase 3.

The first alternating run measured parse at 425.434 ms for Phase 7 and 687.170
ms for virtual spans (+61.5%). The generated Java factory now compiles each
schema declaration into an exhaustive clean layout: fixed nodes and constructed
roles return their slot count and presence mask, while lists validate their
item/separator policy directly. Shared construction appends those slots using
the parser's existing token range and text length. Missing, malformed, or
unexpected input still falls through to the unchanged generic recovery
materializer. An exact 398-fixture differential test compares every node, role,
token, trivia sequence, range, recovery flag, and `Empty` slot against that
generic reference.

This removes generic matcher interpretation from the clean path and reduces the
stable Spring parse median to approximately 567 ms, about 17% faster than the
unoptimized virtual implementation but still roughly 37% slower than Phase 3.
The remaining sampled cost includes virtual-role construction, event indexing,
and recursive parent/offset layout. These measurements reject the two-model
architecture; they do not prescribe a sequence of local deletions. Phase 9 stays
blocked until Phase 8 is rebuilt as the uniform physical-node design.

Kotlin remains on the compact matcher but shares the changed tree/IR runtime.
Its canonical parse, format, and end-to-end medians are approximately 10.5,
18.9, and 29.7 ms versus 9.199, 13.044, and 21.455 ms (about +15%, +45%, and
+39%). Parse allocations rise from 10,629 to 11,114 (+4.56%); format allocations
are unchanged. This shared regression also must be attributed before Phase 9.

The virtual rewrite plus generated factory is +14,169/-12,912 Rust lines versus
Phase 7, net +1,257. The full stack is +21,738/-14,413 versus `2197128`, net
+7,325. This count includes untracked Rust implementation and test-support files
but excludes fixtures, snapshots, benchmark JSON, and documentation. The final
net-negative-from-main gate remains binding.

### New Phase 9: Kotlin Whole-Language Construction And Formatting Pivot

After Java proves the architecture, apply the uniform compact physical-node
construction and formatting pivot atomically to Kotlin, including
soft/contextual token roles. Replace Kotlin's bounded compact audit matcher with
ordinary node events and the same generated production factory used by Java.
Lists and constructed values become ordinary nodes; categories and aliases
remain typed views. Switch the Kotlin root to tracked rendering only after all
token, comment, formatter-ignore, removal, normalization, bogus, and
lexical-boundary paths carry exact claims. Delete Kotlin's compact/fallback
construction and direct child-search recovery accessors in the same commit.

Implementation status: **complete, acceptance blocked by the performance gate**.
Kotlin now uses the same uniform physical node/slot model as Java. Generated
fixed fields, constructed roles, and physical lists replace the 3,571-line
handwritten recovery-accessor layer. The formatter consumes only typed physical
fields/lists, dispatches syntax-owned malformed cores through tracked verbatim
output, centralizes lexical safety, and carries exact claims for comments,
formatter-ignore ranges, separator removal, and precedence parentheses. Valid
canonical snapshots match Phase 8 except that comment-only files now correctly
retain their EOF comments. The realistic ktfmt and MapLibre Compose inputs
format deterministically, reparse, and are idempotent.

The 246-fixture Kotlin schema audit records 15,884 nodes: 15,636 exact valid
shapes and 180 syntax-owned malformed nodes. Clean fixtures have zero missing
required shapes and zero unexpected shapes. The constrained delegated-property
regression is covered explicitly, separator-removal claims verify direct
recovery-free list ownership, and lexical safety covers every compound
punctuation boundary, including `=>` and `;;`. Phase 9 is net -2,328 Rust lines
relative to Phase 8, excluding fixtures, snapshots, reports, and documentation.

The same-machine report is nevertheless red against the accepted Phase 8 report:

| MapLibre Kotlin metric        |   Phase 8 |   Phase 9 |  Delta |
| ----------------------------- | --------: | --------: | -----: |
| Parse                         |  7.613 ms |  9.673 ms | +27.1% |
| Format                        | 12.804 ms | 13.724 ms |  +7.2% |
| End-to-end                    | 20.293 ms | 23.750 ms | +17.0% |
| Parse allocated bytes         |  40.46 MB |  46.90 MB | +15.9% |
| Tree reserved bytes per token |    146.81 |    166.69 | +13.5% |

Java's unchanged parse path measured 3.6% slower in the same run, which
indicates machine drift, but drift-adjusted Kotlin parse remains about 23%
slower. The physical Kotlin tree grows from 156,227 to 242,680 nodes as list and
constructed roles become real nodes; per-node reserve improves from 147.08 to
107.51 bytes, but total tree reserve still exceeds the incremental budget. The
Phase 9 architecture checkpoint is committed, but its roadmap performance
acceptance remains open. Phase 10 may not silently inherit the regression.

#### Post-Phase 9 straightforward optimization record

The direct follow-up exhausts the architecture-preserving construction and
storage cleanup identified by review:

- reserve the exact physical-node count from `events = 2 * nodes + tokens`;
- use measured language-owned event, token, and trivia capacity estimates;
- store source ranges and trivia lengths in the tree's existing `u32` domain,
  reducing `SyntaxTokenData` from 56 to 36 bytes and `SyntaxTrivia` from 16 to 8
  bytes;
- consume forward-parent events in place, reject caller-supplied consumed
  markers, and bound the pending-child scratch reservation; and
- remove Kotlin's redundant import, modifier-sequence, modifier-item, and
  annotation-modifier wrappers while retaining delimiter/recovery-owning list
  containers.

The realistic Kotlin tree falls from 242,680 nodes and 166.69 reserved
bytes/token to 235,153 nodes and 121.18 bytes/token. Parse allocated bytes fall
from 46.90 MB to 32.79 MB. The final timing report remains noisy but red against
Phase 8: Kotlin parse remains about 9.6 ms (+26%) while unchanged Java parse
varies around 420 ms (+7%); drift-adjusted Kotlin parse remains roughly 16-18%
slower. Storage and allocation gates now pass; the strict timing gate does not.

Further changes are architecture experiments rather than straightforward
cleanup: parentless green nodes with ancestry-bearing red views, packed bounded
events, and schema/factory-provided exact slot counts. Do not disguise those as
capacity tuning or delete intentional physical list/constructed nodes to chase
the gate.

### New Phase 10: Shared Uniform-Tree Architecture Closeout

Remove test-only raw/reference tree construction, the Phase 7 dynamic schema
matcher and its static audit metadata, and the red-tree special case that made a
generic parser error kind malformed without factory ownership. Retain only the
single physical node/slot model, one generated factory per language, typed
borrowed views, generated slot indices/categories, and the compact parent
overlay. The two formatter-local `FormatterInsertedToken` enums were already
replaced by syntax-issued normalization claims in Phases 8 and 9; forbid their
return rather than adding another replacement. Pre-size the formatter document
arena only if measurement confirms a remaining growth cost.

Make the architecture claims executable: validate physical parent/slot
relationships and generated node coverage, reject forbidden production patterns,
assert that recovery-free trees render zero malformed-verbatim fragments, retain
hard malformed conservation/idempotence tests, report the exact implementation
LOC projection against `2197128`, and keep the existing realistic benchmark as
the only performance harness. Phase 10 inherits the explicitly accepted Phase 9
timing exception; benchmark readiness must not be reported as a passing timing
gate.

Do not migrate every remaining parser `ErrorNode` in this shared closeout. Those
sites span all Java and Kotlin syntax families and their category-compatible
bogus replacements belong to the focused vertical Phases 11–20. Freeze the
inventory here, remove each site in its owning vertical phase, and delete the
kind with the final transitional recovery architecture in Phase 22. Likewise, do
not fake bidirectional diagnostic ownership with file-level diagnostic flags or
overlapping source ranges: the current diagnostic value has no structural owner
identity. Each vertical phase must introduce exact node or missing-slot
ownership for the structural diagnostics it migrates; Phase 24 enables the
workspace-wide bidirectional proof after that inventory is complete.

Implementation status: **implemented, gate-green, committed, and pushed**. The
public raw tree builder and red-node generic-error fallback are gone. The
1,188-line generic schema interpreter is replaced by a small physical-tree
inventory plus language-local test expansion of the production schema; the
expansion checks slot count, exact token/node/category kinds, required empties,
list alternation, parent/index links, and malformed ownership without retaining
a second runtime shape model. Ten Java and twelve Kotlin diagnostic list shapes
are now correctly reported as missing required physical slots instead of
unexpected reconstructed child sequences; clean/exact/malformed counts are
unchanged.

Debug/test recovery-free render completion now rejects every malformed-verbatim
ledger entry. The architecture test freezes generic `ErrorNode` use by grammar
file, forbids deleted construction/audit/normalization names, and enforces this
exact implementation projection command:

```sh
git diff --numstat 2197128 -- \
  ':(glob)crates/**/*.rs' \
  ':(glob)tools/**/*.py'
```

The Phase 10 review point is +25,936/-24,204 implementation lines, net +1,732
from `2197128`, down 583 lines from accepted Phase 9's net +2,315. The final
roadmap remains responsible for crossing below zero.

Every canonical-layout phase from 11 through 20 restores the historical fixture
scopes assigned to it by `formatter-retained-regressions.toml`. Imported clean
trees must reparse cleanly; imported diagnostic trees must retain their
diagnostic inventory. Both lanes require exact reconstruction, authorized-
normalization-aware token conservation, represented-comment conservation,
determinism, and idempotence. Only absent or non-reconstructing trees may enter
Phase 5's deferred manifest. No phase commits a red test or snapshots a nonempty
failure list.

### New Phase 11: Java Programs, Modules, And Imports

Vertically migrate compilation units, packages, imports, modules/directives, EOF
comments, and sorting barriers. Valid nodes remain structured; malformed spans
use the narrowest category-compatible bogus owner.

Implementation status: **implemented, gate-green, and committed**.
`CompilationUnit` now owns one ordered `CompilationUnitItemList`; packages,
imports, modules, declarations, removable empty declarations, and
`BogusCompilationUnitItem` share that source-order-preserving category. Module
directive lists directly contain the generated `ModuleDirective` category
instead of a redundant wrapper node. Import recovery owns only a
`BogusImportSuffix`, and the scoped parser no longer constructs `ErrorNode`.

The parser now attaches each migrated structural diagnostic to either an exact
node identity or an exact generated missing-slot index. Parser event anchors are
resolved while the physical tree is built; clean parses do not allocate that
temporary resolver. Focused proof tests check reachable owners, empty owned
slots, and both directions for the three scoped bogus kinds. Formatters consume
the ordered typed views, treat malformed/commented items as sorting barriers,
allow commentless syntax-authorized empty declarations to disappear inside an
import run, and retain explicit O(r log r) time/O(r) scratch bounds per bounded
run. The old program/module raw-document recovery streams, broad scoped verbatim
wrappers, and dead comment helper module are deleted.

Four historical fixture scopes are restored, and the six deferred prettier
package/import paths now pass reconstruction, exact authorized-removal-aware
token conservation, represented-comment conservation, reparse, determinism, and
idempotence gates. The imported corpus also exposed adjacent unary operator
fusion (`- -1` to `--1`, `+ +1` to `++1`); that valid-expression lexical-safety
regression is recorded for Phase 14 rather than hidden by Phase 11's scoped
gate.

The Phase 11 review point is +26,461/-24,368 implementation lines, net +2,093
from `2197128`: 361 lines above Phase 10 because this first vertical phase adds
the reusable exact diagnostic-owner identity/proof path. Fixtures, snapshots,
reports, and documentation are excluded; tests and test support are included.
The final roadmap remains responsible for crossing below zero.

### New Phase 12: Java Names, Types, And Declaration Prefixes

Vertically migrate names, types, dimensions, annotations, modifiers, parameters,
declarators, and throws clauses. Delete range-derived and skip-capable recovery
accessors for these families.

Implementation status: **implemented, gate-green, and committed**. The Java
schema now gives malformed types, modifiers, formal parameters, and annotation
arguments category-compatible bogus variants. Scoped parser diagnostics attach
directly to those nodes or to the exact required slot that is absent; the
migrated type and identifier grammar no longer constructs the generic
`ErrorNode`. The architecture inventory freezes that reduction at zero type and
identifier sites and four later-phase declaration-container sites.

Formatters consume the generated typed categories exhaustively. Annotation
arguments and modifiers no longer use broad `format_or_verbatim` containers, and
recovered import names no longer receive an invented sortable sentinel.
Malformed syntax remains syntax-owned and verbatim, while valid names, types,
annotations, parameters, declarators, and throws clauses always take structured
layout. The scoped formatter code is net negative; no token cursor, cloned
source, range-derived accessor, or skip-capable recovery API was added or
retained in these families.

The historical variable-declarator singleton regression is restored as a
Phase-12 fixture. Existing and restored cases pass reconstruction, exact token
and trivia conservation, represented-comment conservation, reparse, determinism,
and idempotence gates. Empty `extends` and `throws` clauses now also report
their previously missing required-type diagnostics with exact syntax owners.

The Phase 12 review point is +26,866/-24,472 implementation lines, net +2,394
from `2197128`: 301 lines above Phase 11. Shared exact-owner support accounts
for 64 net lines: 24 in parser infrastructure and 40 in the reusable proof
helper. Java syntax and its compact language-specific owner cases account for
292 net lines, while Java formatter cleanup removes 55 net lines. Fixtures,
snapshots, reports, and documentation are excluded; tests and test support are
included. The remaining vertical phases must reuse the shared proof helper,
delete the declaration-container and transitional recovery layers, and bring the
final roadmap below zero.

### New Phase 13: Java Declarations

Vertically migrate fields, methods, constructors, initializers, annotation
elements, classes, interfaces, enums, records, members, and bodies. Missing-body
diagnostics must have narrow syntax owners; valid declarations may not replay.

Implementation status: **implemented, gate-green, and uncommitted for review**.
Class and record bodies now contain the generated `ClassBodyMember` category
directly; the redundant `ClassBodyDeclaration` wrapper and its formatter
dispatch layer are deleted. Constructor bodies likewise expose an exhaustive
typed entry category, including a narrow bogus entry for misplaced or duplicate
explicit constructor invocations. Methods expose one required block-or-semicolon
body role, and all five type declarations expose missing-body semicolons without
turning the valid declaration into a replay range.

The declaration grammar no longer constructs generic `ErrorNode` nodes. Missing
record-header delimiters, field and annotation-element semicolons, callable
parameter delimiters and bodies, constructor braces, enum constant names, and
type-body delimiters each have an exact node or missing-slot owner. Unexpected
body fragments use the narrow body-member bogus category for their context.
Generated category accessors expose only schema-declared bogus category nodes as
typed family members; unrelated malformed valid nodes remain explicit malformed
entries, so recovery cannot leak a node into a role that did not declare it.

Declaration formatters now dispatch valid fields, callables, initializers, type
declarations, enum constants, members, and bodies structurally. Broad
`format_or_verbatim` wrappers and declaration-local replay loops are gone.
Syntax-owned bogus members remain the only declaration-level verbatim path. The
focused recovery fixture covers nameless annotated enum constants with arguments
and bodies, empty recovered constants, separators, and following body members.
It passes reconstruction, token and trivia conservation, represented comment
conservation, reparse, determinism, and idempotence gates; the imported deferred
manifest contains no Phase-13-owned paths.

The Phase 13 review point is +27,052/-24,544 implementation lines, net +2,508
from `2197128`: 114 lines above Phase 12. Java formatter cleanup removes 85 net
lines, while the Java syntax ownership, category, and compact exact-owner proof
cases add 199 net lines. Fixtures, snapshots, reports, and documentation are
excluded; tests and test support are included. Later vertical phases must keep
reusing the shared ownership machinery and the final roadmap remains responsible
for crossing below zero.

### New Phase 14: Java Expressions And Patterns

Vertically migrate remaining operators, primary expressions, calls, references,
lambdas, arrays/objects, patterns, and expression-owned lists. Preserve borrowed
operator identity and delete local recovery formatting.

Implementation status: **implemented, gate-green, and committed as `17c30ab`**.
Java's expression and pattern grammar no longer constructs the generic
`ErrorNode`. Missing delimiters, names, operands, lambda roles, class-literal
targets, method-reference receivers, creation types, pattern types, and
declaration-only pattern suffixes now attach diagnostics to their exact required
slot or to a syntax-owned bogus node. Class literals accept only primitive,
`void`, and type-name-shaped targets; method references accept only their
declared receiver grammar; record and type patterns expose their actual ordered
roles instead of reusing a local-variable declaration.

The generated schema now carries category-compatible bogus variants for the
remaining heterogeneous expression roles. Syntax exposes typed classifiers for
invocation names, class-literal targets, lambda modifiers and bodies, assignment
targets, creation types, method-reference receivers, `instanceof` targets, and
pattern types. Java formatter rules consume those roles exhaustively. Valid
expressions and patterns have no replay path and no formatter token parsing;
only syntax-owned bogus variants use malformed verbatim formatting. Borrowed
operator tokens feed the shared lexical-safety decision, including the adjacent
unary `- -1` and `+ +1` cases.

Phase 14 also closes two literal source-identity defects assigned to this slice.
Legacy string-template literals are scanned contextually after the processor
dot, including balanced nested interpolations, without accepting `\{` in an
ordinary string. Java Unicode preprocessing now retains a sparse normalized-to-
raw source map, remaps token, trivia, and diagnostic ranges, and constructs the
tree against the original borrowed source. Files without translated Unicode
escapes retain the previous no-allocation, no-remap path.

The focused Phase-14 recovery fixture covers malformed lambda siblings and
names, annotated dimensions, method-reference and class-literal receivers,
member-chain and lambda-close comments, pattern recovery, and unary lexical
safety. These cases pass exact token and represented-comment conservation,
reparse, determinism, and idempotence. The prettier template-expression and
text-block paths leave the deferred queue and enter the permanent conservation
gate. The two arrow-parenthesis corpora and member-chain corpus contain invalid
standalone lambda statements; they now enter the permanent represented-
diagnostic lane instead of being hidden by a file-level diagnostic skip.

The Phase 14 review point is +28,123/-24,694 implementation lines, net +3,429
from `2197128`: 921 lines above Phase 13. Java syntax and lossless source-range
ownership add 778 net lines, Java formatter changes add 143 net lines, and Rust
test/support code is net zero. A focused reduction pass removed 163 net lines by
matching small generated categories directly, generating only the two useful
family projections, remapping Unicode ranges in place, and collapsing parallel
member-chain state. This remains the largest Java vertical slice because it
removes the final expression/pattern `ErrorNode` sites and fixes the
pre-existing Unicode source-reconstruction boundary at the same time. Fixtures,
snapshots, reports, and documentation are excluded; tests and test support are
included. The final roadmap remains responsible for deleting the transitional
architecture and crossing below zero.

### New Phase 15: Java Statements And Control Flow

Vertically migrate simple statements, loops, switches, resources, catches, and
remaining control-flow families. Delete Java's final bespoke recovery
formatting.

Implementation status: **implemented, gate-green, and uncommitted**. Java's
statement grammar no longer constructs the generic `ErrorNode`. Enhanced-for
variables, resource declarations and accesses, switch entries, switch guards,
and malformed switch-label items now have exact category-compatible bogus
owners. Missing statement and block bodies own their declared empty slots
without consuming `else`, `while`, switch labels, closing braces, or EOF.
Ordinary and resource `try` statements are separate represented forms rather
than a resource statement nested inside an optional ordinary-try shell.

Java statement formatting is now always structured for valid nodes. Phase 15
removes all 29 remaining `format_or_verbatim` call sites, deletes that helper,
removes broad malformed-container gates, and replaces formatter role probes with
exhaustive typed projections. Only explicit bogus children and malformed list
parts use tracked verbatim formatting. The focused `phase-15-statements.java`
fixture passes reconstruction, token/comment conservation, reparse, determinism,
and idempotence, as do the clean Java corpora. Catch-parameter array dimensions
are now emitted structurally instead of being silently omitted.

The prettier `try_catch` and `yield-statement` files intentionally contain
invalid bare-name expression statements and forbidden unqualified `yield(...)`
invocations. They now run through the permanent represented-diagnostic corpus
lane without weakening those diagnostics. The lane requires stable diagnostic
inventory, exact reconstruction, authorized-normalization-aware token
conservation, represented-comment conservation, determinism, and idempotence.
All previously parser-diagnostic-only Java paths enter the same lane, leaving
the deferred manifest empty.

The Phase 15 review point is +28,515/-24,679 implementation lines, net +3,836
from `2197128`: 407 lines above Phase 14. The final Java syntax ownership and
typed role surface add 339 net lines; deleting Java's last valid-node recovery
dispatch while adding the dedicated enhanced-for/resource layout adds 11 net
formatter lines. The permanent imported represented-diagnostic conservation lane
adds 57 net test lines and empties the deferred manifest. Fixtures, snapshots,
reports, and documentation are excluded; tests and test support are included.
The later transitional-architecture deletion phase remains responsible for
crossing below zero.

### Java Closure Tranche

Before starting Kotlin, execute the Java-local portions of Phases 21–24 so that
Java becomes a closed reference implementation without prematurely deleting
shared machinery that Kotlin still uses.

1. **Normalization and totality audit.** Verify every Java token removal,
   reordering, and panic/empty fallback against parser-reachable malformed
   trees. Syntax must issue exact normalization claims; denied claims preserve
   represented tokens.
2. **Java-local transitional cleanup.** Delete duplicated Java recovery wrappers
   and partial canonicalization of valid islands inside malformed containers. A
   malformed modifier, import, module-directive, or requires-modifier container
   preserves its complete source order; a clean container retains canonical
   sorting and comment barriers.
3. **Java closure proof.** Run the realistic benchmark, all Java conservation
   and idempotence lanes, and a corpus-wide bidirectional diagnostic-ownership
   proof. Every structural diagnostic names a reachable node or declared empty
   slot, and every directly malformed Java node has an exact diagnostic owner.

Implementation status: **implemented, review-ready, and uncommitted**. The
normalization audit found and fixed three source-loss paths: labeled empty
statements, denied trailing-resource-semicolon removal, and denied
switch-guard-parenthesis removal. Lambda parenthesis removal now requires the
exact single simple untyped, unmodified parameter shape in syntax rather than a
broader formatter predicate.

The Java factory no longer treats an arbitrary directly malformed child as
compatible with every required field. Required fields and list items now accept
only their declared token, node, or category. The schema explicitly declares the
parser-reachable nested bogus compatibility needed by block items, variable
initializers, type bounds, catch types, and `instanceof` patterns. The modifier
schema now uses the lexer's actual `DefaultKw`, and the typed local-declaration
role covers every declaration the parser accepts, including records, enums, and
annotation interfaces. This keeps missing token slots physical and exact instead
of greedily shifting a later bogus node into them. The full in-repository and
521-file imported Java fixture corpus now proves both directions of structural
diagnostic ownership.

The same-machine realistic benchmark is stable relative to Phase 15: Java parse
is +0.17%, format is -1.42%, and end-to-end is +1.72%; RSS is flat or lower,
format/end-to-end allocation counts are lower, allocated bytes move by less than
0.4%, and tree bytes per token fall 0.70%. Parse allocation calls rise 2.74%
while allocated bytes fall 0.12% because formerly raw-malformed `default`
modifier lists now construct their required fixed slots. Relative to Phase 3,
Java remains +2.20% parse, +3.38% format, and +7.63% end-to-end while allocation
bytes and tree storage are substantially lower; the final Phase 23 cumulative
timing gate remains open.

This tranche removes 125 net Java implementation lines relative to Phase 15.
Java-specific implementation remains 209 lines above `2197128`; the shared Phase
22 deletion and final cross-language proof remain responsible for the roadmap's
net-negative completion gate.

### New Phase 16: Kotlin Programs, Packages, Imports, And Names

Vertically migrate files, duplicate package/import containers, names, EOF
comments, and imports. Retain Phase 2 trivia behavior and use
category-compatible bogus entries as sorting barriers.

Implementation status: **complete and gate-green**. `KotlinFile` now exposes one
source-ordered item list containing package headers, import-list containers, and
body items. Duplicate or misplaced headers remain in that order with exact
diagnostic ownership instead of falling into positional root fields. Qualified
names use an exact separated segment list with category-compatible bogus
segments; import stars and malformed package/import suffixes have dedicated
typed roles.

The formatter consumes those typed parts directly. It no longer replays a whole
file, package, import, alias, name, or qualified name when one child is
malformed. Clean import runs sort in bounded `O(r log r)` time, while comments,
formatter-ignore runs, missing roles, and syntax-owned bogus entries form
conservation-safe barriers. EOF comments retain the Phase 2 behavior. A focused
recovery fixture covers duplicate and late containers, missing and malformed
names, import stars and aliases, bogus suffixes, and an orphan top-level brace;
the syntax corpus proves bidirectional ownership for all of its diagnostics.
Removing whole-file replay also exposed an existing empty trailing type
constraint: the formatter now suppresses layout around that zero-token role so
the newly structured root remains whitespace-clean and idempotent. Phase 17
still owns the full type/list migration.

The phase adds 227 net implementation lines to the formal projection relative to
Phase 15, moving the cumulative baseline delta to +4,063. Most of the local
growth is the first complete Kotlin structural-diagnostic ownership plumbing and
import-local formatter-ignore partitioning; the old positional root and
whole-node fallback paths were deleted. Fixtures and mechanically changed syntax
snapshots are excluded from that projection. The final Phase 22 deletion and
Phase 24 net-negative gate remain binding.

Quality audit: the Rust 1.96 fix gate, all 130 workspace tests, imported Kotlin
and Java corpora, recovery idempotence and conservation, CLI/dprint coverage,
schema audit, bidirectional diagnostic ownership, and `git diff --check` pass.
No benchmark was run; Phase 23 remains the designated cumulative performance
gate.

### New Phase 17: Kotlin Types And Parameters

Vertically migrate names/types, arguments/parameters, constraints, projections,
context parameters, function types, and type-owned lists not covered by Phase 9.
Delete range-derived recovery and list state machines.

Implementation status: **complete and gate-green**. User types now contain exact
dot-separated `UserTypeSegment` nodes with annotations, names, and optional type
arguments. Type arguments directly expose type references, variance projections,
star projections, or category-compatible bogus entries. Function types,
definitely-non-nullable forms, type parameters, constraints, function and
context parameters, value parameters, and parameter names all use generated
typed categories instead of generic role casts.

The parser replaces every Phase 17 `ErrorNode` site with the smallest typed
bogus entry and gives missing types, segments, `where`, colons, assignments, and
list gaps exact diagnostic ownership. Recovery stops at newline declaration
boundaries, so a missing type cannot consume the next declaration. Consecutive
constraint commas now create a `BogusTypeConstraint` barrier and continue
through the remaining constraints and function body. Value and context defaults
remain represented when `=` is missing; `vararg` and destructuring parameter
names have explicit typed roles.

The formatter deletes all 24 in-scope whole-node fallbacks, the manual
type-argument recovery machine, the trailing-empty-constraint heuristic, and
runtime type-form/segment dispatch. Typed physical comma lists preserve every
separator, including consecutive commas, while zero-token recovery roles add no
layout. Central malformed lexical safety no longer inserts a fusion-prevention
space across an already represented source line break. The shared modifier-list
fallback remains intentionally scoped to Phase 18.

The focused recovery fixture covers missing declaration types, modified
declaration boundaries, malformed user segments, type projections, function
types, type/value/context parameters and defaults, missing constraints, and
constraint gaps. Its diagnostics pass the bidirectional ownership proof. The
phase adds 419 net implementation lines, moving the cumulative baseline delta to
+4,482; `types.rs` itself is slightly smaller, while the growth is primarily
exact parser recovery, schema categories, owner tests, and parameter formatting.
Phase 22 and the Phase 24 net-negative gate remain binding.

Quality audit: valid formatter snapshots are unchanged; Rust 1.96 checks,
focused and imported Kotlin parse/format corpora, recovery conservation and
idempotence, schema exactness, parser progress, bidirectional ownership, and
`git diff --check` pass. No benchmark was run; Phase 23 remains the designated
cumulative performance gate.

### New Phase 18: Kotlin Declarations

Vertically migrate properties, functions, constructors, accessors, type aliases,
classes, objects, interfaces, enum entries, delegation, and member bodies.
Delete prefix/header/tail partitioning and declaration recovery loops.

Implementation status: **complete and gate-green**. Callable and property names,
property initializers, function and accessor bodies, constructor delegation,
class delegation, enum entries, class members, property body members, and
missing required declaration roles now have generated typed shapes. The parser
owns the smallest complete malformed declaration role and its exact diagnostic
owner instead of constructing generic `ErrorNode` tails or leaving the formatter
to infer structure from source order.

Property and accessor expression recovery uses a bounded next-accessor stop, so
a missing `=` cannot absorb the following accessor. Missing initializer
operators retain the represented expression inside the property. Constructor and
class delegation preserve missing colons, calls, specifiers, comma gaps, and
partial `by` clauses as typed slots or category-compatible bogus entries.
Malformed enum entries and orphan class members likewise have exact owners, and
missing class braces remain physical empty slots. The existing 256-token
declaration lookahead remains finite; replacing it requires a separately
reviewed forked-cursor design with a linear cost proof.

The formatter consumes the typed declaration roles directly. It deletes all
in-scope whole-node fallback, prefix/header/tail partitioning, declaration
recovery loops, and formatter-side `fun interface` pairing. Class and property
members retain syntax order, formatter-ignore ranges remain idempotent, and
zero-token missing roles do not create trailing whitespace or synthetic layout.
The only declaration-file `format_or_verbatim` call left formats object
expressions and belongs to Phase 19.

The focused recovery fixture covers missing function names and parameter lists,
callable receiver pieces, property bindings and initializers, backing fields,
accessor bodies, type-alias assignments, constructor and class delegation,
orphan members and separators, enum entries, and missing class braces. A clean
fixture also proves terminal and annotated enum entries use the typed path.
Phase 18 adds 368 net implementation lines, moving the cumulative baseline delta
to +4,850. The four formatter source files shrink by 99 net lines; the remaining
growth is typed syntax, parser recovery ownership, and its tests. Fixtures and
mechanically changed snapshots are excluded. Phase 22 and the Phase 24
net-negative gate remain binding.

Quality audit: Rust checks, focused and imported Kotlin parse/format corpora,
recovery conservation and idempotence, trivia conservation, schema exactness,
parser progress, bidirectional diagnostic ownership, bounded-algorithm review,
and `git diff --check` pass. Valid formatting changes are limited to typed class
body layout, compact enum-entry runs with an attached terminating semicolon, and
the structured property-accessor layout. No benchmark was run; Phase 23 remains
the designated cumulative performance gate.

### New Phase 19: Kotlin Expressions And Calls

Vertically migrate operators, strings, lambdas, collections, callable
references, object literals, and remaining call/navigation families. Delete
filtered token fallback and expression-local recovery state.

Implementation status: **complete and gate-green**. Assignment, binary, unary,
postfix, parenthesized, annotated, call, index, collection, navigation, callable
reference, string-template, lambda, anonymous-function, and object-expression
roles now expose exact typed fields. Expression holes, invalid assignment
targets, missing selectors and reference targets, delimiter gaps, template
closes, and list/lambda comma gaps use the smallest category-compatible bogus
node with exact diagnostic ownership. Anonymous functions require represented
parameters and bodies; object expressions require a represented class body; and
`this`/`super` labels use a shared required `LabelReference`.

Operator recovery uses a bounded declaration discriminator: a missing operand
stops before a named declaration on the next line without rejecting valid
multiline, anonymous-function, or receiver-function operands. Empty-first commas
in calls, indexes, collections, and lambda parameters receive bogus owners,
while a trailing comma after a real item remains valid. String-template parts,
long-template boundaries, object delegation, and anonymous declaration bodies
are syntax-owned and ordered. Existing expression lookaheads remain explicitly
capped, and all list and member-chain walks remain linear.

Seven heterogeneous roles use the schema's named projections plus syntax-owned
exhaustive classifiers: binary operators and right operands, navigation
operators, callable-reference receivers, string-template content, lambda
bindings, and value-argument prefixes. Formatter rules match those enums
directly; structural casts remain only for lambda-body `BlockItem` handling,
which belongs to Phase 20.

The formatter removes all 17 in-scope whole-node fallbacks, duplicate lambda
destructuring, anonymous-body state inference, object-delegation duplication,
generic square-list collectors, and argument recovery flags. Only the
`LoopExpression` fallback remains for Phase 20. Missing zero-token operands,
bodies, and labels add no synthetic or trailing layout. Malformed leading-comma
examples retain a harmless double internal space, but have exact owners and pass
conservation and idempotence without per-list repair exceptions.

Phase 19 adds 208 net implementation lines, moving the cumulative baseline delta
to +5,058. Scoped formatter/helper source shrinks by 274 net lines; the
remaining growth is exact parser recovery, typed schema projections/classifiers,
diagnostic ownership, and their tests. Fixtures and mechanically changed
snapshots are excluded. Phase 22 and the Phase 24 net-negative gate remain
binding.

Quality audit: Rust checks, focused and imported Kotlin parse/format corpora,
recovery and trivia conservation, idempotence, schema exactness, parser
progress, bidirectional diagnostic ownership, exhaustive projection review,
bounded-algorithm review, valid formatter snapshot review, and
`git diff --check` pass. No benchmark was run; Phase 23 remains the designated
cumulative performance gate.

### New Phase 20: Kotlin Statements And Control Flow

Vertically migrate branches, loops, `when`, `try`, and remaining statements and
blocks. Delete Kotlin's final bespoke recovery formatting.

Implementation status: **complete and gate-green**. Statements, expression
statements, local declarations, blocks, lambda bodies, `if`, `when`, `try`,
`for`, `while`, `do`, labels, jumps, and throws now expose typed physical roles.
The final parser `ErrorNode` construction is replaced by represented
`EmptyStatement`; statement content, block/lambda list elements, branches,
control bodies, loop variables, when values/bodies, and try clauses use named
projections plus syntax-owned exhaustive classifiers.

Required conditions, bodies, delimiters, arrows, loop headers, catch parameters,
and block closes have exact owners. `ForVariable` represents its actual
modifier/name-or-destructuring/type shape rather than accepting an arbitrary
expression. Catch uses an exact single `CatchParameter`. Jump forms distinguish
return's optional value from invalid break/continue tails and share typed
`LabelReference`; throw recovery stops before a following declaration.

`WhenExpression` retains missing-brace entries in its ordered entry list.
Bounded newline lookahead separates a missing-arrow/body entry from the next
physical arrow-bearing entry while preserving multiline conditions whose
operators continue across lines. Condition comma gaps use `BogusWhenCondition`,
and legal trailing commas remain valid. `TryExpression` contains an ordered
`TryClauseList`; late catches, duplicate finally clauses, and other invalid
clause orderings remain represented as owned bogus clauses instead of escaping
into generic expression recovery. Missing `while` in a do-while retains the
following parenthesized condition.

The formatter removes all 23 remaining statement/control-flow whole-node
fallbacks, deletes `LoopExpression`, duplicate destructuring logic, delimiter
recovery helpers, do/while presence state, and the block-local generic Recovery
variant. Formatter-ignore range bookkeeping remains, but block, lambda, when,
try, and loop contents are formatted from typed ordered parts. Missing closing
braces no longer grow blank lines across passes; zero-token roles add no
trailing whitespace; valid empty bodies format compactly as `if (condition);`
and `while (condition);`. Long or heavily commented when entries keep the
condition and arrow together and break the body after `->`, preserving a
parser-valid layout.

Phase 20 adds 761 net implementation lines, moving the cumulative baseline delta
to +5,819. The formatter's five Phase 20 source files grow by 43 net lines; most
growth is exact ordered parser recovery, schema roles/classifiers, ownership
proofs, and focused progress coverage. Fixtures and mechanically changed
snapshots are excluded. Phase 22 and the Phase 24 net-negative gate remain
binding.

Quality audit: Rust checks, focused and imported Kotlin parse/format corpora,
recovery and trivia conservation, idempotence, schema exactness, parser
progress, bidirectional diagnostic ownership, exhaustive projection review,
ordered when/try recovery, bounded-algorithm review, valid formatter snapshot
review, and `git diff --check` pass. No benchmark was run; Phase 23 remains the
designated cumulative performance gate.

### New Phase 21: Normalization And Totality Audit

Audit every spelling/reordering/synthetic normalization and every panic or empty
fallback. Normalizations require valid syntax and exact debug/test claims;
malformed syntax is preserved verbatim rather than repaired. Resolve every
normalization finding by distinguishing documented valid normalization from
source loss without adding per-path exceptions.

Status: complete.

Implementation: the shared syntax layer now issues closed, language-authorized
claims for canonical reordering as well as removal, replacement, and synthesis.
Java import, modifier, module-directive, and `requires` ordering and Kotlin
import ordering consume those recovery-free claims; denied claims retain source
order. Java control-body brace synthesis is authorized by the complete enclosing
control node, so a valid child can no longer normalize a malformed parent.
Denied separator removal and replacement paths preserve their represented
tokens, including malformed enum separators, and paired guard parentheses are
removed only atomically. Empty array initializers no longer gain a synthesized
trailing comma.

Totality: production formatter rules contain no `panic!`, `expect`, `unwrap`, or
`unreachable!` paths. Corpus-wide syntax proofs require every directly malformed
node to expose a malformed verbatim core and every physical empty slot to expose
a zero-width missing core. Focused recovery fixtures cover denied Java and
Kotlin normalization, malformed reordering, and malformed control structures.

Size: Phase 21 adds 968 and removes 217 implementation lines, net +751. The
cumulative projection against `2197128` is +31,620/-25,050, net +6,570.
Fixtures, snapshots, reports, and documentation are excluded. Phase 22 deletion
and the Phase 24 net-negative completion gate remain binding.

Quality audit: focused normalization authorization, recovery snapshots, imported
Java and Kotlin idempotence, trivia conservation, syntax corpus and schema
audits, malformed/missing totality proofs, production panic-path scans, Rust
checks, `git diff --check`, `mise run fix`, and `mise run test` pass. No
benchmark was run; Phase 23 remains the designated cumulative performance gate.

### New Phase 22: Delete Transitional Recovery Architecture

Remove obsolete recovery accessors, filtered token fallbacks, source-range
ownership, recovery sorters, and local recovery join helpers. Prove that every
valid node kind has a structured rule, every malformed category has tracked
verbatim dispatch, and clean corpora emit zero verbatim tags.

Status: complete.

Implementation: the final generic `ErrorNode` kind and parser/language special
cases are deleted; parser recovery is owned only by category-compatible bogus
nodes and directly malformed valid nodes. The unused shared token-range filter
module is gone. Kotlin's remaining whole-node `format_or_verbatim` wrappers,
empty-token/list filters, source-gap boundary inference, and duplicated
delimiter-recovery joins are deleted. Java modifier recovery is an ordinary
malformed sort barrier rather than a whole-list fallback, and modifier accessors
no longer silently filter unexpected separators.

The retained `FormatField`, `FormatListPart`, and `FormatDelimiter` types are
small behavioral results over generated borrowed syntax roles: they centralize
value-versus-recovery document handling and do not describe or reconstruct tree
shape. A trial direct-match rewrite was rejected during the quality audit
because it duplicated hundreds of call-site matches without deleting a syntax
model.

Proof: syntax corpus audits require every directly malformed node to own a
tracked verbatim core and every physical empty slot to own a zero-width core.
Formatter dispatch is exhaustive over generated node/category/list enums;
recovery corpora complete token/comment conservation and idempotence, while the
debug render gate rejects any malformed-verbatim fragment for a recovery-free
tree. Zero-width missing roles validate their core and emit no layout fragment.
The renderer now defers and coalesces layout spaces, discarding them before line
breaks and indentation, so centralized exceptional joins cannot create trailing
or duplicate whitespace.

Size: Phase 22 adds 611 and removes 742 implementation lines, net -131. The
cumulative projection against `2197128` is +31,536/-25,097, net +6,439.
Fixtures, snapshots, reports, and documentation are excluded. Phase 24's
net-negative completion gate remains binding.

Quality audit: Java and Kotlin syntax/formatter corpora, imported idempotence,
recovery snapshots, trivia conservation, shared renderer/conservation tests,
schema audits, forbidden transitional-pattern scans, Rust checks,
`git diff --check`, `mise run fix`, and `mise run test` pass. No benchmark was
run; Phase 23 is the designated cumulative performance gate.

### New Phase 23: Final Performance Gate

Status: complete.

Repeat the Phase 3 release benchmarks on the same machine and manifests. The
first measurement found a deterministic Kotlin-only regression: unchanged tokens
and trivia produced 34,537 additional physical nodes, 48,924 additional slots,
6.56% more reserved tree bytes/token, 9.01% more parse allocation bytes, and
6.81% more parse time than the preceding report.

The audit rejected capacity tuning as the primary answer because most of the
growth was stored final syntax, not transient allocation. Single-slot roles now
remain exhaustive typed schema fields without allocating a one-child node:
binary operator/right roles, navigation operators and identifier selectors, and
string-template content. User-type segments retain their meaningful grouping
node while empty annotation lists are absent and the segment name is a typed
source token rather than a one-token `Name` child. Multi-field constructed
syntax, physical lists, malformed/bogus owners, and diagnostic-owning wrappers
remain physical.

A release profile showed the shared token-buffer `ensure` fast path as the
hottest parse leaf because cursor kind queries crossed an uninlined call even
for already-buffered tokens. The measured cursor chain is forced inline, with a
documented Rust 1.96 lint exception. Parser-owned event streams also bypass the
public builder's redundant scan for a construction-only `Consumed` sentinel that
the marker API cannot emit; low-level external builder inputs retain that
validation and its regression test. Literal-width measurement now uses bounded
ASCII paths before Unicode width decoding, and the profiled compact-concat
append fast path is forced inline. The single harness retains two warmups and
uses twenty recorded samples so CPU ramp-up does not decide a three-percent
release gate.

Final same-machine results pass every incremental and cumulative budget. Against
the preceding report, every Java and Kotlin stage improves by more than four
percent. Against Phase 3, both languages' parse and format stages improve, Java
end-to-end improves, and Kotlin end-to-end remains below the three-percent
limit. Kotlin parse allocation count/bytes and reserved tree bytes/token improve
both incrementally and cumulatively, with cumulative reductions above forty
percent. All peak-memory results remain within five percent. The committed
same-machine report is the exact numeric record.

Quality audit: the full 148-test Java/Kotlin suite, imported idempotence, schema
and diagnostic ownership audits, conservation, CLI/dprint tests, wasm checks,
Rust 1.96 formatting and lints, the external-event validation regression,
independent architecture review, report source-state verification, and
`git diff --check` pass.

The first attempted clean-completion audit exposed three remaining places where
the architecture still detects drift after the fact instead of making the
invalid state unrepresentable: structural diagnostics can be attached separately
from recovery construction, normalization authorization can inspect less than
the complete construct that gives the rewrite meaning, and rendered source text
can still be paired with conservation claims through separate bookkeeping.
Phases 24-29 are the final rearchitecture pivot for those classes of failure.
They are not fixture-by-fixture repair phases, and no transitional API
introduced by them survives the final deletion phase.

### New Phase 24: Atomic Recovery Cause Primitive

Add one parser/tree-construction operation that records a structural recovery
cause while completing its exact category-compatible bogus node,
directly-malformed delimited node, or required empty-slot anchor. Diagnostic
payloads remain in the compact separate diagnostic buffer, but their structural
owner identity is produced by the same parser event or completion operation;
there is no later range matching, file-level diagnostic flag, owner repair, or
independent ownership setter. Prove that the representation cannot create a
structural diagnostic without a reachable owner or a directly malformed node
without at least one structural cause. Keep non-structural lexer and advisory
diagnostics explicitly ownerless.

Quality gate: shared parser/tree tests cover bogus-node, directly-malformed, and
missing-slot construction, multiple diagnostics sharing the smallest complete
malformed owner, and diagnostics without structural consequences. The shared
primitive remains linear, adds no per-node allocation, and replaces rather than
wrapping the current ownership path.

Implementation status: **implemented and independently audited**. A
`PendingDiagnostic` captures its source range before recovery consumes input.
`complete_recovery` and `missing_required_slot` require one or more pending
causes and atomically append their exact structural ownership;
`report_non_structural` is the explicit ownerless path. Parser diagnostics use
one private payload-plus-owner record vector, while the public parse result
retains its compatibility view for the Java and Kotlin migrations. Empty cause
iterators fail before mutating parser events, multiple causes use arrays or
iterators without a temporary allocation, and neither events nor syntax nodes
gain storage. An atomic node cause also marks an otherwise shape-valid semantic
recovery owner directly malformed during tree construction and propagates its
existing recovery flag to ancestors; legacy unmigrated owners do not. The legacy
diagnostic marker and mutating ownership APIs remain only as Phase 25-26
migration debt.

Pending capture reserves the diagnostic's final position immediately, with a
private pending ownership state that must be consumed exactly once by structural
recovery or explicit non-structural reporting. This preserves nested diagnostic
order as well as the pre-recovery source range; parser completion rejects any
unconsumed pending diagnostic. Bounded lookahead capture can anchor a pending
cause on a following buffered token without consuming input or changing parser
events, so recovery can preserve both its exact tree boundary and existing
diagnostic range.

Size: the exact implementation projection is +32,588/-25,104 against `2197128`,
net +7,484. The executable intermediate ceiling is updated to that value; Phase
29's all-implementation net-negative gate remains binding.

### New Phase 25: Java Atomic Recovery Migration

Move every Java structural diagnostic and recovery completion onto the Phase 24
primitive. Delete Java post-hoc owner attachment and any parser branch that can
complete malformed structure separately from its cause. Preserve diagnostic
codes, messages, counts, represented tree shape, and the narrowest complete
malformed owner unless the old owner was demonstrably inconsistent with the
schema.

Quality gate: the full Java syntax corpus proves both directions at the
invalid-node level: every structural diagnostic names a reachable malformed node
or declared required-empty slot, and every directly malformed or
required-incomplete node has a corresponding cause on that node. One cause may
explain multiple cascading empty slots, and multiple diagnostics may share one
smallest complete malformed owner. Java formatter, imported-corpus,
conservation, idempotence, CLI, and dprint tests pass before commit.

Implementation status: **implemented and quality-audited**. Every Java parser
recovery cause now enters through `PendingDiagnostic` and is consumed by
`complete_recovery`, `missing_required_slot`, or the explicit non-structural
path. Java parser source contains no diagnostic ownership marker, mutating
ownership call, unresolved-owner construction, legacy owned helper, plain
structural `expect`, or ownerless grammar helper; an architecture gate rejects
their reintroduction. The decimal integer boundary advisory is the sole
intentionally ownerless Java parser diagnostic.

The corpus-wide proof checks three directions against generated physical slots:
every structural parser diagnostic has an owner, every owner names the exact
directly malformed node or required empty slot, and every such invalid shape has
a cause on that same node. It has no descendant-owner fallback. Atomic recovery
of an otherwise schema-valid kind covers conditional grammar rules such as an
`else` without a branch, restricted type identifiers, mixed lambda parameter
forms, misplaced varargs, non-final type arguments, and invalid array-creation
combinations. Primitive-type annotations are correctly optional in the Java
schema and formatter, eliminating the three formerly clean required-slot
mismatches.

Diagnostic codes, messages, ranges, and ordering are unchanged except for two
consecutive-comma switch labels that previously created required empty item
slots without any cause. Those now receive one primary
`expected switch label
item` diagnostic at the missing item; this closes an
undiagnosed recovery hole rather than adding a cascade diagnostic. Two
inconsistent recovery boundaries are corrected: a class-literal dot remains in
the complete class-literal owner instead of being swallowed by its bogus target,
and a malformed lambda comma is represented by its malformed parameter instead
of creating unowned empty slots in both the parameter and list. Bounded
lookahead capture preserves the original class-token diagnostic ranges. No valid
syntax or formatter snapshot changes. Reviewed malformed snapshots remain
idempotent and conservative.

Quality audit: the isolated Phase 25 stack passes the full Java syntax corpus,
exact diagnostic ownership, parser loss/progress, normalization, Java formatter
corpus, recovery snapshots, imported formatter idempotence and conservation,
layout boundaries, trivia conservation, CLI tests and snapshots, dprint handler
and wasm smoke tests, Rust 1.96 Java syntax and test-support library clippy,
rustfmt, the Java legacy/ownerless-API architecture gate, and
`git diff
--check`. The unrelated pre-existing extension-comparison lint in the
architecture-gate test remains outside this phase.

Size: the isolated implementation projection is +32,954/-25,246 against
`2197128`, net +7,708. The executable intermediate ceiling is updated to that
value; Phase 29's all-implementation net-negative gate remains binding.

### New Phase 26: Kotlin Atomic Recovery Migration And Legacy Deletion

Move every Kotlin structural diagnostic and recovery completion onto the same
primitive, then delete the old shared and language-local ownership plumbing. Fix
recovery boundaries only where the atomic operation proves that an existing
diagnostic and constructed owner disagree; do not add cascade diagnostics to
satisfy a test and do not widen malformed ownership merely to reduce call-site
work.

Quality gate: apply the same invalid-node bidirectional proof to the complete
Kotlin syntax corpus, not a selected phase-fixture subset. Java and Kotlin must
then expose exactly one structural-diagnostic construction path, with searches
and architecture gates rejecting the deleted ownership setters, repair maps,
range inference, and ownerless malformed completion APIs.

### New Phase 27: Complete-Owner Normalization Authority

Make every token removal, replacement, synthesis, and reorder request name the
smallest complete syntax construct whose valid shape gives that normalization
semantic meaning. Authorization is local to that complete owner, never the whole
file: an unrelated malformed subtree must not disable canonical formatting of a
valid construct. Conversely, a clean child list or token cannot authorize
normalization when its enclosing initializer, call, declaration, modifier
sequence, control statement, or other operation owner is malformed.

Generate or centralize the common owner-validity check and retain
language-specific semantic eligibility only where the language rule genuinely
differs. Delete smaller-scope and whole-tree recovery gates. Deterministic
mutations must prove that malformed represented syntax gains no comma,
semicolon, brace, parenthesis, replacement token, or reordered sequence, while
valid neighboring constructs remain canonically formatted.

Quality gate: every exceptional output token has a source identity or an exact
reason-tagged claim tied to its complete owner; denied claims preserve the
represented tokens and source order. Java and Kotlin normalization,
conservation, diagnostic-inventory, lexical-equivalence, determinism, and
idempotence lanes pass.

### New Phase 28: Proof-Carrying Source Documents

Make source emission and conservation ownership one formatter-IR operation.
Structured token/trivia constructors automatically carry their source identity;
syntax-owned malformed verbatim constructors automatically carry every token and
conserved trivia identity in their exact core; reason-tagged normalization
constructors carry their authorized claim. Rendering the selected document
branch performs those claims automatically. Formatter rules cannot emit
source-backed text and separately remember, omit, duplicate, or fabricate its
claim.

Keep ordinary generated whitespace/layout text distinct from source-backed and
reason-tagged exceptional text. Preserve branch-sensitive rendering, zero
release allocation for debug/test proof state, and centralized lexical safety at
exceptional joins. Delete public/manual claim arrays, independent fragment
ledgers, and formatter-local conservation loops rather than adapting them to the
new constructors.

Quality gate: focused renderer tests prove selected-branch accounting,
unselected-branch non-accounting, foreign-source rejection, malformed-verbatim
completion, and reason-tagged normalization. Full Java/Kotlin corpora prove
exact token and comment conservation with no formatter bookkeeping outside the
IR constructors and renderer.

### New Phase 29: Final Architecture Deletion And Size Closure

Delete every API, test carrier, and compatibility path superseded by Phases
24-28. Consolidate the shared Java/Kotlin factory, typed projection, audit, and
proof machinery where the schemas differ only in declared data. Remove
completion-only migration manifests and post-hoc proof representations once
their invariants are construction-enforced. Do not meet the size gate through
minification, moving implementation into excluded files, weakening tests, or
excluding macro schemas, test support, or tooling.

Quality gate: report macro-schema, generated-consumer, audit/proof, and ordinary
implementation LOC separately. The architecture's explicit `:(glob)` pathspec
must prove that all implementation code, including test support but excluding
fixtures and snapshots, is net negative relative to `2197128`.

### New Phase 30: Clean Completion Proof

Run macro-field exhaustiveness, bogus-category and diagnostic-ownership
snapshots, token/comment tracking, valid zero-verbatim gates, deterministic
mutations, in-repository and imported corpora, CLI/dprint tests, `mise run fix`,
and `mise run test`. Require the imported deferred-path mechanism to be absent,
not merely empty. Scan for valid replay, untracked verbatim, raw-gap layout,
repair synthesis, panic paths, unbounded algorithms, formatter-side structural
layers, independent diagnostic ownership, incomplete-owner normalization, and
manual source-claim bookkeeping. Fail if P16-only ordered recovery parts or
local replay loops were reintroduced.

Fail if two grammar-shape descriptions, two structural-diagnostic construction
paths, two normalization authorization paths, or two source-conservation paths
remain. Re-run the realistic performance/allocation benchmark after the final
production deletions. Change status to `CLEAN` only when every correctness,
architecture, size, and performance gate passes.

## Kotlin Structural Recovery Debt

The detailed items below are historical failure findings and migration inputs.
They do not require bespoke formatting of malformed roles. Under the replacement
architecture, each parser-reachable shape must either expose valid generated
fields or belong to the smallest complete syntax-owned malformed/bogus subtree
and use tracked verbatim output. Valid shapes must never use verbatim.

### Types And Parameters

- Format constraints even when `where` is missing:
  `crates/jolt_kotlin_fmt/src/rules/types.rs:55-64`.
- Format represented bounds when `:` is missing:
  `crates/jolt_kotlin_fmt/src/rules/types.rs:163-185`.
- Preserve recovered `TypeReference` children when no typed family exists:
  `crates/jolt_kotlin_fmt/src/rules/types.rs:202-210`.
- Preserve malformed user-type segments, extra dots, annotations, and unassigned
  type arguments: `crates/jolt_kotlin_fmt/src/rules/types.rs:225-278`.
- Do not let a star projection hide a simultaneous represented type:
  `crates/jolt_kotlin_fmt/src/rules/types.rs:383-400`.
- Do not let the `suspend` nested-function shortcut hide other represented
  function-type pieces: `crates/jolt_kotlin_fmt/src/rules/types.rs:501-517`.
- Preserve names, colons, and recovered tokens in context-function parameters:
  `crates/jolt_kotlin_fmt/src/rules/types.rs:584-595`.
- Preserve all represented definitely-non-nullable type children, not only the
  first two: `crates/jolt_kotlin_fmt/src/rules/types.rs:620-643`.
- Preserve a value-parameter default expression when `=` is missing:
  `crates/jolt_kotlin_fmt/src/rules/variables.rs:86-103`.

### Declarations

- Preserve recovered enum-entry pieces when its expression is absent:
  `crates/jolt_kotlin_fmt/src/rules/declarations.rs:138-147`.
- Preserve secondary-constructor delegation when `:` is missing:
  `crates/jolt_kotlin_fmt/src/rules/declarations.rs:308-334`.
- Replace property-body `unwrap_or_else(nil)` with recovered interleaving:
  `crates/jolt_kotlin_fmt/src/rules/declarations.rs:388-424`.
- Give property-body gaps before, between, and after backing fields/accessors
  explicit ownership:
  `crates/jolt_kotlin_fmt/src/rules/declarations.rs:461-505`.
- Preserve accessor expression tails without `=` and simultaneous recovered
  block/expression pieces:
  `crates/jolt_kotlin_fmt/src/rules/declarations.rs:571-618`.
- Preserve destructuring callable names with a missing close delimiter:
  `crates/jolt_kotlin_fmt/src/rules/declarations.rs:635-644`.
- Preserve callable receiver/separator pieces when the final name is missing:
  `crates/jolt_kotlin_fmt/src/rules/declarations.rs:669-693`.
- Preserve type-alias types when `=` is missing:
  `crates/jolt_kotlin_fmt/src/rules/declarations.rs:747-781`.
- Format context-parameter defaults exposed by syntax accessors:
  `crates/jolt_kotlin_fmt/src/rules/declarations.rs:885-911`.
- Make primary-constructor structure independent of declaration-name,
  opening-parenthesis, and source-gap success:
  `crates/jolt_kotlin_fmt/src/rules/declarations/type_declarations.rs:23-45,358-433`.
- Preserve delegation colons and partial specifier pieces:
  `crates/jolt_kotlin_fmt/src/rules/declarations/type_declarations.rs:244-255,324-355`.
- Prove unclassified class members are genuinely unstructured recovered islands
  or expose their structure through syntax accessors:
  `crates/jolt_kotlin_fmt/src/rules/declarations/member_bodies.rs:269-275`.

### Expressions And Control Flow

- Preserve labels/type arguments when `this` or `super` is missing:
  `crates/jolt_kotlin_fmt/src/rules/expressions/leaves.rs:41-67`.
- Preserve lambda parameters/body/close brace when `{` is missing:
  `crates/jolt_kotlin_fmt/src/rules/expressions/lambdas.rs:27-29`.
- Expose dangling assignment and binary operators without requiring a right
  operand: `crates/jolt_kotlin_syntax/src/nodes/accessors.rs:2385-2426` and
  `crates/jolt_kotlin_fmt/src/rules/expressions/operators.rs:54-69,114-119`.
- Preserve navigation selectors when the operator is missing:
  `crates/jolt_kotlin_fmt/src/rules/expressions/calls.rs:54-57`.
- Replace keyword-missing empty returns for `if`, `when`, `try`, `for`, `while`,
  `do`, jump, and throw nodes:
  `crates/jolt_kotlin_fmt/src/rules/expressions/control_flow.rs:26-28,66-68,129-131,192-194,301-303,334-336,395-397,441-443`.
- Preserve `when` entries without `{` and `do` condition pieces without `while`:
  `crates/jolt_kotlin_fmt/src/rules/expressions/control_flow.rs:69-79,337-350`.
- Preserve lambda-as-branch pieces without `{`:
  `crates/jolt_kotlin_fmt/src/rules/expressions/control_flow.rs:871-873`.
- Honor collection-literal leading-trivia ownership:
  `crates/jolt_kotlin_fmt/src/rules/expressions/calls.rs:152-170`.

### Containers

- Add recovered streams for file items and import-list contents:
  `crates/jolt_kotlin_fmt/src/rules/program.rs:31-64,143-150`.
- Preserve comments owned by EOF in comment-only Kotlin files:
  `crates/jolt_kotlin_fmt/src/rules/program.rs:31-34`.
- Preserve duplicate represented package headers and import lists instead of
  overwriting option slots:
  `crates/jolt_kotlin_fmt/src/rules/program.rs:139-148`.
- Expose ordered recovered pieces inside package headers and import directives,
  not only at the enclosing import-list level:
  `crates/jolt_kotlin_fmt/src/rules/program.rs:426-447` and
  `crates/jolt_kotlin_fmt/src/rules/imports.rs:59-78,87-141`.
- Add recovered streams for `when` bodies and try/catch/finally sequences:
  `crates/jolt_kotlin_fmt/src/rules/expressions/control_flow.rs:81-149`.
- Add recovered call-suffix and user-type segment streams:
  `crates/jolt_kotlin_fmt/src/rules/expressions/calls.rs:100-124` and
  `crates/jolt_kotlin_fmt/src/rules/types.rs:225-278`.
- Add recovered qualified-name segments:
  `crates/jolt_kotlin_fmt/src/rules/names.rs:93-169`.
- Preserve direct type-argument content when the projection-list wrapper is
  absent: `crates/jolt_kotlin_fmt/src/rules/types.rs:312-357`.
- Make generic recovered-list delimiter skipping identify the actual boundary
  token rather than every token of the same kind:
  `crates/jolt_kotlin_syntax/src/nodes/accessors.rs:1524-1605` and callers at
  `:938-949,1302-1308,1366-1372,1815-1826,1902-1913,1963-1969,2037-2048,2847-2853,3319-3333`.
- Do not stop recovered-list ownership at an orphan early close delimiter:
  `crates/jolt_kotlin_syntax/src/nodes/accessors.rs:530-553,1024-1047,1149-1172,2504-2532,2604-2632`.

## Kotlin Formatter-Owned Syntax Debt

### Partial Replay And Ownership Inference

- Replace string-template token replay/range matching with ordered syntax parts:
  `crates/jolt_kotlin_fmt/src/rules/expressions/leaves.rs:92-147`.
- Replace whole-node fallback for identifier-less user types:
  `crates/jolt_kotlin_fmt/src/rules/types.rs:225-232`.
- Replace whole-node fallback for anonymous functions missing `fun`:
  `crates/jolt_kotlin_fmt/src/rules/expressions/functions.rs:19-21`.
- Replace whole-node fallbacks for type-binary, unary, and postfix nodes with
  available-piece formatting:
  `crates/jolt_kotlin_fmt/src/rules/expressions/operators.rs:135-143,543-571`.
- Replace value-argument whole-node fallback with structured prefix plus
  recovered remainder:
  `crates/jolt_kotlin_fmt/src/rules/expressions/calls.rs:727-755`.
- Move receiver-modifier, declaration-prefix, property-body-order,
  user-type-segment, callable-reference type-argument, and named-argument
  ownership into syntax accessors:
  `crates/jolt_kotlin_fmt/src/rules/declarations.rs:177-191,361-365,461-471,802-828`,
  `crates/jolt_kotlin_fmt/src/rules/types.rs:229-275`,
  `crates/jolt_kotlin_fmt/src/rules/expressions/references.rs:82-105`, and
  `crates/jolt_kotlin_fmt/src/rules/expressions/calls.rs:758-787`.
- Make `Name` expose malformed additional pieces instead of taking the first
  token: `crates/jolt_kotlin_fmt/src/rules/names.rs:9-23`.
- Replace expression-order/range role inference for `if`, `for`, and calls:
  `crates/jolt_kotlin_syntax/src/nodes/accessors.rs:2739-2761,2900-2938,3147-3181`.
- Represent `fun interface` as one syntax declaration and remove formatter
  pairing of adjacent function/interface declarations:
  `crates/jolt_kotlin_syntax/src/nodes/accessors.rs:438-457`,
  `crates/jolt_kotlin_fmt/src/rules/program.rs:291-336`, and
  `crates/jolt_kotlin_fmt/src/rules/declarations.rs:71-88`.

### Source Gaps And Complexity

- Replace recovered-gap source slicing with parser trivia ownership:
  `crates/jolt_kotlin_fmt/src/helpers/comments.rs:278-297`.
- Replace raw blank-line counting in block/program layout:
  `crates/jolt_kotlin_fmt/src/rules/statements/blocks.rs:310-348` and
  `crates/jolt_kotlin_fmt/src/rules/program.rs:412-424`.
- Remove declaration and constructor source-gap guards in favor of syntax
  ownership:
  `crates/jolt_kotlin_fmt/src/rules/declarations.rs:336-355,388-420,474-504,995-1038,1089-1120`
  and
  `crates/jolt_kotlin_fmt/src/rules/declarations/type_declarations.rs:358-410`.
- Replace formatter-ignore raw delimiter scanning with represented comment
  ownership as formatter-ignore robustness debt:
  `crates/jolt_kotlin_fmt/src/rules/statements/blocks.rs:247-267`.
- Make string-template and user-type matching linear:
  `crates/jolt_kotlin_fmt/src/rules/expressions/leaves.rs:97-129` and
  `crates/jolt_kotlin_fmt/src/rules/types.rs:229-275`.
- Remove property-body sorting by consuming source-ordered syntax entries:
  `crates/jolt_kotlin_fmt/src/rules/declarations.rs:461-471`.
- Document a finite cost model for import sorting or replace it with a compliant
  bounded strategy: `crates/jolt_kotlin_fmt/src/rules/imports.rs:31-47`.

### Synthesis And Panic

- Prevent malformed import first tokens from being normalized into `import`:
  `crates/jolt_kotlin_syntax/src/nodes/accessors.rs:156-160` and
  `crates/jolt_kotlin_fmt/src/rules/imports.rs:87-105`.
- Move alias normalization preconditions into the normalization helper:
  `crates/jolt_kotlin_fmt/src/rules/imports.rs:121-140`.
- Remove production `expect` calls at
  `crates/jolt_kotlin_fmt/src/rules/names.rs:132-137`,
  `crates/jolt_kotlin_fmt/src/rules/statements/blocks.rs:118-123`,
  `crates/jolt_kotlin_fmt/src/rules/expressions/lambdas.rs:61-63,242-247`,
  `crates/jolt_kotlin_fmt/src/rules/expressions/calls.rs:342-347`, and
  `crates/jolt_kotlin_fmt/src/rules/expressions/control_flow.rs:898-906`.

## Java Structural Recovery Debt

### Valid And Recovered Token Loss

- Preserve trailing unsized annotated array dimensions in array creation:
  `crates/jolt_java_fmt/src/rules/expressions/arrays_objects.rs:79-103` and
  `crates/jolt_java_syntax/src/nodes/accessors.rs:2712-2730`.
- Make singleton variable, lambda-parameter, switch-label, and enum-constant
  optimizations account for recovered siblings:
  `crates/jolt_java_fmt/src/rules/variables.rs:40-47,81-89,329-339`,
  `crates/jolt_java_fmt/src/rules/expressions/lambdas.rs:78-91`,
  `crates/jolt_java_fmt/src/rules/statements/switches.rs:130-135,183-200`, and
  `crates/jolt_java_fmt/src/rules/declarations/enums.rs:30-34,60-73`.
- Preserve duplicate represented package/module declarations instead of
  overwriting option slots:
  `crates/jolt_java_fmt/src/rules/program.rs:91-112,161-172`.
- Preserve partial pattern pieces:
  `crates/jolt_java_fmt/src/rules/patterns.rs:21-28,75-82`.
- Format unclassified `for` pieces rather than returning `nil`:
  `crates/jolt_java_fmt/src/rules/statements/control_flow.rs:215-227`.
- Preserve unclassified switch-rule bodies:
  `crates/jolt_java_fmt/src/rules/statements/switches.rs:469-492`.
- Preserve resource content/trailing-semicolon comments without a resource list
  and catch delimiters without a parameter:
  `crates/jolt_java_fmt/src/rules/statements/try_resources.rs:77-111,290-310`.
- Add malformed method-reference receiver recovery:
  `crates/jolt_java_fmt/src/rules/expressions/method_references.rs:67-89`.
- Preserve both leading and trailing EOF comments in comment-only files:
  `crates/jolt_java_fmt/src/rules/program.rs:27-28` and
  `crates/jolt_java_fmt/src/rules/comments.rs:11-19`.

### Recovered Containers And Accessors

- Add recovered segment streams for names and class types:
  `crates/jolt_java_syntax/src/nodes/accessors.rs:201-227,690-743`.
- Add recovered entries for array dimensions and modifiers:
  `crates/jolt_java_syntax/src/nodes/accessors.rs:2340-2359,4087-4129`.
- Preserve direct annotation-interface and annotation-argument content when
  wrapper lists are absent:
  `crates/jolt_java_syntax/src/nodes/accessors.rs:1008-1038,2426-2442`.
- Expose record-pattern components without requiring source `(`:
  `crates/jolt_java_syntax/src/nodes/accessors.rs:4023-4052`.
- Expose module directives without requiring `{` and target names without
  requiring `to`/`with`:
  `crates/jolt_java_syntax/src/nodes/accessors.rs:4243-4266,5294-5382`.
- Preserve orphan/repeated switch colons:
  `crates/jolt_java_syntax/src/nodes/accessors.rs:3723-3749`.
- Add recovered sequencing between try body, catches, and finally:
  `crates/jolt_java_syntax/src/nodes/accessors.rs:3208-3215,3234-3241`.
- Establish a general consumed-pieces/recovered-siblings contract instead of
  relying on filtering helpers that silently hide unmatched children:
  `crates/jolt_java_syntax/src/nodes/mod.rs:1144-1225`.

## Java Formatter-Owned Syntax Debt

### Partial Replay

- Replace whole-node fallbacks for imports, unclassified annotation values,
  component patterns, empty binary expressions, module directives, type
  arguments, expression statements, resources, switch labels, and block
  statements: `crates/jolt_java_fmt/src/rules/imports.rs:99-110`,
  `crates/jolt_java_fmt/src/rules/annotations.rs:64-75`,
  `crates/jolt_java_fmt/src/rules/patterns.rs:65-72`,
  `crates/jolt_java_fmt/src/rules/expressions/operators.rs:96-114`,
  `crates/jolt_java_fmt/src/rules/modules.rs:351-360,417-491`,
  `crates/jolt_java_fmt/src/rules/types.rs:491-503`,
  `crates/jolt_java_fmt/src/rules/statements/simple.rs:36-42`,
  `crates/jolt_java_fmt/src/rules/statements/try_resources.rs:261-276`,
  `crates/jolt_java_fmt/src/rules/statements/switches.rs:447-459`, and
  `crates/jolt_java_fmt/src/rules/statements/blocks.rs:225-236`.
- Complete the shared `format_token_sequence` audit above for the Java primitive
  at `crates/jolt_java_fmt/src/helpers/comments.rs:354-402`.

### Ownership, Source Gaps, And Complexity

- Move operator class/precedence/associativity decisions from token text to
  syntax-owned operator metadata:
  `crates/jolt_java_fmt/src/rules/expressions/operators.rs:90-93,179-181,259-260,283-288,434-491`.
- Move enum separator source-spelling classification into syntax accessors:
  `crates/jolt_java_fmt/src/rules/declarations/enums.rs:231-274`.
- Remove source-gap layout reconstruction from recovered token formatting:
  `crates/jolt_java_fmt/src/helpers/comments.rs:383-402`.
- Document finite cost models or replace unbounded sorting for imports, module
  directives, and malformed modifier runs:
  `crates/jolt_java_fmt/src/rules/imports.rs:32-49`,
  `crates/jolt_java_fmt/src/rules/modules.rs:296-305`, and
  `crates/jolt_java_fmt/src/helpers/modifiers.rs:74-105`.
- Remove quadratic enum lookahead:
  `crates/jolt_java_fmt/src/rules/declarations/enums.rs:103-163`.
- Make formatter-ignore range/item matching and marker line lookup linear:
  `crates/jolt_fmt_ir/src/formatter_ignore.rs:45-110,172-213,312-325`.
- Make argument parent-role lookup constant-time or single-pass:
  `crates/jolt_java_syntax/src/nodes/accessors.rs:1986-1989`.

### Synthesis And Panic

- Stop repairing missing statement, switch, synchronized, try, catch, and
  finally bodies with synthesized `{}`:
  `crates/jolt_java_fmt/src/rules/statements.rs:105`,
  `crates/jolt_java_fmt/src/rules/statements/switches.rs:27-30`,
  `crates/jolt_java_fmt/src/rules/statements/control_flow.rs:607-610`, and
  `crates/jolt_java_fmt/src/rules/statements/try_resources.rs:27-30,58-61,306-309,509-512`.
- Remove production `expect` calls at
  `crates/jolt_java_fmt/src/rules/modules.rs:318-325` and
  `crates/jolt_java_fmt/src/rules/expressions/member_chains.rs:137-142`.

## Verified Clean Areas

- Raw literal source output is limited to tracked syntax-owned malformed/bogus
  subtrees and formatter-ignore ranges; valid clean corpora have zero verbatim
  coverage.
- Formatter production code does not clone parser-owned source text, token
  buffers, or syntax-node buffers.
- Java enum/list normalization and readability-parenthesis insertion are
  explicitly reason-tagged; malformed missing-body brace repair remains open
  above.
- Kotlin readability parentheses are explicitly reason-tagged.

## Accepted Deviations

- Kotlin intentionally excludes `CallCallee` from member-chain child detection
  because trailing-lambda syntax wraps a call as another call's callee;
  including that role suppresses the top-level chain builder. See
  `crates/jolt_kotlin_fmt/src/rules/expressions/calls.rs:485-489`.
