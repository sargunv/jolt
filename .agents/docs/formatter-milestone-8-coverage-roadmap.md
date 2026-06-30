# Formatter Abstraction Layer Improvement Plan

The Java formatter routes parser-clean syntax accepted by `jolt_java_syntax`
through real layout rules. The remaining work is not a different formatter
architecture. It is stronger Java-specific layout abstractions so rule modules
describe syntax roles and the helper layer owns formatting policy, without
turning every oracle mismatch into a local case patch.

This document replaces the old coverage-blocker inventory. Its focus is the
abstraction layer between Java CST wrappers and the shared document IR.

## North Star

Milestone 8 is complete when the pinned oracle corpora reach **100% exact
match**, profile by profile, in this order:

1. **Google** — `google-java-format` profile on the pinned Google corpus
2. **AOSP** — AOSP profile on the same corpus
3. **Palantir** — Palantir profile on the pinned Palantir corpus

Work on a later profile should not regress an earlier one. Idempotence holds for
passing fixtures. Parser-clean syntax formats through real rules with no
missing-layout exits, raw source passthrough, or silent comment drops.
`mise run test` passes.

## Current Status

### Architecture

```text
source text
  -> jolt_java_syntax parser
  -> lossless CST + wrapper accessors
  -> Java rule modules
  -> Java layout helpers and analyzers
  -> shared document IR
  -> shared renderer
```

Layering in code:

```text
shared IR, builders, and renderer
  -> Java formatter context, profile policy, comments, and trivia services
  -> Java node rules with an explicit formatting contract
  -> Java analyzers and layout helpers
```

### Coverage baseline

Parser-clean fixtures format end-to-end with zero missing-rule blockers across
all three oracle profiles. Raw source passthrough, unsupported-layout exits, and
late remaining-comment appendage are gone.

Oracle alignment is the active gap. Current scoreboards (from pinned snapshots):

| Profile  | Exact match | Aggregate diff | Worst fixture          |
| -------- | ----------- | -------------- | ---------------------- |
| Google   | 34.1%       | 2,621          | `B20128760.java` (95)  |
| AOSP     | 34.1%       | 2,883          | `B24909927.java` (205) |
| Palantir | 27.7%       | 4,886          | `B24909927.java` (916) |

Selector chains dominate the largest per-file diffs. `B24909927.java` and
`B20701054.java` remain in the top mismatches on all three profiles. Palantir
aggregate diff is roughly twice Google's.

### What is in place

- **Rule contract** documented in `rules/mod.rs`: identify slots, collect
  comments, delegate to helpers, reject unhandled trivia, return a real `Doc`.
- **Profile policy** in `policy.rs` (`JavaFormatPolicy`): continuation
  indentation, AOSP static-import separation, selector-chain breaking rules.
  Direct `JavaFormatProfile` checks are confined to policy, options, and tests.
- **Generic list mechanics** in `helpers/separated.rs`.
- **Java list helpers** in `helpers/lists.rs`: argument, formal, type
  argument/parameter, keyword-prefixed clause lists, with comment-aware item
  formatting.
- **Declaration helpers** in `helpers/callables.rs` and
  `helpers/type_declarations.rs`, adopted from `declarations.rs`.
- **Body helpers** in `helpers/bodies.rs`: statement blocks, constructor bodies,
  class/interface/enum bodies.
- **Selector chain analyzer and helper** in `analyzers/chains.rs` and
  `helpers/chains.rs`: flattening, metadata, `ChainGroup`s, staged layout
  alternatives, profile-aware breaking.
- **Comment ownership** in `context.rs` and `comments.rs`: leading, trailing,
  dangling, inline, separator, and list-item buckets; `reject_unhandled_*`
  guards; end-of-format unconsumed-trivia diagnostics.
- **Domain rule split** under `rules/`: compilation units, declarations,
  expressions, statements, annotations, types, names, tokens.
- **Oracle scoreboards** for Google, AOSP, and Palantir profiles.
- **Focused unit tests** in `rules/tests.rs` for lists, chains, declarations,
  comments, and narrow-width wrapping.

### Active pressure points

- `declarations.rs`, `expressions.rs`, and `statements.rs` are still large.
  Helpers exist, but rules still assemble many slots and own comment wiring
  locally.
