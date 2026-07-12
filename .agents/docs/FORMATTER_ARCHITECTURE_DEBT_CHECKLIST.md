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
      recovery accessors at
      `crates/jolt_java_syntax/src/nodes/accessors.rs:243-245,286-288,359-362,4087-4095`.
- [x] Preserve repeated `requires` modifiers:
      `fixtures/java/syntax/recovery/module-repeated-requires-modifiers.java`;
      recovered modifier accessors at
      `crates/jolt_java_syntax/src/nodes/accessors.rs:4313-4321`.

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
- [ ] Extend recovery comment conservation from explicit markers to canonical
      inventories of every represented source comment.
- [x] Stop skipping parser-diagnostic fixtures in
      `crates/jolt_java_fmt/tests/corpus.rs:28-33` and
      `crates/jolt_kotlin_fmt/tests/corpus.rs:28-33`; route every represented
      tree through conservation and idempotence checks.
- [ ] Stop skipping diagnostic imported Java and Kotlin inputs in
      `crates/jolt_java_fmt/tests/corpus_fixtures.rs:40-53` and
      `crates/jolt_kotlin_fmt/tests/corpus_fixtures.rs:35-49`; report exact
      skipped paths rather than aggregate counts.
- [ ] Make imported fixture manifests content-addressed instead of validating
      only aggregate counts.
- [ ] Make imported Java and Kotlin syntax reconstruction loss a hard failure
      instead of a summary count in
      `crates/jolt_java_syntax/tests/parser_fixtures.rs:38-42` and
      `crates/jolt_kotlin_syntax/tests/imported_fixtures.rs:25-38`.
- [ ] Give valid structured output and tracked malformed verbatim output exact
      debug/test accounting so duplicated source tokens cannot mask loss.
- [ ] Audit every Java and Kotlin token-sequence/raw replay call. Retain it only
      inside tracked syntax-owned malformed/bogus verbatim or formatter-ignore;
      valid syntax must use structured rules.
- [ ] Return a diagnostic when either formatter receives no syntax tree instead
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

| Old phase                                | Commit    | Classification   | Disposition                                                                                                                                                                                                                                     |
| ---------------------------------------- | --------- | ---------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1. Authoritative gates                   | `86b178b` | Revise           | Keep no-tree diagnostics, content-addressed manifests, diagnostic corpora, comment inventories, and renderer completion. Rework token accounting as dense debug/test tracking over existing IDs; do not add a production parts/provenance tree. |
| 2. Java recovery ownership               | `2aff426` | Supersede        | Keep parser reachability fixes, fixtures, and useful grammar-role vocabulary. Replace skip/range-derived recovery accessors with generated fields and syntax-owned malformed/bogus boundaries.                                                  |
| 3. Kotlin declaration ownership          | `d1783cf` | Supersede        | Keep duplicate-header parsing, fixtures, and parser findings. Replace ordered recovery streams, range-derived headers/bodies, and skip predicates with generated fields and malformed ownership.                                                |
| 4. Kotlin expression/type ownership      | `d5d2fc8` | Supersede        | Keep its recovery corpus and delimiter/list findings. Move ownership into parser recovery and generated fields with explicit malformed nodes; do not reconstruct expression roles in the formatter.                                             |
| 5. Kotlin types/parameters               | `bb3fa8d` | Supersede        | Keep valid canonical layout, parser fixes, list primitives, and fixtures. Replace per-node recovery state machines with structured valid rules plus tracked verbatim malformed dispatch.                                                        |
| 6. Kotlin declarations                   | `35e2e6e` | Supersede        | Keep parser fixes, canonical declaration documents, and fixtures. Remove prefix/header/tail recovery partitioning and format directly from valid decoded fields or a malformed boundary.                                                        |
| 7. Kotlin expressions/control flow       | `c2f7a66` | Supersede        | Keep parser-reachable fixtures and bounded recovery findings. Remove manual recovery loops, range inference, and parser-inexpressible completion claims.                                                                                        |
| 8. Kotlin formatter-owned parsing/replay | `598e535` | Supersede        | Keep the `fun interface` parser correction, canonical rules, and fixtures. Replace filtered token fallbacks with syntax-owned malformed verbatim; valid nodes must never replay.                                                                |
| 9. Kotlin source-gap layout              | `ce91a28` | Accept unchanged | Retain represented-trivia classification, raw-gap removal on valid paths, formatter-ignore boundary, comment ownership, and linear matching.                                                                                                    |
| 10. Kotlin repair/panic paths            | `4d52772` | Revise           | Keep guarded valid-syntax normalization, malformed-token preservation, parser fixtures, and panic removal. Route malformed imports verbatim and keep valid import sorting behind malformed barriers.                                            |
| 11. Java programs/declarations           | `95d158c` | Supersede        | Keep duplicate-package parsing, explicit recovery nodes, singleton removal, valid canonical rules, and fixtures. Replace bespoke recovery streams and dispatch.                                                                                 |
| 12. Java expressions/statements          | `e4db005` | Supersede        | Keep parser fixes, fixtures, and grammar-role vocabulary. Replace optional-anchor/range recovery regions and formatter loops with generated fields and malformed boundaries.                                                                    |
| 13. Java formatter-owned parsing/replay  | `e47c982` | Supersede        | Keep borrowed operator identity, valid canonical rules, fixtures, and the finding that valid replay is unsafe. Replace local recovery formatting with tracked malformed verbatim.                                                               |
| 14. Java repair/panic paths              | `c272352` | Revise           | Re-extract missing-body parser boundaries, no-repair behavior, panic removal, and fixtures. Missing bodies must create syntax-owned malformed boundaries before formatter dispatch.                                                             |
| 15. Cross-language source reconstruction | `fa6055d` | Supersede        | Keep trivia/lexical-boundary findings and fixtures. Recovered source-gap reconstruction is unnecessary once malformed subtrees are tracked verbatim; valid layout remains structured.                                                           |
| 16. Cost model                           | `046eff8` | Accept unchanged | Carry forward bounded formatter-ignore lookup, finite sorting models, constant-time parent-role lookup, comment deduplication, and source-gap helper removal.                                                                                   |

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
IR metrics in that commit. Each reports both parent-to-child incremental and
cumulative deltas from `2197128`; earlier improvements provide no headroom for a
later regression. Performance attribution may not be deferred to the final gate.

