# Formatter Cleanup Plan

Status: active\
Started: 2026-07-22\
Owner: formatter cleanup stack

This document is the durable plan and execution ledger for simplifying Jolt's
Java and Kotlin formatter architecture. It is intentionally stored in the
repository so that the work can continue coherently across long-running agent
sessions, context compactions, rebases, and reviews.

## Outcome

Make the formatter smaller and easier to reason about locally without weakening
its correctness or performance guarantees.

The desired end state has clear ownership boundaries:

```text
document algebra + renderer
          ^
          | structured layout
          |
syntax-aware formatting core
          ^
          | validated, borrowed CST access
          |
Java syntax                 Kotlin syntax
     ^                           ^
     | language layout rules    | language layout rules
     +----------- facade --------+
```

A reader working on a language rule should only need the node's CST accessors,
the local rule, and a small formatting context. They should not have to reason
about parser recovery probes, source-conservation bookkeeping, repeated ignore
range scans, debug-only IR topology, or renderer internals.

## Priorities

In order:

1. Preserve correctness, losslessness, deterministic output, and bounded
   runtime.
2. Remove concepts, branches, plumbing, and duplicated machinery.
3. Make ownership and invariants visible at the narrowest useful boundary.
4. Make formatter code understandable from nearby code.
5. Preserve or improve performance and memory use.
6. Reduce lines of code where doing so does not hide behavior.

Code growth is a design warning. If a pull request adds substantial machinery
without deleting more complicated machinery or materially improving local
reasoning, stop the stack and reassess the design. LOC is not a target by
itself, but architectural bloat is not an acceptable price for purity.

## Invariants

The repository invariants in `AGENTS.md` apply throughout. In particular:

- formatter syntax access remains borrowed from the parser's existing source,
  token, node, and trivia buffers;
- valid syntax is formatted structurally, never replayed from source as a
  fallback;
- malformed represented trees format consistently without panic or refusal;
- every represented source token and comment is preserved exactly once;
- language rules do not parse token streams or synthesize missing syntax;
- layout remains linear or explicitly bounded, with no best-fit search or
  conditional-group search;
- required fixtures are never silently skipped;
- legacy APIs are removed in the same pull request that replaces them;
- structural pull requests preserve byte-for-byte formatter output unless an
  output change is explicitly isolated, justified, and snapshotted.

Additional cleanup constraints:

- no flag-day parser or formatter rewrite;
- no erased common Java/Kotlin AST;
- no one-trait or one-file-per-node framework;
- no speculative public convenience APIs;
- no compatibility layer for pre-release internal APIs;
- no new crate merely to rename an existing module boundary;
- no abstraction whose only client is hypothetical.

## What Is Already Good

The cleanup must retain these strengths:

- lossless CSTs and borrowed syntax access;
- arena-backed document construction;
- constant-time recovery-free classification on syntax nodes;
- syntax-owned normalization authority and source-conservation auditing;
- bounded fit checking rather than global layout search;
- fixture, snapshot, idempotence, recovery, and trivia-conservation coverage.

## Baseline

Audit measurements at the start of the stack:

| Measure                               |                       Baseline |
| ------------------------------------- | -----------------------------: |
| Rust source                           |                   60,978 lines |
| Java + Kotlin formatter source        |                   21,961 lines |
| `jolt_syntax` source                  |                    6,313 lines |
| `jolt_fmt_ir` source                  |                    4,263 lines |
| committed Java/Kotlin fixtures        |                            689 |
| snapshots                             |    1,220, approximately 4.2 MB |
| Java realistic document density       | approximately 3.14 nodes/token |
| Kotlin realistic document density     | approximately 2.59 nodes/token |
| Java realistic reserved tree memory   |  approximately 113 bytes/token |
| Kotlin realistic reserved tree memory |  approximately 121 bytes/token |

Focused formatter unit, corpus, recovery, layout, trivia-conservation, CLI, and
dprint handler tests passed before the stack. All 689 fixtures produced
byte-identical debug and release output. That parity is an initial observation,
not proof that debug/release IR topology is safe. The full all-target test run
failed because the required external `dprint` executable was missing; the test
correctly failed rather than skipping. Strict Clippy exposed two pre-existing
test-only warnings: an oversized Java parser test and a `clone` on a `Copy`
syntax node.

Record fresh measurements in each pull request. Use a fixed realistic Java and
Kotlin corpus when comparing document density, memory, and elapsed time. Treat
performance numbers as regression detectors, not microbenchmark trophies.

