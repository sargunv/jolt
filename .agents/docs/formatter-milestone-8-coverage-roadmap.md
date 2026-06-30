# Formatter Abstraction Layer Improvement Plan

The Java formatter routes parser-clean syntax accepted by `jolt_java_syntax`
through real layout rules. The remaining work is not a different formatter
architecture. It is stronger Java-specific layout abstractions so rule modules
describe syntax roles and the helper layer owns formatting policy, without
turning every oracle mismatch into a local case patch.

This document replaces the old coverage-blocker inventory. Its focus is the
abstraction layer between Java CST wrappers and the shared document IR.

## North Star

Milestone 8 is complete when the pinned oracle corpora reach **100% exact
match**, profile by profile, in this order:

1. **Google** — `google-java-format` profile on the pinned Google corpus
2. **AOSP** — AOSP profile on the same corpus
3. **Palantir** — Palantir profile on the pinned Palantir corpus

Work on a later profile should not regress an earlier one. Idempotence holds for
passing fixtures. Parser-clean syntax formats through real rules with no
missing-layout exits, raw source passthrough, or silent comment drops.
`mise run test` passes.

## Current Status

### Architecture

```text
source text
  -> jolt_java_syntax parser
  -> lossless CST + wrapper accessors
  -> Java rule modules
  -> Java layout helpers and analyzers
  -> shared document IR
  -> shared renderer
```

Layering in code:

```text
shared IR, builders, and renderer
  -> Java formatter context, profile policy, comments, and trivia services
  -> Java node rules with an explicit formatting contract
  -> Java analyzers and layout helpers
```

### Coverage baseline

Parser-clean fixtures format end-to-end with zero missing-rule blockers across
all three oracle profiles. Raw source passthrough, unsupported-layout exits, and
late remaining-comment appendage are gone.

Oracle alignment is the active gap. Current scoreboards (from pinned snapshots):

| Profile  | Exact match | Aggregate diff | Worst fixture          |
| -------- | ----------- | -------------- | ---------------------- |
| Google   | 61.05%      | 403            | `B20128760.java` (18)  |
| AOSP     | 59.61%      | 732            | `B24909927.java` (205) |
| Palantir | 42.85%      | 3,755          | `B24909927.java` (916) |

Selector chains, declaration headers, and expression/list fill dominate the
largest per-file diffs. `B24909927.java` dropped off the Google worst-10 after
chain policy work but remains the top AOSP/Palantir mismatch. Palantir aggregate
diff is now mostly profile-specific chain, lambda, and assignment policy rather
than missing formatter coverage.

### What is in place

- **Rule contract** documented in `rules/mod.rs`: identify slots, collect
  comments, delegate to helpers, reject unhandled trivia, return a real `Doc`.
- **Profile policy** in `policy.rs` (`JavaFormatPolicy`): continuation
  indentation, AOSP static-import separation, selector-chain breaking rules.
  Direct `JavaFormatProfile` checks are confined to policy, options, and tests.
- **Generic list mechanics** in `helpers/separated.rs`.
- **Java list helpers** in `helpers/lists.rs`: argument, formal, type
  argument/parameter, keyword-prefixed clause lists, with comment-aware item
  formatting.
- **Declaration helpers** in `helpers/callables.rs` and
  `helpers/type_declarations.rs`, adopted from `declarations.rs`.
- **Body helpers** in `helpers/bodies.rs`: statement blocks, constructor bodies,
  class/interface/enum bodies.
- **Selector chain analyzer and helper** in `analyzers/chains.rs` and
  `helpers/chains.rs`: flattening, metadata, `ChainGroup`s, staged layout
  alternatives, profile-aware breaking.
- **Comment ownership** in `context.rs` and `comments.rs`: leading, trailing,
  dangling, inline, separator, and list-item buckets; `reject_unhandled_*`
  guards. Rewrite policy lives in `helpers/comments.rs`. guards; end-of-format
  unconsumed-trivia diagnostics.
- **Domain rule split** under `rules/`: compilation units, declarations,
  expressions, statements, annotations, types, names, tokens.
- **Oracle scoreboards** for Google, AOSP, and Palantir profiles.
- **Focused unit tests** in `rules/tests.rs` for lists, chains, declarations,
  comments, and narrow-width wrapping.

### Active pressure points

- `declarations.rs`, `expressions.rs`, and `statements.rs` are still large.
  Helpers exist, but rules still assemble many slots and own comment wiring
  locally.
- `layout.rs` still holds expression-shaped helpers (`assignment_expression`,
  etc.) that belong in a future `helpers/expressions.rs`.
- Comment placement is incomplete in annotation arguments, inline annotation
  positions, some header-boundary positions, and a few branch/else shapes. These
  currently block formatting rather than placing comments.
- Selector chain policy infrastructure exists, but oracle alignment for long
  fluent chains—especially under Palantir—is still a large shared mismatch
  domain.
