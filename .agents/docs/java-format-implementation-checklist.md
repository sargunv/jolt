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

- `[~]` Build layer by layer rather than proof-grade vertical slices.
- `[~]` Start with tests that pin style-guide behavior.
- `[~]` Add missing CST accessors before printer workarounds.
- `[~]` Add reusable layout helpers before full rule implementation.
- `[ ]` Keep rendering bounded and linear.

## Target Layering

- `[~]` `jolt_java_syntax` owns parsing, CST shape, trivia, and accessors.
- `[~]` `jolt_java_fmt` owns Java policy and CST-to-document layout.
- `[x]` `jolt_fmt_ir` owns language-agnostic documents and rendering.
- `[~]` `jolt_fmt_core` owns public options, diagnostics, and language wiring.
- `[ ]` Java formatter rules avoid raw syntax structure when stable accessors
  should exist.

## Proposed Module Shape

- `[ ]` Add `context.rs`.
- `[x]` Add `format.rs`.
- `[~]` Add `rules/` modules.
- `[x]` Add `helpers/` modules.
- `[ ]` Preserve dependency direction: rules -> helpers -> IR and rules ->
  syntax accessors.

## Rule Authoring Shape

- `[ ]` Define the rule dispatch API.
- `[ ]` Define `JavaFormatter` context.
- `[ ]` Implement representative declaration-style rules with named helpers.
- `[ ]` Keep rules free of output string assembly.

## Test Strategy

- `[x]` Add focused rule fixture runner.
- `[~]` Add program/import/module/name/comment fixtures.
- `[~]` Add declaration/type/parameter fixtures.
- `[~]` Add statement/block/switch/try fixtures.
- `[x]` Add expression/operator/call/chain/lambda fixtures.
- `[x]` Assert every rule fixture formats successfully.
- `[x]` Assert every rule fixture equals expected output.
- `[x]` Assert formatting expected fixtures is unchanged.
- `[x]` Assert repeated formatting is deterministic.
- `[x]` Add imported-corpus formatter idempotence tests.
- `[x]` Keep upstream outputs as references only, not pass/fail truth.

## Accessor Discipline

- `[ ]` Compilation-unit item order.
- `[~]` Import static/star/name/comment roles.
- `[~]` Module directive kind/name/target/comment roles.
- `[~]` Modifier keyword and annotation roles.
- `[~]` Declaration headers and bodies.
- `[x]` Constructor body accessors.
- `[~]` Statement body kind.
- `[x]` Switch labels/rules/guards.
- `[x]` For init/condition/update.
- `[x]` Try resources and optional trailing semicolon.
- `[x]` Pattern roles for type, record, component, and match-all patterns.
- `[ ]` Expression parent roles.
- `[ ]` Member-chain linearization.
- `[~]` Lambda parameter classification.
- `[ ]` Wildcard and unnamed `_` roles.

## Comment And Trivia Model

- `[~]` Syntax-owned raw comments, token/source spans, and trivia positions are
  exposed enough for formatting.
- `[ ]` Formatter-owned `CommentMap` classifies leading, trailing, and dangling
  comments.
- `[ ]` Comment placement is explicit for moved constructs.
- `[ ]` Sortable import and module directive comments are barriers for v1.

## Helper Organization

- `[~]` `comma_list`
- `[x]` `semicolon_list`
- `[x]` `parenthesized_list`
- `[x]` `angle_bracket_list`
- `[x]` `braced_block`
- `[ ]` `declaration_header`
- `[~]` `member_body`
- `[x]` `modifier_list`
- `[ ]` `annotation_group`
- `[ ]` `assignment_rhs`
- `[x]` `binary_chain`
- `[x]` `ternary_expression`
- `[x]` `argument_list`
- `[x]` `member_chain`
- `[x]` `qualified_name`
- `[~]` `line_comment`
- `[~]` `block_comment`
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
- `[~]` Add style-guide fixture files.

### 3. Accessor Pass

- `[ ]` Add all accessors listed under Accessor Discipline.

### 4. Low-Level Helpers

