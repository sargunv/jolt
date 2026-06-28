# Formatter Milestone 8: Java Layout Builder

## Current Status

Milestone 7 is complete in this checkout. The shared document IR and renderer
live in `crates/jolt_fmt_ir` and already provide the primitives needed for a
first Java layout builder:

- opaque `Doc` values with constructor-driven invariants,
- groups, labelled groups, forced groups, and marked-break fit constraints,
- soft, hard, empty, and flat-text line variants,
- indentation, alignment, `if_break`, and `indent_if_break`,
- `fill`, `best_fitting`, `line_suffix`, `line_suffix_boundary`, and
  `break_parent`,
- renderer validation and Java-shaped IR tests for calls, chains, annotations,
  comments, class bodies, lambdas, and text blocks.

The Java syntax side is the completed input layer for this milestone:

- `crates/jolt_java_syntax` has a lossless lexer/parser and typed CST wrappers,
- wrapper families cover modules, declarations, statements, expressions, types,
  patterns, names, variable initializers, and body members,
- wrapper accessors expose the main grammar roles needed by a layout builder,
- parser fixture tests validate the pinned google-java-format and Palantir input
  corpora without silent fixture skips.

The likely syntax-adjacent work in milestone 8 is formatter-facing API polish,
not completing the Java syntax layer: adding narrow accessors when a layout rule
needs a specific grammar role, exposing token/trivia facts conveniently, or
building a comment cursor from the existing lexer trivia.

The first Java formatter slice now exists:

- `crates/jolt_java_fmt` formats supported Java CST shapes through the shared
  document IR,
- `crates/jolt_fmt_core::format_source` routes Java inputs through that
  formatter while Kotlin still blocks with `format.unimplemented`,
- the current implementation covers the initial declaration, block, statement,
  expression, and trivia/comment layers needed to start whole-corpus oracle
  measurement,
- unsupported clean Java syntax blocks with formatter diagnostics instead of
  emitting partial output,
- imports, modifier ordering, range formatting, suppression comments, and Kotlin
  layout remain outside this milestone.

The current google-java-format oracle scoreboard is:

- total considered: 209,
- invalid upstream fixtures skipped: 1,
- parse blocked: 0,
- missing-rule blocked: 187,
- other blocked: 0,
- formatted: 21,
- exact matches: 9,
- aggregate diff size: 665.

The next useful step is therefore not more parser or IR design. It is the Java
CST-to-doc layout layer, built horizontally over the completed syntax layer and
wired through the existing core API.

## Reference Audit

Four reference projects were audited for their layout-builder architecture.

### Prettier

Primary sources:

- https://github.com/prettier/prettier/blob/main/src/document/public.d.ts
- https://github.com/prettier/prettier/tree/main/src/document/builders
- https://github.com/prettier/prettier/blob/main/src/document/printer/printer.js
- https://github.com/prettier/prettier/blob/main/src/main/comments/print.js

Findings:

- The document algebra stays small. Strings and arrays are direct leaves and
  concat; commands represent groups, lines, indentation, fill, conditional
  content, suffixes, and break propagation.
- Groups are selected by explicit flat-vs-break measurement. Group ids record a
  chosen mode for later `ifBreak`; they are not a general constraint solver.
- `fill` is a real printer primitive for text-like packing.
- `lineSuffix` is the normal way to place trailing comments. Comment attachment
  lives outside the renderer.
- Conditional groups are treated as an escape hatch because nested alternatives
  can become expensive.

Jolt takeaways:

- Keep the shared IR language-neutral and opaque.
- Keep `best_fitting` explicit and rare.
- Use `line_suffix` for Java end-of-line comments instead of manually appending
  comment text after code docs.
- Preserve the current split between `hard_line`, non-propagating hard lines,
  and explicit `break_parent`.

### Ruff

Primary sources:

- https://github.com/astral-sh/ruff/blob/0dae927d53c0c9d8b12dadfe594494f59713ef5f/crates/ruff_formatter/src/lib.rs
- https://github.com/astral-sh/ruff/blob/0dae927d53c0c9d8b12dadfe594494f59713ef5f/crates/ruff_formatter/src/builders.rs
- https://github.com/astral-sh/ruff/blob/0dae927d53c0c9d8b12dadfe594494f59713ef5f/crates/ruff_formatter/src/printer/mod.rs
- https://github.com/astral-sh/ruff/tree/0dae927d53c0c9d8b12dadfe594494f59713ef5f/crates/ruff_python_formatter/src/comments

