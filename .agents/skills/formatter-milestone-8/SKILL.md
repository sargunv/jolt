---
name: formatter-milestone-8
description: >-
  Burn down Java formatter Milestone 8 oracle layout-policy gaps and finish
  helper-layer architecture migration. Use when working on Milestone 8, oracle
  scoreboards, layout policy gaps, selector chains, profile alignment, or
  migrating formatter logic into helpers/analyzers/policy.rs.
---

# Formatter Milestone 8 Operating Strategy

Source of truth: `.agents/docs/formatter-milestone-8-coverage-roadmap.md`.

Milestone 8 is not parser coverage. Parser-clean Java already formats. Remaining
work is helper-layer architecture plus oracle layout-policy parity. One session
= one substantial work unit (a policy mechanism or architecture migration), not
one fixture or cosmetic shuffle.

## North star

Completion order (do not regress earlier profiles while working later ones):

1. 100% Google on the pinned google-java-format corpus
2. 100% AOSP on the same corpus
3. 100% Palantir on the pinned palantir-java-format corpus

Layering (must hold):

```text
rules/      identify grammar slots, own comment ranges, delegate
analyzers/  flatten/classify syntax shape
helpers/    named Java layout policy
policy.rs   Google/AOSP/Palantir differences
layout.rs   generic Doc plumbing only
jolt_fmt_ir language-neutral document algebra only
```

Invariants (never violate):

- No fallback exits, raw source passthrough for parser-clean syntax, or silent
  comment drops
- Parser-clean syntax formats through real rules and helpers
- Profile checks live in `policy.rs`, not leaf rule modules
- No fixture-name, method-name, or class-name heuristics
- Tests fail on missing fixtures or other environment misconfiguration; do not
  silently skip
- Do not add tests that merely duplicate source definitions; oracle corpora are
  the integration signal for layout policy

Read the roadmap's Architecture North Star, Current Status, and Non-Goals before
changing code. Treat landed migrations listed there as done, not future work.

## Session loop

Copy and update each session:

```text
Milestone 8: [work unit from roadmap Work Order or Architecture Gap Checklist]
Target profile(s): [google | aosp | palantir]
- [ ] 1. Pick work unit (highest-risk / broadest domain first)
- [ ] 2. Baseline from local reports + scoreboard snaps
- [ ] 3. Study local oracle mirrors + Jolt code
- [ ] 4. Plan helper/policy change locally
- [ ] 5. Implement (agent worker or direct edit)
- [ ] 6. Review patch; revise until acceptable
- [ ] 7. Verify (oracle_fixtures vs mise run test)
- [ ] 8. Evaluate and close or iterate
```

### 1. Pick work unit

Default to the roadmap **Work Order** unless the user names a target. Prefer the
highest-risk, broadest domain still open:

1. Global break-selection performance (`B24909927.java` must stop dominating the
   oracle suite)
2. Shared selector-chain policy (Google/AOSP top fixtures)
3. Palantir chain breakability
4. String / text-block handling
5. Argument-list nested-call and format-method fill
6. Declaration header and initializer edges (shrink `rules/declarations.rs`)
7. Comment and annotation placement (explicit debt domains)
8. AOSP import grouping (after shared Google-style gaps are stable)
9. Low-volume tail (arrays, ternaries, switches, record patterns)

Also schedule items from the **Architecture Gap Checklist** when they unblock a
gap above (e.g. move chain assembly out of `rules/expressions.rs`, replace
nested `best_fitting` heuristics, audit raw-comment emission in list helpers).

State explicitly:

- **Work unit name** (policy mechanism, not one fixture)
- **Primary code** (from roadmap evidence paths)
- **Target fixtures** (top 3-5 from roadmap reports for that gap)
- **Target profile** (Google for shared GJF gaps; Palantir only for roadmap
  Palantir-specific work; AOSP after the shared Google shape is understood)
- **Minimum outcome** (expected diff delta on those fixtures / aggregate)

**Too small:** one fixture patch, one break site without general policy,
rename-only move, stub helper with no migrated call sites.

**Right size:** a named gap category or full helper extraction with call sites
routed and policy centralized. Split large gaps by policy mechanism (e.g. chain
flattening vs break rendering), not by fixture file.

### 2. Baseline from local reports

Do not browse the network for oracle implementation. Use local mirrors and
generated reports only.

Pinned scoreboard snapshots:

```sh
rg "exact-match percentage|aggregate diff size|largest per-file" \
  crates/jolt_java_fmt/tests/snapshots/oracle_fixtures__*_scoreboard.snap
```

Fresh per-file diffs (after step 7 oracle run):

