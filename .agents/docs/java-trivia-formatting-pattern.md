# Java Formatter Trivia Pattern

## Problem

The Java formatter sits on a lossless CST: every token keeps its source text,
leading trivia, trailing trivia, and token-only range. That substrate is good
enough to preserve comments placed in legal but unusual Java positions.

The formatter does not yet use that substrate consistently. Some rules emit the
original token with its comments, while other rules reconstruct the same token
with `text("...")` or `format_token_text(token.text())`. Those reconstructed
tokens silently drop comments attached to the original token.

The destructive cases are not limited to friendly comment positions. Java allows
comments between many adjacent tokens:

```java
@ /* comment */ Deprecated
java /* comment */ . util /* comment */ . List
void method /* comment */ ()
int[] values = new int /* comment */ [ /* comment */ ] { 1 };
assert ok : /* comment */ message;
```

Any formatter rule that knows the original token and emits raw text instead is a
potential trivia-loss bug.

## Invariants

- Formatter output must never drop comments from a clean parse.
- Formatter output must not duplicate comments. Every source comment should be
  rendered exactly once, except formatter-control markers intentionally consumed
  by formatter-ignore handling.
- Prefer formatting from original tokens. Synthesized Java tokens are only for
  explicit formatter policy insertions, such as readability parentheses or
  trailing separators. Structural document text, such as spaces and line breaks,
  is not a Java token.
- Recovered syntax may be formatted structurally, but missing source tokens are
  holes. Do not repair recovered syntax by inventing Java tokens that were not
  present in the source.
- Raw combined text spanning multiple Java tokens, such as `".*"`, `"[]"`,
  `"()"`, `".class"`, or `"switch ("`, is synthetic-only. If any constituent
  source token exists, format each token independently.
- `CommentMap` is for constructs that are reordered or moved as whole units. It
  must not replace token-local trivia preservation for ordinary formatting.
- Exact comment placement can improve incrementally, but marker-preservation
  regressions must pass before a case is considered safe.
- A formatter helper that accepts a `JavaSyntaxToken` must render that token's
  comments unless its name or arguments explicitly say otherwise.
- Trivia suppression is only valid when the token has no trivia or when an
  explicit caller has already moved that trivia. Suppression is not a normal
  spacing option.
- Missing typed accessors are not permission to scan raw children in formatter
  rules. For clean parses, add a grammar-role accessor in `jolt_java_syntax` and
  keep the formatter rule token-aware.
- Marker-preservation regression fixtures must continue to require clean parse,
  diagnostics-free formatting, marker preservation, formatted-output parse, and
  idempotence.

## Existing Patterns To Keep

### Token leaf formatting

`format_token_with_comments` is the correct primitive for simple leaf tokens:

```rust
concat([
    format_leading_comments(token),
    format_token_text(token.text()),
    format_trailing_comments(token),
])
```

It should become the default for identifiers, keywords, operators, and
punctuation when the token is not inside a specialized delimiter/list policy.

### Delimiter and separator helpers

The list helpers are the best current model:

- they accept source tokens for open, close, and separators,
- they preserve leading/trailing comments,
- they classify delimiter-attached comments as dangling when appropriate,
- they choose line behavior locally to the delimiter policy.

Keep this pattern for parens, brackets, braces, angle brackets, commas,
semicolons in header lists, `:` separators, arrows, and similar syntax.

### CommentMap for moved constructs

Imports and module directives are sorted. Their leading/trailing construct
comments cannot simply stay where a token renderer would put them after
reordering. `CommentMap` is appropriate there:

- capture comments anchored to the first and last tokens,
- sort the formatted construct with its comment envelope,
- use token-aware formatting inside the construct body.

This pattern is not appropriate for non-moved code because it cannot preserve
comments in the middle of a construct.

### Raw preserved ranges

Formatter-ignore ranges should continue using source slices and `literal_text`.
That is a separate raw-preservation policy, not normal trivia formatting.

## Target API

