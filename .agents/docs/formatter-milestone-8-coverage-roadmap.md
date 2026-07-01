# Formatter Milestone 8 Coverage Roadmap

Last audited: 2026-06-30. This is a code-and-report audit of
`crates/jolt_java_fmt`, the local oracle mirrors, `.oracles/reports/`, and
recent formatter history. The scoreboards and generated report diffs are treated
as accurate; do not rerun tests just to refresh this document.

Milestone 8 is no longer about formatter coverage in the sense of parser-clean
Java failing to format. It is about finishing the helper-layer architecture and
closing the remaining oracle layout-policy gaps without falling back to fixture
patches.

## Architecture North Star

The formatter should stay layered like this:

```text
source text
  -> jolt_java_syntax parser
  -> lossless CST + wrapper accessors
  -> Java rule modules
  -> Java analyzers + layout helpers
  -> profile policy
  -> shared document IR
  -> shared renderer
```

The ownership boundary is:

```text
rules/      identify grammar slots, own comment ranges, delegate
analyzers/  flatten/classify syntax shape
helpers/    own named Java layout policy
policy.rs   centralize Google/AOSP/Palantir differences
layout.rs   keep generic Doc plumbing only
jolt_fmt_ir language-neutral document algebra and rendering only
```

Rule modules should answer grammar questions. They should not decide whether a
selector chain breaks before the first selector, whether an argument list fills
or goes one-per-line, how declaration headers align after a broken type, or how
profile-specific continuation indentation works. Those decisions belong in named
helpers and `JavaFormatPolicy`.

Profiles are compatibility targets, not style knobs. Add profile policy as
narrow methods with oracle evidence, not direct `JavaFormatProfile` matches in
leaf rules. AOSP is Google-style layout at AOSP indentation and import policy;
Palantir is a distinct chain/lambda/assignment policy surface.

The renderer must remain language-neutral. Add IR features only when several
Java helpers need the same general break-selection primitive. Java concepts such
as chains, argument lists, throws clauses, annotations, imports, and switch
labels stay in `jolt_java_fmt`.

## Completion Target

Milestone 8 is complete when the pinned oracle corpora reach 100% exact match in
this order:

1. Google profile against the pinned google-java-format corpus.
2. AOSP profile against the same google-java-format corpus.
3. Palantir profile against the pinned palantir-java-format corpus.

Later profile work must not regress earlier profiles. Parser-clean syntax must
format through real rules: no fallback exits, raw source passthrough for
arbitrary nodes, fixture-name heuristics, method/class-name heuristics, or
silent comment drops.

## Current Status

Current committed scoreboards and report indexes:

| Profile  | Exact match | Mismatches | Aggregate diff | Largest diff           |
| -------- | ----------- | ---------- | -------------- | ---------------------- |
| Google   | 127 / 208   | 81         | 403            | `B20128760.java` (18)  |
| AOSP     | 124 / 208   | 84         | 732            | `B24909927.java` (205) |
| Palantir | 96 / 224    | 128        | 3,755          | `B24909927.java` (916) |

Report indexes:

- `.oracles/reports/java/google-java-format/google/index.md`
- `.oracles/reports/java/google-java-format/aosp/index.md`
- `.oracles/reports/java/palantir-java-format/palantir/index.md`

All three profiles have zero missing-rule buckets. The remaining failures are
layout-policy mismatches.

Recent migrations that are now landed and should not be treated as future work:

- `layout.rs` is reduced to generic braced/spacing wrappers.
- `helpers/expressions.rs` owns assignment, conditional, parenthesized, cast,
  binary, and text-block-aware expression value layout.
- `helpers/statements.rs` owns statement shell layout for inline bodies and
  `for` headers.
- `helpers/switches.rs` owns switch block/rule/label/guard assembly.
- `helpers/lambdas.rs` owns lambda body layout.
- `helpers/imports.rs` owns import declaration and section grouping.
- `helpers/callables.rs` owns callable header/tail policy.
- `helpers/array_initializers.rs` owns initializer layout selection.
- `analyzers/binary.rs` owns same-precedence binary flattening.
- `jolt_fmt_ir::Doc` is structurally shared, so cloning document subtrees for
  alternative layouts is no longer a recursive deep clone.
