# Post-Refactor Regression Audit

Date: 2026-07-19

Audit target: cleanup commit `b7c706684533f97ac1fc8eda5550ca4c65d50c77` against
parent `47ccaeae`.

## Conclusion

The cleanup correctly removed historical code-volume gates, aggregate-count
snapshots, phase labels, and completed design roadmaps. It also removed four
kinds of behavioral coverage that were not equivalent scaffolding:

1. deterministic malformed-input formatter mutations;
2. formatter-corpus route inventories;
3. malformed-parser scaling inputs;
4. the only record of one active Kotlin recovery regression.

The removed mutation sampler exposed two reproducible Java formatter bugs and
two additional Java conservation findings before it was deleted. Those cases
must become stable, named regressions. The arbitrary fixture-family sampler and
debug-string growth assertions should not return.

## Implemented regression coverage

Focused fixture-backed tests now live in:

- `crates/jolt_java_fmt/tests/known_regressions.rs`
- `crates/jolt_java_fmt/tests/fixtures/`
- `crates/jolt_kotlin_fmt/tests/known_regressions.rs`
- `crates/jolt_kotlin_fmt/tests/fixtures/`

The four Java tests are ignored individually while their formatter defects are
open. They remain runnable with:

```sh
cargo test -p jolt_java_fmt --test known_regressions -- --ignored --test-threads=1
```

All four currently fail:

| Test                                                     | Fixture                                                | Observed failure                                                                                       |
| -------------------------------------------------------- | ------------------------------------------------------ | ------------------------------------------------------------------------------------------------------ |
| `malformed_import_without_semicolon_is_idempotent`       | `malformed-import-without-semicolon.java`              | Debug formatting is blocked by `DuplicateTrivia { token: 0, side: Trailing, ordinal: 1 }`.             |
| `malformed_module_name_gap_is_idempotent`                | `incomplete-module-name-gap.java`                      | The first formatting pass changes parser diagnostic classification.                                    |
| `formatter_ignore_constructor_mutation_conserves_trivia` | `formatter-ignore-constructor-missing-open-paren.java` | Formatting is blocked by `DuplicateTrivia { token: 6, side: Leading, ordinal: 2 }`.                    |
| `mixed_line_ending_comment_mutation_is_idempotent`       | `mixed-line-ending-comment-missing-type.java`          | Formatting changes the represented block-comment body from `block crlf * cr lf` to `block crlf cr lf`. |

`class_followed_by_recovered_expression_conserves_contents` is enabled and
passes. It covers the formerly active Kotlin regression.

## Confirmed regressions requiring named fixtures

### Java malformed import duplicates trivia

- Source contract removed with `deterministic_java_recovery_mutations` from
  `crates/jolt_java_fmt/tests/corpus.rs`.
- Exact input:

  ```java
  import /* JOLT-TRIVIA:import-pieces */
  ```

- Observed optimized-build behavior: each formatting pass emits another copy of
  the comment.
- Required regression: formatting must preserve the represented comment exactly
  once and must be idempotent.
- Fixture:
  `crates/jolt_java_fmt/tests/fixtures/malformed-import-without-semicolon.java`.
- Test: `malformed_import_without_semicolon_is_idempotent`.
- Status: ignored and failing as listed above.

### Java incomplete module name is non-idempotent

- Source contract removed with `deterministic_java_recovery_mutations` from
  `crates/jolt_java_fmt/tests/corpus.rs`.
- Exact input:

  ```java
  module recovered. {
    uses z.Service;
    requires transitive + static a.module;
    uses a.Service;
  }
  ```

- Observed optimized-build behavior: successive formatting passes move `module`
  and `uses` tokens across directive boundaries.
- Required regression: preserve represented tokens and trivia and produce the
  same output on the second pass.
- Fixture:
  `crates/jolt_java_fmt/tests/fixtures/incomplete-module-name-gap.java`.
- Test: `malformed_module_name_gap_is_idempotent`.
- Status: ignored and failing as listed above.

## Minimized mutation regressions