Add a small token-emission layer in
`crates/jolt_java_fmt/src/helpers/comments.rs` or a sibling helper module. The
exact names can change, but the API should make the preservation mode visible at
the call site.

Suggested shape:

```rust
enum LeadingTrivia {
    Preserve,
    SuppressAlreadyHandled,
}

enum TrailingTrivia {
    Preserve,
    BeforeLineBreak,
    SuppressAlreadyHandled,
}

fn format_token(token: &JavaSyntaxToken, leading: LeadingTrivia, trailing: TrailingTrivia) -> Doc;
fn format_inserted_policy_token(text: &'static str, reason: InsertedTokenReason) -> Doc;
fn format_required_token(token: Option<&JavaSyntaxToken>, mode: TokenMode) -> Doc;
fn format_keyword(
    token: Option<&JavaSyntaxToken>,
    fallback: &'static str,
    mode: KeywordMode,
) -> Doc;
fn format_punctuation(
    token: Option<&JavaSyntaxToken>,
    fallback: &'static str,
    mode: PunctuationMode,
) -> Doc;
```

The important behavior:

- `format_token(...)` is the only place that combines token text with leading
  and trailing comments. Delimiter, separator, and terminator helpers are part
  of this token-emission layer because they deliberately reclassify some token
  trivia as dangling or line-suffix trivia.
- `format_inserted_policy_token(...)` makes it obvious that no source trivia can
  be preserved and that the formatter is intentionally inserting a Java token as
  formatting policy, not syntax repair.
- `format_required_token(...)` uses the source token if present. If the token is
  missing in recovered syntax, it renders a hole such as `nil()` and lets the
  surrounding recovered construct format structurally. It must not print a
  fallback Java token.
- Keyword and punctuation helpers may accept fallback strings only for
  unreachable clean-parse absence or diagnostic/debug paths. When a source token
  is present, they must render `token.text()`, not the fallback string.
- `LeadingTrivia::Preserve` formats leading comments as leading comments before
  the token. `LeadingTrivia::SuppressAlreadyHandled` is only for tokens whose
  leading trivia was already emitted by an enclosing helper.
- `TrailingTrivia::Preserve` formats trailing comments and emits a hard line
  after line comments or multiline block comments. `BeforeLineBreak` formats
  trailing comments but leaves the final break decision to the caller.
  `SuppressAlreadyHandled` is only for explicitly moved trailing trivia.

Do not require every helper to be generic. It is fine to have focused helpers:

- `format_keyword_head`
- `format_keyword_with_space_after`
- `format_open_delimiter`
- `format_close_delimiter`
- `format_separator`
- `format_statement_terminator`

The key requirement is that source tokens flow into the helper rather than being
flattened to text at the rule site.

Inline token runs such as qualified names may need focused helpers that place
comments around `.` without forcing ordinary leading-comment layout. Those
helpers should still be token-driven and should document their comment placement
policy.

## Reordered Token Policy

Some formatter policies reorder source tokens or constructs. Their comments need
an ownership rule before migration:

- Whole constructs, such as imports and module directives, may move with a
  `CommentMap` envelope around a token-aware inner document.
- Sorted modifiers should not freely reorder across comment-bearing modifier
  entries. Split modifier lists into runs at entries with leading or trailing
  comments, sort only comment-free runs, and emit comment-bearing entries in
  their original relative positions with token-aware formatting.
- Removed or merged tokens must pass their comments through a named relocation
  helper, such as `format_removed_token_comments`; do not rely on trivia
  disappearance as an implementation detail.
- Delimiter dangling comments are owned by delimiter/list helpers.
- Formatter-ignore comments are owned by raw-range preservation.

## Syntax Accessor Requirements

Some rules currently synthesize tokens because the typed wrapper does not expose
the token. Before fixing a formatter rule with raw child scans, add a small
grammar-role accessor in `jolt_java_syntax`.

Accessors likely needed:

- package `package` keyword and semicolon,
- import `import`, `static`, `module`, star, and semicolon tokens,
- import on-demand dot token before `*`,
- annotation `@` token,
- annotation element-value-pair `=` token,
- annotation element `default` keyword,
- type declaration kind keywords: `class`, `interface`, `record`, `enum`, `@`,
  `interface`,
- constructor, method, and annotation element paren tokens,
- wildcard `?` token and `extends`/`super` bound keyword,
- type parameter bound `extends` keyword token,
- class type segment separator dot tokens, including nested `NameSyntax` dots
  inside class type segments,
- array dimension open and close bracket tokens,
- class literal dot and `class` token,
- label colon token,
- assert detail colon token,
- enhanced-for colon token,
- switch label `case`, `default`, `when`, colon, and rule expression semicolon,
- `SwitchExpression` keyword/open/close paren tokens,
- `SwitchBlock` open and close brace tokens,
- catch parameter or catch clause open and close paren tokens,
- lambda parameter-list open/close paren tokens and comma entries,
- lambda, formal parameter, and record component ellipsis tokens,
- object creation qualifier dot token,
- static initializer `static` keyword token,
- module declaration `open`, `module`, braces, directive keywords, modifiers,
  connective keywords, and semicolons,
- constructor invocation target dot tokens.

## Migration Checklist

Use this checklist as the current complete migration seed for
`crates/jolt_java_fmt/src`. When new formatter rules are added, extend this list
before relying on broad regression tests.

Each migration item has three completion gates:

1. any missing typed syntax accessors exist,
2. the formatter site uses source-token helpers rather than reconstructed text,
3. a marker regression fixture covers the syntax shape or an existing fixture is
   identified as covering it.

### Shared Helpers

- [ ] Add token-emission mode types and helpers.
- [ ] Replace direct calls to `format_token_text(token.text())` at rule sites
      with a mode-aware token helper.
- [ ] Keep `format_token_text` private to the token helper layer if possible.
- [ ] Add a required-token helper that renders missing recovered tokens as holes
      rather than fallback Java tokens.
- [ ] Add an explicit inserted-policy-token helper for deliberate formatter
      insertions such as readability parentheses or trailing separators.
- [ ] Add punctuation helpers for open/close delimiters, separators, and
      terminators.
- [ ] Add a helper or policy for raw combined literals so `".*"`, `"[]"`,
      `"()"`, `".class"`, `"switch ("`, and similar strings cannot appear when
      source tokens are available.
- [ ] Update `format_separator_with_comments` to use the new token helper.
- [ ] Update `format_statement_semicolon` and for-header semicolon handling to
      share the same terminator policy.
- [ ] Keep `format_removed_token_comments` for syntax that is intentionally
      removed, and audit every caller.

### Raw Text Audit

- [ ] Audit every `text("...")` call containing Java keywords, punctuation, or
      combined token text.
- [ ] Audit every `format_token_text(token.text())` call outside the shared
      token helper layer.
- [ ] Audit every literal containing multiple source tokens, including `".*"`,
      `"[]"`, `"()"`, `"{}"`, `".class"`, `" -> "`, `" : "`, `"switch ("`, and
      keyword-plus-space strings.
- [ ] For each raw literal, classify it as structural whitespace, explicit
      formatter policy insertion, unreachable diagnostic fallback, or a
      migration bug.
- [ ] Add missing syntax accessors before replacing raw literals that represent
      source tokens.

### Comment Ownership Audit

- [ ] Audit `CommentMap` users and keep it limited to moved whole-construct
      envelopes.
- [ ] Audit delimiter dangling comment helpers and make their ownership rules
      explicit.
- [ ] Audit removed-token helpers and require every dropped token to either have
      no comments or route comments through the helper.
- [ ] Audit sorted modifiers and implement the comment-bearing-entry barrier
      policy.
- [ ] Audit enum separator comment movement and document why each moved comment
      changes owner.
- [ ] Audit formatter-ignore raw ranges and keep formatter-control comments out
      of ordinary token rendering.