### New Phase 1: Carry Forward Accepted Cost Controls

Re-extract old Phase 16's bounded formatter-ignore lookup, finite sorting
models, constant-time parent-role lookup, comment deduplication, and source-gap
helper removal. Run the production complexity scan so the baseline includes
these already-accepted improvements.

### New Phase 2: Carry Forward Represented-Trivia Layout

Re-extract old Phase 9's represented-trivia classification, valid-path raw-gap
removal, formatter-ignore boundary, comment ownership, and linear matching
without carrying its superseded recovery consumers.

### New Phase 3: Establish Performance And Allocation Baselines

Extend the benchmark harness with parse-only and format-only execution,
allocation count/bytes, peak memory, syntax-tree bytes per node/token, and
formatter-document nodes per input token. Record reproducible release baselines
at `2197128` for the Java, Kotlin, realistic, and adversarial manifests,
including commands, toolchain, machine identity, manifest digests, raw samples,
and dispersion. Enforce the architecture's three-percent time, one-percent
allocation, five-percent peak-memory, and five-percent tree-byte budgets both
incrementally and cumulatively.

### New Phase 4: Retained Regression Inventory

Create a machine-readable inventory of every parser-reachable fixture and parser
fix from old Phases 1–15, mapped to its owning replacement phase. This commit
contains no formatter mechanism.

### New Phase 5: Public Completion And Corpus Baseline

Re-extract no-tree diagnostics, halted-render rejection, content-addressed
manifests, exact failing paths, represented-comment inventories, and diagnostic
corpus routing from old Phase 1. Nonempty loss or idempotence findings fail
instead of becoming snapshot allowlists.

### New Phase 6: Grammar Schema, Syntax Factory, And Field Representation

Create the single declarative grammar-shape source that generates category
unions and bogus kinds, construction-time shape validation, stored grammar slots
including `Empty`, and constant-time typed fields. Replace `TreeElement` with
`TreeSlot` in the flat arena; do not add a parallel role or decoder layer. Add
the sealed `Valid(fields)`, `Bogus(owner)`, or `InvariantError` formatter
result, direct-slot exhaustiveness tests, and parse-only CPU, allocation,
memory, tree-byte, and production-line measurements. Stop if the representation
exceeds the performance budgets relative to its parent or cumulatively. Record a
by-crate final projection that is net negative against `2197128`, including
generated source. Add forbidden-pattern gates for P16-only ordered recovery
parts and formatter-local recovery loops. No production formatter call site
changes here.

### New Phase 7: Tracked Verbatim Primitive

Add API-only tracked verbatim output, debug/test dense token and derived-trivia
accounting, exceptional-fragment lexical boundaries, formatter-ignore
separation, valid-path verbatim tags, and normalization claims. Test it against
small constructor-valid/bogus trees without wiring a production formatter
family. Add focused allocation and throughput microbenchmarks for the primitive;
optimized builds add no per-node tracker or comment-map allocation.

