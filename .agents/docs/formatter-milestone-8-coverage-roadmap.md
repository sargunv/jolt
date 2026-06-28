# Formatter Milestone 8 Coverage Roadmap

Milestone 8 is not complete until Jolt can format any valid Java source accepted
by `jolt_java_syntax` without returning `java.format.missing_layout_rules`.

The current parser/input layer is not the blocker. The pinned google-java-format
oracle corpus already round-trips through the parser, and the formatter oracle
snapshot currently reports:

- total considered: 209,
- invalid upstream fixtures skipped: 1,
- parse blocked: 0,
- missing-rule blocked: 187,
- other blocked: 0,
- formatted: 21,
- exact matches: 0,
- aggregate diff size: 711.

That means the current failure is formatter coverage. The layout builder still
contains broad unsupported-shape guards and explicit `missing_layout` exits for
valid Java syntax. Those exits are useful while developing because they prevent
partial output, but every valid-Java `missing_layout` path must be removed
before Milestone 8 is done.

## Completion Bar

Coverage and oracle compatibility are related but separate:

- Coverage: every valid Java source that parses cleanly formats without
  `java.format.missing_layout_rules`.
- Safety: parse diagnostics still block formatting without output.
- Rendering: render failures are internal bugs, not acceptable blocked output.
- Oracle compatibility: the pinned valid google-java-format corpus reaches 100%
  exact matches for the Google profile.
- Source inventory: every `missing_layout` call site is either removed, narrowed
  to invalid/unreachable parser-clean structure, or covered by a test proving
  the input is not valid clean Java.

The source inventory criterion is required because the oracle corpus is finite.
Passing the pinned corpus is not the same as being able to format any valid Java
source.

## Current Blocker Buckets

Current blocked diagnostics from
`.oracles/reports/java/google-java-format/google/`:

| Count | First blocker                              |
| ----: | ------------------------------------------ |
|    35 | method declaration shapes                  |
|    20 | block statement shapes                     |
|    19 | multiline block comments                   |
|    16 | declaration annotations                    |
|    10 | unhandled comment or ignored trivia        |
|    10 | nested class declarations                  |
|     7 | enum declarations                          |
|     6 | non-own-line leading comments              |
|     6 | interface declarations                     |
|     5 | field declaration shapes                   |
|     6 | annotation interface declarations          |
|     4 | type shapes                                |
|     4 | method invocation shapes                   |
|     4 | multiline literals                         |
|     4 | lambda expressions                         |
|     3 | method invocation receivers                |
|     3 | module declarations                        |
|     3 | method reference expressions               |
|     3 | array creation expressions                 |
|     2 | record declarations                        |
|     2 | nested interface declarations              |
|     2 | nested enum declarations                   |
|     2 | class type parameters                      |
|     1 | local variable declaration shape           |
|     1 | constructor declaration shape              |
|     1 | package annotations                        |
|     1 | nested record declarations                 |
|     1 | object creation expressions                |
|     1 | conditional expressions                    |
|     1 | class literal expressions                  |
|     1 | array access expressions                   |
|     1 | contextual class modifiers                 |
|     1 | compact compilation-unit field declaration |
|     1 | compact compilation-unit empty declaration |

This table is "first blocker per file", not total missing syntax. Removing one
large blocker will expose deeper blockers in the same files. The roadmap should
therefore be driven by both this histogram and the source-level `missing_layout`
inventory.

## Source-Level Audit

### Global Preflight Blockers

Completed cleanup:

- the whole-file descendant scan for `Annotation` was removed,
- the whole-file descendant scan for `EmptyDeclaration` was removed,
- class-body empty declarations now format as `;`,
- package annotations now fail at the package declaration rule,
- declaration annotations now fail at the owning modifier-list rule.

Remaining compilation-unit preflight blockers are local to direct compilation
unit structure:

- module declarations,
- compact compilation-unit children such as `FieldDeclaration` and
  `EmptyDeclaration`.

### Compilation Units And Imports

Missing or incomplete:

- module declarations,
- compact compilation units with top-level fields/methods/classes,
- malformed import shape diagnostics should remain unreachable for parser-clean
  valid Java, while ordinary single/static/type-on-demand imports must format.

Roadmap:

1. Add module declaration and directive formatting.
2. Add compact compilation-unit member formatting through the same declaration
   and member rules used inside classes.
3. Keep import order policy separate, but make every valid import declaration
   format.

### Type Declarations

Currently supported narrowly:

- ordinary classes with simple headers and bodies.

Missing or incomplete:

- class type parameters,
- `extends`, `implements`, and `permits`,
- records and record components,
- enums, enum constants, enum class bodies,
- interfaces,
- annotation interfaces,
- nested class/record/enum/interface/annotation declarations,
- local class/interface declarations,
- compact constructors.

