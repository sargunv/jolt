# Java Formatter Backlog

This document records valid Java formatting cases that appear in imported
formatter corpus fixtures but are not yet settled Jolt style contracts. Entries
here should graduate into style-guide rules and focused fixtures only after the
formatting policy is decided.

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
- If readability parentheses are allowed later, which mixed-precedence
  operators justify them?
- Should non-flattened binary trees use stronger indentation to expose
  precedence without changing tokens?