Findings:

- Ruff has a generic formatter crate plus language-specific formatting rules. A
  context carries options, source text, comments, and profile state.
- Formatting is trait/rule based. Rules write into a formatter buffer through a
  small builder API.
- Comment handling is centralized around leading, dangling, and trailing
  buckets. Debug/test paths assert that comments were consumed.
- Trailing comments are emitted as line suffixes and participate in width
  measurement.
- Source positions are optional but designed into the print path for range
  formatting and stable output slicing.

Jolt takeaways:

- Add a Java formatting context and Java formatting rules above `jolt_fmt_ir`.
- Make comment accounting part of the first design, with staged implementation
  ordered by comment class rather than deferred as a post-layout cleanup.
- Do not add range formatting in this milestone, but avoid API choices that make
  source markers or stable formatted ranges hard later.

### Oxc

Primary sources:

- https://github.com/oxc-project/oxc/blob/eedf4c7360160cf491307a5ec3881f1ca32280d7/crates/oxc_formatter_core/src/format_element/mod.rs
- https://github.com/oxc-project/oxc/blob/eedf4c7360160cf491307a5ec3881f1ca32280d7/crates/oxc_formatter_core/src/builders.rs
- https://github.com/oxc-project/oxc/blob/eedf4c7360160cf491307a5ec3881f1ca32280d7/crates/oxc_formatter_core/src/printer/mod.rs
- https://github.com/oxc-project/oxc/blob/eedf4c7360160cf491307a5ec3881f1ca32280d7/crates/oxc_formatter/src/formatter/comments.rs
- https://github.com/oxc-project/oxc/blob/eedf4c7360160cf491307a5ec3881f1ca32280d7/crates/oxc_formatter/src/formatter/trivia.rs

Findings:

- Oxc uses a reusable core with language-specific layers for JavaScript,
  TypeScript, and JSON.
- Builder functions return small formatting adapters; callers compose them with
  `format_args!` and `write!`.
- Generated glue handles leading/trailing comment hooks and then delegates real
  layout policy to hand-written node formatters.
- Comments are a span-ordered source fact, not vectors copied onto every node.
  The formatter owns contextual placement.

Jolt takeaways:

- Prefer small helper functions and rule objects over a large fluent layout
  builder.
- Generated formatting glue may be useful later, but milestone 8 should keep
  Java layout hand-written until the formatter's use of the wrapper API
  stabilizes.
- Build a span-ordered comment cursor from Java token trivia instead of storing
  comment vectors on every CST wrapper.

### google-java-format

Primary sources:

- https://github.com/google/google-java-format/blob/fb9528917c524c8eb9c8c0d7b4bcd7ce3b6a604b/core/src/main/java/com/google/googlejavaformat/java/Formatter.java
- https://github.com/google/google-java-format/blob/fb9528917c524c8eb9c8c0d7b4bcd7ce3b6a604b/core/src/main/java/com/google/googlejavaformat/OpsBuilder.java
- https://github.com/google/google-java-format/blob/fb9528917c524c8eb9c8c0d7b4bcd7ce3b6a604b/core/src/main/java/com/google/googlejavaformat/Doc.java
- https://github.com/google/google-java-format/blob/fb9528917c524c8eb9c8c0d7b4bcd7ce3b6a604b/core/src/main/java/com/google/googlejavaformat/java/JavaInputAstVisitor.java
- https://github.com/google/google-java-format/blob/fb9528917c524c8eb9c8c0d7b4bcd7ce3b6a604b/core/src/main/java/com/google/googlejavaformat/java/JavaCommentsHelper.java
- https://github.com/google/google-java-format/blob/fb9528917c524c8eb9c8c0d7b4bcd7ce3b6a604b/core/src/main/java/com/google/googlejavaformat/java/ImportOrderer.java
- https://github.com/google/google-java-format/blob/fb9528917c524c8eb9c8c0d7b4bcd7ce3b6a604b/core/src/main/java/com/google/googlejavaformat/java/ModifierOrderer.java

