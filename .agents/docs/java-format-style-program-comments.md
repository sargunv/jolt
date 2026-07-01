# Jolt Java Style: Program, Comments, Modules, Names, Lexical Structure

This document owns file-level Java formatting policy. Prettier-Java is useful
background for syntax coverage, but these rules are Jolt's style.

## Program Layout

- Empty/comment-only files print comments followed by a final newline.
- Non-empty files end with a final newline.
- Package declarations, import groups, module declarations, and top-level type
  declarations are separated by one blank line where present.
- Redundant top-level semicolons are removed.
- Extra top-level blank lines collapse to one where a separator is needed.

## Package Declarations

- Package annotations print one per line before the `package` declaration.

```java
@ParametersAreNonnullByDefault
package com.example;
```

- The package declaration itself prints as `package <qualified-name>;`.

## Imports

- Formatting sorts imports already present in the file.
- Formatting must not add imports, remove imports, or resolve unused imports.
- Normal imports come before static imports.
- There is one blank line between normal and static import groups when both are
  present.
- Imports sort by a deterministic, locale-free, case-sensitive comparator over
  the import path text or semantic path segments.
- Star imports sort as their own path segment.
- Comments between imports are barriers. Sort only uninterrupted import runs.
- Do not move comments with imports in v1.

```java
import java.util.List;
import java.util.Map;

import static java.util.Comparator.comparing;
import static org.assertj.core.api.Assertions.assertThat;
```

When comments occur between imports:

```java
import z.Zed;

// Used by generated adapters.
import a.Adapter;
import m.Middle;
```

the comment prevents sorting across that boundary.

## Modules

- Module directives are sorted by directive kind first, then by the same
  comparator used for imports.
- Directive kind order is `requires`, `exports`, `opens`, `uses`, `provides`.
- Different directive kinds are separated by one blank line.
- Comments between directives are barriers.
- `requires` modifiers use canonical order: `static` before `transitive`.

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

- Broken `to` and `with` target lists use ordinary one-indent continuation.

```java
module demo {
  exports com.example.internal to
      com.example.consumer.one,
      com.example.consumer.two;

  provides com.example.Plugin with
      com.example.impl.FirstPlugin,
      com.example.impl.SecondPlugin;
}
```

## Qualified Names

- Formatter accessors expose qualified names as semantic segment lists.
- Segment-level annotations and comments must remain representable.
- Dots are normalized tightly.
- Block comments around dots attach to adjacent segments and do not force
  multiline layout.
- Line comments inside qualified names force a leading-dot continuation layout.

```java
com.example.deep.Name
```

## Comments

- Jolt uses leading, trailing, and dangling comment concepts.
- Attachment is computed from Jolt CST roles and source spans, not by porting
  Prettier's global attachment heuristics.
- Constructs that move code own their comment placement.
- Leading comments print before the construct they decorate.
- Trailing comments stay on the same line when they fit and when the construct's
  policy allows it.
- Dangling comments print inside empty delimiters or list/block interiors.

## Star Blocks And Javadocs

- Star-block comments are structurally normalized.

```java
/**
 * Gets the name.
 *
 * @return the name
 */
```

- Javadoc tag parsing, prose reflow, and embedded-language formatting are
  deferred.
- Arbitrary block comments should not be semantically rewritten.

## Formatter Ignore

- Supported ignore ranges use generic or IDE-compatible spelling:

```java
// @formatter:off
int x=       1+2;
foo( a,b,c );
// @formatter:on
```

- Ignore ranges preserve their interior source slices verbatim.
- The surrounding file must still parse.
- Do not support branded compatibility spellings such as `prettier-ignore`.
- Do not introduce a Jolt-branded spelling unless a future need emerges.

## Text Blocks

- Text-block internal indentation and content are preserved exactly for v1.
- Embedded-language formatting in text blocks is deferred.

```java
String json = """
        {
          "name": "Ada"
        }
        """;
```

## Template Expressions

- Java template expressions are out of initial formatter scope unless Java
  reintroduces a stable form.

## Literals

- Numeric literal style may be normalized when semantics are unchanged:
  prefix/suffix casing, hex digit casing, separator grouping, and leading-zero
  decimal forms.
- Do not rewrite string or character escape content as formatter behavior.

```java
int mask = 0xFF;
long count = 1_000_000L;
double ratio = 0.5D;
```

## Accessor Requirements

- Root program items in source order.
- Full token stream where punctuation or keywords affect formatting.
- Semantic qualified-name segment accessors.
- Source spans including comments.
- Original text for ignore ranges, same-line checks, and blank-line checks.
- Comment kind and attachment metadata.
- Import and module directive classification, including static/star/open tokens
  and directive kind.
