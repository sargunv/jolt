# Kotlin Parser and Formatter Roadmap

This roadmap translates the existing Java syntax/formatter architecture and the
Kotlin grammar report into an implementation plan for Kotlin. The goal is a
lossless, current-Kotlin parser that can support formatter work immediately,
while keeping preview syntax visible and easy to audit.

## Inputs

- `crates/jolt_java_syntax`: reference implementation for the event-based
  parser, lazy token buffer, typed CST wrappers, diagnostics, recovery, and
  snapshot tests.
- `crates/jolt_kotlin_syntax`: existing Kotlin lexer, token inventory, language
  glue, and fixture corpus.
- `.agents/docs/kotlin-grammar-report.md`: Kotlin 2.4 parser reference,
  especially the ambiguity table, precedence ladder, syntax changelog, and
  validation plan.
- `fixtures/kotlin`: parser, trivia, style, property, and upstream-doc fixtures
  already shaped as the acceptance corpus.

## North Star

Build one formatter parser for the current Kotlin surface:

- accept Kotlin 2.4 stable syntax;
- also accept documented preview syntax needed by real code and fixtures,
  especially collection literals and name-based destructuring;
- do not use language-version flags in the formatter parser;
- mark preview-only productions in code comments and node names where useful;
- never synthesize source tokens for parser recovery when source tokens and
  trivia exist;
- keep disambiguation bounded and explicit.

This mirrors the Java crate's philosophy: produce a source-preserving CST first,
then add typed accessors only where formatting, traversal, recovery, or
ambiguity resolution benefits from them.

## Architecture To Mirror From Java

### Parser Shell

Create `crates/jolt_kotlin_syntax/src/parser` with the same high-level shape as
Java:

- `parser/mod.rs`
  - `KotlinParseDiagnosticCode`
  - `KotlinParse`
  - `parse_kotlin_file(source: &str) -> KotlinParse`
  - `finish_parse(...)`
- `parser/source.rs`
  - `Parser`
  - `TokenBuffer`
  - `TokenCursor`
  - `ParseEvents`
- `parser/grammar.rs`
  - shared stop-set utilities;
  - module includes for grammar families.

Do not copy Java-specific token splitting. Kotlin's generic/type-argument
ambiguity is not the Java `>>` problem. It should be handled with bounded
speculative parsing and cursor checkpoints.

### Token Buffer Requirements

The Kotlin buffer needs everything Java has, plus newline awareness:

- `current_kind`, `nth_kind`, `current_text`, `text_at`;
- `fork_cursor` / checkpoint / rewind for speculative type-argument parsing;
- `tokens_are_adjacent` for `!in`, `!is`, callable-reference diagnostics, and
  call-suffix checks;
- `newline_before_current()` and `newline_between(left, right)` derived from
  trivia, not from synthetic syntax tokens;
- semicolon helpers that recognize explicit `;`, `;;`, EOF, `}`, and legal
  newline boundaries without inserting an `EolOrSemicolon` token into the tree.

`KotlinSyntaxKind::EolOrSemicolon` currently exists but the lexer does not emit
it. Keep it that way initially: the lexer should preserve physical newlines as
trivia, while the parser decides whether a newline is semantically significant
in the current grammar context. Kotlin newlines can terminate statements in one
position, be ignored after `=`, `,`, or an open delimiter, or stop a
postfix/call chain depending on surrounding syntax. Emitting a semantic newline
token from the lexer would either be too context-free to be correct or would
push parser knowledge into the lexer. Parser-side newline queries keep the lexer
source-preserving, avoid synthetic semicolon tokens, and put the decision where
the grammar context is available.

### Diagnostics

Start with a small stable set:

- `kotlin.parse.expected_syntax`
- `kotlin.parse.unexpected_syntax`
- `kotlin.parse.invalid_assignment_target`
- `kotlin.parse.malformed_type_argument_list`
- `kotlin.parse.invalid_when_guard`
- `kotlin.parse.reserved_callable_reference_call`
- `internal.syntax.invalid_event_stream`

Add more only when a fixture requires a distinct error code or formatter
behavior benefits from the distinction.

