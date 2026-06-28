# Milestone 7 Document IR Working Plan

Milestone 7 should produce the shared document algebra and renderer that Java
and Kotlin layout builders will target. This is not the Java formatter yet. It
is the formatting substrate that must be expressive enough for the Java
formatter to match upstream oracle behavior without adding ad hoc rendering
logic to `jolt_java_fmt`.

## Reference Audit

### Wadler and Prettier

Prettier is the closest conceptual reference for Jolt's shared layer. Its
document model is a tree of text, concatenation, groups, lines, indentation,
conditional output, fill, and line suffixes. The printer uses a command stack,
tries groups in flat mode, measures whether they fit in the remaining width, and
falls back to break mode when they do not fit.

Useful lessons:

- `Group` is the core layout decision. Layout builders should describe where a
  region may flatten; the renderer should decide whether it actually flattens.
- `Line`, `SoftLine`, and `HardLine` need distinct flat-mode behavior.
- `IfBreak` must be able to target either the current group or a labelled group.
  Java needs this for trailing separators and profile-specific punctuation.
- `Fill` is required for independently packed sequences where breaking one
  separator should not force every separator in the enclosing group to break.
- `LineSuffix` and `LineSuffixBoundary` are the clean way to print trailing
  comments without every layout builder manually knowing the end of the current
  rendered line.
- `ConditionalGroup` or `BestFitting` is expensive when nested, but useful for
  rare constructs that need more than a flat-vs-broken choice.

Primary references:

- Prettier command documentation:
  https://github.com/prettier/prettier/blob/main/commands.md
- Prettier document builders:
  https://github.com/prettier/prettier/tree/main/src/document/builders
- Prettier printer:
  https://github.com/prettier/prettier/blob/main/src/document/printer/printer.js

### Biome and Ruff

Biome and Ruff are the strongest Rust references. They validate that a
language-agnostic IR works well across syntax frontends, but also show where a
plain recursive enum gets stretched. Their format element layer includes line
modes, source/token text, line suffix boundaries, best-fitting variants, group
labels, indentation tags, and validation for invalid document structure.

Useful lessons:

- Text width should be stored or cheaply computable with the text element. This
  avoids recomputing widths during fit checks and leaves room for
  profile-specific width policy.
- `LineMode` should include an empty-line form. Java blank-line behavior should
  not be represented only by accidentally adjacent hard lines.
- `BestFitting` should be represented directly instead of encoded as nested
  `IfBreak` chains. It gives the renderer one bounded place to try alternatives.
- Group labels are not optional. `IfBreak` and `IndentIfBreak` are significantly
  less useful if they can only inspect the nearest group.
- The renderer should reject invalid document structure in tests. A broken IR
  should be a formatter bug, not silently weird output.

Primary references:

- Biome formatter crate: https://docs.rs/biome_formatter/latest/biome_formatter/
- Biome `FormatElement` source:
  https://github.com/biomejs/biome/blob/main/crates/biome_formatter/src/format_element.rs
- Biome printer source:
  https://github.com/biomejs/biome/blob/main/crates/biome_formatter/src/printer/mod.rs
- Ruff formatter source:
  https://github.com/astral-sh/ruff/tree/main/crates/ruff_formatter

### google-java-format

google-java-format is the Java compatibility reference. It does not expose a
Prettier-style document tree, but its `OpsBuilder` and `Doc` classes show which
layout concepts Java formatting needs: open/close levels, tokens, spaces,
optional breaks, forced breaks, fill-mode breaks, flat text for an unbroken
break, extra indentation when a break is taken, and tags that remember whether a
break was taken.

Useful lessons:

- Jolt needs a break primitive richer than just `Line` and `SoftLine`.
- A break may have flat text such as `" "` or `""`.
- A break may add extra indentation only when the break is taken.
- Java needs both unified breaks and independently packed breaks.
- Some Java decisions depend on whether a break was taken. Jolt should model
  this as group ids plus `IfBreak` first, and add explicit break tags only if
  oracle work proves group ids are insufficient.

Primary references:

- google-java-format repository: https://github.com/google/google-java-format
- `Doc.java`:
  https://github.com/google/google-java-format/blob/master/core/src/main/java/com/google/googlejavaformat/Doc.java
- `OpsBuilder.java`:
  https://github.com/google/google-java-format/blob/master/core/src/main/java/com/google/googlejavaformat/OpsBuilder.java

### dprint