- The renderer has shared fit memoization across cloned fit checkers.
- `jolt_fmt_ir` has a language-neutral GJF-style level primitive with level-wide
  broken indent plus `UNIFIED`, `INDEPENDENT`, and `FORCED` breaks.
- Java formatter hot expression helpers now emit level/break trees for method
  invocation arguments, selector chains, lambdas, callable heads, annotations,
  array initializers, and nested generic type arguments instead of broad
  flat-vs-broken `best_fitting` alternatives.
- Direct `JavaFormatProfile` checks are centralized in `policy.rs`,
  `options.rs`, context defaults, and tests.

Still true:

- `rules/declarations.rs` and `rules/expressions.rs` are the largest rule
  modules and still contain policy-shaped assembly that should move behind
  helper APIs.
- `helpers/chains.rs` is correctly the chain policy center, but it still carries
  many local width/fit heuristics that should be recast as explicit GJF
  level/open/break construction.
- Comment debt is explicit: unowned comments are rejected or tested as debt, not
  silently ignored.
- Raw source is still legitimate at token/literal preservation boundaries, but
  list/comment helper paths that emit raw comment text should be audited and
  routed through centralized rewrite where possible.

## Current Top Diff Drivers

Top concentration matters more than the long tail:

- Google top 10 account for 126 / 403 diff lines.
- AOSP top 10 account for 444 / 732 diff lines.
- Palantir top 10 account for 2,163 / 3,755 diff lines.

Current top files:

| Profile  | Top fixtures                                                                                                      |
| -------- | ----------------------------------------------------------------------------------------------------------------- |
| Google   | `B20128760.java` 18, `B20701054.java` 17, `B24909927.java` 13, `A.java` 12, `B21331232.java` 12                   |
| AOSP     | `B24909927.java` 205, `B20128760.java` 44, `M.java` 39, `LiteralReflow.java` 34, `B26207047.java` 18              |
| Palantir | `B24909927.java` 916, `RSL.java` 329, `M.java` 199, `B20701054.java` 175, `palantir-deeply-nested-calls.java` 112 |

Retired or demoted buckets from older roadmap versions:

- Type declaration headers (`B28066276.java` 63-line era) are now residual edge
  work, not a top remaining bucket.
- Broad array-initializer extraction is done; remaining issues are specific
  multidimensional suffix, tabular, and break-selection edges.
- Import helper extraction is done; remaining import work is oracle grouping and
  source-level ordering policy.
- Statement shell extraction, switch extraction, lambda extraction, expression
  helper extraction, and callable tail extraction are done.
- `B24543625.java` no longer appears as a large current comment-driver in the
  report indexes.

## Architecture Gap Checklist

- [ ] Move remaining selector/member assembly out of `rules/expressions.rs` and
      into `helpers/chains.rs` / `helpers/expressions.rs`. Current rule residue
      includes receiver-head argument alternatives, `ChainMember` construction,
      method-reference wrapping, and array-access wrapping.

- [ ] Move remaining declaration-header/comment policy out of
      `rules/declarations.rs`. Current rule residue includes type-clause
      continuation wiring, formal-parameter plumbing, before-name comment
      layout, and record component annotation inline-vs-vertical selection.

- [x] Remove broad Java `best_fitting` layout alternatives. The Java formatter
      no longer calls `best_fitting`; method invocation arguments, format-method
      arguments, selector chains, expression lambdas, callable heads, annotated
      parameters, array initializers, type arguments, record components, and
      resource lists now use one-tree level/group/fill layouts.

- [ ] Continue replacing local width accounting with faithful GJF
      open/level/break construction. Remaining debt is not broad `best_fitting`,
      and selector-chain line-limit/dot-fill helpers have been replaced by level
      construction. Remaining width-policy debt is now mostly
      declaration/local-variable head accounting, list fill classification
      metadata, and Palantir last-dot behavior. Renderer fit caching is a
      temporary optimization, not the final algorithm.