- [ ] Add a debug-only or test-only helper to count `JOLT-TRIVIA:*` markers in
      input and output until the preservation harness is green.

### Program And Imports

- [ ] `rules/program.rs`: format package keyword from its token.
- [ ] `rules/program.rs`: format package semicolon from its token.
- [ ] `rules/program.rs`: preserve comments before the package declaration when
      the file is not comment-only.
- [ ] `rules/program.rs`: keep package annotations token-aware.
- [ ] `rules/imports.rs`: format `import` from its token.
- [ ] `rules/imports.rs`: format `module` and `static` from their tokens.
- [ ] `rules/imports.rs`: format on-demand imports from the source dot and `*`
      tokens instead of appending `.*`.
- [ ] `rules/imports.rs`: format import semicolons from their tokens.
- [ ] `rules/imports.rs`: keep `CommentMap` only for the reorder envelope;
      preserve interior import trivia token-by-token.

### Names, Annotations, And Modifiers

- [ ] `rules/names.rs`: route all identifier tokens through the shared token
      helper.
- [ ] `rules/names.rs`: keep dot-token aware formatting for FQN crevices.
- [ ] `helpers/names.rs`: either keep only for synthetic dot joins or replace
      usages with token-aware name formatting.
- [ ] `rules/annotations.rs`: format `@` from its token.
- [ ] `rules/annotations.rs`: format annotation pair names with token comments.
- [ ] `rules/annotations.rs`: format annotation `=` from its token if exposed.
- [ ] `rules/modifiers.rs`: split sorted modifier lists at comment-bearing
      entries and sort only comment-free runs.
- [ ] `helpers/modifiers.rs`: replace local leading/trailing handling with the
      shared token helper.
- [ ] `helpers/modifiers.rs`: document the policy for comments attached to
      reordered modifier entries.

### Types

- [ ] `rules/types.rs`: format primitive and `void` keywords through the shared
      token helper.
- [ ] `rules/types.rs`: make class type segment joins dot-token aware at both
      class-type segment boundaries and nested `NameSyntax` boundaries.
- [ ] `rules/types.rs`: format type parameter names with token comments.
- [ ] `rules/types.rs`: format `extends` in type parameter bounds from its
      token.
- [ ] `rules/types.rs`: format `&` and `|` separators through shared separator
      helpers.
- [ ] `rules/types.rs`: format wildcard `?` from its token.
- [ ] `rules/types.rs`: format wildcard `extends`/`super` from their tokens.
- [ ] `rules/types.rs`: format array dimension `[` and `]` from their tokens;
      remove synthesized `[]` for source dimensions.
- [ ] `rules/types.rs`: preserve comments between varargs `...` and parameter
      names by formatting the ellipsis token when exposed.
- [ ] `rules/patterns.rs`: format match-all `_` through the shared token helper.

### Declarations

- [ ] `rules/declarations/type_declarations.rs`: format `class`, `interface`,
      `record`, `enum`, and `@interface` from source tokens.
- [ ] `rules/declarations/type_declarations.rs`: format type declaration names
      with token comments.
- [ ] `rules/declarations/type_declarations.rs`: preserve comments between a
      type keyword and the name.
- [ ] `rules/declarations/type_declarations.rs`: format type declaration body
      braces from source tokens.
- [ ] `rules/declarations/type_declarations.rs`: keep header clause keywords
      token-aware and migrate them to shared helpers.
- [ ] `helpers/blocks.rs`: replace synthesized block/body braces in
      `braced_block`, `braced_body`, and `empty_block` with helpers that can
      accept source brace tokens.
- [ ] `rules/statements/blocks.rs`: preserve full open/close brace-token trivia
      for blocks, not only open trailing and close leading comments.
- [ ] `rules/statements/switches.rs`: format switch block braces from source
      tokens rather than via tokenless `braced_block`.
- [ ] `rules/declarations/callables.rs`: format constructor names with token
      comments.
- [ ] `rules/declarations/callables.rs`: format compact constructor names with
      token comments.