- `layout.rs` still holds expression-shaped helpers (`assignment_expression`,
  etc.) that belong in a future `helpers/expressions.rs`.
- Comment placement is incomplete in annotation arguments, inline annotation
  positions, some header-boundary positions, and a few branch/else shapes. These
  currently block formatting rather than placing comments.
- Selector chain policy infrastructure exists, but oracle alignment for long
  fluent chains—especially under Palantir—is still the largest shared mismatch
  domain.
- Import section policy lives in `rules/compilation_unit.rs` rather than a
  dedicated helper module.

## Reference Project Lessons

Jolt should copy formatter **boundaries** from the projects below, not their
exact APIs.

Emulate:

- a small language-neutral document algebra,
- a language-specific syntax-to-document layer,
- formatter context services for comments, source positions, and options,
- named helpers for lists, chains, declarations, expressions, and bodies,
- source-aware comment classification before rendering,
- analyzer objects for complex syntax shapes such as selector chains.

Do not emulate:

- AST-only source models,
- AST mutation for comment attachment,
- JavaScript/Python-specific naming heuristics without Java oracle evidence,
- plugin API shapes that do not fit Jolt,
- arbitrary style configuration,
- ignored-source passthrough as a coverage mechanism.

Oracle compatibility targets are **google-java-format** and
**palantir-java-format**. Prettier, Oxc, and Ruff are architectural references
for IR shape, comment ownership, and rule layering.

### google-java-format

Primary oracle for the Google and AOSP profiles.