- `.oracles/reports/java/google-java-format/google/index.md`
- `.oracles/reports/java/google-java-format/aosp/index.md`
- `.oracles/reports/java/palantir-java-format/palantir/index.md`

Open representative `.md` reports for the target fixtures. Summarize in one
paragraph: **what layout behavior differs** (expected vs actual), not "fixture X
fails."

If a local oracle mirror under `.oracles/repos/` is missing, report a setup
problem and continue with reports, fixtures, and Jolt code only.

### 3. Study references

For the selected gap, read the roadmap's **Oracle Gap Checklist** entry
(evidence paths, mismatch shape). Then read matching code locally:

- **Google / AOSP:**
  `.oracles/repos/google__google-java-format/core/src/main/java/com/google/googlejavaformat/`
  (especially `java/JavaInputAstVisitor.java`, `Doc.java`, `OpsBuilder.java`,
  `StringWrapper.java`, `ImportOrderer.java`, `JavaCommentsHelper.java`)
- **Palantir:** `.oracles/repos/palantir__palantir-java-format/` (especially
  `JavaInputAstVisitor.java`, `Level.java`, `BreakBehaviour.java`,
  `LastLevelBreakability.java`, `PartialInlineability.java`)
- **Pinned fixtures / reports:** `.oracles/fixtures/`, `.oracles/reports/`
- **Jolt:** modules cited in the roadmap entry for that gap

Use `rg` locally. Do not fall back to GitHub or other network browsing.

Classify the bug:

- missing or wrong `JavaFormatPolicy` in `policy.rs`
- policy wired in rules instead of helpers
- missing analyzer metadata
- nested `best_fitting` / local width heuristics vs global break selection
- comment placement debt (see `rules/tests.rs` named debt cases)

For **Global Break Selection Debt**, read that roadmap section before changing
chain, list, or Palantir break behavior.

### 4. Plan locally

Write a short plan before coding:

- Which helper/analyzer/policy surface changes
- Which rule modules get thinner (delegate only)
- Which roadmap fixtures must move
- Whether the change needs IR-level break-state support vs helper thresholds
  only

**Prefer:** extend a named helper/analyzer; add a narrow `JavaFormatPolicy`
accessor with oracle evidence; verify via oracle reports.

**Reject:** open-coded layout in `rules/*.rs`; fixture/method/class heuristics;
fallback exits or raw passthrough to green tests; mechanical splits without a
real helper surface; new unit tests duplicating oracle-covered policy; new broad
`best_fitting` use in Java helpers.

### 5. Prefer implementation with a fast worker

For substantial implementation, prefer using a fast code-edit worker. The main
agent should plan, constrain the work, review the patch, and decide whether it
is good enough. Let the worker do the heavy editing when the task has a clear
file scope and acceptance criteria:

```sh
agent --yolo --print --trust '<precise prompt>'
```

Prompt must include:

- Work unit and target profile
- Exact files to touch (from roadmap + step 3)
- Required layering (rules delegate, policy in `policy.rs`)
- Invariants above
- Target fixtures and the layout behavior to match
- What not to do (no fixture heuristics, no fallback exits, no new redundant
  tests, no new broad `best_fitting`)

Expect the agent worker to not return any output while it works; silence does
NOT mean the agent is stalled.

Use direct edits only for small surgical fixes, review corrections, or cases
where the work is too subtle to delegate safely. Still follow the same plan and
invariants.

After chain/list/declaration/profile changes, audit stray profile checks:

```sh
rg "CONTINUATION_INDENT|JavaFormatProfile::" crates/jolt_java_fmt/src
```

Expect profile matches only in `policy.rs`, `options.rs`, context defaults, and
tests (per roadmap Current Status).

### 6. Review patch and revise

You own correctness. After the worker returns:

1. Read the full diff. Check layering, invariants, and scope (one work unit).
2. Re-read affected oracle report snippets for target fixtures if reports exist.
3. If wrong or incomplete, kick back with a **precise revision prompt** (what to
   keep, what to revert, exact behavior still missing). Re-run the worker or fix
   directly.
4. Repeat until the patch is acceptable before running full verification.

Do not accept aggregate scoreboard wins that regress unrelated top fixtures or
violate profile order.

### 7. Verify: oracle_fixtures vs mise run test

**During iteration** (layout policy work; regenerates `.oracles/reports/`):

```sh
INSTA_UPDATE=no cargo test -p jolt_java_fmt --test oracle_fixtures
```

- Oracle tests pass even when fixtures mismatch; they snapshot scoreboard
  summaries.
- Insta fails when aggregate metrics change -- expected while iterating.
- Per-file diffs always land under `.oracles/reports/java/.../`; use those to
  judge progress without updating scoreboard snaps.

