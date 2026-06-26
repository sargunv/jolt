# Java Parser Review Notes

Review performed after commit
`04f6aa5 Model shared diagnostics and syntax
outcomes`.

## Highest Priority

- Java spec correctness: the parser accepts several invalid Java constructs
  cleanly.
  - Arbitrary expressions can be called like functions, such as `(f)();`,
    `this();` in a method, or `new C()();`.
  - Assignment left-hand sides are not grammar-checked, so `1 = x;`,
    `a + b = c;`, and `(a) = b;` can parse as clean assignments.
  - Object creation accepts missing or impossible constructor syntax, such as
    `new C;`, `new C {}`, and `new int()`.
  - Enhanced-for and resource declarations reuse full local-variable declaration
    parsing where Java requires a narrower single-variable shape.

- Recovered diagnostics are not consistently represented in CST shape. Some
  recovery paths emit diagnostics without an `ErrorNode`, which will make
  recovered formatting or CST wrappers unable to identify malformed regions from
  tree shape alone.

## Medium Priority

- Reference-type-only grammar sites accept unrestricted `parse_type`, including
  primitive or array types in type bounds, catch types, throws clauses, and
  `instanceof`.

- Parameter-list grammar is too permissive. It does not enforce last-varargs,
  and lambda parameter parsing accepts invalid mixed forms such as
  `(x, int y) -> y` or `(var x, y) -> y`.

- Array creation accepts illegal dimension and initializer combinations, such as
  `new int[][3]` and `new int[3] { 1, 2 }`.

- `SyntaxOutcome::Recovered` is currently derived from any diagnostic. That is
  acceptable while Java syntax diagnostics are all syntax-affecting errors, but
  the outcome should eventually be derived from syntax validity facts rather
  than the mere presence of diagnostics once warnings or notes exist.

- Parser organization is showing strain:
  - grammar concepts live in `source.rs` next to token cursor and event
    machinery;
  - `include!` flattens all grammar files into one namespace;
  - top-level delimiter scanning is duplicated in resource detection, for-header
    detection, switch-label detection, and member-header scanning.

## Lower Priority / Watchlist

- The public Java syntax surface currently exposes raw CST aliases and
  `JavaSyntaxKind`. This is expected until typed wrappers exist, but downstream
  consumers should avoid treating the raw shape as stable.

- Dangling/comment-sensitive trivia currently relies on leading/trailing token
  attachment. This will need a clear formatter-facing access story, but it is
  not a Java parser correctness blocker yet.

- The claim that there is no corpus parser gate is false. The parser fixture
  test parses the imported oracle Java inputs in
  `crates/jolt_java_syntax/tests/parser_fixtures.rs`.