Findings:

- google-java-format lowers javac AST nodes into a linear op stream, builds a
  nested document, computes breaks, then writes output.
- Its core IR is small: levels, tokens, non-token comments, spaces, and breaks.
  Java policy lives in visitor helpers.
- `OpsBuilder` does more than convenience composition: it validates nesting,
  syncs tokens, records partial-format boundaries, inserts optional tokens, and
  splices comments.
- Java declarations use conditional break tags and conditional indentation
  heavily, especially around return types, names, parameters, throws clauses,
  variable initializers, and field annotations.
- Method chains are a specialized lowering problem. The formatter flattens
  member selects, invocations, and array accesses, classifies prefixes, and
  emits dot breaks with chain-specific behavior.
- Imports and modifier ordering are separate rewrite passes, not normal layout
  docs.

Jolt takeaways:

- Keep Java policy out of `jolt_fmt_ir`.
- Build reusable Java helpers early: declaration headers, comma-separated lists,
  braced bodies, annotation/modifier layout, method chains, switch groups, and
  comment placement.
- Treat imports and modifier ordering as later source-rewrite passes with
  separate validation.
- Use Rust types and constructor invariants rather than google-java-format's
  mutable builder phase state.

## Milestone Goal

Milestone 8 should produce the first useful Java formatter implementation:

```text
source text
  -> jolt_java_syntax::parse_compilation_unit
  -> Java layout builder
  -> jolt_fmt_ir::Doc
  -> jolt_fmt_ir::render
  -> formatted source text
```

The deliverable is the Google-profile Java layout layer wired through
`jolt_fmt_core::format_source`, with coverage organized by formatter layer:
compilation units, declarations, statements, expressions/lists, comments, and
then whole-corpus oracle comparison.

This milestone should optimize for completing the Java formatting layer in the
same order the repo has used so far: one layer at a time, with tests at each
boundary and oracle comparison as the integration check.

## Non-Goals

- Do not redesign the shared document IR unless the Java layout layer exposes a
  real missing primitive.
- Do not implement Kotlin layout.
- Do not implement import sorting or unused-import removal.
- Do not implement modifier reordering as part of normal doc layout.
- Do not add formatter suppression comments.
- Do not add range formatting.
- Do not add arbitrary formatter options beyond the existing profile model.
- Do not add convenience APIs that only wrap existing helpers without carrying
  Java layout policy.

## Architecture

### Java Formatter Context

Add a Java-specific context in `crates/jolt_java_fmt`.

Conceptual shape:

```rust
pub struct JavaFormatContext<'src> {
    source: &'src str,
    profile: JavaProfile,
    comments: JavaCommentCursor<'src>,
    groups: GroupIdAllocator,
    markers: BreakMarkerAllocator,
}
```

The context should own Java formatter state, not shared renderer state. It may
allocate group ids and break markers, expose profile predicates, and provide
comment lookup/marking. It should not own filesystem, CLI, dprint, or oracle
concerns.

### Formatting Rules

Use hand-written rules for milestone 8. Keep the contract direct:

```rust
trait FormatJava {
    fn fmt(&self, ctx: &mut JavaFormatContext<'_>) -> Doc;
}
```

This is a planning shape, not a mandate for exact public API. The important
contract is:

- CST wrappers are the normal input,
- rules return `Doc`,
- Java helpers live in `jolt_java_fmt`, not in `jolt_fmt_ir`,
- raw syntax traversal is allowed only where wrappers do not yet expose the
  needed grammar role or token boundary.

### Helper Layer

Build helpers that carry Java policy. Avoid generic helper aliases that just
hide `concat` or `group`.

Initial helpers:

- `space_separated`: declaration and modifier fragments where Java requires one
  space in flat mode,
- `comma_separated`: argument, parameter, type parameter, record component, and
  enum constant lists,
- `parenthesized`: `(`, optional inner group, `)`,
- `braced_block`: `{`, members/statements, blank lines, `}`,
- `declaration_header`: modifiers, type parameters, type/name, parameters,
  throws, and optional body,
- `modifier_list`: annotations and keyword modifiers in source order for this
  milestone,