- [Formatting pipeline (`Formatter.java`)](https://github.com/google/google-java-format/blob/master/core/src/main/java/com/google/googlejavaformat/java/Formatter.java)
  — visitor → `OpsBuilder` → `DocBuilder` → break selection → output; the
  end-to-end shape Jolt mirrors with CST rules and a shared renderer.
- [Document IR (`Doc.java`)](https://github.com/google/google-java-format/blob/master/core/src/main/java/com/google/googlejavaformat/Doc.java)
  — Oppen-style document tree and width-driven break selection; reference for
  `jolt_fmt_ir::Doc`.
- [Op stream builder (`OpsBuilder.java`)](https://github.com/google/google-java-format/blob/master/core/src/main/java/com/google/googlejavaformat/OpsBuilder.java)
  — imperative layout emission before tree lowering; analogue to rule modules
  building `Doc` via helpers.
- [Ops-to-doc lowering (`DocBuilder.java`)](https://github.com/google/google-java-format/blob/master/core/src/main/java/com/google/googlejavaformat/DocBuilder.java)
  — converts flat ops into nested levels; two-phase emit-then-structure split.
- [Java layout visitor (`JavaInputAstVisitor.java`)](https://github.com/google/google-java-format/blob/master/core/src/main/java/com/google/googlejavaformat/java/JavaInputAstVisitor.java)
  — monolithic per-node policy; reference for what Jolt splits across `rules/`
  and `helpers/`.
- [Method chains (`visitDot`)](https://github.com/google/google-java-format/blob/master/core/src/main/java/com/google/googlejavaformat/java/JavaInputAstVisitor.java#L2998)
  — flattens selector syntax and chooses break-before-dot layout; primary oracle
  for `analyzers/chains.rs` and `helpers/chains.rs`.
- [Declaration headers (`declareOne`)](https://github.com/google/google-java-format/blob/master/core/src/main/java/com/google/googlejavaformat/java/JavaInputAstVisitor.java#L3614)
  — type/name/initializer alignment and vertical annotation breaks; reference
  for `helpers/callables.rs` and declaration rules.
- [Comma-separated lists (`addArguments`)](https://github.com/google/google-java-format/blob/master/core/src/main/java/com/google/googlejavaformat/java/JavaInputAstVisitor.java#L3386)
  — tabular pairs, fill modes, and special argument-list layout; reference for
  `helpers/lists.rs`.
- [Input token and comment model (`JavaInput.java`)](https://github.com/google/google-java-format/blob/master/core/src/main/java/com/google/googlejavaformat/java/JavaInput.java)
  — token and trivia sequencing; reference for lossless CST comment ranges in
  the rule contract.
- [Comment rewriting (`JavaCommentsHelper.java`)](https://github.com/google/google-java-format/blob/master/core/src/main/java/com/google/googlejavaformat/java/JavaCommentsHelper.java)
  — comment rewrite during break computation; reference for comment wrappers and
  rejecting unhandled trivia.
- [Import ordering (`ImportOrderer.java`)](https://github.com/google/google-java-format/blob/master/core/src/main/java/com/google/googlejavaformat/java/ImportOrderer.java)
  — standalone import pass, not part of core layout; validates keeping import
  policy out of layout rules.
- [Multi-pass CLI (`FormatFileCallable.java`)](https://github.com/google/google-java-format/blob/master/core/src/main/java/com/google/googlejavaformat/java/FormatFileCallable.java)
  — chains format → remove unused imports → reorder; separate passes, not one
  visitor.

### palantir-java-format

Primary oracle for the Palantir profile. Fork of google-java-format with
systematic chain, indentation, and lambda policy differences.

- [Style profiles (`JavaFormatterOptions.java`)](https://github.com/palantir/palantir-java-format/blob/develop/palantir-java-format-spi/src/main/java/com/palantir/javaformat/java/JavaFormatterOptions.java)
  — Google, AOSP, and Palantir profile constants; Palantir uses 2× indent with a
  120-column limit.
- [Chain-breaking rationale (README)](https://github.com/palantir/palantir-java-format/blob/develop/README.md)
  — documents Palantir's 80-column cap on the last chain dot even when the full
  line fits.
- [AST visitor hub (`JavaInputAstVisitor.java`)](https://github.com/palantir/palantir-java-format/blob/develop/palantir-java-format/src/main/java/com/palantir/javaformat/java/JavaInputAstVisitor.java)
  — `METHOD_CHAIN_COLUMN_LIMIT` and Palantir-specific chain, lambda, and
  argument formatting.
- [Column-limit enforcement (`Level.java`)](https://github.com/palantir/palantir-java-format/blob/develop/palantir-java-format/src/main/java/com/palantir/javaformat/doc/Level.java)
  — rejects inlining when the last chain dot exceeds
  `columnLimitBeforeLastBreak`.
- [Partial-inlining guards (`PartialInlineability.java`)](https://github.com/palantir/palantir-java-format/blob/develop/palantir-java-format/src/main/java/com/palantir/javaformat/PartialInlineability.java)
  — when a level may be partially inlined without degenerate FQN breaks.
- [Inline-chain termination (`LastLevelBreakability.java`)](https://github.com/palantir/palantir-java-format/blob/develop/palantir-java-format/src/main/java/com/palantir/javaformat/LastLevelBreakability.java)
  — heuristics for stopping inlining in nested chain levels.
- [Break strategies (`BreakBehaviour.java`)](https://github.com/palantir/palantir-java-format/blob/develop/palantir-java-format/src/main/java/com/palantir/javaformat/BreakBehaviour.java)
  — Palantir break behaviours (`preferBreakingLastInnerLevel`, etc.) behind more
  aggressive inlining than upstream.
- [Fixture: chains and lambdas (`palantir-chains-lambdas.output`)](https://github.com/palantir/palantir-java-format/blob/develop/palantir-java-format/src/test/resources/com/palantir/javaformat/java/testdata/palantir-chains-lambdas.output)
  — golden output for chained calls with lambda arguments.
- [Fixture: deeply nested calls (`palantir-deeply-nested-calls.output`)](https://github.com/palantir/palantir-java-format/blob/develop/palantir-java-format/src/test/resources/com/palantir/javaformat/java/testdata/palantir-deeply-nested-calls.output)
  — golden output for deeply nested call trees under Palantir.

### Prettier

Architectural reference for language-neutral IR and comment attachment
boundaries.

- [Doc algebra spec (`commands.md`)](https://github.com/prettier/prettier/blob/main/commands.md)
  — canonical description of `group`, `ifBreak`, `fill`, `lineSuffix`, and line
  variants.
- [Exported Doc types (`public.d.ts`)](https://github.com/prettier/prettier/blob/main/src/document/public.d.ts)
  — typed IR contract keeping builders and printer separate from language
  plugins.
- [`group` builder](https://github.com/prettier/prettier/blob/main/src/document/builders/group.js)
  — core break-or-flat primitive.
- [`ifBreak` builder](https://github.com/prettier/prettier/blob/main/src/document/builders/if-break.js)
  — conditional docs chosen after break decisions.
- [`fill` builder](https://github.com/prettier/prettier/blob/main/src/document/builders/fill.js)
  — separator-aware wrapping distinct from all-or-nothing groups.
- [`lineSuffix` builder](https://github.com/prettier/prettier/blob/main/src/document/builders/line-suffix.js)
  — defers trailing content to end-of-line without scanning output.
- [Doc printer (`printer.js`)](https://github.com/prettier/prettier/blob/main/src/document/printer/printer.js)
  — pure layout engine; comment-free.
- [Comment attachment (`attach.js`)](https://github.com/prettier/prettier/blob/main/src/main/comments/attach.js)
  — pre-print pass decorating nodes with leading/trailing/dangling comments.
- [Comment print layer (`print.js`)](https://github.com/prettier/prettier/blob/main/src/main/comments/print.js)
  — wraps node docs with comment docs via `lineSuffix` for end-of-line comments.
- [AST → Doc pipeline (`ast-to-doc.js`)](https://github.com/prettier/prettier/blob/main/src/main/ast-to-doc.js)
  — attach → print → wrap → render layering.

### Oxc

Architectural reference for span-ordered comment cursors and core/language
split.

- [Core vs language boundary (`AGENTS.md`)](https://github.com/oxc-project/oxc/blob/main/crates/oxc_formatter_core/AGENTS.md)
  — language-agnostic IR/printer vs consumer-owned comments and grammar rules.
- [FormatElement IR (`mod.rs`)](https://github.com/oxc-project/oxc/blob/main/crates/oxc_formatter_core/src/format_element/mod.rs)
  — language-neutral document elements including line-suffix boundaries.
- [IR builders (`builders.rs`)](https://github.com/oxc-project/oxc/blob/main/crates/oxc_formatter_core/src/builders.rs)
  — shared helpers building IR over a generic context type.
- [Printer (`mod.rs`)](https://github.com/oxc-project/oxc/blob/main/crates/oxc_formatter_core/src/printer/mod.rs)
  — IR-to-text second stage.
- [FormatContext traits (`traits.rs`)](https://github.com/oxc-project/oxc/blob/main/crates/oxc_formatter_core/src/traits.rs)
  — core traits without language semantics.
- [FormatState (`state.rs`)](https://github.com/oxc-project/oxc/blob/main/crates/oxc_formatter_core/src/state.rs)
  — per-document context holder for an entire format pass.
- [Comment cursor (`comments.rs`)](https://github.com/oxc-project/oxc/blob/main/crates/oxc_formatter/src/formatter/comments.rs)
  — on-demand cursor with `printed_count`; primary reference for Jolt's
  claimed-comment model.
- [Trivia formatting (`trivia.rs`)](https://github.com/oxc-project/oxc/blob/main/crates/oxc_formatter/src/formatter/trivia.rs)
  — leading/trailing/dangling emission and cursor advancement.
- [JS format context (`context.rs`)](https://github.com/oxc-project/oxc/blob/main/crates/oxc_formatter/src/formatter/context.rs)
  — comments live in the language crate, not the core.
- [Line suffix printing (`line_suffixes.rs`)](https://github.com/oxc-project/oxc/blob/main/crates/oxc_formatter_core/src/printer/line_suffixes.rs)
  — defer trailing trivia until line break or boundary.

### Ruff

Architectural reference for rule contracts and comment bucket accounting.

- [Formatter crate overview (`lib.rs`)](https://github.com/astral-sh/ruff/blob/main/crates/ruff_formatter/src/lib.rs)
  — `FormatRule`, `FormatContext`, and IR → print pipeline.
- [Formatter session API (`formatter.rs`)](https://github.com/astral-sh/ruff/blob/main/crates/ruff_formatter/src/formatter.rs)
  — context-aware formatting with shared list/break builders.
- [Layout builders (`builders.rs`)](https://github.com/astral-sh/ruff/blob/main/crates/ruff_formatter/src/builders.rs)
  — breaking/flat layout expressed in IR, not in each rule.
- [IR buffer (`buffer.rs`)](https://github.com/astral-sh/ruff/blob/main/crates/ruff_formatter/src/buffer.rs)
  — how rules accumulate `FormatElement`s during formatting.
- [Printer (`mod.rs`)](https://github.com/astral-sh/ruff/blob/main/crates/ruff_formatter/src/printer/mod.rs)
  — width-aware breaking from IR alone.
- [FormatElement document (`document.rs`)](https://github.com/astral-sh/ruff/blob/main/crates/ruff_formatter/src/format_element/document.rs)
  — tagged document structure rules emit.
- [`FormatNodeRule` contract (`lib.rs`)](https://github.com/astral-sh/ruff/blob/main/crates/ruff_python_formatter/src/lib.rs)
  — leading → fields → trailing ordering; Python analogue of Jolt's rule
  contract.
- [Comment bucket model (`comments/mod.rs`)](https://github.com/astral-sh/ruff/blob/main/crates/ruff_python_formatter/src/comments/mod.rs)
  — leading/dangling/trailing ownership and `assert_all_formatted`.
- [Per-node comment map (`map.rs`)](https://github.com/astral-sh/ruff/blob/main/crates/ruff_python_formatter/src/comments/map.rs)
  — bucket storage and lookup per syntax node.
- [Comment placement heuristics (`placement.rs`)](https://github.com/astral-sh/ruff/blob/main/crates/ruff_python_formatter/src/comments/placement.rs)
  — syntax-aware reassignment when default buckets are wrong.

## Design Principles

### Keep The Renderer Language-Neutral

`jolt_fmt_ir` should not learn Java concepts. It may grow general document
features if a Java helper proves the need, but concepts such as method chains,
throws clauses, type parameters, imports, annotations, and switch labels belong
in `jolt_java_fmt`.

Add renderer features only when the helper layer cannot express a broad policy
with existing primitives. Preference order:

1. Compose existing `Doc` primitives in a Java helper.
2. Add a small Java helper that names the formatting policy.
3. Add a general IR primitive only after multiple helpers need the same renderer
   behavior.

Keep generic formatter ergonomics separate from Java policy. A reusable
delimited or separated-list utility can live below Java-specific argument,
parameter, annotation, or type-list policy. A selector-chain classifier, by
contrast, is Java formatter policy and should not leak into `jolt_fmt_ir`.

### Make Rule Modules Structural

Rule modules should answer grammar questions:

- What kind of syntax node is this?
- Which child nodes and source ranges belong to each semantic slot?
- Which helper should format this Java construct?
- Which associated comments must be emitted or delegated?

Rule modules should avoid owning low-level wrapping decisions such as whether a
comma list fills independently, whether a selector chain breaks before the first
selector, or how a declaration header aligns when a type breaks.

### Formatter Rule Contract

Each Java node rule follows the same contract:

1. Identify the node's source range and grammar slots through CST wrappers.
2. Ask the formatter context/comment service for comments associated with the
   node or slot.
3. Format child slots through rule functions or domain helpers.
4. Emit leading and trailing comments through shared wrappers.
5. Explicitly place, delegate, or reject dangling and inline comments.
6. Return a real `Doc` for parser-clean syntax.

Raw source passthrough is not a formatter rule. For parser-clean syntax,
returning a document made from an arbitrary node's original source text hides
missing layout coverage. Source text may be used only at the token/literal
boundary where preserving the token spelling is the formatting rule.

### Preserve Context Explicitly

Prettier uses `AstPath` to make parent, sibling, and list-position-sensitive
decisions. Jolt does not need a generic path object, but it does need an
explicit context story.

CST wrappers should expose grammar roles and source ranges. Formatter rules and
helpers may pass narrow context objects when layout depends on ancestry or
position, for example:

- whether an expression is a nested argument,
- whether a selector chain is itself a receiver,
- whether a node is first, last, or separated by comments in a list,
- whether a declaration appears in a class, interface, enum, annotation body,
  compact compilation unit, or local block,
- whether parenthesization is required by parent precedence.

Avoid hiding layout policy in CST wrappers. Wrappers expose source facts;
formatter rules and helpers decide layout.

### Put Policy In Named Helpers

The helper layer should have Java-domain names. A future reader should be able
to scan a rule and see Java formatting intent rather than raw document-builder
plumbing.

Good helper names:

- `callable_header`
- `formal_parameter_list`
- `argument_list`
- `type_argument_list`
- `type_parameter_list`
- `selector_chain`
- `binary_expression_chain`
- `assignment_expression`
- `annotation_group`
- `declaration_modifiers`
- `class_body_members`
- `statement_block`
- `switch_block`
- `import_section`

Prefer "helper", "rule", "analyzer", or "formatter" for Java-domain code. Save
"builder" for generic document construction APIs if Jolt grows a Ruff/Oxc-like
builder facade.

Poor long-term rule-module patterns:

- open-coded `concat([text("("), soft_line(), ...])`
- open-coded comma separators in multiple domains
- width-sensitive conditionals in rule modules
- profile checks repeated near leaf syntax formatting
- ad hoc comment fallbacks that append source text outside the owning rule

### Treat Comments As Layout Inputs

Comments are not an afterthought. A helper that formats a list, block, chain, or
declaration should have an answer for comments inside that construct.

The comment model has two distinct phases:

1. Classify and associate comments from source positions.
2. Render associated comments through node rules and helpers.

Source-position classification distinguishes own-line, end-of-line, and
inline/remaining comments. Ownership resolution uses preceding, enclosing, and
following ranges plus adjacency and blank-line facts.

Rendering exposes associated comments as leading, trailing line, inline block,
dangling, list-item, and before-closing-delimiter buckets. Jolt already has
`line_suffix` and `line_suffix_boundary`; trailing comments should route through
helpers consistently.

The formatter fails tests when comments are unaccounted. New supported syntax
must place comments through the owning rule or helper, not through silent
appendage or ignored trivia.

### Keep Profiles Coarse And Opinionated

Profiles are compatibility targets, not arbitrary style knobs. The formatter
should expose Google, AOSP, and Palantir profile behavior through a small number
of policy structs or helper methods, not through dozens of independent options.

Profile differences should be centralized when possible:

- indentation width,
- import section grouping,
- continuation indentation,
- chain and argument wrapping preferences,
- Palantir-specific line breaking where it is a systematic style difference.

## Module Layout

Current structure under `crates/jolt_java_fmt/src/`:

```text
layout.rs                  low-level Doc composition helpers
policy.rs                  profile policy accessors
comments.rs                comment formatting wrappers
context.rs                 comment ownership and trivia cursor
rules/                     syntax-to-document rules by domain
helpers/
  separated.rs             generic separated/delimited mechanics
  lists.rs                 Java list policy and comment-aware items
  callables.rs             callable declaration headers and tails
  type_declarations.rs     type declaration headers
  chains.rs                selector chain layout policy
  bodies.rs                blocks and type bodies
  annotations.rs           annotation layout helpers
  literals.rs              literal formatting
analyzers/
  chains.rs                chain flattening, metadata, grouping
```

Not yet extracted (extract only when a real policy surface justifies it):

- `helpers/expressions.rs` — binary chains, assignment, lambda, array
  initializer
- `helpers/imports.rs` — import section grouping and blank-line policy
- `analyzers/binary.rs` — same-precedence binary flattening

Do not move code merely to satisfy this tree.

## Helper Surface Status

### Lists — largely done

`helpers/separated.rs` and `helpers/lists.rs` cover delimited comma lists,
one-per-line lists, fill-style behavior, keyword-prefixed clause lists, and
comment-aware item formatting for arguments, parameters, type arguments, and
type parameters.

Remaining: extend comment placement into any list domains still blocking on
unowned trivia; keep profile-specific indentation centralized in policy.

### Callable and type declarations — partial

`helpers/callables.rs` and `helpers/type_declarations.rs` are wired into
`declarations.rs` for methods, constructors, compact constructors, annotation
elements, and type headers.

Remaining: move throws/default/annotation-header comment policy out of
`declarations.rs`; shrink the rule module so header wrapping changes happen in
one helper.

### Selector chains — in progress

`analyzers/chains.rs` flattens selector syntax into `ChainMember` sequences with
metadata and `ChainGroup`s. `helpers/chains.rs` renders staged alternatives with
profile-aware breaking via `JavaFormatPolicy`.

Remaining: close oracle gaps on long fluent chains, mixed field/call chains,
nested-argument receivers, and Palantir-specific indentation—using syntax shape,
not fixture-name heuristics.

### Expressions — not extracted

Binary, assignment, conditional, cast, parenthesized, array initializer, and
lambda wrapping still live in `rules/expressions.rs` and `layout.rs`.

Target: a `helpers/expressions.rs` (and possibly `analyzers/binary.rs`) that
owns precedence, associativity, and comment-forced breaks.

### Blocks and bodies — largely done

`helpers/bodies.rs` covers statement blocks, constructor bodies, and class,
interface, and enum bodies with dangling-comment support.

Remaining: switch-block spacing polish and any body domains still delegating
blank-line policy through scattered `join(hard_line())` calls in rules.

### Imports and compilation units — partial

`rules/compilation_unit.rs` handles package, imports, modules, and compact
members. AOSP static-import separation goes through `JavaFormatPolicy`.

Remaining: extract an `import_section` helper when compilation-unit rules need
more profile-specific grouping without growing the rule module.

## Remaining Work

Work is organized by the original phase sequence. Phases 1–3 are complete; later
phases overlap in practice.

### Phase 1: Stabilize the helper vocabulary — complete

Profile policy, rule contract, separated/delimited mechanics, Java list helpers,
and narrow-width helper tests are in place.

### Phase 2: Eliminate raw source passthrough — complete

All parser-clean syntax formats through real rules. Oracle suites are the broad
coverage signal for this milestone; unit tests stay focused on helper
boundaries.

### Phase 3: Comment ownership — largely complete

Ownership buckets, rejection guards, and unconsumed-trivia diagnostics replace
order-only handling and late appendage.

Remaining edge domains (currently blocked, not silently ignored):

- annotation argument comments,
- inline comments inside annotations and some header positions,
- some header-boundary and branch/else comment positions.

See `rules/tests.rs` tests named `*_remain_unowned_formatter_debt`.

### Phase 4: Callable and type declaration helpers — in progress

Helpers exist and are adopted, but `declarations.rs` has not shrunk materially.
Success means declaration wrapping and header comment policy change in helpers,
with neutral or improved declaration-related oracle diffs.

### Phase 5: Selector chain policy — in progress

Analyzer and helper infrastructure is in place. Success means explainable chain
behavior from syntax shape and materially smaller diffs on `B24909927.java`,
`B20701054.java`, and Palantir nested-call fixtures—without Palantir regressions
elsewhere.

Measure Google, AOSP, and Palantir scoreboards after each broad chain rule
change. Review by domain, not aggregate number alone.

### Phase 6: Owned comments in helpers — in progress

List, body, and chain helpers already integrate many comment buckets.

Remaining: finish placement in the blocked domains above; add helper-boundary
comment tests where oracle fixtures do not already cover a shape.

### Phase 7: Profile-specific oracle alignment — in progress

Policy accessors centralize known profile differences. Remaining work tracks
oracle reports: Palantir chain and indentation behavior is the largest gap.

Keep Google as the base unless a helper has a documented profile divergence.

## Verification

Standard local gates:

```sh
cargo fmt --check
INSTA_UPDATE=no cargo test -p jolt_java_fmt
cargo test -p jolt_java_syntax --lib
```

When changing oracle-facing layout policy:

```sh
INSTA_UPDATE=always cargo test -p jolt_java_fmt --test oracle_fixtures
rg -n "missing-rule blocked|aggregate diff size|largest per-file diff" \
  crates/jolt_java_fmt/tests/snapshots/oracle_fixtures__*_scoreboard.snap
```

Scoreboard changes should be reviewed by domain, not only by aggregate number.
An aggregate improvement that creates a new concentrated regression in a core
fixture should be treated skeptically.

Coverage invariants (missing-layout exits, raw source passthrough, late
remaining-comment appendage) are satisfied and should stay that way. Re-check
with `rg` only if those mechanisms are reintroduced.

## Non-Goals

- Do not introduce arbitrary user style knobs.
- Do not move Java policy into `jolt_fmt_ir`.
- Do not add unsupported-layout exits as scaffolding.
- Do not add raw-source formatting fallbacks as scaffolding.
- Do not optimize for one fixture by naming methods, classes, or files.
- Do not silently drop, append, or ignore comments to make tests pass.
- Do not split modules mechanically without extracting a real abstraction.