- [ ] `rules/declarations/callables.rs`: format method names with token
      comments.
- [ ] `rules/declarations/callables.rs`: format annotation element names with
      token comments.
- [ ] `rules/declarations/callables.rs`: keep constructor and method parameter
      parens token-aware through the shared delimiter helper.
- [ ] `rules/declarations/callables.rs`: format annotation element parens from
      source tokens instead of synthesized `()`.
- [ ] `rules/declarations/callables.rs`: format annotation element `default`
      keyword from its token.
- [ ] `rules/declarations/callables.rs`: keep `throws` token-aware and migrate
      to shared keyword helpers.
- [ ] `rules/declarations/enums.rs`: format enum constant names with token
      comments using the shared helper.
- [ ] `rules/declarations/enums.rs`: format enum body semicolons from source
      tokens.
- [ ] `rules/declarations/member_bodies.rs`: format `static` initializer keyword
      from its token.
- [ ] `rules/declarations/constructor_bodies.rs`: format explicit constructor
      invocation target tokens with comments.
- [ ] `rules/declarations/constructor_bodies.rs`: format target dots from their
      source tokens.

### Variables And Parameters

- [ ] `rules/variables.rs`: preserve construct-leading comments for field
      declarations.
- [ ] `rules/variables.rs`: format `var` through the shared token helper.
- [ ] `rules/variables.rs`: format ellipsis tokens for varargs once exposed.
- [ ] `rules/variables.rs`: format commas through the shared separator helper.
- [ ] `rules/variables.rs`: keep initializer operators token-aware through the
      shared helper.
- [ ] `rules/variables.rs`: preserve comments between type and variable name
      without relying on ad hoc spaces.
- [ ] `rules/variables.rs`: keep receiver parameter dot and `this` token-aware.

### Expressions

- [ ] `rules/expressions/leaves.rs`: migrate `format_leaf_token` to the shared
      token helper.
- [ ] `rules/expressions/leaves.rs`: format class literal dot and `class` from
      source tokens instead of `text(".class")`.
- [ ] `rules/expressions/calls.rs`: keep argument parens token-aware through the
      shared delimiter helper; `text("()")` must not be used to repair a missing
      `ArgumentList` in recovered syntax.
- [ ] `rules/expressions/calls.rs`: keep method names token-aware.
- [ ] `rules/expressions/member_chains.rs`: migrate member dot handling to the
      shared punctuation helper.
- [ ] `rules/expressions/method_references.rs`: migrate `::` handling to the
      shared punctuation helper.
- [ ] `rules/expressions/method_references.rs`: keep `new` token-aware.
- [ ] `rules/expressions/arrays_objects.rs`: format qualifier dots from source
      tokens.
- [ ] `rules/expressions/arrays_objects.rs`: keep `new` token-aware through the
      shared keyword helper.
- [ ] `rules/expressions/arrays_objects.rs`: migrate bracket handling to shared
      delimiter helpers.
- [ ] `rules/expressions/arrays_objects.rs`: keep array initializer braces and
      commas on the existing list-helper path.
- [ ] `rules/expressions/casts_patterns.rs`: migrate cast parens to shared
      delimiter helpers.
- [ ] `rules/expressions/casts_patterns.rs`: keep `instanceof` token-aware
      through the shared keyword helper.
- [ ] `rules/expressions/lambdas.rs`: format concise lambda parameter names with
      token comments.
- [ ] `rules/expressions/lambdas.rs`: format lambda parameter parens and commas
      from source tokens.
- [ ] `rules/expressions/lambdas.rs`: format lambda arrows through a shared
      separator helper.
- [ ] `rules/expressions/lambdas.rs`: format `var` and ellipsis tokens through
      source tokens.
- [ ] `rules/expressions/operators.rs`: keep assignment, ternary, binary, unary,
      and postfix operators token-aware through the shared helper.
- [ ] `rules/expressions/operators.rs`: document inserted precedence parens as
      synthetic and separately audit any code path that removes original parens
      with comments.