Roadmap:

1. Remove top-level/nested declaration asymmetry. Nested and local declarations
   should reuse the same declaration formatters with only placement-specific
   wrapping.
2. Implement class header clauses and type parameters before records/enums,
   because records, interfaces, methods, constructors, and annotation types all
   depend on type parameter and type-list formatting.
3. Implement interfaces and annotation interfaces.
4. Implement records, including record components, compact constructors, and
   canonical constructors.
5. Implement enums after class/interface bodies can handle mixed constants,
   fields, methods, constructors, and nested declarations.
6. Implement module declarations once the declaration-list machinery is stable.

### Modifiers And Annotations

Current blockers:

- global annotation preflight,
- declaration annotations in modifier lists,
- contextual modifiers,
- type-use annotations.

Missing or incomplete:

- marker, single-member, and normal annotations,
- annotation element values,
- annotation arrays and nested annotations,
- annotations in modifiers, types, dimensions, type parameters, record
  components, receiver parameters, casts, patterns, and package declarations,
- contextual modifiers such as `sealed`, `non-sealed`, and `permits`-related
  forms where they belong.

Roadmap:

1. Add an annotation formatter independent of declaration kind.
2. Replace `format_modifier_list` returning raw tokens with a doc-producing
   modifier/annotation formatter.
3. Support type-use annotations in the type formatter and dimension formatter.
4. Only after annotations format locally, remove the global annotation
   preflight.

### Types

Current type formatting is simple-token based.

Missing or incomplete:

- primitive and void types in every grammar position,
- qualified class/interface types with type arguments,
- nested generic types and split `>` token handling,
- wildcard type arguments and bounds,
- arrays and annotated dimensions,
- varargs dimensions,
- union and intersection types,
- `var` local variable types,
- receiver parameter types,
- class literal pseudo-types.

Roadmap:

1. Replace `simple_layout_tokens` as the main type path with structured type
   formatting.
2. Add reusable comma-list, type-argument-list, bound-list, and dimension-list
   helpers.
3. Format arrays/dimensions as part of both types and declarators, since Java
   allows dimensions after the type and after the variable name.
4. Add narrow tests for nested generic close tokens to protect parser/CST
   semantics while formatting.

### Class Bodies And Members

Current blockers:

- nested declarations,
- compact constructors,
- field declaration shapes,
- method declaration shapes,
- constructor declaration shapes.

Missing or incomplete:

- initializer blocks,
- empty semicolon declarations,
- multiple field/local declarators,
- array dimensions on declarators,
- method parameters, receiver parameters, varargs, type parameters, throws
  clauses, trailing dimensions,
- abstract/native methods and annotation elements without bodies,
- annotation type elements and defaults,
- field and local variable array initializers.

Roadmap:

1. Add shared method/constructor signature formatting: type parameters,
   parameters, receiver parameters, throws, varargs, and trailing dimensions.
2. Add method body vs semicolon body handling.
3. Add annotation elements and defaults.

### Statements

Current supported statements are a small subset:

- local variable declarations with simple shape,
- nested blocks,
- return, throw, yield,
- expression statements for a few expression kinds.

Missing or incomplete:

- empty statements,
- labeled statements,
- local declarations,
- `if`/`else`,
- `assert`,
- `switch` statements and rules/groups,
- `while`, `do`, basic `for`, enhanced `for`,
- `break` and `continue`,
- `synchronized`,
- `try`, `catch`, `finally`,
- try-with-resources,
- constructor invocations in constructor bodies.

Roadmap:

1. Add constructor invocation formatting before broader block work, because
   constructors are valid Java and currently blocked by constructor-body shape.
2. Implement simple control flow (`if`, loops, `break`, `continue`, labels,
   empty statements).
3. Implement try/catch/finally and try-with-resources after resource formatting
   exists.
4. Implement switch statements and switch expressions with one shared switch
   block formatter.
5. Add statement-body helpers so braced and unbraced bodies use one policy.

### Expressions

Current expression support is partial:

- literals except multiline literals,
- simple names,
- `this` and `super`,
- parenthesized expressions,
- field access with limited receivers,
- method invocation with limited receivers and arguments,
- unary, postfix, binary, assignment.

Missing or incomplete:

- object creation and anonymous classes,
- array creation, array access, and array initializers,
- class literals,
- casts,
- conditional expressions,
- lambdas,
- method references,
- `instanceof` expressions and patterns,
- switch expressions,
- qualified `this` and `super`,
- generic method invocations and explicit type arguments,
- broader method/field access receivers,
- expression names vs primary expressions in assignment/update operands,
- multiline text block literals.

Roadmap:

1. Add receiver-chain formatting so method invocation, field access, array
   access, qualified `this`, qualified `super`, and method references share one
   selector pipeline.
2. Add object creation and anonymous class bodies after class body/member
   formatting is reusable.