The two previously unretained mutation findings were reproduced from their base
fixtures with the deleted deterministic eighth-position token-removal algorithm
and reduced to exact fixture inputs.

### Formatter-ignore constructor boundary

- Base fixture:
  `fixtures/java/style/declarations/formatter-ignore-constructor-body.java`.
- Minimized mutation: remove the opening `(` from `Example()`, producing
  `Example)`.
- Fixture:
  `crates/jolt_java_fmt/tests/fixtures/formatter-ignore-constructor-missing-open-paren.java`.
- Test: `formatter_ignore_constructor_mutation_conserves_trivia`.
- Current failure: the formatter blocks with
  `DuplicateTrivia { token: 6, side: Leading, ordinal: 2 }`.

### Mixed CR/CRLF line comments

- Base fixture: `fixtures/java/style/comments/mixed-comment-line-endings.java`.
- Minimized mutation: remove the `int` token before the CR-terminated `crlf=1;`
  declaration while retaining the original mixed line endings.
- Fixture:
  `crates/jolt_java_fmt/tests/fixtures/mixed-line-ending-comment-missing-type.java`.
- Test: `mixed_line_ending_comment_mutation_is_idempotent`.
- Current failure: formatting changes the represented block-comment body,
  removing the meaningful `*` before `cr`.

## Previously active retained regression

The deleted `.agents/docs/formatter-retained-regressions.toml` contained two
`status = "active"` inline entries.

### Kotlin class followed by recovered expression

- Inventory ID: `kotlin-class-followed-by-recovered-expression`.
- Exact input:

  ```kotlin
  class Unexpected + (val value: Int)
  ```

- Historical failure: the class ended before `+`; formatting the separately
  recovered parenthesized expression emitted `class Unexpected\n+()` and lost
  `val value: Int`.
- Current optimized-build result, including a second formatting pass:

  ```kotlin
  class Unexpected
  +(val value: Int)
  ```

- Disposition: covered by
  `crates/jolt_kotlin_fmt/tests/fixtures/class-followed-by-recovered-expression.kt`
  and the enabled `class_followed_by_recovered_expression_conserves_contents`
  test. The test passes.

### Java adjacent unary operators

- Inventory ID: `java-adjacent-unary-operators`.
- Input shape: adjacent unary `- -` and `+ +` operators must not fuse into
  decrement/increment tokens.
- Disposition: retained by `fixtures/java/syntax/recovery/expressions.java` and
  its current syntax and formatter snapshots. No recovery action.

## Lost coverage contracts

### Formatter route classification

Deleted snapshots:

- `crates/jolt_java_fmt/tests/snapshots/corpus__formatter_fixture_manifest.snap`
- `crates/jolt_kotlin_fmt/tests/snapshots/corpus__formatter_fixture_manifest.snap`

The surviving syntax manifests inventory paths only. They do not assert whether
a formatter fixture is formatted as clean syntax, audited as diagnostic syntax,
or skipped because parsing returned no represented tree. The current corpus
tests can therefore silently move a formerly clean fixture into a weaker audit
lane or skip a no-tree parse.

The Java and Kotlin corpus tests now require every fixture to produce a
represented tree and assert the expected formatter lane. Negative parser
fixtures are selected by their `diagnoses-` or `recovers-` names, plus the two
reviewed exceptional routes. Both corpus tests pass without restoring duplicate
full-path manifests.

### Deterministic token-removal mutations

Deleted symbols:

- `jolt_test_support::deterministic_token_removal_candidates`
- `jolt_java_fmt::corpus::deterministic_java_recovery_mutations`
- `jolt_kotlin_fmt::corpus::deterministic_kotlin_recovery_mutations`

The tests deleted represented tokens from selected fixtures and checked
completion, diagnostic stability, comment/trivia conservation, reconstruction,
and idempotence. The Java test found the four cases recorded above. The Kotlin
test was the only neighboring-token-omission coverage across Kotlin fixture
families, but no concrete Kotlin failure was retained.

Disposition: do not restore directory-family equality or order-dependent
sampling. Convert every observed failure into a named fixture. If broader
mutation coverage is still wanted, give it a globally bounded deterministic case
list whose entries describe syntax contracts rather than fixture directory
names.

