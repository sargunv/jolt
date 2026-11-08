# Java style

This page describes the code Jolt writes. The examples use the default options
(line width 80, two-space indentation); each one is what Jolt prints for it.

## Principles

- **Fit or break.** A construct stays on one line if it fits. Otherwise it
  breaks by its own rule, and its parts may then break by theirs.
- **Lists break all or nothing.** When a list of arguments, parameters, array
  elements or similar doesn't fit, every element goes on its own line, and the
  closing delimiter goes on a line of its own. Jolt never fills a line with as
  many elements as fit.
- **One indentation unit.** Each continuation line is exactly one level deeper
  than the line it continues. Nothing is aligned to a column, and nothing is
  indented twice.
- **Blank lines come from the source.** Jolt keeps at most one blank line where
  the source has one, between members or statements, and adds them only where a
  rule below says so.
- **Code stays the same code.** Besides whitespace and line breaks, Jolt makes
  only the changes listed here: it sorts imports, module directives and
  modifiers; adds braces around single-statement bodies; removes redundant `;`
  and parentheses; adds parentheses around nested bitwise and shift operations;
  adds trailing commas to broken array initializers and enum constants; and
  drops the parentheses around a single lambda parameter. It doesn't remove
  unused imports, rename anything, or change literals.
- **Comments are never lost.** Each comment is printed exactly once and never
  moves across code, except in the few cases [Comments](#comments) describes.

## Compared with google-java-format

Most Java code is formatted by
[google-java-format](https://github.com/google/google-java-format), so these are
the differences readers notice first:

- **Line width and indentation.** google-java-format always uses a line width of
  100 and indents continuation lines by 4. Jolt defaults to 80 and indents
  continuations by the same unit as blocks, both
  [configurable](./configuration.md).
- **Broken lists.** google-java-format puts as many parameters as fit on a
  continuation line and keeps the closing `)` after the last argument. Jolt puts
  each element on its own line and `)` on its own line, so adding or removing an
  element changes one line of a diff.
- **Trailing commas.** Jolt adds a trailing comma to a broken array initializer
  and to enum constants, for the same reason. google-java-format doesn't.
- **Braces.** Jolt adds braces around the body of `if`, `else`, `for`, `while`
  and `do` when it is a single statement; google-java-format leaves
  `if (ready) run();` as written.
- **Annotations.** Jolt puts the annotations of every declaration, including
  fields, on their own lines. google-java-format moves a field's annotations
  onto the declaration line (`@Inject private Foo foo;`).
- **Imports.** Jolt puts static imports after the other imports;
  google-java-format puts them first. google-java-format removes unused imports.
  Jolt doesn't, since that needs knowing which names the code uses.
- **Member chains.** After a simple receiver such as a local variable or a class
  name, Jolt keeps the first call on the receiver's line
  (`builder.withName(name)`). google-java-format starts every call on a new
  line.
- **Javadoc.** google-java-format rewraps Javadoc text, and expands a one-line
  Javadoc that doesn't fit. Jolt keeps the text of every comment as written and
  only realigns the `*` at the start of its lines.
- **Lambdas.** Jolt removes the parentheses around a single untyped lambda
  parameter (`x -> x`).

## Whitespace

- Indentation is two spaces per level by default, or one tab per level with
  `use-tabs`.
- Lines end with `\n` and have no trailing whitespace, except inside text blocks
  and comments. The file ends with exactly one newline, and an empty file
  formats to a single newline.
- There is at most one blank line in a row, and no blank line directly after an
  opening brace or before a closing brace.
- A byte order mark at the start of the file stays first, and a trailing SUB
  (`\u001a`) character stays after the final newline.

## Compilation unit

```java
// file header comment

package com.example.demo;

import a.Alpha;
import z.Zed;

class A {}
```

- Header comments, the package declaration, the imports and the top-level types
  keep their order. A blank line separates the package, the imports and each
  top-level declaration. Blank lines between header comments are kept.
- Package annotations go one per line before `package`.
- A stray `;` between top-level declarations is removed. Comments on it stay
  where they were.
- In a compact source file (`void main() { … }`), the top-level members format
  and are separated like class members, at column 0.

### Imports

::: code-group

```java [Input]
import static z.Zed.z;
import b.Beta;
import static a.Alpha.alpha;

import module java.base;
import a.Alpha;
```

```java [Output]
import a.Alpha;
import b.Beta;
import module java.base;

import static a.Alpha.alpha;
import static z.Zed.z;
```

:::

- Regular imports, including `import module`, come first, then static imports,
  with one blank line between the two groups and no blank lines inside them.
- Each group is sorted by name, character by character and case-sensitively. An
  on-demand import sorts as its name followed by `.*`.
- An import with a comment on the line before it starts a new run: it stays
  where it is, the imports after it sort among themselves, and a blank line
  before the comment is kept. So comments never move away from the imports they
  describe.
- An import with a comment inside its name, or a syntax error, stays in place;
  the imports before and after it sort separately.
- Duplicate imports are kept.
- A comment at the end of an import's line stays with that import:

```java
import a.Alpha; // alpha
import z.Zed; // zed
```

### Modules

```java
module demo {
  requires static transitive com.example.lib;
  requires java.sql;

  exports com.example.api;

  opens com.example.internal;

  uses com.example.Plugin;

  provides com.example.Plugin with com.example.impl.PluginImpl;
}
```

- Directives are grouped by kind, in the order `requires`, `exports`, `opens`,
  `uses`, `provides`, with a blank line between groups, and sorted by name
  within a group.
- `requires` modifiers print as `static transitive`, unless one of them carries
  a comment.
- A directive with a comment on the line before it, or with a syntax error,
  stays in place; the directives before and after it sort separately.
- The name lists after `to` and `with` break all or nothing.

## Declarations

### Annotations and modifiers

```java
@Generated("tool")
@Deprecated
public final class User {
  @FieldAnno(name = "id", values = { 1, 2 })
  private final String id;

  public @Nonnull String name(@Nonnull String prefix) {
    for (@Nonnull String part : parts) {
      use(part);
    }
    return id;
  }
}
```

- Annotations before the first modifier keyword annotate the declaration, and go
  one per line before it. This holds for every declaration, including local
  variables. Annotations after a modifier keyword annotate the type and stay
  inline.
- Variables declared in a header keep their annotations inline: method, lambda
  and `catch` parameters, `try` resources, enhanced `for` variables and type
  patterns.
- Modifier keywords are sorted into this order:

```text
public protected private abstract default static final transient volatile
synchronized native sealed non-sealed strictfp
```

- Annotations keep their order among themselves. A modifier with a comment
  before or after it stays in place, and the keywords on either side sort
  separately (`final /* stable */ public class C`).
- Parameter modifiers (`final` and annotations) keep their order.
- Element value arrays in annotations have a space inside the braces
  (`{ 1, 2 }`). An argument list that doesn't fit breaks like a call's.

### Type declarations

```java
class Derived<T>
  extends base.Parent<String>
  implements
    First, // first
    pkg.Second
  permits
    Child, // child
    other.Child {}

interface Shape extends Drawable, Scalable permits Circle, Square {}
```

- The header, from the modifiers to the opening brace, is one line if it fits.
  Otherwise each `extends`, `implements` and `permits` clause goes on its own
  line one level deeper. A clause whose types don't fit puts its keyword alone
  on the line and each type on its own line one more level deeper.
- When the type parameters break, the clauses follow the closing `>` if they fit
  there (`> extends Base {`).
- The opening brace ends the header's last line.
- Type parameters that don't fit break like a list. Bounds that don't fit break
  before each `&`.

### Members

```java
class Groups {
  int first;

  int second;
  int third;

  Groups() {}

  static {
    initialize();
  }

  void a() {}
}
```

- Members go one per line, with a blank line between them. Two consecutive field
  declarations keep the source's blank line, or its absence.
- A redundant `;` between members is removed. Comments on it stay where they
  were.
- A body with nothing in it prints `{}`: methods, constructors, initializers,
  classes, blocks, lambdas, switches and modules alike. A body that holds only
  comments breaks.

### Enums

```java
enum Mode {
  BASIC,
  @Marker
  SPECIAL(helper(1), new Box()) {
    void run() {}
  },
}

enum Color {
  RED,
  GREEN,
  BLUE;

  private final int code;
}
```

- Constants go one per line. Without members, every constant is followed by a
  comma, including the last, and a `;` after the constants is removed. With
  members, the last constant is followed by `;`, then a blank line and the
  members.
- A comment after the last constant stays on its line, after the comma or `;`:

::: code-group

```java [Input]
enum E {
  A,
  B // last
}

enum F {
  B, // last
  ;
  int code;
}
```

```java [Output]
enum E {
  A,
  B, // last
}

enum F {
  B; // last

  int code;
}
```

:::

- Comments on their own line before the `;` move after it, with a blank line on
  each side.
- Constant annotations go on their own lines. Arguments format like a call's,
  and bodies like a class body.

### Records

```java
record CustomerProfile(
  String displayName,
  CustomerIdentifier primaryIdentifier,
  CommunicationPreferences communicationPreferences
) {}

record Range(int low, int high) {
  public Range {
    check(low, high);
  }
}
```

Components break like a list, and `implements` like a type declaration's clause.

### Methods and constructors

```java
class Service {
  public Result compute(
    Request request,
    ExecutionContext executionContext,
    RetryPolicy retryPolicy
  ) throws IOException, TimeoutException {}

  public void run(Request request, ExecutionContext executionContext)
    throws IOException, TimeoutException {}
}
```

- The signature is one line if it fits. Otherwise the parameters break like a
  list, with `)` at the declaration's indentation.
- A `throws` clause follows the `)` if it fits there. Otherwise it goes on its
  own line one level deeper, and if its types don't fit either, `throws` stands
  alone and each type goes on its own line one more level deeper.
- Type parameters that don't fit break like a list: `public <`, the parameters,
  then `> void run() {`.
- Varargs (`String... values`), receiver parameters (`Foo this`) and array
  dimensions after the name stay as written.
- `super(…)`, `this(…)` and `outer.super(…)` format like calls.
- Annotation interface elements print as `String value() default "x";`.

### Fields and variables

```java
final @Nullable String shortName =
  computeCreationUserAgentForTheCurrentRequestAndSession(request, session);

int firstFieldVariable = 1,
  secondFieldVariable = 2,
  thirdFieldVariable = provider();
```

- An initializer that doesn't fit moves to the next line, as an assignment's
  value does.
- Several declarators stay on one line if they fit. Otherwise each declarator
  after the first goes on its own line, one level deeper.

## Statements

### Blocks

- Statements go one per line. Blank lines between statements are kept, never
  added.
- An empty statement (`;`) in a block or switch group is removed, and comments
  on it stay where they were. A switch group's only `;` stays, since removing it
  would merge two groups.

### Control flow

::: code-group

```java [Input]
if (ready) run();
else if (waiting) pause();
else stop();
```

```java [Output]
if (ready) {
  run();
} else if (waiting) {
  pause();
} else {
  stop();
}
```

:::

```java
if (
  user.isActive()
    && account.hasPermission("write")
    && featureFlags.isEnabled("collaborative-editing")
) {
  run();
}
```

- The bodies of `if`, `else`, `while`, `do`, `for` and enhanced `for` are always
  blocks: Jolt adds braces around a single statement. `else if` chains stay
  flat.
- `} else {`, `} while (…);`, `} catch (…) {` and `} finally {` share the line
  with the closing brace.
- A condition that doesn't fit breaks inside the parentheses: `(` ends the line,
  the condition goes one level deeper, and `)` returns to the statement's
  indentation.
- A `for` header that doesn't fit puts each of its three parts on its own line,
  and `) {` on a line of its own. Empty parts print as `for (;;)`.
- A label goes on its own line before the statement it labels.

### Switch

```java
switch (value) {
  case 1:
  case 2:
    handleNumber();
    break;
  case String s when s.isEmpty() -> handleEmpty(s);
  case String s -> {
    handleString(s);
  }
  default -> throw new IllegalArgumentException();
}
```

- Labels are one level inside the switch, and the statements of a `case …:`
  group one more. Each `case …:` label is on its own line. A group whose only
  statement is a block keeps the block on the label line (`case 1: {`).
- A rule (`case X ->`) whose body doesn't fit breaks after `->`, with the body
  one level deeper.
- Label lists (`case null, default`) and guards (`when …`) stay on the label
  line. Redundant parentheses around a guard are removed.
- A switch expression formats the same way.

### Try

```java
try (var input = open()) {
  use(input);
} catch (java.io.IOException | RuntimeException ex) {
  return 1;
} finally {
  cleanup();
}
try (
  var input = openTheInputStreamWithALongName();
  var output = createTheOutputStream()
) {
  copy(input, output);
}
```

- The resources stay on the `try` line if they fit. Otherwise they break like a
  list: `try (`, one resource per line, then `) {`. A `;` after the last
  resource is removed.
- A `catch` parameter that doesn't fit breaks inside the parentheses, before
  each `|`.

## Expressions

### Assignment

```java
allowed =
  user.isActive()
    && account.hasPermission("write")
    && featureFlags.isEnabled("collaborative-editing");
```

An assignment or initializer that doesn't fit breaks after the operator, and the
value continues one level deeper, where it may break further by its own rule.
This holds for every value, including lambdas, anonymous classes, array
initializers and switch expressions.

### Binary operators

```java
return user.isActive()
  && account.hasPermission("write")
  && featureFlags.enabled()
  && auditPolicy.allows(user);
```

- A chain of operators with the same precedence breaks before every operator,
  one level deeper than the start of the chain.
- Parentheses that only repeat the order of evaluation are removed:
  `((x + x) - x) + x` prints as `x + x - x + x`.
- An operand of `&`, `|`, `^`, `<<`, `>>` or `>>>` that is itself a binary
  operation gets parentheses: `a & b | c` prints as `(a & b) | c`.
- `instanceof` chains like a comparison, and its patterns stay inline
  (`value instanceof String text`).

### Conditional

```java
int j =
  i == 0
    ? 0
    : i == 1
      ? 1
      : i;
```

A conditional that doesn't fit puts `?` and `:` each at the start of a line one
level deeper than the condition. A conditional in the `:` branch nests one level
further.

### Calls

```java
return create(
  user.profile().displayName(),
  account.permissions().primaryRole(),
  settings.region().identifier()
);
```

- Arguments break like a list. A lambda or anonymous class as the last argument
  gets no special treatment.
- Explicit type arguments print without spaces: `this.<String>convert(x)`.
- `new` expressions, including `outer.new Inner(…)`, format like calls.

### Member chains

```java
builder.withFirstValue(first.reallyLongDisplayName())
  .withSecondValue(second.reallyLongDisplayName())
  .build();
return com.example.deep.config.ConflictResolutionFactory
  .newBuilder()
  .withStrategy(strategy)
  .build();
```

- A chain of calls is one line if it fits. Otherwise each call goes on its own
  line starting with `.`, one level deeper than the start of the chain.
- When the receiver is a single name, `this` or `super`, the first call stays on
  its line (`builder.withFirstValue(…)`, `Foo.builder()`). After a qualified
  receiver (`this.a.b`, `com.example.Factory`) the first call starts a new line
  too.

### Lambdas and method references

```java
Function<String, Integer> length = text -> text.length();
BinaryOperator<Integer> sum = (left, right) -> left + right;
Runnable task = () -> {};
Supplier<String> name = User::name;
```

- A single parameter without a type, annotation or modifier loses its
  parentheses: `(x) -> x` prints as `x -> x`.
- Parameters that don't fit break like a list. Empty parentheses `()` never
  break.
- An expression body stays on the arrow's line when it fits, and a block body
  opens there.
- Method references have no spaces: `String::length`, `int[]::new`.

### Arrays

```java
String[] pair = new String[] { "a", "b" };
int[] empty = new int[] {};
String[] labels =
  {
    "a very long label that forces the initializer to break across lines",
    "another very long label that keeps each item on its own line",
  };
```

- Initializers have a space inside the braces when they fit on one line
  (`{ 1, 2 }`), and print `{}` when empty.
- A broken initializer puts each element on its own line followed by a comma,
  and `}` on its own line. A trailing comma in the source keeps it broken.

### Other expressions

- Casts: `(String) value`, `(Runnable & AutoCloseable) value`.
- Unary operators attach to their operand (`!ready`, `-x`, `i++`), with a space
  only where two operators would otherwise merge (`- -x`).
- Parenthesized expressions that don't fit break like a call.
- Literals print exactly as written. Text blocks keep their content and closing
  delimiter byte for byte, and the line holding a text block always breaks
  around it.

### Types

- Annotations stay inline in types: `java.util.@Marker List<@Marker String>`.
- Type arguments have no spaces inside `<>`, and break like a list when they
  don't fit.
- Wildcards print as `? extends Number` and `? super T`. Intersection bounds
  break before `&`.

## Comments

Comments stay where they are relative to the code, with the exceptions in this
section, which keep a comment next to the code it describes.

- **Own line.** A comment that starts its line keeps its own line, at the
  indentation of the code after it. A blank line after it is kept. Comments
  before a member or statement stay attached to it.
- **End of line.** A comment that ends its line stays at the end of the line. A
  `,` or `;` after it moves before it, so a comma or semicolon never starts a
  line. A line comment always ends the line, and counts toward the line width.
- **Inline.** A block comment between code on one line stays inline, with one
  space on each side, except none after an opening bracket, `.`, `::` or `@`,
  and none before a closing bracket, `,` or `;`.
- **After an opening bracket.** A comment that ends the line after `{`, `(`, `[`
  or `<` moves to the first line inside.
- **Before a header clause.** A block comment that ends the line before
  `extends`, `implements`, `permits` or `throws` goes with the clause, onto the
  clause's line.
- **Between a header and its body.** A comment between a declaration or control
  statement header and its body moves to the first line inside the body, unless
  it is inline (`class X /* c */ {` stays as written).

::: code-group

```java [Input]
class Example extends Base // base
{
  void call(String name /* the name */) {
    if (ready) // fast path
      run(
        name, // first
        other /* second */);
  }
}
```

```java [Output]
class Example extends Base {
  // base
  void call(String name /* the name */) {
    if (ready) {
      // fast path
      run(
        name, // first
        other /* second */
      );
    }
  }
}
```

:::

- **After an operator or keyword.** A comment that ends the line after `=`,
  `return`, `throw`, `yield`, `break`, `continue` or `assert` stays there, and
  the value continues on the next line one level deeper:

```java
int field = // a comment ends the line after the operator
  1;
return // a comment ends the line after the keyword
  someLongMethodCall(firstArgumentName, secondArgumentName)
    + anotherMethodCall(thirdArgumentName);
```

- **In an empty body or list.** Comments in an otherwise empty body or list, and
  comments on their own lines before a closing bracket, go inside the brackets
  one per line, one level deeper. Comments that shared a line keep sharing it:

```java
log(
  // no arguments
);
```

- **Comment text.** Trailing whitespace is removed. Jolt doesn't rewrap or
  otherwise change the text of a comment. A block comment whose lines start with
  `*` is realigned so the `*`s line up under the opening `/*`, and each line
  keeps its text after the `*`, so code samples in `<pre>` keep their
  indentation. Other multi-line block comments move as a unit, keeping their
  lines' relative indentation. A multi-line Javadoc comment starts its text on
  the line after `/**`:

::: code-group

```java [Input]
/** Gets the name. */
String name();

    /** Runs it:
       * <pre>
       *   if (ready) {
       *     run();
       *   }
       * </pre>
       */
void run();
```

```java [Output]
/** Gets the name. */
String name();

/**
 * Runs it:
 * <pre>
 *   if (ready) {
 *     run();
 *   }
 * </pre>
 */
void run();
```

:::

## Formatter-ignore

```java
// @formatter:off
int x=       1+2;
int[] identity = {1,0,
                  0,1};
// @formatter:on
int y = 3;
```

- `// @formatter:off` and `// @formatter:on`, or the same text in block
  comments, turn formatting off and back on. Other markers, such as
  `// prettier-ignore`, have no effect.
- Formatting turns off for whole members, statements, top-level declarations and
  module directives whose first token is between the markers. Each prints
  exactly as written, shifted to the current indentation. Text blocks are never
  shifted. The markers, the comments between items, and the blank lines between
  them are formatted as usual.
- Markers work only between the items of one body or at the top level. An `off`
  whose `on` is in a different body has no effect, and neither has that `on`. A
  marker inside an item, such as in an argument list, has no effect.
- An `off` without a later `on` lasts to the end of its body, or to the end of
  the file at the top level.

## Code with syntax errors

The CLI and the dprint plugin refuse files with syntax errors, unless told to
format them anyway (`--format-with-errors`). Then everything Jolt can parse is
formatted by the rules above, and the code around each error prints exactly as
written. Jolt never adds code to repair an error and never drops any.

- A list or header that isn't closed ends where its statement or body does: in
  `f(;` followed by `g();`, `g();` stays a statement of its own.
- Nothing is printed where a missing closing bracket would go, so what follows
  continues on the line of the last element, as in the source.
- Stray `;`s stay in a body or switch group that has an error.