- [ ] `rules/expressions/parenthesized.rs`: migrate paren handling to shared
      delimiter helpers.
- [ ] `rules/expressions/switches.rs`: replace synthesized `switch (`, `)`, and
      `{}` with token-aware formatting.

### Statements

- [ ] `rules/statements/simple.rs`: format label colon from its token.
- [ ] `rules/statements/simple.rs`: format assert detail colon from its token.
- [ ] `rules/statements/simple.rs`: migrate statement keyword helpers to the
      shared keyword API.
- [ ] `rules/statements/simple.rs`: keep jump labels token-aware.
- [ ] `rules/statements/simple.rs`: consolidate semicolon handling with the
      shared terminator helper.
- [ ] `rules/statements.rs`: preserve empty-statement and empty-statement-body
      comments explicitly instead of formatting them as tokenless empty blocks.
- [ ] `rules/statements/control_flow.rs`: migrate condition parens to shared
      delimiter helpers.
- [ ] `rules/statements/control_flow.rs`: format enhanced-for colon from its
      token.
- [ ] `rules/statements/control_flow.rs`: consolidate for-header semicolon
      handling with the shared terminator helper.
- [ ] `rules/statements/switches.rs`: format switch label `case`, `default`,
      `when`, colon, and arrow from source tokens.
- [ ] `rules/statements/switches.rs`: format rule expression semicolons from
      their source tokens or explicitly move comments when synthesized.
- [ ] `rules/statements/try_resources.rs`: keep resource specification parens on
      the existing source-token path while migrating them to shared delimiter
      helpers.
- [ ] `rules/statements/try_resources.rs`: format catch parameter parens from
      source tokens.
- [ ] `rules/statements/try_resources.rs`: keep catch union `|` token-aware
      through the shared separator helper.
- [ ] `rules/statements/try_resources.rs`: migrate `try`, `catch`, and `finally`
      to shared keyword helpers.
- [ ] `rules/statements/blocks.rs`: keep empty-statement and removed-token
      comment handling explicit.

### Modules

- [ ] `rules/modules.rs`: format module declaration `open`, `module`, `{`, and
      `}` from source tokens.
- [ ] `rules/modules.rs`: format directive keywords from source tokens:
      `requires`, `exports`, `opens`, `uses`, `provides`.
- [ ] `rules/modules.rs`: format directive modifiers `static` and `transitive`
      from source tokens.
- [ ] `rules/modules.rs`: format directive connective keywords `to` and `with`
      from source tokens.
- [ ] `rules/modules.rs`: format directive semicolons from source tokens.
- [ ] `rules/modules.rs`: keep `CommentMap` only for sorted directive envelopes;
      preserve interior directive trivia token-by-token.

## Rollout Order

1. Add syntax accessors and token helper API without changing behavior broadly.
2. Migrate statement terminators, separators, and delimiters first; they are the
   most reusable and easiest to verify.
3. Migrate leaf identifiers/keywords where the typed accessor already exposes
   source tokens.
4. Add missing syntax accessors for currently synthesized grammar tokens.
5. Migrate complex moved constructs last, keeping `CommentMap` as an envelope
   around token-aware inner formatting.
6. Run `cargo test -p jolt_java_fmt --test trivia_regressions` after each slice.
   The test must fail on dropped markers or fixture misconfiguration; do not add
   silent skips. If an allowlist is ever needed, it must be explicit, checked
   in, and shrink monotonically.
7. Promote representative marker fixtures into exact style fixtures only after
   the preservation harness is green.

## Review Checklist For Future Formatter Changes

- Did the rule use a real `JavaSyntaxToken` whenever one exists?
- Is every raw `text("keyword")` or `text("punctuation")` an explicit formatter
  policy insertion rather than syntax repair?
- If a token is removed or combined with another token, are its comments moved
  through an explicit helper?
- If a construct is reordered, are outer construct comments kept with the moved
  construct while interior comments remain token-aware?
- Does the trivia regression harness include a marker for the new syntax shape?