### New Phase 8: Java Vertical Slice

For expression statements, binary/unary expressions, `instanceof` patterns, one
list, and blocks, add category-compatible bogus nodes, structural diagnostic
ownership, generated fields, structured valid rules, malformed-only verbatim
dispatch, cleanup, fixtures, and Phase 3 measurements in one commit.

### New Phase 9: Kotlin Vertical Slice

Apply the same vertical migration to function types, value parameters, property
bodies, calls/navigation, and one block/control-flow family. Cover missing and
orphan delimiters, comment boundaries, cleanup, and Phase 3 measurements.

### New Phase 10: Java Programs, Modules, And Imports

Vertically migrate compilation units, packages, imports, modules/directives, EOF
comments, and sorting barriers. Valid nodes remain structured; malformed spans
use the narrowest category-compatible bogus owner.

### New Phase 11: Java Names, Types, And Declaration Prefixes

Vertically migrate names, types, dimensions, annotations, modifiers, parameters,
declarators, and throws clauses. Delete range-derived and skip-capable recovery
accessors for these families.

### New Phase 12: Java Declarations

Vertically migrate fields, methods, constructors, initializers, annotation
elements, classes, interfaces, enums, records, members, and bodies. Missing-body
diagnostics must have narrow syntax owners; valid declarations may not replay.

### New Phase 13: Java Expressions And Patterns

Vertically migrate remaining operators, primary expressions, calls, references,
lambdas, arrays/objects, patterns, and expression-owned lists. Preserve borrowed
operator identity and delete local recovery formatting.

### New Phase 14: Java Statements And Control Flow

Vertically migrate simple statements, loops, switches, resources, catches, and
remaining control-flow families. Delete Java's final bespoke recovery
formatting.

### New Phase 15: Kotlin Programs, Packages, Imports, And Names

Vertically migrate files, duplicate package/import containers, names, EOF
comments, and imports. Retain Phase 2 trivia behavior and use
category-compatible bogus entries as sorting barriers.

### New Phase 16: Kotlin Types And Parameters

Vertically migrate names/types, arguments/parameters, constraints, projections,
context parameters, function types, and type-owned lists not covered by Phase 9.
Delete range-derived recovery and list state machines.

### New Phase 17: Kotlin Declarations

Vertically migrate properties, functions, constructors, accessors, type aliases,
classes, objects, interfaces, enum entries, delegation, and member bodies.
Delete prefix/header/tail partitioning and declaration recovery loops.

### New Phase 18: Kotlin Expressions And Calls

Vertically migrate operators, strings, lambdas, collections, callable
references, object literals, and remaining call/navigation families. Delete
filtered token fallback and expression-local recovery state.

### New Phase 19: Kotlin Statements And Control Flow

Vertically migrate branches, loops, `when`, `try`, and remaining statements and
blocks. Delete Kotlin's final bespoke recovery formatting.

### New Phase 20: Normalization And Totality Audit

Audit every spelling/reordering/synthetic normalization and every panic or empty
fallback. Normalizations require valid syntax and exact debug/test claims;
malformed syntax is preserved verbatim rather than repaired.

### New Phase 21: Delete Transitional Recovery Architecture

Remove obsolete recovery accessors, filtered token fallbacks, source-range
ownership, recovery sorters, and local recovery join helpers. Prove that every
valid node kind has a structured rule, every malformed category has tracked
verbatim dispatch, and clean corpora emit zero verbatim tags.

### New Phase 22: Final Performance Gate

Repeat the Phase 3 release benchmarks on the same machine and manifests. Reject
per-node allocation, a release comment map, or a result exceeding the
architecture's incremental or cumulative time, allocation, memory, or tree-byte
budgets without an explicit approved architecture amendment.

### New Phase 23: Clean Completion Proof

Run generated-field exhaustiveness, bogus-category and diagnostic-ownership
snapshots, token/comment tracking, valid zero-verbatim gates, deterministic
mutations, in-repository and imported corpora, CLI/dprint tests, `mise run fix`,
and `mise run test`. Scan for valid replay, untracked verbatim, raw-gap layout,
repair synthesis, panic paths, unbounded algorithms, and formatter-side
structural layers. Fail if P16-only ordered recovery parts or local replay loops
were reintroduced. Report generated and hand-written production LOC separately,
prove that total production Rust under `crates/**/src/**/*.rs` excluding
`jolt_test_support` is net negative relative to `2197128`, including generated
code, and fail if two grammar-shape descriptions remain. Change status to
`CLEAN` only when every correctness, size, and performance gate passes.

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