## Principal Rough Spots

### Document meaning changes by build profile

Empty normalization proof claims are real document nodes in debug builds but
collapse to `nil` in release builds. Some language rules compare opaque `Doc`
handles to `nil` to infer semantic visibility. A document handle must describe
the same topology in every profile, and semantic visibility must be explicit.

Primary locations:

- `crates/jolt_fmt_ir/src/document.rs`
- `crates/jolt_kotlin_fmt/src/rules/program.rs`
- `crates/jolt_kotlin_fmt/src/helpers/lists.rs`

### Formatter-ignore planning repeats syntax and source scans

Ignore ranges are discovered below multiple nested containers. Each discovery
can rescan source and collect tokens for an overlapping subtree, making a
nominally local feature capable of superlinear work. Ignore directives are a
file-level concern and should be planned once, then queried or spliced linearly.

Primary location: `crates/jolt_fmt_ir/src/formatter_ignore.rs` and its eleven
language-container call sites.

### Validated trees expose impossible field states

Generated CST accessors return `Result<_, SyntaxInvariantError>` even for trees
constructed by the matching generated factory. Formatters consequently contain
roughly 108 `block_on_invariant` calls, although some of those guard custom
semantic projections rather than generated physical fields. There are also 153
malformed-field branches and 84 malformed or invisible list branches. Shape
validation belongs at construction or typed-root conversion; downstream access
should express field cardinality, not factory implementation uncertainty.

Primary location: `crates/jolt_syntax/src/projection.rs`, generated syntax
factories/accessors, and both language formatters.

### The shared formatter crate owns unrelated layers

`jolt_fmt_ir` currently combines document algebra, rendering, formatter
options/results, source-conservation claims, normalization, recovery, trivia,
lexical safety, and formatter-ignore behavior. This obscures dependency
direction and makes language layout code sensitive to global mechanics.

### Recovery and trivia contracts leak into language rules

Java uses `RequireNonEmptyRange` while Kotlin uses `TokensOnly`; Kotlin also has
`Invisible(Doc)` and `layout_visible`. These differences look more like
inconsistent syntax contracts than language formatting policy. Approximately 280
token-formatting calls manually select trivia-placement modes. Java and Kotlin
duplicate entrypoint, comment, and recovery mechanics, with at least 83 exact
common function names.

### The renderer has multiple interpreters

`render.rs` is approximately 1,897 lines including tests. Rendering and fit
checking separately interpret every document node. In debug mode, source
verification renders the document fully to a discard sink before the real
render. Shared structural traversal and a single auditable verification path
should replace incidental duplication, without making hot loops generic or
allocation-heavy.

### Syntax tooling duplicates mechanisms and parallel grammars

Java lookahead is a roughly 560-line parallel grammar. Java and Kotlin lexers
share around 35 mechanical cursor/trivia function names while separately
implementing UTF-8 movement and trivia collection. Share only language-neutral
cursor mechanics. Keep token classification and lexical semantics
language-owned. Make lookahead memoized or otherwise explicitly bounded rather
than generalizing the grammar twice.

### Test knowledge and architecture knowledge live in the wrong places

The Java formatter corpus hardcodes filename-specific normalization removals.
`docs/internals/formatter.md` omits recovery, source conservation,
normalization, ignore semantics, and fit costs. Tests should report structured
audit facts and architecture docs should explain actual ownership boundaries.

## Target Boundaries

Names are provisional until module boundaries prove useful:

```text
pure document modules
  Document algebra, width model, bounded fit engine, and output sinks.

jolt_syntax + jolt_{java,kotlin}_syntax
  Validated lossless CST, source identities, recovery classification,
  cardinality-aware accessors, and language-owned normalization authority.

root formatting coordination
  One root ignore plan and source audit, plus narrow trivia, recovery, and
  lexical-boundary capabilities passed only where needed.

jolt_{java,kotlin}_fmt
  Language CST to structured layout. Leaf rules normally need only a
  `DocBuilder` and their typed node.

jolt_formatter
  Thin dispatch facade used by the CLI and dprint plugin.
```

Do not create `jolt_doc` or `jolt_fmt_core` up front. First create clean module
boundaries inside the existing crate. Extract a crate only when dependency
direction is stable and the extraction removes coupling or compile surface.
Keeping one well-partitioned crate is preferable to several crates with cyclic
conceptual ownership.

