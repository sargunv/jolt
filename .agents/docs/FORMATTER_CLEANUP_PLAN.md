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
                                      └─ cleanup/09-kotlin-rules
                                          └─ cleanup/10-java-rules
                                              └─ cleanup/11-lexer-substrate
                                                  └─ cleanup/12-java-lookahead
                                                      └─ cleanup/13-java-comment-conservation
                                                          └─ cleanup/14-final-reconciliation
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

### PR 11 — Lexer-local state and bounded scanning

Accepted scope:

- delete the redundant `previous_end` field and helpers independently in both
  scanners; the value was always exactly the current byte offset;
- delete Java's one-use current-character/range pair helper;
- make Kotlin multi-dollar prefix classification end-to-end linear while
  preserving its existing token boundaries and diagnostics;
- retain language-owned UTF-8 movement, token classification, trivia,
  diagnostics, and Unicode semantics.

Expected deletions: duplicated position state and its hot-loop stores. The
bounded Kotlin scan may add one narrow cached boundary, but must not add generic
cursor policy or change malformed-token ownership.

Risks: Unicode/EOF range changes, hiding repeated scans behind a locally linear
helper, and adding state whose invalidation is hard to reason about.

Gates:

- Unicode, trivia, malformed-input, and full syntax fixtures pass for both
  languages with no snapshot delta;
- generated long valid and invalid dollar runs demonstrate linear end-to-end
  Kotlin scanning without a giant committed fixture;
- parse throughput, allocations, peak memory, native size, and WASM size do not
  regress;
- both scanners remain locally understandable and expose no new shared/public
  API.

Rejected shared-cursor prototype (2026-07-22): explicit composition produced a
+298/-298 Java diff, a +316/-260 Kotlin diff, and 107 lines of shared/export
surface: +163 production lines overall. Qualified cursor access replaced the
deleted mechanics with wrapping and noise at every token rule. Short field
names, implicit `Deref`, implementation macros, or a trait of callback-like
accessors could manufacture a smaller source diff but would make movement less
local and less explicit. The prototype compiled, preserved all Java corpus
snapshots, and passed the complete Kotlin syntax suite, so this is a design
rejection rather than a correctness failure.

Keep the duplicated two-field source cursors until Rust can express composition
without expanding their clients or a later design deletes more surrounding
scanner machinery.

Trivia sharing was also rejected. Kotlin has string-mode gating, shebangs,
nested block comments, and LF-only line-comment termination; Java has Unicode
pretranslation, final-SUB handling, non-nested block comments, and CR-or-LF
line-comment termination. A common trivia loop therefore needs semantic hooks or
policy flags and weakens the language boundary.

### PR 12 — Java lookahead audit and local deletion

Scope:

- measure adversarial work in the Java lookahead parallel grammar;
- shrink repeated productions and pass completed classifications directly into
  their consuming grammar where this needs no cache or extra ownership model;
- prototype bounded replacements for the proven superlinear paths, but keep them
  only when they remain locally understandable and allocation-neutral;
- delete parallel productions as soon as the owning parse decision migrates.

Expected deletions: repeated lookahead work and parallel grammar code.

Risks: changed ambiguity decisions, cache memory growth, or moving the same
grammar complexity behind a generic API.

Gates:

- counted prototypes identify the retained adversarial costs and prove any
  claimed improvement; PR 12 must not add a new unmeasured repeated scan;
- Java parser fixtures and recovery snapshots remain stable;
- realistic and adversarial parser time/allocation improve or remain neutral;
- the resulting grammar is shorter and locally traceable.

Audit findings and working scope (2026-07-23):

- in the PR 11 parent, nested ordinary parenthesized expressions are
  `Theta(n^2)`: every `(` scans to its matching close to reject a lambda, and
  cast classification repeats that scan. A depth-`D` generated family performs
  approximately `4D^2 + 6D` balanced-token advances. PR 12 deletes the cast
  pre-scan but retains the fundamental lambda-rejection scan;
- malformed top-level annotation suffixes are also `Theta(n^2)`: recovery asks
  package, module, and type predicates to rescan the remaining annotation run at
  successive `@` tokens;
- one nested generic type probe is linear in tokens but recursively uses input
  depth. The consuming type grammar is recursive too, so changing only the
  parallel scanner would not close the stack-depth risk;
- Java-private speculative-step counting at actual lookahead and balanced-scan
  advances measured the prototypes below. It was reverted with them because the
  accepted implementation does not claim a new asymptotic bound;
- the retained implementation deletes the second typed-lambda probe, cast lambda
  pre-scan, duplicate top/local type-declaration predicates, the
  resource-variable alias, module cursor replay, duplicated primitive/literal
  kind matches, and repeated pattern-prefix classification where the result is
  passed directly;
- do not fast-forward failed annotation runs. Unterminated annotation arguments
  can contain semicolon or later program boundaries that tokenwise recovery must
  preserve. Any annotation memo must reproduce suffix boundaries exactly; a
  safe-subset policy is rejected as a leaky second grammar;
- reject speculative parser transactions, Kotlin-style fixed token caps, generic
  query caches, and a member-classification framework unless a smaller local
  prototype preserves the exact existing precedence and recovery trees.

Rejected parenthesis-summary prototype (2026-07-23): a Java-private lazy cache
made the generated depth-1,600 ordinary-parenthesis family effectively linear
and reduced its debug parse time from about 0.37 seconds to 0.02 seconds. It
also added 80 production lines and 65 test lines, increased realistic Java parse
allocations from 109,539 to 113,644 (+4,105, +3.75%), and allocated 293,888 more
bytes. A flat/spill representation would add another state model. The cache was
fully reverted; retain the remaining repeated lambda rejection scan until its
grammar decision can be made without a material side structure.

Rejected annotation-start memo prototype (2026-07-23): remembering failed
top-level annotation starts plus their run end made flat malformed runs exactly
`16N` speculative advances, malformed import suffixes `8N`, and repeated
balanced top-level `@A(value=@B)` items `70N`. It did not bound a single deeply
nested annotation: depth 256 still performed 600,581 advances because every
interior `@` must run the ordinary boundary predicates, each of which scans the
remaining nested suffix. Broader memoization would introduce the generic cache
and invalidation model rejected by this PR; fast-forwarding is recovery-inexact
because malformed annotation arguments can contain later declaration boundaries,
as in `import a @A(value; class C {}`, `@A( class C {}`, and `@module 0`. The
prototype was fully reverted.

Rejected member-header classifier (2026-07-23): annotation elements, malformed
constructors, methods, fields, and compact members intentionally use different
precedence and recovery gates. With no rewindable lookahead, an exact classifier
must restart those same probes and merely hide them behind an enum, growing by
an estimated 0-15 lines without reducing work. Keep the decisions local until a
smaller owner-specific deletion is proven.

### PR 13 — Java declaration comment conservation

Scope:

- make each Java declaration rule that relocates construct-leading comments own
  them exactly once;
- suppress leading trivia only on the actual first header token after that
  relocation;
- cover annotated locals, generic callables, and annotated annotation elements
  with focused integration fixtures grounded in the reproduced Spring failures.

Expected result: no missing/duplicate comment claims when a declaration starts
with a modifier annotation, modifier token, or type-parameter list.

Risks: double-formatting comments on one declaration shape while repairing
another, changing ordinary comment placement, or adding a flag-heavy generic
header abstraction.

Gates:

- focused debug fixtures reproduce the current failures before the fix and pass
  after it without weakening conservation checks;
- Java corpus, recovery, trivia-conservation, idempotence, and release formatter
  suites pass;
- the complete Spring architecture benchmark succeeds in debug-audited dprint as
  well as the native release path;
- ownership stays declaration-local and the repair adds no parser or generic
  formatting context.

Audit trigger (2026-07-23): the final benchmark reproduced 199 missing or
duplicate trivia claims in Spring source, including a line comment before an
annotated local, a doc comment before a generic method, and a doc comment before
an annotated annotation element. The exact PR 12 WASM artifact reproduces the
same single-file failures, and the affected modifier/type-parameter code is
unchanged from `main`; this is a pre-stack correctness hole exposed by the final
gate, not a PR 12 regression.

### PR 14 — Final reconciliation, docs, and API deletions

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

Final reconciliation scope (2026-07-23):

- compare the PR 00-12 implementation stack from `main` commit `a82ab675` to PR
  12 commit `8437c8a`, then record PR 13's code-only delta separately; exclude
  PR 13 documentation from the implementation delta and do not manufacture a
  main-relative artifact comparison that was not measured;
- document the architecture that exists: language-owned typed layout and
  recovery decisions, one shared root lifetime, syntax-owned normalization,
  selected-render source conservation, root-scoped formatter-ignore planning,
  exceptional lexical joins, arena document topology, and bounded iterative fit
  probing;
- audit every remaining public/hidden facade item and delete only convenience or
  transition surface with no current caller. Do not rename retained APIs merely
  to make this final PR contain code;
- record intentional residue rather than reopening it here: modifier-prefix
  visibility inferred from builder emptiness, two language-owned root EOF
  ignore-range collections, Java quadratic lookahead and recursive generic
  depth, the debug audit traversal, physical recovery list states, and
  language-specific lexer cursor/trivia mechanics;
- keep deferred output-policy questions out of this structural stack: Kotlin
  zero-width import/program visibility and `when` recovery spacing, plus Java
  ordinary/ignored program joining around invisible entries.

## Residue Resolution Extension (2026-07-23)

The PR 14 audit proved that several intentionally recorded seams still exist.
Resolve them in new descendants of PR 14 rather than rewriting reviewed PRs.
Structural, behavior, and parser-complexity changes have separate rollback
boundaries:

```text
cleanup/14-final-reconciliation
  └─ cleanup/15-modifier-presence
      └─ cleanup/16-recovery-layout-parts
          └─ cleanup/17-ignore-boundary-ownership
              └─ cleanup/18-java-program-joining
                  └─ cleanup/19-kotlin-recovery-layout
                      └─ cleanup/20-kotlin-marker-recovery
                          └─ cleanup/21-java-delimiter-summaries
                              └─ cleanup/22-java-generic-depth
                                  └─ cleanup/23-residue-reconciliation
```

### PR 15 — Syntax-owned Java modifier presence

- replace semantic `ConcatBuilder::is_empty` decisions in Java modifier-prefix
  formatting with syntax-derived presence;
- preserve tokenless recovery proof documents without treating them as lexical
  layout content;
- delete the generic opaque annotation-document iterator and duplicate prefix
  state where the narrower representation permits it;
- preserve byte-for-byte output, source claims, and modifier comment ownership.

Reject any wrapper that merely mirrors builder emptiness or survives with one
hypothetical client. This PR closes the unfulfilled PR 01 -> PR 04 promise.

### PR 16 — Barrier-aware recovery layout parts

- introduce one narrow formatter-IR layout carrier that distinguishes visible
  content from claim-only recovery while preserving every physical syntax-list
  position;
- use it to replace Kotlin `Invisible(Doc)`/`layout_visible` and Java's parallel
  `(part, visible)` resolution only where separator behavior remains local;
- do not add a generic list visitor or move comma, sorting, normalization, or
  orphan-separator policy out of its language owner;
- preserve byte-for-byte output and exact source claims.

This PR is conditional on net deletion of branches/concepts and non-growth of
the touched production subsystem. If the shared carrier cannot meet that gate,
keep the physical recovery states and close the debt by documenting them as the
minimal barrier representation; do not publish a renamed boolean framework.

### PR 17 — Formatter-ignore boundary ownership

- let formatter-ignore runs answer semantically whether they own a control
  comment at the root boundary;
- keep the already-derived root runs alive through EOF formatting;
- delete both language-owned `Vec<Range<usize>>` projections and duplicated raw
  containment checks without rescanning tokens or source;
- preserve output and the existing linear discovery/query bounds.

### PR 18 — Java program joining behavior

- make invisible retained segments transparent to the preceding visible section
  state in ordinary and ignore-aware joining;
- preserve every formatter control marker exactly once across adjacent or
  separated root ignore runs;
- isolate and snapshot any intended whitespace correction;
- require source conservation and second-pass idempotence for focused fixtures.

### PR 19 — Kotlin recovery visibility and spacing

- derive import/program section visibility from represented syntax tokens or
  comments, never opaque `Doc` topology;
- make top-level missing and tokenless malformed pieces claim-only for joining;
- make malformed `when` keyword recovery use the same lexical spacing rule as a
  represented keyword when it has visible source;
- isolate all output changes in focused recovery snapshots and prove
  idempotence/source conservation.

### PR 20 — Kotlin parser marker recovery

- fix the parser panic discovered while isolating malformed `when` keyword
  spacing: `fun f(x: Any) = when(x) { is @ String -> 1 }` must return a
  represented recovered tree and diagnostics rather than abandoning a non-latest
  marker;
- keep the change in Kotlin parser marker/recovery ownership; do not teach the
  formatter to avoid the represented case or weaken the marker invariant;
- add the smallest parser fixture plus losslessness, following-syntax recovery,
  formatter non-panic, and native/WASM stack checks;
- once the recovered keyword field is reachable, make its formatter spacing
  syntax-visible: a token-owning malformed keyword gets the same following space
  as a represented keyword, while tokenless recovery remains claim-only; isolate
  the intended output in that exact recovery snapshot;
- preserve linear parsing and avoid a general marker rollback framework unless
  the exact cause proves one is required.

This layer was inserted after PR 19 probing exposed a distinct parser defect. It
remains separate so the formatter behavior PR has a narrow rollback boundary.

### PR 21 — Bounded Java delimiter and annotation summaries

