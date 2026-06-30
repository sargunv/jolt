---
name: formatter-milestone-8
description: >-
  Burn down Java formatter Milestone 8 oracle compatibility diffs and finish
  helper-layer architecture migration. Use when working on Milestone 8, oracle
  scoreboards, layout policy gaps, selector chains, Palantir/AOSP profile
  alignment, or migrating formatter logic into helpers/analyzers/policy.rs.
---

# Formatter Milestone 8 Burn-Down

Close oracle gaps **one work unit at a time** while moving logic into the
architecture described in
`.agents/docs/formatter-milestone-8-coverage-roadmap.md`.

Each session tackles **one substantial work unit** — not a single fixture, call
site, or cosmetic refactor:

1. **One compatibility gap** — a full row from
   [Impact-ordered fix map](.agents/docs/formatter-milestone-8-coverage-roadmap.md#impact-ordered-fix-map)
   (e.g. all of selector chain breaking, all of argument-list fill), implemented
   as helper/analyzer/policy changes that should move **many** related fixtures
2. **One architecture migration** — a complete helper extraction or rule-module
   slim-down from
   [Helper surface status](.agents/docs/formatter-milestone-8-coverage-roadmap.md#helper-surface-status)
   (e.g. stand up `helpers/expressions.rs` and route assignment/binary/lambda
   policy through it; shrink `declarations.rs` by moving header policy into
   callables/type helpers)

Do not mix **unrelated** gaps in one session unless the roadmap explicitly
groups them (e.g. wiring `policy.continuation_indent_levels()` through chain
helpers).

## Work unit sizing

**Too small** (stop and widen the unit):

- Fixing one oracle fixture by name or special case
- Tweaking one break site without a general policy in helper/analyzer/policy
- Moving a few lines to a helper with no behavior change
- "Preparing" an extraction without routing real call sites through it

**Right size:**

- A named gap category from the roadmap, with success measured across its top
  fixtures and aggregate diff on the target profile
- A full helper module landing with call sites migrated and policy centralized
- A profile policy surface (e.g. Palantir chain breaking) wired end-to-end

If a gap is too large for one session, split by **policy mechanism** (e.g. chain
flattening vs chain break rendering), not by fixture file.

## North star (session must not regress this)

Profile completion order:

1. **100% Google** exact match on the pinned Google corpus
2. **100% AOSP** on the same corpus
3. **100% Palantir** on the pinned Palantir corpus

Invariants (must stay satisfied):

- No `missing_layout` / raw source passthrough / silent comment drops
- Parser-clean syntax formats through real rules and helpers
- Profile checks live in `policy.rs`, not leaf rule modules
- No fixture-name or method-name heuristics

Read the roadmap's
[North Star](.agents/docs/formatter-milestone-8-coverage-roadmap.md#north-star)
and
[Design principles](.agents/docs/formatter-milestone-8-coverage-roadmap.md#design-principles)
before changing code.

## Loop

Copy this checklist at the start of each session and update it as you go:

```text
Milestone 8 session: [gap name OR arch migration name]
Target profile(s): [google | aosp | palantir]
- [ ] 1. Select work unit
- [ ] 2. Baseline oracle + reports
- [ ] 3. Study references
- [ ] 4. Plan helper/policy change
- [ ] 5. Implement
- [ ] 6. Test
- [ ] 7. Evaluate
- [ ] 8. Close or iterate
```

### 1. Select work unit

Pick from the roadmap, highest impact first unless the user names a target:

| If working on…        | Start here in roadmap                                                                                                                                                                                                   |
| --------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Oracle diff reduction | [Oracle compatibility gaps](.agents/docs/formatter-milestone-8-coverage-roadmap.md#oracle-compatibility-gaps) → [Impact-ordered fix map](.agents/docs/formatter-milestone-8-coverage-roadmap.md#impact-ordered-fix-map) |
| Architecture cleanup  | [Helper surface status](.agents/docs/formatter-milestone-8-coverage-roadmap.md#helper-surface-status)                                                                                                                   |
| Profile-specific      | [AOSP amplification](.agents/docs/formatter-milestone-8-coverage-roadmap.md#aosp-specific-amplification) or [Palantir-specific gaps](.agents/docs/formatter-milestone-8-coverage-roadmap.md#palantir-specific-gaps)     |

State explicitly:

- **Gap / migration name** (full category, not one fixture)
- **Primary code** (`helpers/chains.rs`, `helpers/lists.rs`, …)
- **Target fixtures** — top 3–5 from the gap's roadmap list; session is not done
  until most of these move, not just one
- **Target profile** — default Google until Google is at 100%; do not chase
  Palantir-only policy before shared gaps are closed unless the user asks
- **Minimum outcome** — expected aggregate diff delta or fixture diff sizes that
  justify closing the session

### 2. Baseline oracle + reports

Run from the **repository root**.

Pinned scoreboard snapshots (last committed baseline):

```sh
rg "exact-match percentage|aggregate diff size|largest per-file" \
  crates/jolt_java_fmt/tests/snapshots/oracle_fixtures__*_scoreboard.snap
```

Fresh metrics and per-file diffs: run the oracle harness (step 6), then read:

- `.oracles/reports/java/google-java-format/google/index.md`
- `.oracles/reports/java/google-java-format/aosp/index.md`
- `.oracles/reports/java/palantir-java-format/palantir/index.md`

Also open representative `.md` reports linked from those indexes (see
[Representative reports to inspect](.agents/docs/formatter-milestone-8-coverage-roadmap.md#representative-reports-to-inspect)).

Before coding, summarize in one paragraph: **what layout behavior differs**
(expected vs actual), not just "fixture X fails."

### 3. Study references

For the selected gap, read the relevant oracle implementation from the local
mirrors under `.oracles/repos/`. These checked-out sources are the primary
reference for Milestone 8; use `rg` locally instead of browsing GitHub.

- **Google / AOSP policy** →
  `.oracles/repos/google__google-java-format/core/src/main/java/com/google/googlejavaformat/`
  - Java layout visitor: `java/JavaInputAstVisitor.java`
  - Formatting pipeline: `java/Formatter.java`
  - Document IR and op lowering: `Doc.java`, `OpsBuilder.java`,
    `DocBuilder.java`
  - Token/comment model: `java/JavaInput.java`, `java/JavaCommentsHelper.java`
  - Import ordering: `java/ImportOrderer.java`
  - String/text handling: `java/StringWrapper.java`
- **Palantir policy** → `.oracles/repos/palantir__palantir-java-format/`
  - Java layout visitor:
    `palantir-java-format/src/main/java/com/palantir/javaformat/java/JavaInputAstVisitor.java`
  - Column-limit and break machinery:
    `palantir-java-format/src/main/java/com/palantir/javaformat/doc/Level.java`,
    `palantir-java-format/src/main/java/com/palantir/javaformat/BreakBehaviour.java`,
    `palantir-java-format/src/main/java/com/palantir/javaformat/LastLevelBreakability.java`,
    `palantir-java-format/src/main/java/com/palantir/javaformat/PartialInlineability.java`
  - Style profiles:
    `palantir-java-format-spi/src/main/java/com/palantir/javaformat/java/JavaFormatterOptions.java`
  - Palantir fixtures:
    `palantir-java-format/src/test/resources/com/palantir/javaformat/java/testdata/`
- **Pinned oracle fixtures and generated reports** → `.oracles/fixtures/` and
  `.oracles/reports/`.
- **Comment ownership architecture only** → use external Ruff/Oxc references
  only when specifically working on comment-bucket architecture.

Do **not** fall back to network browsing for oracle implementation references.
If a local oracle mirror is missing or lacks the needed file, report that as a
repository setup problem and continue only with local reports, fixtures, and
Jolt code until the mirror is restored.

Also read Jolt's current implementation at the **Primary code** path from the
fix map. Identify whether the bug is:

- missing policy in `policy.rs`
- wrong policy wiring (hardcoded indent, profile ignored)
- missing analyzer metadata
- rule module owning policy that belongs in a helper

### 4. Plan helper/policy change

Plan must follow the layering target:

```text
rules/     → identify slots, comments, delegate
analyzers/ → flatten/classify syntax shape
helpers/   → named Java layout policy
policy.rs  → profile differences
layout.rs  → generic Doc plumbing only
```

**Prefer:**

- Extend or fix a named helper/analyzer
- Add a narrow `JavaFormatPolicy` accessor with a comment citing oracle evidence
- Verify through oracle reports and scoreboards, not new unit tests

**Reject:**

- Open-coded layout in `rules/*.rs` that duplicates list/chain/declaration
  policy
- `if fixture == …` or method/class-name checks
- `missing_layout`, raw source passthrough, or ignoring unhandled comments to
  green tests
- Moving Java policy into `jolt_fmt_ir` unless multiple helpers need the same IR
  primitive (roadmap preference order)
- **New unit tests** that duplicate oracle-covered layout policy — the pinned
  corpora are the integration signal; do not bloat `rules/tests.rs`

For architecture migrations: extract a **complete** helper surface (module +
call-site migration + policy moved out of rules). Do not land empty modules or
single-function shuffles.

### 5. Implement

Implement the **full work unit** — general policy for the gap category, not a
fixture patch. Touch the modules needed to express that policy; do not stop
after the first green fixture if siblings in the same gap still fail the same
way.

When changing chain, list, declaration, comment, or profile behavior, grep for
hardcoded continuation indent and profile checks outside `policy.rs`:

```sh
rg "CONTINUATION_INDENT|JavaFormatProfile::" crates/jolt_java_fmt/src
```

### 6. Test

**Oracle fixtures are the primary verification** for layout policy. Existing
unit tests in `rules/tests.rs` guard regressions on already-landed behavior; do
not add new ones for gap burn-down unless the user explicitly asks.

Run from the **repository root**.

**Tight iteration loop** (oracle only — regenerates `.oracles/reports/` every
run):

```sh
INSTA_UPDATE=no cargo test -p jolt_java_fmt --test oracle_fixtures
```

Notes:

- Oracle tests **pass even when many fixtures mismatch**; they snapshot the
  scoreboard summary. When aggregate metrics change, this command fails on
  **insta snapshot drift** — that is expected while iterating.
- Per-file diffs are always written under `.oracles/reports/java/…/` during the
  run. Use those reports to evaluate progress without updating scoreboard snaps.
- The harness runs three scoreboard tests (Google, AOSP, Palantir) plus two
  small diff-size unit tests in the same file.

**Before closing a session** (full workspace regression gate):

```sh
mise run test
```

This runs `INSTA_UPDATE=no cargo test` across the workspace (see `mise.toml`).

**When accepting improved scoreboard metrics** (updates only the three oracle
scoreboard snapshots):

```sh
INSTA_UPDATE=always cargo test -p jolt_java_fmt --test oracle_fixtures
```

Do not use `mise run test-update` for oracle-only work — it runs
`INSTA_UPDATE=always cargo test` on the **entire workspace** and may update
unrelated snapshots.

Coverage invariant check (no matches expected; `rg` exits 1 when clean):

```sh
rg -n "missing_layout|format_raw_source_text|take_remaining_comment_docs" \
  crates/jolt_java_fmt || test $? -eq 1
```

### 7. Evaluate

Compare **before vs after** using `.oracles/reports/…/index.md` and top fixture
`.md` files from step 6. Update scoreboard snaps only when closing a session
with intentional metric improvement.

| Signal         | Pass criteria                                                                             |
| -------------- | ----------------------------------------------------------------------------------------- |
| Gap fixtures   | Diff size down on fixtures listed for this gap in the roadmap                             |
| Aggregate diff | Down on the target profile report index, or unchanged with concentrated win               |
| Worst fixtures | No new top-10 regression in an unrelated domain                                           |
| Profile order  | Google not regressed while working AOSP; Google/AOSP not regressed while working Palantir |
| Architecture   | Policy lives in helper/analyzer/policy; rules got thinner if migration was in scope       |
| Invariants     | No missing-layout/raw-passthrough/comment-debt violations                                 |

Review by **domain**, not aggregate number alone. An aggregate win that creates
a new concentrated regression in a core chain fixture is a partial failure.

If evaluation fails: return to step 3 (reference + plan), not ad-hoc tweaks in
rules.

### 8. Close or iterate

**Close** the work unit when:

- Target gap fixtures improved materially (or architecture migration acceptance
  criteria from the roadmap phase are met)
- `mise run test` passes
- Scoreboard snaps updated if metrics improved (`INSTA_UPDATE=always` oracle
  run)
- Change is explainable from syntax shape / documented profile policy

**Iterate** when:

- Same fixture still dominates diff with the same failure mode
- Improvement traded away on a sibling fixture in the same gap category

When closing, leave a short session note (in the PR or chat):

```markdown
## Milestone 8: [work unit]

**Gap / migration:** … **Profile:** … **Scoreboard:** aggregate X→Y, exact match
A%→B% **Fixtures:** … (diff sizes) **Code:** … **Architecture:** … **Next gap:**
…
```

Update `.agents/docs/formatter-milestone-8-coverage-roadmap.md` only when
baseline numbers or gap status materially changed (new helper landed, gap
closed, new pressure point). Do not rewrite the roadmap every session.

## Work unit catalog (quick pick)

Shared gaps (all profiles) — see roadmap for detail:

1. Selector / fluent chain breaking → `analyzers/chains.rs`, `helpers/chains.rs`
2. Comment + blank-line preservation → `comments.rs`, `helpers/bodies.rs`
3. Argument-list fill → `helpers/lists.rs`
4. Control-flow blocks / inline `if` → `helpers/bodies.rs`,
   `rules/statements.rs`
5. `extends` / `implements` indent → `helpers/type_declarations.rs`
6. Array initializers → future `helpers/expressions.rs`
7. Switch patterns/guards → switch helpers in statements/bodies
8. Generics / ternary / annotations → types, expressions, annotations helpers

Profile-specific:

- **AOSP** — wire `policy.continuation_indent_levels()`, import groups; same
  fixtures as Google
- **Palantir** — 80-col last-dot / `marked_break`, reluctant `=` break, lambda
  args, text blocks (`helpers/literals.rs`)

Architecture migrations (schedule explicitly; each is a full landing, not prep
work):

- Extract `helpers/expressions.rs` and migrate assignment/binary/lambda/array
  policy out of `rules/expressions.rs` and `layout.rs`
- Extract `helpers/imports.rs` and migrate import-section policy out of
  `rules/compilation_unit.rs`
- Shrink `declarations.rs` by completing callable/type header migration
- Finish comment placement in blocked domains (oracle + existing debt tests
  only)

## Anti-patterns

Stop and replan if you find yourself:

- Taking a **too-small bite** — one fixture, one callsite, rename-only move,
  stub helper with no migrated policy
- Editing oracle `.snap` files by hand without running tests
- Using `mise run test-update` when you only meant to refresh oracle scoreboards
- Adding **new unit tests** for policy already covered by oracle fixtures
- Adding profile branches in `rules/` instead of `policy.rs`
- Fixing one fixture by special-casing its filename
- Expanding `layout.rs` with Java-domain policy instead of a helper
- Chasing 100% Palantir before shared Google gaps are exhausted (unless user
  directs otherwise)

## Related docs

- `.agents/docs/formatter-milestone-8-coverage-roadmap.md` — source of truth
- `.agents/docs/formatter-plan.md` — milestone context
- `.oracles/reports/java/**/index.md` — per-profile mismatch indexes
- `crates/jolt_java_fmt/src/rules/mod.rs` — formatter rule contract
