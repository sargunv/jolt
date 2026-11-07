# Java style

This page specifies the Java output of `jolt`. It documents the style the
formatter produced at commit `53efc726`, which is the reference, and lists the
deliberate changes from it.

## Normative reference

The reference is, in order of precedence:

1. The formatter snapshots at commit `53efc726`
   (`crates/jolt_java_fmt/tests/snapshots/corpus__*.snap`, the `formatted:`
   section), whose inputs are `fixtures/java/**`.
2. The reference corpus outputs in `tools/import/.imports/.jolt-ref/` (a durable
   copy of `/tmp/jolt-ref-ScpJkX`): `old-output/` (line width 80) and
   `old-output-w100/` (spring-framework at line width 100) for the inputs in
   `input/`, produced by `jolt-old` at commit `de12e2cb`, whose Java output is
   identical to `53efc726`. Files with syntax errors were left untouched by
   `jolt-old` and are not part of the reference.
3. This page, which summarizes 1 and 2 so rules can be written against it.

Where this summary and the reference disagree, the reference wins. Any output
difference from the reference that is not listed under
[Deliberate changes](#deliberate-changes) is a regression. Examples below use
the default options and are taken from the snapshots unless noted.

## Options and basics

- Line width 80, indentation 2 spaces, spaces not tabs (`line_width`,
  `indent_width`, `use_tabs`). With tabs, one tab per level.
- Every continuation is exactly one indentation level deeper than the line it
  continues. There is no alignment to columns and no double indent.
- Output uses `\n` line endings, has no trailing whitespace (outside text block
  and comment content), and ends with exactly one newline. An empty file formats
  to a single newline.
- At most one blank line anywhere. No blank line directly after an opening brace
  or before a closing brace.
- A byte order mark at the start of the file is preserved as the first bytes of
  the output, before any comment; a trailing SUB (`\u001a`) character is
  preserved after the final newline.
- Layout is "fit or break": a construct goes on one line if it fits, otherwise
  its parts break by the construct's rule. Lists break all-or-nothing: every
  element on its own line, no filling.

## Compilation unit

```java
// file header comment

package com.example.demo;

import a.Alpha;
import z.Zed;

class A {}
```

- Order is as written: header comments, package, imports, then types (or
  top-level members of a compact compilation unit). A blank line separates the
  package, the import section, and each following top-level item; blank lines
  between header comments are preserved (one at most).
- Package annotations go one per line before `package`.
- Stray top-level `;` are removed (comments on them are kept in place).
- A compact compilation unit (`void main() { … }`) formats its members like
  class members at column 0.

### Imports

```java
import a.Alpha;
import b.Beta;
import module java.base;
import module.foo.Bar;

import static a.Alpha.alpha;
import static z.Zed.z;
```

- Normal imports (including `import module`) come first, then static imports,
  separated by one blank line.
- Within each group, imports sort by their dotted name (the module name for
  `import module`), compared character by character, case-sensitively, with an
  on-demand import's name followed by `.*`.
- An import that carries a leading comment is an anchor: it starts a new run and
  stays first in it, and only the imports after it sort among themselves.
  Comments therefore never move relative to the imports around them, except that
  the normal/static split may move an import past a comment.
- An import that is not recovery-free, or whose name contains comments, is a
  barrier: imports on either side sort separately and it stays in place.
- Blank lines between imports are removed, except that a source blank line
  before a comment-carrying import (an anchor) is kept. The blank line between
  the normal and static groups is always present. Duplicates are not removed.
- A trailing comment stays with its import:

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

Directives are grouped by kind in the order `requires`, `exports`, `opens`,
`uses`, `provides`, groups separated by a blank line, sorted by name within a
group. `requires` modifiers are ordered `static transitive` unless one carries a
comment. A directive with leading comments, a recovered directive, and a
formatter-ignored one are barriers: each stays in place on its own, and the
directives on each side sort separately. `to`/`with` name lists break
all-or-nothing.

## Declarations

### Annotations and modifiers

```java
@Generated("tool")
@Deprecated
public final class User {
  @FieldAnno(name = "id", values = { 1, 2 })
  private final String id;

  public @Nonnull String name() {
```

- Declaration annotations (those before the first modifier keyword) go one per
  line before the declaration. Annotations after a modifier keyword are type-use
  annotations and stay inline. This applies to every declaration kind, including
  local variables and try resources (`@Nonnull` / `final String s = "x";`);
  formal and lambda parameters keep their annotations inline
  (`void m(@Nonnull String p)`).
- Modifier keywords are reordered to this order:

  ```java
  public protected private abstract default static final transient volatile
  synchronized native sealed non-sealed strictfp
  ```

  Annotations keep their order among themselves (and may move past
  `non-sealed`). A modifier carrying a comment is a barrier; the keywords on
  each side sort separately and the commented one stays put:
  `final /* stable */ public class C`.
- Parameter modifiers (`final` and annotations) keep source order.
- Annotation arguments: `@A(x)`, `@A(name = "demo", enabled = true)`; element
  value arrays are padded `{ 1, 2 }` when flat. A broken argument list puts each
  element on its own line, closing `)` on its own line.

### Type declarations

```java
class Derived<T>
  extends /* base */ base.@Marker Parent<String>
  implements
    First, // first
    pkg.Second
  permits
    Child, // child
    other.Child {}

interface Shape extends Drawable, Scalable permits Circle, Square {}
```

- The header (`modifiers kind Name<T>` plus `extends`, `implements`, `permits`)
  is one line if it fits, braces included. Otherwise each clause goes on its own
  line, one indent deeper than the declaration; a clause whose types do not fit
  puts the keyword alone on its line and each type on its own line one further
  indent deeper.
- The opening brace ends the header line.
- Type parameters break like a list: `<`, one parameter per line, `>` on its own
  line. Bounds that do not fit break before each `&` at one more indent.

### Bodies and members

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

- Members are one per line. Between two members there is a blank line, except
  between two consecutive field declarations, which keep the source's blank line
  or its absence (one at most). The same holds for interface, enum, record and
  annotation-interface members.
- Redundant `;` members are removed; their comments stay in place.
- Empty bodies (methods, constructors, initializers, types, blocks, anonymous
  classes, lambda blocks, switch blocks, module bodies) print `{}` (D1); a body
  holding only comments breaks.

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

- Constants are always one per line. Without members, every constant is followed
  by a comma, including the last. With members, the last constant is followed by
  `;`, then a blank line and the members. A source `;` with no members is kept
  after the last constant (`B;`). The constants' own comments stay with them,
  and no comma is added after a last constant with a trailing line comment
  (`B // c`).
- When the constants end with `;`, the comments of a removed trailing comma and
  the comments before the `;` move after `B;` and a blank line, where they lead
  the members (or end the body). The `;`'s comments are followed by another
  blank line; a comma's comments lead the first member directly:

```java
enum Mode {
  B;

  // was after the comma: `B, // c`
  int code;
}
```

- Constant annotations go on their own lines; arguments and bodies format like
  calls and class bodies.

### Records

```java
record CustomerProfile(
  String displayName,
  CustomerIdentifier primaryIdentifier,
  CommunicationPreferences communicationPreferences
) {}
```

Components are a list (flat if they fit, else one per line with `)` on its own
line); `implements` follows the type-declaration header rule. Compact
constructors print `public Range {`.

### Methods and constructors

```java
public Result compute(
  Request request,
  ExecutionContext executionContext,
  RetryPolicy retryPolicy
)
  throws /* checked */ IOException, // io
    TimeoutException {}
```

- `modifiers <T> ReturnType name(params) throws X {` on one line if it fits.
- Otherwise the parameters break: `(`, one parameter per line, `)` on its own
  line at the declaration's indent.
- A `throws` clause that does not fit after `)` goes on its own line one indent
  deeper; its types break one indent further.
- Generic methods with long type parameters break them as a list: `public <`,
  parameters, `> void run() {`.
- Varargs `String... values`, receiver parameters `Foo this`, and old-style
  array dimensions after the name are kept as written.
- Explicit constructor invocations (`super(…)`, `this(…)`, `outer.<T>super(…)`)
  format like calls.
- Annotation interface elements: `String value() default "x";`, with array
  defaults formatted like array initializers.

### Fields and variables

```java
  final @Nullable String shortName =
    computeCreationUserAgentXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX();

  int firstFieldVariable = 1,
    secondFieldVariable = 2,
    thirdFieldVariable = provider();
```

- An initializer that does not fit breaks after `=` and continues one indent
  deeper (the assignment rule below). This holds for every right-hand side,
  including lambdas, anonymous classes and array initializers.
- Multiple declarators stay on one line if they fit; otherwise each declarator
  after the first goes on its own line one indent deeper.

## Statements

### Blocks

- Statements are one per line. Blank lines between statements are preserved (one
  at most), never added.
- Empty statements (`;`) inside blocks and switch groups are removed; their
  comments stay in place.

### Control flow

```java
if (ready) {
  run();
} else if (waiting) {
  pause();
} else {
  stop();
}
if (
  user.isActive()
    && account.hasPermission("write")
    && featureFlags.enabled()
) {
  run();
}
```

- Bodies of `if`, `else`, `while`, `do`, `for` and enhanced `for` are always
  blocks: a single statement body gets braces. `else if` chains stay flat.
  `} else {`, `} while (…);`, `} catch (…) {` and `} finally {` share the line
  with the closing brace.
- A header condition that does not fit breaks inside the parentheses: `(` ends
  the line, the condition is one indent deeper, `)` returns to the statement's
  indent.
- `for` headers: `for (int i = 0; i < 10; i++) {`; empty parts as `for (;;)` and
  `for (; ready(); tick())`. A header that does not fit puts each of the three
  parts on its own line (`init;`, `condition;`, `update`), with `) {` on its own
  line. Enhanced for: `for (String value : values) {`.
- Labels go on their own line before the labeled statement: `retry:` then
  `for (…) {`.
- `synchronized (this) {`, `assert value != null : value;`, `throw new X();`,
  `return x;`, `break label;`, `continue label;`, `yield value;`.

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

- Labels are one indent inside the switch; statements of a colon group one more.
  Each `case …:` label is on its own line. A colon group whose single statement
  is a block keeps the block on the label line: `case 1: {`.
- Rules: `case X -> expression;`, `case X -> {` block, `case X -> throw …;`. A
  rule body that does not fit breaks after `->` one indent deeper.
- Case label lists (`case null, default`) and guards (`when cond`) format on the
  label line; redundant parentheses around a guard condition are removed.
- Switch expressions format the same way; as a value they follow the enclosing
  expression's rules (an assignment breaks after `=` before `switch`).

### Try

```java
try (
  var declared = open()
) {
  use(declared);
} catch (java.io.IOException | RuntimeException ex) {
  return 1;
} finally {
  cleanup();
}
```

- The resource specification always breaks, even for one short resource:
  `try (`, one resource per line (separated by `;`), `) {`. A trailing `;` after
  the last resource is removed.
- A catch parameter that does not fit breaks inside the parentheses, the union
  breaking before each `|`.

## Expressions

### Assignment

`target op value` stays on one line if it fits; otherwise it breaks after the
operator and the value continues one indent deeper, where it may break further
by its own rule:

```java
allowed =
  user.isActive()
    && account.hasPermission("write")
    && featureFlags.enabled();
```

### Binary operators

```java
return user.isActive()
  && account.hasPermission("write")
  && featureFlags.enabled()
  && auditPolicy.allows(user);
```

- A chain of operators of one precedence level breaks all-or-nothing, before
  each operator, with the continuation one indent deeper than the chain start.
- Left-nested operations of the same precedence are flattened into one chain,
  and redundant parentheses around such a left operand are removed:
  `((x + x) - x) + x` prints as `x + x - x + x`. Shifts, and `%` or mixed
  multiplicative operators, are not flattened into each other.
- Operands of a bitwise or shift operator (`&`, `|`, `^`, `<<`, `>>`, `>>>`)
  that are themselves binary expressions get readability parentheses, which
  break like a parenthesized expression.
- `instanceof` participates in chains like a relational operator; patterns print
  inline (`value instanceof String text`).

### Conditional

```java
int j =
  i == 0
    ? 0
    : i == 1
      ? 1
      : i;
```

Flat if it fits; otherwise `?` and `:` each start a line one indent deeper than
the condition, and a nested conditional in the false branch nests one indent
further.

### Calls and arguments

```java
return create(
  user.profile().displayName(),
  account.permissions().primaryRole(),
  settings.region().identifier()
);
```

- Arguments are flat if they fit; otherwise `(` ends the line, each argument is
  on its own line one indent deeper, and `)` is on its own line. There is no
  special treatment for a last lambda or anonymous class argument.
- Explicit type arguments: `this.<String>convert(x)`, `target::<String>id`.
- Constructor calls follow the same rule: `new Box(…)`, and qualified ones such
  as `outer.new Inner(…)`.

### Member chains

```java
builder.withFirstValue(first.reallyLongDisplayName())
  .withSecondValue(second.reallyLongDisplayName())
  .build();
return com.example.deep.config.ConflictResolutionFactory
  .newBuilder()
  .build();
```

- A chain is a receiver followed by `.name(…)` calls (and field accesses, array
  accesses, postfix operators). It is flat if it fits.
- Otherwise every call after the head goes on its own line, starting with `.`,
  one indent deeper than the chain start. The head is everything before the
  first call (a name, field accesses, a parenthesized expression, or an
  unqualified call such as `getFoo()`). When that receiver is a single
  identifier, `this` or `super`, the first call joins the head
  (`someLocal.call(a)`, `Foo.builder()`, `this.toStringHelper()`); after a
  dotted receiver (`this.a.b`, `com.example.Factory`) the first call starts a
  new line.

### Lambdas and method references

```java
return (left, right) -> left + right;
return user -> user.name();
return () -> {
  run();
};
```

- A single untyped, unmodified parameter drops its parentheses: `(x) -> x`
  prints `x -> x`. Typed, annotated, `final` or `var` parameters keep them.
- Parameters that do not fit break as a list: `(`, one per line, `) -> body`.
- Expression bodies stay on the arrow line when they fit; block bodies open on
  the arrow line.
- Method references print without spaces: `String::length`, `int[]::new`,
  `Example.super::toString`.

### Arrays

```java
return new String[] { "a", "b" };
return new int[] {};
return new String[] {
  "a very long label that forces the initializer to break across lines",
  "another very long label that keeps each item on its own line",
};
```

- Initializers are padded when flat (`{ 1, 2 }`), `{}` when empty, and when
  broken put each element on its own line with a trailing comma, closing brace
  on its own line. A trailing comma in the source forces the broken layout
  (`int[] b = {1, 2,};` prints broken, after `=`); the same holds for annotation
  element arrays.
- Dimensions: `new int[size][count]`, `String[][].class`, `int[] a`.

### Other expressions

- Casts: `(String) value`, `(Runnable & AutoCloseable) value`.
- Unary operators attach to their operand (`!ready`, `-x`, `++i`, `i++`) with a
  space only where two operators would otherwise merge (`- -x`).
- Parenthesized expressions break like a call: `(`, contents one indent deeper,
  `)` on its own line.
- Literals, including text blocks, print exactly as written. Text blocks keep
  their content and closing delimiter byte for byte, and a construct containing
  one is never flat.
- Class literals `void.class`, `this`, `Outer.this`, `super.x`.

### Types

- `java.util.@Marker List<@Marker String> @Marker [] names` — annotations
  inline, no spaces inside `<>`.
- Type argument lists that do not fit break as a list: `<`, one argument per
  line, `>` on its own line.
- Wildcards `? extends Number`, `? super T`; intersection bounds break before
  `&`.

## Comments

Comments are never dropped, duplicated, or merged, and never move across a token
except past a `,` or `;` (below) and into a control statement's body (C7).
Placement follows [the formatter's comment model](./formatter.md#comments); the
visible rules are:

- **Own line.** A comment that starts its line in the source starts its line in
  the output, at the indentation of the code it precedes. Blank lines after it
  are preserved (one at most). A run of comments before a member or statement
  stays attached to it.
- **Start of line.** A block comment that starts its line with code after it on
  the same line keeps that code on its line (`/* lead */ static {`, C5).
- **End of line.** A line comment that ends its line stays at the end of the
  line, after any `,` or `;` that follows its code (`user, // selected`,
  `return x; // done`, C8). The line always breaks after it, and its width
  counts toward the line. A closing delimiter is not pulled up: `foo(a // c` /
  `)` keeps `)` on the next line. A block comment that ends its line stays after
  its code but does not force a break: `foo(a, /* c */` / `b)` prints
  `foo(a, /* c */ b)` when it fits, and `x = 1 /* c */` / `;` prints
  `x = 1 /* c */;`.
- **After a keyword or operator.** An end-of-line comment after `=`, `return`,
  `throw`, `yield`, `break`, `continue` or `assert` keeps its place, and the
  value continues on the next line one indent deeper:

```java
int field = // a comment ends the line after the operator
  1;
return // a comment ends the line after the keyword
  someLongMethodCall(firstArgumentName, secondArgumentName)
    + anotherLongValue;
```

- **Inline.** A block comment with code on both sides stays inline with one
  space on each side, except none after an opening delimiter, `.`, `::` or `@`
  and none before a closing delimiter, `,` or `;` (C4).
- **Opening braces and parentheses.** An end-of-line comment after an opening
  `{`, `(`, `[` or `<` moves to its own line as the first line inside:
  `class X { // c` prints `class X {`, then `// c` one indent deeper (C6). An
  end-of-line block comment after `(`, `[` or `<` instead stays inline before
  what follows (`run( /* d */` / `a)` prints `run(/* d */ a)`).
- **Before a body's brace.** A block comment on its own line between a header
  and its `{` moves onto the header line (`class X /* c */ {`,
  `if (ready) /* c */ {`); a line comment there ends the header line and the
  brace takes the next line (`class X // c` / `{`), except in control statements
  (next rule).
- **Control headers.** A comment that ends the header's line, and a line comment
  on its own line, between a control statement's header and its body move to the
  first line of the body (C7). After `else` followed by `if`, the comment stays:
  `} else // c` / `if (…) {`.
- **Dangling.** Comments in an otherwise empty body, argument list or other
  delimited list, and comments on their own lines before a closing delimiter,
  print one per line inside the delimiters at the inner indentation, after any
  items, keeping one blank line before the first when the source had one.
  Comments that shared a source line keep sharing it (`/* a */ /* b */`):

```java
log(
  // no args
);
```

- **Comment text.** Trailing whitespace is removed. A multi-line block comment
  whose continuation lines start with `*` is re-aligned as a star block (`*`
  under the opening `/*`, `*/` closing); other multi-line block comments have
  each continuation line left-trimmed and re-indented. An own-line single-line
  doc comment is expanded:

```java
/**
 * Gets the name.
 */
```

## Formatter-ignore

```java
// @formatter:off
int x=       1+2;
foo( a,b,c );
// @formatter:on
```

- `// @formatter:off` and `// @formatter:on` (also as block comments) switch
  formatting off and on. Other spellings (`// prettier-ignore`,
  `// jolt-ignore`) have no effect.
- Granularity is the list item: a member, statement, top-level declaration,
  switch group statement, or module directive whose first token lies inside an
  off region prints exactly as written, shifted to the current indentation
  (lines lose the indentation of the item's first source line and get the
  current one; text block lines are never touched). The markers and the comments
  around an item are formatted normally. Items in a region keep the source's
  blank line or its absence between them, at most one (F1).
- Markers count only between the items of one body (or at top level). An `off`
  whose next `on` is in a different body has no effect, and neither has that
  `on` (fixture `formatter-ignore-nested-member-body`). A marker inside an item,
  such as in an argument list, has no effect.
- An `off` with no later `on` anywhere lasts to the end of its body (to the end
  of the file at top level).

## Code with syntax errors

The CLI and dprint refuse files with syntax errors unless asked to format them
anyway. When formatting them, every represented piece is formatted structurally;
each syntax-error node prints exactly as written (shifted like a
formatter-ignore item). No token is added to repair the code and none is lost.

## Deliberate changes

These are the only intended differences from the reference. Each is visible in
the snapshot diff at integration and must be explainable by one entry here.

### D1. Empty bodies print `{}`

The reference always breaks an empty body, even after a `throws` clause where
`{}` fits:

```java
// reference
  public void run() throws IOException, TimeoutException {
  }
// new
  public void run() throws IOException, TimeoutException {}
```

A braced body with no members, statements, constants or comments prints `{}`:
method, constructor and initializer bodies, blocks (including control statement
bodies and `catch`/`finally`), type bodies, anonymous class bodies, and lambda
block bodies, and also switch blocks, module bodies and enum bodies (every body
`braced_body` prints). As a consequence, an assignment whose value ends in an
empty body no longer has to break (`Runnable r = () -> {};`). A body containing
only comments still breaks. Array initializers already printed `{}`. D1 is
applied at integration as its own commit, after the rest of the output matches
the reference.

### D2. Type-use annotations among a method's modifiers stay in place

The reference moved a type-use annotation written among a generic method's
modifiers after the type parameters (`public @A <T> T m()` printed
`public static <T> @A T m()`). That moves the annotation from the modifier list
to the return type in the tree, a normalization outside N1–N10, so the
annotation now stays where it was written: `public static @A <T> T m()`. No
corpus file has this shape.

### C1. Own-line comments stay on their own line (#223)

The reference printed a comment that started its line as `comment` + newline
without first starting a line, so the next pass read it as a trailing comment
and joined it to the previous line. Now it keeps its own line.

```java
// input
    return
    /*c*/
    1;
// reference, pass 1 / pass 2
    return /*c*/
    1;
    return /*c*/ 1;
// new (stable)
    return
    /*c*/
    1;
```

### C2. A comment after an unclosed opener is stable (#222)

The reference indented the comment on the first pass and moved it to column 0 on
the second. Now the first pass already prints the second pass's output: with the
closer missing, the comment attaches to the end of the file, outside the
unclosed body.

```java
// input (no closing brace)
public @interface kHz { // No prefix defined in the annotation itself
// reference: pass 1 indents the comment, pass 2 moves it to column 0
// new (stable)
public @interface kHz {
// No prefix defined in the annotation itself
```

### C3. Every comment is printed exactly once (#224)

The reference could drop an own-line comment after an annotation that had its
own trailing comment, and could print a comment twice at a formatter-ignore
boundary. Both are fixed; there is no visible style rule.

Comments also keep their place among the tokens that remain when a `;` is
removed: `if (test); /*test*/ else` prints `if (test) {} /*test*/ else`, where
the reference moved the comment into the promoted block.

### C4. Inline comments get one space on each side, except next to delimiters

The reference inserted a space between `(`, `[`, `<`, `.` or `::` and a
following inline block comment, and printed no space between an inline comment
and a following `.`. Now an inline comment always has one space on each side,
except none after `(`, `[`, `<`, `.`, `::` or `@` and none before `)`, `]`, `}`,
`>`, `,` or `;`:

```java
// reference
    if ( /* guard */ ready) {
    return values[ /* index */ index];
    builder. /* call */ add(first);
    this /**/.field = 1;
// new
    if (/* guard */ ready) {
    return values[/* index */ index];
    builder./* call */ add(first);
    this /**/ .field = 1;
```

The same holds before a `:` (`retry /* label */ :`, `default /* d */ :`), where
the reference had no space, and after an opener when a separator follows
(`for (/* c */;;)`, where the reference printed `for ( /* c */;;)`).

### C5. A block comment keeps the code after it on its line

A block comment followed by code on the same line keeps that code on its line
instead of pushing it to the next line, also when the layout puts the comment at
the start of a line (`class C { /* c */ { body(); } }` now prints `/* c */ {` as
the first line of the body, where the reference printed `/* c */` and `{` on
separate lines):

```java
// input
  DeclarationHeaders(
      /* ctor-param-leading */ String name) {
// reference
  DeclarationHeaders(
    /* ctor-param-leading */
    String name
  ) {
// new
  DeclarationHeaders(
    /* ctor-param-leading */ String name
  ) {
```

### C6. End-of-line comments after an opening `(` or `<` move inside

Bodies already moved an end-of-line comment after `{` to the first line inside
the body; switch blocks now do too (`switch (v) { // c` puts `// c` on the first
line inside, where the reference left it on the brace line). Parentheses and
angle brackets now do the same (the reference left the comment on the opening
line, sometimes producing `for ( // c` then `;;)`):

```java
// reference
    for ( // keep going
    ;;) {
// new: the comment forces the header to break, one part per line
    for (
      // keep going
      ;
      ;
    ) {
```

### C7. End-of-line comments after a control header move into the body

An end-of-line comment after the `)` of an `if`/`while`/`for`/`switch`/
`synchronized`/`catch`/`try (…)` header, or after `else`, `do`, `try` or
`finally`, and a line comment on its own line between such a header and its
body, move to the first line of the body instead of staying before the brace
(the reference printed `if (` / `ready` / `) // fast path` / `{`):

```java
// input
    if (ready) // fast path
      run();
// new
    if (ready) {
      // fast path
      run();
    }
```

The exception is `else` followed by `if`, which keeps the reference output
`} else // c` / `if (…) {`.

### C8. End-of-line comments before `;` move after it

`,` already pulled an end-of-line line comment past itself
(`user, // selected`); `;` now does too, so a statement never ends with its `;`
alone on a line (a comment on its own line before `;` keeps its line, C1):

```java
// reference
    return // no value follows the keyword, so the terminator stays put
    ;
    return // a comment ends the line after the keyword
      y // a comment before the terminator keeps it in the continuation
      ;
    break // no label follows the keyword
    ;
// new
    return; // no value follows the keyword, so the terminator stays put
    return // a comment ends the line after the keyword
      y; // a comment before the terminator keeps it in the continuation
    break; // no label follows the keyword
```

### C9. No blank line is inserted after an end-of-line comment

The reference inserted a blank line after a line that ended with a line comment
in many places: after an annotation (`@Test // gh-123`), a `case` label, a
broken operand or chain segment, a `}` before `else`, a `for` header part and an
annotation's closing `)`. Blank lines now come only from the source, between
list items, never inside an expression or a declaration (corpus files
`B20844369.java`, `SwitchComment.java`, and 610 Spring files):

```java
// reference
    "(?:(?<androidId>\\d+)\\+)?" // optional Android ID

      + "(?<type>\\d+)" // type
  @Test // gh-32365

  void mostSpecific() {
// new
    "(?:(?<androidId>\\d+)\\+)?" // optional Android ID
      + "(?<type>\\d+)" // type
  @Test // gh-32365
  void mostSpecific() {
```

### C10. Comment text is kept outside star blocks

The reference turned some single-line block comments into star blocks and
mangled their text (`/* **/` printed `/*`, `* *`, `*/`). A single-line block
comment now prints as written; only an own-line single-line doc comment is
expanded into a star block, and only multi-line comments are re-aligned.

### C11. Comments do not block normalizations of what they precede

The reference owned a member's Javadoc by the member's first token, so a
modifier with a Javadoc before it was a sorting barrier and
`abstract public int size();` stayed unsorted. A comment before a declaration
now leads the declaration, so its modifiers sort as usual
(`public abstract int size();`). Only a comment between two modifiers is a
barrier (N2).

### C12. Dangling comments keep a shared line, and brackets are no exception

Comments in an empty region or on their own lines before a closer print one per
line inside the delimiters, except that comments that shared a source line keep
sharing it (the reference split `{/* a */ /* b */}` into two lines, and
re-joined comments it had printed from a removed `;`). The reference kept
comments in empty brackets on the bracket line (`int[ /* c */] x`); they now
print like every other dangling comment:

```java
// new
  int[
    /* c */
  ] x;
```

### L1. No stray blank lines in `for` headers and empty switch groups

A broken `for` header with an empty update part (`for (init; cond;)`), and a
switch whose last group has no statements (`default:` before `}`), printed a
blank line in the reference. Neither does now.

### F1. Formatter-ignore works item by item

The reference spliced the raw source of a whole off region, including the text
between items. Now each item in the region is printed as written, and what lies
between items is formatted: comments between items are re-indented like any
own-line comment, items that shared a source line each start their own line, and
the source's blank lines between items are kept (one at most). A region whose
items are one per line with ordinary indentation, as in every reference fixture,
prints exactly as before.
