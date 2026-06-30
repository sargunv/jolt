# Formatter Abstraction Layer Improvement Plan

The Java formatter has crossed the first important architectural threshold:
parser-clean Java accepted by `jolt_java_syntax` should be routed through real
layout rules, not unsupported-layout exits. The next phase is to make the
formatter easier to improve without turning every oracle mismatch into a local
case patch.

This document replaces the old coverage-blocker inventory. Its focus is the
abstraction layer between Java CST wrappers and the shared document IR.

## Current Status

The broad architecture remains correct:

```text
source text
  -> jolt_java_syntax parser
  -> lossless CST + wrapper accessors
  -> Java rule modules
  -> Java layout helpers
  -> shared document IR
  -> shared renderer
```

The right next step is not a different formatter architecture. The right next
step is stronger Java-specific layout abstractions so the rule modules describe
syntax roles and the helper layer owns formatting policy.

Current strengths:

- Java formatting profiles are part of formatter options and context.
- Rule code is split by domain: compilation units, declarations, expressions,
  statements, annotations, types, names, and tokens.
- The shared renderer stays language-neutral.
- Comment trivia is tracked in the Java formatter rather than hidden in the
  parser or renderer.
- Oracle scoreboards provide integration feedback across Google, AOSP, and
  Palantir profiles.

Current pressure points:

- `declarations.rs`, `expressions.rs`, and `statements.rs` still contain too
  much local policy.
- `layout.rs` has useful primitives, but not enough domain-shaped helpers.
- Comment handling is accounted for in simple cases, but the current
  source-ordered cursor and late remaining-comment appendage are transitional.
  The long-term model needs explicit ownership before list, chain, declaration,
  and body helpers can place comments well.
- Profile-specific behavior exists, but the abstraction boundary for profile
  policy is still thin.
- The largest oracle diffs are layout-policy issues, especially nested call
  chains, argument lists, text blocks, and Palantir-specific indentation.

## Reference Project Lessons

The plan is intentionally informed by Ruff, Oxc, and Prettier, but Jolt should
copy their boundaries rather than their exact APIs.

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

Layering target:

```text
shared IR, builders, and renderer
  -> Java formatter context, profile policy, comments, and trivia services
  -> Java node rules with an explicit formatting contract
  -> Java analyzers and layout helpers
```

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

### Define A Formatter Rule Contract

Each Java node rule should follow the same contract:

1. Identify the node's source range and grammar slots through CST wrappers.
2. Ask the formatter context/comment service for comments associated with the
   node or slot.
3. Format child slots through rule functions or domain helpers.
4. Emit leading and trailing comments through shared wrappers.
5. Explicitly place, delegate, or reject dangling and inline comments.
6. Return a real `Doc` for parser-clean syntax.

This contract is the local analogue of Ruff's node-formatting rule layer and
Prettier's path-driven printer. It should make unsupported or unplaced source
facts visible during tests instead of being hidden by late fallback output.

Raw source passthrough is not a formatter rule. For parser-clean syntax,
returning a document made from an arbitrary node's original source text is
illegal for the same reason `missing_layout` is illegal: it hides missing layout
coverage instead of implementing it. Source text may be used only at the
token/literal boundary where preserving the token spelling is the formatting
rule.

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

The long-term comment model should have two distinct phases:

1. Classify and associate comments from source positions.
2. Render associated comments through node rules and helpers.

Source-position classification should distinguish:

- own-line comments,
- end-of-line comments,
- inline or remaining comments.

Ownership resolution should use:

- preceding node range,
- enclosing node range,
- following node range,
- adjacency and blank-line facts.

Rendering should expose associated comments as:

- leading comments before a node,
- trailing line comments after a node,
- inline block comments between tokens,
- dangling comments inside empty or sparse containers,
- comments between list items,
- comments before closing delimiters.

The formatter should continue to fail tests when comments are unaccounted. The
current `take_remaining_comment_docs` appendage is debt: it preserves text while
coverage is maturing, but new supported syntax should place comments through the
owning rule/helper instead of relying on a late append.

Jolt already has `line_suffix` and `line_suffix_boundary`; the work is to route
trailing comments through helpers consistently, not to add basic IR support.

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

## Target Helper Surface

### Lists