### Malformed parser scaling inputs

Deleted files:

- `crates/jolt_java_syntax/tests/parser_progress.rs`
- `crates/jolt_kotlin_syntax/tests/parser_progress.rs`

The Java suite scaled repeated malformed members, formal/type parameters, throws
clauses, calls, switch labels, and try resources. The Kotlin suite scaled
malformed headers and commas, delegation specifiers, newline-start primaries,
call/type suffixes, `when`, `try`, and loops. No surviving fixtures contain
those generated stress inputs.

The deleted tests bounded `format!("{parse:#?}").len()` growth below six times
when input grew four times. That is not a parser complexity or memory bound: it
conflates diagnostics, tree representation, and debug formatting.

Disposition: keep these tests deleted. Preserve the adversarial input families
in the parser benchmark corpus, where runtime and allocation growth can be
measured directly. Add an integration regression only for a concrete hang,
nontermination, source loss, or diagnostic/tree explosion with a structural
bound.

## Deleted inventory accountability

The retained-regression inventory contained:

- 52 fixture entries;
- 2 active inline regressions;
- 27 parser-fix entries;
- 1 imported-corpus gate.

Audit result:

- The Kotlin active inline regression is the only entry with neither an exact
  current fixture nor a named test; it is recorded above.
- The Java active inline regression is covered by the renamed `expressions.java`
  recovery fixture.
- The other fixture entries and all 27 parser-fix behaviors remain represented
  by current named parser/recovery fixtures, including the consolidated Java
  semantic fixture names and Kotlin `program`, `types-and-parameters`,
  `declarations`, `expressions`, and `statements-and-control-flow` fixtures.
- Recovery fixture and snapshot renames preserved their contents; phase-number
  removal did not delete those reproductions.
- The imported-corpus gate remains behaviorally covered by exact per-file source
  reconstruction and formatter completion/idempotence checks.

No other `status = "active"`, unchecked retained regression, conservation
allowlist, or unresolved migration exception was found in the deleted inventory
or roadmaps.

## Intentional removals with equivalent or better coverage

### Imported corpus summaries

Deleted aggregate parser and formatter summary snapshots pinned file totals,
formatted totals, reconstruction-change totals, syntax-blocked totals, and
aggregate diagnostic counts. Current imported-corpus tests assert exact source
reconstruction per file and formatter completion/idempotence per file. Those
behavioral assertions are stronger and identify the failing path directly.

### Schema-audit totals

Deleted schema snapshots pinned incidental totals such as fixture count, node
count, exact-node count, and malformed-node count. Current schema audits still
fail directly on missing required slots, unexpected slots, missing diagnostic
ownership, and diagnostic-owned unexpected slots. The useful schema contract
remains; volume pins do not.

### Architecture and implementation-size gates

Deleted `crates/jolt_test_support/tests/architecture_gates.rs` compared net line
changes against historical commit `2197128`, scanned source text for historical
identifiers, and enforced reviewed line-count ceilings. These were migration
controls, not behavioral formatter contracts. Their removal does not erase a
known bug reproduction.

### Completed design and migration documents

Deleted documents:

- `.agents/docs/FORMATTER_ARCHITECTURE_DEBT_CHECKLIST.md`
- `.agents/docs/FORMATTER_RECOVERY_ARCHITECTURE.md`
- `.agents/docs/kotlin-grammar-report.md`
- `.agents/docs/kotlin-parser-formatter-roadmap.md`

Their implemented formatter invariants remain in `AGENTS.md`. Kotlin grammar
research limitations around explicit backing fields, collection literals, and
name-based destructuring now have named parser fixtures. Historical phase and
performance-gate prose was completed or superseded, not live regression
coverage.

## Outstanding recovery work

1. Fix the four ignored Java regression tests and enable each test as its
   formatter defect is closed.
2. Move malformed scaling input families into benchmarks if synthetic stress
   corpora are added to the benchmark harness; do not restore debug output
   length as a complexity proxy.
