# Jolt Java Formatter Implementation Checklist

This checklist mirrors
[`java-format-implementation-spec.md`](java-format-implementation-spec.md). Keep
it current while implementing the Java layout builder. Record only permanent,
intentional design deviations from the spec here; temporary bootstrap work and
unfinished rules belong in the checklist status.

Status legend:

- `[ ]` not started
- `[~]` in progress
- `[x]` complete

## Build Philosophy

- `[x]` Build layer by layer rather than proof-grade vertical slices.
- `[x]` Start with tests that pin style-guide behavior.
- `[x]` Add missing CST accessors before printer workarounds.
- `[x]` Add reusable layout helpers before full rule implementation.
- `[x]` Keep rendering bounded and linear: group fit probes push the measured
  group as flat, treat nested groups as flat contents unless forced, and are
  covered by deep-nesting/current-group renderer tests.

## Target Layering

- `[x]` `jolt_java_syntax` owns parsing, CST shape, trivia, and accessors.
- `[x]` `jolt_java_fmt` owns Java policy and CST-to-document layout.
- `[x]` `jolt_fmt_ir` owns language-agnostic documents and rendering.
- `[x]` `jolt_fmt_core` owns public options, diagnostics, and language wiring.
- `[x]` Java formatter rules avoid raw syntax structure when stable accessors
  should exist; formatter-ignore token range math is helper-owned, qualified
  name separator dots/comments, contextual modifier entries, and typed modifier
  annotation splits use syntax accessors, and formatter context is threaded
  through declaration, statement, expression, type, variable, modifier,
  annotation, and pattern rule layers.

## Proposed Module Shape

- `[x]` Add `context.rs`.
- `[x]` Add `format.rs`.
- `[x]` Add `rules/` modules.
- `[x]` Add `helpers/` modules.
- `[x]` Preserve dependency direction: rules -> helpers -> IR and rules ->
  syntax accessors.

## Rule Authoring Shape

- `[x]` Define the rule dispatch API.
- `[x]` Define `JavaFormatter` context.
- `[x]` Implement representative declaration-style rules with named helpers.
- `[x]` Keep rules free of output string assembly.

## Test Strategy

- `[x]` Add focused rule fixture runner.
- `[x]` Add program/import/module/name/comment fixtures.
- `[x]` Add declaration/type/parameter fixtures.
- `[x]` Add statement/block/switch/try fixtures.
- `[x]` Add expression/operator/call/chain/lambda fixtures.
- `[x]` Assert every rule fixture formats successfully.
- `[x]` Assert every rule fixture equals expected output.
- `[x]` Assert formatting expected fixtures is unchanged.
- `[x]` Assert repeated formatting is deterministic.
- `[x]` Add imported-corpus formatter idempotence tests.
- `[x]` Keep upstream outputs as references only, not pass/fail truth.

## Accessor Discipline

- `[x]` Compilation-unit item order.
- `[x]` Import static/star/name/comment roles.
- `[x]` Module directive kind/name/target/comment roles.
- `[x]` Modifier keyword and annotation roles.
- `[x]` Declaration headers and bodies.
- `[x]` Formal parameter lists expose receiver-parameter entries.
- `[x]` Typed modifier lists expose declaration annotations separately from
  post-modifier type-use annotations.
- `[x]` Constructor body accessors.
- `[x]` Statement body kind.
- `[x]` Switch labels/rules/guards.
- `[x]` For init/condition/update.
- `[x]` Try resources and optional trailing semicolon.
- `[x]` Pattern roles for type, record, component, and match-all patterns.
- `[x]` Expression parent roles.
- `[x]` Member-chain linearization.
- `[x]` Lambda parameter classification.
- `[x]` Wildcard and unnamed `_` roles.

## Comment And Trivia Model

- `[x]` Syntax-owned raw comments, token/source spans, and trivia positions are
  exposed enough for formatting.
