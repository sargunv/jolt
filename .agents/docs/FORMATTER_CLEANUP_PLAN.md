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

- make generated valid/list casts exclude syntax-owned directly malformed nodes,
  while schema-declared malformed wrappers retain that ownership;
- rely only on the existing production factory proof: fixed slots and accessors
  are generated from the same schema invocation;
- remove the outer `SyntaxInvariantError` result from generated physical
  required/optional fields and list parts;
- delete formatter error plumbing that guarded impossible production-factory
  slot mismatch;
- retain fallibility for custom semantic projections with genuine failure.

Expected deletions: generated-slot invariant unwraps and impossible shape
branches. Do not promise removal of all current `block_on_invariant` calls.

This slice does not redesign optional or recovery values. `Present`, `Missing`,
`Malformed`, separators, recovery visibility, and fallible verbatim-core access
remain represented until PR 04. The doc-hidden low-level custom factory surface
is trusted internal construction and does not define the typed language API's
production-factory guarantee. Do not add runtime validation, provenance flags,
proof graphs, dual accessors, or a second schema model to support adversarial
custom factories.

Risks: hiding truly optional tokens, conflating parser recovery with invalid
factory shape, generated churn, and weaker diagnostics.

Gates:

- the schema-derived physical audit checks every represented non-direct node,
  and focused tests prove cast ownership and malformed-source roots;
- exactly the generated physical field/list invariant branches disappear while
  genuinely fallible semantic projections remain;
- generated code and representative Java/Kotlin formatter call sites shrink;
- compile time, generated code size, and malformed-tree behavior do not regress;
- there is still one declarative schema authority and no runtime second model.

### PR 04 — Syntax-owned malformed boundaries

Scope:

- let exceptional fragments decide lexical joins from their own syntax-owned
  boundary atoms;
- delete the Java `RequireNonEmptyRange` versus Kotlin `TokensOnly` policy enum,
  language arguments, and export without adding a replacement presence flag;
- retain physical variable-list `Missing` and tokenless `Malformed` parts,
  because they are positional recovery barriers and can carry source claims;
- retain Kotlin `Invisible(Doc)`, `layout_visible`, and Java's parallel
  visibility-aware resolver until a separate barrier-aware layout adapter can
  preserve claims without a second syntax-list API.

Expected deletions: the formatter-selected empty-core policy and its
language-specific plumbing.

Risks: zero-token or zero-width malformed fragments accidentally selecting
lexical separators, and boundary-comment relocation exposing the wrong neighbor.

Gates:

- zero-token malformed cores emit no lexical separator when exceptional
  neighbors are offered;
- boundary comments still suppress the neighbor on the relocated side;
- every represented token/comment remains claimed exactly once;
- Java and Kotlin recovery fixture output remains byte-identical;
- the policy enum, export, imports, and call-site arguments are deleted without
  a replacement visibility or presence concept.

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

