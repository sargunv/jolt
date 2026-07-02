# Jolt Java Style: Declarations, Modifiers, Parameters, Types

This document defines Jolt's Java declaration style.

## Modifier And Annotation Policy

- Modifier keywords are reordered canonically: `public`, `protected`, `private`,
  `abstract`, `default`, `static`, `final`, `transient`, `volatile`,
  `synchronized`, `native`, `sealed`, `non-sealed`, `strictfp`.
- Declaration annotations and type-use annotations are distinct accessor roles.
- Declaration annotations print one per line.
- Type-use annotations remain inline with the annotated type.
- When source syntax does not distinguish an annotation's semantic target, Jolt
  uses a deterministic syntactic rule for typed declarations: annotations before
  the first modifier keyword are declaration annotations; annotations after a
  modifier keyword or after method type parameters are type-use annotations; if
  there is no modifier keyword, annotations before the type are declaration
  annotations.

```java
@Deprecated
@Generated("tool")
public final class User {
}

public @Nonnull String name;

@Override
String displayName() {
  return name;
}
```

## Type Declarations

- Class, interface, enum, record, and annotation-interface declarations use
  grouped headers with ordinary one-indent continuation.
- Type headers use canonical clause ordering:
  - class: type parameters, `extends`, `implements`, `permits`
  - interface: type parameters, `extends`, `permits`
  - record: type parameters, components, `implements`
  - enum: `implements`
- When a type declaration header breaks, put the opening brace on its own line
  unless implementation proves this materially complicates the layout engine.

```java
public final class VeryLongClassName<T>
    extends AbstractBase
    implements FirstInterface,
    SecondInterface
{
  private final Repository repository;
}
```

## Body Declarations

- Body declarations get automatic blank-line padding between member categories,
  capped at one blank line.
- Preserve at most one intentional user blank line in bodies.
- Fields of the same category may remain adjacent.
- Constructors, methods, initializers, nested types, and semantically distinct
  member categories are separated by one blank line.

```java
class User {
  private final String name;
  private final int age;

  User(String name, int age) {
    this.name = name;
    this.age = age;
  }

  String name() {
    return name;
  }
}
```

- Empty statements in type bodies are removed while preserving comments.

## Empty Blocks

- Empty blocks expand with the closing brace on its own line.
- Comments-only blocks are expanded because comments are block contents.

```java
class Empty {
}

void todo() {
  // intentionally empty
}
```

## Parameters And Record Components

- Broken parameter and record-component lists put the closing delimiter on its
  own line, Ruff-style.
- Do not tie `)` to the last item in a broken parameter/component list.
- Record components use ordinary parameter-list layout.
- Declaration annotations on components print one per line only when they are
  declaration annotations; type-use annotations stay inline.

```java
record User(
    String name,
    int age,
    @Nonnull Email email
)
{
  boolean adult() {
    return age >= 18;
  }
}
```

The same closing-delimiter rule applies to methods and constructors:

```java
Result compute(
    Request request,
    ExecutionContext context
)
    throws IOException,
        TimeoutException
{
  return executor.run(request, context);
}
```

## Throws

- When a `throws` clause fits, keep it on the declaration line.
- When a `throws` clause breaks, keep `throws` with the first exception and put
  subsequent exceptions one per continuation line.
- Subsequent exception lines are nested one normal indent deeper than the
  `throws` line.

```java
void run()
    throws IOException,
        SQLException,
        TimeoutException
{
}
```

## Variables And Assignments

- Field, constant, local-variable, and resource declarations share declaration
  pieces where possible: modifiers, type, dimensions, declarators, initializer.
- Multi-declarator declarations may break one declarator per line when any
  declarator has an initializer or when the declaration exceeds width.
- Initializers use normal expression formatting.
- Type-use annotations remain attached to the type or dimension they annotate.

## Type Parameters And Type Arguments

- Simple single type parameters/arguments may stay inline.
- Broken type parameter and argument lists use ordinary comma-list formatting
  inside `<` and `>`.
- When a type-parameter bound list breaks, keep the first bound after `extends`
  and put each additional `&` bound on its own line one normal indent deeper
  than the type-parameter line.
- Comments inside type parameter/argument lists force normal broken-list layout.

```java
class Box<
    T extends Serializable
        & Closeable
        & Flushable,
    U
> {
}
```

## Enums

- Multiline enum constant lists get a trailing comma when the enum has no body
  declarations.

```java
enum Color {
  RED,
  GREEN,
  BLUE,
}
```

- Enums with body declarations use the required semicolon before declarations.

```java
enum Color {
  RED,
  GREEN,
  BLUE;

  boolean primary() {
    return this == RED;
  }
}
```

## Annotation Interfaces And Annotation Values

- Annotation interface declarations follow normal type-declaration rules.
- Annotation type elements follow method-like declaration layout.
- Annotation array values use the same bounded array/list helper as Java array
  initializers unless a later policy needs annotation-specific behavior.

## Accessor Requirements

- Stable declaration roles: name, modifiers, declaration annotations, type-use
  annotations, type parameters, return type, parameters, receiver parameter,
  throws, body, superclass, interfaces, permits, record components, enum
  constants, and body declarations.
- Do not rely on printer-local child-index scans when the wrapper can expose a
  grammar role.
- Expose source spans and comments for body padding and annotation placement.
- Expose empty statements in type bodies so they can be removed deliberately.