Each oracle report index includes a `Slowest Fixtures` section. `B24909927.java`
taking tens of seconds is known Global Break Selection Debt, not report
generation overhead. Treat large timing regressions as architecture failures
even if diff scoreboards improve. Do not exclude pathological fixtures except as
a last resort when unrelated work is completely blocked.

Treat existing `best_fitting` calls in `jolt_java_fmt` as migration targets.
Small bounded choices may survive temporarily, but chains, argument lists,
declarations, arrays, and lambdas should move toward one grouped document with
optional breaks rather than parallel finished layouts.

**Architecture-only edits** (no oracle-facing layout change):

```sh
cargo fmt --check
INSTA_UPDATE=no cargo test -p jolt_java_fmt
```

**Before closing a session** (full workspace regression):

```sh
mise run test
```

**When accepting improved scoreboard metrics** (updates only the three oracle
scoreboard snapshots):

```sh
INSTA_UPDATE=always cargo test -p jolt_java_fmt --test oracle_fixtures
```

Do not use `mise run test-update` for oracle-only work; it updates unrelated
workspace snapshots.

Raw-source/comment invariant audit:

```sh
rg -n "raw_text\\(|source_text\\(|text\\(context\\.raw" crates/jolt_java_fmt/src
```

Raw text is legitimate at token/literal/comment-preservation boundaries. Inspect
new matches and reject arbitrary node passthrough or catch-all fallback exits.

### 8. Evaluate and close

Compare before vs after on report indexes and top fixture `.md` files. Review by
**layout category**, not aggregate number alone.

| Signal         | Pass criteria                                                 |
| -------------- | ------------------------------------------------------------- |
| Gap fixtures   | Diff size down on roadmap-listed fixtures for this unit       |
| Aggregate diff | Down on target profile index, or unchanged with focused win   |
| Worst fixtures | No new top-10 regression in an unrelated domain               |
| Profile order  | Google stable while on AOSP; Google/AOSP stable on Palantir   |
| Architecture   | Policy in helper/analyzer/policy; rules thinner if in scope   |
| Invariants     | No fallback exits / raw-passthrough / comment-debt violations |

**Close** when target fixtures improved materially, `mise run test` passes,
scoreboard snaps updated if metrics improved, and the change is explainable from
syntax shape / documented profile policy.

**Iterate** when the same failure mode persists or a sibling fixture in the same
gap regressed. Return to step 3, not ad-hoc rule tweaks.

Session note template:

```markdown
## Milestone 8: [work unit]

**Unit:** ... **Profile:** ... **Scoreboard:** aggregate X->Y, exact match
A%->B% **Fixtures:** ... (diff sizes) **Code:** ... **Architecture:** ...
**Next:** ...
```

Update the roadmap doc only when baseline numbers or gap status materially
changed. Do not rewrite it every session.

## Quick domain map (see roadmap for detail)

| Domain                         | Jolt focus                                                                       |
| ------------------------------ | -------------------------------------------------------------------------------- |
| Selector chains (Google/AOSP)  | `analyzers/chains.rs`, `helpers/chains.rs`, residue in `rules/expressions.rs`    |
| Palantir chain breakability    | `helpers/chains.rs`, `policy.rs`, break-state / IR                               |
| Argument lists / nested calls  | `helpers/lists.rs`, `analyzers/format_strings.rs`                                |
| Declarations / initializers    | `helpers/callables.rs`, `helpers/type_declarations.rs`, `rules/declarations.rs`  |
| Strings / text blocks          | `helpers/literals.rs`, `helpers/expressions.rs`                                  |
| Comments / vertical whitespace | `comments.rs`, `helpers/comments.rs`                                             |
| AOSP imports                   | `helpers/imports.rs`, `policy.rs`                                                |
| Tail edges                     | `helpers/array_initializers.rs`, `helpers/expressions.rs`, `helpers/switches.rs` |

## Anti-patterns

- Fixing one fixture by name or special case
- Adding broad `best_fitting(flat_subtree, [broken_subtree])` alternatives in
  Java formatter helpers
- Palantir-only policy while ignoring a shared Google/AOSP mechanism that is the
  real cause of the selected gap
- Treating landed extractions (expressions, imports, switches, lambdas, etc.) as
  future work
- Editing oracle `.snap` files by hand
- `mise run test-update` when only oracle scoreboards should move
- Network browsing for oracle source instead of `.oracles/repos/`
- Accepting a worker patch without reading the diff

## Related docs

- `.agents/docs/formatter-milestone-8-coverage-roadmap.md`
- `.agents/docs/formatter-plan.md`
- `.oracles/reports/java/**/index.md`
- `crates/jolt_java_fmt/src/rules/mod.rs`