- `argument_list`: grouped call arguments with optional multiline trailing comma
  behavior controlled by profile only if required by oracle evidence,
- `method_chain`: dot-chain analyzer and marked-break layout,
- `trailing_comment`: end-of-line comments via `line_suffix`,
- `leading_comments`: own-line comments before declarations/statements.

Helpers should express policy names. For example, prefer
`method_chain(receiver, selectors)` over exposing raw marked-break group setup
at every call site.

### Comments

Build a formatter-side comment model from Java token trivia.

Inputs already exist:

- lexer `Token` has `leading` and `trailing` trivia with source ranges,
- trivia kinds distinguish whitespace, newline, line comment, block comment,
  Javadoc comment, and ignored input,
- green tokens preserve raw trivia text for lossless syntax reconstruction.

Milestone 8 comment policy:

- classify comments into source-ordered records with range, kind, leading or
  trailing origin, and newline adjacency,
- consume comments through the Java formatter context,
- use `line_suffix` for trailing end-of-line comments,
- print Javadocs and own-line block comments as leading comments before the
  following declaration or statement,
- implement dangling comments in a staged order, starting with empty class
  bodies and empty blocks because they force the ownership model earliest,
- add tests that fail if a supported-formatting path leaves a comment
  unconsumed.

Do not pre-attach `Vec<Comment>` to every CST wrapper. The Oxc-style cursor is a
better starting point for Java because ownership depends on token gaps,
modifiers, annotations, empty containers, and declaration context.

### Error And Refusal Contract

Formatting must be non-destructive.

- If Java parsing aborts or reports syntax-affecting diagnostics, return
  `FormatStatus::Blocked` and forward diagnostics.
- During development, if a clean Java file reaches a layout rule that has not
  landed yet, return `FormatStatus::Blocked` with a formatter diagnostic naming
  the first missing layout rule. This is an implementation staging guard, not a
  milestone completion criterion.
- Every supported node rule must account for the node's direct structural shape
  before emitting output. If a clean node has extra direct children, contextual
  tokens, or grammar variants that the rule does not handle yet, the formatter
  must block instead of reconstructing a partial doc.
- If comment accounting fails, return blocked in release code and panic/assert
  in focused tests.
- Never fall back to source-text passthrough while reporting success.

This matches the existing plan's parse-error policy and avoids a formatter that
silently rewrites only parts of a file while the layout layer is being built.

## Implementation Plan

### Step 1: Formatter Crate Skeleton

Files:

- `crates/jolt_java_fmt/src/lib.rs`
- `crates/jolt_fmt_core/src/lib.rs`
- `crates/jolt_java_fmt/Cargo.toml`
- `crates/jolt_fmt_core/Cargo.toml`

Work:

- Replace the placeholder Java formatter item with a real `format_java_source`
  entry point.
- Parse source with `parse_compilation_unit`.
- Block on non-clean syntax outcomes.
- Lower clean syntax to `Doc`.
- Render with profile-derived `RenderOptions`.
- Wire `Language::Java` in `jolt_fmt_core::format_source` to the Java formatter.
- Keep Kotlin blocked with the existing unimplemented diagnostic.

Acceptance:

- A minimal `class A {}` input formats successfully.
- Invalid Java still blocks without formatted output.
- Kotlin behavior is unchanged.

### Step 2: Rule And Helper Infrastructure

Files:

- `crates/jolt_java_fmt/src/context.rs`
- `crates/jolt_java_fmt/src/rules.rs`
- `crates/jolt_java_fmt/src/helpers.rs`

Work:

- Add `JavaFormatContext`.
- Add group-id and break-marker allocation.
- Add `FormatJava` or equivalent internal rule trait.
- Add helpers for tokens, joining, lists, parentheses, blocks, and staged
  missing-rule diagnostics.
- Keep helper APIs internal until real call sites prove their shape.

Acceptance:

- Rules compose by returning `Doc`; no rule writes directly to a `String`.
- Missing layout rules produce a deterministic formatter diagnostic while the
  layer is under construction.

### Step 3: Compilation Units And Declarations

Layer scope:

- ordinary compilation units,
- package declarations,
- import declarations preserving source order,
- class declarations,
- record declarations,
- modifier lists with annotations preserved in source order,
- fields,
- methods,
- constructors,
- empty and non-empty class bodies.

Work:

- Format top-level sections with profile-compatible blank lines.
- Preserve import order in this milestone. Do not sort imports yet.
- Format declaration headers with grouped parameters and throws clauses.
- Format class bodies with one member per logical section and blank lines where
  required by source/profile evidence.
- Add direct-shape accounting for every declaration/member rule as it lands, so
  unsupported clean syntax cannot be silently dropped.

Acceptance:

- Focused snapshot tests cover package/import/class/member layouts.
- The oracle scoreboard shows the corpus-level impact of the declaration layer:
  more exact matches and smaller aggregate diffs.

### Step 4: Statements And Blocks

Layer scope:

- blocks,
- local variable declarations,
- expression statements,
- `return`, `throw`, and `yield`,
- `if`/`else`,
- `while`, `do`, basic `for`, enhanced `for`,
- `try`/`catch`/`finally`.

Work:

- Build braced block layout once and reuse it for methods, constructors,
  lambdas, class initializers, and statement bodies.
- Keep dangling comments in empty blocks supported.
- Preserve the parser's statement structure; do not reconstruct statements from
  raw token strings.

Acceptance:

- Focused snapshot tests cover blocks and control flow.
- The oracle scoreboard shows the corpus-level impact of statement formatting:
  more exact matches and smaller aggregate diffs.
- Missing statement rules are treated as development blockers for this layer,
  not accepted final behavior.

### Step 5: Expressions And Lists

Layer scope:

- names and literals,
- parenthesized expressions,
- assignment, conditional, binary, unary, postfix, and cast expressions,
- method invocations,
- object creation with and without anonymous class bodies,
- array access, array creation, and array initializers,
- lambdas with expression and block bodies.

Work:

- Build precedence-aware expression formatting from CST wrappers.
- Add comma-separated argument and parameter builders.
- Add method-chain analysis as a named Java helper using marked breaks and
  indentation.
- Reserve `best_fitting` for cases that normal grouping cannot express.

Acceptance:

- Focused snapshot tests cover method calls, nested calls, lambdas, binary
  expressions, assignments, and chained calls.
- Tests include narrow width cases to force multiline behavior.
- The oracle scoreboard shows the corpus-level impact of expression formatting:
  more exact matches and smaller aggregate diffs.

### Step 6: Comments

Work:

- Build `JavaCommentCursor` from token trivia.
- Add leading Javadoc/block/line comment formatting before declarations and
  statements.
- Add trailing line comments via `line_suffix`.
- Add dangling comments for empty class bodies and empty blocks.
- Add comment accounting assertions in tests.

Acceptance:

- Focused snapshot tests cover each landed comment class.
- A test fails if any formatted file leaves a comment unformatted.
- Trailing comments affect group fitting through `line_suffix`.
- The oracle scoreboard shows the corpus-level impact of comment formatting:
  more exact matches and smaller aggregate diffs.

### Step 7: Oracle Harness For Formatting

Files:

- `crates/jolt_java_fmt/tests/...`
- existing `.oracles/fixtures/...`
- `tools/oracles/formatter-harness/...` only if expected-output generation needs
  a small extension.

Work:

- Use the pinned oracle inputs already imported under `.oracles/fixtures`.
- Materialize or call oracle expected outputs for the pinned fixture corpora.
- Run the whole pinned corpus for the active profile on every oracle pass.
- Report aggregate progress as a stable textual summary suitable for an `insta`
  snapshot:
  - total files considered,
  - invalid upstream fixtures skipped because the input is not valid Java,
  - files blocked by parse diagnostics,
  - files blocked by in-progress missing layout rules,
  - files that formatted,
  - exact matches,
  - exact-match percentage, where higher is better,
  - mismatching formatted files,
  - aggregate diff size as aligned added/deleted line count, where lower is
    better,
  - largest per-file diff, to identify the worst remaining policy mismatch.
- Keep the snapshot compact and deterministic. It should pin counts,
  percentages, and the worst few file paths by diff size. Full per-file diffs
  should be emitted as normal test diagnostics or optional artifacts, not
  embedded in the snapshot.