- [ ] Audit raw-comment emission in list helpers. Token spelling and literal
      spelling may preserve source text; arbitrary comment rendering should
      route through the centralized comment rewrite path unless the raw spelling
      is the documented formatting rule.

- [ ] Keep profile checks out of rule modules. Add or refine `JavaFormatPolicy`
      accessors when a gap is profile-specific.

## Oracle Gap Checklist

Use local oracle mirrors as primary references. Do not browse GitHub for these
files unless the local mirror is missing.

### 1. Selector Chains, Shared Google/AOSP

- [ ] Match GJF regular-dot and prefixed-chain break search.

Evidence:

- Oracle:
  `.oracles/repos/google__google-java-format/core/src/main/java/com/google/googlejavaformat/java/JavaInputAstVisitor.java`
  around `visitDotWithPrefix` / `visitRegularDot`.
- Jolt: `crates/jolt_java_fmt/src/analyzers/chains.rs`,
  `crates/jolt_java_fmt/src/helpers/chains.rs`,
  `crates/jolt_java_fmt/src/rules/expressions.rs`.
- Reports: `B20128760.java`, `B20701054.java`, `B24909927.java`.

Current mismatch shape:

- Some nested builders are broken too shallow or too deep.
- Field-prefix runs and receiver-call runs do not always choose the same dot as
  GJF.
- Conditional or parenthesized receivers still expose indent and chain-splitting
  edges.
- AOSP amplifies the same decisions because 4-space indentation changes line
  budgets.

Do not fix this by fixture names or receiver method names. The unit of work is a
syntax-shape policy in the chain analyzer/helper.

### 2. Palantir Chain Breakability

- [ ] Model Palantir chain semantics instead of approximating them with width
      thresholds.

Evidence:

- Oracle:
  `.oracles/repos/palantir__palantir-java-format/palantir-java-format/src/main/java/com/palantir/javaformat/java/JavaInputAstVisitor.java`
  around Palantir chain handling.
- Oracle:
  `.oracles/repos/palantir__palantir-java-format/palantir-java-format/src/main/java/com/palantir/javaformat/doc/Level.java`
- Oracle:
  `.oracles/repos/palantir__palantir-java-format/palantir-java-format/src/main/java/com/palantir/javaformat/BreakBehaviour.java`
- Oracle:
  `.oracles/repos/palantir__palantir-java-format/palantir-java-format/src/main/java/com/palantir/javaformat/LastLevelBreakability.java`
- Oracle:
  `.oracles/repos/palantir__palantir-java-format/palantir-java-format/src/main/java/com/palantir/javaformat/PartialInlineability.java`
- Jolt: `crates/jolt_java_fmt/src/helpers/chains.rs`,
  `crates/jolt_java_fmt/src/policy.rs`.
- Reports: `B24909927.java`, `B20701054.java`,
  `palantir-deeply-nested-calls.java`, `palantir-lambda-multiline-arg.java`.

Current mismatch shape:

- Palantir keeps long nested builder and assertion chains flatter than Jolt.
- Palantir's last-dot and partial-inlineability behavior is not represented as
  break-state machinery.
- Nested argument heads such as builder calls inside `ImmutableList.of(...)`
  still explode vertically.

This probably needs IR-level support for marked/limited breaks or a
cached/global fit strategy, not only more helper thresholds.

### 3. Argument Lists and Nested Calls

- [ ] Finish GJF argument-list fill vs one-per-line policy for nested calls,
      format-method calls, annotation values, and Palantir inlineability.

Evidence:

- Oracle:
  `.oracles/repos/google__google-java-format/core/src/main/java/com/google/googlejavaformat/java/JavaInputAstVisitor.java`
  around `addArguments`, `argList`, and format-method handling.