- `[x]` Formatter-owned `CommentMap` classifies leading, trailing, and
  delimiter-dangling comments. Generic construct-leading comment formatting and
  removed-token comment preservation are shared helpers, along with star-block
  and token-has-comment classification. Construct-leading comments for type
  declarations, formal parameters, record components, receiver parameters,
  type/permits header entries, module target lists, constructors, methods, and
  constructor invocations read through `CommentMap`.
- `[x]` Comment placement is explicit for moved constructs; sortable
  imports/modules treat leading comments as barriers, removed empty
  statements/declarations preserve comments, sorted modifiers preserve token
  comments, enum separator comments move when commas become semicolons, removed
  try-resource trailing semicolon comments are preserved, and broken header/list
  entries preserve leading and delimiter comments.
- `[x]` Sortable import and module directive comments are barriers for v1.

## Helper Organization

- `[x]` `comma_list`
- `[x]` `semicolon_list`
- `[x]` `parenthesized_list`
- `[x]` `angle_bracket_list`
- `[x]` `braced_block`
- `[x]` `declaration_header`
- `[x]` `member_body`
- `[x]` `modifier_list`
- `[x]` `annotation_group`
- `[x]` `assignment_rhs`
- `[x]` `binary_chain`
- `[x]` `ternary_expression`
- `[x]` `argument_list`
- `[x]` `member_chain`
- `[x]` `qualified_name`
- `[x]` `line_comment`
- `[x]` `block_comment`
- `[x]` `star_block_comment`

## Rule Implementation Order

### 1. Formatting Harness

- `[x]` Add `jolt_java_fmt::format_source`.
- `[x]` Wire Java formatting through `jolt_fmt_core::format_source`.
- `[x]` Convert `FormatOptions` to explicit `RenderOptions`.
- `[x]` Keep parse-error no-write behavior in the shared diagnostic policy.
- `[x]` Add tests that Java formatting no longer returns the unimplemented
  diagnostic once the layout builder is enabled.

### 2. Rule Fixture Harness

- `[x]` Add input/expected fixture runner.
- `[x]` Add idempotence assertions.
- `[x]` Add determinism assertions.
- `[x]` Add style-guide fixture files.

### 3. Accessor Pass

- `[x]` Add all accessors listed under Accessor Discipline.

### 4. Low-Level Helpers

- `[x]` Broken closing delimiter on its own line.
- `[x]` Trailing separator policies.
- `[x]` Body blank-line capping.
- `[x]` Leading-operator chains with flat continuation alignment.
- `[x]` Break-all argument lists.
- `[x]` Ruff-shaped member-chain heads.

### 5. Program Layer

- `[x]` Final newline.
- `[x]` Package declarations.
- `[x]` Package annotations.
- `[x]` Import sorting with comment barriers.
- `[x]` Module directive sorting/grouping with comment barriers.
- `[x]` Redundant top-level semicolon removal.
- `[x]` Qualified names, including block comments around dots and line-comment
  forced leading-dot continuation.
- `[x]` Literal/token leaves.

### 6. Declaration Layer

- `[x]` Modifier sorting, including contextual `sealed` and `non-sealed` type
  modifiers and comments attached to sorted modifiers.
- `[x]` Declaration/type-use annotation placement.
- `[x]` Post-modifier field and method return type-use annotations remain inline
  with the type.
- `[x]` Method return annotations after type parameters remain inline with the
  return type.
- `[x]` Ambiguous no-modifier typed-declaration annotations have final
  style/accessor policy.
- `[x]` Class/interface/record/enum/annotation headers.
- `[x]` Structural type declaration `extends`/`implements`/`permits` clauses.
- `[x]` Broken-header brace placement.
- `[x]` Body member category padding.
- `[x]` Parameter and record-component lists.
- `[x]` Receiver parameters in method and constructor parameter lists.
- `[x]` Varargs formal parameters and record components.
- `[x]` Inline formal-parameter and record-component annotations, including
  varargs type-use annotations before `...`.