- `[~]` Broken closing delimiter on its own line.
- `[~]` Trailing separator policies.
- `[x]` Body blank-line capping.
- `[~]` Leading-operator chains with flat continuation alignment.
- `[x]` Break-all argument lists.
- `[~]` Ruff-shaped member-chain heads.

### 5. Program Layer

- `[x]` Final newline.
- `[x]` Package declarations.
- `[x]` Package annotations.
- `[~]` Import sorting with comment barriers.
- `[~]` Module directive sorting/grouping.
- `[x]` Redundant top-level semicolon removal.
- `[x]` Qualified names.
- `[~]` Literal/token leaves.

### 6. Declaration Layer

- `[~]` Modifier sorting.
- `[~]` Declaration/type-use annotation placement.
- `[~]` Class/interface/record/enum/annotation headers.
- `[x]` Structural type declaration `extends`/`implements`/`permits` clauses.
- `[ ]` Broken-header brace placement.
- `[~]` Body member category padding.
- `[~]` Parameter and record-component lists.
- `[x]` Varargs formal parameters and record components.
- `[x]` Inline formal-parameter and record-component annotations, including
  varargs type-use annotations before `...`.
- `[~]` `throws`.
- `[x]` Structural constructor bodies.
- `[x]` Compact record constructors.
- `[~]` Enum constants and trailing comma policy.
- `[x]` Enum constants with annotations, arguments, and class bodies.
- `[x]` Annotation interface elements and default values.
- `[x]` Type parameters, type arguments, wildcards, and annotated dimensions.
- `[~]` Type-body empty statement removal.

### 7. Statement Layer

- `[~]` Blocks and braced bodies.
- `[x]` Labels.
- `[~]` `if`/`else`.
- `[x]` Local class/interface declarations.
- `[x]` Loops and broken `for` headers.
- `[x]` Switch labels/rules/guards.
- `[x]` Structural `case` labels, `default`, `case null, default`, and switch
  guards.
- `[~]` `return`/`throw`/`yield`.
- `[x]` Try/catch/finally/resources.
- `[x]` Structural try-with-resources declarations and variable accesses.
- `[x]` Inline catch-parameter annotations and modifiers.
- `[x]` `assert`/`break`/`continue`.

### 8. Expression Layer

- `[~]` Parenthesized expressions.
- `[x]` Method references.
- `[~]` Binary/operator chains.
- `[~]` Ternaries.
- `[~]` Assignments.
- `[~]` Calls and break-all argument lists.
- `[~]` Member chains.
- `[~]` Lambdas.
- `[x]` Literal, name, `this`, `super`, and class-literal expression leaves.
- `[x]` Untyped, `var`, typed, annotated, final, and varargs lambda parameters.
- `[x]` Arrays/initializers.
- `[~]` Casts, `instanceof`, patterns, object creation, and type arguments.
- `[x]` Type, record, component, and match-all patterns in `instanceof` and
  switch labels.
- `[x]` Anonymous class bodies in object creation expressions.
- `[x]` Constructor type arguments in object creation expressions.

### 9. Comments And Ignore Hardening

- `[ ]` Leading/trailing/dangling comment classification.
- `[ ]` Comment placement for moved constructs.
- `[x]` Star-block comment normalization.
- `[ ]` `@formatter:off/on` raw range preservation.
- `[ ]` Text-block internal preservation.
- `[x]` Imported-corpus comment/idempotence tests.

## Definition Of Done Audit

- `[ ]` All style-guide rule fixtures pass.
- `[ ]` Every style-guide rule has one or more focused tests.
- `[ ]` Formatting expected fixtures is idempotent.
- `[x]` Imported Java fixture inputs format without formatter panics.
- `[x]` Formatted imported fixtures parse.
- `[x]` Repeated formatting is deterministic.
- `[~]` No parser-accepted syntax reaches an unimplemented formatter fallback.
- `[ ]` Code review can trace every formatting choice to the style guide or
  spec.
- `[ ]` Audit report links each definition-of-done item to tests, fixtures, or
  implementation evidence.

## Permanent Intentional Deviations

None.
