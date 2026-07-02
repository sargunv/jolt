# Jolt Java Style: Statements, Blocks, Switch, Try/Resources

This document defines Jolt's Java statement style.

## Blocks

- Non-empty blocks print with braces on their own structural lines and contents
  indented one level.
- Empty blocks expand with the closing brace on its own line.
- Comments-only blocks are expanded because comments are content.
- Preserve at most one intentional blank line inside bodies.
- Standalone empty statements inside blocks are removed unless comments need to
  be preserved.

```java
void run() {
  prepare();

  execute();
}

void empty() {
}
```

## Braced Bodies

- Unbraced statement bodies are normalized to braced bodies.
- Empty statement loop/control bodies normalize to braced empty blocks.

```java
if (ready) {
  run();
}

while (ready) {
}
```

- This policy applies to `if`, `else`, loops, and other control-flow bodies
  where Java permits unbraced statements.

## Labels

- Labeled statements put the label on its own line.
- The labeled body does not gain an extra indentation scope solely because of
  the label.

```java
retry:
for (;;) {
  run();
}
```

## If/Else

- Conditions use normal parenthesized expression formatting.
- Consequences and alternatives are braced.
- `else if` chains stay as `else if`.
- Comments between a then branch and `else` print according to comment
  attachment and block ownership.

```java
if (ready) {
  run();
} else if (waiting) {
  pause();
} else {
  stop();
}
```

## Loops

- Loop bodies are braced.
- `do` loops use the normal Java shape with the `while` clause after the body.
- Empty loop bodies become empty blocks.

```java
do {
  run();
} while (ready);
```

## For Headers

- If a `for` header fits, keep it on one line.
- Empty `for` control headers print as `for (;;)`, never as a broken header.
- When it breaks, split first into init, condition, and update segments.
- Each segment then uses normal declaration, expression, or list formatting only
  when that segment itself needs to break.

```java
for (;;) {
  run();
}
```

```java
for (
    int i = 0;
    i < count;
    i++
) {
  run(i);
}
```

Long segments break internally:

```java
for (
    int index = startIndex,
    limit = computeLimit(input);
    index < limit
    && shouldContinue(index);
    index++,
    processed++
) {
  run(index);
}
```

## Switch

- Switch selectors use normal parenthesized expression formatting.
- Long multi-value `case` labels keep the first value with `case`; subsequent
  values continue as an indented comma list.

```java
switch (value) {
  case FIRST,
      SECOND,
      THIRD -> handle();
}
```

- A single block body in a colon switch case stays on the same line as the case
  label.

```java
switch (kind) {
  case USER: {
    handleUser();
    break;
  }
}
```

- Switch guards use normal expression formatting and preserve user-authored
  parentheses.

```java
case User user when user.isActive()
    && user.hasPermission("write") -> handle(user);
```

## Return, Throw, Yield

- `return`, `throw`, and `yield` arguments use normal expression formatting.
- Preserve user-authored parentheses.
- Do not add break-only parentheses as a special return-like statement rule.

```java
return user.isActive()
    && account.hasPermission("write")
    && featureFlags.enabled();
```

If binary indentation policy changes, this example follows that expression
policy.

## Try, Catch, Finally

- `try`, `catch`, and `finally` clauses form a clause chain with spaces between
  adjacent braced clauses.
- Catch parameters use normal parameter/declaration formatting.
- Multi-catch alternatives use the same indented leading-operator shape as
  binary expressions.

```java
try {
  run();
} catch (
    IOException
        | SQLException
        | TimeoutException e
) {
  recover(e);
} finally {
  cleanup();
}
```

## Try With Resources

- Resource declarations use normal declaration/assignment formatting.
- Resource lists break like semicolon-separated lists.
- Optional trailing semicolons in try-with-resources resource lists are removed.

```java
try (
    Connection c = open();
    Statement s = c.createStatement()
) {
  run(s);
}
```

## Assert, Break, Continue

- `assert` prints as `assert <condition>;` or `assert <condition> : <detail>;`.
- `break` and `continue` print their optional label followed by `;`.

## Accessor Requirements

- Expose statement body kind directly: block, empty semicolon, or statement.
- Expose switch entries as labels/rules/groups rather than raw child scans.
- Expose `for` header segments and try-with-resources resource separators.
- Expose whether optional semicolons were present when policy needs to remove
  them.
- Expose comments and source spans for blank-line preservation and comment
  placement.
