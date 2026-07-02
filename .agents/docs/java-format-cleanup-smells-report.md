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