Lists are the most important abstraction to strengthen. They appear in
arguments, parameters, type arguments, type parameters, annotation arguments,
array initializers, resources, throws clauses, switch labels, enum constants,
and module directive targets.

Build a small family of policy-bearing list helpers:

- generic separated-list and delimited-list mechanics,
- delimited comma list: `(...)`, `<...>`, `{...}`
- one-per-line delimited list
- fill-style list for short items
- forced-break list for declaration-sensitive constructs
- semicolon list for `for` headers and resources
- pipe list for catch unions
- keyword-prefixed lists such as `throws`, `implements`, `permits`, `to`, and
  `with`

Each list helper should own:

- delimiters,
- separators,
- indentation,
- empty-list behavior,
- trailing and dangling comments,
- whether items break independently or as one group,
- profile-specific indentation.

Refactoring target: rule modules should pass item docs and list kind, not build
the separator machinery directly.

Keep the layers separate:

- generic separated/delimited helpers own reusable separator mechanics,
- Java helpers such as argument lists and parameter lists own construct-specific
  wrapping and comment policy.

### Callable Declarations

Method, constructor, annotation element, compact constructor, and record
component declarations share a header problem: modifiers, type parameters,
result type, name, parameter list, throws/default clauses, and body/default
value all compete for line width.

Create a callable declaration helper that accepts named slots:

- modifiers and annotations,
- optional type parameters,
- optional result type,
- callable name,
- formal parameter or receiver parameter list,
- optional trailing dimensions,
- optional throws clause,
- optional default value,
- body shape.

The helper should own:

- breaking between type and name,
- continuation indentation for throws/default clauses,
- when parameter lists force one-per-line behavior,
- comment attachment around header slots,
- profile-specific continuation rules.

### Type Declarations

Classes, records, interfaces, annotation interfaces, and enums share a header
shape: modifiers, declaration keyword, name, type parameters, clauses, and body.

Create a type declaration helper that owns:

- vertical declaration annotations,
- keyword/name spacing,
- type parameter placement,
- `extends`, `implements`, and `permits` clauses,
- record component list placement,
- body opening and empty-body behavior,
- blank lines between body member groups.

This should reduce the amount of local header construction in `declarations.rs`.

### Selector Chains

Selector chains are currently the largest oracle-alignment domain. The helper
should flatten all selector-like syntax before layout:

- member select,
- method invocation,
- array access,
- class instance creation selectors,
- `this` and `super` qualified selectors where applicable.

The chain helper should classify:

- field-only chains,
- static factory plus builder chains,
- long fluent call chains,
- mixed field/call chains,
- chains used as nested arguments,
- chains with long argument lists,
- chains ending in simple terminal calls.

The helper should own:

- whether the first selector remains glued to the receiver,
- when to break before every dot,
- when field prefixes use fill behavior,
- how nested argument lists influence selector breaking,
- profile-specific chain preferences.

Avoid case patches keyed to fixture names or method names. If selector metadata
is needed, add it because it describes syntax shape: selector kind, argument
count, argument complexity, source span, or whether the receiver is itself a
chain.

The implementation should be analyzer-first:

1. Flatten selector syntax into a `ChainMember` sequence.
2. Attach syntax metadata such as selector kind, argument count, argument
   complexity, receiver shape, comments, and source ranges.
3. Partition members into `ChainGroup`s or equivalent grouping objects.
4. Render from those groups with a small number of staged alternatives.

Name-based heuristics should be avoided by default. If a Java profile or oracle
eventually requires one, document the evidence and keep the heuristic narrow.

### Expressions

Expression helpers should focus on precedence and shape rather than individual
syntax variants.

Targets:

- binary chain helper that flattens only same-precedence operators,
- associativity and operator exception handling for binary flattening,
- assignment helper that breaks after the operator,
- conditional expression helper,
- cast and parenthesized expression helpers,
- array initializer helper with policy for short values, nested values, and
  one-per-line values,
- lambda helper for concise vs typed parameters and expression vs block bodies.

Rule modules should not hand-roll binary or assignment wrapping. Binary-like
formatting should know about parent precedence, associativity, operator
families, and comment-forced breaks.

### Blocks And Bodies

Blocks need explicit blank-line and separator policy. This should not live as
scattered `join(hard_line())` calls.

Targets:

- ordinary statement blocks,
- constructor bodies with explicit constructor invocation,
- class and interface bodies,
- enum bodies with constants and members,
- annotation interface bodies,
- switch blocks,
- module directive blocks,
- compact compilation-unit member lists.

Each body helper should own:

- empty body behavior,
- blank lines between member groups,
- dangling comments,
- empty declarations,
- separators after enum constants,
- switch group/rule spacing.

### Imports And Compilation Units

Compilation-unit formatting should stay thin. It should identify package,
imports, module declaration, and compact members, then delegate policy.

Targets:

- import section helper with profile-specific grouping,
- package annotations helper,
- module directive grouping helper,
- compact compilation-unit member list helper,
- top-level blank-line policy helper.

## Suggested Module Shape

The current domain split is good. The next split should separate syntax rules
from reusable policy helpers.

Possible structure:

```text
crates/jolt_java_fmt/src/
  layout.rs                  low-level Doc composition helpers
  policy.rs                  profile policy accessors
  comments.rs                comment collection, ownership, and formatting
  rules/
    compilation_unit.rs
    declarations.rs
    expressions.rs
    statements.rs
    annotations.rs
    types.rs
    names.rs
    tokens.rs
  helpers/
    separated.rs
    lists.rs
    callables.rs
    type_declarations.rs
    chains.rs
    expressions.rs
    bodies.rs
    imports.rs
  analyzers/
    chains.rs
    binary.rs
```

This split should happen incrementally. Do not move code merely to satisfy this
tree. Extract a helper module when there is a real policy surface and at least
two call sites or one high-complexity call site that becomes clearer.

## Work Plan

### Phase 1: Stabilize The Helper Vocabulary

Goal: make future formatter changes speak in domain helpers.

Tasks:

1. Introduce a profile policy accessor layer, even if it initially wraps the
   existing `JavaFormatProfile` checks.
2. Define the formatter rule contract in code comments or module docs near the
   rule entry points.
3. Move generic separated/delimited-list mechanics out of generic `layout.rs`
   into a focused helper module or clearly named section.
4. Define Java helper entry points for argument lists, formal parameter lists,
   type parameter lists, type argument lists, and keyword-prefixed clause lists.
5. Add narrow-width tests at helper boundaries for flat and broken forms.

Success signal:

- rule modules call named helpers for common list shapes,
- rules expose source ranges and grammar slots without hiding policy in CST
  wrappers,
- no oracle regressions,
- no missing layout exits,
- formatter and syntax tests pass.

### Phase 2: Eliminate Raw Source Passthrough

Goal: remove arbitrary source-copy formatting paths for parser-clean syntax.

Tasks:

1. Inventory every `format_raw_source_text` call site by syntax domain.
2. Replace raw-source fallbacks with real layout rules, starting with the
   largest domains rather than isolated cases.
3. Keep exact token spelling through token or literal formatting helpers only.
4. Add focused tests only for rule boundaries or syntax shapes not already
   exercised by existing formatter tests or oracle fixtures.
5. Remove `format_raw_source_text` when no longer needed by formatter rules.

Success signal:

- `rg -n "format_raw_source_text" crates/jolt_java_fmt/src` produces no
  formatter rule matches,
- parser-clean syntax either formats through real rules or exposes a parser/
  wrapper bug that must be fixed,
- no missing layout exits,
- formatter and syntax tests pass.

The oracle suites are the broad coverage signal for this phase. Unit tests
should stay focused and minimal; do not add one test per removed fallback when
an existing oracle fixture already covers the domain.

### Phase 3: Build Comment Ownership Before Comment Placement

Goal: replace order-only comment handling with explicit ownership.

Tasks:

1. Classify comments by source position: own-line, end-of-line, and
   inline/remaining.
2. Resolve ownership using preceding, enclosing, and following ranges.
3. Expose leading, trailing, dangling, inline, and list-item comment buckets to
   rules/helpers.
4. Keep `line_suffix` and `line_suffix_boundary` as the trailing-comment
   rendering mechanism.
5. Add tests that fail on unconsumed comments instead of relying on late
   appendage.

Success signal:

- new supported syntax does not depend on `take_remaining_comment_docs`,
- helper tests cover comments inside lists, bodies, chains, and declarations,
- ambiguous comment ownership is reported in tests or diagnostics rather than
  silently appended.

