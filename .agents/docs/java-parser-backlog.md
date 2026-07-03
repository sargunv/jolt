# Java Parser Backlog

This document records valid or preview Java syntax that appears in imported
formatter corpus fixtures but currently produces Jolt parser or lexer
diagnostics. It intentionally excludes upstream inputs that are invalid Java and
formatter fixture fragments that are not complete Java compilation-unit syntax.
It also records explicit parser recovery audits when formatter behavior depends
on useful CST shapes for incomplete edit-time syntax.

Each entry includes the smallest useful sample, fixture pointers, and the
specification source that should guide parser work.

## Java 14 Switch-Rule Lambda Results

Status: parser backlog.

Current fixtures:

- `tools/import/.imports/prettier-java/input/lambda/arrow-parens-always/arrow-parens-always.java`
- `tools/import/.imports/prettier-java/input/lambda/arrow-parens-avoid/arrow-parens-avoid.java`

Sample:

```java
import java.util.function.Function;

class Example {
  Function<Integer, Integer> fn(Object value) {
    return switch (value) {
      case Integer base -> x -> x + base;
      default -> x -> x;
    };
  }
}
```

Why this is valid:

- Switch expressions and switch rules are final in Java 14.
- A switch rule expression can be an expression, and a lambda expression is an
  expression when a target type is available.

Implementation note:

The parser should treat the expression after a switch-rule `->` as a full
expression, including a lambda expression. This requires keeping the switch-rule
arrow distinct from a lambda arrow in the rule expression.

Spec links:

- JEP 361, Switch Expressions: <https://openjdk.org/jeps/361>
- JLS 14.11.1, Switch Blocks:
  <https://docs.oracle.com/javase/specs/jls/se21/html/jls-14.html#jls-14.11.1>
- JLS 15.27, Lambda Expressions:
  <https://docs.oracle.com/javase/specs/jls/se21/html/jls-15.html#jls-15.27>
- JLS 15.28, Switch Expressions:
  <https://docs.oracle.com/javase/specs/jls/se21/html/jls-15.html#jls-15.28>

## Java 21 Pattern-Switch Guards With Lambda Results

Status: parser backlog.

Current fixtures:

- `tools/import/.imports/prettier-java/input/lambda/arrow-parens-always/arrow-parens-always.java`
- `tools/import/.imports/prettier-java/input/lambda/arrow-parens-avoid/arrow-parens-avoid.java`

Sample:

```java
import java.util.function.Function;

class Example {
  Function<Integer, Integer> fn(Object value) {
    return switch (value) {
      case Boolean enabled when enabled -> x -> x + 1;
      default -> x -> x;
    };
  }
}
```

Why this is valid:

- Pattern matching for `switch`, including guarded case labels, is final in
  Java 21.
- The same Java 14 switch-rule expression handling applies after the guarded
  label.

Implementation note:

The parser should accept guarded pattern case labels before parsing the
switch-rule expression. The rule expression can then be a target-typed lambda.

Spec links:

- JEP 441, Pattern Matching for switch: <https://openjdk.org/jeps/441>
- JLS 14.11.1, Switch Blocks:
  <https://docs.oracle.com/javase/specs/jls/se21/html/jls-14.html#jls-14.11.1>
- JLS 14.30, Patterns:
  <https://docs.oracle.com/javase/specs/jls/se21/html/jls-14.html#jls-14.30>
- JLS 15.27, Lambda Expressions:
  <https://docs.oracle.com/javase/specs/jls/se21/html/jls-15.html#jls-15.27>

## Flexible Constructor Bodies

Status: unsupported Java version syntax.

Current fixture:

- `tools/import/.imports/prettier-java/input/constructors/constructors.java`

Sample:

```java
class Base {
  Base(int value) {}
}

class Derived extends Base {
  Derived(String text) {
    int value = Integer.parseInt(text);
    super(value);
  }
}
```

Why this is valid:

- This syntax was previewed in Java 22 as statements before `super(...)`.
- The feature was finalized as flexible constructor bodies in Java 25.

Implementation note:

The parser currently reports `java.parse.misplaced_constructor_invocation` when
an explicit constructor invocation is not the first constructor-body statement.
Supporting this feature requires parsing constructor bodies as a prologue,
explicit constructor invocation, and epilogue, while leaving semantic
restrictions to later analysis.

Spec links:

- JEP 447, Statements before super(...) (Preview):
  <https://openjdk.org/jeps/447>
- JEP 513, Flexible Constructor Bodies: <https://openjdk.org/jeps/513>
- Java 22 preview JLS, Statements Before super(...):
  <https://docs.oracle.com/javase/specs/jls/se22/preview/specs/statements-before-super-jls.html>
- JLS 8.8.7, Constructor Body:
  <https://docs.oracle.com/javase/specs/jls/se25/html/jls-8.html#jls-8.8.7>

## String Templates

Status: unsupported preview syntax.

Current fixture:

- `tools/import/.imports/prettier-java/input/template-expression/template-expression.java`

Sample:

```java
class Example {
  String greeting(String name) {
    return STR."Hello \{name}";
  }
}
```

Why this is valid:

- String templates were previewed in Java 21 and re-previewed in Java 22.
- The feature was withdrawn after Java 22, so this is preview-only syntax rather
  than a current final Java feature.

Implementation note:

The lexer currently sees template embedded-expression markers such as `\{` as
ordinary string escape content and reports lexer diagnostics before parsing can
recover. Supporting this feature starts in the lexer with template tokens and
continues in the parser with embedded expression handling.

Spec links:

- JEP 430, String Templates (Preview): <https://openjdk.org/jeps/430>
- JEP 459, String Templates (Second Preview): <https://openjdk.org/jeps/459>
- Java 22 preview JLS, String Templates:
  <https://docs.oracle.com/javase/specs/jls/se22/preview/specs/string-templates-jls.html>

## Selector Relational Expressions

Status: parser backlog.

Current benchmark corpus:

- `tools/bench/bench.py`, `realistic` corpus excludes grouped under "relational
  expressions whose left side is a selector, array access, or call" for Spring
  Framework `v7.0.8`.

Sample:

```java
class Example {
  int index;

  boolean hasMore(String[] names) {
    return this.index < names.length;
  }
}
```

Why this is valid:

- Relational expressions accept any numeric expression on either side.
- Field access, array access, and method invocation are valid primary
  expressions.

Implementation note:

The parser currently accepts simple `x < 0` but reports
`java.parse.expected_syntax: expected type` for examples such as
`this.index < names.length` and `names.length < 2`. The expression parser is
likely treating `<` after a selector-like left-hand side as type-argument syntax
instead of a relational operator.

Spec links:

- JLS 15.10.3, Array Access Expressions:
  <https://docs.oracle.com/javase/specs/jls/se21/html/jls-15.html#jls-15.10.3>
- JLS 15.11, Field Access Expressions:
  <https://docs.oracle.com/javase/specs/jls/se21/html/jls-15.html#jls-15.11>
- JLS 15.20, Relational Operators:
  <https://docs.oracle.com/javase/specs/jls/se21/html/jls-15.html#jls-15.20>

## Array Types And Patterns In Instanceof

Status: parser backlog.

Current benchmark corpus:

- `tools/bench/bench.py`, `realistic` corpus excludes grouped under "array types
  and pattern variables in `instanceof`" for Spring Framework `v7.0.8`.

Sample:

```java
class Example {
  boolean isBytes(Object value) {
    return value instanceof byte[];
  }

  boolean isString(Object value) {
    return value instanceof String string && !string.isEmpty();
  }
}
```

Why this is valid:

- The legacy `instanceof` operator accepts reference types, including array
  types such as `byte[]` and `String[]`.
- Pattern matching for `instanceof` is final in Java 16.

Implementation note:

The parser currently reports `expected class or interface type` on array
instanceof checks and also trips on some type patterns with binding variables.
The fix should keep legacy type tests and pattern tests distinct while allowing
array reference types in the legacy branch.

Spec links:

- JEP 394, Pattern Matching for instanceof: <https://openjdk.org/jeps/394>
- JLS 10.1, Array Types:
  <https://docs.oracle.com/javase/specs/jls/se21/html/jls-10.html#jls-10.1>
- JLS 15.20.2, Type Comparison Operator instanceof:
  <https://docs.oracle.com/javase/specs/jls/se21/html/jls-15.html#jls-15.20.2>

## Explicit This And Super Constructor Invocations

Status: parser backlog.

Current benchmark corpus:

- `tools/bench/bench.py`, `realistic` corpus excludes grouped under "explicit
  constructor invocations using `this(...)` or `super(...)`" for Spring
  Framework `v7.0.8`.

