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
roughly 108 `block_on_invariant` calls, 153 malformed-field branches, and 84
malformed or invisible list branches. Shape validation belongs at construction
or typed-root conversion; downstream access should express field cardinality,
not factory implementation uncertainty.

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

Names are provisional until extraction proves useful:

```text
jolt_doc
  Pure document algebra, width model, fit engine, and output sinks.

jolt_syntax + jolt_{java,kotlin}_syntax
  Validated lossless CST, source identities, recovery classification,
  cardinality-aware accessors, and language-owned normalization authority.

jolt_fmt_core
  Formatting context, source audit, recovery fragments, shared trivia,
  lexical boundaries, and one root formatter-ignore plan.

jolt_{java,kotlin}_fmt
  Language CST to structured layout only.

jolt_formatter
  Thin dispatch facade used by the CLI and dprint plugin.
```

Do not create `jolt_doc` or `jolt_fmt_core` up front. First create clean module
boundaries inside the existing crate. Extract a crate only when dependency
direction is stable and the extraction removes coupling or compile surface.
Keeping one well-partitioned crate is preferable to several crates with cyclic
conceptual ownership.

A likely context shape is:

```rust
struct FormatContext<'source, L> {
    docs: DocBuilder<'source>,
    audit: SourceAudit<'source>,
    ignores: FormatterIgnorePlan<'source>,
    style: L::FormatStyle,
}

struct Formatted<'source> {
    doc: Doc<'source>,
    visibility: Visibility,
    boundaries: Boundaries<'source>,
}
```

These types are design sketches, not mandates. Introduce only the fields that
replace current plumbing. Rules must not infer visibility or lexical boundaries
from opaque document handles.

## Stack

Every pull request is a draft until the entire dependent slice has passed its
gates. Branches are stacked in this order:

```text
main
  └─ cleanup/00-plan-and-gates
      └─ cleanup/01-doc-semantics
          └─ cleanup/02-formatter-ignore-plan
              └─ cleanup/03-validated-cst-fields
                  └─ cleanup/04-format-context
                      └─ cleanup/05-pure-doc-core
                          └─ cleanup/06-renderer
                              └─ cleanup/07-language-rules
                                  └─ cleanup/08-syntax-tooling
                                      └─ cleanup/09-tests-docs-api
```

The plan is deliberately ambitious, but the stack is not immutable. Merge
adjacent entries when one cannot deliver an independently coherent deletion.
Split an entry when review would require holding too many invariants in mind.
Update both the graph and ledger before changing stack shape.

### PR 00 — Plan and gates

Scope:

- commit this plan and status ledger;
- make debug/release output parity reproducible without committing duplicate
  fixture output;
- add or document bounded deep-nesting and complexity checks needed by later
  structural changes;
- capture stable baseline measurement commands.

Expected simplification: none in production code. This PR creates only the
minimum safety harness needed to delete production machinery confidently. If the
harness becomes a framework, reduce it.

Gates:

- current focused formatter suites pass;
- all committed fixtures have debug/release output parity;
- complexity guard fails loudly when its fixture/tooling is missing;
- baseline commands and prerequisites are reproducible.

### PR 01 — Profile-independent document semantics

Scope:

- make every `Doc` operation produce the same topology in debug and release;
- remove semantic `Doc == nil` and equivalent emptiness inference;
- introduce explicit visibility metadata only at call sites that need it;
- keep debug normalization/source claims observational rather than structural.

Expected deletions: profile-conditioned document construction, opaque handle
comparisons, Kotlin `Invisible(Doc)`/`layout_visible` special cases where an
explicit result makes them redundant.

Risks: normalization audit coverage, group-fit decisions, comment-only nodes.

Gates:

- IR topology test is identical across debug and release;
- all fixture output remains byte-identical;
- normalization/source-conservation failures remain actionable in debug;
- document-node density does not regress materially.

### PR 02 — One formatter-ignore plan

Scope:

- discover and validate ignore directives once at the formatted root;
- associate source ranges with syntax-owned malformed/verbatim fragments;
- consume the plan monotonically while formatting ordered children;
- consolidate duplicate sequence-splicing paths;
- delete nested subtree token/source scans.

Expected deletions: eleven overlapping discovery paths, subtree token collection
used only to rediscover source ranges, duplicated Java/Kotlin splice logic.

Risks: nested containers, directives in malformed syntax, boundary comments,
disabled regions at the beginning/end of a file.

Gates:

- prove the planning pass and consumption are each linear in source/tokens;
- stress deeply nested ignored regions with an explicit time/work bound;
- source conservation and trivia tests pass;
- existing ignore snapshots remain byte-identical.

### PR 03 — Validated, cardinality-aware CST fields

Scope:

- validate generated factory shape once at construction or typed-root entry;
- model required, optional, and repeated fields explicitly;
- keep recovery pieces representable and syntax-owned;
- migrate formatters away from `SyntaxInvariantError` plumbing;
- delete `block_on_invariant` and impossible malformed-field branches as their
  callers migrate.

Do this as vertical slices through representative Java and Kotlin nodes before
changing the generator globally. The API is successful only if call sites get
smaller and retain full malformed-tree behavior.

