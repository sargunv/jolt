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
- **Assignments.** When a value doesn't fit after `=`, google-java-format moves
  it to the next line, even when it can't fit there either, such as a lambda
  with a block body (`Runnable task =` / `() -> {`). Jolt moves a value to the
  next line only if that makes it fit; otherwise the value starts after `=` and
  breaks there (`Runnable task = () -> {`).
- **Member chains.** After a simple receiver such as a local variable or a class
  name, Jolt keeps the first call on the receiver's line
  (`builder.withName(name)`); google-java-format breaks a chain of several calls
  before each call (`builder` / `.withName(name)`). A single call after a call
  stays on the line of that call's `)` when moving it doesn't make it fit
  (`assertThat(` / … / `).isEqualTo(3);`), where google-java-format moves it to
  a line of its own (`)` / `.isEqualTo(3);`).
- **Javadoc.** Both fill Javadoc text and lay out its blocks in much the same
  style (see [Doc comments](#doc-comments)). google-java-format removes `</li>`
  and a `<p>` that starts the comment, adds `<p>` after a blank line (which
  changes the rendered page), joins a plain-text list into one paragraph,
  removes the indentation that the lines of `<pre>{@code` share, removes a tab
  after `*` in `<pre>`, breaks a line between `{@link` and its reference, and
  moves a Javadoc comment between code (`foo(/** c */ 1)`) to a line of its own.
  Jolt keeps every token and what javadoc reads, so `<pre>`, tables, ASCII art
  and multi-line `{@code …}` keep their lines, and a comment that shares its
  line with code stays as written. google-java-format wraps a long Markdown doc
  comment line (`///`) into an ordinary `//` comment line, which javadoc doesn't
  read; Jolt fills the paragraphs of a Markdown doc comment and keeps every line
  a `///` line.
- **Lambdas.** Jolt removes the parentheses around a single untyped lambda
  parameter (`x -> x`). For a lambda with a block body or an anonymous class as
  the last argument, google-java-format puts every argument on a new line when
  they don't all fit (`run(` / `() -> {`); Jolt keeps the call open on its line
  (`run(() -> {`) when the arguments before the body fit there.

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
  comma, including the last, and a `;` after the constants is removed, its
  comments where they were. With members, the last constant is followed by `;`,
  then a blank line and the members.
- A comment after the last constant stays on its line, after the comma or `;`.
  With members, when a comment is on the comma after the last constant or on the
  `;`, both stay as written, and the `;` goes on its own line after the comma:

::: code-group

```java [Input]
enum E {
  A,
  B // last
}

enum F {
  B // last
  ;
  int code;
}

enum G {
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

enum G {
  B, // last
  ;

  int code;
}
```

:::

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

- An initializer that doesn't fit is laid out as an
  [assignment's value](#assignment) is.
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
- A rule's expression body (`case X -> value;`) that doesn't fit is laid out as
  an [assignment's value](#assignment) is.
- A label with a single item keeps it after `case`. A pattern that doesn't fit
  breaks by its own rule: its `)` returns to the label's indentation, and the
  guard, `->` and the body follow it.
- A label with several items (`case null, default`) is one line if it fits.
  Otherwise `case` stands alone on its line and each item goes on its own line
  one level deeper, as the types of an `implements` clause do.
- A guard (`when …`) stays on the label line. A condition that doesn't fit
  breaks by its own rule, as a `return` value does. Redundant parentheses around
  a guard are removed, unless a comment is on or inside them.
- After a label that ends on a continuation line (its items or its guard wrap),
  the body starts on the next line.

```java
switch (shape) {
  case Point(
    int xCoordinateValue,
    int yCoordinateValue,
    int zCoordinateValue
  ) when xCoordinateValue > 0 -> describePoint(
    xCoordinateValue,
    yCoordinateValue,
    "first quadrant",
    moreArguments
  );
  case Point(int x, int y) when x > 0
    && someVeryLongConditionName(x, y)
    && another(y) -> 1;
  case
    FirstVeryLongTypeName,
    SecondVeryLongTypeName,
    ThirdVeryLongTypeName,
    FourthType ->
    handleTheseFourTypes(shape, firstOptionValue, secondOptionValue, third);
  default -> {}
}
```

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
T factory = instantiateFactory(
  implementationName,
  factoryType,
  argumentResolver,
  failureHandler
);
Supplier<String> greeting =
  () -> greetingService.greetingFor(currentUser.preferredLocale());
Runnable task = () -> {
  run();
};
allowed =
  user.isActive()
    && account.hasPermission("write")
    && featureFlags.isEnabled("collaborative-editing");
CompletableFuture<ServerResponse> responseFuture =
  CompletableFuture.supplyAsync(() -> {
    return render();
  });
SomeHandlerType handler =
  new SomeHandlerType(firstArgumentValue, secondArgument) {
    public void run() {}
  };
```

- A value that doesn't fit on the operator's line moves to the next line, one
  level deeper, if it fits there. A call that stays open for a last argument
  with a body (see [Calls](#calls)) counts as fitting there if its line fits up
  to the body.
- Otherwise the value starts on the operator's line and breaks by its own rule,
  as a `return` value does: a call breaks its arguments, a lambda or an
  anonymous class opens its body, an array initializer opens its braces. The
  value stays on the operator's line only if the line then fits, what breaks is
  the last part of the value on that line, and on the next line the same part
  would have to break as well. When an earlier part would have to break instead,
  such as the type arguments before a call's arguments or a lambda's parameters
  before its body, or when the part would fit on the next line, such as the
  header of an anonymous class, the value moves to the next line.
- The lines the value breaks into must be indented deeper than the line it
  starts on. When the operator ends a continuation line, such as a switch label
  whose pattern or guard wraps, the value moves to the next line.
- An operator chain (`a && b`, `a + b`, `a instanceof T`) or a conditional that
  doesn't fit always moves to the next line, so that its operands line up one
  level deeper.
- These rules apply to `=`, to compound assignments such as `+=`, to variable
  initializers, and to the expression after a switch rule's `->`.

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
tasks.register(taskName(sourceSet), ArchitectureCheck.class, task -> {
  task.setClasses(sourceSet.getOutput().getClassesDirs());
});
template.execute(
  first -> {
    first.run();
  },
  second -> {
    second.run();
  }
);
```

- Arguments break like a list.
- When the last argument has a body that always breaks (a lambda with a block
  body, an anonymous class, a switch expression) or is a text block, and the
  arguments before it have none, the call stays open on its line: the body
  breaks at the call's indentation and `)` follows the body's last line (`});`).
  This applies when the line fits up to the body's opening line and the last
  argument doesn't break before its body (its parameters stay on one line).
  Otherwise the arguments break like a list, as they do when two arguments have
  bodies.
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
- When the last call is the only one after the receiver
  (`this.registry.register(…)`, `assertThat(…).isTrue()`), or the only one after
  the first call that stays on the receiver's line
  (`Mono.fromCallable(…).subscribeOn(…)`), it is laid out as an
  [assignment's value](#assignment) is: it moves to the next line only if that
  makes it fit. Otherwise it stays on the line where the receiver ends and its
  arguments break:

```java
this.endpointRegistry.registerListenerContainer(
  endpoint,
  this.containerFactory,
  this.startImmediately
);
assertThat(
  registry.getListenerContainer("firstListenerName").getConcurrencyLimit()
).isEqualTo(3);
return Mono.fromCallable(() -> {
  return compute();
}).subscribeOn(Schedulers.boundedElastic());
```

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
- A lambda as the last argument of a call keeps a block body at the call's
  indentation (see [Calls](#calls)).
- Method references have no spaces: `String::length`, `int[]::new`.

### Arrays

```java
String[] pair = new String[] { "a", "b" };
int[] empty = new int[] {};
String[] labels = {
  "a very long label that forces the initializer to break across lines",
  "another very long label that keeps each item on its own line",
};
```

- Initializers have a space inside the braces when they fit on one line
  (`{ 1, 2 }`), and print `{}` when empty.
- A broken initializer puts each element on its own line followed by a comma,
  and `}` on its own line. A trailing comma in the source keeps it broken.

### Other expressions

- Casts: `(String) value`, `(Runnable & AutoCloseable) value`. A cast whose
  value doesn't fit lays out the value after the type as an
  [assignment's value](#assignment) is.
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
  line, except an enum's `;` kept on its own line (see [Enums](#enums)) and a
  stray `;`, which stays after the comment and is removed or goes on its own
  line (`} // c` followed by `;` prints `} // c`). A line comment always ends
  the line, and counts toward the line width.
- **Inline.** A block comment between code on one line stays inline, with one
  space on each side, except none after an opening bracket, `.`, `::` or `@`,
  and none before a closing bracket, `,` or `;`.
- **After an opening bracket.** A comment that ends the line after `{`, `(`, `[`
  or `<` moves to the first line inside.
- **Before a header clause.** A block comment that ends the line before
  `extends`, `implements`, `permits` or `throws` goes with the clause, onto the
  clause's line.
- **Before a part that continues a line.** A comment that starts its line before
  an array initializer's `{`, a guard's `when`, a type parameter's `extends` or
  an annotation element's `default` puts that part one level deeper, as a
  continuation line of the line before it:

```java
int[] values = new int[]
  // the first primes
  { 2, 3, 5, 7 };
```

- **Between a header and its body.** A comment between a declaration or control
  statement header and its body moves to the first line inside the body, unless
  it is inline (`class X /* c */ {` stays as written). A Javadoc comment before
  the `{` of a class body with members goes before the `{` on the header's line
  instead, since inside it would document the first member.

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

- **After an operator, keyword or arrow.** A comment that ends the line after
  `=`, `return`, `throw`, `yield`, `break`, `continue`, `assert`, `->`, or the
  `&` or `|` between types stays there, and the value continues on the next line
  one level deeper. A comment on its own line before the value puts the value
  one level deeper too. An inline block comment after them stays inline, and the
  value follows it on the same line (`return /* c */ value;`):

```java
int field = // a comment ends the line after the operator
  1;
return // a comment ends the line after the keyword
  someLongMethodCall(firstArgumentName, secondArgumentName)
    + anotherMethodCall(thirdArgumentName);
run(
  () -> // a comment ends the line after the arrow
    task.run()
);
```

- **In an empty body or list.** Comments in an otherwise empty body or list, and
  comments on their own lines before a closing bracket, go inside the brackets
  one per line, one level deeper. Comments that shared a line keep sharing it:

```java
log(
  // no arguments
);
```

- **Markdown doc comments.** Consecutive `///` lines are one Markdown doc
  comment, and so are a `///` line that ends a line of code and the `///` lines
  right after it. When Jolt moves a `///` comment next to another one, as it
  moves a comment before a body's `{` into the body, a blank line keeps the two
  apart, so they don't become one doc comment:

::: code-group

```java [Input]
class Example
/// Not documentation: no declaration follows.
{
  /// Runs it.
  void run() {}
}
```

```java [Output]
class Example {
  /// Not documentation: no declaration follows.

  /// Runs it.
  void run() {}
}
```

:::

- **Comment text.** Trailing whitespace is removed. Jolt doesn't rewrap or
  otherwise change the text of a comment, except a doc comment that starts its
  line (see [Doc comments](#doc-comments)). A block comment whose lines start
  with `*` is realigned so the `*`s line up under the opening `/*`, and each
  line keeps its text after the `*`. Other multi-line block comments move as a
  unit, keeping their lines' relative indentation.

```java
int run() {
  /*
   * Starts the run.
   */
  start();
  return total;
}
```

## Doc comments

Jolt lays out the text of each Javadoc comment (`/** … */`) that starts its own
line, and of each Markdown doc comment (`///`, see
[Markdown doc comments](#markdown-doc-comments)): it fills paragraphs to the
line width and puts lists, block tags and code in one fixed style. What javadoc
reads in a comment stays the same, and every character it reads is kept. Jolt
reads a comment the way javadoc does (it follows javac's own rules), so it
changes only whitespace that the rendered page doesn't show: the line breaks and
spaces between words of text.

Everything else in the comment keeps its exact text, line breaks included, and
affects only itself:

- `<pre>` and `<table>` elements, from their start tag to their end tag;
- inline tags whose text javadoc keeps as it is: `{@code …}`, `{@literal …}`,
  `{@snippet …}`, `{@value …}`, `{@index …}`'s term, and tags javadoc doesn't
  know;
- references (`{@link Map#put(Object, Object)}`, `@throws IOException`), HTML
  tags with their attribute values, HTML comments (`<!-- … -->`) and entities;
- what javadoc reads as an error, such as an `<` that starts no tag or an
  unclosed `{@code`, and whether a space or a line break follows it;
- a paragraph of plain text whose lines are indented differently, by two or more
  columns, such as ASCII art, a grammar or aligned text.

A reference that spans lines (`{@link Map#put(Object,` / `Object)}`) is the
exception: it is joined onto one line with single spaces, since whitespace
inside a reference doesn't change what it refers to (unless a later line starts
with `@`, which ends the reference for javadoc). Any other such part that spans
lines keeps its lines: each later line is printed after `*` with the indentation
that javadoc reads on it (javadoc removes the `*` and the indentation that all
lines of the comment share). A tab after the `*` stays a tab.

Jolt keeps these comments as written, realigning only their `*`s as for any
other comment (see [Comments](#comments)): a comment that shares its line with
code; a comment that starts with `/***` or has a line of only `*`s, which is a
banner; `/**/`; a comment before `package` or an import in a file that declares
a type, which is a file header such as a licence; the Javadoc of a member that
formatting is turned off for (see [Formatter-ignore](#formatter-ignore)); a
comment with a Unicode escape (`@`), which javac decodes before javadoc reads
the comment; and a comment that javac's reader can't read in bounded time or
depth: one that nests inline tags more than 32 deep, one that it reads in more
than linear time (it rescans some malformed text), and a Markdown comment whose
last line it never finishes reading (such as `/// + +`).

A comment that Jolt lays out gets this style:

- **Paragraphs** are filled: each line takes as many words as fit in the line
  width, counting the comment's indentation and the `" * "` before the text. A
  word that is wider than its line, such as a long link, goes on a line of its
  own and overflows. The text starts on the line after `/**`, and `*/` gets a
  line of its own.
- **One line.** A comment that is one paragraph with no block tags prints as
  `/** Text. */` when that fits, else on separate lines. An empty comment prints
  as `/** */`.
- **Words stay whole.** Jolt doesn't break a line inside any of the parts listed
  above, or between an inline tag and its reference (`{@link Map#get}`). The
  label of a link, and the text of `{@return …}` and `{@summary …}`, are filled
  like other words. Two parts written without a space between them stay
  together.
- **No new structure.** A wrapped line never starts with a word that javadoc or
  Jolt would read as new structure: a word that starts with `@` (a block tag),
  or a plain-text list marker (`-`, `*`, `+`, `•`, or a number and `.` or `)`).
  Such a word stays on the line of the word before it, and the line breaks
  before the pair when they don't fit. A word that starts with `@` at the start
  of a source line keeps starting its line.
- **Block tags** (`@param`, `@return`, …) start a line each, with single spaces
  after the tag and its name, and the first word of the tag's text on the tag's
  line. Their wrapped lines are indented by 4. A blank line goes before the
  first block tag, and none between block tags.
- **Blank lines.** One blank line goes before `<p>`, a list, `<pre>`, a table
  and a heading, and after a list, a heading, `<pre>` or a table when text
  follows. A blank line of the source between two paragraphs is kept, without
  adding a `<p>`. Inside a list and in block tag text, Jolt adds no blank lines
  and keeps at most one.
- **`<p>`** is joined to the text after it, and `</p>` to the text before it.
- **Lists.** `<ul>`, `<ol>` and `<dl>` and their closing tags start a line. Each
  item (`<li>`, `<dt>`, `<dd>`) starts a line indented by 2, and its wrapped
  lines are indented 4 more. `</li>` is joined to the text before it. A block
  inside an item, such as a nested list, starts a line at the item's wrapped
  line indentation. Block tag text holds lists the same way, inside its indent.
- **Other blocks.** The tags of `<blockquote>`, `<div>`, `<hr>` and other
  block-level elements start a line, and so does the text after them.
- **Line breaks.** A line break follows `<br>`. The source's line break stays
  before a line that starts with a list marker, and around a line made only of
  punctuation, such as `-------`. Each such line is filled on its own.

::: code-group

```java [Input]
class Resolver {
  /**
   * Resolve the given placeholder using the properties of this resolver, which may be long.
   * <p>
   * Delegates to the helper, see {@link PropertyPlaceholderHelper#replacePlaceholders}
   * for the syntax of nested placeholders.
   * <ul>
   * <li>first</li>
   * <li>second item, which is long enough that it does not fit on one line of the comment</li>
   * </ul>
   * <pre class="code">
   *     resolver.resolve("${a}");
   * </pre>
   * @param placeholder    the placeholder to resolve, which may contain
   *        nested placeholders
   * @return the value
   */
  String resolve(String placeholder);

  /**
   * Returns the name.
   */
  String name();
}
```

```java [Output]
class Resolver {
  /**
   * Resolve the given placeholder using the properties of this resolver, which
   * may be long.
   *
   * <p>Delegates to the helper, see
   * {@link PropertyPlaceholderHelper#replacePlaceholders} for the syntax of
   * nested placeholders.
   *
   * <ul>
   *   <li>first</li>
   *   <li>second item, which is long enough that it does not fit on one line of
   *       the comment</li>
   * </ul>
   *
   * <pre class="code">
   *     resolver.resolve("${a}");
   * </pre>
   *
   * @param placeholder the placeholder to resolve, which may contain nested
   *     placeholders
   * @return the value
   */
  String resolve(String placeholder);

  /** Returns the name. */
  String name();
}
```

:::

Lines that start with a list marker keep their line breaks, a paragraph of ASCII
art keeps its lines, and a multi-line `{@code …}` keeps its line breaks while
the text around it is filled:

::: code-group

```java [Input]
class Interceptor {
  /**
   * Callbacks, in order:
   * 1) before, when the task is submitted, which is long enough that it wraps onto a line of its own
   * 2) after, once the task completes
   *
   * The states,   as a graph:
   *      new
   *     /   \
   *   done  failed
   *
   * Call {@code
   * intercept(task)} once, before the task runs, so that the callbacks see every state.
   */
  void intercept();
}
```

```java [Output]
class Interceptor {
  /**
   * Callbacks, in order:
   * 1) before, when the task is submitted, which is long enough that it wraps
   * onto a line of its own
   * 2) after, once the task completes
   *
   * The states,   as a graph:
   *      new
   *     /   \
   *   done  failed
   *
   * Call {@code
   * intercept(task)} once, before the task runs, so that the callbacks see
   * every state.
   */
  void intercept();
}
```

:::

### Markdown doc comments

A run of `///` lines is a Markdown doc comment (JEP 467) when it is the last doc
comment before a declaration that javadoc documents: a class, interface, enum,
record, annotation interface, field, method, constructor, enum constant,
annotation element, module, or the package of a `package-info.java`. A run is a
group of `///` lines one after the other, with no blank line between them; the
first may end a line of code. Jolt reads the position after formatting: a run
that Jolt moves to just before a declaration counts. Other `///` lines are
ordinary line comments, which Jolt prints as written.

javadoc reads the description and the text of each block tag as a Markdown
document of its own. Jolt reads them the same way and fills their paragraphs;
every other block keeps its lines exactly as written: lists, headings, code
blocks, block quotes, tables, HTML blocks, link reference definitions and
thematic breaks. So do lines that two blocks share, and a run with a block tag
that doesn't start its line. Lines printed as written lose their trailing
whitespace, except two or more spaces after text before a line with text, which
may be a hard break.

A Markdown doc comment gets this style:

- **The frame** is `///` and the indentation that every line of the run shares,
  usually one space. A run written without a space, such as `///CLOVER:OFF`,
  keeps having none.
- **Paragraphs** are filled as in a Javadoc comment. The line width counts the
  comment's indentation and the frame.
- **Blank lines** are kept as written. One is added before the first block tag
  when there is none.
- **Block tags** start a line each, with single spaces after the tag and its
  name. The tag's text follows on the tag's line, and its wrapped lines are
  flush with the frame: in Markdown, an indented line can start code or continue
  a list.
- **Hard breaks** stay: a line that ends in two spaces or `\` inside a paragraph
  keeps its line break and its spaces or `\`.
- **Words stay whole.** Jolt doesn't break a line inside a code span
  (`` `a b` ``), a link (`[text](url)`, `[text][ref]`,
  `[Map#put(Object, Object)]`), an autolink or HTML tag (`<…>`), a `<pre>`
  element, or an inline tag that javadoc keeps whole (`{@code …}`). The label of
  a link and the text of `{@return …}`, `{@summary …}` and `{@index …}` are
  filled when they are plain words; one with Markdown that a line break could
  change (a code span, a link, HTML, an escape, a hard break or a blank line)
  keeps the whole tag as written.
- **No new structure.** Besides the words listed for Javadoc comments, a wrapped
  line never starts with a word that could start a Markdown block: one that is
  `#` to `######`, starts with `>`, `|`, three or more `` ` `` or `~`, or `<`
  and a letter, `/`, `!` or `?`, or is made only of `*`, `-`, `_`, `=` and `+`.
  A word that ends in `\` never ends a wrapped line, since a line break after it
  is a hard break. In the first sentence, which javadoc ends at a line that
  starts with `-`, `+`, `*`, `#`, `=`, `_` or `>`, a word that starts with one
  of those never starts a wrapped line, and one that starts a source line keeps
  starting its line.

::: code-group

````java [Input]
class Splitter {
  /// Splits the text at each comma and trims each part. Empty parts are dropped - except the last one, which is kept.
  ///
  /// ```java
  ///   split("a, b,", 0); // ["a", "b", ""]
  /// ```
  /// @param text the text to split, which may be empty or contain only commas and spaces
  /// @return the parts,
  ///     in order
  String[] split(String text, int limit);

  /// Joins the parts:
  /// - with a comma between two parts, which is long enough to cross the line width;
  /// - with nothing after the last one.
  String join(String[] parts);

  ///CLOVER:OFF
  void generated() {}
}
````

````java [Output]
class Splitter {
  /// Splits the text at each comma and trims each part. Empty parts are
  /// dropped - except the last one, which is kept.
  ///
  /// ```java
  ///   split("a, b,", 0); // ["a", "b", ""]
  /// ```
  ///
  /// @param text the text to split, which may be empty or contain only commas
  /// and spaces
  /// @return the parts, in order
  String[] split(String text, int limit);

  /// Joins the parts:
  /// - with a comma between two parts, which is long enough to cross the line width;
  /// - with nothing after the last one.
  String join(String[] parts);

  ///CLOVER:OFF
  void generated() {}
}
````

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
  them are formatted as usual, except that a Javadoc comment between the markers
  keeps its text as written (see [Doc comments](#doc-comments)).
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