- Write detailed per-file diagnostics to a gitignored report directory near the
  oracle fixtures, currently `.oracles/reports/java/google-java-format/google/`.
  A useful layout is one file per mismatching input with the formatted output
  and diff, plus an index sorted by descending diff size. Keeping the artifacts
  next to the imported fixtures makes input, expected output, actual output, and
  diff easy to inspect without pinning large diffs in snapshots.
- Fail on missing fixture directories, missing expected outputs, and fixture
  count drift.

Acceptance:

- `mise run test` runs the focused Java formatter tests.
- The oracle harness always runs the full pinned corpus for the active profile
  and prints the same scoreboard fields.
- The scoreboard is asserted with `insta` so progress and regressions are easy
  to review.
- Missing oracle fixtures fail tests, preserving the existing invariant.

## Fixture Strategy

The oracle fixtures are not grouped by formatter layer, so the harness should
not pretend they are. It should run the entire pinned corpus and report a stable
scoreboard after every layer lands.

The useful signals are directional:

- exact-match percentage should go up,
- missing-rule blockers should go down to zero,
- aggregate diff size should go down,
- largest per-file diff should point to the next high-impact policy mismatch.

Focused unit and snapshot tests are still layer-local. Oracle fixtures are the
whole-corpus integration signal.

## API Boundaries

Public or semi-public by the end of the milestone:

- `jolt_fmt_core::format_source` formats Java through the completed layout
  layer,
- Java profile selection flows from `FormatOptions`,
- diagnostics distinguish parse failures, missing in-progress layout rules, and
  render failures.

Internal for now:

- Java rule trait,
- helper functions,
- comment cursor,
- method-chain analyzer,
- group and marker allocators.

Do not expose these until multiple language builders or external call sites need
them.

## Validation

Default validation for milestone work:

```bash
mise run test
cargo test -p jolt_java_fmt
cargo test -p jolt_fmt_core
cargo test -p jolt_fmt_ir
```

Use narrower commands while developing, but the milestone should end with the
full repo test task passing. If formatting or lint hooks rewrite docs, run:

```bash
mise run fix
```

## Risks

### Formatter-Facing Accessor Gaps

The Java syntax layer is the formatter input. The likely gaps are
formatter-facing conveniences: some layout rules may need additional wrapper
accessors or token/trivia methods for annotations, dimensions, type arguments,
switch labels, try resources, and method chains.

Mitigation: add narrow wrapper accessors only when a layout rule needs them.
Treat those as API polish discovered by layout work, not as a parser milestone.
Do not fall back to broad raw traversal as the normal formatter style.

### Comment Ownership

Java comments are more position-sensitive than a simple leading/trailing split.
Javadocs, inline block comments, end-of-line comments, empty-body comments, and
modifier/annotation comments need different placement rules.

Mitigation: build comment accounting immediately and land comment classes in
layers. Comment positions not yet implemented should be tracked as in-progress
layout work, not accepted final behavior.

### google-java-format Compatibility

google-java-format uses javac AST quirks and token recovery in several Java
policy hotspots. Jolt's CST is better for lossless formatting, but it means the
layout builder must make some choices directly instead of inheriting javac's
tree shape.

Mitigation: compare against oracle output fixture by fixture. Use the CST as the
source of truth and add Java-specific helpers where oracle behavior requires
them.

### Scope Creep

Imports, modifier reordering, suppression comments, and range formatting are all
tempting because they sit near layout.

Mitigation: keep them out of milestone 8. Imports and modifier ordering should
be separate source-rewrite passes with their own validation.

## Completion Criteria

Milestone 8 is complete when:

- `jolt_fmt_core::format_source` can format Java source through the completed
  Java layout layer,
- in-progress missing-rule diagnostics have been driven down to zero for the
  pinned valid google-java-format corpus,
- parse diagnostics block formatting without output,
- Java layout rules use `jolt_fmt_ir` rather than direct string rendering,
- comment accounting exists for formatted Java files,
- the oracle harness reports 100% exact matches for the pinned valid
  google-java-format corpus,
- targeted unit tests cover forced multiline declaration, call, chain, block,
  lambda, and trailing-comment layouts,
- `mise run test` passes.
