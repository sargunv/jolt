# Formatter

This page describes how Jolt's Java formatter works, from source text to
formatted output.

## Pipeline

Formatting is a one-way pipeline:

```
Java source
  -> parse        lexer + parser
  -> syntax tree  lossless concrete syntax tree
  -> format       tree -> document IR
  -> document     layout description
  -> render       fit-or-break renderer
  -> text         streamed to a sink
```

The same pipeline drives the native CLI and the WebAssembly dprint plugin. A
facade dispatches by language (just Java today) into a language-specific
lowering, then into the shared renderer.

## Syntax tree

The parser produces a lossless concrete syntax tree: a representation that
retains every byte of the source, including whitespace and comments. Every
character has a home in the tree, so the original text is fully reconstructable
and formatting never drops or relocates a comment by accident.

The parser is error-resilient. Source with syntax errors still produces a
complete tree where practical, with diagnostics attached rather than a hard
failure—so the formatter can run on partially edited files and produce
reasonable output.

Whitespace and comments are stored as **trivia** attached to tokens. Each token
carries two runs of trivia: **leading** (everything before it on the current or
previous lines) and **trailing** (horizontal whitespace and same-line comments
after it, never crossing a line break).

```
Source:    // greeting
           String name = "world";

token      leading trivia              trailing trivia
------     -----------------------     ----------------------
String     "// greeting", newline      space
name       (none)                      space
=          (none)                      space
"world"    (none)                      (none)
;          (none)                      (none)
```

## Document IR

The formatter does not emit text directly. Each construct lowers into a
**document**: a small algebra of layout operators.

| Operator                                       | Meaning                                                                                      |
| ---------------------------------------------- | -------------------------------------------------------------------------------------------- |
| `text`, `space`, `nil`                         | Literal output.                                                                              |
| `literal_text`                                 | Multiline pre-rendered span (block comments, already-formatted snippets).                    |
| `concat`, `join`                               | Sequencing.                                                                                  |
| `group`                                        | A region rendered on one line if it fits, broken across lines otherwise.                     |
| `force_group`                                  | A region that always breaks.                                                                 |
| `indent`                                       | Increase the indent level for a region.                                                      |
| `line`, `soft_line`, `hard_line`, `empty_line` | Break points whose effect depends on whether their enclosing group is on one line or broken. |
| `if_break`                                     | Emit one document when the enclosing group is broken, another when it is on one line.        |

A `group` is the unit of layout decision. The renderer checks whether a group's
content fits within the configured line width; if it does, the group stays on
one line, and otherwise it breaks at its break points.

Java source:

```
foo(first, second, third)
```

Lowered to a document with a group wrapping the call, with break points (·)
between the arguments:

```
group( "foo(" indent( first · second · third ) ")" )
```

If the group fits the line width, each · becomes a space:

```
foo(first, second, third)
```

If it does not fit, each · becomes a newline and the indent applies:

```
foo(
  first,
  second,
  third,
)
```

This algebra follows Wadler's "A prettier printer" and Prettier's implementation
of it.

## Further reading

- [Wadler, "A prettier printer"](https://homepages.inf.ed.ac.uk/wadler/papers/prettier/prettier.pdf),
  the document-IR tradition Jolt's `Doc` follows.
- [Prettier, "Technical details"](https://prettier.io/docs/technical-details),
  the practical IR and layout algorithm that popularized Wadler's approach.
- [Biome, "Architecture"](https://biomejs.dev/internals/architecture/), the
  lossless-CST and trivia model Jolt's tree is kin to.
- [Oxfmt](https://oxc.rs/docs/guide/usage/formatter.html) and
  [Ruff formatter](https://docs.astral.sh/ruff/formatter/), practical IR design
  and option philosophy.
