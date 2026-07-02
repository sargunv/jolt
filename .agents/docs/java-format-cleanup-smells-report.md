# Java Formatter Cleanup Smells Report

This report records review smells found after completing the Java formatter
implementation goal. These are not permanent intentional deviations from the
style guide or implementation spec. They are cleanup and hardening items for
keeping the architecture easy to reason about as the formatter grows.

The implementation direction still looks sound:

- Jolt-owned style fixtures are the pass/fail source of truth.
- Imported upstream formatter inputs are broad corpus coverage, not expected
  output truth.
- Parser-clean Java is expected to receive real formatting rules.
- The shared renderer uses bounded group fitting rather than compatibility
  search.
- The formatter has no known permanent intentional deviations.

## Oversized Rule Modules

The formatter rules are layered, but three modules have grown large enough to
hide future policy drift:

- `crates/jolt_java_fmt/src/rules/declarations.rs`
- `crates/jolt_java_fmt/src/rules/statements.rs`
- `crates/jolt_java_fmt/src/rules/expressions.rs`

Large files are not automatically bad, but formatter policy is easiest to review
when each module maps directly to a small style-guide domain. These files now
mix public dispatch, policy helpers, comment preservation, delimiter handling,
and several syntax domains.

Cleanup checklist:

- [x] Split declaration formatting by subdomain, likely type declarations,
      member bodies, callable declarations, enum declarations, and constructor
      bodies.
- [x] Split statement formatting by subdomain, likely blocks/bodies,
      conditionals/loops, switch, try/resources, and simple jump/assert
      statements.
- [x] Split expression formatting by subdomain, likely leaves/parentheses,
      operators, calls/arguments, member chains, object/array creation, lambdas,
      and casts/patterns.
- [x] Keep existing fixture behavior unchanged during module splits.
- [x] Prefer moving repeated layout shape into named helpers only when the
      helper expresses real policy, not just a shorter function call.
- [x] After each split, scan for duplicated comment-placement logic that should
      move to an existing comment helper.

## Recovery Token-Sequence Branches

Some declaration formatting functions still contain token-sequence recovery
branches for malformed syntax shapes with missing required names. The public
formatter blocks non-clean parses before layout, so these are not normal
parser-accepted fallback exits today.

This is acceptable as a recovery guard, but it is easy for future code to
misread these branches as a sanctioned formatting fallback.

Cleanup checklist:

- [ ] Rename the helper or wrapper path so recovery-only token formatting is
      visibly not a normal formatting rule.
- [ ] Add a focused invariant test that clean public formatting never reaches
      recovery token-sequence declaration branches.
- [ ] Keep the existing `declaration_recovery_nodes_do_not_reach_layout` test,
      but make the recovery-only contract more direct.
- [ ] Do not add new token-sequence formatting branches for parser-clean syntax.
- [ ] When a recovery branch becomes reachable for clean syntax, fix the parser
      or add missing CST accessors and real formatter rules instead.

## Imported Corpus Syntax-Diagnostic Allowlist

The imported corpus test has an allowlist of fixture inputs that are expected to
produce syntax diagnostics. This matches the current fixture reality, especially
for upstream unit fixtures that are not all complete Jolt-valid Java compilation
units.

The risk is quiet backlog growth: an allowlist can become a place where parser
coverage problems disappear from review.

Cleanup checklist:

- [x] Add comments grouping allowlisted paths by reason, such as intentionally
      invalid upstream Java, fixture fragment, unsupported Java version syntax,
      or parser backlog.
- [x] For any parser backlog entry, link or record the syntax feature that would
      remove it. See `.agents/docs/java-parser-backlog.md`.
- [x] Fail the test when an allowlisted fixture starts parsing cleanly but still
      remains on the allowlist.

## Formatter Ignore Range Math

Formatter ignore support necessarily preserves raw source slices, but its range
mapping is one of the few formatter helpers that operates directly on source
offsets and token ranges.

That is the right ownership boundary for this feature, but it is more fragile
than ordinary CST-to-document rules.

Cleanup checklist:

- [ ] Add focused tests for adjacent ignored ranges.
- [ ] Add focused tests for ignore ranges ending at EOF.
- [ ] Add focused tests for ignore ranges surrounded by comments that are not
      formatter control comments.
- [ ] Add CRLF fixtures for ignored ranges.
- [ ] Add tests for ignored ranges inside nested blocks and member bodies.
- [ ] Keep all ignore range math inside `helpers/formatter_ignore.rs` or typed
      accessors; do not spread offset calculations into ordinary rules.

## Fill Rendering Stress Coverage

Group rendering has explicit deep-nesting coverage and uses one flat fit probe
per group. `fill` also appears bounded, but it performs adjacent-pair fit checks
and clones checker state while rendering or fitting entries.

This does not currently look like the old runaway GJF-compatibility problem.
Still, fill-heavy documents deserve a stress test so future helper changes do
not accidentally make pair fitting expensive.

Cleanup checklist:

- [ ] Add a renderer stress test for thousands of fill entries.
- [ ] Add a renderer stress test for fill entries containing nested fitting
      groups.
- [ ] Assert render stats or elapsed-independent structural behavior where
      possible, rather than relying on wall-clock timing.
- [ ] Avoid adding best-fitting or conditional-group primitives to solve fill
      layout concerns.
- [ ] If fill becomes too costly, simplify the fill algorithm rather than adding
      exploratory layout search.

## Traceability Granularity

The DoD audit records broad traceability from helper/rule modules to the style
guide. That is enough for the completed milestone, but future reviewers may need
a tighter map from each style-guide bullet to at least one fixture and one rule
or helper.

This is especially useful now that the formatter has many comment-preservation
rules.

Cleanup checklist:

- [ ] Add a lightweight style-rule-to-fixture matrix, either in the DoD audit or
      a generated report.
- [ ] For each new style-guide bullet, require one or more focused fixtures in
      the same change.
- [ ] Prefer fixture names that reveal the style rule being pinned.
- [ ] When a rule changes, update the style guide first if the policy changed.
- [ ] Keep temporary implementation gaps out of permanent deviation logs.

## Comment Placement Surface Area

The comment model is explicit and formatter-owned, which matches the spec.
However, much of the late implementation work involved preserving specific
separator, delimiter, and moved-construct comments. That pattern is a sign that
comments remain the highest-risk behavior surface.

This is not a reason to port a global Prettier-style attachment engine. It is a
reason to keep consolidating repeated comment shapes.

Cleanup checklist:

- [ ] Look for repeated delimiter-comment patterns after splitting large rule
      modules.
- [ ] Keep moved-construct comment policy close to the construct that moves the
      code.
- [ ] Add narrow fixtures when a construct both moves code and rewrites
      separators.
- [ ] Add fixtures for comments around sorted imports and module directives
      after fixture refreshes introduce new examples.
- [ ] Preserve the current v1 barrier policy for comments inside sortable runs
      unless the style guide changes.