dprint is useful as a contrast. Its formatting core is an imperative print-item
stream with signals, conditions, anchors, line/column queries, and reevaluation.
That model is powerful and fast for hand-written formatters, but it is broader
than Jolt needs for milestone 7.

Useful lessons:

- Avoid a general dynamic condition engine for now. It complicates renderer
  state, backtracking, and validation.
- Keep a narrow declarative IR first. If Java later needs line/column dependent
  conditions that cannot be expressed by groups, add them as explicit, named
  algebra instead of exposing arbitrary callbacks.
- The renderer should still expose enough internal state for tests and future
  debugging: current line, current column, group modes, and line suffix queues.

Primary references:

- dprint core formatting module:
  https://github.com/dprint/dprint/tree/main/crates/core/src/formatting
- dprint `print_items.rs`:
  https://github.com/dprint/dprint/blob/main/crates/core/src/formatting/print_items.rs
- dprint printer:
  https://github.com/dprint/dprint/blob/main/crates/core/src/formatting/printer.rs

## Design Direction

Use a declarative document tree with explicit builder functions. Do not expose a
mutable printer API to language layout builders. `jolt_java_fmt` should build a
`Doc`; `jolt_fmt_ir` should render it.

The renderer should use a Prettier/Biome-style command stack:

1. Start with the root document in expanded mode.
2. When a `Group` is encountered, try rendering its contents in flat mode with a
   fit checker against the remaining line width.
3. If the group fits, render it flat and record its group mode.
4. If it does not fit, render it expanded and record its group mode.
5. In flat mode, soft breaks render as their flat text.
6. In expanded mode, soft breaks render as newlines with current indentation.
7. Hard and empty lines always break and may force containing groups to expand.
8. `Fill` packs alternating content and separator entries independently.
9. `LineSuffix` entries are buffered and flushed before the next real newline or
   explicit boundary.

The initial implementation can use owned `Doc` values and `Vec<Doc>` for
children. Do not introduce an arena, interner, or tag-stream representation in
milestone 7 unless tests show the recursive form is a real problem. Biome's
tag-stream approach is a performance and allocation optimization, not a
requirement for the first Jolt renderer.

## Proposed IR Algebra

This is the full algebra Jolt should plan around for Java formatting. Some
builders can be thin convenience functions, but the renderer should have first
class behavior for each semantic category.

```rust
pub enum Doc {
    Nil,
    Text(Text),
    LiteralText(Text),
    Concat(Vec<Doc>),
    Group(Group),
    Fill(Vec<FillEntry>),
    Indent(Indent),
    Align(Align),
    Line(Line),
    IfBreak(IfBreak),
    IndentIfBreak(IndentIfBreak),
    LineSuffix(Box<Doc>),
    LineSuffixBoundary,
    BestFitting(Vec<Doc>),
    BreakParent,
}
```

Supporting types:

```rust
pub struct Text {
    pub text: Box<str>,
    pub width: TextWidth,
}

pub struct Group {
    pub id: Option<GroupId>,
    pub should_break: bool,
    pub fit: GroupFit,
    pub contents: Box<Doc>,
}

pub enum GroupFit {
    LineWidth,
    MarkedBreak {
        marker: BreakMarkerId,
        max_column_before_last_marked_break: TextWidth,
    },
}

pub struct FillEntry {
    pub content: Doc,
    pub separator: Option<Doc>,
}

pub struct Indent {
    pub levels: u16,
    pub contents: Box<Doc>,
}

pub struct Align {
    pub spaces: u16,
    pub contents: Box<Doc>,
}

pub struct Line {
    pub mode: LineMode,
    pub flat: FlatLine,
    pub indent_delta: i16,
    pub propagate_break: bool,
    pub marker: Option<BreakMarkerId>,
}

pub enum LineMode {
    Soft,
    SoftOrSpace,
    Hard,
    Empty,
}

pub enum FlatLine {
    Empty,
    Space,
    Text(Box<str>, TextWidth),
}

pub struct IfBreak {
    pub group_id: Option<GroupId>,
    pub breaks: Box<Doc>,
    pub flat: Box<Doc>,
}

pub struct IndentIfBreak {
    pub group_id: GroupId,
    pub contents: Box<Doc>,
    pub negate: bool,
}
```

### Required In Milestone 7

`Nil`: empty document.

`Text`: formatted text that must not contain `\r` or `\n`. Use this for Java
keywords, punctuation, identifiers, and most comments after comment formatting
has normalized their internal layout.