Do not pass a general `FormatContext` through the rule graph. A root coordinator
may own the builder, ignore plan, and audit, but leaf rules receive only the
narrow capabilities they use. Visibility and lexical boundaries may use small
domain values when those values delete ambiguous branches; rules must never
infer either property from opaque document handles.

The generated factory and accessors remain one declarative authority. Do not add
a second runtime schema interpreter or check in generated CST output unless a
measured prototype proves that it deletes more authority and machinery than it
adds. Prior failed dual-model/generated-shape experiments are a warning, not a
starting point.

## Stack

Every pull request is a draft until its dependent slice has passed its gates.
Branches are stacked in this order:

```text
main
  └─ cleanup/00-plan-and-gates
      └─ cleanup/01-doc-semantics
          └─ cleanup/02-formatter-ignore-plan
              └─ cleanup/03-infallible-generated-fields
                  └─ cleanup/04-syntax-recovery-visibility
                      └─ cleanup/05-root-coordination
                          └─ cleanup/06-source-audit-reporting
                              └─ cleanup/07-core-module-boundaries
                                  └─ cleanup/08a-renderer-boundaries
                                      └─ cleanup/08b-renderer-audit-pass
                                          └─ cleanup/09-kotlin-rules
                                              └─ cleanup/10-java-rules
                                                  └─ cleanup/11-lexer-substrate
                                                      └─ cleanup/12-java-lookahead
                                                          └─ cleanup/13-final-reconciliation
```

The plan is deliberately ambitious, but the stack is not immutable. Merge
adjacent entries when one cannot deliver an independently coherent deletion.
Split an entry when review would require holding too many invariants in mind.
Update both the graph and ledger before changing stack shape. Every PR must
compile, test, and be independently revertible; no temporary dual API may
survive a PR boundary.

### PR 00 — Plan and gates

Scope:

- commit this plan and status ledger;
- record debug/release output parity and complexity gates for later PRs;
- capture stable baseline measurements and environment limitations.

Expected simplification: none in production code. Do not build a generic test
framework here; put regression tests beside the invariant changed by the owning
implementation PR.

Gates:

- current focused formatter suites pass;
- all committed fixtures have debug/release output parity;
- baseline commands, prerequisites, and known failures are recorded.

### PR 01 — Profile-independent document semantics

Scope:

- make every `Doc` operation produce the same topology in debug and release;
- remove all semantic `Doc == nil` comparisons;
- use existing CST/list classification or a narrow `(Doc, visible)` result only
  where separator/layout visibility is genuinely needed;
- keep debug normalization/source claims observational rather than structural;
- audit semantic-looking `ConcatBuilder::is_empty` calls and either replace them
  with syntax/counter state or name structural queries explicitly.

Expected deletions: profile-conditioned source-fragment construction, opaque
handle comparisons, and duplicated visibility branches. Do not introduce a
general `Formatted` wrapper solely for this migration.

Risks: normalization audit coverage, group-fit decisions, comment-only nodes,
and increased release document density from zero-width claim nodes.

Gates:

- an all-profile test proves empty claims are non-nil and topology is stable;
- all fixture output remains byte-identical in debug and release;
- normalization/source-conservation failures remain actionable in debug;
- same-machine document density, allocation, memory, and elapsed measurements
  are compared with both the immediate parent and stack baseline.

### PR 02 — One formatter-ignore plan

Scope:

- discover complete ignore-directive pairs once at the formatted root;
- query one immutable plan from syntax-owned content intervals and direct item
  ranges;
- reject a range when it would partially overlap a structured item or cross
  independently formatted syntax partitions;
- consolidate duplicate sequence-splicing paths;
- delete every nested subtree token/source scan API in the same PR.

Expected deletions: eleven overlapping discovery paths, subtree token collection
used only to rediscover source ranges, and duplicate Java/Kotlin splice logic.

Risks: nested containers, directives in malformed syntax, boundary comments, and
disabled regions at file boundaries.

Gates:

- counted-work tests prove linear root discovery and bound query work to
  `O(items * log(ranges + 1) + items + runs)`; wall-clock timing alone is
  insufficient;
- source conservation and trivia tests pass;
- all unrelated formatter snapshots remain byte-identical; any directive-policy
  correction is isolated and snapshotted.

### PR 03 — Infallible generated physical fields

Scope:

- validate factory shape once at generated construction or typed-root entry;
- make only generated physical required/optional/list field accessors
  cardinality-aware and infallible;
- migrate representative Java and Kotlin vertical slices, then the generator;
- delete `SyntaxInvariantError` plumbing that guarded impossible slot mismatch;
- retain fallibility for custom semantic projections with genuine failure.

Expected deletions: generated-slot invariant unwraps and impossible shape
branches. Do not promise removal of all current `block_on_invariant` calls.

Risks: hiding truly optional tokens, conflating parser recovery with invalid
factory shape, generated churn, and weaker diagnostics.

Gates:

- an exhaustive schema audit checks factory/accessor agreement;
- generated code and representative formatter call sites shrink;
- compile time, generated code size, and malformed-tree behavior do not regress;
- there is still one declarative schema authority and no runtime second model.

### PR 04 — Syntax-owned recovery and list visibility

Scope:

- classify recovery fragments and list visibility at the syntax boundary;
- remove Java `RequireNonEmptyRange` versus Kotlin `TokensOnly` policy from
  language rules;
- remove Kotlin `Invisible(Doc)`, `layout_visible`, and equivalent formatter
  special cases once syntax classification replaces them;
- preserve all represented malformed pieces and their trivia.

Expected deletions: malformed/invisible list branches, formatter-selected empty
core policy, and recovery probes that rediscover syntax state.

Risks: dropping invisible-but-conserved recovery docs, treating malformed as
absent, or making language-specific formatting policy look like syntax.

Gates:

- malformed, empty, and comment-only lists have explicit syntax-owned cases;
- every represented token/comment remains claimed exactly once;
- Java and Kotlin recovery fixture output remains byte-identical;
- formatter rule call sites shrink rather than wrap old policy.

### PR 05 — Narrow root coordination

Scope:

- consolidate Java/Kotlin entrypoint and root setup/teardown mechanics;
- let a small root coordinator own only builder, ignore plan, and source audit;
- pass narrow trivia, recovery, or lexical-boundary capabilities only to rules
  that require them;
- delete duplicate root helpers and parameter plumbing as each path moves.

Expected deletions: duplicate root orchestration and per-rule global plumbing.
This PR does not introduce a context threaded through every formatting rule.

Risks: hidden mutable state, lifetime complexity, and a root object expanding
into a service locator.

Gates:

- ordinary leaf rules still accept the builder and typed syntax they use;
- the coordinator has a short field list and no language-layout methods;
- moved helpers and old entrypoints are deleted in the same PR;
- formatter rule LOC and common call-site argument count decline.

### PR 06 — Structured source-audit reporting

Scope:

- replace filename/count normalization allowlists with structured audit facts;
- make failures identify the responsible syntax identity and source range;
- distinguish authorized normalization from loss or duplicate claims;
- delete test knowledge that belongs in syntax-owned normalization authority.

Expected deletions: filename-specific removals, count allowlists, and opaque
debug-only failure paths.

Risks: weakening exact test expectations or moving formatter policy into the
audit layer.

Gates:

- every previous exception is represented by a syntax-owned authorization;
- deleting a required authorization produces a focused deterministic failure;
- output remains byte-identical and audit allocation cost remains bounded.

### PR 07 — Internal core module boundaries

Scope:

- establish internal boundaries between pure documents/rendering and
  syntax-aware formatting mechanics;
- reverse dependencies from document algebra into syntax/language concerns;
- reduce cross-layer re-exports and options/results mixed with algebra;
- extract a crate only if already-clean module edges make the extraction a net
  deletion of coupling, concepts, and compile surface.

Expected deletions: cross-layer re-exports, syntax-aware document helpers, and
obsolete facade modules. Crate extraction is a decision gate, not a deliverable.

Risks: file or crate churn without simplification, public API expansion, and
longer compile times.

Gates:

- module imports make dependency direction obvious;
- concepts and re-exports decrease;
- if extraction adds scaffolding, keep one partitioned crate and record why;
- release native and WASM builds, compile time, and WASM size are compared.

### PR 08a — Renderer boundaries without behavior change

Scope:

- isolate rendering, fit decisions, sinks, and audit observation behind small
  internal modules with explicit contracts;
- keep hot loops concrete and local;
- share node dispatch only where the result is smaller and more readable;
- preserve bounded fit budgets and streaming output.

Expected deletions: renderer state aliases and genuinely duplicate dispatch.
Moving lines between files is not itself a success.