Sample:

```java
class Example extends RuntimeException {
  Example(String name) {
    this(name, null);
  }

  Example(String name, Throwable cause) {
    super("Missing " + name, cause);
  }
}
```

Why this is valid:

- Java constructor bodies may begin with an explicit constructor invocation:
  either an alternate constructor invocation `this(...)` or a superclass
  constructor invocation `super(...)`.

Implementation note:

The parser currently reports `java.parse.invalid_statement_expression` on Spring
constructors whose first statement is `this(...)` or `super(...)`. This is
distinct from flexible constructor bodies: these examples are ordinary
pre-Java-22 constructor bodies where the invocation already appears first.

Spec links:

- JLS 8.8.7.1, Explicit Constructor Invocations:
  <https://docs.oracle.com/javase/specs/jls/se21/html/jls-8.html#jls-8.8.7.1>

## Yield Statements In Switch Expressions

Status: parser backlog.

Current benchmark corpus:

- `tools/bench/bench.py`, `realistic` corpus excludes grouped under "`yield`
  statements in switch expression block rules" for Spring Framework `v7.0.8`.

Sample:

```java
class Example {
  Object scope(int value) {
    return switch (value) {
      case 1 -> {
        yield "one";
      }
      default -> "other";
    };
  }
}
```

Why this is valid:

- Switch expressions are final in Java 14.
- A switch expression block can produce its value using a `yield` statement.

Implementation note:

The parser currently reports
`java.parse.unqualified_yield_method_invocation: unqualified yield method
invocation is not allowed`
for valid `yield` statements inside switch expression block rules. The statement
parser should recognize `yield` in the switch-expression context before falling
back to expression-statement parsing.

Spec links:

- JEP 361, Switch Expressions: <https://openjdk.org/jeps/361>
- JLS 14.21, The yield Statement:
  <https://docs.oracle.com/javase/specs/jls/se21/html/jls-14.html#jls-14.21>
- JLS 15.28, Switch Expressions:
  <https://docs.oracle.com/javase/specs/jls/se21/html/jls-15.html#jls-15.28>

## Generic Array Constructor References

Status: parser backlog.

Current benchmark corpus:

- `tools/bench/bench.py`, `realistic` corpus excludes grouped under "generic
  array constructor references" for Spring Framework `v7.0.8`.

Sample:

```java
import java.util.Set;

class Example {
  Class<?>[] copy(Set<Class<?>> classes) {
    return classes.toArray(Class<?>[]::new);
  }
}
```

Why this is valid:

- Constructor references include array constructor references.
- The array component type can be parameterized or contain wildcards, as in
  `Class<?>[]::new`.

Implementation note:

The parser currently reports `expected expression` around `Class<?>[]::new`.
Method-reference parsing should accept array types before `::new`, including
generic component types and wildcard type arguments.

Spec links:

- JLS 15.13, Method Reference Expressions:
  <https://docs.oracle.com/javase/specs/jls/se21/html/jls-15.html#jls-15.13>
- JLS 15.13.1, Compile-Time Declaration of a Method Reference:
  <https://docs.oracle.com/javase/specs/jls/se21/html/jls-15.html#jls-15.13.1>

## Recoverable Parse Opportunity Audit

Status: parser recovery audit.

Current trigger:

- Formatter cleanup for declaration nodes with missing required names made the
  straightforward constructor, method, annotation element, and enum constant
  holes recover as structural CST nodes instead of opaque error-node fragments.

Sample:

```java
class Example {
  <T>() {}
  void () {}
}

@interface AnnotationExample {
  int ();
}

enum EnumExample { , }
```

Why this matters:

- The public formatter may still refuse to write recovered parses by default.
- The lower-level formatter rules should nevertheless receive useful CST shapes
  when the parser can confidently recognize the surrounding construct.
- Auto-format while typing and future explicit recovered-formatting modes depend
  on parser recovery producing structured holes instead of broad skipped
  regions.

Audit note:

Search declaration, statement, type, pattern, and expression branch predicates
for lookahead that rejects an otherwise recognizable construct only because one
required child is missing. For each narrow and unambiguous edit-time shape,
prefer entering the normal parser production and letting `expect_*` emit the
diagnostic over wrapping the whole fragment in an `ErrorNode`.