- Oracle:
  `.oracles/repos/palantir__palantir-java-format/palantir-java-format/src/main/java/com/palantir/javaformat/java/JavaInputAstVisitor.java`
  around Palantir argument-list wrapping.
- Jolt: `crates/jolt_java_fmt/src/helpers/lists.rs`,
  `crates/jolt_java_fmt/src/analyzers/format_strings.rs`,
  `crates/jolt_java_fmt/src/policy.rs`.
- Reports: `B26207047.java`, `B21954779.java`, `B20128760.java`,
  `palantir-deeply-nested-calls.java`.

Current mismatch shape:

- Google still has residual format-method and long-rest-argument packing gaps.
- Palantir often wants nested single-call arguments and assertion chains flatter
  than Jolt emits.
- List helpers now emit GJF-shaped `addArguments` / `argList` levels for normal
  method invocation arguments. Remaining list gaps are policy/comment/tabular
  edges, not broad flat-vs-broken argument-list selection.

### 4. Declaration Headers and Initializers

- [ ] Close callable, field, record-component, and type-clause edge gaps.

Evidence:

- Oracle:
  `.oracles/repos/google__google-java-format/core/src/main/java/com/google/googlejavaformat/java/JavaInputAstVisitor.java`
  around method declarations, class declarations, and `declareOne`.
- Oracle:
  `.oracles/repos/palantir__palantir-java-format/palantir-java-format/src/main/java/com/palantir/javaformat/java/JavaInputAstVisitor.java`
  around declaration and initializer break behavior.
- Jolt: `crates/jolt_java_fmt/src/helpers/callables.rs`,
  `crates/jolt_java_fmt/src/helpers/type_declarations.rs`,
  `crates/jolt_java_fmt/src/rules/declarations.rs`,
  `crates/jolt_java_fmt/src/policy.rs`.
- Reports: `M.java`, `B21331232.java`, `B20128760.java`,
  `AnnotationFields.java`, Palantir `E.java`.

Current mismatch shape:

- Some long return type / method name / throws combinations split differently.
- Record component annotations and blank lines diverge under Palantir.
- Palantir's reluctant initializer and assignment breaks need richer policy than
  "same as Google with a wider line."

### 5. String Literals and Text Blocks

- [ ] Bring string literal reflow and text block handling to oracle parity.

Evidence:

- Oracle:
  `.oracles/repos/google__google-java-format/core/src/main/java/com/google/googlejavaformat/java/StringWrapper.java`
- Jolt: `crates/jolt_java_fmt/src/helpers/literals.rs`,
  `crates/jolt_java_fmt/src/helpers/expressions.rs`,
  `crates/jolt_java_fmt/src/policy.rs`.
- Reports: `LiteralReflow.java`, Palantir `RSL.java`.

Current mismatch shape:

- AOSP exposes additional long-string reflow because it shares a 100-column
  limit with wider indentation.
- Jolt's string reflow is simpler than GJF's escape-aware wrapper.
- Palantir `RSL.java` is a large text-block preservation/indentation mismatch:
  content, closing delimiters, assignment breaks, and call arguments diverge.

### 6. Comments and Vertical Whitespace

- [ ] Finish comment rewrite/placement edge cases without silent fallback.

Evidence:

- Oracle:
  `.oracles/repos/google__google-java-format/core/src/main/java/com/google/googlejavaformat/java/JavaCommentsHelper.java`
- Jolt: `crates/jolt_java_fmt/src/comments.rs`,
  `crates/jolt_java_fmt/src/helpers/comments.rs`,
  `crates/jolt_java_fmt/src/context.rs`.
- Reports: `B24702438.java`, Palantir `NON-NLS.java`, annotation and branch
  fixtures.
- Tests: `rules/tests.rs` contains named remaining comment-debt cases.

Current mismatch shape:

- Some annotation argument comments, inline annotation positions, header
  boundary comments, and branch/else comments remain blocked or mispositioned.
- Special line-comment forms such as `//noinspection`, `//$NON-NLS`, and
  vertical whitespace boundaries must stay centralized in comment helpers.