Risks: abstraction overhead, altered group decisions, and churn disguised as
architecture.

Gates:

- output hash and snapshots have zero delta;
- fit work remains explicitly bounded;
- throughput, allocation bytes/count, peak RSS, and document memory are compared
  with both parent and baseline;
- renderer modules can be reasoned about without syntax recovery details.

### PR 08b — Optional single-pass renderer audit

Scope:

- attempt to remove the debug discard render only if source verification can
  observe the actual render without weakening sink semantics;
- specify partial-output, sink-halt, late-error, and error-atomicity behavior;
- abandon this PR if the replacement adds more state or obscures the hot path.

Expected deletion: one complete debug traversal and its discard sink. If that
cannot be deleted cleanly, close this optional PR with the rationale recorded.

Risks: writing partial output before a late conservation failure, coupling audit
state to sink errors, and slowing release rendering.

Gates:

- explicit tests cover sink failure, halted sinks, partial writes, and late
  conservation errors;
- debug verification observes exactly the real rendered bytes and claims;
- release performance and code paths do not regress;
- the implementation is smaller than the two-pass contract it replaces.

### PR 09 — Kotlin rule purification

Scope:

- simplify Kotlin declarations, control flow, calls, and list layout after core
  seams are stable;
- replace booleans/tuples with domain values only when call-site branching and
  impossible states decrease;
- flatten rules so syntax shape and layout choices are visible together;
- delete remaining compatibility/recovery helpers.

Expected deletions: opaque option tuples, repeated list/body layout, and
unreachable branches made impossible by validated CST access.

Risks: accidental style changes and proliferation of tiny types.

Gates:

- each hotspot slice is output-preserving unless explicitly isolated;
- local branches and file size decline;
- every new value eliminates more states/concepts than it introduces;
- Kotlin fixtures, recovery, trivia, and idempotence pass after each slice.

### PR 10 — Java rule purification

Scope:

- simplify Java modules, annotations, member bodies, and remaining hotspots;
- remove Java-only document macros and repeated separator/body layouts;
- consolidate with Kotlin only after identical mechanics are demonstrated;
- keep Java syntax and style decisions language-owned.

Expected deletions: macro indirection, duplicated annotation/module loops,
unreachable defensive branches, and stale ignore/recovery helpers.

Risks: style changes hidden in cleanup and over-sharing with Kotlin.

Gates:

- each hotspot slice is output-preserving unless explicitly isolated;
- local branches, macro surface, and file size decline;
- Java fixtures, recovery, trivia, and idempotence pass after each slice.

### PR 11 — Shared lexer cursor substrate

Scope:

- extract only language-neutral UTF-8 movement and trivia cursor mechanics
  demonstrated to be identical in both lexers;
- retain language-owned token classification and lexical semantics;
- delete the two old mechanical implementations as each operation moves.

Expected deletions: duplicate byte/cursor/trivia movement.

Risks: Unicode boundary bugs, lexer performance loss, and false sharing between
different language rules.

Gates:

- Unicode, trivia, and malformed-input fixtures pass for both languages;
- lexer throughput, allocations, and peak memory do not regress;
- shared APIs contain no Java/Kotlin token semantics;
- new substrate code is materially smaller than the duplicates removed.

### PR 12 — Bounded Java lookahead

Scope:

- measure adversarial work in the Java lookahead parallel grammar;
- memoize, explicitly bound, shrink, or replace productions using existing
  parser mechanisms;
- delete parallel productions as soon as the owning parse decision migrates.

Expected deletions: repeated lookahead work and parallel grammar code.

Risks: changed ambiguity decisions, cache memory growth, or moving the same
grammar complexity behind a generic API.

Gates:

- adversarial tests assert counted finite work, not wall-clock alone;
- Java parser fixtures and recovery snapshots remain stable;
- realistic and adversarial parser time/allocation improve or remain neutral;
- the resulting grammar is shorter and locally traceable.

### PR 13 — Final reconciliation, docs, and API deletions

Scope:

- document actual recovery, conservation, normalization, ignore, and fit-cost
  ownership in `docs/internals/formatter.md`;
- delete all transition harnesses and temporary re-exports;
- polish CLI/dprint/facade APIs only where the completed stack leaves a concrete
  obsolete seam;
- record final metrics, concept/LOC ledger, and unresolved follow-ups.

Expected deletions: temporary adapters and obsolete facade APIs. This is not a
miscellaneous cleanup bucket; behavior changes require their own PR.