Expected deletions: most of the 108 invariant unwraps, 153 malformed-field
branches, and 84 malformed/invisible list branches; redundant recovery policy
selection that becomes a CST property.

Risks: confusing parser recovery with invalid factory shape, hiding genuinely
optional tokens, generated-code churn, degraded diagnostics.

Gates:

- factories reject or classify invalid shapes at the chosen boundary;
- all representable malformed trees remain formattable and lossless;
- accessor code and formatter call sites shrink in representative slices;
- compile time and generated code size do not regress materially.

### PR 04 — A small formatting context

Scope:

- introduce a context that owns only currently shared formatting state;
- consolidate Java/Kotlin entrypoints and common root mechanics;
- centralize source-audit, ignore-plan, trivia, recovery-fragment, and lexical
  boundary services behind narrow methods;
- delete parameter threading and duplicate helpers as each service moves;
- make recovery policy a syntax contract, not a language-rule choice.

Expected deletions: duplicate root setup/teardown, per-rule audit plumbing,
Java/Kotlin comment and recovery mechanics, placement-mode choices that can be
derived from token ownership.

Risks: a god context, hidden mutable state, lifetime complexity, making pure
rules harder to test.

Gates:

- the context has a short field list with one owner per field;
- leaf rules remain pure-looking and locally testable;
- moved helpers are deleted, not wrapped indefinitely;
- no new source/token/node cloning or unbounded search;
- formatter rule LOC and call-site argument count decrease.

### PR 05 — Purify the document core

Scope:

- establish module boundaries between pure documents/rendering and syntax-aware
  formatting mechanics;
- reverse any dependency from document code into syntax or language concerns;
- extract `jolt_doc` and/or rename the residual core only if the internal
  boundary already proves the dependency win;
- preserve the `jolt_fmt_ir` surface temporarily only when a current client
  requires it, then remove the facade in the same stack.

Expected deletions: cross-layer re-exports, syntax-aware document helpers,
options/results mixed with algebra, obsolete facade modules.

Risks: crate churn without conceptual simplification, public API expansion,
longer compile times.

Gates:

- the pure layer has no syntax-crate dependency;
- dependency direction is visible from Cargo manifests and module imports;
- total concepts and re-exports decrease;
- if extraction adds more scaffolding than it removes, retain modules in one
  crate and document that decision.

### PR 06 — One auditable renderer

Scope:

- split document traversal, fit decisions, sinks, and verification into small
  modules with explicit contracts;
- share structural interpretation where it removes real duplication;
- avoid a full discard render before the real render when source verification
  can observe the actual pass safely;
- retain bounded fit budgets and streaming output.

Expected deletions: duplicated node-dispatch code, debug discard sink and second
full traversal where redundant, renderer state aliases and special cases exposed
by the split.

Risks: hot-loop abstraction overhead, altered group decisions, source-audit
state coupled to sink errors.

Gates:

- byte-identical output for structural changes;
- fit work remains explicitly bounded;
- realistic throughput and memory do not regress materially;
- debug source verification observes exactly the bytes/nodes used by the real
  render;
- renderer modules can be understood independently from syntax recovery.

### PR 07 — Simplify language layout rules

Scope:

- tackle the largest rule hotspots after core seams are stable;
- replace booleans, tuples, and Java-only document macros with small domain
  values only when they remove branching at call sites;
- consolidate identical Java/Kotlin mechanics without merging language syntax;
- remove remaining duplicate ignored/recovery paths and dead compatibility
  helpers;
- flatten rules so syntax shape and layout choices are visible together.

Initial hotspots:

- Kotlin declarations (approximately 1,042 lines);
- Kotlin control flow (approximately 856 lines);
- Java modules (approximately 770 lines);
- Kotlin calls (approximately 701 lines);
- Java member bodies (approximately 641 lines).

Expected deletions: opaque option tuples, macro indirection, repeated separator
and body layouts, unreachable defensive branches made impossible by validated
CST access.

Risks: accidental style changes hidden in structural cleanup, over-sharing
different Java/Kotlin rules, proliferation of tiny types.

Gates:

- each hotspot change is separately reviewable and output-preserving unless
  labeled as a formatting change;
- local cyclomatic/branch complexity and file size decline;
- new domain values eliminate more states than they introduce concepts;
- fixture and idempotence suites pass after each language slice.

### PR 08 — Syntax and lexer mechanics

Scope:

- extract only language-neutral UTF-8 cursor and trivia collection mechanics
  shared by both lexers;
- retain language-owned token classification and lexical rules;
- memoize, bound, shrink, or replace Java's parallel lookahead grammar using
  validated CST/parser mechanisms;
- evaluate checked-in generated CST code only if it makes accessors easier to
  inspect and removes generator/macro complexity.

Expected deletions: duplicated byte/cursor/trivia movement, repeated lookahead
work, parallel grammar productions, generator indirection that has no net
reasoning benefit.

Risks: lexer performance regressions, subtle Unicode boundary bugs, coupling
unrelated language grammars, enormous generated diffs.

Gates:

- lexer throughput and allocations do not regress materially;
- Unicode, trivia, and malformed-input fixtures pass;
- lookahead has a documented finite work bound;
- generated-code policy is chosen on measured local-reasoning and diff-size
  evidence, not aesthetics.

### PR 09 — Tests, architecture docs, and API polish

Scope:

- replace filename/count normalization allowlists with structured audit output
  where safe;
- document recovery, conservation, normalization, ignore ownership, and fit
  costs in `docs/internals/formatter.md`;
- update public CLI/dprint/facade APIs after internal seams settle;
- delete transition harnesses and temporary re-exports;
- record final metrics and unresolved follow-ups.

Expected deletions: filename-specific test knowledge, temporary adapters,
obsolete snapshots/harness paths, facade APIs with no current behavior.

Risks: making tests less specific, exposing internal concepts publicly, turning
final polish into a miscellaneous dumping ground.

Gates:

- audit failures identify the responsible syntax/range without filename
  exceptions;
- docs match code ownership and complexity guarantees;
- CLI and dprint integrations remain thin;
- final stack metrics and remaining debts are recorded below.

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

Subagents may research, implement a bounded non-overlapping slice, or review a
branch. The stack owner integrates changes, controls branches and commits,
rebases descendants, runs final verification, and publishes PR updates. A
subagent should not add a new abstraction outside its assigned slice without
first updating this plan through the stack owner.

## Verification Matrix

Run the narrowest relevant checks during implementation and the full available
matrix before publishing each draft update.

| Concern           | Required evidence                                                                    |
| ----------------- | ------------------------------------------------------------------------------------ |
| Build profiles    | debug/release IR and output parity                                                   |
| Java formatting   | unit, corpus snapshots, recovery, layout, idempotence                                |
| Kotlin formatting | unit, corpus snapshots, recovery, layout, idempotence                                |
| Losslessness      | source-conservation and trivia ownership checks                                      |
| Ignore handling   | beginning/end, nested containers, malformed ranges, stress bound                     |
| Syntax            | parser/lexer fixture snapshots and malformed input                                   |
| Integrations      | `jolt_formatter`, CLI, and dprint handler tests                                      |
| Static analysis   | formatting and strict Clippy, with pre-existing debt identified                      |
| Complexity        | deep nesting and adversarial lookahead/ignore inputs                                 |
| Performance       | fixed Java/Kotlin realistic corpus: time, document nodes/token, reserved bytes/token |

Prefer the repository's Ona `test` automation. If environment automation cannot
run, use repository `mise` tasks. Use direct Cargo commands only when those
layers are unavailable or blocked, and record the fallback and missing external
prerequisites. Never weaken a test because the environment lacks a required
fixture or executable.

## Execution Ledger

This table is the source of truth after a context compaction. Update it whenever
a branch is created, a PR is opened, scope changes, a gate fails, or a PR is
ready for review.

| PR | Branch                             | Status      | Parent | Draft PR | Verification            | Notes                                                  |
| -- | ---------------------------------- | ----------- | ------ | -------- | ----------------------- | ------------------------------------------------------ |
| 00 | `cleanup/00-plan-and-gates`        | in progress | `main` | —        | baseline audit complete | Write plan, then add only necessary gates.             |
| 01 | `cleanup/01-doc-semantics`         | planned     | PR 00  | —        | —                       | Profile-independent topology and explicit visibility.  |
| 02 | `cleanup/02-formatter-ignore-plan` | planned     | PR 01  | —        | —                       | One linear root plan.                                  |
| 03 | `cleanup/03-validated-cst-fields`  | planned     | PR 02  | —        | —                       | Begin with vertical slices.                            |
| 04 | `cleanup/04-format-context`        | planned     | PR 03  | —        | —                       | Prevent a god context.                                 |
| 05 | `cleanup/05-pure-doc-core`         | planned     | PR 04  | —        | —                       | Extraction is conditional on proven module boundaries. |
| 06 | `cleanup/06-renderer`              | planned     | PR 05  | —        | —                       | One bounded, auditable traversal model.                |
| 07 | `cleanup/07-language-rules`        | planned     | PR 06  | —        | —                       | May split by independent hotspot.                      |
| 08 | `cleanup/08-syntax-tooling`        | planned     | PR 07  | —        | —                       | Share mechanics, not language semantics.               |
| 09 | `cleanup/09-tests-docs-api`        | planned     | PR 08  | —        | —                       | Finalize only after core seams settle.                 |

## Decision Log

| Date       | Decision                                                     | Reason                                                                                                |
| ---------- | ------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------- |
| 2026-07-22 | Use a stack of small draft PRs rather than a rewrite.        | Preserves review and rollback boundaries while allowing ambitious end-state changes.                  |
| 2026-07-22 | Centralize branches, commits, rebases, and publication.      | Subagents share a filesystem; central integration avoids hidden branch state and conflicting commits. |
| 2026-07-22 | Make crate extraction conditional.                           | Purity is about dependency direction, not maximizing crate count.                                     |
| 2026-07-22 | Treat growth without deleted complexity as a stop condition. | The cleanup exists to reduce reasoning load, not install a new framework.                             |

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
