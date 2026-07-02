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