`LiteralText`: source text that may contain newlines and must be emitted
verbatim. Use this for Java text block literals and any future verbatim
fallback. The renderer must update line and column state while preserving the
literal bytes.

`Concat`: ordered sequence.

`Group`: flatten-or-break decision point. It must support ids, forced break, and
fit constraints. Most groups should use normal line-width fitting. Palantir
method-chain prep needs a narrow marked-break fit constraint for cases such as
"the group may fit globally, but reject flat mode if the last marked chain break
would start after profile column N."

`Fill`: independent packing for lists, comments, annotations, and import/static
import groups where breaking one separator should not force every separator in
the outer group.

`Indent`: add one or more configured indent levels after nested line breaks.

`Align`: add fixed spaces after nested line breaks. This should exist even if
the Java formatter uses it sparingly; Java continuation and chained-call
experiments need a way to express visual alignment without abusing text.

`Line`: one primitive that backs `line`, `soft_line`, `hard_line`, `empty_line`,
and google-java-format-style breaks with custom flat text and break-only
indentation. It should be able to carry an optional marker used only by group
fit constraints; this is not a general break tag.

`IfBreak`: conditionally print punctuation or whitespace based on whether a
group broke.

`IndentIfBreak`: optimized and explicit form for Java continuation shapes that
depend on an already-labelled group.

`LineSuffix`: trailing comment support.

`LineSuffixBoundary`: stop trailing comments from escaping their syntactic
region.

`BestFitting`: bounded alternatives from most-flat to most-expanded. Google and
AOSP Java formatting should not need it, but Palantir prep is in scope for this
milestone. Keep `BestFitting` as a narrow tool for partial-inlining cases around
nested method chains, lambdas, and call arguments. Do not use it for ordinary
list formatting that `Group` and `Fill` can express.

`BreakParent`: force ancestor groups to expand. Hard lines should usually imply
this, but keeping it explicit lets tests cover propagation and lets the renderer
support hard lines that do not propagate if needed later.

### Deliberately Deferred

`Cursor`: not needed for formatting correctness. Add with editor integration.

`Trim`, `MarkAsRoot`, `DedentToRoot`: useful for Markdown, preprocessors, and
template languages, but not Java milestone 8.

General dynamic conditions: dprint proves this can work, but Jolt should avoid
callbacks or renderer-state predicates until a Java or Kotlin profile has a
specific construct that cannot be expressed with groups, fill, and `IfBreak`.

Break tags separate from group ids: google-java-format uses break tags. Jolt
should first attempt to model these with group ids and `IfBreak`; add
`BreakId`/`IfBreakTaken` only after a failing oracle case demonstrates the need.

Dynamic alignment: neither google-java-format nor Palantir Java Format appears
to require current-column or anchor-based alignment for Java compatibility.
Fixed-space `Align` is enough for milestone 7. Palantir's distinctive behavior
belongs in chain fit policy, not dynamic alignment.

## Width Policy

Text width should be stored on text elements. The default builder can compute
Unicode display width, following Biome and Ruff, but the type should allow
explicit widths. That keeps the renderer generic and leaves room for Java
compatibility if a profile needs google-java-format-style character counting.

The Java Google/AOSP oracle builders should supply google-java-format-compatible
UTF-16 code-unit widths for Java tokens and comments. The shared renderer must
not bake this policy in as the default because Kotlin and future profiles may
prefer Unicode display width.

Renderer fit checks must use `TextWidth`, not `text.len()`.

## Renderer Details

Inputs:

- `Doc`
- `RenderOptions { line_width, indent_width, indent_style, line_ending }`

Outputs:

- `Rendered { text, stats }`
- `RenderError` for invalid document structure or invalid text

Core state:

- output buffer,
- current line number,
- current column,
- current indentation,
- command stack,
- group mode map,
- pending line suffix stack,
- fit-check scratch stack.

Fit checker:

- Simulates rendering from a command stack without writing output.
- Returns `false` when width exceeds `line_width`.
- Stops successfully at command exhaustion, hard line, or a break in expanded
  mode depending on the fit context.
- Treats pending line suffixes as consuming width before deciding a group fits.
- Uses the most expanded `BestFitting` alternative as the guaranteed fallback.

Validation:

- Reject `Text` and `FlatLine::Text` containing line terminators.
- Reject empty `BestFitting`.
- Reject `Fill` entries where separators are malformed.
- Reject `IndentIfBreak` pointing to a group id that was never observed by the
  time it is needed.
