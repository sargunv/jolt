# Java Formatter Backlog

This document records valid Java formatting cases that appear in imported
formatter corpus fixtures but are not yet settled Jolt style contracts. Entries
here should graduate into style-guide rules and focused fixtures only after the
formatting policy is decided.

## Fit Boundaries for Long Declarations and Arrow Expressions

Status: failing style fixtures; formatter backlog.

Current fixtures:

- `crates/jolt_java_fmt/tests/style/declarations/long-array-dimensions.input.java`
- `crates/jolt_java_fmt/tests/style/declarations/long-variable-declarator.input.java`
- `crates/jolt_java_fmt/tests/style/declarations/type-header-clauses.input.java`
- `crates/jolt_java_fmt/tests/style/expressions/assignment-ternary-continuation.input.java`
- `crates/jolt_java_fmt/tests/style/expressions/operators-and-ternaries.input.java`
- `crates/jolt_java_fmt/tests/style/statements/switch-rule-arrow-break.input.java`

Observed gaps:

- Long field and local declarations keep the type/modifiers and declarator name
  on one line even when the complete declaration exceeds the line width.
- Long array dimensions on types and declarators are not considered a useful
  split point.
- Single long `implements` or `permits` clauses stay after the keyword instead
  of moving to the next indented line; long combined clauses also stay packed.
- Assignment RHS ternaries can stay flat after a broken `=` even when the style
  expects the ternary arms to break.
- Long switch rule expressions stay on the `default ->` line instead of breaking
  after the arrow.
- Long flattened binary groups may keep an overlong prefix flat and only break
  late in the group, rather than breaking each operand consistently once the
  group exceeds the boundary.

Why this needs a formatter decision:

- These cases require measuring complete declaration, clause, arrow-expression,
  and operator-group boundaries rather than only local subexpressions.
- The style-guide change appears to prefer early, semantically meaningful break
  points over partially-flat overflow.
- The renderer should stay linear or explicitly bounded; any improved fitting
  must avoid unbounded best-fit search.

## Mixed-Precedence Binary Expressions

Status: formatter backlog.

Current fixture:

- `.fixtures/fixtures/palantir-java-format/input/I.java`

Sample:

```java
class Example {
  void run() {
    int x =
      0 >>> 0 + 0 / 0 * 0 - 0 & 0 << 0 * 0 / 0 >> 0 - 0
      ^ 0 * 0 / 0 >>> 0 << 0 * 0 - 0 / 0
      | 0 * 0 >> 0 + 0 / 0 * 0 - 0 << 0
      & 0 * 0 / 0 >>> 0 - 0 * 0 >> 0 / 0 << 0 * 0 + 0 - 0 / 0 * 0
      | 0 - 0 * 0 >>> 0 << 0 / 0 * 0 >> 0 - 0 ^ 0 * 0 / 0 & 0 << 0 + 0;
  }
}
```

Why this needs a formatter decision:

- The expression mixes shift, additive, multiplicative, bitwise-and,
  bitwise-xor, and bitwise-or precedence levels.
- Jolt currently preserves the parsed tree and does not add readability
  parentheses in v1.
- The current output is legal and deterministic, but it is hard to visually
  audit because line breaks alone do not make the precedence structure obvious.

Reference comparison:

The JavaScript equivalent run through `oxfmt` adds explicit parentheses around
precedence groups and breaks primarily around the top-level `|` groups. That is
useful as a readability reference, but adopting that exact shape would require a
Jolt policy change because it introduces formatter-authored parentheses.

Open questions:

- Should Jolt keep mixed-precedence binary expressions parenthesis-free in v1,
  even when they become hard to audit?
- If readability parentheses are allowed later, which mixed-precedence operators
  justify them?
- Should non-flattened binary trees use stronger indentation to expose
  precedence without changing tokens?