### 7. Imports and AOSP Grouping

- [ ] Match oracle import grouping where the formatter owns the source-level
      import pass.

Evidence:

- Oracle:
  `.oracles/repos/google__google-java-format/core/src/main/java/com/google/googlejavaformat/java/ImportOrderer.java`
- Jolt: `crates/jolt_java_fmt/src/helpers/imports.rs`,
  `crates/jolt_java_fmt/src/rules/compilation_unit.rs`,
  `crates/jolt_java_fmt/src/policy.rs`.
- Reports: `i55.java`, `i60.java`, `TypeAnnotations.java`, Palantir `E.java`.

Current mismatch shape:

- AOSP grouping is more than "separate static imports": it distinguishes
  Android, third-party, Java, and package boundaries.
- Do not bloat `rules/compilation_unit.rs`; add an import-order/grouping helper
  policy or a dedicated import pass when needed.

### 8. Annotations, Arrays, Ternaries, Switches, And Tail Edges

- [ ] Burn down the remaining low-volume shared gaps after the larger buckets
      above.

Evidence:

- Annotations: GJF annotation argument and array-member behavior in
  `JavaInputAstVisitor.java`.
- Arrays: `crates/jolt_java_fmt/src/analyzers/array_initializers.rs`,
  `crates/jolt_java_fmt/src/helpers/array_initializers.rs`.
- Ternary/binary/cast: `crates/jolt_java_fmt/src/helpers/expressions.rs`,
  `crates/jolt_java_fmt/src/analyzers/binary.rs`.
- Switches: `crates/jolt_java_fmt/src/helpers/switches.rs`.
- Reports: `A.java`, `C.java`, `SwitchRecord.java`, `SwitchGuardClause.java`,
  `ExpressionSwitch.java`.

Current mismatch shape:

- Multidimensional array suffix and creation dim breaks still differ.
- Assert-message, cast, generic type, and chained ternary indentation have small
  shared residuals but larger Palantir amplification.
- Switch guard, record-pattern, and comment-after-arrow shapes are close but not
  exact.

## Global Break Selection Debt

Historically, Jolt helpers eagerly formatted child slots into `Doc` values, then
built complete flat and broken subtrees wrapped in
`best_fitting(flat, [broken])`. If those subtrees already contained nested
`BestFitting` nodes, break selection had to explore increasingly large trees.
Deeply nested call and chain fixtures could become pathological.

GJF uses a different shape:

1. `OpsBuilder` emits tokens plus optional breaks, fill modes, and indentation.
2. `DocBuilder` lowers the op stream to a document tree.
3. Break selection happens as a global pass.

GJF does not have a broad `best_fitting(flat_subtree, broken_subtree)` analogue.
It computes memoized flat widths for `Doc.Level`s, then uses `Break`s with
`UNIFIED`, `INDEPENDENT`, or `FORCED` fill modes to decide where that one tree
breaks. Palantir extends the same level/break model with break behaviours and
last-level breakability; it still does not build arbitrary parallel finished
layouts for large subtrees.

The current Jolt IR now structurally shares `Doc` subtrees, memoizes fit checks,
and has a language-neutral `break_level_with_indent` primitive for GJF/PJF-style
levels: one level, a level-wide broken indent, and breaks with `UNIFIED`,
`INDEPENDENT`, or `FORCED` modes. Java helpers no longer call `best_fitting`;
the former all-short depth gate has been removed, and shallow and deep method
invocation argument lists share the same GJF-shaped `addArguments` / `argList`
construction.

The remaining issue affects chains, declaration headers, binary/conditional
chains, and Palantir last-dot policy: some helpers still carry local width
accounting or specialized fill policies instead of being direct translations of
GJF/PJF open/level/break construction.

Current timing evidence from the generated report indexes in debug builds:

| Profile  | Total profile time | `B24909927.java` format time | Report time |
| -------- | ------------------ | ---------------------------- | ----------- |
| Google   | ~4.27s             | ~4.27s                       | ~0.002s     |
| AOSP     | ~4.20s             | ~4.19s                       | ~0.006s     |
| Palantir | ~4.11s             | ~4.08s                       | ~0.029s     |

The old ~2s full-suite expectation is closer but still not restored. Formatter
unit tests, syntax unit tests, and syntax fixture tests remain sub-second. The
oracle suite is still slow because one deeply nested formatter fixture dominates
CPU time. Treat the remaining ~4s as architecture debt, not report-generation
overhead.

Do not exclude `B24909927.java` as a normal tactic. Temporarily skipping it
would restore fast iteration, but it would also hide the broad
chain/list/declaration break-selection failure mode that blocks real oracle
parity. Exclusion is only a last resort if global break-selection work cannot
make progress and unrelated work is completely stalled.

Target direction:

- Treat broad `best_fitting` in `jolt_java_fmt` as banned architecture debt.
  Current code has no Java uses; do not reintroduce them.
- Prefer optional breaks and groups over nested subtree trials.
- Defer or contextualize nested slot formatting when parent width matters.
- Renderer structural sharing, fit caching, and the GJF-style level primitive
  are landed. Prefer migrating remaining helper-local width policies to that
  primitive rather than adding new thresholds.
- Treat Palantir last-dot and partial-inlineability as evidence for general
  break-state support, not Java-specific logic in `jolt_fmt_ir`.

Success means `B24909927.java` formats in milliseconds rather than tens of
seconds in debug builds, Palantir nested-call fixtures stay below pathological
runtime, and short nested chains can still inline when the global break decision
says they fit.

## Work Order

Prefer one substantial work unit per session. Pick a gap by policy mechanism,
not by fixture name.

1. Global break-selection performance: finish making `B24909927.java`
   non-pathological in debug builds while preserving real formatting and report
   coverage. The IR level primitive and Java `best_fitting` removal are landed;
   chain/list/type/callable hot paths now emit level construction; next verify
   runtime and replace remaining declaration/list-classification width policy
   with faithful GJF/PJF mechanisms where applicable.
2. Shared selector-chain policy: move Google/AOSP top chain fixtures together
   without regressing Palantir catastrophically.
3. Palantir chain breakability: model last-dot/partial-inline behavior instead
   of stacking thresholds.
4. String/text-block handling: especially `RSL.java` and AOSP
   `LiteralReflow.java`.
5. Argument-list nested-call and format-method fill: one helper policy, many
   reports.
6. Declaration header and initializer edge policy: continue shrinking
   `rules/declarations.rs`.
7. Comment and annotation edge placement: unblock or correctly place comments in
   currently explicit debt domains.
8. AOSP import grouping and continuation audit: finish profile wiring after
   shared Google-style gaps are stable.
9. Low-volume tail: arrays, ternaries, switches, record patterns, and 1-3 line
   residuals.

## Verification

For architecture-only edits:

```sh
cargo fmt --check
INSTA_UPDATE=no cargo test -p jolt_java_fmt
```

For oracle-facing layout policy:

```sh
INSTA_UPDATE=no cargo test -p jolt_java_fmt --test oracle_fixtures
rg -n "exact-match percentage|aggregate diff size|largest per-file" \
  crates/jolt_java_fmt/tests/snapshots/oracle_fixtures__*_scoreboard.snap
```

Per-file reports live under `.oracles/reports/java/`. Review the report diffs by
layout category, not aggregate number alone. A small aggregate improvement can
still be a bad change if it regresses the policy shape of an earlier profile.

## Non-Goals

- Do not add formatter fallback exits for parser-clean syntax.
- Do not add raw-source formatting fallbacks for parser-clean syntax.
- Do not add arbitrary user style knobs.
- Do not move Java policy into `jolt_fmt_ir`.
- Do not optimize for fixture names, method names, class names, or corpus
  quirks.
- Do not silently drop, append, or ignore comments to make reports greener.
- Do not split modules mechanically without extracting a real helper or analyzer
  surface.