### Phase 4: Extract Callable And Type Declaration Helpers

Goal: make declarations declarative instead of locally assembled.

Tasks:

1. Add a callable declaration helper for methods, constructors, annotation
   elements, and compact constructors.
2. Add a type declaration helper for class, record, interface, annotation
   interface, and enum headers.
3. Move throws, default value, type parameter, and declaration annotation
   policies into these helpers.
4. Add tests for long headers, annotations, type parameters, throws clauses,
   record components, and compact constructors.

Success signal:

- `declarations.rs` shrinks materially,
- declaration wrapping changes can be made in one helper,
- largest declaration-related oracle diffs decrease or remain neutral.

### Phase 5: Rebuild Selector Chain Policy

Goal: attack the largest shared oracle mismatch domain without case patches.

Tasks:

1. Make selector flattening produce structured metadata, not just docs.
2. Introduce chain member/group analyzer types.
3. Classify chain shape before rendering.
4. Implement staged alternatives for field chains, builder chains, and deeply
   nested call chains.
5. Measure Google, AOSP, and Palantir scoreboards after each broad rule.
6. Keep unit tests for narrow selector chains so local readability does not
   regress unnoticed.

Success signal:

- `B24909927.java` improves across at least Google and AOSP without a Palantir
  blow-up,
- `B20701054.java` and deeply nested call fixtures do not regress materially,
- chain behavior is explainable from syntax shape.

### Phase 6: Integrate Owned Comments Into Helpers

Goal: move comments from accounting correctness toward layout correctness.

Tasks:

1. Teach list helpers about associated comments between items and before closing
   delimiters.
2. Teach body helpers about dangling comments and blank-line preservation.
3. Teach selector-chain and callable helpers where inline and dangling comments
   may appear.
4. Add focused comment tests for arguments, parameters, blocks, class bodies,
   switch blocks, and selector chains.

Success signal:

- fewer late remaining-comment appendages,
- comment tests exercise helper boundaries,
- oracle diffs caused by comments shrink without hiding unhandled trivia.

### Phase 7: Profile-Specific Oracle Alignment

Goal: make profile differences explicit and maintainable.

Tasks:

1. Centralize known profile differences in policy accessors.
2. Keep Google as the base unless a helper has a documented profile divergence.
3. Add AOSP import grouping and indentation behavior through profile policy.
4. Add Palantir-specific wrapping only where reports show systematic style
   divergence.

Success signal:

- profile checks are rare in rule modules,
- profile behavior is easy to audit,
- scoreboard changes can be attributed to policy decisions.

## Verification Gates

Every abstraction change should preserve the coverage invariant:

```sh
rg -n "MissingLayoutRules|missing_layout_rules|missing_layout" crates/jolt_java_fmt
```

The command should produce no formatter matches.

Minimum local gates:

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

Raw source passthrough is illegal and must be eliminated alongside
missing-layout exits:

```sh
rg -n "format_raw_source_text" crates/jolt_java_fmt/src
```

This command should produce no formatter rule matches. Token and literal rules
may preserve exact token spelling through dedicated token-formatting helpers,
but rules must not format arbitrary accepted syntax by copying its source text.

## Non-Goals

- Do not introduce arbitrary user style knobs.
- Do not move Java policy into `jolt_fmt_ir`.
- Do not add unsupported-layout exits as scaffolding.
- Do not add raw-source formatting fallbacks as scaffolding.
- Do not optimize for one fixture by naming methods, classes, or files.
- Do not silently drop, append, or ignore comments to make tests pass.
- Do not split modules mechanically without extracting a real abstraction.

## Practical Next Moves

The best next implementation sequence is:

1. Define the formatter rule contract and profile policy accessor layer.
2. Extract generic separated/delimited mechanics, then route Java argument,
   formal, and type lists through Java helpers.
3. Eliminate raw source passthrough domain by domain.
4. Replace order-only comment handling with comment ownership buckets.
5. Extract callable and type declaration helpers from `declarations.rs`.
6. Rework selector chain metadata and staged layout alternatives.
7. Integrate owned list/body/chain comments into those helpers.
8. Centralize profile policy decisions as they become visible.

This keeps the formatter moving in broad domains while improving the codebase's
ability to absorb oracle-alignment work without becoming a fixture-by-fixture
patch pile.
