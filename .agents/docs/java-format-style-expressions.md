# Jolt Java Style: Expressions, Calls, Chains, Operators, Arrays

This document defines Jolt's Java expression style.

## Expression Principles

- Preserve user-authored expression parentheses and required syntactic grouping
  from the parsed tree.
- Do not add readability parentheses in v1. Readability-parentheses policy can
  be added later as an explicitly scoped formatter rule.
- Blank lines inside expression internals are normalized away. Intentional
  blank-line preservation is for bodies, not argument lists or member chains.
- Java and Kotlin share low-level document/list helpers only. Each language owns
  its high-level expression policy.

## Binary And Operator-Like Expressions

- Binary operators wrap at the start of continuation lines.
- Operator continuation lines indent one normal level from the construct that
  owns the expression, even when the operator starts the line.
- Nested operator continuations add another normal indent level.
- When a binary operator chain breaks, every operator in the chain breaks. Keep
  the first operand with the owning construct where possible, then put each
  operator and its right operand on its own continuation line.
- Do not fill the first line with as many operators as possible and only break
  the remaining tail.
- Multi-catch alternatives use the same indented leading-operator shape.
- Operator chains flatten only when doing so is semantics-preserving and covered
  by a finite rule.

```java
boolean allowed =
    user.isActive()
        && account.hasPermission("write")
        && featureFlags.enabled();
```

```java
return x
    + x
    + x
    + x;
```

Inside parenthesized control conditions:

```java
if (
    user.isActive()
        && account.hasPermission("write")
        && featureFlags.enabled()
) {
  run();
}
```

The same indented leading-operator shape applies inside user-authored
parentheses and prefix expressions. Closing delimiters return to the indentation
level of the construct that opened the parenthesized expression.

```java
return !(
    bounds.getLeft() > getRight()
        || bounds.getRight() < getLeft()
        || bounds.getTop() > getBottom()
        || bounds.getBottom() < getTop()
);
```

## Ternaries

- Broken ternaries put `?` and `:` at the start of continuation lines.
- When the condition is itself on a broken continuation line, `?` and `:` are
  nested one normal indent deeper than the condition.
- Preserve user-authored parentheses around ternary expressions.

```java
String label =
    user.isActive()
        ? user.displayName()
        : "inactive";
```

## Assignments

- Assignment left-hand sides and right-hand sides use normal expression
  formatting.
- Long right-hand sides break after the assignment operator only when normal
  expression/list layout requires it.
- Do not introduce a special global fitting search for assignments.

```java
boolean allowed =
    user.isActive()
        && account.hasPermission("write");
```

## Calls And Argument Lists

- Empty argument lists print as `()`, with dangling comments inside if needed.
- If a non-empty argument list fits, keep it inline.
- When an argument list breaks, every argument gets its own line.
- Do not add Prettier-style first-argument or last-argument expansion in v1.
- Broken argument lists put the closing delimiter on its own line.

```java
call(
    user,
    account,
    settings
);
```

Blank lines inside argument lists are normalized away.

## Member Chains

- A contiguous run of dot-separated identifiers with no calls, indexes,
  operators, or other selectors is a dotted identifier run.
- Keep dotted identifier runs tight when they fit. This rule is syntactic: the
  run may be a package/type name, static member path, enum constant path, or
  property chain.
- Do not break inside a dotted identifier run unless the run itself cannot fit
  on a line.
- If a chain containing calls, indexes, or other non-identifier selectors does
  not fit on one line, end the first line at the dotted identifier run when that
  run fits, then put each later selector on an indented continuation line.
- If the receiver is complex or multiline, the first selector moves to the
  continuation line.
- This follows Ruff's general shape rather than Prettier-Java's
  compatibility-specific chain variants.
- Blank lines inside member chains are normalized away.

```java
ImmutableList.builder()
    .add(first)
    .add(second)
    .build();

com.example.deep.config.Factory.DEFAULT_VALUE
    .getNumber();
```

Complex receiver:

```java
veryLongExpressionReturningARepository()
    .findActiveUsers(region, limit)
    .stream()
    .toList();
```

## Lambdas

- Optional parentheses are omitted for a lone untyped lambda parameter.
- Typed, annotated, multiple, or zero parameters use parentheses.
- This does not change the policy of preserving user-authored expression
  parentheses.

```java
users.stream().map(user -> user.name()).toList();
users.stream().map((User user) -> user.name()).toList();
```

- Lambda bodies use normal expression or block formatting.

## Arrays And Initializers

- Empty array initializers print as `{}` only if they are true expression/list
  initializers with no block-body policy attached.
- Non-empty initializers use normal comma-list formatting.
- Inline non-empty initializers include spaces just inside `{` and `}`.
- Broken initializer lists put items one per line and may use trailing
  separators where the surrounding Java construct permits them.

```java
Object[] objects = new Object[] { new Object(), new Object() };

int[] values = {
    1,
    2,
    3,
};
```

## Casts, Instanceof, Patterns

- Cast type lists use normal type/list formatting.
- `instanceof` and pattern expressions use normal expression formatting.
- Switch guards use normal expression formatting and preserve user-authored
  parentheses.
- Wildcards and `_` are exposed as distinct context-specific syntax constructs
  in formatter accessors, such as wildcard type arguments, unnamed variables,
  unnamed lambda parameters, unnamed exception parameters, and unnamed patterns.
  Leaf printers may still render them as simple tokens.

```java
List<?> values;
int _ = compute();
case _ -> handle();
```

## Object Creation

- `new` expressions print annotations, optional type arguments, type, arguments,
  and optional anonymous class body in source syntax order.
- Anonymous class bodies use declaration/block formatting.
- A space separates constructor arguments from an anonymous class body.

## Type Arguments

- Type arguments use the shared bounded angle-bracket list helper.
- Simple type argument lists may remain inline.
- Broken type argument lists put one argument per line and preserve comments.

```java
Map<
    String,
    List<User>
> users;
```

## Accessor Requirements

- Expose stable expression roles: left, operator, right, condition, consequence,
  alternative, receiver, selector, arguments, type arguments, array, index,
  lambda parameters, lambda body, cast types, and initializer.
- Expose parent role queries where layout needs them, such as receiver of a
  member chain or condition of a control statement.
- Expose member chains as a lossless linear view: root plus ordered suffixes.
- Expose whether receivers or child docs are complex/multiline.
- Expose comments and source spans, but keep comment placement owned by the
  construct being formatted.