| PR  | Branch                                   | Status      | Parent | Draft PR                                     | Verification               | Notes                                         |
| --- | ---------------------------------------- | ----------- | ------ | -------------------------------------------- | -------------------------- | --------------------------------------------- |
| 00  | `cleanup/00-plan-and-gates`              | draft open  | `main` | [#2](https://github.com/sargunv/jolt/pull/2) | baseline audit complete    | Durable plan and gates only.                  |
| 01  | `cleanup/01-doc-semantics`               | draft open  | PR 00  | [#3](https://github.com/sargunv/jolt/pull/3) | debug/release + benchmark  | Profile-independent topology/presence.        |
| 02  | `cleanup/02-formatter-ignore-plan`       | draft open  | PR 01  | [#4](https://github.com/sargunv/jolt/pull/4) | debug/release + benchmark  | Root plan with bounded immutable queries.     |
| 03  | `cleanup/03-infallible-generated-fields` | draft open  | PR 02  | [#5](https://github.com/sargunv/jolt/pull/5) | debug/release + benchmark  | Generated physical slots only.                |
| 04  | `cleanup/04-syntax-recovery-visibility`  | draft open  | PR 03  | [#6](https://github.com/sargunv/jolt/pull/6) | full + release + benchmark | Syntax-owned malformed lexical boundaries.    |
| 05  | `cleanup/05-root-coordination`           | draft open  | PR 04  | [#7](https://github.com/sargunv/jolt/pull/7) | full + release + benchmark | Narrow root ownership, no god context.        |
| 06  | `cleanup/06-source-audit-reporting`      | draft open  | PR 05  | [#8](https://github.com/sargunv/jolt/pull/8) | full + release + benchmark | Syntax claims replace filename/count policy.  |
| 07  | `cleanup/07-core-module-boundaries`      | in progress | PR 06  | —                                            | full + release + benchmark | Keep one crate; publish pending.              |
| 08a | `cleanup/08a-renderer-boundaries`        | planned     | PR 07  | —                                            | —                          | Separate contracts without output changes.    |
| 08b | `cleanup/08b-renderer-audit-pass`        | optional    | PR 08a | —                                            | —                          | Proceed only if two-pass semantics shrink.    |
| 09  | `cleanup/09-kotlin-rules`                | planned     | PR 08b | —                                            | —                          | Kotlin hotspot purification.                  |
| 10  | `cleanup/10-java-rules`                  | planned     | PR 09  | —                                            | —                          | Java hotspot purification.                    |
| 11  | `cleanup/11-lexer-substrate`             | planned     | PR 10  | —                                            | —                          | Share cursor mechanics only.                  |
| 12  | `cleanup/12-java-lookahead`              | planned     | PR 11  | —                                            | —                          | Counted bounded lookahead work.               |
| 13  | `cleanup/13-final-reconciliation`        | planned     | PR 12  | —                                            | —                          | Actual docs, metrics, and API deletions only. |

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

### PR 03 evidence

- Generated valid/list wrappers now reject directly malformed ownership, while
  schema-declared malformed wrappers require it. Generated physical fields and
  list parts return `Present`, `Missing`, `Malformed`, item, or separator states
  directly; custom semantic projections remain fallible.
- Exactly 28 generated physical invariant-forwarding branches were deleted. The
  formatter retains 85 semantic or unrelated invariant sites: Java keeps five
  `JavaSyntaxInvariantError` references for custom projections and Kotlin keeps
  no `KotlinSyntaxInvariantError` formatter references.
- The production factory, fixed slots, accessors, and physical audits still
  derive from one schema invocation. One private cold contradiction path covers
  bugs in schema lowering or values built through the doc-hidden custom
  factory/tree-sink boundary. No validator, provenance flag, proof model,
  optional-field redesign, or dual API was added.
- Focused integration tests cover required missing, optional malformed,
  malformed list items, valid/list cast rejection, malformed-wrapper ownership,
  Java category-bogus family/list behavior, and malformed-source Java/Kotlin
  typed roots. Complete corpus schema audits still traverse both languages.
- Production Rust is +594/-1,025 lines (-431 net); all Rust is +743/-1,060 (-317
  net). The shared projection/accessor generator is 1,064 lines versus 1,068 in
  PR 02. The language formatter migration accounts for most deletions.
- Repository-defined Ona automation passed 183 workspace tests with no skips.
  `mise run check`, focused strict Clippy, formatting, syntax/formatter corpora,
  imported fixtures, recovery, trivia, layout, idempotence, release formatter
  suites, the native release build, and the optimized dprint WASM smoke build
  passed. Formatter output and snapshots are unchanged.
- Clean package check was 9.173 seconds versus 9.140 in PR 02; the release
  package build was 19.515 seconds versus 19.425. Raw WASM declined from
  2,333,463 to 2,214,345 bytes and optimized WASM from 1,862,892 to 1,767,457
  bytes. The final optimized hash is
  `b623cdd3fa903c32928283f39342c0d9c342256d8779a4452724009c7f039364`.
- Java/Kotlin document topology, allocation count/bytes, and maximum-live
  allocation metrics are identical to PR 02. Stable Kotlin native timing pairs
  with the retained native accessor inline policy moved -0.45% and -0.12%; Java
  runs were noisy and showed no credible regression. The native CLI grew 179,192
  bytes (+3.11%). This tradeoff preserves native throughput while the optimized
  WASM shrinks 95,435 bytes (-5.12%).
- Generated accessors keep their established native inline hint but leave WASM
  unhinted. Required/optional field resolution is a documented WASM-only codegen
  boundary because direct aggregate projection otherwise duplicates every layout
  rule. It is not classified as a cold path.
- Production parser roots are fixed non-direct-recovery owners and remain
  castable for malformed sources. Arbitrary doc-hidden custom trees are outside
  that production guarantee; PR 04 must retain malformed, missing, separator,
  recovery-visibility, and fallible verbatim-core states while refining their
  ownership.

### PR 04 evidence

- An attempted global source-bearing list projection was rejected before
  publication. Java fixtures contain real interior list shapes such as
  `item, comma, empty, comma, item`; those empty positions stop sorting and
  normalization runs. Kotlin likewise uses physical recovery positions to
  normalize valid runs on either side independently.
- Omitting physical empty parts caused a Java recovery corpus source-audit
  failure for a trailing block comment. Replacing positional barriers with a
  whole-list `is_recovery_free()` gate changed Kotlin malformed-file-item
  normalization output. The sparse projection also changed exact iterator size
  hints and removed existing capacity reservations.
- The experiment was fully restored. Physical `Missing`, tokenless `Malformed`,
  `Invisible(Doc)`, `layout_visible`, and visibility-aware Java resolution
  remain unchanged; no second list or recovery-run API was added.
- The retained code delta only deletes `MalformedBoundaryPolicy`, its export,
  and the Java/Kotlin policy arguments. Exceptional fragments now always receive
  syntax-owned neighboring tokens, except on sides whose boundary comments were
  relocated. The 44 shared IR tests and both language recovery snapshot suites
  pass unchanged.
- Production Rust is +18/-49 lines (-31 net). Repository-defined Ona automation
  passed all 183 workspace tests with no skips. `mise run fix`,
  `mise run check`, both complete release formatter suites, the PGO native
  build, and the optimized dprint plugin build passed.
- Clean paired PR 03/PR 04 measurements have identical syntax/document topology,
  allocation count/bytes, and maximum-live allocation metrics. Java median
  format time moved -3.50% and Kotlin -1.38%; treat both as non-regression
  evidence rather than a causal speedup.
- The non-PGO native CLI is 5,955,096 bytes versus 5,952,640 (+0.04%). Optimized
  WASM is 1,768,947 bytes versus 1,767,457 (+0.08%), with SHA-256
  `3cc8b03d6c44d5f9f726c4363cc438047c603e8325c86f9198bc0e3ce7822625`. This
  bounded codegen cost is accepted for the newly exercised Java zero-range
  neighbor path; the source and conceptual model still shrink.
- The complete benchmark wrapper could not collect peak RSS because this
  environment lacks `/usr/bin/time`. Its exact timing and allocation drivers ran
  against the same 9,206-file Java and 485-file Kotlin corpora in isolated
  parent/child builds; no test or measurement was silently skipped.

### PR 05 evidence

- One 59-line shared root function now owns formatter-ignore discovery,
  root-builder construction, layout handoff, arena metrics, source-conserving
  render, and sink/error outcome mapping. Parsing and stable diagnostic policy
  remain language-owned.
- `FormatterIgnorePlan`, its discovery function, root builder construction,
  source rendering, and its internal outcome are crate-private. The public
  install-after-construction setter and its double-install state/test are gone;
  a source-format run receives its immutable plan atomically.
- All 682 Java/Kotlin leaf `DocBuilder` parameters and all nine ignore-aware
  container queries are unchanged. No context object, language trait, extra plan
  parameter, token scan, source clone, or new crate was introduced.
- The copied Java/Kotlin root lifecycles and their one-use ignore-plan wrappers
  are deleted, as is one Kotlin forwarding layer. Rust source is +118/-137 lines
  (-19 net), including the new root module and test-state cleanup.
- Repository-defined Ona automation passed all 182 workspace tests with no
  skips. The count declined by one because atomic root construction makes the
  former double-plan-install test state unrepresentable. Java exercised 1,019
  fixture iterations and 289 snapshots; Kotlin exercised 557 imported files, 265
  corpus files, 53 recovery fixtures, and 212 snapshots without output changes.
- `mise run fix`, `mise run check`, default and benchmark-feature package
  checks, both full release formatter suites, the PGO native build, and the
  optimized dprint plugin build passed.
- Syntax/document topology and every allocation metric are identical to PR 04.
  Alternating isolated parent/child timing runs had pooled median movement of
  -1.46% for Java and -0.16% for Kotlin; dispersion and host drift make these
  non-regression signals, not causal speedup claims. Peak RSS remains
  unavailable because this environment lacks `/usr/bin/time`. The non-PGO CLI
  shrank from 5,955,096 to 5,950,968 bytes (-0.07%); optimized WASM shrank from
  1,768,947 to 1,767,501 bytes (-0.08%), with SHA-256
  `5eb348e99f1f4fc84ad98a2e933fe21226b88e6671d05078e0279c902f6561de`.

### PR 06 evidence

- Dense selected-render accounting is now the sole source-loss authority in
  debug and test builds. Normalization failures retain the closed replacement,
  removal, synthesis, or reorder operation while missing, duplicate, foreign,
  and unauthorized identities report exact token/trivia indices, sides, kinds,
  ordinals, and source ranges.
- `RepresentedTokenRemoval`, both token-loss reporters, both duplicated
  conservation-failure builders, token inventories, clean/recovery allowance
  callbacks, Java's 17 filename policies, both unlimited recovery-directory
  exceptions, and the imported-Java normalization rediscovery traversal are
  deleted. Reparse diagnostics, comment inventories, trivia markers,
  determinism, idempotence, snapshots, and exact debug identity accounting
  remain.
- The syntax layer is intentionally trusted to issue semantics-preserving
  normalization claims. The deleted spelling/count inventory could neither
  identify which equal-spelling token disappeared nor reject output-only tokens.
  Release suites prove output/snapshot/idempotence parity; the dense identity
  proof remains debug-only and the private formatter-ignore adapter remains
  backed by its dedicated range and snapshot suites.
- The production-growth reassessment rejected a larger successful-fact report,
  test-only formatter API, owner descriptor, four tracker methods, and storing
  complete claim wrappers in documents. That prototype added 131 production
  lines and accidentally made synthesis permissions reusable through `Copy`. The
  retained failure-only design keeps claims affine, document topology and size
  unchanged, and adds one debug-only closed operation value.
- Rust source is +329/-534 lines (-205 net): shipping source before test modules
  is +100 lines for exact audit identities/reporting, while test and
  test-support policy shrinks by 305 lines. The net deletion and removal of the
  parallel normalization policy justify the deliberately reviewed production
  diagnostic cost; no context, observer, report vector, traversal, or release
  allocation is introduced.
- Repository-defined Ona automation passed all 185 workspace tests with no
  skips. Focused debug and release Java/Kotlin corpus, recovery, and imported
  fixture suites passed 6/6, covering 425/265 committed fixtures, 54/53 recovery
  subsets, and 521/557 imported fixtures without snapshot changes. The 45 IR, 17
  syntax, and four test-support tests pass in debug; the inherited release IR
  audit/profile mismatch remains intentionally outside the release suite.
- `mise run fix`, `mise run check`, strict workspace Clippy, native/WASM checks,
  the 9,899-file PGO build, and the optimized dprint plugin build passed. The
  full benchmark wrapper again stopped only at its unavailable required
  `/usr/bin/time`; exact timing and allocation drivers completed separately.
- Parent/child topology and all allocation counts/bytes are identical. Median
  timing moved +0.77% for Java and -15.05% for Kotlin; the Kotlin host shift
  makes these non-regression evidence only. The non-PGO CLI is exactly unchanged
  at 5,950,968 bytes. Optimized WASM is 1,767,697 bytes, +196 (+0.011%), with
  SHA-256 `d34636c0956b3371a5f462ad4cd1ff307b5638fbc01bf0eb699a1ce94d8968ae`.

### PR 07 evidence

- Crate extraction was rejected after tracing the real strongly connected
  component: documents carry debug source claims, the root builder owns the
  formatter-ignore plan, and rendering observes source-conservation proof state.
  Splitting files or crates would require a context, side table, callback trait,
  or generic audit layer while deleting no dependency. The existing partitioned
  crate is smaller and makes those lifecycle edges explicit; PR 08a retains the
  renderer-specific boundary work.
- Replacement and synthesis normalization now return ordinary `Doc` values. They
  no longer construct exceptional lexical fragments only to resolve them with no
  neighbors. The unused exceptional join operation, lexical-kind projection,
  identity resolution calls, and two hypothetical import-keyword normalization
  capabilities are deleted. Source claims remain affine and the selected-render
  conservation proof is unchanged.
- `DocBuilder` can no longer be default-constructed outside the crate or release
  its arena. Arena identifiers, arena storage, renderer options, text widths,
  ignore-run coordinates, malformed fragment construction, and exceptional
  resolution are crate-owned. The one language rule that needs the first ignored
  member receives a behavioral accessor rather than raw run fields.
- The compiler-reachable `jolt_fmt_ir` root surface fell from 46 to 28 names
  under default features and 47 to 29 with benchmarks. The Java/Kotlin formatter
  roots together fell from six to two default names, and `NormalizedToken` fell
  from nine to seven closed variants. Both five-line formatter-ignore alias
  modules are deleted, and Java normalization capabilities come from the syntax
  crate that owns them.
- `jolt_formatter` now reexports the two sink contracts required by its public
  function, allowing the CLI and dprint plugin to delete three direct
  `jolt_fmt_ir` dependency declarations without changing the transitive build.
  This is the only added facade surface; language formatter crates remain
  implementation details.
- Rust source is +149/-293 lines (-144 net). Excluding integration-test import
  changes, Rust is +137/-287 (-150 net). No crate, context object, side table,
  callback, compatibility path, or second lifecycle was introduced.
- Repository-defined Ona automation passed all 184 workspace tests with no
  skips, including dprint's external smoke tests. `mise run fix`,
  `mise run
  check`, dependency machete, all-features workspace checking,
  strict Clippy, WASM checking, both complete release formatter suites, the
  9,899-file PGO build, and the optimized dprint plugin build passed. Formatter
  snapshots and output are unchanged.
- Clean workspace check time was 19.834 seconds for PR 06 and 19.962 seconds for
  PR 07 (+0.65%). Parent and child syntax/document topology and every allocation
  count, total-byte, and maximum-live-byte measurement are identical on the
  9,206-file Java and 485-file Kotlin corpora. Alternating format-only medians
  moved -4.37% for Java and -2.36% for Kotlin; treat these only as
  non-regression evidence.
- The non-PGO native CLI shrank from 5,950,968 to 5,948,344 bytes (-0.04%). The
  optimized WASM plugin shrank from 1,767,697 to 1,766,057 bytes (-0.09%), with
  SHA-256 `12f75107b0b83dc0527a154448258cd32ddc4959d4340b14fea6cabc973e8230`.
  Peak RSS remains unavailable because this environment lacks the benchmark's
  required `/usr/bin/time`; no test or other measurement was skipped.

## Decision Log

| Date       | Decision                                                       | Reason                                                                                                                                                                                                     |
| ---------- | -------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 2026-07-22 | Use a stack of small draft PRs rather than a rewrite.          | Preserves review and rollback boundaries while allowing ambitious end-state changes.                                                                                                                       |
| 2026-07-22 | Centralize branches, commits, rebases, and publication.        | Subagents share a filesystem; central integration avoids hidden branch state and conflicting commits.                                                                                                      |
| 2026-07-22 | Make crate extraction conditional.                             | Purity is about dependency direction, not maximizing crate count.                                                                                                                                          |
| 2026-07-22 | Treat growth without deleted complexity as a stop condition.   | The cleanup exists to reduce reasoning load, not install a new framework.                                                                                                                                  |
| 2026-07-22 | Keep leaf rules free of a general formatting context.          | Narrow dependencies prevent a service locator and preserve local reasoning.                                                                                                                                |
| 2026-07-22 | Split CST, recovery, renderer, language, and lexer risks.      | Each now has an independent correctness model, benchmark gate, and rollback boundary.                                                                                                                      |
| 2026-07-22 | Accept PR 01's bounded release document-node cost.             | Stable topology costs 1.5–2.2% more nodes but keeps allocations/max-live memory effectively flat.                                                                                                          |
| 2026-07-22 | Use immutable bounded ignore-plan queries.                     | Independent nested rules stay order-independent; counted binary search replaces hidden mutable cursors.                                                                                                    |
| 2026-07-22 | Reject partial and cross-partition ignore ranges.              | Structured syntax ownership must never overlap a raw source claim; rejected markers remain structured.                                                                                                     |
| 2026-07-22 | Temporarily transport the ignore plan on `DocBuilder`.         | The field is private, immutable, install-once, and absent from `DocArena`; PR 05 must revisit it.                                                                                                          |
| 2026-07-22 | Scope PR 03 to production-factory physical projections.        | Runtime validation or a second proof model would add more architecture than deleting outer results.                                                                                                        |
| 2026-07-22 | Split PR 03 codegen policy by native versus WASM.              | Native accessor inlining preserves measured throughput; a narrow WASM field boundary reverses aggregate-return code duplication.                                                                           |
| 2026-07-22 | Let exceptional fragments own malformed lexical joins.         | Existing boundary atoms already make empty fragments inert, so Java/Kotlin pre-filter policies duplicate the same fact.                                                                                    |
| 2026-07-22 | Keep physical recovery parts in variable-list iteration.       | Real interior empty and tokenless malformed parts are positional sorting/normalization barriers; some zero-width recovery docs also own comment claims.                                                    |
| 2026-07-22 | Defer list visibility staging to a barrier-aware layout slice. | A global sparse projection lost trivia and segmented normalization, while a second recovery-run API would increase rather than reduce local reasoning.                                                     |
| 2026-07-22 | Make PR 05 root coordination a one-shot shared function.       | One lifecycle function can delete duplicated setup/audit code and narrow APIs without creating a persistent context or changing any leaf-rule signature.                                                   |
| 2026-07-22 | Defer the root EOF ignore-range leak.                          | Replacing two raw range vectors is worthwhile only when one ignore-owned capability deletes more plumbing than it adds; PR 05 does not yet prove that.                                                     |
| 2026-07-22 | Keep PR 06 reporting failure-oriented.                         | Exact selected-render identity accounting already owns normalization authority; operation-tagged failures delete weaker filename/count policy without a successful-fact vector or test-only formatter API. |
| 2026-07-22 | Trust syntax-issued normalization claims in corpus tests.      | Syntax accessors are the declared authority; test-side spelling counts duplicated policy, could bless the wrong equal-spelling token, and ignored output-only tokens.                                      |
| 2026-07-22 | Keep normalization permissions affine.                         | Retaining whole claims in copyable document nodes made synthesis authority reusable and grew production reporting machinery; selected claims instead retain only their existing operation data.            |
| 2026-07-22 | Keep dense source auditing debug-only.                         | Debug owns exact identity proof and actionable ranges; release suites own output parity, snapshots, idempotence, integration behavior, and performance without a second audit path.                        |
| 2026-07-22 | Keep the document, ignore, and render lifecycle in one crate.  | Their real source-claim, root-plan, and proof-observation cycle cannot be extracted without adding a context, side table, callback, or generic audit abstraction.                                          |
| 2026-07-22 | Make `jolt_formatter` the integration-facing facade.           | Its public sink function requires the sink/control contracts; CLI and dprint should not depend directly on the lower document IR, while language formatter crates remain implementation details.           |

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
