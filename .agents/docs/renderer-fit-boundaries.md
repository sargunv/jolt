# Renderer Fit Boundaries

## Problem

Jolt currently decides whether a `Group` fits by measuring only that group's
document body. This misses mandatory same-line syntax that is emitted after the
group as a sibling.

The failure that exposed this is an assignment expression statement whose
assignment group fits exactly at the configured line width, followed by a
semicolon:

```java
currentEstimate = (currentEstimate + xxxxxxxxxxxxx / currentEstimate) / 2.0f;
```

The assignment group fits at column 80, but the semicolon is outside the group.
The renderer accepts the flat assignment and then prints `;`, producing an
81-column line.

This is not a Java-only semicolon issue. The same class of bug can appear any
time a breakable group is followed by mandatory syntax that must remain on the
same physical line:

- statement semicolons after expressions, returns, throws, yields, asserts, and
  variable declarations;
- mandatory separators after list entries;
- closing delimiters when they are modeled outside the group that owns the break
  decision;
- comment suffix boundaries if delayed trailing content is not included in the
  fit model.

## Current Jolt Model

The Java layout builder emits possible breakpoints with the document IR:

- `line()` means space in flat mode and newline in break mode.
- `soft_line()` means empty in flat mode and newline in break mode.
- `hard_line()` always emits a newline.
- `group(doc)` tries `doc` flat and expands only if `doc` itself does not fit.
- `if_break` and related helpers branch on the current or labelled group state.

The renderer owns the flat-vs-break decision, but only among breakpoints already
encoded by the layout builder. It does not discover new breakpoints.

The important limitation is that `group_fits` measures `group.contents` in
isolation from later sibling docs. It does account for the current column and
indent state, so already-consumed context reduces the fit budget. It does not
account for future same-line tokens that have not been consumed yet.

That makes trailing mandatory syntax different from indentation:

```text
already-written indent/prefix -> visible in renderer state
future semicolon/comma/closer -> invisible to current group fit
```

## Upstream Formatter Pattern

Local references in `.fixtures/repos` point to a different model in mature
formatter cores.

Ruff, Biome, and Oxc measure group fits against the current print queue rather
than against a pre-sliced group body. When a `StartGroup` is encountered, the
fit pass walks upcoming format elements until a line boundary or the relevant
fit predicate stops measurement. Ordinary tokens in that queue count toward the
fit width.

Consequences:

- mandatory syntax is usually normal stream content, not a special suffix;
- semicolons, commas, and closing delimiters count if they occur before the next
  line boundary;
- optional trailing separators are conditional docs tied to an enclosing group
  state, for example `if_group_breaks(",")`;
- `line_suffix` is mainly for delayed end-of-line content such as comments, not
  mandatory syntax.

`prettier-java` uses Prettier-style builders and commonly groups delimiters and
optional trailing separators with their enclosing construct. It also appends
many mandatory semicolons as plain sibling docs, which relies on the underlying
printer's broader fit behavior to avoid the boundary bug.

## Design Implication

A suffix-aware Java expression or statement helper would be too narrow. It would
fix one symptom while leaving the renderer model vulnerable wherever a breakable
group is followed by required same-line syntax.

The stronger invariant is:

> A group fit decision must include all mandatory ordinary document content that
> would be printed on the same line before the next hard boundary.

Two broad implementation directions can satisfy this:

1. Move Jolt's renderer toward a queue/stack model where fitting begins at the
   current group and sees subsequent same-line docs.
2. Add an explicit bounded IR construct for reserving or attaching mandatory
   same-line trailing content to a group fit decision.

The queue/stack model matches Ruff, Biome, and Oxc more closely. It also avoids
spreading suffix plumbing through every Java and Kotlin syntax form that can end
with required punctuation.

If Jolt adopts queue-based fitting, it should preserve the project invariant
that rendering remains linear or explicitly bounded. The upstream cores do this
with a bounded fit pass over the print queue and state such as
`measured_group_fits` so nested groups do not blindly contradict an enclosing
flat measurement.

## Regression Shape

Tests for this class should avoid pinning broad style output. They should assert
the invariant directly:

- format a construct where the breakable group fits exactly without its trailing
  mandatory syntax;
- ensure the formatted output has no line wider than the configured line width;
- cover distinct syntactic surfaces that append mandatory syntax outside the
  breakable group.

Representative Java cases include expression statements, return statements,
local variable initializers, field initializers, and call expressions followed
by statement semicolons.

Regression coverage should also guard against over-correction:

- fitting must stop at the current line boundary and must not measure into the
  next statement;
- a parent/enclosing fit failure must not blindly force every nested group to
  expand when a nested expression still fits on its continuation line;
- optional break-only syntax, such as trailing commas printed with
  `if_group_breaks`, must remain tied to the intended enclosing group state
  rather than being treated as mandatory flat-width content.
