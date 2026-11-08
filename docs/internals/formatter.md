# Formatter architecture

This page explains the design of Jolt's formatter for contributors: the crates,
their contracts, the core data structures and algorithms with their costs, and
why each mechanism exists. The Java output itself is specified in
[Java style](../guides/style.md). The signatures and the contract of each item
are in its rustdoc (`cargo doc --workspace --no-deps --document-private-items`);
this page names items by path. When a change makes this page wrong, fix the page
in the same change.

## Goals

- Format Java correctly. For source without syntax errors, the output reparses
  without errors and has the same structure except for the
  [normalizations](#normalizations). Every comment is printed exactly once, and
  formatting the output again changes nothing. The [oracle](#oracle) checks
  these properties, and the corpus sweep (`mise run hunt`) runs it with an empty
  `KNOWN_OPEN`.
- Format code with syntax errors without losing or adding anything: the parsed
  pieces are formatted by the normal rules, a node the parser could not shape is
  printed as written, and no token is synthesized to repair an error.
- Keep every phase linear or explicitly bounded (see [Overview](#overview)).
- Keep the core multi-language-capable: the tree, parser infrastructure,
  document IR, printer, comment placement and formatter-ignore are
  language-agnostic crates. Only Java is built.

The formatter does not check source conservation at runtime (the tests and the
debug [emission check](#emission-check) do), does not assign diagnostics to
owner nodes, and has no style options beyond line width, indent width and tabs.

## Overview

```text
source ──lex──▶ tokens+trivia ──parse──▶ events ──build──▶ SyntaxTree (fixed slots)
                                                              │
                                     comment placement pass ◀─┤
                                     formatter-ignore regions ◀┘
                                                              │
                     Formatter::node (one chokepoint) ──rules──▶ Doc arena
                                                              │
                                              printer ──chunks──▶ Sink
```

Every phase is a single linear pass, except two explicitly bounded searches: the
printer's fit checks (at most `FIT_BUDGET` commands each: one per group or
[hug](#hang-and-hug), at most three per [hang](#hang-and-hug)) and the parser's
speculative lookahead (each scan capped by `MAX_DEPTH` nesting, balanced
brackets answered by a table built once in linear time; see
[Parser rules](#parser-rules)).

## Crates

| Crate               | Kind     | Owns                                                                                                                                                                     |
| ------------------- | -------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `jolt_syntax`       | agnostic | Text sizes and ranges, line index, `Diagnostic`, the lossless arena tree and its builder, borrowed cursors, trivia, parser infrastructure (markers, guards, token sets). |
| `jolt_fmt`          | agnostic | `FormatOptions`, `Sink`, the document IR, the printer, comment placement and printing, verbatim emission, item lists with formatter-ignore, the `Formatter` context.     |
| `jolt_java_syntax`  | Java     | Lexer, recursive-descent parser, the Java schema and the code generated from it (kinds, factory, typed nodes, accessors).                                                |
| `jolt_java_fmt`     | Java     | The Java `FormatLanguage` (tables and comment hooks), node dispatch, helpers, and the layout rules.                                                                      |
| `jolt_markup`       | agnostic | Doc comment layout: the frame, Javadoc's tag syntax, the HTML-lite and Markdown body parsers, and the fill layout that turns them into docs.                             |
| `jolt_formatter`    | facade   | Language detection and the single API the CLI and dprint plugin call.                                                                                                    |
| `jolt_config`       | app      | The user-facing format options (`jolt.toml`, CLI flags, dprint config, JSON schemas).                                                                                    |
| `jolt_test_support` | tests    | Fixture collection, the oracle (structure fingerprint, comment multiset, idempotence, reparse), and the corpus sweep with comment probes.                                |
| `jolt_cli`          | app      | The `jolt` binary.                                                                                                                                                       |
| `jolt_fmt_dprint`   | app      | The dprint WASM plugin.                                                                                                                                                  |

Dependency graph (arrows point at dependencies):

```text
jolt_cli ───────┬─▶ jolt_config ─┐
jolt_fmt_dprint ┘                ▼
                          jolt_formatter ─▶ jolt_java_fmt ─▶ jolt_markup ─▶ jolt_fmt ─▶ jolt_syntax
                                                  │  │                        ▲             ▲
                                                  │  └────────────────────────┘             │
                                                  └──▶ jolt_java_syntax ────────────────────┘
jolt_test_support ─▶ jolt_fmt, jolt_syntax
```

`jolt_formatter` is a small facade because it is the only place that knows the
set of languages and the stable boundary for the CLI and dprint; folding it into
a language crate would invert the dependency direction.

## `jolt_syntax`

### Text, diagnostics and languages

Offsets are `TextSize`, a `u32` byte offset (sources are smaller than 4 GiB),
and ranges are `TextRange`. `LineIndex` turns an offset into a zero-based line
and byte column. A `Diagnostic` is a lexer or parser error: a stable code (such
as `java.parse.expected_syntax`), a message and a range. There is no severity,
stage or owner: every diagnostic is a syntax error at a source range.

`jolt_syntax::Language` holds the static facts the shared crates need about one
language's tree, implemented by a unit struct (`JavaLanguage`): the `EOF` token,
the single `BOGUS` node kind, which every category slot and list item position
accepts, the `BRACKETS` that deep-nesting recovery keeps balanced, the codes of
the diagnostics the shared parser reports, and which kinds are lists. Comment
placement never attaches to a list node, only to its items. Kinds are stored as
`RawSyntaxKind(u16)`.

### Tree storage

The tree is one flat arena per parse: nodes, a shared slot vector, tokens, and
trivia, all with `u32` indices. A node stores its kind, flags, its slot range,
its token range, its parent and its index in the parent. A token stores its
parent node, so `SyntaxToken::parent` is `O(1)`. Slots are fixed per node kind
(see the factory below); an empty slot is a valid absent optional or a missing
required element.

Flags, two bits, identical in debug and release builds:

- `MALFORMED`: the node is the bogus kind, or its children did not fit its
  kind's shape and are stored raw.
- `HAS_RECOVERY`: the node or any descendant is malformed or has a missing
  required slot. Propagated to ancestors during the build.

There are no debug-only fields anywhere in the tree, cursors, comments, or
document arena. A struct has one layout in every profile.

### Cursors

Cursors borrow the tree and the source. They are not generic over the language;
language crates wrap them in typed nodes.

`SyntaxNode`, `SyntaxToken` and `SyntaxElement` (either of the two) are `Copy`
handles: the tree, the source and a `u32` id. Two handles are equal when they
have the same id in the same tree; equality and hashing compare the tree
pointer, not the source, since comparing the source by content would make every
comparison linear in the file. Every step a formatter takes is `O(1)`: a token's
parent, the tokens before and after it (one flat token vector in source order),
and a node's first and last token (a node stores its token range).
`SyntaxNode::slot` reads one fixed slot for the typed accessors and returns
`None` for an empty slot and for every slot of a malformed node, whose children
are raw; `SyntaxNode::children` gives the raw children of any node.
`SyntaxNode::descendants` walks iteratively, so deep trees cannot overflow the
stack.

Typed nodes implement `jolt_syntax::AstNode` (`can_cast`, `cast`, `syntax`). A
list's elements are `jolt_syntax::ListElement`s: an item, a separator, a missing
item or separator, or a `Bogus` node at an item position whose type cannot hold
it.

Trivia attachment: a token's trailing trivia is the horizontal whitespace, line
comments and single-line block comments that follow it up to (not including) the
next line break; a block comment that contains a line break ends the trailing
trivia and, like everything after it, is the following token's leading trivia.
The final `Eof` token carries the file's trailing trivia. Comment placement
never depends on which token's trivia holds a comment; it uses only the tokens
around the comment and the line breaks between them (see
[Classification](#classification)).

### Parser infrastructure

The parser emits events through markers (`start`, `complete`, `precede`,
`abandon`, forward parents). Diagnostics have no owner node: the parser reports
a diagnostic at the current token, and the factory discovers missing required
slots from the children.

The pieces a grammar is written with:

- `TokenSet`, a `u128` bitset of token kinds (codegen asserts that the token
  kinds fit in `0..128`).
- `Parser::expect`, which eats a token or records an `expected` diagnostic; the
  factory turns the absent token into a missing slot.
- `LoopGuard`, the release-active progress guard: every grammar loop that calls
  back into grammar functions holds one and calls `check` at the top of each
  iteration. If no token was consumed since the previous `check`, it wraps the
  current token in a `BOGUS` node with an `unexpected` diagnostic.
- `Parser::nested`, which runs a grammar function one recursion level deeper, or
  returns `None` without running it when `MAX_DEPTH` (128) levels are active,
  and `Parser::recover_too_deep`, the one deep-nesting recovery, which wraps
  tokens into a single `BOGUS` node up to a stop token at bracket depth zero.
- `Parser::skip_balanced`, which skips tokens up to a stop token outside the
  brackets the skipped tokens open.
- `Parser::split_current`, which splits a compound `>` token (see
  [Lexer](#lexer)).
- `Parser::lookahead`, a read-only cursor for speculative lookahead.

`Marker` has a drop check: a debug assertion that a marker was completed or
abandoned. It guards programmer error, not input.

Cost: the guard makes every loop advance by at least one token per iteration, so
a parse is `O(tokens)` iterations in release builds regardless of grammar bugs.
Recursion is bounded by `MAX_DEPTH` active `nested` calls; the value 128 fits
the 1 MiB WASM stack. A block that is the body of a control or labeled statement
takes no level of its own (the statement took one), so brace promotion (N4)
never makes the output nest deeper than the input, and an `else if` chain is
parsed in a loop and takes one level however long it is.

### Tree building and the factory

`jolt_syntax::build_tree` replays the parser's events into the arena and calls
the language's generated `SyntaxFactory::make` for each node.

The builder is infallible: the marker API cannot produce an unbalanced stream,
so a malformed event stream is a parser bug and panics with a message.

Factory rules (generated, see the Java schema):

1. A fixed node matches each child against its slots in order. A required slot
   with no matching child is left empty and the node gets `HAS_RECOVERY`.
2. A category slot accepts the category's members and the bogus kind.
3. A list accepts its item matcher and the bogus kind at every item position.
   Separated lists record missing items and separators as empty slots. A list
   child that fits neither (a constructor in an interface body, `[]` in an
   argument list) is wrapped in a new bogus node at an item position, so a list
   node is never malformed and its other items keep their structured layout.
4. In a fixed node, any child that fits no slot makes the whole node `MALFORMED`
   (raw children, kind kept so the node still fits its parent's slot).
5. Every mismatch of rules 3 and 4 that no parser diagnostic overlaps gets an
   `unexpected` diagnostic from the builder, so a tree with a bogus or malformed
   node always has a diagnostic and the CLI refuses it by default.

Because of rule 4, a typed accessor never needs a "malformed" variant: a field
is `Option<T>`, where `None` is absent-or-missing and `Some` is always a node of
the declared type (which may itself be malformed and is then formatted
verbatim).

## `jolt_java_syntax`

### Lexer

The lexer translates Unicode escapes into a temporary buffer before lexing and
maps token and trivia ranges back to the raw source, so the tree is a plain
borrow of the source and the output keeps escapes exactly as written. `>` is
lexed by maximal munch into `Gt`, `GtEq`, `RShift`, `RShiftEq`,
`UnsignedRShift`, `UnsignedRShiftEq` and is **never split eagerly**, so binary
and assignment operators are single tokens.

Where a type argument or type parameter list expects its closing `>` and the
current token is one of the compound kinds, the consuming parser calls
`split_current(Gt, remainder)` (`>>` → `>` `>`; `>>>` → `>` `>>`; `>=` → `>`
`=`; `>>=` → `>` `>=`; `>>>=` → `>` `>>=`) and eats the `>`. Speculative generic
lookahead runs on a forked cursor and counts the `>` characters of a compound
token without splitting. The consuming parser never rewinds, so a split is never
undone.

### Schema and generated code

One schema is the single source of truth for kinds and tree shape. It is plain
Rust data in `crates/jolt_java_syntax/tests/codegen/schema.rs` (struct literals
or small builder functions), so there is no schema language and no schema parser
to maintain. Its vocabulary: tokens; categories; nodes with
`required`/`optional`/`many`/`one_or_more` fields over `token`, `token_set`,
`contextual`, `node`, `node_set`, `category`, `list`, `choice`; separated lists
with separator, minimum and trailing policy. There is one `Bogus` kind, accepted
wherever factory rule 2 or 3 applies.

`crates/jolt_java_syntax/tests/codegen/main.rs` renders the schema to
`crates/jolt_java_syntax/src/generated.rs` (checked in, `// @generated`). The
test fails with a diff when the file is stale and rewrites it when
`INSTA_UPDATE=always` (what `mise run test --update` sets). The generator is
plain Rust, not macros, so the generated code is greppable and has real rustdoc.
It emits the `JavaSyntaxKind` enum, the factory (one match arm per kind), and
the typed tree:

- One newtype per node kind (`ClassDeclaration`), with one accessor per field.
  Every accessor returns an `Option`: a token, a node, or a list node.
- One enum per category (`Expression`) and per `choice` field
  (`ModifierElement`). Every node alternative is a variant, token alternatives
  share one `Token` variant, and every such enum has a `Bogus` variant.
- A list whose item type holds `Bogus` (a plain list of a category or choice)
  has `items()`. Every other list, and every separated list, has only
  `elements()`, which yields `ListElement`s that keep bogus items and
  separators, so no rule can skip a bogus item by accident.

Semantic helpers that are not schema projections are hand-written in
`jolt_java_syntax/src/semantics.rs` and carry real behavior: operator precedence
by token kind, assignment operators, contextual keyword tests, modifier
partitioning (declaration vs type-use annotations). Nothing in it re-derives a
schema choice.

### Parser rules

- Every `while`/`loop` in `parser/grammar/**` that calls a grammar function
  holds a `LoopGuard`. A loop without a guard is a pure token scan that always
  bumps.
- Recursive grammar owners use `nested`; on `None` they call `recover_too_deep`
  with the stop set of their position (expression: `; , ) ] }`; pattern: those
  and `-> : && || ?`; statement and body member: `}`; type: `> , ) ] {` plus the
  `>` compounds). The result is always one `Bogus` node at a category or list
  position.
- No expression contains a `;` or `}` outside the brackets it opened, so
  expression recovery (`parse_expression_until`) stops at either, and so do the
  element loops of argument, parameter, annotation-argument and
  array-initializer lists: an unclosed list ends where its statement, header or
  body does, with a missing closer, and the next statement parses normally
  (`f(;` / `g();` is two statements, not one `Bogus` argument that runs to the
  block's `}`). A parameter list also ends at `{`. A type-body member is a field
  only when a type and a name start it; a member that is no declaration
  (`consume_body_member_fragment`) is one `Bogus` that ends after a `;` or a
  balanced braced body, so a malformed constructor (`Example) {…}`) is one bogus
  member and the next member parses normally. In a module declaration a name
  ends where the next name starts, with no keyword matching: junk after a name
  is one `Bogus` up to `;`, `}` or the next name (`requires a + b;` recovers `b`
  as the next item), and a dangling `.` joins the next word into the name
  (`exports a. to b;` exports `a.to`), as the grammar reads it.
- Recovery has exactly three representations: an empty required slot (missing),
  a `Bogus` node at a category or list position (unexpected tokens), and a
  `MALFORMED` node whose children did not fit its shape. There is no other.
- One lookahead table, built in linear time when the parser is created, stores
  for every token where the scans that start there end: the matching `)` or `]`
  of a `(` or `[`, the first `{` or `;` of a member header and the first `;` or
  `}` of a statement outside brackets, and the end of a run of annotations and
  of a run of annotations and modifiers. Queries are `O(1)`. Splitting a `>`
  token changes no token the table depends on and no token index. Whether a
  `for` header has a top-level `:` and whether a switch label ends in `->` are
  forward scans that skip bracketed groups through the table. Whether a formal
  parameter is a receiver (`X<T, U> this`) is a forward scan of that parameter,
  capped at 512 tokens.
- Every other speculative lookahead (type arguments, casts, lambda heads,
  modifiers) scans at most one type, annotation or header whose nesting is
  capped at `MAX_DEPTH`, or answers balanced-bracket questions from the table,
  so the total lookahead cost is `O(tokens × MAX_DEPTH)`. A new lookahead entry
  point needs a bound of the same kind.

### Entry point

`jolt_java_syntax::parse` returns a `JavaParse`: the source, the tree and the
diagnostics. Its root is always a `CompilationUnit`. Parser snapshots use
`jolt_syntax::debug_tree`, an iterative, language-agnostic tree dump that names
kinds with `Language::kind_name`.

## `jolt_fmt`

### Entry point and the language trait

`jolt_fmt::format` formats a tree into a `Sink`: comment placement, ignore
regions, doc construction, the debug emission check, and printing, in that
order. A `ByteOrderMark` trivia piece at the start of the source is written
before everything else, and a `TrailingSubstitute` piece after the final
newline; no rule sees either. The final newline is left out after an open
literal that ends the file (Java: an unterminated text block), which would take
it in. The only options are `FormatOptions::line_width`, `indent_width` and
`use_tabs`.

A language provides `jolt_fmt::FormatLanguage`:

- `format_node`, the rule dispatch, which only `Formatter::node` calls.
- Token tables: `is_separator` (rule S1), `closer_for` and `is_closer` (rules G1
  to G3 and unclosed openers), `hugs_next` (no space before an inline comment),
  `is_open_literal`, and `joins`, which tells the printer where two texts need a
  space so that they do not lex as one (`- -x`). The tables take a token, not a
  kind, because the same kind can play two roles (Java's `<` and `>` delimit
  type arguments or are operators).
- `place_comment`, the [comment hooks](#java-comment-hooks), and `ignore_scope`,
  the [item-list owner](#item-lists-blank-lines-formatter-ignore) of a
  formatter-ignore marker.
- `DOC_LINE_COMMENT`, the opener of a doc line comment (Java: `///`), and
  `doc_comment`, the layout of a [doc comment](#doc-comments) that starts its
  output line.

### Document IR

A `Doc` is a `u32` handle into one append-only arena per format run. Text is
always borrowed (`&'a str`: source slices and `&'static` punctuation), so no
string is allocated during layout. Node size: 12 bytes; texts live in a side
table with their precomputed display width (ASCII fast path) and flags. The
arena is sized from the token count up front, so it rarely grows while a file is
formatted.

| Constructor                     | Meaning                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| ------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `text(&'a str)`                 | Literal text. A text containing a line break (a text block literal) is written raw (line breaks as `\n`) and cannot be flat.                                                                                                                                                                                                                                                                                                                                                                           |
| `sticky(&'a str)`               | A synthesized separator (`,` `;`) that is printed before a pending line suffix instead of flushing it (see printer).                                                                                                                                                                                                                                                                                                                                                                                   |
| `line_comment(&'a str, bool)`   | The last text of a run of doc line comments, for a `doc_comment` layout: the line ends after it, and a doc line comment that starts the next line gets a blank line first (see printer). The flag glues it as `glued` does.                                                                                                                                                                                                                                                                            |
| `glued(&'a str)`                | Comment text written right after the text before it, without the space `joins` would add: the `\` of a Markdown hard break after a word, or a word after a `///` frame (`///**`).                                                                                                                                                                                                                                                                                                                      |
| `reindented(&'a str, Reindent)` | Inside `jolt_fmt` only (comment printing and verbatim output). A multi-line source slice (a block comment, or verbatim source) whose continuation lines the printer rewrites while writing them: `Reindent::Comment` lays out a block comment's lines (see Printing); `Reindent::Strip(prefix)` removes `prefix` from a line that starts with it. Each continuation line is then written after the indentation of the output line the text started on; blank lines get no indentation. Cannot be flat. |
| `space()`                       | One space; dropped at line start and before a newline; consecutive spaces collapse into one.                                                                                                                                                                                                                                                                                                                                                                                                           |
| `line()`                        | Space when flat, newline when broken.                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| `soft_line()`                   | Nothing when flat, newline when broken.                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| `hard_line()`                   | Newline; forces every enclosing group to break.                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| `empty_line()`                  | Blank line (two newlines); forces enclosing groups to break.                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| `line_break(blank)`             | `empty_line()` when `blank`, else `hard_line()`: a separator that keeps or adds a blank line.                                                                                                                                                                                                                                                                                                                                                                                                          |
| `concat([Doc])`                 | Sequence.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| `group(Doc)`                    | Flat if the contents fit, else broken. Contents without a line of their own are returned unchanged (see below).                                                                                                                                                                                                                                                                                                                                                                                        |
| `hang(line, value)`             | `line` (a `line` or `soft_line`), then `value`: flat if it fits; else `value` on the next line one indent deeper if it fits there flat; else `value` broken on this line if its first line fits; else the next line (see [Hang](#hang-and-hug)).                                                                                                                                                                                                                                                       |
| `hug(hugged, broken)`           | `hugged` (a list's items with spaces for its lines, printed broken) if its first line fits up to a required line break outside its groups; else `broken`, which has a forced break (see [Hang](#hang-and-hug)).                                                                                                                                                                                                                                                                                        |
| `indent(Doc)`                   | One more indentation level for newlines inside.                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| `if_break(broken, flat)`        | Chooses by the mode of the enclosing group.                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| `line_suffix(Doc)`              | Deferred until the next newline (or a non-sticky text; see printer).                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| `break_parent()`                | Forces every enclosing group to break.                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| `will_break(Doc)` (query)       | Whether the doc contains a forced break; a rule may choose a layout from it (qualified names break every segment when a comment forces a break inside).                                                                                                                                                                                                                                                                                                                                                |
| `Doc::NIL`                      | Nothing.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |

There is no `align`: the Java style never aligns continuation lines, and all
continuation is one indent level. Newline coalescing (below) handles blank lines
and line boundaries, so there is one kind of each line command.

There is no `fill` either. The one filled text, doc comment text, is built from
the constructors above (see [Doc comment layout](#doc-comment-layout)): between
two words is a wrap point, `group(if_break(cat![soft_line, frame], line))`, one
shared doc per comment and indentation, so a word costs one text and two child
slots. The fit check of a wrap point measures the space, the next word and the
words glued to it with `space()`, and then reaches the next wrap point in the
rest of the stack, which is in break mode there, so its `soft_line` ends the
check: a constant number of commands and one word's width. A dedicated printer
kind could skip the group and `if_break` commands of each wrap point, but they
are a small part of the cost (see the cost model there).

Each node stores a `BREAKS` bit computed at construction from its children
(`hard_line`, `empty_line`, `break_parent`, multi-line text). A group whose bit
is set is broken without a fit check; this is Prettier's `propagateBreaks`, done
in `O(1)` per constructed node. A second bit, `LINES`, records whether the node
has a `line`, `soft_line`, `if_break` or `hang` outside every group inside it:
whether its layout depends on the mode of a group around it. `group` returns
contents without that bit unchanged, since a group around them would only add a
fit check. So every group has a line break of its own, which the
[hang](#hang-and-hug)'s probe relies on.

Text side-table flags: `sticky`, `line_comment` (set by the comment printer for
line comments; see the printer's "line comments end their line"), `run_end` (the
last text of a run of doc line comments, set with `line_comment` by
`DocBuilder::line_comment`; see the printer's "doc line runs stay apart"),
`glued` (no `joins` check before the text), the multi-line mode (`raw` for
`text`, the `Reindent` mode for `reindented`), and `verbatim_start` on the first
text of a verbatim node (see
[Verbatim edges](#item-lists-blank-lines-formatter-ignore)). Display width is
the sum of `unicode-width` widths of the characters, with a tab inside a text
counting 1 and an ASCII fast path. The width of a multi-line text for fitting is
its first line's; after printing it, the column is the width of its last line
(plus the indentation written before it, for `reindented`).

Ergonomics: `Formatter` derefs to `DocBuilder`. Because building a doc needs
`&mut`, `jolt_fmt` exports three macros that evaluate their arguments before
borrowing the builder: `cat!`, `group!` and `indent!`, each written
`name![f; doc, doc, …]`. These are the only macros in the formatter crates.

### Printer

The printer is iterative: a command stack of `(indent, mode, doc)`, a line
suffix buffer, and output buffered into chunks for the sink. Its line handling:

- **Newline coalescing.** A newline request (broken `line`/`soft_line`,
  `hard_line`) sets `pending = max(pending, 1)`; `empty_line` sets
  `pending = 2`. Nothing is written until the next non-space text, which first
  writes `pending` newlines and the indentation of the most recent request.
  Consequently a newline at line start is a no-op, two blank-line requests never
  stack, a newline before the first text of the file is dropped, and trailing
  whitespace is never produced (pending spaces are discarded at a newline).
- **Line suffixes.** A broken newline request with pending suffixes first prints
  the suffixes (Prettier semantics: the request is pushed back and the suffix
  contents are printed first, so a suffix may itself contain line breaks).
  Additionally, any text that is not `sticky` with pending suffixes first prints
  the suffixes and then a newline at that text's indent. This is what keeps an
  end-of-line comment from being carried past a token: `} // c` followed by
  `else` prints `} // c` / `else`. Only synthesized separators are `sticky`
  (`b // c` followed by a synthesized trailing `,` prints `b, // c`); a
  represented `,` or `;` never follows a pending suffix: placement rule S1
  attached the comment after it, or the `;` is a stray one (hook H3), which is
  removed or starts its own line.
- **Line comments end their line.** After a text flagged `line_comment`, the
  next text on the same output line (including the next flushed suffix) is
  preceded by a newline at the current indent. Two trailing line comments can
  therefore never be merged into one comment, whatever the rules produce.
- **Doc line runs stay apart.** After a line that starts with a doc line comment
  (a text that starts with `DOC_LINE_COMMENT`) and ends with a text flagged
  `run_end`, a doc line comment that starts the next line gets a blank line
  before it instead. The comment printer flags the last line of every run, and
  also a doc line comment in no run (one that ends a line of code in the
  source), which placement may put on a line of its own. Consecutive doc line
  comments are one doc comment to javadoc, so two runs that a layout puts on
  adjacent lines (a comment moved into a body by H1 above the first member's
  `///` run, or the comments of a removed `;`) would merge into one.
- **End of input.** Flush suffixes, then exactly one `\n` (also for an empty
  file).

Output reaches the sink in chunks of about 1 KiB, so a comparing sink (check
mode) stops the printer soon after the first difference.

Spaces collapse in the fit check as they do when printing (a run of `space` and
flat `line` counts one column before the next text, and none at the start of a
line).

Fit check for a group not already forced to break: Prettier's `fits` over the
group's contents in flat mode followed by the rest of the stack in its recorded
modes, with remaining width `line_width - column`. The contents of a line suffix
(an end-of-line comment) count toward the width of the line they end. It returns
`false` on width overflow and on a forced newline inside the group. When, inside
the group's contents, a non-`sticky` text is reached while a line suffix is
pending (from before the group or from inside it), that text will be preceded by
a forced newline: if a flat `line` or `soft_line` was measured in the group
before it, the group returns `false` (breaking it moves that text); otherwise
breaking cannot change where the text starts, so the check continues from a
fresh line at the text's indentation. It returns `true` at the first newline of
the rest, at a non-`sticky` text of the rest while a suffix is pending, or at
the end. It visits at most `FIT_BUDGET = 4096` commands; exhausting the budget
returns `false` (break). Cost: `O(doc nodes × FIT_BUDGET)` worst case, in
practice `O(width)` per group because the check stops at the line end.

The pending-suffix rule is what puts the value of `x = // c` and `return // c`
on the next line: the value's group sees the pending comment, breaks, and the
value continues on the next line at its own indent, where it may break further
by its own rule. If `fits` treated that text as fitting, the group would print
flat and a long value could never break. The fresh-line rule is what keeps
`a ||  // c` / `(b && c)` from breaking the parenthesized operand: its group
starts with text, so it is measured on the line the comment pushes it to.

Indentation is `indent_width` spaces per level, or one tab per level with
`use_tabs` (a tab counts `indent_width` columns for fitting).

#### Hang and hug

`hang(line, value)` is the one layout choice besides flat or broken. It lays out
the value after an operator (`x = value`, `case X -> value;`, `(Type) value`)
and the last call of a member chain when it is the only call left after the
receiver and the first call that joined the receiver's line
(`this.registry.register(…)`, `assertThat(…).isTrue()`,
`Mono.fromCallable(…).subscribeOn(…)`). In a flat context it prints flat. In a
broken context the printer picks the first layout that applies:

1. **Flat**: `line` and `value` flat, if they fit (the group fit check).
2. **Next**: `line` broken and `value` one indent deeper, if `value` fits flat
   there (a fit check from the start of that line, followed by the rest of the
   stack).
3. **Here**: `line` flat and `value` broken at the hang's indent, if the hung
   probe below fits.
4. **Next**, with `value` broken, otherwise. A pending line suffix (`x = // c`)
   selects this layout at once: the comment ends the line.

The hung probe answers whether the value's first line, printed broken after
`line`, fits and breaks in the right place. The value's outermost groups (and
hangs and hugs) are its breakable parts. The probe measures them flat, and the
groups inside them flat too, noting each part's first line break; a line break
that the value requires (a hard line, a text block, a line of a group with a
forced break) ends the line where it is. A line break qualifies only if it is
indented deeper than the base: the hang's indent, or the indent of the line the
value starts on when that line is a continuation line indented deeper (a wrapped
switch label before `->`, a wrapped array index before `=`). The probe fits
when:

- the line fits up to the first required line break, and that break qualifies;
  or
- the line is too long, a qualifying first line break of a breakable part was
  passed before the overflowing text, and the value up to that text is too wide
  for the next line as well, so that layout 4 would break the same part. The
  line then ends at the first line break of the part passed last. After the
  overflow the probe measures on flat, through the rest of that part's line
  breaks, only until the text it measured is wider than the next line; it fails
  at another breakable part, a required line break or the first line break after
  the value.

These conditions follow the printer's greedy decisions, in which a group breaks
when its contents up to the next possible line break do not fit, and accept only
the case where the part that breaks is the last one on the line. When an earlier
part would break instead (type arguments before the call's arguments, a lambda's
parameters before its body, the first call of a member chain before the chain's
own line breaks), layout 4 is chosen. A forced line break inside a part ends the
line exactly as overflowing text there would, so a comment that forces a line
break which the first pass already printed leads to the same choice on the
second pass.

Inside any fit check, a hang in a broken context is not decided again. A group's
check and the next-line check measure it as layout 4, whose line break comes
first; layout 4 is always available to the hang, and it picks layout 1 or 3 only
when that fits, so the measured line is never shorter than the printed one. The
hung probe measures a broken hang outside every breakable part as a breakable
part of its own whose first line break comes before its value, one indent
deeper, where it qualifies like any other; when a line comment is pending there,
the hang will pick layout 4, so the line ends at that line break. So measuring a
hang costs the same as measuring its children, and no fit check starts another.

**Hug.** `hug(hugged, broken)` lays out an argument list whose last argument has
a body that always breaks: `broken` is the list broken as usual (it has a forced
break), and `hugged` is `open a, b, last close` with a space for each of the
list's lines, so that the body breaks at the call's indentation. The printer
prints `hugged`, broken, if the hung probe above fits for `hugged` measured flat
from the current position (its groups flat, except those with a forced break):
the line must fit up to its first required line break, which must be indented
deeper than the hug and must not be inside a group of `hugged`. A block, class
body, switch body or text block is not inside a group, so it qualifies; a forced
break in the last argument's parameters or arguments, or in a call it is nested
in, does not, and the list breaks. The list builder offers a hug only when the
last argument is a lambda with a block body, an anonymous class, a switch
expression or a text block, no argument before it has a forced break, and the
list is recovery-free and has no dangling comments, so two lambda bodies keep
the broken list. A group's check and the next-line check measure a hug as
`broken`, which is always available. One fit check per hug.

The hung probe measures a hug in the value, outside the groups of a breakable
part, as `hugged`: a breakable part (or a place where the part it is in breaks)
whose first line break comes after its first text, `open`, where `broken` breaks
when the hug does not fit. Its body's line break is required, so the line ends
there when it fits up to it, and the hug then fits on the operator's line. When
the line is too long before the body, the probe decides as above: if the value
up to the body fits on the next line, layout 4 moves it there, and the hug's own
check fits on that line; this is how an assignment's value keeps its last
argument's body hugged (`handler =` / `someFactory(a, b, x -> {`), as an
anonymous class value keeps its header whole. A required line break that the
body does not start with (one inside a group of `hugged`, or one not indented
deeper than the hug, such as before a comment on its own line) makes the hug
fail on either line, so after an overflow it ends the line at `open` instead of
choosing layout 4.

Cost: the printer reaches each hang once and makes at most three fit checks
there, each bounded like a group's: it visits at most `FIT_BUDGET` commands and
stops at the end of the line it measures. After an overflow the hung probe
measures the rest of the part flat, but only up to the next line's width of
text, and stops at the next breakable part, a required line break or the first
line break after the value. Measuring a hug as `hugged` instead of `broken`
visits the same commands. Total: `O(doc nodes × 3 × FIT_BUDGET)` worst case,
`O(width)` per hang in practice.

### Comments

#### Classification

The placement pass walks every token's trivia once and records each comment as a
`jolt_fmt::Comment`: its text and range, its kind (line, block or doc; an
unterminated comment is a block comment), `lines_before` and `lines_after` (the
line breaks since the previous token or comment and until the next, capped at
2), and its source layout:

| Layout        | Source                                                    |
| ------------- | --------------------------------------------------------- |
| `OwnLine`     | Newline before and after (file start and end count).      |
| `StartOfLine` | Newline before, code after on the same line (block only). |
| `EndOfLine`   | Code before, newline after.                               |
| `Inline`      | Code before and after on the same line (block only).      |

Placement decides the layout the comment prints with, which may differ.

`prev` and `next` are the tokens around a comment (`next` always exists: the
`Eof` token). The **enclosing node** is the deepest node containing both,
computed on demand (hooks, debug assertions and formatter-ignore scopes use it).

The source layout is decided from line breaks alone: "newline before" means a
line break between the end of `prev` (or of the previous comment) and the
comment's start, or no `prev`; "newline after" means a line break between the
comment's end and the start of `next` (or of the next comment), or `next` is
`Eof`. A comment with code before it on its line that is followed only by other
comments up to the line's end ends its line too: `/* a */` in `x; /* a */ // b`
is `EndOfLine` like `// b`. A multi-line block comment is classified by its
start and end lines the same way. `lines_before` and `lines_after` count line
breaks in the same gaps.

#### Placement

Every comment is attached exactly once, as a `jolt_fmt::Placement`:
`Leading(element, layout)`, `Trailing(element, layout)` or `Dangling(node)`. A
language hook sees a `CommentContext`: the comment, `prev`, `next`, and the
queries `enclosing`, `is_unclosed` and `has_unclosed_child_until`.

Targets. `leading_target(next)` is the outermost node other than the root whose
first token is `next`, or the token itself when no such node exists. If that
node is a list, the target is instead its child on the path to `next` (repeated
while that child is a list too). `trailing_target(prev)` is the same with last
tokens. So a comment attaches to the outermost element that begins (or ends) at
the adjacent token, never to a list (always to its item, so a member's Javadoc
leads the member, not its `ModifierList`), and never past an adjacent token. One
exception keeps a comment next to its token in a recovered tree: the trailing
walk does not enter a node that has recovery, is missing its last piece (an
empty last slot, or a last child without tokens), and does not begin at the
token. That node's layout may put a line for the missing piece (`x =` with no
value prints `x =`, a line, nothing), and the comment, printed after the node,
would land past it and read differently on the next pass. A node that begins at
the token is entered anyway, so a comment before it (S1) never attaches inside
it. Recovery-free trees are unaffected. Comments therefore never move across a
token, except by S1 (past `,`/`;`) and hook H1 (into a body).

An opener is **unclosed** (`is_unclosed::<L>(open)`, which layouts ask too) when
no token of kind `closer_for(open)` follows it in its parent. Placement answers
this from a set of unclosed openers computed once per tree in one linear pass
(empty for a recovery-free tree).

Rules, first match wins:

1. `L::place_comment(ctx)`.
2. **G1, empty delimited region.** `prev` opens a region (`closer_for(prev)` is
   `Some`) and `next` is its matching closer (same parent, kind
   `closer_for(prev)`): `Dangling(prev.parent())`. An unclosed opener gets no
   special treatment here or in G2 and the `Inline` default: the comment falls
   through to the rules below and attaches by its neighbours, often `Eof`,
   outside the unclosed node, and prints at the outer indentation. That is
   stable (the first pass is the fixpoint) and never hides a comment inside a
   node that may be printed verbatim.
3. **G2, end-of-line after an opener.** `prev` opens a region whose closer is
   present, or only `Eof` follows, the layout is `EndOfLine`, and no other
   comment lies between `prev` and this comment:
   `Leading(leading_target(next), OwnLine)`.
4. **G3, own-line before a closer.** `next` is a closer (`is_closer`) that does
   not begin its parent (a stray `}` is a `Bogus` of its own, and a comment
   before it leads it), and the layout is `OwnLine` or `StartOfLine`:
   `Dangling(next.parent())`. So a dangling comment always lies inside its node.
5. **S1, end-of-line before a separator.** The comment is a line comment, the
   layout is `EndOfLine`, `next` is a separator (`is_separator`), `prev` is not
   one, and no other comment lies between this comment and `next`:
   `Trailing(trailing_target(next), EndOfLine)`. This is the one placement that
   moves a comment across a token (`a // c` `,` prints `a, // c`); doing it in
   placement rather than in the printer keeps the comment's `break_parent` at
   the same depth in the document on every pass, which idempotence needs. S1
   moves a comment past one separator only, and not at all when the comment
   already follows one: moving it across a run of separators would attach it to
   removed `;` items after an import or statement. A block comment stays before
   the separator (`1 /* c */;`). Closers do not pull comments past themselves:
   every layout that may break before a closer already puts a line there, so
   `a[i // c` / `]` keeps the comment before the `]`.
6. Defaults: `OwnLine`/`StartOfLine` → `Leading(leading_target(next), layout)`;
   `EndOfLine` → `Trailing(trailing_target(prev), EndOfLine)`; `Inline` →
   `Trailing(trailing_target(prev), Inline)` if `next` is a closer or a
   separator and `prev` does not open a region whose closer is present, else
   `Leading(leading_target(next), Inline)`. (`EndOfLine` and `Inline` imply a
   `prev`.) The opener exception keeps `{/* c */;}` stable: the `;` may be
   removed, and a comment trailing the `{` would then be alone in the region.
   After an unclosed opener the closer that follows belongs to an enclosing node
   and starts its own line (`@A( /* c */ }`), so the comment trails the opener,
   as it does on the next pass as an `EndOfLine` comment.

Two rules then keep the comments of one gap in order, whatever the rules above
chose:

- **Runs.** Once a comment of a gap is placed `Leading` or `Dangling`, a later
  comment of the same gap that would be `Trailing` follows it instead: the same
  target and side, with its own layout (so `Leading` can carry `EndOfLine` here:
  `/* a */ /* b */` alone on a line before code keeps both on that line).
- **After a trailing line comment.** S1 can put a comment after a trailing line
  comment of the same element (`f // a` `;` `/* b */`). Nothing can follow a
  line comment on its line, so the generic rules place such a second trailing
  comment with the layout `OwnLine` instead of its source layout, where the next
  pass finds it.

Hook constraints (checked by debug assertions in the pass): a hook may only
target elements inside the enclosing node; `Leading` layouts are `OwnLine`,
`StartOfLine` or `Inline`; `Trailing` layouts are `EndOfLine` or `Inline`.

**Who can have dangling comments.** Only G1, G3 and hook H1 produce `Dangling`,
always on the parent of an opener or closer token. So the nodes that can carry
dangling comments are exactly the kinds with a delimiter token slot: bodies and
blocks, argument, parameter, type-argument and type-parameter lists, and also
nodes that hold their own parentheses or brackets (Java: method, constructor,
record and annotation-element declarations; `if`, `while`, `do`, `for`,
`switch`, `synchronized`, `catch` headers; `Guard`; `ResourceSpecification`;
`ParenthesizedExpression`, `CastExpression`, `LambdaExpression`,
`ArrayAccessExpression`, `DimExpression`, `ArrayDimension`, `ArrayInitializer`,
`AnnotationArgumentList`, `AnnotationArrayInitializer`, `RecordPattern`). The
rule of every such kind prints `f.dangling(node)` just before its closer, either
directly, through `f.with_dangling`, or by passing the node as `owner` to
`delimited`, `enclosed`, `parenthesized`, `braced` or `braced_body`. A rule that
forgets fails the emission check on the first probe that reaches it.

**Doc line runs.** After placement, one linear pass groups doc line comments
(line comments that start with `DOC_LINE_COMMENT` and not with one more of its
last character: `///` but not `////`) into runs: comments on their own lines in
the source, one after the other in one gap with no blank line between, with the
same target, side and layout. Javadoc reads such a run as one doc comment (JEP
467). The first comment of a run, its head, prints the whole run; see
[Doc comments](#doc-comments).

Storage: one `Vec` of placed comments in source order, each record holding the
comment, the tokens before and after its gap, its target, side and effective
layout, and its place in a doc line run; attachments in a `Vec` sorted by
`(target, side, source order)`; two bitsets (`NodeId`, `TokenId`) answer "has
attached comments" in `O(1)`, so elements without comments never search.
Placement is `O(tokens + trivia + Σ_comments a(c))`, where `a(c)` is the number
of nodes that begin or end at the comment's adjacent tokens (the target walk
stops as soon as a parent starts or ends elsewhere). That is bounded by tree
depth and is large only at the head of a deep left-nested spine.

#### Printing

The formatter, not the rules, prints leading and trailing comments, from the
effective layout only:

| Attachment | Layout                              | Printed as                                                                                                          |
| ---------- | ----------------------------------- | ------------------------------------------------------------------------------------------------------------------- |
| Leading    | `OwnLine`                           | `hard_line, C, (empty_line if lines_after ≥ 2 else hard_line)`                                                      |
| Leading    | `StartOfLine`                       | `hard_line, C`, then `space` unless `next` is a closer or separator                                                 |
| Leading    | `Inline`                            | `space` unless `L::hugs_next(prev)`, then `C`, then `space` unless `next` is a closer or separator                  |
| Leading    | `EndOfLine`                         | `space` unless `L::hugs_next(prev)`, then `C, (empty_line if lines_after ≥ 2 else hard_line)` (Runs)                |
| Trailing   | `EndOfLine`, line comment           | `line_suffix(space, C), break_parent`                                                                               |
| Trailing   | `EndOfLine` block comment, `Inline` | `space` unless `L::hugs_next(prev)`, then `C`, then `space` unless `next` is a closer or separator                  |
| Dangling   | any                                 | by the owning rule via `Formatter::dangling`: one per line, except that comments that shared a source line share it |

So an inline comment gets one space on each side, none after a `hugs_next`
token, and none before a closer or separator: `this /* c */.x` prints as
`this /* c */ .x`.

A block comment that ends its line trails its code like an inline comment: it
does not force the line to break, so `foo(a, /* c */` / `b)` prints
`foo(a, /* c */ b)` when it fits. On the next pass the comment is `Inline` and
leads `b`, which prints the same text.

Because spaces collapse and are dropped at line start, these are correct
whatever separator the rule itself put at that position: `a /* c */ + b`,
`foo(/* c */ a)`, `a /* c */)`, `this /* c */ .field`.

`C` is the comment doc:

- A line comment: its text with trailing whitespace removed, flagged
  `line_comment`, and `run_end` when it is a doc line comment.
- A single-line block comment: its text.
- A multi-line block comment: `reindented(text, Reindent::Comment)`, so
  continuation lines follow the indentation of the line the comment starts on. A
  multi-line doc comment is a star block whose opener text moves to its own `*`
  line; a block comment whose continuation lines start with `*` is a star block
  that keeps its opener text. Every star block puts one space before each `*`,
  keeps each line's text after the `*` (a doc comment's whole run of `*`s, as
  javadoc reads it) and one space, and ends with `*/` on its own line. A doc
  comment with a content line that has no leading `*` is not a star block:
  javadoc keeps the indentation of such a line (a code sample in `<pre>` relies
  on it), so each continuation line only loses the indentation of the source
  line the comment starts on, when it starts with it, as a verbatim node's lines
  do. A banner (`/***`, or a line made only of two or more `*`s, which javadoc
  reads as blank) is kept as written the same way. Such comments are rare, so
  the printer lays a doc comment out as a star block and starts again as written
  only when it meets such a line; other doc comments are scanned once. Other
  multi-line block comments lose only the indentation their continuation lines
  share.
- An unterminated comment: its text, written raw.
- Any other single-line comment: its text as written.
- The head of a doc line run: the run's lines, each a line comment, one per
  line; the last is flagged `run_end`. The run's other comments print nothing
  (they still count as emitted, where their attachment is printed), and the head
  uses the last line's `lines_after`. In `removed`, when the last comment is in
  a run, its head is the comment that prints no line after itself.

See [Java style](../guides/style.md) for the resulting output.

#### Doc comments

A doc comment that starts its output line is laid out by the language's
`FormatLanguage::doc_comment` hook when it returns a doc; `None` prints the
comment as in the list above. A doc comment is a doc block comment
(`DocCommentSource::Block`, `/** … */`) or a doc line run
(`DocCommentSource::Lines`, one text per `///` line, trailing whitespace kept,
with the source tokens printed around the run), with its placement target. The
chokepoints call the hook only for a comment that starts its output line, since
only there no group's fit check measures past the line break before it into the
comment, so its layout can't change the code around it:

- a comment printed `OwnLine` (leading, trailing or dangling);
- the first dangling comment of a node (the owning rule puts a line break before
  it), and a later one whose `lines_before` is not 0 (`own_lines` puts one with
  `lines_before == 0` on the line of the comment before it);
- the last comment `removed` keeps, which leads on its own line.

The comment must also lie outside every formatter-ignore region (the Javadoc of
an ignored member is printed as written), and its target must be recovery-free
(the target node, or a token target's parent): a comment layout is a
normalization (N9), and normalizations apply only to recovery-free nodes. Rules
never see doc comments. The Java hook lays out a doc block comment with
`jolt_markup` (see [Doc comment layout](#doc-comment-layout)), except a file
header: a comment that leads an import, or the package declaration of a file
that declares a type or a module, which javac doesn't read as documentation and
which is usually a licence.

A doc line run is documentation only where it prints: JEP 467 makes a `///` run
documentation when it is the last doc comment before a declaration, and
placement and normalizations move comments (an annotation hoisted before the
modifiers takes its comments along, a removed `;` leaves its comments before the
next member, a comment before a body's `{` moves into the body). So the hook
gets the run's neighbours in the output: the source token emitted last before
it, and the first one emitted after it, unless another doc comment is emitted
first. A leading run that may be laid out is a slot (`DocBuilder::slot`) whose
layout waits for the next source token: `token` (and `verbatim`) fill it when
they emit one, a doc comment fills it with `after = None`, and so does the end
of the tree. Rules emit tokens in output order, and `node` emits a node's
leading comments before its body, so call order is output order there. Dangling
and trailing runs never wait, since a rule may emit dangling comments after its
closing token; they print as written. The Java hook lays out a run when its next
token is the first token printed of a declaration that javadoc documents (a
module, a package without types in its file, a type that is not local, or a
member): the innermost declaration around that token, which the token before the
run lies outside of. Deciding on the output's neighbours is what makes this
idempotent: the next pass reads the run where this one printed it.

Why this is idempotent: printing preserves each comment's neighbours (no comment
crosses a token, except across separators by S1, which is decided the same way
on every pass) and its newline-before/newline-after facts, except where the
layout itself put a newline next to an `Inline` or `EndOfLine` comment, or
removed one after an end-of-line block comment (which prints the same as the
`Inline` comment it becomes, and a rule that chooses a layout from leading
comments asks whether they start a line, not whether there are any). In that
case every enclosing group was already broken (by `break_parent`, a multi-line
text, or the pending-suffix fit rule), so on the next pass the comment (now
`StartOfLine` or `EndOfLine`) forces exactly the breaks that already happened
and the output is unchanged. A synthesized `sticky` separator may move ahead of
an end-of-line comment once (`b // c` + synthesized `,` → `b, // c`); on the
next pass the separator is represented and S1 attaches the comment after it,
inside the same groups, so the result is a fixpoint. These arguments are checked
by the probes test and the corpus sweep, not by proof; a failure there is fixed
in placement or printing, not with a special case in a rule.

### The `Formatter` context

`jolt_fmt::Formatter` is the context every rule receives. It derefs to the
`DocBuilder` and holds the placed comments, the ignore regions and the emission
counters. Its methods fall in three groups:

- **Chokepoints**, which emit source: `node` (and `fmt`, `fmt_opt` for typed
  nodes), `token`, `element`, `leading_comments` and `trailing_comments`,
  `removed`, `verbatim`, `dangling` and `with_dangling`. `node` prints the
  leading comments, then the node (verbatim when it is malformed, otherwise
  through `FormatLanguage::format_node`), then the trailing comments. For a node
  without comments that costs one flag read and one bitset probe.
- **Queries**, which emit nothing: `has_comments` and its leading, trailing and
  dangling forms, `leading_comments_break` and `trailing_comments_break`
  (whether a comment puts a line break there), `has_comments_within`,
  `in_ignore_region` and `blank_line_before`.
- **`item_list`**, below.

Rules emit source tokens only with `token`, nodes only with `node`/`fmt`, and
synthesized punctuation (normalizations) with `text`/`sticky`. Comments are
emitted only through the chokepoints. `leading_comments`/`trailing_comments` are
for the four rules that build a node's doc themselves: the iterative spines and
the `else if` chain ([Deep spines](#deep-spines)), the readability parentheses
of N7, and the declaration header. A rule says _what_ to format, never how its
comments print. A rule formats each element at most once; to use the same
element in two `if_break` branches, build its `Doc` once and reference the
handle twice.

### Item lists, blank lines, formatter-ignore

`item_list` is the one place that implements:

- **Separators.** Between consecutive formatted items it calls `separator`; the
  language decides `hard_line` vs `empty_line` from the kinds and
  `blank_line_in_source`. Between two items that are both inside one ignore
  region, and between the last item of a region and the item after it,
  `item_list` does not call `separator` and uses the source's choice
  (`empty_line` iff `blank_line_in_source`), so a region prints as written.
  Nothing is emitted before the first item or after the last, so bodies never
  start or end with a blank line.
- **Elision.** An `Elide` item prints `removed(item)`; if that is NIL the item
  vanishes (no separator), otherwise its comments stand in its place. The last
  comment of an elided item, when it is an own-line leading comment, is not
  followed by its blank line itself; instead a blank line after it in the source
  counts as a blank line before the next item
  (`Separator::blank_line_in_source`), so every body keeps the source's blank
  line there.
- **Formatter-ignore.** An item (`Format` or `Elide`) whose first token's text
  starts inside an ignore region prints `verbatim(item)` between its own leading
  and trailing comments (including the markers), which are formatted normally.
  Such an item is never elided; this check comes before elision. Each item in a
  region starts its own line and keeps the source's separation from the next
  (blank line or not).

Ignore regions are computed once after placement. A marker is a comment whose
body, after stripping `//` or `/* */` and surrounding whitespace, is exactly
`@formatter:off` or `@formatter:on`. A marker's **list** is the innermost
item-list owner (Java: compilation unit, type body, block, constructor body,
switch block statement group, module body) whose range contains the marker while
no item of that list contains it, i.e. the marker sits between items, or before
the first or after the last item; a marker inside an item's interior (say, in an
argument list) is in no list. Scanning markers in source order, an `off` that is
in a list opens a region at its end; the next `on` closes it at its start (later
`off`s inside a region are ignored). The region is kept only if that `on` is in
the same list; otherwise it is dropped and has no effect. Pairing markers within
one list means a region always covers whole items of that list, so verbatim
output and the separators around it are well defined; an `off`/`on` pair that
crosses a body boundary would cover part of a body. An `off` with no later `on`
at all ends at the start of its list's closing token, or at end of file at top
level. Kept regions are sorted and disjoint; membership is a binary search,
`O(log regions)` per item, and finding a marker's list walks its ancestors once.
Nested lists inside a verbatim item are never visited; nested lists inside a
formatted item apply the same test to their own items.

Malformed nodes use the same `verbatim`: `node()` on a malformed node prints
`verbatim(node)` between the node's leading and trailing comments.

**Verbatim edges.** A verbatim node prints the comments inside its text and
marks them emitted; attachment printing skips a comment verbatim already
printed, which covers a comment inside the slice that S1 attached to an element
outside the node (`return x // c` `;` with a verbatim `return`). Placement
attaches nearly every comment outside the node's text to the node, an ancestor,
or an element outside it, so the normal leading and trailing path prints it:
every target rule picks the outermost element that begins or ends at an adjacent
token, which is the verbatim node or an ancestor when the token is the node's
first or last; G1 needs the closer, so its region lies inside one node; G3 needs
the closer not to begin its node, so a dangling comment lies inside its node;
and H1 requires the body it targets not to be malformed. The remaining cases
attach a comment outside the text to an element inside the node: H4 attaches an
end-of-line comment to an unbraced control body, which may end a
formatter-ignore item; the trailing walk's recovery exception can stop below a
node that ends at the last token; and S1 can attach a comment before the node to
an element that begins it. So `verbatim` walks the node's descendants and prints
every comment attached to one of them that it has not printed: before the slice
when the comment starts before the node, after it otherwise, each formatted in
its attachment's layout, which is what formatting the element would have printed
at that point. Every verbatim piece of one node re-indents against the
indentation of the output line the node starts on (recorded at its first text),
also after a text block. So nothing is lost or duplicated at the boundary.

### Emission check

`Formatter` keeps one emission counter per comment in every build. After the
document is built, `format` asserts every counter is exactly 1 when
`cfg!(debug_assertions)` is true, panicking with each offending comment's text
and position. Release builds skip the check but keep the same layout. Tokens
need no tracking: the oracle's structure fingerprint covers them. The count is
taken at construction, so it catches duplicates and comments never reached, but
not a doc that was built and then dropped by a rule; the oracle's comment
multiset catches that.

## `jolt_java_fmt`

### Module layout and ownership

```text
crates/jolt_java_fmt/
  src/lib.rs                 pub fn format, JavaFormat, type JavaFormatter
  src/language.rs            impl FormatLanguage: separator and delimiter tables, ignore scopes, hooks H1 to H5
  src/dispatch.rs            total match: node kind → rule function
  src/helpers/lists.rs       delimited comma lists, one-element delimited nodes, parenthesized headers, list items
  src/helpers/mod.rs         `spaced`, `spaced_node`: a space before what follows, if anything does; `after_token`: the value after a keyword or arrow
  src/helpers/bodies.rs      braced bodies, body items with N5, member and statement separators
  src/helpers/assignment.rs  the value after an operator: `left = right`, a switch rule's `-> value;`, a cast's operand
  src/rules/declarations/    compilation unit, imports, modules, type declarations and headers, bodies, members
  src/rules/statements/      blocks, control flow, jumps, switch, try
  src/rules/expressions/     operators, binary chains, calls, object creation, lambdas, the receiver spine
  src/rules/types/           types, generics, names, annotations, modifiers, variables, patterns
```

insta derives snapshot names from the test file names (`corpus__…`,
`recovery_snapshots__…`), so renaming a test file renames all its snapshots.

`JavaFormat` implements `FormatLanguage`, and `JavaFormatter<'a>` is
`Formatter<'a, JavaFormat>`. Every rule function takes the formatter and its
typed node and returns a `Doc`
(`fn class_declaration<'a>(f: &mut JavaFormatter<'a>, node: ClassDeclaration<'a>) -> Doc`).
It is named after the kind in `snake_case` and re-exported from its module's
`mod.rs` (`declarations`, `statements`, `expressions`, `types`). `dispatch.rs`
maps each kind to a small function that calls exactly these names (a table of
function pointers rather than one large `match` frame, so the depth tests' 128
nesting levels fit the 1 MiB stack in debug builds), so the files inside a
module can be reorganized freely as long as the entry points keep their names
and signatures. `Bogus` has no rule: it is always `MALFORMED`, so `node()`
prints it verbatim before dispatch, and its dispatch arm is `unreachable!`.

List kinds never carry comments (placement attaches to their items) and are
never malformed (factory rule 3), so a parent may iterate a list's elements
directly. A list kind therefore has a rule function only when some other rule
formats it as a unit with `f.fmt`; those are exactly the lists in the
[layout contracts](#layout-contracts-between-modules) (`ModifierList`,
`ParameterModifierList`, `VariableDeclaratorList`, `ArrayDimensions`,
`AnnotationList`). Every other list kind is laid out by its parent's rule. The
dispatch's fallback arm sends it to `helpers::lists::unexpected_list`, which
`debug_assert!`s and formats the children separated by spaces; the arm asserts
`is_list()`, so a new list kind needs no dispatch edit. A token, or a node kind
that has no rule, panics there (`dispatch.rs`).

### Java comment hooks

`JavaFormat::place_comment` implements only these exceptions to the generic
rules. "Header end" below means the `)` closing the header of `if`, `while`,
`for` (basic or enhanced), `switch` (statement or expression), `synchronized`,
`catch`, or the resource specification of a `try`, or the keyword `else` or
`do`, and "the body" is the statement that follows it.

- **H1, comment between a header and its body.** Two cases, both
  `Leading(t, OwnLine)` into the body, where `t` is `leading_target` of the
  first token inside the body block, or `Dangling(body)` when the block is
  empty:
  - `next` is the `{` of a body that follows a header in the same parent (a
    type, method, constructor, lambda, switch-rule or initializer body, an
    anonymous class or enum constant body, a switch block, a control statement
    body block), the body is not malformed, no opener in the header before
    `prev` is unclosed (recovery), and the comment is not `Inline` (an inline
    comment leads the `{`: `class X /* c */ {`). A doc comment (`/**`) before
    the `{` of a type body that has members leads the `{` inline instead:
    inside, it would document the first member. An end-of-line line comment
    before a separator that the `{` follows (a trailing separator, recovery)
    goes in too, since S1 would move it past the separator. A `{` that starts
    its parent (a nested block, `case 1: {`) or a labeled statement's body has
    no header before it, so H1 does not apply.
  - `prev` is a header end, `next` is the first token of a non-block body, and
    the comment is an `EndOfLine` comment or an `OwnLine`/`StartOfLine` line
    comment: `t` is the body statement itself (brace promotion will wrap it).
    Exception: after `else` whose body is an `if` (an `else if` chain), H1 does
    not apply and the generic rules give `} else // c` / `if (…)`.

  This moves the comment across the body's `{`, for declarations and control
  statements alike, so a comment about the body starts it. It is stable because
  on the next pass the comment is own-line inside the body. All comments between
  a header and its body go the same way, so their order is kept.
- **H2, type-argument angles.** `<`/`>` of `TypeArgumentList` and
  `TypeParameterList` are openers/closers (via `closer_for`/`is_closer`); `<`
  and `>` as operators are not.
- **H3, end-of-line comment before a stray `;`.** The comment is `EndOfLine`,
  `next` is the `;` of an `EmptyStatement` or `EmptyDeclaration`, and `prev` is
  not an opener (G2 applies there):
  `Trailing(trailing_target(prev), EndOfLine)`. Without H3, S1 would move the
  comment onto the `;`, so `} // c` followed by `;` would print the comment on
  the next line: on a line of its own when N5 removes the `;`, after it when the
  `;` stays (recovery, or a switch group's only `;`). With H3 the comment stays
  on the `}` line, and a `;` that stays starts its own line.
- **H4, end-of-line comment after a promoted body.** The comment is `EndOfLine`
  and `prev` is the last token of a non-block body of an `if`, `else`, `while`,
  `for` or enhanced `for` (not an `else if` chain): `Trailing(body, EndOfLine)`.
  The generic rules would make the comment trail the whole statement and print
  after the synthesized `}` of brace promotion; H4 keeps `if (a) return; // c`
  on the line of `return;` inside the new block.
- **H5, block comment at the end of a line after `(`, `[`, `<` or an assignment
  operator, or before a header clause.** A block comment with layout `EndOfLine`
  right after one of these openers (not before its closer, where G1 applies) or
  after `=`, `+=`, … is `Leading(leading_target(next), Inline)`: it stays inline
  before the next token (`run( /* d */` / `a)` prints `run(/* d */ a)`), which
  is where the next pass, reading it as inline, puts it. After `=` it is part of
  the value, so a long declaration breaks the same way on both passes. G2 still
  moves it to its own line after `{`. H5 needs what follows to be inside: the
  opener's closer is present, or the operator's node extends past the comment.
  Otherwise (`x = /* c */` / `}` with the value missing) `next` belongs to an
  enclosing node, and the comment trails the operator by the generic rules, as
  it does when it is `Inline` (`x = /* c */;`). Before the first clause of a
  declaration header (`extends`, `implements`, `permits`, `throws`), the comment
  leads the clause inline for the same reason: the head (up to its `)` or `>`)
  and the clauses are separate groups, so a comment trailing the head would
  count toward the head's width on one pass and the clauses' on the next.

Anything else that looks like it needs a hook should first be checked against
the generic rules; a new hook requires a fixture and an entry here.

### Helpers

The shared layouts live in `helpers`, so each shape is implemented once:

- `lists::delimited`: a comma-separated list between delimiters with the
  break-all layout, in one of three `ListLayout`s (parameters and type
  arguments, call arguments with the [hug](#hang-and-hug) of a last argument
  with a body, and initializers with N8's trailing comma).
- `lists::enclosed`: a node with one inner element between delimiters (array
  access, dimension expressions, casts), with no line breaks of its own.
- `lists::parenthesized`: `(inner)` that breaks like a call (statement headers,
  parenthesized expressions, the resource specification).
- `lists::joined` and `lists::list_item`: a list's elements in order, a bogus
  item through `node`.
- `bodies::braced_body`, `bodies::braced`, `bodies::body_items` and the member
  and statement separators: every braced body, so an empty body prints `{}` with
  no per-kind exception, and N5.
- `assignment::assignment_like`, `operator_value` and `value_after`: the value
  after an operator.
- `after_token`: the value after a keyword or arrow.

### Layout contracts between modules

What a node's `Doc` contains when a rule in another module formats it with
`f.fmt`. Every doc excludes surrounding spaces unless stated. The owning module
is in parentheses.

| Kind (owner)                                                   | Doc contract                                                                                                                                                                                                                                                                                                                                                                                                                            |
| -------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `ModifierList`, `ParameterModifierList` (types)                | Annotations and keywords **including** the trailing separator: `hard_line` after declaration annotations (those before the first keyword), `space` after the last keyword/type-use annotation. NIL when empty. Via `f.fmt` this is the declaration layout, used for fields, methods, types and local variables.                                                                                                                         |
| `types::inline_modifiers(f, ParameterModifierList)` (types)    | The inline layout: every annotation and keyword on one line, separated by spaces, followed by `space`; NIL when empty. A plain `pub(crate)` function (lists carry no comments, so no `f.fmt` is needed), used for `FormalParameter` (types), `RecordComponent` (declarations), `CatchParameter` (statements), type patterns, enhanced-for variables and resources (`void m(@Nonnull String p)`, `catch (@Nonnull final Exception ex)`). |
| `AnnotationList` (types)                                       | Type-use annotations inline: separated by spaces, followed by `space`; NIL when empty. `types::annotation_lines(f, Option<AnnotationList>)` lays out package, module and enum-constant annotations one per line.                                                                                                                                                                                                                        |
| `ArrayDimensions` (types)                                      | `[]` per dimension, an annotated one preceded by its annotations and a space (`@A []`); no space before an unannotated `[`.                                                                                                                                                                                                                                                                                                             |
| `Name`, `QualifiedName` (types)                                | The dotted name on one line; never breaks.                                                                                                                                                                                                                                                                                                                                                                                              |
| `Block` (statements)                                           | `{ … }` from `braced_body` with `statement_separator`; caller adds the space before.                                                                                                                                                                                                                                                                                                                                                    |
| `ClassBody`, `InterfaceBody`, `EnumBody`, … (declarations)     | `{ … }`; `ClassBody` is used by expressions for anonymous classes.                                                                                                                                                                                                                                                                                                                                                                      |
| `ArgumentList` (expressions)                                   | `( … )` via `delimited`; used by declarations (enum constants, explicit constructor invocations).                                                                                                                                                                                                                                                                                                                                       |
| `FormalParameter`, `ReceiverParameter` (types)                 | One parameter. The parens and break-all list belong to the declaration rule, which passes `parameters().elements()` to `delimited`.                                                                                                                                                                                                                                                                                                     |
| `TypeParameterList`, `TypeArgumentList` (types)                | `< … >` group; used by declarations and expressions.                                                                                                                                                                                                                                                                                                                                                                                    |
| `VariableDeclaratorList` (types)                               | `name = init, …` with `assignment_like`; continuation declarators on new lines at one indent when broken.                                                                                                                                                                                                                                                                                                                               |
| `Expression` category (expressions), `SwitchExpression` (stmt) | A complete expression; never includes a trailing `;`.                                                                                                                                                                                                                                                                                                                                                                                   |
| Patterns (types)                                               | Used by expressions (`instanceof`) and statements (case labels).                                                                                                                                                                                                                                                                                                                                                                        |
| Statements (statements)                                        | Include their `;`. Bodies of control statements are formatted by the statement rules only.                                                                                                                                                                                                                                                                                                                                              |

A rule never works around another module's output; when it needs a different
shape, change the contract here and the owning rule together.

Comment-driven layout rules owned by one module:

- **The value after a token (helpers).** `helpers::after_token` prints the value
  after `return`, `throw`, `yield`, `break`, `continue`, `assert`, a lambda's
  `->`, a switch rule's `->` before a `throw` or block, and the `&` or `|`
  between types: `token space value tail` normally. When a trailing comment of
  the token ends its line (`return // c` / `y + 1;`), it prints
  `token group(indent(line, value tail))` instead: the pending-suffix fit rule
  breaks the group, so the value continues one indent deeper and may break
  further by its own rule. When a leading comment of the value breaks the line,
  it prints `token space indent(value tail)`, so the comment and the value go
  one indent deeper.
- **The value after an operator (helpers).** `assignment::value_after` prints
  `op hang(line, value)`. For an operator chain or a conditional, and when a
  trailing comment of the operator ends its line (`x = // c`), it prints
  `op group(indent(line, value))` instead, so that a chain's operands start one
  level deeper than the operator's line, and, after a comment, as `after_token`
  does after a keyword: with a block comment, which is not a line suffix, the
  hang could keep the value on the comment's line, and the next pass would read
  the comment as an inline one.
- **A node after a space (helpers).** `spaced_node` prints `space node`. When a
  leading comment of the node breaks the line (an array initializer's `{`, a
  guard, a bound or a `default` value after a comment that starts its line), it
  prints `space indent(node)`, so the comment and the node continue the line
  before one indent deeper instead of starting a line at the enclosing indent,
  where they would read as the next statement or member. A `finally` clause is
  not a continuation, so `try` uses `spaced`, and the clause starts its line at
  the statement's indent like `}` / `// c` / `finally {`.
- **Promoted empty bodies (statements).** A non-block body that is an
  `EmptyStatement` (`if (a);`) promotes to an empty block: the `;` is elided
  through `removed`, exactly as an `EmptyStatement` inside a block is, so the
  first output is the fixpoint.

### Normalizations

Normalizations are the closed list below. Each applies only when the owning node
`is_recovery_free()`; for N5 in a body or switch group, that is the body or
group, so a body with an error keeps its stray `;`s (an unclosed body that
dropped one would move its comments out of the body on the next pass). Removed
tokens print their comments through `removed`; N6 never removes parentheses that
carry a comment. The structure fingerprint (`STRUCTURE_POLICY` in
`jolt_java_fmt/tests/common`) ignores exactly these changes.

| #   | Normalization                                  | Where                                                                                                                                                                             | Extra condition                                                                                                                                                                                                                                                                                                                                                                     |
| --- | ---------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| N1  | Import grouping and sorting                    | Imports of a compilation unit                                                                                                                                                     | Per import: recovery-free, not in an ignore region, name without interior comments; any other import is a barrier that stays in place and splits the section. An import with leading comments is an anchor: it starts a new sorted run and stays first in it. The comments of a stray `;` between imports lead the import after it, which then anchors. Duplicate imports are kept. |
| N2  | Modifier ordering                              | Declaration `ModifierList`                                                                                                                                                        | Keyword runs between comment-carrying or bogus modifiers sort independently.                                                                                                                                                                                                                                                                                                        |
| N3  | Module directive and `requires` modifier order | `ModuleDeclaration`                                                                                                                                                               | A directive with leading comments, recovery or in an ignore region is a barrier: it stays in place on its own, and the directives on each side sort separately. `requires` modifiers sort only in a recovery-free directive without comments on them.                                                                                                                               |
| N4  | Brace promotion                                | Non-block bodies of `if`, `else`, `while`, `do`, `for`, enhanced `for`                                                                                                            | Body is not malformed.                                                                                                                                                                                                                                                                                                                                                              |
| N5  | Redundant `;` removal                          | `EmptyStatement` in blocks and switch groups and as a promoted body, `EmptyDeclaration` in compilation units and type bodies, trailing `;` of try resources, enum body separators | Without members, an enum's `;` is removed. With members, the `;` moves up to the last constant past a removed trailing comma, unless the comma or the `;` carries a comment: then both stay as written.                                                                                                                                                                             |
| N6  | Redundant parenthesis removal                  | Parenthesized left operand of a same-precedence binary chain; switch guard `when (x)`; single untyped lambda parameter `(x) ->`                                                   | No comment on or inside the parentheses: `has_comments_within` is false for the parenthesized expression, and for a guard or lambda the parentheses carry no comments, it has no dangling comments, and `has_comments_within` is false for the condition or parameter.                                                                                                              |
| N7  | Readability parentheses                        | Binary operands of `&`, `\|`, `^`, `<<`, `>>`, `>>>` that are themselves binary expressions                                                                                       | —                                                                                                                                                                                                                                                                                                                                                                                   |
| N8  | Trailing commas                                | Array and annotation array initializers (iff broken); the last enum constant of a body without members                                                                            | —                                                                                                                                                                                                                                                                                                                                                                                   |
| N9  | Comment text                                   | Trailing whitespace, re-indentation, star alignment; the whitespace and line breaks of a doc comment that starts its line (`jolt_markup`)                                         | Words, block tags and verbatim regions unchanged, as the oracle compares them; a doc comment with a line without `*`, or a banner, keeps its lines as written.                                                                                                                                                                                                                      |
| N10 | Whitespace                                     | Line endings become `\n`; one final newline; BOM and trailing SUB preserved                                                                                                       | —                                                                                                                                                                                                                                                                                                                                                                                   |

[Java style](../guides/style.md) shows the output of each.

### Deep spines

The parser builds left-deep binary/`instanceof` chains and receiver chains
iteratively, so their depth is unbounded. The receiver spine is every kind the
postfix loop wraps around its receiver: `MethodInvocationExpression` with
`QualifiedMethodInvocation`, `FieldAccessExpression`, `ArrayAccessExpression`,
`PostfixExpression`, `MethodReferenceExpression`, `ClassLiteralExpression`, and
the qualified forms of `ObjectCreationExpression`, `ThisExpression` and
`SuperExpression`. Their rules (`rules/expressions/spine.rs` and `binary.rs`) do
not recurse through `f.node` along the spine: they walk the spine with an
explicit loop, format the leaves with `f.node`, and emit each interior spine
node's comments with `leading_comments` (before the head) and
`trailing_comments` (after the segment the node ends with), its dangling
comments in its segment. Parentheses removed by N6 carry no comments. A
malformed spine node stops the walk and is formatted with `f.node`. The
`else if` chain is walked the same way by `if_statement`. Every other recursion
is bounded by the parser's `MAX_DEPTH`. `tests/deep.rs` formats a 20 000-long
`else if` chain, a 20 000-term `+` chain, a 20 000-call chain, a 20 000-deep
`a[0][0]…`, a 20 000-deep `a.new B().new B()…` and similar receiver chains on a
256 KiB thread, and the deepest accepted nesting of each recursive construct on
a 1 MiB thread (the WASM stack).

## Doc comment layout

The crate `jolt_markup` lays out a doc comment for the language's `doc_comment`
hook. It depends only on `jolt_fmt`, has no other runtime dependency, and builds
for wasm32. One entry point per flavour (`javadoc` for `/** … */`, `markdown`
for a `///` run) returns the comment's doc (`javadoc` returns `None` to print it
as written). The Javadoc (HTML) and Markdown flavours share every stage but the
body parser:

1. **Frame** (`frame`). The comment's content lines, borrowed from its text:
   each line loses its leading whitespace, its run of `*`s and one space, as
   javadoc reads it, and its trailing whitespace; each keeps its source slice
   from the `*` on, for printing it as written. The frame returns `None` for the
   comments N9 keeps as written: `/***` banners and `/**/`, a line of only `*`s,
   and a content line without a leading `*`. A `///` run's lines lose `///` and
   the indentation that all its lines with text share, as JEP 467 reads them;
   that prefix is the run's frame, so `///CLOVER:OFF` keeps having no space.
   Each line remembers whether it ended in two spaces (a hard break).
2. **Tags** (`tags`). Inline tags (`{@…}`) are matched first, by brace balance,
   in one pass with a stack over every brace; nothing inside one is a block tag,
   a block element or a verbatim region (javadoc renders a line-leading `@Bean`
   in a multi-line `{@code` as code). A block tag is a line that starts with `@`
   and a letter outside inline tags and verbatim regions. The word rules both
   flavours share live here too: list markers, rulers, and the words that must
   not start a line.
3. **Body parser.** `html` for Javadoc and `markdown` for Markdown. The parser
   walks the lines once and calls the layout with lines (a new line at an
   indentation, after a blank line or not), words (joined to the word before by
   a wrap point, a space or nothing) and verbatim lines.
4. **Layout** (`layout`). Builds the docs (below) and, for Javadoc, chooses the
   one-line form.

The HTML parser recognizes block-level elements (`p`, `ul`, `ol`, `li`, `dl`,
`dt`, `dd`, `pre`, `table`, `blockquote`, `h1`–`h6`, `br`) anywhere outside
inline tags, without regard to case, and keeps a stack of open blocks: block tag
text (indent 4), lists (their items at 2 more) and list items (their other lines
at 4 more). A block tag closes every open block. `<pre>`, `<table>`,
`<blockquote>`, `{@snippet}` and a multi-line HTML comment are verbatim regions:
the first line from the opener starts a block (except the HTML comment, which
may be glued to the words around it), and the lines after it are printed as
written, the last up to the closer; an unclosed region runs to the comment's
last line of text. Other HTML tags, entities and words are atoms; an inline tag
with spaces inside (`<a href="…">`) is one atom when it ends on its line.

Line structure the source gives some text is kept: a line that starts with a
list marker keeps the line break before it, a ruler keeps its own line, and a
unit of plain lines (between blank lines, block tags and block elements) whose
indentation differs by two columns or more is printed as written. A word that
would start a block tag or a list at the start of a line is glued to the word
before it, so a wrap never creates structure that the next pass would read
differently. These are the rules that make the layout idempotent: every line
break that depends on the source's lines is one the output reproduces.

The Markdown parser splits the run at block tags as the oracle does (a line that
starts with `@` and a letter outside a fence) and reads each part with
CommonMark's block algorithm: a stack of open block quotes and list items
(matched at the start of each line, a list item by its content column, with tabs
at stops of 4), the open leaf block (paragraph, fence, indented code, HTML block
of type 1 to 7 with its end condition, table), lazy continuation lines (which
belong to the open item or quote), and which blocks may interrupt a paragraph.
Only a paragraph outside every container is filled: the description's and the
text of block tags, with continuation lines flush at the frame, since an
indented line can be code or list text. Every other line, blank lines included,
is a verbatim line from its `///`. One line of lookahead keeps a paragraph as
written when the line after it makes it a heading (a setext underline) or a
table header (a delimiter row, read loosely), and a paragraph that may start
with a link reference definition or holds a `{@snippet}` is kept too. Only one
blank line is added, before the first block tag: Markdown blank lines change the
page (a blank line in a list item makes the list loose).

Inside a paragraph, atoms keep their words together: code spans (a backtick run
is closed by the next run of the same length, found through a table of the next
run of each length, so the matching is linear), links (`[…]`, `[…](…)`,
`[…][…]`, from bracket and parenthesis pairs matched with stacks outside code
spans), and raw HTML and autolinks (`<…>`, scanned by CommonMark's tag grammar,
whose quoted attribute values may hold `>` and line breaks; a failed scan
resumes where it failed, and a closer that is not found is never searched for
again, so each byte is scanned once). An atom that spans lines keeps its line
breaks, since a line break and a space differ inside raw HTML and a link title.
`{@…}` tags glue their reference to their name, as in Javadoc. Emphasis is not
parsed: a space and a line break are the same to it. A source line that starts
with a word that could start a block keeps its line break, unless it is indented
as code (four columns or more), where its word joins the line before; a line
that must start a line in the output and has such a word (after a hard break, or
inside an atom) keeps its indentation. A two-space hard break becomes a `\`
glued to its word (`DocBuilder::glued`, since Java's `joins` would put a space
before a `\` after a word), and a line that ends in an odd run of `\` keeps its
line break. The layout never wraps after a word that ends in `\`.

The docs:

| Piece         | Doc                                                                                                                        | Fit-check cost                                                                  |
| ------------- | -------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------- |
| Wrap point    | `group(if_break(cat![soft_line, frame], line))`, one shared doc per indentation                                            | The space, the next word and the words glued to it, up to the next wrap point.  |
| Glued word    | `space()` before it                                                                                                        | None.                                                                           |
| Line start    | `hard_line`, then the frame: `" * "` and the indentation's spaces, one `'static` text, or the run's `///` frame (borrowed) | None (forced).                                                                  |
| Blank line    | `hard_line`, `" *"` or `///`                                                                                               | None. Each blank line has its own text, so newline coalescing can't merge them. |
| Verbatim      | `hard_line`, `" "`, the source line from its `*`; or the source line from its `///`                                        | None.                                                                           |
| One-line form | `group(["/**", if_break(soft_line + frame, " "), words, if_break(soft_line + " */", " */")])`                              | Up to the line width; only for a comment that is one line of words.             |

The printer writes the command's indentation after a newline, then the frame, so
a wrap happens at the comment's real column. A Markdown run has no delimiters:
its first line starts with the frame or a verbatim line, and its last text is a
`DocBuilder::line_comment`, which ends the line after the run. A verbatim line
never takes the frame's list or tag indentation, which would change code in a
`<pre>` inside a list item, and is one text from its `*`, so the printer's join
check can't add a space inside it (`*` before `=` joins in Java). A comment that
starts its output line follows a forced line break, which ends every fit check
that reaches it, so its layout can't change the code around it.

Cost model. Building is one pass over each line and character; each word is one
text node and two child slots. Measured on Spring (9,122 files, 37,030 `/**`
comments, 1.33M words) with `--threads 1` and a full write, against the build
without doc comment layout (about 20.0G instructions): the frame and the inline
tag matching add about 0.27G, parsing 0.41G, building the docs 0.58G, and
printing the filled text 0.58G more than printing the comments as written, about
9% in all (`perf stat -e instructions`; see the PR for wall times).

## `jolt_formatter` (facade)

The CLI and the dprint plugin use only this API and `jolt_config`.

`jolt_formatter::format` takes the source, a `Language` (detected from the file
extension by `Language::from_path`), the options, an `OnSyntaxError` policy and
a sink. With `Refuse`, a source with syntax errors gets `Outcome::Refused` with
the errors, and nothing is written; otherwise the result is
`Outcome::Formatted { halted }`, where `halted` means the sink stopped the
printer early. Feature `serde` forwards to `jolt_fmt/serde`.

`check` mode is a sink that compares chunks against the source and breaks at the
first difference (`jolt_cli`'s `Compare`); after `Formatted { halted: false }`
the CLI also checks that the whole source was consumed (the output may be a
strict prefix of the source).

## Verification

### Oracle

For an input `S` and output `F = format(S)`:

1. **Reparse.** If `S` parses without diagnostics, `F` does too. Otherwise the
   multiset of diagnostic codes of `F` equals that of `S`.
2. **Structure.** For clean `S`, the structure fingerprint of `F` equals that of
   `S` modulo N1–N8: imports, module directives, requires modifiers and
   declaration modifier keywords compared as multisets; `EmptyStatement`,
   `EmptyDeclaration`, `ParenthesizedExpression`, promoted `Block`s and
   `BlockStatementList`/`BlockStatement` wrappers transparent; `, ; { } ( )`
   ignored (`STRUCTURE_POLICY`).
3. **Comments.** The multiset of comments (kind and canonical text) of `F`
   equals that of `S`. The canonical text of a line or block comment drops blank
   lines and trailing whitespace, strips star prefixes and one space after them,
   and removes the indentation all continuation lines share, so relative
   indentation inside a comment is compared. Doc comments are compared by what
   javadoc reads in them (`jolt_test_support`'s `doc_comments`), so that a
   layout may rewrap their text but not change it:
   - A `/**` comment: the whitespace-separated words of the description, with
     block-level HTML tags (`<p>`, `<li>`, `</ul>`, …) split off as words of
     their own; each block tag (a line starting with `@` and a letter) with its
     words; and the verbatim regions (`<pre>`, `<table>`, `{@snippet}`) as exact
     lines. Each line of a multi-line comment first loses its run of leading
     `*`s and one space after them, as javadoc reads it. Inline tags (`{@…}`,
     matched by brace balance) are lexed first, so nothing inside one starts a
     block tag, a block element or a verbatim region. A `/**` comment with a
     content line that has no leading `*` is compared line by line, each line
     without the indentation of the source line the comment starts on.
   - A run of `///` lines (consecutive lines that start with `///` but not
     `////`, with no blank line between, grouped from the trivia alone, as JEP
     467 reads them): the run loses `///`, the indentation its lines share and
     trailing whitespace, is split at block tags outside fenced code, and each
     part is compared by its pulldown-cmark events (tables on). Adjacent texts
     are joined and their whitespace runs collapse; code is compared exactly. A
     line break, hard or soft, is a space: Jolt never writes trailing
     whitespace, so a two-space hard break can only survive in another spelling.
     A `///` comment after code on its line is no doc comment to javadoc; it is
     a run of its own, so a layout may move it to a line of its own (placement
     does, after another trailing line comment) but not into a run.
     pulldown-cmark is a dependency of `jolt_test_support` only, which every
     crate uses as a dev-dependency, so it never reaches the CLI or the plugin.
4. **Idempotence.** `format(F) == F`.
5. **Tokens.** For dirty `S` (whose recovered tree the structure check cannot
   compare), the multiset of token texts of `F` equals that of `S`, ignoring
   `, ; { } ( )` and counting the `>` family by character.

### Tests

`mise run test` runs these; the full probe sweep over the imported corpora runs
only in `mise run hunt`, because it is too slow for the normal suite.

- `jolt_syntax/tests/parser.rs`: the loop guard, the depth limit, iterative tree
  building and separated-list slots, with a toy language whose grammar is
  deliberately broken.
- `jolt_java_syntax/tests/parser_fixtures.rs`: `debug_tree` and diagnostics
  snapshots for `fixtures/java/syntax/**`; every fixture and every imported
  upstream input parses losslessly; token soups, deep nestings and long
  unbalanced runs terminate and stay lossless.
- `jolt_java_syntax/tests/codegen/`: generated code is current.
- `jolt_java_fmt/tests/corpus.rs`: every fixture under `fixtures/java/` except
  `syntax/lexer/` and `syntax/recovery/` (so including `syntax/parser/`,
  `style/`, `trivia/`, `properties/` and `upstream-docs/`) is formatted, checked
  with the oracle at widths 40, 80 and 120, and snapshotted at 80
  (`corpus__<dir>__<file>.snap`). A fixture expected to carry diagnostics gets
  the oracle without a snapshot.
- `jolt_java_fmt/tests/recovery_snapshots.rs`:
  `fixtures/java/syntax/recovery/**` with the oracle at width 80 and
  input/output/diagnostics snapshots. These pin how code with syntax errors
  formats; review their diffs for loss, duplication and stability.
- `jolt_java_fmt/tests/corpus_fixtures.rs`: the oracle at width 80 over the
  imported upstream fixture suites
  (`tools/import/.imports/{google-java-format,palantir-java-format,prettier-java}/input`)
  and the CommonMark spec examples (`commonmark-spec/input`, below). This is the
  only imported-corpus check `mise run test` runs.
- `jolt_java_fmt/tests/probes.rs`: every comment probe shape at every token
  boundary of the fixtures under `fixtures/java/style/comments/` and
  `fixtures/java/trivia/`, plus a list of small constructs, with the full oracle
  at width 80. It is sized to run quickly in a debug build; the sweep covers
  everything else.
- `jolt_java_fmt/tests/deep.rs`: the stack-depth tests (see
  [Deep spines](#deep-spines)). A fit-budget test lives in `jolt_fmt`
  (`printer.rs`).
- `jolt_java_fmt/tests/style_guide.rs`: formats every `` ```java `` example in
  [Java style](../guides/style.md). Each block formats to itself, and a block
  titled `[Input]` formats to the `[Output]` block after it. A block that parses
  as a method body is formatted as one, with the width widened by the method's
  indentation.
- `jolt_cli/tests/cli.rs` and `jolt_fmt_dprint/tests/dprint.rs`: the CLI and the
  plugin end to end.

### Corpus tools

- `mise run hunt` (`examples/jhunt.rs`): the sweep over the repository fixtures
  and `tools/import/.imports` with the oracle at widths 40, 80 and 120 and the
  comment probe shapes (`PROBE_SHAPES`): an inline block comment, an end-of-line
  and an own-line line comment, an own-line block comment, an own-line
  `/** … */` with about 100 characters of words (so a layout that fills doc
  comments wraps it at every position and width), and an own-line `///` line.
  Every boundary at every width for the fixtures, the gjf, palantir and prettier
  inputs up to 4 KiB and the CommonMark examples; sampled boundaries at width 80
  for larger files, Spring and the OpenJDK slice. It also deletes each token of
  those small files in turn (the oracle's token check catches code loss), which
  stresses parser recovery and the formatting of recovered trees. Reports land
  in `target/hunt/`. `KNOWN_OPEN` must be empty.
- `examples/jcheck.rs`: replays one file or one variant (`File.java:ownline12`,
  `File.java:comment3@40`, `File.java:delete7`).
- `examples/jref.rs`: `jref <dir>` checks `format(F) == F` for every `.java`
  file (or the `formatted:` section of every `.snap`) under `<dir>`;
  `jref --input <in> --expected <out> [--line-width N]` checks
  `format(in) == out` file by file. It prints totals and writes each mismatch's
  actual output and a summary to `target/jref/`. Input files with syntax errors
  are skipped and counted separately, since the CLI refuses them and leaves them
  untouched. Inputs without an expected output are counted as missing. It exits
  1 when an output differs or is missing.
- `tools/baseline.sh build [revision]` builds the CLI at a revision (default
  `origin/main`) and formats the imported corpora with it into
  `tools/import/.imports/.baseline/`: `input/`, `output/` (width 80),
  `output-w100/` (Spring at width 100), the binary `jolt`, and a copy of Spring
  for timing in `bench/spring-framework`. `tools/baseline.sh compare <binary>`
  (with an optional output suffix) formats the same inputs with another build
  and counts the files that differ per corpus; `jref --input … --expected …`
  shows them file by file. Review those differences before changing output, and
  describe them in the PR. `import.py` keeps dot-named entries of `.imports`
  when it re-imports.
- `tools/import/import.py` imports the corpora at pinned commits: the gjf,
  palantir and prettier-java test inputs, Spring, an OpenJDK slice, and the
  CommonMark spec examples. The OpenJDK slice is the files listed in
  `tools/import/openjdk-files.txt`, fetched with a shallow sparse checkout: the
  JDK sources with `///` doc comments, and a sample with `{@snippet}`,
  `<!-- -->` and `/**` lines without a leading `*`. The CommonMark examples are
  generated at import time, each as a `///` comment on a method, because the
  spec's licence (CC-BY-SA) asks for attribution and share-alike; examples a
  `///` comment can't carry unchanged (a line starting with `/`, or `\u`) are
  left out.

### Performance

A change must not make formatting Spring slower than the baseline build:
`jolt format --no-config --check` over
`tools/import/.imports/.baseline/bench/spring-framework`, timed for the new
build against `.baseline/jolt`, with default threads and with `--threads 1`
(`hyperfine -N -i --warmup 3 --runs 10 '<jolt> format --no-config --check bench/spring-framework'`).
Measure both back to back on a quiet machine. Separately, `mise run bench` times
the release CLI and plugin, and with `--compare` writes the numbers the docs
show.

What keeps formatting fast: no per-token comment work without a bitset hit;
borrowed text only; 12-byte doc nodes with precomputed widths and break bits;
cursors that compare and hash by tree and id; early sink halt in check mode; no
debug-only work in release; and a profile-guided release build (`tools/pgo.py`).
Parsing and document construction take most of the time; document construction
(many small allocations in the rules) is the likeliest place for a further
speedup, together with reusing the token, node and doc vectors across files on a
thread.