### Grammar Modules

Use this bootstrap layout, with real submodules from the beginning:

- `grammar/file.rs`
- `grammar/annotations.rs`
- `grammar/declarations.rs`
- `grammar/declarations/classes.rs`
- `grammar/declarations/functions.rs`
- `grammar/declarations/properties.rs`
- `grammar/declarations/constructors.rs`
- `grammar/declarations/type_aliases.rs`
- `grammar/types.rs`
- `grammar/expressions.rs`
- `grammar/expressions/postfix.rs`
- `grammar/expressions/operators.rs`
- `grammar/expressions/lambdas.rs`
- `grammar/expressions/control_flow.rs`
- `grammar/expressions/literals.rs`
- `grammar/statements.rs`
- `grammar/strings.rs`
- `grammar/support/mod.rs`
- `grammar/support/identifiers.rs`
- `grammar/support/lookahead.rs`
- `grammar/support/recovery.rs`
- `grammar/support/semi.rs`
- `grammar/support/token_sets.rs`

This intentionally mirrors Java only at the public entrypoint level. Do not let
`declarations.rs` or `expressions.rs` become large staging files that later need
to be split under pressure. One lesson from the Java parser is that early file
coalescing makes later work more expensive: helper visibility, local invariants,
test failures, and formatter-driven changes all become tangled once several
grammar families share one file.

Create or keep a separate grammar file as soon as one of these is true:

- a construct has distinct recovery behavior;
- a construct needs newline-sensitive or speculative parsing;
- a construct has enough helper predicates to form its own local vocabulary;
- formatter work is likely to iterate on the construct independently;
- comments/trivia attach differently from the surrounding production;
- a preview feature should remain easy to find and audit.

The parent files should mostly dispatch and share small orchestration helpers.
Kotlin's soft-keyword, newline, and lookahead logic should still live in support
modules instead of being copied into every grammar file.

## SyntaxKind Expansion

`KotlinSyntaxKind` currently contains tokens plus `ErrorNode` and `KotlinFile`.
Expand it in parser-driven batches. Do not attempt a one-to-one copy of every
Kotlin grammar production.

### Batch 1: File And Names

- `PackageHeader`
- `ImportList`
- `ImportDirective`
- `ImportAlias`
- `ModifierList`
- `Annotation`
- `AnnotationUseSiteTarget`
- `AnnotationArgumentList`
- `ValueArgumentList`
- `ValueArgument`
- `Name`
- `QualifiedName`
- `TypeArgumentList`
- `TypeArgument`

### Batch 2: Declarations

- `ClassDeclaration`
- `InterfaceDeclaration`
- `ObjectDeclaration`
- `CompanionObject`
- `EnumEntry`
- `ClassBody`
- `ClassMemberDeclaration`
- `PrimaryConstructor`
- `SecondaryConstructor`
- `ConstructorDelegationCall`
- `InitializerBlock`
- `FunctionDeclaration`
- `PropertyDeclaration`
- `PropertyAccessor`
- `ExplicitBackingField`
- `TypeAliasDeclaration`
- `TypeParameterList`
- `TypeParameter`
- `TypeConstraintList`
- `TypeConstraint`
- `ContextParameterClause`
- `ContextParameter`
- `DelegationSpecifierList`
- `DelegationSpecifier`

### Batch 3: Types

- `UserType`
- `NullableType`
- `FunctionType`
- `ContextFunctionType`
- `ReceiverType`
- `ParenthesizedType`
- `DefinitelyNonNullableType`
- `TypeProjection`
- `TypeProjectionList`

### Batch 4: Expressions And Statements