- Reject marked-break group fit constraints if the referenced marker is never
  observed during fit checking.
- Keep `LiteralText` legal but test column tracking across embedded newlines.

## Builder API

Expose small, obvious builders rather than requiring callers to construct all
structs directly:

```rust
nil()
text("class")
literal_text(source_text)
concat([a, b, c])
join(separator, docs)
group(doc)
group_id(id, doc)
force_group(doc)
group_with_fit(fit, doc)
fill(entries)
indent(doc)
indent_by(levels, doc)
align(spaces, doc)
line()
soft_line()
hard_line()
empty_line()
break_(flat, indent_delta)
marked_break(marker, flat, indent_delta)
if_break(breaks, flat)
if_group_breaks(id, breaks, flat)
indent_if_break(id, doc)
line_suffix(doc)
line_suffix_boundary()
best_fitting([flat, partially_expanded, expanded])
break_parent()
```

Do not add convenience APIs that only rename source definitions. Each builder
above carries semantic behavior or prevents callers from constructing invalid
documents.

## Test Plan

Add focused renderer tests in `crates/jolt_fmt_ir`:

- text and concat render without allocation surprises,
- `Text` rejects line terminators,
- flat group fits on one line,
- group expands when width is exceeded,
- group fit constraints reject otherwise-fitting Palantir chain layouts,
- nested groups remeasure after hard lines,
- `SoftLine`, `Line`, `HardLine`, and `EmptyLine` flat and expanded behavior,
- indentation after nested line breaks,
- break-only `indent_delta`,
- marked break references,
- alignment spaces,
- `IfBreak` against current group,
- `IfBreak` against labelled group,
- `IndentIfBreak`,
- `Fill` packs independently,
- `BestFitting` chooses the first fitting variant,
- `LineSuffix` flushes before newline,
- `LineSuffixBoundary` prevents comment escape,
- `LiteralText` preserves embedded newlines and updates column,
- `BreakParent` propagates expansion,
- explicit text widths affect fit decisions.

Add a small Java-shaped test module using only the IR, not the Java CST:

- method invocation arguments,
- chained method calls,
- Palantir nested chains inside call arguments,
- Palantir lambda argument followed by chained calls,
- class body with blank lines,
- trailing line comments,
- block comment before a declaration,
- annotation argument list,
- lambda body,
- text block literal.

These tests should prove the algebra can express Java layout shapes before
milestone 8 starts.

## Implementation Steps

1. Replace the `jolt_fmt_ir` placeholder with public IR structs, ids, text
   width, render options, render output, and errors.
2. Add builder functions and keep constructors private where invalid states are
   easy to create.
3. Implement the stack-based renderer for `Text`, `Concat`, `Group`, `Indent`,
   `Align`, and `Line`.
4. Add group mode tracking and `IfBreak`.
5. Add `Fill`.
6. Add `LineSuffix` and `LineSuffixBoundary`.
7. Add `BestFitting`.
8. Add marked-break group fit constraints for Palantir chain prep.
9. Add validation and renderer stats useful for debugging.
10. Add the full renderer test suite.
11. Wire `jolt_fmt_ir` into workspace checks without changing formatter core
    behavior yet.

## Acceptance Criteria

Milestone 7 is complete when:

- `jolt_fmt_ir` exports the planned IR and builder API.
- The renderer is deterministic, filesystem-free, and wasm-compatible.
- Renderer tests cover every supported algebra item.
- Java-shaped IR tests demonstrate the algebra needed by Google, AOSP, and
  Palantir Java formatter profiles.
- `cargo test -p jolt_fmt_ir` passes.
- `mise run test` passes, unless blocked by unrelated repository state that is
  documented at the time.

## Resolved Research Notes

- Google Java Format oracle compatibility requires Java layout builders to
  supply UTF-16 code-unit widths for Java tokens and comments. The shared IR
  still stays width-policy agnostic.
- Group ids plus `IfBreak` and `IndentIfBreak` are enough for milestone 7.
  google-java-format-style break tags remain deferred until an oracle case
  proves they are necessary.
- Google and AOSP formatting do not clearly need `BestFitting`; Palantir prep
  does, specifically for partial inlining around nested chains, lambdas, and
  call arguments.
- Palantir compatibility does not require dynamic current-column alignment.
  Palantir's distinctive behavior is better modeled as marked-break fit policy
  for chains and selected annotation/record-parameter cases.
