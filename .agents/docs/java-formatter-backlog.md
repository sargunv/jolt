# Java Formatter Backlog

This document records valid Java formatting cases that appear in imported
formatter corpus fixtures but are not yet settled Jolt style contracts. Entries
here should graduate into style-guide rules and focused fixtures only after the
formatting policy is decided.

## Continuation Indentation for Broken Declaration and Expression Groups

Status: failing style fixtures; formatter backlog.

Current fixtures:

- `crates/jolt_java_fmt/tests/style/declarations/broken-method-parameters-and-throws.input.java`
- `crates/jolt_java_fmt/tests/style/declarations/throws-continuation.input.java`
- `crates/jolt_java_fmt/tests/style/declarations/type-parameter-bounds.input.java`
- `crates/jolt_java_fmt/tests/style/declarations/types-and-type-arguments.input.java`
- `crates/jolt_java_fmt/tests/style/expressions/operators-and-ternaries.input.java`
- `crates/jolt_java_fmt/tests/style/expressions/unary-parenthesized-operators.input.java`
- `crates/jolt_java_fmt/tests/style/expressions/member-chains.input.java`

Observed gaps:

- Broken `throws` lists currently align every exception at the same indentation
  as the first exception after `throws`; expected output gives later exceptions
  an additional continuation indent, including when trivia is attached to the
  first exception.
- Broken type-parameter bounds currently print `&` clauses at the same
  indentation as the `extends` line; expected output indents each continued
  bound under the bound list.
- Broken binary and ternary groups inside assignments, `if` conditions, and
  parenthesized unary operands currently lose the extra continuation indentation
  required after the surrounding construct has already broken.
- Member chains following `return` currently get an extra two spaces compared to
  expression-statement chains.

Why this needs a formatter decision:

- These are all indentation policy gaps rather than parse or trivia-loss bugs.
- The expected style distinguishes top-level continuation under a construct from
  sibling alignment within the group, especially after `throws`, `extends`, `&`,
  `&&`, `?`, `:`, and `.` tokens.
- Implementing this likely needs a consistent way to pass surrounding break
  context into list, operator, and chain renderers without ad hoc per-node
  offsets.

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

## Array Initializer Compact and Broken Forms

Status: failing style fixtures; formatter backlog.

Current fixtures:

- `crates/jolt_java_fmt/tests/style/declarations/modifiers-and-annotations.input.java`
- `crates/jolt_java_fmt/tests/style/expressions/array-access-and-creation.input.java`
- `crates/jolt_java_fmt/tests/style/expressions/casts-instanceof-arrays.input.java`
- `crates/jolt_java_fmt/tests/style/expressions/long-array-initializer.input.java`

Observed gaps:

- Compact non-empty array initializers currently print without interior spaces,
  for example `{1, 2}` instead of `{ 1, 2 }`.
- Array initializers inside annotations have the same spacing issue.
- Long array initializers attached to a declaration can remain compact on the
  continuation line, including trailing comma placement, instead of expanding
  one element per line.

Why this needs a formatter decision:

- Empty array initializers already stay compact as `{}`; non-empty compact
  initializers need a separate spacing rule.
- Long initializer breaking depends on both the initializer width and the
  surrounding declaration or assignment boundary.
- Annotation element arrays and expression arrays should share the same brace
  and element-list policy unless the style guide explicitly splits them.

## Dotted Identifier Runs and Member-Chain Break Shape

Status: failing style fixtures; formatter backlog.

Current fixtures:

- `crates/jolt_java_fmt/tests/style/expressions/dotted-identifier-runs.input.java`
- `crates/jolt_java_fmt/tests/style/expressions/member-chains.input.java`

Observed gaps:

- Long dotted constants such as
  `com.example.deep.config.ConflictResolutionFactory.DEFAULT_VALUE.getNumber()`
  currently break at every dot.
- Expected output keeps the package/type/constant run together and breaks only
  before the terminal method call when possible.
- Return-statement member chains currently indent selectors more deeply than the
  same chain used as an expression statement.

Why this needs a formatter decision:

- Java uses dots for both qualified names and fluent method chains, but the
  style expectations treat those as different break units.
- The formatter needs to identify stable dotted-name prefixes without losing
  comments around dots or changing method-chain behavior.

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
