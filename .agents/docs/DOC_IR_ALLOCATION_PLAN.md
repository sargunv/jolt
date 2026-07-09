# Doc IR Allocation Plan

## Summary

The formatter pipeline already gets a large performance win by borrowing from
shared source, token, trivia, and syntax buffers for the lifetime of a
formatting run. The document IR does not follow that model yet. It still builds
an owned, recursive tree with many small heap allocations, and that allocation
shape is a major remaining formatter cost.

The preferred direction is to move the doc IR to shared arena/tape storage owned
by the formatter run. `Doc` should become a small copyable handle into that
storage instead of an owned recursive value.

## Current Shape

`crates/jolt_fmt_ir/src/document.rs` currently represents documents roughly as:

```rust
pub struct Doc<'source>(DocKind<'source>);

enum DocKind<'source> {
    Nil,
    Text(Text<'source>),
    LiteralText(LiteralText<'source>),
    Concat(Vec<Doc<'source>>),
    Group(Group<'source>),
    Indent(Indent<'source>),
    Line(Line),
    IfBreak(IfBreak<'source>),
}

struct Group<'source> {
    contents: Box<Doc<'source>>,
}

struct Indent<'source> {
    contents: Box<Doc<'source>>,
}

struct IfBreak<'source> {
    breaks: Box<Doc<'source>>,
    flat: Box<Doc<'source>>,
}
```

Important allocation sources:

- `Concat(Vec<Doc<'source>>)` allocates a fresh vector for most concatenations.
- `Group`, `Indent`, and `IfBreak` allocate boxes for child documents.
- `join` clones the separator document for every item.
- Many Java and Kotlin formatter helpers build a `Vec<Doc>` only to immediately
  pass it to `concat`.
- The renderer creates transient vectors for command stacks and flat-fit probe
  overlays.

This is the inverse of the syntax pipeline's shared-buffer model. The formatter
avoids cloning source data, but then allocates heavily while describing layout.

## Target Shape

Store doc nodes and child lists in shared buffers owned by the formatting run:

```rust
pub struct DocArena<'source> {
    nodes: Vec<DocNode<'source>>,
    children: Vec<DocId>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DocId(u32);

enum DocNode<'source> {
    Nil,
    Text {
        text: &'source str,
        width: TextWidth,
    },
    LiteralText {
        text: &'source str,
        final_width: TextWidth,
        line_count: u32,
    },
    Concat {
        start: u32,
        len: u32,
    },
    Group {
        contents: DocId,
        should_break: bool,
    },
    Indent {
        contents: DocId,
        levels: i16,
    },
    Line(Line),
    IfBreak {
        breaks: DocId,
        flat: DocId,
    },
}
```

With this shape:

- `concat` appends child ids into `arena.children` and stores a span.
- `group`, `force_group`, `indent`, and `if_break` store child ids directly.
- Cloning a document is copying a `DocId`, not cloning a recursive subtree.
- `join` can reuse the same separator id between items.
- The renderer traverses indexed nodes and child spans from contiguous buffers.

This should be the main optimization rather than replacing `Vec` with
`SmallVec`. `SmallVec` can reduce some tiny concat allocations, but it keeps the
recursive ownership model, increases `Doc` size, leaves boxed unary nodes in
place, and does not address separator cloning or renderer scratch allocation.

## Formatter API Direction

The formatter context should own or borrow a doc builder for the duration of a
file format:

```rust
pub struct DocBuilder<'source> {
    arena: DocArena<'source>,
}

impl<'source> DocBuilder<'source> {
    pub fn text(&mut self, value: &'source str) -> DocId;
    pub fn literal_text(&mut self, value: &'source str) -> DocId;
    pub fn concat(&mut self, docs: impl IntoIterator<Item = DocId>) -> DocId;
    pub fn group(&mut self, contents: DocId) -> DocId;
    pub fn force_group(&mut self, contents: DocId) -> DocId;
    pub fn indent(&mut self, contents: DocId) -> DocId;
    pub fn if_break(&mut self, breaks: DocId, flat: DocId) -> DocId;
}
```

Then language formatters would produce handles:

```rust
let doc = formatter.doc.concat([
    formatter.doc.text("class"),
    formatter.doc.space(),
    name,
]);
```

The render entrypoint should take the arena plus root handle:

```rust
render_to(&arena, root, options, sink)
```

The current free-function constructor API (`concat`, `group`, `text`, etc.) is
ergonomic, but it encourages owned recursive documents. During migration it may
be useful to keep a compatibility layer, but the long-term API should make the
shared doc storage explicit enough that accidental heap-backed docs cannot creep
back in.

## Migration Plan

1. Add `DocArena`, `DocId`, and arena-backed `DocNode` to `jolt_fmt_ir` behind
   the existing document module boundary.
2. Add a builder API that allocates nodes and child spans into the arena.
3. Port the renderer to traverse `(arena, root: DocId)` while preserving current
   rendering behavior and flat-fit bounds.
4. Update `JavaFormatter` and `KotlinFormatter` contexts to own or borrow the
   builder for one formatting run.
5. Convert formatter rules from returning owned `Doc<'source>` to returning
   `DocId` handles.
6. Remove or sharply limit the old owned `Doc` constructors once both formatters
   use arena-backed documents.

This is a broad mechanical refactor, but it has a clean boundary: document
construction and rendering. It should not change syntax access, formatting
policy, or output snapshots.

## Renderer Scratch

The renderer has its own allocation sources independent of doc construction:

- `Renderer::render_doc` creates a command stack.
- `FitChecker` creates a group stack per probe.
- `FitStack` creates an overlay vector per probe.

Flat-fit already has `FLAT_FIT_COMMAND_BUDGET`, so the probe cost model is
bounded. The scratch storage should follow the same principle:

- Reuse render and fit stacks for the duration of a render where practical.
- Consider fixed/inline storage only for bounded scratch, not for the primary
  doc representation.
- Avoid introducing unbounded layout search or best-fitting behavior.

Renderer scratch optimization is secondary to arena-backed docs, but it is worth
doing after the IR is moved because fit probes are frequent in group-heavy code.

## Constraints

- Preserve the formatter invariant that syntax crates own tree shape and token
  ownership; formatter crates should consume structured accessors for layout.
- Preserve all source tokens and trivia through existing token formatting
  helpers.
- Do not replay raw source text except for formatter-ignore ranges.
- Keep layout algorithms linear or explicitly bounded. The arena change should
  improve allocation shape without adding conditional-group search or unbounded
  fitting.
- Prefer output-preserving migration with snapshot coverage over policy changes.