- eliminate quadratic nested parenthesized-lambda rejection with a Java-local
  lazy delimiter summary that activates only after an explicitly counted scan
  budget;
- route annotation-argument skipping through the same exact parenthesis boundary
  summary, eliminating the deeply nested malformed-annotation rescan without
  introducing an independent annotation memo;
- preserve exact grammar classification and recovery; the budget may select an
  implementation path but must never cap accepted syntax or change a tree;
- prove total balanced-token work is `O(B * tokens)` for fixed documented `B`;
- require zero new allocation on the realistic corpus common path and reject a
  material side model or production growth without offsetting deletion.

### PR 22 — Bounded Java generic depth

- prototype removal of input-depth recursion from both generic-type lookahead
  and the consuming type grammar with an explicit iterative work stack;
- if that duplicates the type grammar or adds a large state machine, use one
  documented finite nesting limit shared by lookahead and consumption, then
  preserve the over-depth suffix in one syntax-owned malformed type with an
  owned diagnostic; never silently truncate or lose tokens;
- keep ownership local to type parsing and delete recursive helper state as the
  iterative path lands;
- prove linear token work, bounded native/WASM stack use, following-declaration
  recovery, fixture parity, and neutral realistic allocations.

### PR 23 — Residue reconciliation

- update formatter architecture and finite-cost documentation to match the
  implemented extension;
- record exact stack LOC/concept/performance deltas and every rejected
  prototype;
- remove transition-only APIs introduced by PRs 15-22;
- leave no item labeled deferred without either an implemented owner or an
  explicit minimality decision supported by the extension's measurements.

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