- `[x]` `throws`.
- `[x]` Structural constructor bodies.
- `[x]` Compact record constructors.
- `[x]` Enum constants and trailing comma policy.
- `[x]` Enum constants with annotations, arguments, and class bodies.
- `[x]` Annotation interface elements and default values.
- `[x]` Annotation-interface nested type members.
- `[x]` Type parameters, type arguments, wildcards, and annotated dimensions.
- `[x]` Type-body empty statement removal.

### 7. Statement Layer

- `[x]` Blocks and braced bodies.
- `[x]` Labels.
- `[x]` `if`/`else`.
- `[x]` Local class/interface declarations.
- `[x]` Loops and broken `for` headers.
- `[x]` Switch labels/rules/guards.
- `[x]` Structural `case` labels, `default`, `case null, default`, and switch
  guards; single-block colon switch groups keep the block with the label.
- `[x]` `return`/`throw`/`yield`.
- `[x]` Try/catch/finally/resources.
- `[x]` Structural try-with-resources declarations and variable accesses.
- `[x]` Inline catch-parameter annotations and modifiers.
- `[x]` `assert`/`break`/`continue`.

### 8. Expression Layer

- `[x]` Parenthesized expressions.
- `[x]` Method references.
- `[x]` Binary/operator chains.
- `[x]` Ternaries.
- `[x]` Assignments.
- `[x]` Calls and break-all argument lists, including blank-line normalization
  inside argument lists.
- `[x]` Member chains, including complex-receiver continuation and blank-line
  normalization inside chains.
- `[x]` Lambdas.
- `[x]` Literal, name, `this`, `super`, and class-literal expression leaves.
- `[x]` Untyped, `var`, typed, annotated, final, and varargs lambda parameters.
- `[x]` Arrays/initializers, including compact empty expression/list
  initializers.
- `[x]` Casts, `instanceof`, patterns, object creation, and type arguments.
- `[x]` Type, record, component, and match-all patterns in `instanceof` and
  switch labels.
- `[x]` Anonymous class bodies in object creation expressions.
- `[x]` Constructor type arguments in object creation expressions.

### 9. Comments And Ignore Hardening

- `[x]` Leading/trailing/dangling comment classification; delimiter dangling
  comments, token comment checks, star-block checks, and removed-token comment
  preservation are helper-owned.
- `[x]` Comment placement for moved constructs.
- `[x]` Star-block comment normalization.
- `[x]` `@formatter:off/on` raw range preservation across top-level, module
  directive, type-member, constructor-body, and block-statement sequences.
- `[x]` Unsupported branded ignore spellings are treated as ordinary comments.
- `[x]` Text-block internal preservation.
- `[x]` Imported-corpus comment/idempotence tests.

## Definition Of Done Audit

- `[x]` All style-guide rule fixtures pass.
- `[x]` Every style-guide rule has one or more focused tests.
- `[x]` Formatting expected fixtures is idempotent.
- `[x]` Imported Java fixture inputs format without formatter panics.
- `[x]` Formatted imported fixtures parse.
- `[x]` Repeated formatting is deterministic.
- `[x]` No parser-accepted syntax reaches an unimplemented formatter fallback
  (declaration method/constructor header and record-component-list comment
  fallbacks removed; variable-declarator-list comment fallback removed;
  lambda-parameter comment fallback removed; argument-list comment fallback
  removed; method-reference comment fallback removed; statement-expression-list
  comment fallback removed; catch-parameter comment fallback removed;
  declaration recovery branches remain only behind the public formatter's
  non-clean parse gate and are covered by
  `declaration_recovery_nodes_do_not_reach_layout`).
- `[x]` Code review can trace every formatting choice to the style guide or
  spec.
- `[x]` Audit report links each definition-of-done item to tests, fixtures, or
  implementation evidence.

## Permanent Intentional Deviations

None.