3. Add array access/creation/initializers and reuse list formatting.
4. Add casts and class literals with type formatting.
5. Add conditional, lambda, method reference, `instanceof`, patterns, and switch
   expression support.
6. Add multiline literal/text block formatting once comment/trivia placement can
   preserve multiline raw text safely.

### Comments And Trivia

Current comment support is intentionally narrow:

- own-line leading line comments,
- own-line single-line block/Javadoc comments,
- trailing line comments.

Current blockers:

- multiline block/Javadoc comments,
- non-own-line leading comments,
- trailing block comments,
- dangling comments in empty bodies/blocks/lists,
- comments inside parameter, argument, type argument, array initializer, and
  switch label lists,
- ignored trivia such as trailing SUB.

Roadmap:

1. Replace the single cursor with placement-aware comment attachment records:
   leading, trailing, dangling, inner, and list-item comments.
2. Support multiline block and Javadoc comments as preserved raw text with
   indentation normalization where required by google-java-format evidence.
3. Add dangling comments for empty class bodies, blocks, parameter lists,
   argument lists, array initializers, and switch blocks.
4. Add list-aware comment handling before parameters and arguments are expanded.
5. Decide and test the policy for ignored trivia; valid Java formatting should
   not be blocked by trailing ignored trivia unless preserving it would be
   destructive.

### Shape Guards And Accessors

Current formatter coverage is limited by boolean guards such as:

- `unsupported_layout_child`,
- `has_supported_layout_shape`,
- `has_single_declarator_layout_shape`,
- `has_expression_layout_shape`,
- `simple_layout_tokens`.

These guards were useful for safe scaffolding, but they must not remain as broad
coverage boundaries. They should evolve into one of:

- structured accessors used by complete formatting rules,
- local unreachable diagnostics for parser-clean impossible shapes,
- tests that demonstrate an unsupported shape is not valid Java.

Roadmap:

1. For each formatter family, replace the broad boolean guard with explicit
   child extraction and formatting.
2. Keep adding accessors in `jolt_java_syntax` as the formatter needs them.
3. Add tests for each accessor only when grounded in grammar behavior or a
   formatter rule, not as duplicate source-definition tests.
4. Track remaining guards in the coverage roadmap until none block valid Java.

## Recommended Implementation Order

The implementation order should maximize coverage while avoiding partial output.
After each step, run the full oracle harness and update the blocker histogram.

1. Improve reporting:
   - add a missing-rule histogram to the oracle report index,
   - optionally include the top buckets in the compact `insta` snapshot,
   - keep per-file diagnostics gitignored.
2. Continue localizing coarse blockers:
   - module declarations,
   - compact compilation-unit members,
   - remaining broad shape guards.
3. Comments baseline:
   - multiline block/Javadoc comments,
   - dangling comments in empty blocks/bodies/lists,
   - list-aware comment attachment.
4. Initializer and empty declarations:
   - compact compilation-unit members.
5. Signatures and declarators:
   - method/constructor parameters,
   - type parameters,
   - throws clauses,
   - receiver parameters,
   - multiple declarators and declarator dimensions.
6. Type formatter:
   - generics,
   - arrays,
   - wildcards,
   - union/intersection types,
   - type-use annotations.
7. Declarations:
   - interfaces,
   - annotation interfaces,
   - records,
   - enums,
   - nested and local declarations,
   - modules.
8. Statements:
   - constructor invocations,
   - control flow,
   - try/resources,
   - switch.
9. Expressions:
   - selector chains,
   - object/array creation,
   - class literals,
   - casts,
   - conditionals,
   - lambdas,
   - method references,
   - patterns and switch expressions,
   - multiline literals.
10. Compatibility pass:
    - with `missing-rule blocked: 0`, drive exact-match percentage to 100%,
    - fix policy diffs by descending aggregate diff size,
    - keep import sorting, modifier ordering, suppression comments, and range
      formatting explicitly scoped unless oracle evidence proves they are part
      of the required Google profile behavior.

## Tracking Checklist

Milestone 8 remains open until all of these are true:

- [ ] `missing-rule blocked: 0` for the pinned valid google-java-format corpus.
- [ ] `other blocked: 0` for the pinned valid google-java-format corpus.
- [ ] `formatted` equals the number of valid corpus files.
- [ ] exact matches are 100% for the pinned valid google-java-format corpus.
- [ ] every valid-Java `missing_layout` call site in `jolt_java_fmt` has been
      removed or proven unreachable for clean parser output.
- [ ] broad shape guards no longer block valid Java grammar families.
- [ ] parser diagnostics still block formatting without output.
- [ ] formatter output remains comment/trivia-accounted.
- [ ] targeted tests cover each Java grammar family, not only oracle fixtures.
- [ ] `mise run test` passes.