| PR  | Branch                                   | Status      | Parent | Draft PR                                       | Verification               | Notes                                            |
| --- | ---------------------------------------- | ----------- | ------ | ---------------------------------------------- | -------------------------- | ------------------------------------------------ |
| 00  | `cleanup/00-plan-and-gates`              | draft open  | `main` | [#2](https://github.com/sargunv/jolt/pull/2)   | baseline audit complete    | Durable plan and gates only.                     |
| 01  | `cleanup/01-doc-semantics`               | draft open  | PR 00  | [#3](https://github.com/sargunv/jolt/pull/3)   | debug/release + benchmark  | Profile-independent topology/presence.           |
| 02  | `cleanup/02-formatter-ignore-plan`       | draft open  | PR 01  | [#4](https://github.com/sargunv/jolt/pull/4)   | debug/release + benchmark  | Root plan with bounded immutable queries.        |
| 03  | `cleanup/03-infallible-generated-fields` | draft open  | PR 02  | [#5](https://github.com/sargunv/jolt/pull/5)   | debug/release + benchmark  | Generated physical slots only.                   |
| 04  | `cleanup/04-syntax-recovery-visibility`  | draft open  | PR 03  | [#6](https://github.com/sargunv/jolt/pull/6)   | full + release + benchmark | Syntax-owned malformed lexical boundaries.       |
| 05  | `cleanup/05-root-coordination`           | draft open  | PR 04  | [#7](https://github.com/sargunv/jolt/pull/7)   | full + release + benchmark | Narrow root ownership, no god context.           |
| 06  | `cleanup/06-source-audit-reporting`      | draft open  | PR 05  | [#8](https://github.com/sargunv/jolt/pull/8)   | full + release + benchmark | Syntax claims replace filename/count policy.     |
| 07  | `cleanup/07-core-module-boundaries`      | draft open  | PR 06  | [#9](https://github.com/sargunv/jolt/pull/9)   | full + release + benchmark | Kept one crate; narrowed lifecycle and APIs.     |
| 08a | `cleanup/08a-renderer-boundaries`        | draft open  | PR 07  | [#10](https://github.com/sargunv/jolt/pull/10) | full + release + benchmark | Kept hot loop concrete; deleted duplicate state. |
| 08b | `cleanup/08b-renderer-audit-pass`        | rejected    | PR 08a | —                                              | design audit complete      | Exact single pass requires an output trace.      |
| 09  | `cleanup/09-kotlin-rules`                | draft open  | PR 08a | [#11](https://github.com/sargunv/jolt/pull/11) | full + release + benchmark | Rule state and helper indirection deleted.       |
| 10  | `cleanup/10-java-rules`                  | draft open  | PR 09  | [#12](https://github.com/sargunv/jolt/pull/12) | full + release + benchmark | Total rules and native module parts.             |
| 11  | `cleanup/11-lexer-substrate`             | draft open  | PR 10  | [#13](https://github.com/sargunv/jolt/pull/13) | full + release + benchmark | Shared cursor rejected; local scans are bounded. |
| 12  | `cleanup/12-java-lookahead`              | draft open  | PR 11  | [#14](https://github.com/sargunv/jolt/pull/14) | full + release + benchmark | Local deletion; cache frameworks rejected.       |
| 13  | `cleanup/13-java-comment-conservation`   | draft open  | PR 12  | [#15](https://github.com/sargunv/jolt/pull/15) | full + release + benchmark | Localize Java comment and separator ownership.   |
| 14  | `cleanup/14-final-reconciliation`        | draft open  | PR 13  | [#16](https://github.com/sargunv/jolt/pull/16) | full + static checks       | Actual docs, metrics, and API deletions only.    |
| 15  | `cleanup/15-modifier-presence`           | draft open  | PR 14  | [#17](https://github.com/sargunv/jolt/pull/17) | full + benchmark           | Syntax-owned modifier layout presence.           |
| 16  | `cleanup/16-recovery-layout-parts`       | draft open  | PR 15  | [#18](https://github.com/sargunv/jolt/pull/18) | full + benchmark           | Barrier-aware visible/claim-only layout.         |
| 17  | `cleanup/17-ignore-boundary-ownership`   | draft open  | PR 16  | [#19](https://github.com/sargunv/jolt/pull/19) | full + benchmark           | Delete raw EOF ignore-range projections.         |
| 18  | `cleanup/18-java-program-joining`        | draft open  | PR 17  | [#20](https://github.com/sargunv/jolt/pull/20) | full + benchmark           | Reconcile root joining and marker ownership.     |
| 19  | `cleanup/19-kotlin-recovery-layout`      | in progress | PR 18  | —                                              | diagnosis                  | Isolate Kotlin recovery behavior corrections.    |
| 20  | `cleanup/20-kotlin-marker-recovery`      | planned     | PR 19  | —                                              | —                          | Fix malformed-when parser marker panic.          |
| 21  | `cleanup/21-java-delimiter-summaries`    | planned     | PR 20  | —                                              | —                          | Bound lambda and annotation parenthesis scans.   |
| 22  | `cleanup/22-java-generic-depth`          | planned     | PR 21  | —                                              | —                          | Bound recursive generic-type parsing.            |
| 23  | `cleanup/23-residue-reconciliation`      | planned     | PR 22  | —                                              | —                          | Final evidence and transition deletion.          |

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

### PR 08a evidence

- Three independent audits rejected renderer, fit, sink, error, and source-audit
  file extraction. Fit consumes the active render continuation, group modes,
  column, pending indentation, and horizontal-whitespace state. Moving it
  creates a back-edge into `Renderer` or a shared context while deleting no
  interpreter dispatch. Output delegation has the same problem because fit must
  inspect most of the proposed emitter state. The concrete hot loop therefore
  remains in one module.
- The retained renderer is smaller instead. `RenderOptions` and `IndentStyle`
  duplicated `FormatOptions`; `GroupFrame` duplicated `Mode`; and
  `measured_group_fits` duplicated the proof already carried by flat mode.
  Rendering begins broken and can enter flat mode only after a complete accepted
  probe, whose measured group rejects hard/empty lines, multiline text, forced
  groups, and budget exhaustion.
- The single-use render command scratch vector and its forwarding loop are
  deleted. Pending indentation is one count whose character derives from
  immutable options. Fit no longer tracks indentation depth it never reads, but
  its unit end-indent command preserves the exact 4,096-command budget. Fit
  scratch vectors, lazy concat cursors, continuation overlay, and measured-group
  boundary remain concrete and unchanged.
- The private unaudited renderer route, caller-asserted `source_verified` flag,
  missing-proof error, and hypothetical test are deleted. Production still has
  exactly the debug discard audit followed by trusted emission, while release
  remains single-pass. Debug errors are still output-atomic, late audit errors
  still precede sink halt, and sink callback/chunk behavior is unchanged.
- Malformed source claims now retain only their consumed `SourceRangeClaim`;
  unread kind/range copies are deleted. `FormatSinkResult::Blocked` carries the
  one fatal diagnostic produced by every caller instead of a vector, deleting an
  error-path allocation and dprint's impossible empty/multiple formatting cases.
- Rust source is +149/-296 lines (-147 net), including integration-test API
  migration. No new trait, context, visitor, module, callback, state object, or
  compatibility path was introduced.
- Repository-defined Ona automation passed all 183 workspace tests with no
  skips. One hypothetical private-renderer test was deleted with its impossible
  state. Debug IR passed 43 tests; release IR now passes its 31 applicable tests
  instead of compiling audit-only assertions into a profile without audit
  claims. `mise run fix`, `mise run check`, dependency machete, strict Clippy,
  WASM checks, all-features checks, both complete release formatter suites, the
  9,899-file PGO build, and optimized dprint build passed with zero snapshot or
  output delta.
- Parent and child syntax/document topology and all allocation counts, total
  bytes, and maximum-live bytes are identical on the 9,206-file Java and
  485-file Kotlin corpora. Alternating format-only medians moved +1.39% for Java
  and -3.12% for Kotlin; treat both as noise-level non-regression evidence.
  Clean workspace check time moved from 19.962 to 19.693 seconds (-1.35%).
- The non-PGO native CLI shrank from 5,948,344 to 5,943,952 bytes (-0.07%). The
  optimized WASM plugin shrank from 1,766,057 to 1,763,515 bytes (-0.14%), with
  SHA-256 `90f16905c8f2dff3ee97b58ef02f5fe5daf7acbb483489f7ee49d3da55db8b08`.
  Peak RSS remains unavailable because this environment lacks the benchmark's
  required `/usr/bin/time`; no test or other measurement was skipped.

### PR 08b rejection

- Debug formatting currently checks arena invariants, renders the complete
  selected layout into a zero-sized discard sink while consuming claims,
  finishes conservation, and checks malformed dispatch before the real sink
  receives its first callback. A late audit error therefore produces no partial
  output, and an early sink halt cannot hide an invalid tail claim.
- Three independent audits established the lower bound for preserving that
  contract: either replay the deterministic producer after validation, as the
  current implementation does, or retain the exact sink-callback trace until
  validation succeeds. Callback boundaries are observable because a sink may
  halt on any chunk.
- The smallest exact single-traversal prototype needs a `String` plus every
  chunk-end offset. It adds a type, two allocation streams, roughly 25-40 lines,
  `O(output bytes + callbacks)` live memory, and duplicate output buffering for
  CLI, dprint, and tests. It deletes only the zero-sized discard sink and a few
  call-site lines.
- A coalesced string changes halt/chunk behavior; borrowed chunk vectors add
  lifetime machinery; group-decision logs retain two traversals and add
  per-occurrence state; transactional sinks cannot roll back arbitrary side
  effects and expand the public contract; direct proof-to-sink permits partial
  output and lets halt hide later failures.
- PR 08b is therefore intentionally not opened. The current debug-only second
  traversal is allocation-free, explicitly bounded by the same finite fit
  budgets, and smaller than every behavior-equivalent replacement. PR 09 stacks
  directly on PR 08a.

### PR 09 working scope

- Begin with three disjoint local state reductions: remove unreachable member-
  chain builder states, unify the duplicated declaration assignment-body layout,
  and replace the impossible block-content option/boolean product with its two
  reachable states.
- Follow with Kotlin-local helper deletion: inline one-use or constant-policy
  layers where their owner remains clear, remove stale optional inputs, and
  centralize only byte-identical delimiter behavior with more than one client.
- Audit repeated control-flow field resolution, stale optional node parameters,
  and physical comma assembly only after the local slices pass. Keep a slice
  only when it reduces branches and concepts without obscuring malformed-token
  spacing or orphan-comma ownership.
- Preserve byte-for-byte output throughout PR 09. Potential fixes for zero-
  width import/program section visibility and when-condition recovery spacing
  are behavior changes and remain outside this structural PR.
- Do not introduce a generic list visitor, control-flow macro, shared class-
  header context, or Java/Kotlin rule abstraction. The apparent duplicates carry
  different separator, recovery, comment, and formatter-ignore policies.

### PR 09 evidence

- Ten production commits remove unreachable member-chain and block-body states,
  unify assignment-body layout, resolve seven control-flow fields once, remove
  three stale optional rule inputs, and delete one-use or constant-policy token,
  comment, delimiter, and member-body helpers. No context, visitor, macro,
  compatibility layer, public API, or Java/Kotlin abstraction was added.
- Kotlin production Rust is +201/-349 lines (-148 net). Including the durable
  plan updates, the branch is +241/-366 (-125 net). Formatter output and every
  snapshot are unchanged.
- Two prototypes were rejected and fully reverted. Merging malformed/invisible
  list variants retained the same boolean state and grew policy-sensitive
  matches. A generic physical-comma walker could delete 45 lines, but exact
  delegation recovery required independent comma-attachment and orphan-rendering
  policies, turning it into a two-dimensional mini-framework.
- Independent adversarial review found no correctness, recovery, trivia,
  source-claim, or caller-exhaustiveness regression. The repository-defined Ona
  task passed all 183 workspace tests with zero skips, including dprint smoke
  tests. `mise run fix`, strict Clippy, WASM checks, both complete release
  formatter suites, the 9,899-file PGO build, and the optimized dprint build
  passed.
- Paired parent/child architecture runs used the same 9,206-file Java and
  485-file Kotlin corpora. Java topology and allocation metrics are identical;
  its format median moved -4.47%. Kotlin format time moved -1.74%, allocation
  count 38,083 -> 38,076, allocation bytes 49,074,816 -> 49,068,984, and peak
  RSS 22,872,064 -> 22,523,904 bytes.
- Resolving each represented control-flow field once creates one independent
  space document for each ordinary `while` body rather than reusing the
  condition's handle: Kotlin document nodes are 461,999 -> 462,345 (+346,
  +0.075%). Child references fell by 816, reserved document memory fell by 2,356
  bytes, and maximum live allocation bytes remained identical. Special-casing
  this one handle would reintroduce state into the otherwise local helper, so
  the bounded topology cost is accepted.
- The non-PGO native CLI shrank from 5,943,952 to 5,942,576 bytes (-0.02%). The
  optimized WASM plugin grew from 1,763,515 to 1,764,334 bytes (+0.05%), with
  SHA-256 `4684101e45d8e02f9d7d793cdda545cad2211858238a07f778ab99c68956fe4f`.
  This deterministic 819-byte cost is accepted against the rule-state deletion;
  no runtime or allocation regression accompanied it.

### PR 10 working scope

- Begin with owner-local totality and wrapper deletion: remove stale optional
  list/node inputs, impossible expression re-casts, total `Option<BodyItem>` and
  `Option<FormattedMember>` results, duplicated recovery-field wrappers, and the
  one-client normalized/synthesized-token module.
- Purify member chains, binary chains, throws clauses, and module directives
  only where each field is classified once and the replacement deletes states or
  mirror representations. Preserve formatter-ignore category planning,
  sortable-run barriers, and every malformed/missing physical list part.
- Keep `BodyContent`'s absent, present-invisible, and visible states. Keep the
  standard-member-body macro unless a concrete replacement shrinks its three
  syntax-specific expansions without a trait, context, or dynamic role cast.
- Preserve byte-for-byte output. Program-section joining has a policy difference
  around invisible entries between ignored runs; treat any reconciliation as a
  separate behavior change, not structural cleanup.
- Do not generalize Java/Kotlin lists, imports, body pipelines, delimiters, or
  operator sequences. Their recovery, normalization, comment, and separator
  policies are materially different.

### PR 10 evidence

- Eleven production commits remove stale optional rule inputs, impossible
  member/binary chain states, always-present body/member results, duplicated
  field recovery, one-use token/comment/list helpers, and the module directive
  mirror. Java production Rust is +482/-727 lines (-245 net).
- Module formatting consumes native `JavaSyntaxListPart` values. The common
  no-ignore path streams two fresh bounded iterators instead of allocating a
  mirror plus ignore-index staging; the ignore path retains one-to-one syntax
  indices. Missing, malformed, separator, comment, and non-sortable nodes remain
  explicit sorting and normalization barriers.
- Three independent adversarial reviews found no correctness, recovery, trivia,
  source-claim, ordering, topology, or bounded-work regression. All removed
  options were always `Some`; member and binary chain fallbacks were
  unreachable; and the narrowed type-clause match is exhaustive after its
  present/present arm.
- The repository-defined Ona task passed all 183 tests with zero skips.
  `mise
  run fix`, strict production Clippy, dependency and WASM checks, both
  complete release formatter suites, the 9,899-file PGO build, and the optimized
  dprint build passed with no output or snapshot delta. The known all-target
  Clippy warning remains the pre-existing oversized Java imported-fixture test.
- Against PR 09 on the same 9,206-file Java corpus, format median moved
  1,609.294 -> 1,597.447 ms (-0.74%), document nodes 20,930,870 -> 20,928,024,
  allocation count 1,502,952 -> 1,502,824, allocation bytes 1,984,578,290 ->
  1,984,384,210, and peak RSS 720,424,960 -> 720,302,080 bytes. Kotlin topology
  and allocation metrics are identical; its timing movement is noise-level.
- The non-PGO native CLI shrank from 5,942,576 to 5,923,408 bytes (-0.32%). The
  optimized WASM plugin shrank from 1,764,334 to 1,759,618 bytes (-0.27%), with
  SHA-256 `f9131f96fe1cc5c8d90ab1a4c093f01bd61421975138daf24d037f5098599307`.

### PR 11 evidence

- Java and Kotlin no longer store a duplicate previous-end offset or write it
  after every scalar. Java also deletes its one-use current-character/range
  pair. Production Rust is +46/-73 lines (-27 net); the boundedness regression
  adds 26 test lines, leaving all Rust at +72/-73 (-1 net).
- Kotlin classifies a valid multi-dollar string prefix once. A failed maximal
  dollar run records only its exclusive end, so its first token scans the run
  and every suffix token rejects in constant time while preserving the existing
  one-Unknown/one-diagnostic-per-dollar recovery contract. The cached word
  replaces Kotlin's removed previous-end word; Java's scanner loses one word.
- The 65,536-dollar valid/invalid stress test exercises actual tokenization and
  finishes in about 0.12 seconds in debug. The uncached byte-scan prototype
  would perform 2,147,516,416 predicate visits; the original indexed-character
  loop would perform roughly 46.9 trillion character visits.
- Three adversarial reviews found no range, Unicode remapping, EOF, trivia,
  string-mode, token-boundary, cache-invalidation, or dependency-boundary issue.
  They did find and close the end-to-end malformed-dollar complexity hole. The
  shared cursor prototype was rejected because explicit composition grew
  production by 163 lines; trivia sharing was rejected because it required
  language-semantic hooks or flags.
- The repository-defined Ona task passed all 184 tests with zero skips after the
  final fix. `mise run fix`, strict workspace Clippy, dependency and WASM
  checks, complete Java/Kotlin release syntax and formatter suites, the
  9,899-file PGO build, and the optimized dprint build passed with no output or
  snapshot delta.
- Against PR 10 on the same machine and corpora, Java parse median moved
  1,040.179 -> 1,020.168 ms (-1.92%) and Kotlin parse moved 27.665 -> 27.490 ms
  (-0.64%). Every parse/format/end-to-end allocation count and byte total is
  identical. Parse peak RSS moved 64,278,528 -> 64,253,952 bytes for Java and
  4,530,176 -> 4,644,864 bytes for Kotlin; the latter 112 KiB increase is below
  process/page noise and Kotlin scanner size is unchanged.
- The non-PGO native CLI shrank from 5,923,408 to 5,919,472 bytes (-0.07%). The
  optimized WASM plugin shrank from 1,759,618 to 1,758,605 bytes (-0.06%), with
  SHA-256 `88e74840fd1cc89bfe575d86281026ec8471ae58f7231f6945672da4cbbe525a`.

### PR 12 evidence

- Duplicate typed-lambda and cast probes, top/local type-declaration predicates,
  the resource-variable alias, module cursor replay, five wrapper-only offset
  helpers, and repeated primitive/literal token taxonomies are deleted. One
  private `PatternStart` decision replaces up to three pattern-prefix probes and
  is passed directly into its consuming grammar.
- Production Rust is +112/-220 lines (-108 net). Tests add no source, state, or
  fixture. Including the durable audit and decision ledger, the whole PR is
  +214/-225 lines (-11 net). No public API, CST schema, parser cache, or
  compatibility path is added.
- Measured parenthesis-summary, annotation-start-memo, and member-header
  classifier prototypes were rejected and fully reverted. The first added 80
  production lines and 3.75% realistic parse allocations; the second did not
  bound a single deeply nested annotation; the third had to restart the same
  precedence-sensitive probes it purported to classify.
- The retained grammar still has known `Theta(n^2)` nested ordinary-parenthesis
  lambda rejection and malformed nested-annotation recovery. Generic-type
  lookahead and its consuming grammar both retain recursive input depth. PR 12
  records these costs explicitly rather than hiding them behind a generic cache
  or claiming an asymptotic improvement it did not achieve.
- Repository-defined Ona automation passed all 184 workspace tests with zero
  skips. `mise run fix`, complete Java syntax/formatter release suites, strict
  workspace Clippy, dependency and WASM checks, the architecture benchmark, and
  the 9,899-file PGO build passed with no output or snapshot delta.
- Against PR 11 on the same machine and 9,206-file Java corpus, realistic parse
  median moved 1,020.168 to 1,033.988 ms (+1.35%, treated as run noise). Parse
  allocations are exactly unchanged at 109,539 and 1,288,505,194 bytes; peak RSS
  moved 64,253,952 to 64,303,104 bytes (+48 KiB). Syntax/document structure is
  unchanged.
- The non-PGO native CLI shrank from 5,919,472 to 5,917,176 bytes (-0.04%). The
  optimized WASM plugin shrank from 1,758,605 to 1,757,814 bytes (-0.04%), with
  SHA-256 `32fa2028a827adc33b606ac8a08c116c8951e91cae4189697a01c1d225832b16`.

### PR 13 evidence

- Java modifier prefixes now own their own leading boundary. Recovery-free
  prefixes relocate exactly the source-first structured comments under their
  existing reorder authorization; recovery-bearing prefixes stay in source order
  and preserve malformed trivia naturally. Declaration callers no longer thread
  construct-first identity or duplicate that ownership decision.
- Intermediate annotations and modifiers preserve their own comments, and a
  structured modifier with a trailing line comment forces the next entry or
  declaration header onto a hard line. Focused valid, malformed, type-use,
  `non-sealed`, generic callable, enum, pattern, local, resource, field, method,
  constructor, parameter, and type fixtures cover the boundary.
- The required debug Spring audit also exposed two older idempotence defects.
  Formatter-ignore splices now honor their explicit single-line separator after
  raw content, and Java syntax normalization withholds trailing-comma synthesis
  when a line comment would make the comma invisible. Both fixes stay at the
  existing ownership boundary and have focused integration fixtures.
- Production Rust is +312/-277 lines (+35 net): formatter source is +297/-270
  (+27), and Java syntax normalization is +15/-7 (+8). The rejected generalized
  first-token/recovery framework would have grown this slice by roughly 191
  lines before the two audit repairs and was fully reverted.
- The whole stack now contains 60,269 Rust lines, down 709 from the 60,978-line
  baseline. Java and Kotlin formatter source contains 21,165 lines, down 796
  from 21,961. The optimized WASM plugin is 1,756,843 bytes, 971 bytes smaller
  than PR 12 (-0.06%) and 5.69% smaller than the earliest exact PR 02 anchor;
  SHA-256 is `fe9faffba9a90de923a558ca5835c988f7e9c3a51feb0be8318e684e8c67dcbf`.
- Repository-defined Ona automation passed all 184 tests with zero skips.
  `mise run fix`, complete Java debug and release suites, strict workspace
  Clippy, dependency and WASM checks, the architecture benchmark, a fresh
  9,206-file debug-WASM Spring conservation/idempotence audit, and the
  9,899-file PGO build all passed.
- Against PR 12 on the same 9,206-file corpus, Java parse median is effectively
  unchanged at 1,033.988 -> 1,034.121 ms (+0.01%). Parse allocation count and
  bytes are exactly unchanged at 109,539 and 1,288,505,194. The final Java
  format median is 1,571.916 ms, end-to-end median is 2,753.066 ms, and native
  and dprint whole-corpus medians are 1,963.889 and 4,136.840 ms.

### Whole-stack reconciliation through PR 14

The implementation comparison is `main` commit `a82ab675` through this final
reconciliation branch, a 74-commit stack. Durable planning and architecture
documentation are excluded from production source metrics.

| Measure                        |   Main |  PR 14 |         Delta |
| ------------------------------ | -----: | -----: | ------------: |
| All Rust                       | 60,978 | 60,262 | -716 (-1.17%) |
| Rust under crate `src/` trees  | 57,887 | 57,297 | -590 (-1.02%) |
| Java + Kotlin formatter source | 21,961 | 21,165 | -796 (-3.62%) |
| Java formatter source          | 13,315 | 12,908 | -407 (-3.06%) |
| Kotlin formatter source        |  8,646 |  8,257 | -389 (-4.50%) |

The central `jolt_fmt_ir` source grows by 447 lines because it now owns root
ignore planning, exact source-conservation failures, and the shared run
lifecycle. That growth deletes outer policy and machinery: Java/Kotlin formatter
source falls by 796 lines, `jolt_test_support` falls by 208, the two language
syntax sources fall by a net 116, and four language helper files disappear.

Concepts and parallel authorities removed across the stack include:

- ten semantic `Doc == nil` decisions, eleven nested ignore-discovery paths, 28
  generated invariant-forwarding branches, `MalformedBoundaryPolicy`, and
  duplicated Java/Kotlin root lifecycles;
- token spelling/count inventories, two loss reporters, duplicate conservation
  builders, 17 Java filename policies, unlimited recovery exceptions, and the
  exceptional normalization/join path;
- 18 compiler-reachable `jolt_fmt_ir` root names, four Java/Kotlin formatter
  root exports, two `NormalizedToken` variants, and three direct integration
  dependencies on the IR crate;
- duplicate renderer state and scratch, vector-valued fatal diagnostics,
  impossible Kotlin/Java rule states, Java's `DirectiveEntry` mirror, duplicate
  lexer end-position state, repeated Java lookahead probes/classifiers, and
  declaration-level modifier comment ownership.

The only exact multi-PR artifact anchor is PR 02: optimized WASM shrinks from
1,862,892 bytes there to 1,756,843 at PR 13 (-106,049, -5.69%). No main artifact
was recorded, so this ledger does not invent a main-relative binary delta.
Likewise, noisy adjacent timing medians are treated only as repeated
non-regression evidence. The material topology costs remain the explicitly
accepted PR 01 profile-independent nodes and PR 09's 346 Kotlin nodes; later
slices remove Java nodes and allocations or leave topology unchanged.

### PR 14 evidence

- The unused `Language::from_extension` convenience seam is deleted; its only
  behavior remains directly in the sole `from_path` client. Production Rust is
  +1/-8 lines (-7 net), with no replacement API or compatibility layer.
- `docs/internals/formatter.md` now describes the actual run coordinator, syntax
  and rule boundary, source-conservation proof, normalization authority,
  malformed verbatim cores, root formatter-ignore plan, exceptional lexical
  joins, arena topology, and bounded fit algorithm. It does not describe a
  desired architecture as though it already existed.
- The final reconciliation records exact main-to-stack source deltas and the
  earliest honest artifact anchor without inventing a main-relative binary or
  treating noisy timing medians as improvements.
- `mise run fix` passed strict workspace formatting, Clippy, dependency, native,
  and WASM checks. Repository-defined Ona automation passed all 184 tests with
  zero skips on the final branch. PR 13 already supplies the immediately prior
  release, benchmark, debug-WASM Spring, optimized WASM, and PGO evidence; PR 14
  changes no formatting or rendering behavior.

### PR 15 evidence

- Java modifier and annotation layout now derives visibility from represented
  tokens rather than `ConcatBuilder::is_empty`, collection emptiness, or
  physical first/last positions. Claim-only missing and tokenless malformed
  documents remain emitted in source order and remain sorting barriers without
  selecting separators, terminal layout, or leading-comment ownership.
- One `VisibleDoc` carrier is used only where a preformatted annotation or
  ellipsis must retain syntax visibility. Inline and ordinary modifier joining
  share one state machine; the generic annotation-document iterator, duplicate
  modifier loop, and duplicate first-visible traversal are deleted.
- Adversarial review found and closed analogous source-order, trailing
  annotation, type-use, receiver, varargs, and ellipsis presence leaks before
  publication. Visible syntax retains the existing output; tokenless recovery no
  longer injects or suppresses lexical spaces or line breaks.
- Java formatter production Rust is +314/-255 lines (+59 net). The explicit
  state closes the unfulfilled PR 01 promise and makes tokenless recovery
  behavior independent of document topology; no public API or cross-language
  framework was added.
- The first consolidated implementation added one concat node per visible
  prefix: +24,144 Java document nodes, +15 allocations, and +2,832,944 allocated
  bytes. That result was rejected before publication. Terminal separators now
  emit inside the existing concat. Against the pre-PR parent, the final Java
  corpus has five fewer document nodes, identical child count, reserved memory,
  allocation count, and allocation bytes; format median moved -0.38% and peak
  RSS -139,264 bytes, both treated as neutral.
- `mise run fix` passed workspace formatting, Clippy, dependency, native, and
  WASM checks. All 184 repository tests passed with zero skips, including both
  formatter corpora, recovery snapshots, imported-fixture idempotence/source
  conservation, layout, trivia, CLI, and dprint integration tests. Formatter
  snapshots are unchanged.

### PR 16 evidence

- `LayoutDoc::{Visible, ClaimOnly}` and generic `FormatListPart` replace Java's
  language-local list-part enum and `(part, visible)` result plus Kotlin's
  parallel malformed/invisible variants. Item and separator visibility remains
  explicit at Java call sites; comma attachment, sorting, normalization, and
  orphan-separator behavior remain language-owned.
- Kotlin `CommaListItem` now owns the same layout contribution plus its optional
  comma. Its duplicate `layout_visible` field and `push_recovery_item` helper
  are deleted. Claim-only documents remain emitted in physical order but do not
  become comma targets, affect visible counts, or select delimiter layout.
- Formatter production Rust is +306/-362 lines (-56 net). No tests or snapshots
  changed. Independent call-site and adversarial reviews found every recovery
  document still emitted and every former barrier, grouping, and separator
  policy preserved.
- Representative native sizes are neutral: `Doc` is 4 bytes, `LayoutDoc` is 8,
  the old and new Java list-part representations are both 40 bytes, and the old
  and new Kotlin comma-item representations are both 40 bytes.
- On both realistic corpora, document nodes, children, reserved bytes,
  allocation count, and allocation bytes are exactly unchanged. Format peak RSS
  moved -16,384 bytes for Java and +176,128 bytes for Kotlin. Timing and
  whole-CLI samples moved inconsistently within a noisy run and are treated as
  neutral. Optimized WASM shrank from 1,755,396 to 1,751,838 bytes (-3,558,
  -0.20%).
- `mise run fix` passed strict workspace formatting, Clippy, dependency, native,
  and WASM checks. All 184 repository tests passed with zero skips, including
  both corpora, imported-fixture idempotence/conservation, recovery, layout,
  trivia, CLI, and dprint integration tests. The architecture benchmark report
  records committed subject `0a3543a`; formatter snapshots are unchanged.

### PR 17 evidence

- Root-derived `FormatterIgnoreRun`s now remain alive through Java and Kotlin
  EOF formatting. Their opaque boundary-comment query replaces the public raw
  on-marker range projection, two `Vec<Range<usize>>` copies, two duplicated
  containment policies, and Kotlin's mixed document-plus-metadata result.
- The first prototype scanned every run for every EOF comment. Adversarial
  review rejected its `O(runs * comments)` cost. The final query uses the
  ordered run starts and `partition_point` to select the only possible owner in
  `O(log runs)` per comment; a counted 1,024-run midpoint test requires at most
  11 comparisons.
- Boundary ownership remains byte-for-byte equivalent: only a selected run that
  owns its closing on marker can suppress the fully contained boundary comment.
  Adjacent runs, terminal runs, half-open equality, and non-owning candidates
  have focused assertions. Formatter snapshots remain unchanged.
- Production Rust across formatter IR and the two roots is +48/-51 lines (-3
  net). The new bounded-work test adds 36 lines while replacing four weaker
  projection assertions.
- Realistic Java and Kotlin document topology, allocation counts, and allocation
  bytes are exactly unchanged. Format peak RSS moved -81,920 bytes for Java and
  +77,824 bytes for Kotlin; timing moved -2.45% and +0.10% respectively and is
  treated as neutral. Optimized WASM shrank from 1,751,838 to 1,750,965 bytes
  (-873, -0.05%).
- `mise run fix` passed workspace formatting, Clippy, dependency, native, and
  WASM checks. All 185 repository tests passed with zero skips, including both
  formatter corpora, recovery, imported-fixture idempotence/conservation,
  trivia, CLI, and dprint integration. The benchmark records committed subject
  `e97b098`; snapshots are unchanged.

### PR 18 evidence

- The exact root sequence ignored class / removed semicolon / ignored class
  dropped the first `@formatter:on` and inserted a phantom blank line. The first
  run correctly left its marker to the physical semicolon owner, but removed
  token formatting claimed control comments to nil and the invisible retained
  segment reset the outer ignored-state boolean.
- Removed tokens now format every represented comment, including formatter
  controls. A skipped physical owner is never formatted, so a marker included in
  a raw ignore run cannot be emitted twice. The valid Java/Kotlin corpus harness
  now compares canonical represented-comment inventories before and after
  formatting, closing the test gap that allowed a claim-only deletion to pass.
- One Java-local `ProgramSection { doc, visible, compact_after }` stream and one
  joiner replace the two tuple meanings, the retained-segment visibility scan,
  and the duplicate ignore-aware join loop. Missing and tokenless malformed
  sections emit claims without changing prior visible state; token-owning
  recovery, comment-only removed declarations, imports, and ignored runs carry
  explicit layout semantics.
- Java formatter production is +89/-97 lines (-8 net); the shared conservation
  assertion adds six test-support lines. The new irregular fixture proves both
  ignored classes remain byte-preserved, the semicolon disappears, all four
  markers occur once, the adjacent runs stay compact, and the following class is
  structured. It is idempotent and source/comment conserving. No existing
  formatter snapshot changed.
- Realistic Java and Kotlin document topology, allocation count, and allocation
  bytes are exactly unchanged. Format timing moved -1.65% and +0.03%; format
  peak RSS moved +364,544 and -155,648 bytes respectively, all treated as
  neutral. Optimized WASM shrank from 1,750,965 to 1,746,572 bytes (-4,393,
  -0.25%).
- `mise run fix` passed strict formatting, workspace Clippy, dependency, native,
  and WASM checks. Snapshot update and the subsequent non-update run both passed
  all 185 tests with zero skips. The benchmark records committed subject
  `9d142fd`; only the new intended Java syntax/formatter snapshots were added.

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
| 2026-07-22 | Keep render, fit, and output state in one concrete hot loop.   | Fit consumes the active command continuation and output-position state; extracting it adds a back-edge or shared context while deleting no interpreter dispatch.                                           |
| 2026-07-22 | Treat flat mode as the successful-fit proof.                   | Rendering starts broken and enters flat mode only after a complete accepted probe; a second mutable flag duplicated that state and could not diverge on a reachable flat path.                             |
| 2026-07-22 | Carry exactly one diagnostic when formatting is blocked.       | All five producers create one fatal diagnostic; a vector added an allocation and forced consumers to handle impossible empty/multiple cases.                                                               |
| 2026-07-22 | Reject single-pass debug rendering without an output trace.    | Atomic late failures plus observable sink chunk/halt behavior require either deterministic replay or retaining every callback; replay is zero-allocation and smaller.                                      |
| 2026-07-22 | Purify Kotlin rules through bounded owner-local slices.        | Member chains, assignment bodies, blocks, control flow, and helper layers have provably unreachable or duplicated state; broad rule frameworks would obscure distinct recovery and trivia contracts.       |
| 2026-07-22 | Keep PR 09 structural and byte-for-byte output preserving.     | Zero-width section visibility and when-condition recovery spacing may be bugs, but mixing style corrections with state deletion would weaken review and rollback boundaries.                               |
| 2026-07-22 | Reject a boolean-valued Kotlin recovery-list merger.           | It preserved the same two layout states, grew policy-sensitive matches, and reduced no behavioral concept.                                                                                                 |
| 2026-07-22 | Keep distinct physical comma assembly loops.                   | Delegation and destructuring differ in both attachment search and orphan rendering; exact sharing requires a two-policy mini-framework that weakens local reasoning.                                       |
| 2026-07-22 | Accept one extra document node per ordinary Kotlin `while`.    | Resolving each field once deletes seven repeated probes and 39 lines; the measured +0.075% node cost does not increase allocation count, reserved memory, peak RSS, or format time.                        |
| 2026-07-22 | Keep Java borrow-order macros with multiple clients.           | The concat/group/indent macros prevent repeated mutable-borrow temporaries at hundreds of sites; wholesale replacement would grow rules. Only the one-use `if_break` macro was deleted.                    |
| 2026-07-22 | Use native syntax parts for Java module directives.            | The mirror copied all four physical variants and forced allocation; native parts preserve ignore indices, recovery barriers, and reorder ownership while shrinking the owner.                              |
| 2026-07-22 | Keep Java program join policies separate.                      | Ordinary and ignored section joining differ around invisible entries between ignored runs; reconciling them may change output and does not belong in structural PR 10.                                     |
| 2026-07-22 | Reject the shared lexer cursor prototype.                      | Explicit composition grew production by 163 lines and made every token rule noisier; traits, macros, implicit dereferencing, or cryptic names only hide that cost.                                         |
| 2026-07-22 | Cache only the end of failed Kotlin dollar runs.               | A locally linear prefix helper still rescanned malformed suffixes quadratically; one forward-only boundary preserves token ownership with one scan per maximal run.                                        |
| 2026-07-23 | Reject the Java parenthesis-summary cache.                     | It fixes quadratic lambda rejection, but adds 80 production lines and 3.75% realistic parse allocations; a second representation would further weaken local reasoning.                                     |
| 2026-07-23 | Reject the top-level annotation-start memo.                    | It makes flat and repeated top-level runs linear but cannot bound one deeply nested annotation without broadening into a generic cache or changing exact recovery boundaries.                              |
| 2026-07-23 | Reject a general Java member-header classifier.                | Exact declaration precedence still requires independent restarts, so the enum would hide rather than remove repeated grammar work and would not shrink the parser.                                         |
| 2026-07-23 | Extend the stack rather than rewrite PRs 01-14.                | Residue now crosses structural layout, output policy, and parser cost models; new descendants preserve reviewed rollback boundaries.                                                                       |
| 2026-07-23 | Make modifier layout presence syntax-owned end to end.         | A preformatted claim document cannot reveal whether it owns visible syntax; one narrow carrier closes builder, collection, first/last, varargs, and ellipsis presence leaks.                               |
| 2026-07-23 | Share one thresholded parenthesis summary in PR 21.            | Lambda rejection and nested annotation recovery repeat the same balanced-parenthesis scan; one dormant Java-local summary can bound both without an independent annotation memo.                           |
| 2026-07-23 | Share resolved recovery layout contribution in formatter IR.   | One visible/claim-only carrier deletes both language-local list states and Kotlin's duplicate comma-item boolean while leaving all joining and separator policy with each language.                        |
| 2026-07-23 | Isolate Kotlin marker abandonment in a new PR 20.              | PR 19 formatter probing exposed a parser panic on malformed `when` syntax; parser marker ownership needs its own recovery fixture and rollback boundary before Java complexity work.                       |

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