Risks: docs describing the intended rather than actual design and speculative
public API work.

Gates:

- docs match code ownership and finite-cost guarantees;
- CLI/dprint integrations remain thin and release WASM behavior/size is stable;
- final metrics compare main, immediate parent, and completed stack;
- every ledger row has verification evidence and no temporary dual API remains.

## Pull Request Contract

Every pull request description records:

- its parent PR and exact dependency;
- behavior intended to remain unchanged;
- code and concepts removed;
- concepts introduced and why they are fewer or narrower;
- how local reasoning improved;
- production and test LOC delta;
- formatter output/snapshot delta;
- test and benchmark results;
- rollback boundary and known follow-ups.

Before starting the next PR, pause for explicit design review if production LOC
grew by more than 200 lines or five percent of the touched subsystem, whichever
is smaller, unless the PR deleted an older cross-cutting concept or has measured
correctness/performance evidence that requires the growth. Any new cross-cutting
concept without a deleted predecessor triggers the same pause regardless of LOC.
This is a reassessment threshold, not a mandate to compress readable code.

Subagents may research, implement a bounded non-overlapping slice, or review a
branch. The stack owner integrates changes, controls branches and commits,
rebases descendants, runs final verification, and publishes PR updates. A
subagent should not add a new abstraction outside its assigned slice without
first updating this plan through the stack owner.

## Verification Matrix

Run the narrowest relevant checks during implementation and the full available
matrix before publishing each draft update.

| Concern           | Required evidence                                                                                     |
| ----------------- | ----------------------------------------------------------------------------------------------------- |
| Build profiles    | debug/release IR and output parity                                                                    |
| Java formatting   | unit, corpus snapshots, recovery, layout, idempotence                                                 |
| Kotlin formatting | unit, corpus snapshots, recovery, layout, idempotence                                                 |
| Losslessness      | source-conservation and trivia ownership checks                                                       |
| Ignore handling   | beginning/end, nested containers, malformed ranges, counted stress bound                              |
| Syntax            | parser/lexer fixture snapshots and malformed input                                                    |
| Integrations      | `jolt_formatter`, CLI, dprint handler, native release, and release WASM size                          |
| Sink semantics    | halted/erroring sinks, partial output, and late conservation errors where rendering changes           |
| Static analysis   | formatting and strict Clippy, with pre-existing debt identified                                       |
| Complexity        | counted deep-nesting and adversarial lookahead/ignore work, not timing alone                          |
| Performance       | same-machine parent/baseline time, allocation count/bytes, peak RSS, doc nodes/token, reserved memory |

Prefer the repository's Ona `test` automation. If environment automation cannot
run, use repository `mise` tasks. Use direct Cargo commands only when those
layers are unavailable or blocked, and record the fallback and missing external
prerequisites. Never weaken a test because the environment lacks a required
fixture or executable.

## Execution Ledger

This table is the source of truth after a context compaction. Update it whenever
a branch is created, a PR is opened, scope changes, a gate fails, or a PR is
ready for review.