- **Break-selection architecture debt:** helpers eagerly embed fully formatted
  subtrees with nested `best_fitting` nodes. Deeply nested call trees (e.g.
  `B24909927.java`) can make the renderer's fit pass exponential in nesting
  depth. A chain hotfix skips `best_fitting` for `ChainRole::NestedArgument`;
  the same class of problem affects lists, assignments, array initializers, and
  Palantir last-dot policy. See
  [Global break selection](#global-break-selection-architecture-debt).
- Import section grouping and blank-line policy now live in
  `helpers/imports.rs`; `rules/compilation_unit.rs` still owns
  package/module/member traversal.

See [Oracle compatibility gaps](#oracle-compatibility-gaps) for a report-derived
breakdown of what the mismatches actually are.

## Oracle Compatibility Gaps

Every oracle mismatch is a **layout policy gap**, not a missing syntax rule.
Missing-rule blockers are zero on all profiles. Detailed per-file diffs live
under `.oracles/reports/java/`; regenerate them with the oracle fixture test
harness.

Report indexes:

- Google: `.oracles/reports/java/google-java-format/google/index.md`
- AOSP: `.oracles/reports/java/google-java-format/aosp/index.md`
- Palantir: `.oracles/reports/java/palantir-java-format/palantir/index.md`

Google has **81** mismatching formatted files; AOSP has **84** on the same
corpus. Palantir has **128** mismatches on its own corpus. Aggregate diff grows
profile-to-profile because wrong break decisions cost more lines at 4-space
indent, and Palantir adds chain, lambda, and assignment policy beyond
google-java-format.

| Profile  | Mismatching | Aggregate diff | vs Google |
| -------- | ----------- | -------------- | --------- |
| Google   | 81 / 208    | 403            | —         |
| AOSP     | 84 / 208    | 732            | +329      |
| Palantir | 128 / 224   | 3,755          | +3,352    |

Gap categories overlap in practice; impact estimates below are directional, not
additive to 100%.

### Shared gaps (all three profiles)

These dominate Google and AOSP and account for roughly half of Palantir's diff.
Fixing them moves all scoreboards together.

#### 1. Selector / fluent chain breaking — largest bucket (~30% of Google diff)

**What differs:** Where the formatter breaks `.method()` chains — whether
`Receiver.method()` stays on one line, whether nested builders split receiver
and selector, break depth on field-prefix runs, and cast/paren receiver
grouping.

**Top fixtures:** `B24909927.java` (88 / 205 / 916), `B20128760.java` (95 / 119
/ 116), `B20701054.java` (85 / 84 / 215), `M.java`, `B21305044.java`,
`B24202287.java`

**Example (`B24909927.java`):** Oracle keeps nested selectors on separate lines;
Jolt merges receiver and first selector:

```diff
- XxxxxxxXxxx
-         .xxxXxxxxxx()
+ XxxxxxxXxxx.xxxXxxxxxx()
```

At AOSP 4-space indent, every such decision multiplies diff lines.

**Fix locus:** `analyzers/chains.rs`, `helpers/chains.rs`, expression chain
collection. Several helpers hardcode `CONTINUATION_INDENT_LEVELS = 2` instead of
`policy.continuation_indent_levels()`.

#### 2. Comment and vertical-whitespace preservation (~18–22%)

**What differs:** google-java-format preserves interior column alignment in
block comments, trailing spaces on comment lines, blank lines between members
and switch cases, and malformed-javadoc structure. Jolt normalizes or reflows
them.

**Top fixtures:** `B24543625.java` (~91 lines on Google, almost entirely this
category), `B24702438.java`, `B20535125.java`, `A.java`

**Example (`B24543625.java`):** ASCII-art block comment columns get
trailing-space padding; interior alignment is rewritten instead of preserved.

**Fix locus:** `comments.rs`, `context.rs`, body and member blank-line policy.

#### 3. Argument-list fill vs one-per-line (~12–16%)

**What differs:** Whether short arg lists pack on one line, break after `(`, use
paired rows in annotation arrays, or force one arg per line at a width
threshold.

**Top fixtures:** `B22815364.java`, `C.java`, `M.java`, `B24202287.java`,
`PairedArguments.java`

**Fix locus:** `helpers/lists.rs` fill and break policy — one cohesive helper
change, many call sites.

**Google baseline (2025-06-30):** aggregate 2112→**2049** via GJF
`isFormatMethod` in `analyzers/format_strings.rs` and
`format_method_argument_list` in `helpers/lists.rs` (format string on its own
continuation line; remaining args filled, not one-per-line). `B26207047.java`
64→**15**. General long-arg lists still use one-per-line when any item exceeds
`MAX_ITEM_LENGTH_FOR_FILLING`; switching all arg lists to fill regressed
`M.java` and aggregate.

#### 4. Control-flow block compaction (~10–14%)

**What differs in two directions:**

- Oracle **expands** empty blocks (`try {} catch (E e) {}` on one line); Jolt
  collapses blank lines inside blocks differently (`B20535125.java`).
- Oracle **keeps** single-statement `if (true) stmt;` inline when within width;
  Jolt always breaks the body (`B20569245.java`, identical diff size on Google
  and AOSP).

**Fix locus:** `helpers/bodies.rs`, statement rules for if, try, and for.

#### 5. Type declaration headers — `extends` / `implements` (~4–5%)

**What differs:** Continuation lines indented under the keyword vs flush at
class indent.

**Top fixture:** `B28066276.java` (63 lines, same pattern on all profiles)

**Fix locus:** `helpers/type_declarations.rs` — relatively small, high-leverage.

**Google baseline (2025-06-30):** aggregate 2049→**2026** via GJF
`visitClassDeclaration` / `classDeclarationTypeList` in
`helpers/type_declarations.rs` (soft `breakToFill` between name and clauses) and
`type_clause_list` in `helpers/lists.rs` (first type on keyword line,
continuations indented under keyword). `B28066276.java` 63→**7**.

#### 6. Array / initializer layout (~7–9%) — **partial**

**What differs:** Fill width for large `int[]` literals, compact `{0,1}` rows in
2D arrays vs expanded blocks, annotation array row grouping.

**Top fixtures:** `A.java` (75→**42**), `B22815364.java` (60→**10**),
`LiteralReflow.java` (30, unchanged)

**Fix locus:** `analyzers/array_initializers.rs` (GJF `argumentsAreTabular`) and
`helpers/array_initializers.rs` (tabular rows, INDEPENDENT vs UNIFIED fill via
`best_fitting`, array-creation dim breaks). Google aggregate **1987→1846** on
this landing. Remaining: multi-dim `[][]` trailing breaks (`arrayWithLongName`),
A.java blank-line/ternary/chain-index debt in other domains.

#### 7. Switch formatting (~4–5%)

**What differs:** `case Type var when expr` line breaks, record-pattern
component layout, continued case label indent, comments after `->`.

**Top fixtures:** `SwitchGuardClause.java`, `ExpressionSwitch.java`,
`SwitchRecord.java`, `SwitchComment.java`

**Fix locus:** switch block helpers in `rules/statements.rs` and
`helpers/bodies.rs`.

#### 8. Generics, ternary, and annotations (medium tail)

- **Generics:** split `List<Pair<…>>` from initializer, multiline diamond
  (`M.java`, `F.java`, `B21305044.java`)
- **Ternary:** nested `? :` indent vs flattening (`F.java`, `C.java`)
- **Annotations:** `@Inject int x` inline vs annotation on its own line; stacked
  type-use annotations (`B24702438.java`, `TypeAnnotations.java`,
  `ParameterComment.java`)

**Fix locus:** type rules, expression helpers, annotation helpers.

### AOSP-specific amplification

AOSP is not a different failure set from Google — it is the **same 137
fixtures** with **+262 aggregate diff lines**, mostly the same bugs at 4-space
indent.

| Fixture          | Google diff | AOSP diff | Extra cause                                       |
| ---------------- | ----------- | --------- | ------------------------------------------------- |
| `B24909927.java` | 88          | 205       | Chain flattening plus deeper 4-space body indent  |
| `M.java`         | 82          | 114       | Shared wrapping; wider indent shifts break points |
| `A.java`         | 71          | 80        | Same array/ternary bugs, more lines at 4-space    |

**Already in code:**

- `JavaFormatPolicy::separates_static_import_section()` — blank lines between
  import prefix groups (visible in `M.java` vs Google)
- `indent_width: 4` for AOSP in `options.rs`

**AOSP wiring gaps:**

- `helpers/chains.rs`, `helpers/callables.rs`, and `layout.rs` hardcode
  continuation indent instead of reading policy
- Chain break decisions must respect the 4-space line budget, not inherit
  2-space Google break points

Closing shared layout bugs moves both Google and AOSP together. AOSP-only wins
are narrower: wire policy through all layout paths and keep import-group
separation tied to `JavaFormatPolicy`.

### Palantir-specific gaps (~41–43% of 4,886 lines)

Palantir shares all Google-style gaps above but adds a large chain, lambda, and
assignment policy layer. The 30 `palantir-*.java` fixtures account for ~648 diff
lines (13%); `B24909927.java` alone is **916**.

#### Palantir chain policy

**What differs from Google-style breaking:**

- Keep chains flat longer; break only when the **last `.` before a wrap exceeds
  ~80 columns** even if the full line fits in 120
- Builder chains: count-based thresholds before wrapping (`.add()` × N)
- Nested-argument heads: preserve `ImmutableList.of(builder()…)` inline

**Example (`palantir-deeply-nested-calls.java`):** Oracle keeps nested builders
inline in arguments; Jolt breaks before `MagicConfigV1.builder()` and explodes
each `ImmutableList.of(...)` argument.

**Missing in Jolt:** `marked_break` IR usage for last-dot column limits.
`policy.rs` sets `selector_chain_breaks_before_first_selector = false` for
Palantir, but `helpers/chains.rs` still applies width heuristics without 80-col
enforcement. See palantir-java-format references for `Level.java`,
`BreakBehaviour.java`, and `LastLevelBreakability.java`.

#### Reluctant `=` break

Palantir keeps `Type x = Receiver.chain()` on one line; Jolt breaks before the
RHS. Hits `palantir-4`, `palantir-5`, `palantir-6`, `palantir-11`,
`palantir-gcv-1`, `palantir-lambda-multiline-arg.java`, and many top B-fixtures.

**Fix locus:** assignment layout in `layout.rs` or future
`helpers/expressions.rs`, driven by Palantir policy.

#### Lambda argument policy

Palantir fixtures encode when a lambda stays on the call line vs breaks:

- inline block lambdas in simple calls (`palantir-break-lambda-arg.java`)
- break when the lambda body contains chains
  (`palantir-lambda-inlining-prefers-break.java`)
- expression-lambda arg packing (`palantir-expression-lambda-1.java`,
  `palantir-expression-lambdas.java`)
- cast lambdas (`palantir-lamda-cast.java`)

**Fix locus:** new `helpers/lambdas.rs` or extension of `helpers/callables.rs`.

#### Text blocks — `RSL.java` (310 lines, #2 Palantir mismatch)

Palantir **preserves text-block content verbatim** (no interior normalization).
Jolt breaks after `=`, reflows content, and mis-indents closing `"""`.

**Fix locus:** `helpers/literals.rs` or a dedicated text-block helper.

#### Palantir vs shared GJF gaps

Rough split of the 4,886-line aggregate diff:

| Bucket                                         | Est. share |
| ---------------------------------------------- | ---------- |
| Chain / lambda / assignment-inline policy      | ~41–43%    |
| Shared GJF-style (throws, blanks, comments, …) | ~18–22%    |
| Text blocks and string-concat preservation     | ~7–8%      |
| Other structural (generics, switch, imports)   | ~27–33%    |

Palantir is not "Google with 4-space indent." It is a different chain and lambda
policy surface even when the underlying syntax is the same.

### Impact-ordered fix map

| Priority | Gap                              | Est. relief        | Primary code                               |
| -------- | -------------------------------- | ------------------ | ------------------------------------------ |
| 1        | Selector chain breaking (shared) | ~800+ all profiles | `analyzers/chains.rs`, `helpers/chains.rs` |
| 2        | Palantir 80-col last-dot         | ~900+ Palantir     | `helpers/chains.rs`, `marked_break` in IR  |
| 3        | Comment interior + blank lines   | ~200+              | `comments.rs`, body blank-line policy      |
| 4        | Argument list fill               | ~300+ spread       | `helpers/lists.rs`                         |
| —        | **Global break selection**       | cross-cutting      | `jolt_fmt_ir`, helpers across domains      |
| 5        | Palantir `=` + lambda args       | ~500+ Palantir     | lambda/assignment policy helpers           |
| 6        | Text block preservation          | ~310 (`RSL.java`)  | `helpers/literals.rs`                      |
| 7        | `extends` / `implements` indent  | ~60+               | `helpers/type_declarations.rs`             |
| 8        | Empty blocks / inline `if`       | ~100+              | `helpers/bodies.rs`, if/try rules          |
| 9        | Array initializers               | ~80+               | expression/array helper                    |
| 10       | Switch patterns/guards           | ~50+               | switch formatting                          |

### What each north-star profile requires

**100% Google** closes gaps 1, 3, 4, 5, 7, 8, 9, and 10 — mostly shared layout
policy, not new syntax coverage.

**100% AOSP** is Google parity plus correct 4-space continuation wiring and
import-group blank lines. Do not treat AOSP as a separate rule set.

**100% Palantir** requires everything above **plus** Palantir chain engine
(80-col rule, nested-argument heads, reluctant assignment breaks, lambda-arg
helpers, text-block preservation).

### Representative reports to inspect

| Category               | Report                                                                                     |
| ---------------------- | ------------------------------------------------------------------------------------------ |
| Deep fluent chains     | `.oracles/reports/java/google-java-format/google/B24909927.java.md`                        |
| Chain + declarations   | `.oracles/reports/java/google-java-format/google/B20128760.java.md`                        |
| Comments               | `.oracles/reports/java/google-java-format/google/B24543625.java.md`                        |
| Empty blocks / blanks  | `.oracles/reports/java/google-java-format/google/B20535125.java.md`                        |
| `extends`/`implements` | `.oracles/reports/java/google-java-format/google/B28066276.java.md`                        |
| Switch guards          | `.oracles/reports/java/google-java-format/google/SwitchGuardClause.java.md`                |
| Palantir nested calls  | `.oracles/reports/java/palantir-java-format/palantir/palantir-deeply-nested-calls.java.md` |
| Text blocks            | `.oracles/reports/java/palantir-java-format/palantir/RSL.java.md`                          |
| AOSP chain inflation   | `.oracles/reports/java/google-java-format/aosp/B24909927.java.md`                          |

## Reference Project Lessons

Jolt should copy formatter **boundaries** from the projects below, not their
exact APIs.

Emulate:

- a small language-neutral document algebra,
- a language-specific syntax-to-document layer,
- formatter context services for comments, source positions, and options,
- named helpers for lists, chains, declarations, expressions, and bodies,
- source-aware comment classification before rendering,
- analyzer objects for complex syntax shapes such as selector chains.

Do not emulate:

- AST-only source models,
- AST mutation for comment attachment,
- JavaScript/Python-specific naming heuristics without Java oracle evidence,
- plugin API shapes that do not fit Jolt,
- arbitrary style configuration,
- ignored-source passthrough as a coverage mechanism.

Oracle compatibility targets are **google-java-format** and
**palantir-java-format**. Prettier, Oxc, and Ruff are architectural references
for IR shape, comment ownership, and rule layering.

### google-java-format

Primary oracle for the Google and AOSP profiles.

- [Formatting pipeline (`Formatter.java`)](https://github.com/google/google-java-format/blob/master/core/src/main/java/com/google/googlejavaformat/java/Formatter.java)
  — visitor → `OpsBuilder` → `DocBuilder` → break selection → output; the
  end-to-end shape Jolt mirrors with CST rules and a shared renderer.
- [Document IR (`Doc.java`)](https://github.com/google/google-java-format/blob/master/core/src/main/java/com/google/googlejavaformat/Doc.java)
  — Oppen-style document tree and width-driven break selection; reference for
  `jolt_fmt_ir::Doc`.
- [Op stream builder (`OpsBuilder.java`)](https://github.com/google/google-java-format/blob/master/core/src/main/java/com/google/googlejavaformat/OpsBuilder.java)
  — imperative layout emission before tree lowering; analogue to rule modules
  building `Doc` via helpers.
- [Ops-to-doc lowering (`DocBuilder.java`)](https://github.com/google/google-java-format/blob/master/core/src/main/java/com/google/googlejavaformat/DocBuilder.java)
  — converts flat ops into nested levels; two-phase emit-then-structure split.
- [Java layout visitor (`JavaInputAstVisitor.java`)](https://github.com/google/google-java-format/blob/master/core/src/main/java/com/google/googlejavaformat/java/JavaInputAstVisitor.java)
  — monolithic per-node policy; reference for what Jolt splits across `rules/`
  and `helpers/`.
- [Method chains (`visitDot`)](https://github.com/google/google-java-format/blob/master/core/src/main/java/com/google/googlejavaformat/java/JavaInputAstVisitor.java#L2998)
  — flattens selector syntax and chooses break-before-dot layout; primary oracle
  for `analyzers/chains.rs` and `helpers/chains.rs`.
- [Declaration headers (`declareOne`)](https://github.com/google/google-java-format/blob/master/core/src/main/java/com/google/googlejavaformat/java/JavaInputAstVisitor.java#L3614)
  — type/name/initializer alignment and vertical annotation breaks; reference
  for `helpers/callables.rs` and declaration rules.
- [Comma-separated lists (`addArguments`)](https://github.com/google/google-java-format/blob/master/core/src/main/java/com/google/googlejavaformat/java/JavaInputAstVisitor.java#L3386)
  — tabular pairs, fill modes, and special argument-list layout; reference for
  `helpers/lists.rs`.
- [Input token and comment model (`JavaInput.java`)](https://github.com/google/google-java-format/blob/master/core/src/main/java/com/google/googlejavaformat/java/JavaInput.java)
  — token and trivia sequencing; reference for lossless CST comment ranges in
  the rule contract.
- [Comment rewriting (`JavaCommentsHelper.java`)](https://github.com/google/google-java-format/blob/master/core/src/main/java/com/google/googlejavaformat/java/JavaCommentsHelper.java)
  — comment rewrite during break computation; reference for comment wrappers and
  rejecting unhandled trivia.
- [Import ordering (`ImportOrderer.java`)](https://github.com/google/google-java-format/blob/master/core/src/main/java/com/google/googlejavaformat/java/ImportOrderer.java)
  — standalone import pass, not part of core layout; validates keeping import
  policy out of layout rules.
- [Multi-pass CLI (`FormatFileCallable.java`)](https://github.com/google/google-java-format/blob/master/core/src/main/java/com/google/googlejavaformat/java/FormatFileCallable.java)
  — chains format → remove unused imports → reorder; separate passes, not one
  visitor.

### palantir-java-format

Primary oracle for the Palantir profile. Fork of google-java-format with
systematic chain, indentation, and lambda policy differences.

- [Style profiles (`JavaFormatterOptions.java`)](https://github.com/palantir/palantir-java-format/blob/develop/palantir-java-format-spi/src/main/java/com/palantir/javaformat/java/JavaFormatterOptions.java)
  — Google, AOSP, and Palantir profile constants; Palantir uses 2× indent with a
  120-column limit.
- [Chain-breaking rationale (README)](https://github.com/palantir/palantir-java-format/blob/develop/README.md)
  — documents Palantir's 80-column cap on the last chain dot even when the full
  line fits.
- [AST visitor hub (`JavaInputAstVisitor.java`)](https://github.com/palantir/palantir-java-format/blob/develop/palantir-java-format/src/main/java/com/palantir/javaformat/java/JavaInputAstVisitor.java)
  — `METHOD_CHAIN_COLUMN_LIMIT` and Palantir-specific chain, lambda, and
  argument formatting.
- [Column-limit enforcement (`Level.java`)](https://github.com/palantir/palantir-java-format/blob/develop/palantir-java-format/src/main/java/com/palantir/javaformat/doc/Level.java)
  — rejects inlining when the last chain dot exceeds
  `columnLimitBeforeLastBreak`.
- [Partial-inlining guards (`PartialInlineability.java`)](https://github.com/palantir/palantir-java-format/blob/develop/palantir-java-format/src/main/java/com/palantir/javaformat/PartialInlineability.java)
  — when a level may be partially inlined without degenerate FQN breaks.
- [Inline-chain termination (`LastLevelBreakability.java`)](https://github.com/palantir/palantir-java-format/blob/develop/palantir-java-format/src/main/java/com/palantir/javaformat/LastLevelBreakability.java)
  — heuristics for stopping inlining in nested chain levels.
- [Break strategies (`BreakBehaviour.java`)](https://github.com/palantir/palantir-java-format/blob/develop/palantir-java-format/src/main/java/com/palantir/javaformat/BreakBehaviour.java)
  — Palantir break behaviours (`preferBreakingLastInnerLevel`, etc.) behind more
  aggressive inlining than upstream.
- [Fixture: chains and lambdas (`palantir-chains-lambdas.output`)](https://github.com/palantir/palantir-java-format/blob/develop/palantir-java-format/src/test/resources/com/palantir/javaformat/java/testdata/palantir-chains-lambdas.output)
  — golden output for chained calls with lambda arguments.
- [Fixture: deeply nested calls (`palantir-deeply-nested-calls.output`)](https://github.com/palantir/palantir-java-format/blob/develop/palantir-java-format/src/test/resources/com/palantir/javaformat/java/testdata/palantir-deeply-nested-calls.output)
  — golden output for deeply nested call trees under Palantir.

### Prettier

Architectural reference for language-neutral IR and comment attachment
boundaries.

- [Doc algebra spec (`commands.md`)](https://github.com/prettier/prettier/blob/main/commands.md)
  — canonical description of `group`, `ifBreak`, `fill`, `lineSuffix`, and line
  variants.
- [Exported Doc types (`public.d.ts`)](https://github.com/prettier/prettier/blob/main/src/document/public.d.ts)
  — typed IR contract keeping builders and printer separate from language
  plugins.
- [`group` builder](https://github.com/prettier/prettier/blob/main/src/document/builders/group.js)
  — core break-or-flat primitive.
- [`ifBreak` builder](https://github.com/prettier/prettier/blob/main/src/document/builders/if-break.js)
  — conditional docs chosen after break decisions.
- [`fill` builder](https://github.com/prettier/prettier/blob/main/src/document/builders/fill.js)
  — separator-aware wrapping distinct from all-or-nothing groups.
- [`lineSuffix` builder](https://github.com/prettier/prettier/blob/main/src/document/builders/line-suffix.js)
  — defers trailing content to end-of-line without scanning output.
- [Doc printer (`printer.js`)](https://github.com/prettier/prettier/blob/main/src/document/printer/printer.js)
  — pure layout engine; comment-free.
- [Comment attachment (`attach.js`)](https://github.com/prettier/prettier/blob/main/src/main/comments/attach.js)
  — pre-print pass decorating nodes with leading/trailing/dangling comments.
- [Comment print layer (`print.js`)](https://github.com/prettier/prettier/blob/main/src/main/comments/print.js)
  — wraps node docs with comment docs via `lineSuffix` for end-of-line comments.
- [AST → Doc pipeline (`ast-to-doc.js`)](https://github.com/prettier/prettier/blob/main/src/main/ast-to-doc.js)
  — attach → print → wrap → render layering.

### Oxc

Architectural reference for span-ordered comment cursors and core/language
split.

- [Core vs language boundary (`AGENTS.md`)](https://github.com/oxc-project/oxc/blob/main/crates/oxc_formatter_core/AGENTS.md)
  — language-agnostic IR/printer vs consumer-owned comments and grammar rules.
- [FormatElement IR (`mod.rs`)](https://github.com/oxc-project/oxc/blob/main/crates/oxc_formatter_core/src/format_element/mod.rs)
  — language-neutral document elements including line-suffix boundaries.
- [IR builders (`builders.rs`)](https://github.com/oxc-project/oxc/blob/main/crates/oxc_formatter_core/src/builders.rs)
  — shared helpers building IR over a generic context type.
- [Printer (`mod.rs`)](https://github.com/oxc-project/oxc/blob/main/crates/oxc_formatter_core/src/printer/mod.rs)
  — IR-to-text second stage.
- [FormatContext traits (`traits.rs`)](https://github.com/oxc-project/oxc/blob/main/crates/oxc_formatter_core/src/traits.rs)
  — core traits without language semantics.
- [FormatState (`state.rs`)](https://github.com/oxc-project/oxc/blob/main/crates/oxc_formatter_core/src/state.rs)
  — per-document context holder for an entire format pass.
- [Comment cursor (`comments.rs`)](https://github.com/oxc-project/oxc/blob/main/crates/oxc_formatter/src/formatter/comments.rs)
  — on-demand cursor with `printed_count`; primary reference for Jolt's
  claimed-comment model.
- [Trivia formatting (`trivia.rs`)](https://github.com/oxc-project/oxc/blob/main/crates/oxc_formatter/src/formatter/trivia.rs)
  — leading/trailing/dangling emission and cursor advancement.
- [JS format context (`context.rs`)](https://github.com/oxc-project/oxc/blob/main/crates/oxc_formatter/src/formatter/context.rs)
  — comments live in the language crate, not the core.
- [Line suffix printing (`line_suffixes.rs`)](https://github.com/oxc-project/oxc/blob/main/crates/oxc_formatter_core/src/printer/line_suffixes.rs)
  — defer trailing trivia until line break or boundary.

### Ruff

Architectural reference for rule contracts and comment bucket accounting.

- [Formatter crate overview (`lib.rs`)](https://github.com/astral-sh/ruff/blob/main/crates/ruff_formatter/src/lib.rs)
  — `FormatRule`, `FormatContext`, and IR → print pipeline.
- [Formatter session API (`formatter.rs`)](https://github.com/astral-sh/ruff/blob/main/crates/ruff_formatter/src/formatter.rs)
  — context-aware formatting with shared list/break builders.
- [Layout builders (`builders.rs`)](https://github.com/astral-sh/ruff/blob/main/crates/ruff_formatter/src/builders.rs)
  — breaking/flat layout expressed in IR, not in each rule.
- [IR buffer (`buffer.rs`)](https://github.com/astral-sh/ruff/blob/main/crates/ruff_formatter/src/buffer.rs)
  — how rules accumulate `FormatElement`s during formatting.
- [Printer (`mod.rs`)](https://github.com/astral-sh/ruff/blob/main/crates/ruff_formatter/src/printer/mod.rs)
  — width-aware breaking from IR alone.
- [FormatElement document (`document.rs`)](https://github.com/astral-sh/ruff/blob/main/crates/ruff_formatter/src/format_element/document.rs)
  — tagged document structure rules emit.
- [`FormatNodeRule` contract (`lib.rs`)](https://github.com/astral-sh/ruff/blob/main/crates/ruff_python_formatter/src/lib.rs)
  — leading → fields → trailing ordering; Python analogue of Jolt's rule
  contract.
- [Comment bucket model (`comments/mod.rs`)](https://github.com/astral-sh/ruff/blob/main/crates/ruff_python_formatter/src/comments/mod.rs)
  — leading/dangling/trailing ownership and `assert_all_formatted`.
- [Per-node comment map (`map.rs`)](https://github.com/astral-sh/ruff/blob/main/crates/ruff_python_formatter/src/comments/map.rs)
  — bucket storage and lookup per syntax node.
- [Comment placement heuristics (`placement.rs`)](https://github.com/astral-sh/ruff/blob/main/crates/ruff_python_formatter/src/comments/placement.rs)
  — syntax-aware reassignment when default buckets are wrong.

## Design Principles

### Keep The Renderer Language-Neutral

`jolt_fmt_ir` should not learn Java concepts. It may grow general document
features if a Java helper proves the need, but concepts such as method chains,
throws clauses, type parameters, imports, annotations, and switch labels belong
in `jolt_java_fmt`.

Add renderer features only when the helper layer cannot express a broad policy
with existing primitives. Preference order:

1. Compose existing `Doc` primitives in a Java helper.
2. Add a small Java helper that names the formatting policy.
3. Add a general IR primitive only after multiple helpers need the same renderer
   behavior.

Keep generic formatter ergonomics separate from Java policy. A reusable
delimited or separated-list utility can live below Java-specific argument,
parameter, annotation, or type-list policy. A selector-chain classifier, by
contrast, is Java formatter policy and should not leak into `jolt_fmt_ir`.

### Make Rule Modules Structural

Rule modules should answer grammar questions:

- What kind of syntax node is this?
- Which child nodes and source ranges belong to each semantic slot?
- Which helper should format this Java construct?
- Which associated comments must be emitted or delegated?

Rule modules should avoid owning low-level wrapping decisions such as whether a
comma list fills independently, whether a selector chain breaks before the first
selector, or how a declaration header aligns when a type breaks.

### Formatter Rule Contract

Each Java node rule follows the same contract:

1. Identify the node's source range and grammar slots through CST wrappers.
2. Ask the formatter context/comment service for comments associated with the
   node or slot.
3. Format child slots through rule functions or domain helpers.
4. Emit leading and trailing comments through shared wrappers.
5. Explicitly place, delegate, or reject dangling and inline comments.
6. Return a real `Doc` for parser-clean syntax.

Raw source passthrough is not a formatter rule. For parser-clean syntax,
returning a document made from an arbitrary node's original source text hides
missing layout coverage. Source text may be used only at the token/literal
boundary where preserving the token spelling is the formatting rule.

### Preserve Context Explicitly

Prettier uses `AstPath` to make parent, sibling, and list-position-sensitive
decisions. Jolt does not need a generic path object, but it does need an
explicit context story.

CST wrappers should expose grammar roles and source ranges. Formatter rules and
helpers may pass narrow context objects when layout depends on ancestry or
position, for example:

- whether an expression is a nested argument,
- whether a selector chain is itself a receiver,
- whether a node is first, last, or separated by comments in a list,
- whether a declaration appears in a class, interface, enum, annotation body,
  compact compilation unit, or local block,
- whether parenthesization is required by parent precedence.

Avoid hiding layout policy in CST wrappers. Wrappers expose source facts;
formatter rules and helpers decide layout.

### Put Policy In Named Helpers

The helper layer should have Java-domain names. A future reader should be able
to scan a rule and see Java formatting intent rather than raw document-builder
plumbing.

Good helper names:

- `callable_header`
- `formal_parameter_list`
- `argument_list`
- `type_argument_list`
- `type_parameter_list`
- `selector_chain`
- `binary_expression_chain`
- `assignment_expression`
- `annotation_group`
- `declaration_modifiers`
- `class_body_members`
- `statement_block`
- `switch_block`
- `import_section`

Prefer "helper", "rule", "analyzer", or "formatter" for Java-domain code. Save
"builder" for generic document construction APIs if Jolt grows a Ruff/Oxc-like
builder facade.

Poor long-term rule-module patterns:

- open-coded `concat([text("("), soft_line(), ...])`
- open-coded comma separators in multiple domains
- width-sensitive conditionals in rule modules
- profile checks repeated near leaf syntax formatting
- ad hoc comment fallbacks that append source text outside the owning rule

### Treat Comments As Layout Inputs

Comments are not an afterthought. A helper that formats a list, block, chain, or
declaration should have an answer for comments inside that construct.

The comment model has two distinct phases:

1. Classify and associate comments from source positions.
2. Render associated comments through node rules and helpers.

Source-position classification distinguishes own-line, end-of-line, and
inline/remaining comments. Ownership resolution uses preceding, enclosing, and
following ranges plus adjacency and blank-line facts.

Rendering exposes associated comments as leading, trailing line, inline block,
dangling, list-item, and before-closing-delimiter buckets. Jolt already has
`line_suffix` and `line_suffix_boundary`; trailing comments should route through
helpers consistently.

The formatter fails tests when comments are unaccounted. New supported syntax
must place comments through the owning rule or helper, not through silent
appendage or ignored trivia.

### Keep Profiles Coarse And Opinionated

Profiles are compatibility targets, not arbitrary style knobs. The formatter
should expose Google, AOSP, and Palantir profile behavior through a small number
of policy structs or helper methods, not through dozens of independent options.

Profile differences should be centralized when possible:

- indentation width,
- import section grouping,
- continuation indentation,
- chain and argument wrapping preferences,
- Palantir-specific line breaking where it is a systematic style difference.

## Global break selection (architecture debt)

### What “broken layout directly” means (and does not)

Jolt helpers still **fully format** nested expressions. “Broken” is a layout
variant (merge-first selector, broken argument list, vertical declaration
header)—not skipping formatting or passthrough.

The problem is **when** we choose flat vs broken. Today many helpers build two
complete subtrees and wrap them in `best_fitting(flat, [broken])`. The renderer
then asks “does the entire `flat` doc fit?” by walking the full tree. If that
tree already contains nested `BestFitting` nodes (because an argument was
eagerly formatted with its own flat/broken trial), fit work becomes
**exponential in nesting depth**. Formatting `B24909927.java` this way consumed
unbounded memory on a 128 GiB machine until chain nested-argument layout skipped
`best_fitting`.

### google-java-format model

GJF separates **emission** from **break selection**:

1. [`OpsBuilder`](https://github.com/google/google-java-format/blob/master/core/src/main/java/com/google/googlejavaformat/OpsBuilder.java)
   walks the AST and appends tokens plus **optional breaks** (`breakOp()`), fill
   modes (`INDEPENDENT` / `UNIFIED`), and indent levels.
2. [`DocBuilder`](https://github.com/google/google-java-format/blob/master/core/src/main/java/com/google/googlejavaformat/DocBuilder.java)
   lowers ops to a `Doc` tree once.
3. Break selection runs as a **single global pass** over that tree—nested
   expressions do not each re-run “does my whole subtree fit on one line?”

Concretely:

- **Chains** —
  [`visitDot`](https://github.com/google/google-java-format/blob/master/core/src/main/java/com/google/googlejavaformat/java/JavaInputAstVisitor.java#L2998)
  flattens a chain, classifies prefixes, emits optional breaks before dots;
  inner chains in arguments are separate `visitDot` visits, not nested subtree
  fit trials on pre-rendered docs.
- **Argument lists** —
  [`addArguments`](https://github.com/google/google-java-format/blob/master/core/src/main/java/com/google/googlejavaformat/java/JavaInputAstVisitor.java#L3386)
  optional break after `(`, then `argList` with short-item fill mode—not a
  pre-built flat `(a, b, c)` doc nested inside another list’s fit trial.

### Current Jolt model and hotfix

Jolt **eagerly** formats child slots (including whole selector chains and
argument lists) into `Doc` values, then composes them in parents. Several
helpers use `best_fitting` for width-sensitive policy:

| Location                                   | Pattern                                             |
| ------------------------------------------ | --------------------------------------------------- |
| `helpers/chains.rs`                        | flat chain vs broken chain (many policy branches)   |
| `helpers/lists.rs`                         | flat `(args…)` vs fill / one-per-line (method args) |
| `helpers/chains.rs` `field_selector_chain` | flat `a.b` vs broken                                |
| `helpers/callables.rs` / `declarations.rs` | inline vs vertical signature/header                 |
| `helpers/lists.rs` braced blocks           | fill vs one-per-line array initializers             |

**Chain hotfix (2025):** `chain_layout_preference` uses the broken chain doc
directly for `ChainRole::NestedArgument` instead of `best_fitting`. Correct
broken layout; does not replicate GJF’s global “maybe still fits on this line”
behavior for short inner chains.

### Domains that need the same rearchitecture

This is **not** chain-only. Any helper that (1) pre-formats a subtree containing
its own `BestFitting` or fill trials and (2) embeds that subtree inside another
width-sensitive wrapper hits the same ceiling:

1. **Selector chains** — nested fluent chains in arguments (`B24909927.java`);
   outer chain flat vs broken; Palantir last-dot needs **column state at break
   time**, not per-level subtree fit.
2. **Argument and formal lists** — `best_fitting(flat, broken)` on method args;
   arguments that are calls, lambdas, or inner lists nest more fit trials.
3. **Array initializers** — braced fill in `lists.rs`; elements are full
   expression docs, often with nested calls.
4. **Assignment and declarations** — reluctant `=` breaks and header
   inline/vertical pairs in `callables.rs` / `declarations.rs`; RHS/LHS are
   eager expression docs.
5. **Binary / conditional chains** (future `helpers/expressions.rs`) — GJF uses
   fill with unified/independent modes; porting via nested `best_fitting` will
   repeat the problem.
6. **Palantir profile** — 80-column last-dot and partial inlining require
   [`LastLevelBreakability`](https://github.com/palantir/palantir-java-format/blob/develop/palantir-java-format/src/main/java/com/palantir/javaformat/LastLevelBreakability.java)-style
   **global** constraints; cannot be correct as nested subtree fit alone.

### Target architecture (Phase 8 — cross-cutting)

Keep Java policy in `jolt_java_fmt`; extend `jolt_fmt_ir` only where multiple
helpers need the same break-selection primitive. Direction:

1. **Prefer optional breaks over nested `BestFitting`** — one doc tree with
   `soft_line` / `line()` separators; let the renderer choose breaks once
   (closer to Prettier/Ruff `group` + `ifBreak` than to stacked subtree trials).
2. **Defer formatting of nested slots where parent width matters** — chain
   collectors carry syntax/metadata; member argument docs formatted once at the
   outermost layout boundary, or formatted with break context passed down
   (Jolt’s `ChainRole` is a narrow form of this).
3. **Longer term: op stream or cached fit** — GJF-style emit-then-select, or
   width memoization on subtrees so nested trials do not re-walk `BestFitting`
   stacks (Oxc/Ruff separate IR build from print for the same reason).

Until then, document hotfixes that skip `best_fitting` for known nested roles
(`NestedArgument`, and similar when added) as **performance guards**, not final
policy. Oracle gaps that depend on “short inner chain stays inline on the
current line” may remain until global break selection lands.

**Success criteria:** format `B24909927.java` and Palantir nested-call fixtures
in linear time; nested short chains can still inline when the global pass says
they fit; no regression to missing-layout or comment debt.

## Module Layout

Current structure under `crates/jolt_java_fmt/src/`:

```text
layout.rs                  low-level Doc composition helpers
policy.rs                  profile policy accessors
comments.rs                comment ownership wrappers and Doc wiring
context.rs                 comment ownership and trivia cursor
rules/                     syntax-to-document rules by domain
helpers/
  comments.rs              GJF-style comment rewrite policy (block/line/javadoc-shaped)
  separated.rs             generic separated/delimited mechanics
  lists.rs                 Java list policy and comment-aware items
  callables.rs             callable declaration headers and tails
  type_declarations.rs     type declaration headers
  chains.rs                selector chain layout policy
  bodies.rs                blocks and type bodies
  annotations.rs           annotation layout helpers
  imports.rs               import declaration and section grouping policy
  literals.rs              literal formatting
analyzers/
  chains.rs                chain flattening, metadata, grouping
  binary.rs                same-precedence binary flattening
```

Not yet extracted (extract only when a real policy surface justifies it):

- Broader `helpers/expressions.rs` ownership for conditional, cast,
  parenthesized, array-initializer, and lambda wrapping policy

Do not move code merely to satisfy this tree.

## Helper Surface Status

### Comment rewrite — largely done

`helpers/comments.rs` owns GJF `JavaCommentsHelper` rewrite policy: `column0`
shifting for inline/trailing placement, block comment trailing-whitespace strip,
javadoc-shaped indentation, `preserveIndentation`, single-line javadoc collapse,
line-comment trim/wrap/normalize (`//noinspection`, `//$NON-NLS-`, `// MOE:`),
and parameter-comment normalization (`/* name= */`).

`comments.rs` keeps ownership buckets and Doc emission only; all rewrite routes
through `rewrite_comment_lines()`.

Remaining (Milestone 15): full `JavadocFormatter` HTML/`@tag` normalization for
`/**` comments. Oracle upstream materialization uses `--skip-javadoc-formatting`
until then.

Remaining edge domains (currently blocked, not silently ignored): see Phase 3
below.

### Lists — largely done

`helpers/separated.rs` and `helpers/lists.rs` cover delimited comma lists,
one-per-line lists, fill-style behavior, keyword-prefixed clause lists, and
comment-aware item formatting for arguments, parameters, type arguments, and
type parameters.

Recent: method argument lists use `best_fitting(flat, broken)` with GJF
short-item threshold; lambda parameter lists stay on fill-only path.

Remaining: extend comment placement into any list domains still blocking on
unowned trivia; keep profile-specific indentation centralized in policy. List
fill vs flat policy shares the nested-`BestFitting` ceiling with chains—see
[global break selection](#global-break-selection-architecture-debt).

### Callable and type declarations — partial

`helpers/callables.rs` and `helpers/type_declarations.rs` are wired into
`declarations.rs` for methods, constructors, compact constructors, annotation
elements, and type headers.

Remaining: move throws/default/annotation-header comment policy out of
`declarations.rs`; shrink the rule module so header wrapping changes happen in
one helper.

### Selector chains — in progress

`analyzers/chains.rs` flattens selector syntax into `ChainMember` sequences with
metadata and `ChainGroup`s. `helpers/chains.rs` renders staged alternatives with
profile-aware breaking via `JavaFormatPolicy`. Primary-expression receivers
(parenthesized, conditional) use flat preference before the first selector (GJF
`visitDot` `needDot` + fill).

**Google baseline (2025-06-30):** aggregate diff 2132→**2112**; `B20701054.java`
57→**45** via `selector_chain_with_single_invocation_field_prefix` (field
`field_dot_fill` through lone call, then fluent tail) and a type-name
field-prefix route (`type_name_prefix_member_end_index` without `is_call`
guard). Guardrails held: `B24909927` 13, `B26207047` 64, `B20128760` 77.
Conditional expressions now break before `?` / `:` (GJF
`visitConditionalExpression`).

Remaining: full `visitRegularDot` length loop for mixed call chains (peel-first
fallback still regresses `B26207047` argument-list trials); type-name
`ImmutableList.builder` double-dot on coalesced field+call concat; ternary paren
layout inside chain receivers; Palantir 80-col last-dot policy — using syntax
shape, not fixture-name heuristics.

**Architecture note:** nested chains in arguments currently bypass
`best_fitting` (`ChainRole::NestedArgument`) to avoid exponential fit cost. Full
GJF parity for inline nested chains depends on
[global break selection](#global-break-selection-architecture-debt).

### Expressions — partial

`helpers/expressions.rs` owns assignment expression layout, conditional
expression layout, parenthesized expression layout, cast-primary base layout,
binary chain layout, and text-block-aware expression value handling.
`analyzers/binary.rs` owns same-precedence chain flattening and
precedence/parenthesization metadata. `rules/expressions.rs` still owns
expression syntax traversal and comment slot collection before delegating
expression layout into the helper.

Remaining: array initializer and broader lambda wrapping policy still need
helper ownership. New expression helpers should use optional breaks / deferred
layout—not nested `best_fitting` on eager subtrees—see
[global break selection](#global-break-selection-architecture-debt).

### Blocks and bodies — largely done

### Bodies and control-flow blocks — in progress

`helpers/bodies.rs` covers statement blocks, constructor bodies, and class,
interface, and enum bodies with dangling-comment support. It also owns GJF-style
control-flow body policy: empty-block collapse, leading/trailing blank-line
preservation, if/else body options, loop/do body options, and try/catch/finally
trailing-clause options. `rules/statements.rs` keeps syntax traversal, comment
rejection, and statement-formatting closures.

`layout.rs` uses flat parenthesized conditions and fill-style (`line()`) breaks
for inline if/while/for/do bodies when within width.

Remaining: refine trailing-blank policy on final if/else clauses and try/catch
tails (`B20535125.java` tail), plus switch-block spacing polish.

### Imports and compilation units — partial

`helpers/imports.rs` owns import declaration rendering, import section grouping,
and blank-line separation via `JavaFormatPolicy`. `rules/compilation_unit.rs`
collects package/import/module/member syntax and comment ranges, then delegates
import layout to the helper.

Remaining: implement oracle import ordering/removal only when the formatter
pipeline owns a source-level import-ordering pass; do not grow
`rules/compilation_unit.rs` for profile-specific grouping.

## Remaining Work

Work is organized by the original phase sequence. Phases 1–3 are complete; later
phases overlap in practice.

### Phase 1: Stabilize the helper vocabulary — complete

Profile policy, rule contract, separated/delimited mechanics, Java list helpers,
and narrow-width helper tests are in place.

### Phase 2: Eliminate raw source passthrough — complete

All parser-clean syntax formats through real rules. Oracle suites are the broad
coverage signal for this milestone; unit tests stay focused on helper
boundaries.

### Phase 3: Comment ownership — largely complete

Ownership buckets, rejection guards, and unconsumed-trivia diagnostics replace
order-only handling and late appendage.

Remaining edge domains (currently blocked, not silently ignored):

- annotation argument comments,
- inline comments inside annotations and some header positions,
- some header-boundary and branch/else comment positions.

See `rules/tests.rs` tests named `*_remain_unowned_formatter_debt`.

### Phase 4: Callable and type declaration helpers — in progress

Helpers exist and are adopted, but `declarations.rs` has not shrunk materially.
Success means declaration wrapping and header comment policy change in helpers,
with neutral or improved declaration-related oracle diffs.

### Phase 5: Selector chain policy — in progress

Analyzer and helper infrastructure is in place. Success means explainable chain
behavior from syntax shape and materially smaller diffs on `B24909927.java`,
`B20701054.java`, and Palantir nested-call fixtures—without Palantir regressions
elsewhere.

Measure Google, AOSP, and Palantir scoreboards after each broad chain rule
change. Review by domain, not aggregate number alone.

### Phase 8: Global break selection — not started

Cross-cutting rearchitecture so width-sensitive policy uses optional breaks and
a single renderer pass (GJF `OpsBuilder` / `DocBuilder` shape) instead of nested
`best_fitting` on eagerly formatted subtrees. Unblocks correct inline behavior
for nested chains, list fill inside calls, assignment breaks, array fill, and
Palantir last-dot policy. See
[Global break selection](#global-break-selection-architecture-debt).

### Phase 6: Owned comments in helpers — in progress

List, body, and chain helpers already integrate many comment buckets.

Remaining: finish placement in the blocked domains above; add helper-boundary
comment tests where oracle fixtures do not already cover a shape.

### Phase 7: Profile-specific oracle alignment — in progress

Policy accessors centralize known profile differences. Remaining work tracks
[oracle compatibility gaps](#oracle-compatibility-gaps): Palantir chain, lambda,
and assignment policy is the largest profile-specific gap; AOSP needs policy
wired through all continuation-indent paths.

Keep Google as the base unless a helper has a documented profile divergence.

## Verification

Standard local gates:

```sh
cargo fmt --check
INSTA_UPDATE=no cargo test -p jolt_java_fmt
cargo test -p jolt_java_syntax --lib
```

When changing oracle-facing layout policy:

```sh
INSTA_UPDATE=always cargo test -p jolt_java_fmt --test oracle_fixtures
rg -n "missing-rule blocked|aggregate diff size|largest per-file diff" \
  crates/jolt_java_fmt/tests/snapshots/oracle_fixtures__*_scoreboard.snap
```

Per-file diffs are written to `.oracles/reports/java/` (see
[Oracle compatibility gaps](#oracle-compatibility-gaps)). Review scoreboard
changes by domain, not aggregate number alone.

Coverage invariants (missing-layout exits, raw source passthrough, late
remaining-comment appendage) are satisfied and should stay that way. Re-check
with `rg` only if those mechanisms are reintroduced.

## Non-Goals

- Do not introduce arbitrary user style knobs.
- Do not move Java policy into `jolt_fmt_ir`.
- Do not add unsupported-layout exits as scaffolding.
- Do not add raw-source formatting fallbacks as scaffolding.
- Do not optimize for one fixture by naming methods, classes, or files.
- Do not silently drop, append, or ignore comments to make tests pass.
- Do not split modules mechanically without extracting a real abstraction.