- `Block`
- `Statement`
- `ExpressionStatement`
- `LocalDeclaration`
- `AssignmentExpression`
- `BinaryExpression`
- `UnaryExpression`
- `PostfixExpression`
- `CallExpression`
- `IndexExpression`
- `NavigationExpression`
- `CallableReferenceExpression`
- `LiteralExpression`
- `StringTemplateExpression`
- `StringTemplateEntry`
- `NameExpression`
- `ThisExpression`
- `SuperExpression`
- `ParenthesizedExpression`
- `IfExpression`
- `WhenExpression`
- `WhenSubject`
- `WhenEntry`
- `WhenCondition`
- `WhenGuard`
- `TryExpression`
- `CatchClause`
- `FinallyClause`
- `LoopStatement`
- `ForStatement`
- `WhileStatement`
- `DoWhileStatement`
- `JumpExpression`
- `ThrowExpression`
- `LambdaExpression`
- `LambdaParameterList`
- `LambdaParameter`
- `AnonymousFunctionExpression`
- `ObjectExpression`
- `CollectionLiteralExpression`
- `DestructuringDeclaration`
- `DestructuringEntry`

Preview nodes should be named plainly, with comments near parser entrypoints
noting the feature status. Avoid names that encode temporary compiler flags.

## Parser Phases

Each phase should end with source reconstruction checks and `insta` snapshots
for the relevant `fixtures/kotlin` subset.

### Phase 0: Test Harness And Public API

Deliverables:

- add `jolt_test_support::kotlin_fixture_root` and `collect_kotlin_files`;
- add `jolt_kotlin_syntax/tests/corpus.rs` mirroring Java's syntax snapshot
  test;
- add `parse_kotlin_file` public API returning `KotlinParse`;
- snapshot lexer-only fixtures through the parser wrapper but skip source-tree
  reconstruction assertions for lexer-only cases if parse diagnostics are
  expected.

Acceptance:

- fixture directory must be required, not silently skipped;
- every fixture produces a stable debug snapshot;
- valid parsed trees reconstruct `source_text()` exactly.

### Phase 1: Parser Skeleton And File Structure

Implement:

- event stream creation and syntax tree building;
- file annotations, optional package header, import list, top-level declaration
  dispatch, and EOF handling;
- qualified names that allow soft keywords where Kotlin does;
- semicolon/newline boundary helpers;
- top-level recovery to the next declaration, import, semicolon, newline
  boundary, or EOF.

Fixtures to target:

- `syntax/parser/parses-file-annotations-package-and-imports.kt`
- `syntax/parser/parses-file-with-shebang-and-package.kt`
- `syntax/parser/parses-import-*.kt`
- `style/program/*.kt`
- `trivia/package-and-imports.kt`

### Phase 2: Annotations, Modifiers, And Soft Keywords

Implement:

- annotation entries and use-site targets, including `@all:`;
- annotation arguments and array-like annotation values;
- hard keyword, soft keyword, and modifier keyword predicates;
- modifier lists for declarations and type-use annotations.

Fixtures to target:

- `syntax/parser/parses-annotation-*.kt`
- `syntax/parser/parses-modifier-*.kt`
- `style/declarations/annotations-and-modifiers.kt`
- `trivia/annotations-use-site-targets.kt`

### Phase 3: Types

Implement:

- user types, qualified type segments, type arguments, and projections;
- nullable types and definitely non-nullable types;
- function types, receiver function types, suspend function types, and context
  function types;
- type parameters and `where` constraints;
- bounded speculative parsing for `<...>` that can roll back when the sequence
  is really a comparison expression.

Fixtures to target:

- `syntax/parser/parses-type-*.kt`
- `syntax/parser/parses-class-header-type-parameters-and-where.kt`
- `style/declarations/type-parameters-where.kt`
- `trivia/nullable-and-function-types.kt`
- `trivia/type-parameters-and-where.kt`

### Phase 4: Declarations

Implement:

- classes, interfaces, data/value/sealed headers, primary constructors;
- secondary constructors and delegation calls;
- object declarations, data objects, companions, and object expressions;
- enum entries and enum entry bodies;
- functions, extension receivers, context parameter clauses, expression/block
  bodies;
- properties, delegates, accessors, explicit backing fields, and destructuring
  declarations;
- type aliases, including nested type aliases.

Fixtures to target:

- `syntax/parser/parses-class-*.kt`
- `syntax/parser/parses-interface-*.kt`
- `syntax/parser/parses-object-*.kt`
- `syntax/parser/parses-companion-*.kt`
- `syntax/parser/parses-constructor-*.kt`
- `syntax/parser/parses-function-*.kt`
- `syntax/parser/parses-property-*.kt`
- `syntax/parser/parses-type-alias-top-level.kt`
- `syntax/parser/parses-nested-type-alias.kt`
- `style/declarations/*.kt`
- `trivia/modifiers-and-classes.kt`
- `trivia/property-accessors-and-field.kt`
- `trivia/receiver-and-context-parameters.kt`

### Phase 5: Expressions And Precedence

Implement the precedence ladder from the grammar report:

1. postfix unary and call suffixes;
2. prefix unary;
3. `as` / `as?`;
4. multiplicative;
5. additive;
6. range `..` / `..<`;
7. infix function call;
8. Elvis `?:`;
9. `in` / `!in` / `is` / `!is`;
10. comparisons;
11. equality;
12. `&&`;
13. `||`;
14. assignment and compound assignment.

Special handling:

- postfix chains stop at significant newlines;
- call suffixes accept normal, named, spread, trailing-lambda, and explicit
  receiver/super forms;
- `foo::bar(args)` parses as a callable reference and emits the reserved-form
  diagnostic if `(` follows without a newline;
- collection literals parse only in atomic-expression position;
- `[` after an expression remains indexing;
- string template token sequences become structured string-template nodes, with
  long-template embedded expressions delegated back to expression parsing.

Fixtures to target:

- `syntax/parser/parses-expression-*.kt`
- `syntax/parser/parses-call-*.kt`
- `syntax/parser/parses-callable-reference-*.kt`
- `syntax/parser/parses-postfix-*.kt`
- `syntax/parser/parses-string-*.kt`
- `syntax/parser/parses-collection-literal-preview-*.kt`
- `style/expressions/*.kt`
- `properties/layout-fit-boundaries/*.kt`
- `trivia/call-chains-and-safe-calls.kt`
- `trivia/callable-references-and-labels.kt`
- `trivia/collection-and-indexing.kt`
- `trivia/operators-and-ranges.kt`
- `trivia/string-templates.kt`

### Phase 6: Control Flow And Statements

Implement:

- blocks and statement boundaries;
- `if`, `when`, `try`, loops, labels, returns, breaks, continues, and throws;
- `when` subject binding;
- `when` conditions and guard conditions;
- diagnostic for guard conditions without a subject;
- local declarations inside blocks and scripts.

Fixtures to target:

- `syntax/parser/parses-if-*.kt`
- `syntax/parser/parses-when-*.kt`
- `syntax/parser/parses-try-*.kt`
- `syntax/parser/parses-loop-*.kt`
- `syntax/parser/parses-jump-*.kt`
- `syntax/parser/parses-script-top-level-statements.kt`
- `style/statements/*.kt`
- `trivia/destructuring-and-loops.kt`
- `trivia/when-arrows-and-guards.kt`

### Phase 7: Recovery And Negative Fixtures

Make recovery boring and predictable:

- missing expression after Elvis;
- incomplete lambda/call;
- missing `when` arrow/body;
- malformed type-argument call;
- invalid assignment target;
- missing expression in collection literal.

Fixtures to target:

- `syntax/parser/diagnoses-*.kt`
- `syntax/parser/recovers-*.kt`

Acceptance:

- parser never aborts for malformed input;
- diagnostics point at source ranges that explain the local error;
- recovery produces `ErrorNode`s instead of dropping source tokens;
- source reconstruction remains exact.

## Typed Kotlin Nodes

Create `crates/jolt_kotlin_syntax/src/nodes` after the first parser snapshots
are useful. Follow Java's split:

- `nodes/mod.rs`
  - `KotlinSyntaxToken`
  - `KotlinComment`
  - node wrapper structs;
  - family enums such as `Declaration`, `Expression`, `Type`, `Statement`;
  - `cast_kotlin_file`.
- `nodes/accessors.rs`
  - only accessors needed by tests and formatter rules.

Do not add tests that merely pin accessor defaults or enum plumbing. Prefer
integration snapshots showing that wrappers expose the syntax needed by the
formatter.

Initial family enums:

- `KotlinFileItem`
- `Declaration`
- `ClassMember`
- `Type`
- `Expression`
- `Statement`
- `WhenCondition`
- `StringTemplatePart`
- `ValueArgumentListEntry`
- `TypeArgumentListEntry`
- `DestructuringEntry`

## Formatter Roadmap

Only start the Kotlin formatter after parser phases 0-6 can parse the style and
trivia fixtures without parser diagnostics, except deliberately negative cases.

### Formatter Phase A: Crate And Facade

Add:

- `crates/jolt_kotlin_fmt`
- `KotlinFormatOptions`
- `KotlinFormatSinkResult`
- `format_source_to_sink`
- `jolt_formatter::Language::Kotlin`
- CLI/dprint language detection for `.kt` and `.kts`

Keep options aligned with Java initially: line width, indent width, tabs. Kotlin
style-specific options should wait until real behavior requires them.

### Formatter Phase B: Program, Imports, And Comments

Implement:

- comment-only files;
- package/import sections;
- import sorting and alias/star handling;
- formatter-ignore support, if the chosen Kotlin ignore spelling is specified;
- final newline and top-level blank-line normalization.

Fixtures:

- `style/program/*.kt`
- `style/imports/*.kt`
- `trivia/package-and-imports.kt`

### Formatter Phase C: Declarations

Implement:

- annotations and modifiers;
- class/interface/object headers;
- constructors and delegation specifiers;
- enum entries;
- functions and properties;
- accessors and explicit backing fields;
- type parameters and `where` constraints;
- context parameter clauses.

Fixtures:

- `style/declarations/*.kt`
- declaration-related trivia fixtures.

### Formatter Phase D: Expressions

Implement:

- literals and string templates;
- binary/unary/operator layout;
- call chains and safe calls;
- call argument lists and trailing lambdas;
- lambdas and anonymous functions;
- `when`, `if`, `try`, object expressions, collection literals, destructuring.

Fixtures:

- `style/expressions/*.kt`
- `style/layout-fit-boundaries/*.kt`
- expression-related trivia fixtures.

### Formatter Phase E: Statements And Idempotence

Implement:

- block layout;
- local declarations;
- loops and labels;
- jump expressions;
- statement boundary formatting.

Acceptance:

- every valid Kotlin formatter fixture formats without diagnostics;
- formatting is idempotent;
- parse formatted output and ensure it has no parser diagnostics;
- compare source reconstruction before formatting and after reparse.

## Validation Gates

Add these gates in order:

1. `cargo test -p jolt_kotlin_syntax` passes parser snapshots.
2. Kotlin parser fixture count is asserted, like the Java imported corpus tests.
3. Lexer fixtures remain covered after parser API lands.
4. Every non-negative `fixtures/kotlin/syntax/parser` fixture reconstructs
   exactly.
5. Every `fixtures/kotlin/trivia` fixture reconstructs exactly.
6. `jolt_kotlin_fmt` idempotence snapshots pass for every parse-clean style
   fixture.
7. `mise run test` passes at workspace level.

When snapshots change, use `mise run test --update` only after inspecting that
the syntax tree shape and diagnostics match the intended phase.

## Implementation Order Checklist

1. Add Kotlin test-support helpers and syntax corpus tests.
2. Add parser shell and source-preserving file parser.
3. Add file/import/name/annotation nodes.
4. Add declarations enough for top-level style fixtures.
5. Add types and bounded type-argument lookahead.
6. Add expression precedence and postfix/call parsing.
7. Add strings as syntax nodes over the existing lexer modes.
8. Add statements/control flow.
9. Add recovery diagnostics and negative fixture snapshots.
10. Add typed wrappers for formatter-facing traversal.
11. Add `jolt_kotlin_fmt` and formatter facade integration.
12. Format Kotlin in layers: program/imports, declarations, expressions,
    statements.

## Explicit Non-Goals For The First Parser

- No semantic validation beyond parser-local diagnostics.
- No language-version configuration.
- No legacy context receiver support unless archival source support becomes an
  explicit requirement.
- No unbounded best-fitting parse search.
- No parser-inserted tokens for missing Kotlin syntax.
- No convenience accessors unless the formatter needs them.