| PR  | Branch                                   | Status     | Parent | Draft PR                                     | Verification              | Notes                                         |
| --- | ---------------------------------------- | ---------- | ------ | -------------------------------------------- | ------------------------- | --------------------------------------------- |
| 00  | `cleanup/00-plan-and-gates`              | draft open | `main` | [#2](https://github.com/sargunv/jolt/pull/2) | baseline audit complete   | Durable plan and gates only.                  |
| 01  | `cleanup/01-doc-semantics`               | draft open | PR 00  | [#3](https://github.com/sargunv/jolt/pull/3) | debug/release + benchmark | Profile-independent topology/presence.        |
| 02  | `cleanup/02-formatter-ignore-plan`       | draft open | PR 01  | [#4](https://github.com/sargunv/jolt/pull/4) | debug/release + benchmark | Root plan with bounded immutable queries.     |
| 03  | `cleanup/03-infallible-generated-fields` | planned    | PR 02  | —                                            | —                         | Generated physical slots only.                |
| 04  | `cleanup/04-syntax-recovery-visibility`  | planned    | PR 03  | —                                            | —                         | Syntax-owned recovery/list classification.    |
| 05  | `cleanup/05-root-coordination`           | planned    | PR 04  | —                                            | —                         | Narrow root ownership, no god context.        |
| 06  | `cleanup/06-source-audit-reporting`      | planned    | PR 05  | —                                            | —                         | Structured normalization facts.               |
| 07  | `cleanup/07-core-module-boundaries`      | planned    | PR 06  | —                                            | —                         | Conditional crate extraction gate.            |
| 08a | `cleanup/08a-renderer-boundaries`        | planned    | PR 07  | —                                            | —                         | Separate contracts without output changes.    |
| 08b | `cleanup/08b-renderer-audit-pass`        | optional   | PR 08a | —                                            | —                         | Proceed only if two-pass semantics shrink.    |
| 09  | `cleanup/09-kotlin-rules`                | planned    | PR 08b | —                                            | —                         | Kotlin hotspot purification.                  |
| 10  | `cleanup/10-java-rules`                  | planned    | PR 09  | —                                            | —                         | Java hotspot purification.                    |
| 11  | `cleanup/11-lexer-substrate`             | planned    | PR 10  | —                                            | —                         | Share cursor mechanics only.                  |
| 12  | `cleanup/12-java-lookahead`              | planned    | PR 11  | —                                            | —                         | Counted bounded lookahead work.               |
| 13  | `cleanup/13-final-reconciliation`        | planned    | PR 12  | —                                            | —                         | Actual docs, metrics, and API deletions only. |

### PR 01 evidence

- Empty source claims now have identical non-nil document topology in debug and
  release; all ten language-rule `Doc ==/!= Doc::nil()` decisions were removed.
- Java/Kotlin layout presence is derived from represented tokens or existing
  list visibility while zero-width proof/recovery documents remain emitted.
- Debug `jolt_fmt_ir`, Java, and Kotlin suites passed, including both corpora,
  imported fixtures, recovery, fit, trivia conservation, and idempotence.
- The new topology invariant and all Java/Kotlin formatter suites passed in
  release. Strict library Clippy passed.
- Code-only delta before the ledger update: +326/-210 lines (+116 net). This is
  below the reassessment threshold; the added state deletes opaque document
  identity decisions and makes malformed/list visibility explicit.
- Clean same-machine parent/child measurements found Java/Kotlin document-node
  growth of 2.19%/1.54%, reserved document memory growth of 1.99%/1.02%, and
  allocation-count growth of 0.024%/0.021%. Max-live memory was unchanged. Java
  timing moved +2.68% in a bimodal run and Kotlin -2.19%; neither is strong
  runtime evidence. The bounded structural cost is accepted for
  profile-independent semantics.
- Peak RSS was unavailable because the repository benchmark's required
  `/usr/bin/time` executable is absent. Tests were run through direct Cargo
  commands because the Ona automation had previously failed to start without
  systemd; no test was skipped or weakened.
- `inline_modifier_prefix_from_docs` still infers annotation presence from its
  builder because its opaque `Doc` input lacks visibility metadata. A local
  replacement produced a corpus regression and was rejected. PR 04 must remove
  this last syntax-contract leak when recovery/list visibility becomes
  syntax-owned; PR 01 does not add a second or temporary API for it.

### PR 02 evidence

- Java and Kotlin discover complete formatter-ignore pairs once from the root
  token/trivia buffers. All eleven nested discovery paths, relative-base
  plumbing, and subtree token collections used for rediscovery are deleted.
- The plan contains absolute borrowed ranges and is immutable and
  order-independent. Empty plans do not evaluate item-range iterators or
  allocate per-container vectors; the Java member-body common path remains
  streaming.
- Containers query only the syntax-owned interval between their represented
  delimiters, with list-owned recovery fallbacks. A range is rejected when it
  partially overlaps a sibling or crosses independently formatted enum
  constant/member partitions; its control comments then remain structured.
- Complete pairs are first-off-wins. Adjacent same-line `on`/`off` transitions
  use disjoint ownership. The existing adjacent-range integration fixture and a
  focused enum-boundary fixture isolate and snapshot these policies.
- Root discovery is linear in source/tokens. Counted binary-search comparisons
  enforce the per-query bound `O(items * log(ranges + 1) + items + runs)`.
- Missing and duplicate plan installation use the renderer's existing invariant
  diagnostic path rather than debug-only panics.
- The private, install-once plan currently travels on `DocBuilder`, never
  `DocArena`. PR 05 must revisit this temporary transport when root coordination
  is consolidated; PR 07 must not preserve it merely to justify a new crate.
- Production Rust before test modules is +83 lines: `jolt_fmt_ir` +122 and the
  language formatters -39. Test-module Rust is +422 lines. This is below the
  reassessment threshold and centralizes ownership checks while shrinking the
  language rules; the next slices must not treat the core growth as precedent
  for a general context.
- Repository-defined Ona automation passed all 179 workspace tests with no
  skips. Focused strict Clippy, formatting, Java/Kotlin corpora, recovery,
  trivia, layout, idempotence, and syntax tests passed. Java/Kotlin formatter
  suites and the 17 profile-valid formatter-ignore/builder tests passed in
  release. Nine unrelated `jolt_fmt_ir` conservation tests are intentionally
  debug-only and fail if run as release assertions. Workspace native/WASM
  checks, the native release build, and the optimized dprint plugin build also
  passed.
- Clean same-machine PR 01/PR 02 measurements found identical Java/Kotlin
  document nodes, children, logical/reserved bytes, and nodes/token. Java
  allocation count/bytes declined 0.008%/0.069%, maximum-live allocation count
  declined 0.43%, and maximum-live bytes were unchanged. Every Kotlin allocation
  and maximum-live metric was identical.
- Median format timing moved -5.74% for Java and -13.92% for Kotlin, but the
  Java child had a large outlier and the Kotlin parent was bimodal; treat these
  as reassuring non-regression evidence, not causal speedup claims. Peak RSS
  remains unavailable because `/usr/bin/time` is absent.

## Decision Log

| Date       | Decision                                                     | Reason                                                                                                  |
| ---------- | ------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------- |
| 2026-07-22 | Use a stack of small draft PRs rather than a rewrite.        | Preserves review and rollback boundaries while allowing ambitious end-state changes.                    |
| 2026-07-22 | Centralize branches, commits, rebases, and publication.      | Subagents share a filesystem; central integration avoids hidden branch state and conflicting commits.   |
| 2026-07-22 | Make crate extraction conditional.                           | Purity is about dependency direction, not maximizing crate count.                                       |
| 2026-07-22 | Treat growth without deleted complexity as a stop condition. | The cleanup exists to reduce reasoning load, not install a new framework.                               |
| 2026-07-22 | Keep leaf rules free of a general formatting context.        | Narrow dependencies prevent a service locator and preserve local reasoning.                             |
| 2026-07-22 | Split CST, recovery, renderer, language, and lexer risks.    | Each now has an independent correctness model, benchmark gate, and rollback boundary.                   |
| 2026-07-22 | Accept PR 01's bounded release document-node cost.           | Stable topology costs 1.5–2.2% more nodes but keeps allocations/max-live memory effectively flat.       |
| 2026-07-22 | Use immutable bounded ignore-plan queries.                   | Independent nested rules stay order-independent; counted binary search replaces hidden mutable cursors. |
| 2026-07-22 | Reject partial and cross-partition ignore ranges.            | Structured syntax ownership must never overlap a raw source claim; rejected markers remain structured.  |
| 2026-07-22 | Temporarily transport the ignore plan on `DocBuilder`.       | The field is private, immutable, install-once, and absent from `DocArena`; PR 05 must revisit it.       |

## Resume Protocol

After any interruption or context compaction:

1. Read this entire document and repository `AGENTS.md`.
2. Inspect `git status`, the current branch, recent commits, and the ledger.
3. Inspect open subagents and collect their reports before assigning more work.
4. Verify that the current branch is based on the parent recorded in the ledger;
   never silently flatten the stack.
5. Continue the first `in progress` ledger row. Do not start a later
   architecture until the current PR has its gate evidence and draft PR URL.
6. Update the ledger, metrics, and decision log as facts change.

## Final Success Criteria

The cleanup is complete when:

- language rules consume validated borrowed CST fields and express structured
  layout without parser/recovery probing;
- document semantics are profile-independent and do not encode visibility in
  opaque handles;
- formatter-ignore planning and fit/layout work have documented finite bounds;
- source auditing, recovery, trivia, and lexical boundaries each have one
  obvious owner;
- pure document/render code does not depend on syntax concerns;
- Java/Kotlin duplication is limited to genuine language semantics;
- the largest formatter hotspots are materially easier to understand locally;
- production concepts and code have shrunk overall, or any growth has explicit
  measured justification;
- formatter output, losslessness, malformed-tree support, performance, CLI, and
  dprint behavior remain verified;
- architecture documentation describes the system that actually exists.
