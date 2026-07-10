# Formatter Architecture Debt Checklist

Status: OPEN. This is the canonical checklist for bringing the Java and Kotlin
formatters into full compliance with the formatter invariants in `AGENTS.md`. Do
not claim formatter cleanliness while any item here remains open.

## Clean Completion Gate

- [ ] Every represented input token is either emitted through structured
      formatting or covered by a narrowly documented normalization rule.
- [ ] Every represented comment marker is conserved, including recovered and
      delimiter-owned comments.
- [ ] Every parser-diagnostic fixture with a represented tree is formatted,
      checked for conservation, and checked for idempotence.
- [x] `FormatSinkResult::Halted` is rejected by every `StringSink` test path.
- [ ] No partially structured node is replaced by whole-node token replay.
- [ ] Token-sequence formatting is limited to genuinely unstructured recovered
      islands and formatter-ignore ranges.
- [ ] Formatter rules do not infer syntax ownership by scanning tokens or source
      ranges; syntax accessors expose ordered roles and recovered entries.
- [ ] No formatter layout decision reads raw source gaps outside
      formatter-ignore or represented trivia/comment formatting.
- [ ] Every algorithm is linear or has an explicit, documented finite cost model
      and bound.
- [ ] No production formatter path can panic for a represented tree.
- [ ] No missing-child branch drops available siblings, delimiters, operators,
      comments, or recovered entries.
- [ ] No syntax repair token is synthesized for malformed represented syntax.
- [ ] Full Java/Kotlin syntax, formatter, CLI, dprint, formatting, whitespace,
      and snapshot-hygiene checks pass with no conservation allowlist entry that
      hides an unresolved formatter bug.

## Active Reproductions

The recovery clean gates intentionally fail on these committed regressions until
their formatter or syntax ownership is fixed. These are evidence links; the
architecture sections below own completion of the underlying debt.

### Java

- [ ] Preserve trailing annotated array dimensions:
      `fixtures/java/syntax/recovery/array-creation-trailing-dimensions.java`;
      omission at
      `crates/jolt_java_fmt/src/rules/expressions/arrays_objects.rs:79-103`.
- [ ] Preserve module annotations:
      `fixtures/java/syntax/recovery/module-annotation.java`; formatter omission
      at `crates/jolt_java_fmt/src/rules/modules.rs:22-59`.
- [ ] Preserve malformed import suffixes:
      `fixtures/java/syntax/recovery/import-trailing-tokens.java`; structured
      import path at `crates/jolt_java_fmt/src/rules/imports.rs:99-123`.
- [ ] Preserve a recovered missing-body semicolon:
      `fixtures/java/syntax/recovery/missing-type-body-token.java`; omission at
      `crates/jolt_java_fmt/src/rules/declarations/type_declarations.rs:20-48`.
- [ ] Preserve restricted recovered declaration names and invalid modifiers:
      `fixtures/java/syntax/recovery/recovered-declaration-names-and-modifiers.java`;
      access gaps at
      `crates/jolt_java_syntax/src/nodes/accessors.rs:243-245,286-288,359-362,4087-4095`.
- [ ] Preserve repeated `requires` modifiers:
      `fixtures/java/syntax/recovery/module-repeated-requires-modifiers.java`;
      single-token accessors at
      `crates/jolt_java_syntax/src/nodes/accessors.rs:4313-4321`.

### Kotlin

- [ ] Preserve invalid assignment targets and operators:
      `fixtures/kotlin/syntax/recovery/assignment-invalid-targets.kt`;
      operand-dependent accessor at
      `crates/jolt_kotlin_syntax/src/nodes/accessors.rs:2370-2401`.
- [ ] Preserve comments after opening class/block braces:
      `fixtures/kotlin/syntax/recovery/braced-opening-comments.kt`;
      relocated-but-unowned trivia at
      `crates/jolt_kotlin_fmt/src/helpers/blocks.rs:75-109`.
- [ ] Preserve pre-target callable-reference type arguments:
      `fixtures/kotlin/syntax/recovery/callable-reference-missing-target.kt`;
      omission at
      `crates/jolt_kotlin_fmt/src/rules/expressions/references.rs:14-19,48-105`.
- [ ] Preserve `!!` in represented definitely-non-nullable types:
      `fixtures/kotlin/syntax/recovery/definitely-non-nullable-bang.kt`; missing
      accessor/layout at
      `crates/jolt_kotlin_syntax/src/nodes/accessors.rs:1190-1198` and
      `crates/jolt_kotlin_fmt/src/rules/types.rs:616-643`.
- [ ] Preserve and stabilize nested recovered `when` content:
      `fixtures/kotlin/syntax/recovery/nested-recovered-when-condition.kt`;
      entry formatting at
      `crates/jolt_kotlin_fmt/src/rules/expressions/control_flow.rs:717-769`.
- [ ] Preserve property-body items after a recovered header gap:
      `fixtures/kotlin/syntax/recovery/property-body-recovered-gap.kt`; empty
      fallback at `crates/jolt_kotlin_fmt/src/rules/declarations.rs:421-424`.
- [ ] Preserve top-level orphan tokens:
      `fixtures/kotlin/syntax/recovery/top-level-orphan-token.kt`; file-item
      filtering at `crates/jolt_kotlin_fmt/src/rules/program.rs:31-64`.
- [ ] Preserve trailing user-type dots:
      `fixtures/kotlin/syntax/recovery/trailing-user-type-dot.kt`; segment
      reconstruction at `crates/jolt_kotlin_fmt/src/rules/types.rs:225-278`.
- [ ] Preserve the close brace currently lost by
      `fixtures/kotlin/syntax/recovery/type-constraints.kt`.

## Shared Test Debt

- [x] Recovery gates compare represented input/output token multisets outside
      snapshots, so `INSTA_UPDATE=always` cannot bless token loss.
- [x] Clean and diagnostic corpus fixtures also pass through represented-token,
      marker-conservation, and idempotence gates.
- [x] Recovery gates compare `JOLT-TRIVIA` marker multisets for recovered
      comment conservation.
- [x] Intentional Java token removals are exempted by exact fixture, spelling,
      and bounded count rather than global punctuation classes.
- [x] All Java/Kotlin formatter and dprint tests using `StringSink` reject
      `FormatSinkResult::Halted`.
- [ ] Replace the token-text multiset gate with token provenance if identical
      formatter-synthesized tokens can mask a dropped source token.
- [ ] Extend recovery comment conservation from explicit markers to canonical
      inventories of every represented source comment.
- [x] Stop skipping parser-diagnostic fixtures in
      `crates/jolt_java_fmt/tests/corpus.rs:28-33` and
      `crates/jolt_kotlin_fmt/tests/corpus.rs:28-33`; route every represented
      tree through conservation and idempotence checks.
- [ ] Stop skipping diagnostic imported Java and Kotlin inputs in
      `crates/jolt_java_fmt/tests/corpus_fixtures.rs:40-53` and
      `crates/jolt_kotlin_fmt/tests/corpus_fixtures.rs:35-49`; report exact
      skipped paths rather than aggregate counts.
- [ ] Make imported fixture manifests content-addressed instead of validating
      only aggregate counts.
- [ ] Make imported Java and Kotlin syntax reconstruction loss a hard failure
      instead of a summary count in
      `crates/jolt_java_syntax/tests/parser_fixtures.rs:38-42` and
      `crates/jolt_kotlin_syntax/tests/imported_fixtures.rs:25-38`.
- [ ] Give token-conservation checks source-token provenance so an identical
      synthesized/duplicated output token cannot mask a source-token loss.
- [ ] Audit every Java and Kotlin `format_token_sequence` call and prove that it
      receives a genuinely unstructured recovered island; replace every
      partially structured replay with syntax-owned ordered recovered entries.
- [ ] Return a diagnostic when either formatter receives no syntax tree instead
      of an unexplained empty blocked result:
      `crates/jolt_java_fmt/src/format.rs:33-36` and
      `crates/jolt_kotlin_fmt/src/format.rs:33-36`.

## Diagnostic Corpus Gate Findings

These failures were previously hidden by the formatter corpus diagnostic skip.
They are now hard conservation/idempotence failures outside snapshots.

### Java

- [ ] Preserve all represented pieces in
      `fixtures/java/syntax/parser/diagnoses-invalid-declaration-contexts.java`;
      current losses include invalid modifiers, declarator suffixes, and
      initializer tokens.
- [ ] Preserve and stabilize all represented pieces in
      `fixtures/java/syntax/parser/diagnoses-invalid-expression-forms.java`.
- [ ] Preserve duplicate/recovered parameter names in
      `fixtures/java/syntax/parser/diagnoses-invalid-lambda-parameters.java`.
- [ ] Preserve missing-body recovery semicolons in
      `fixtures/java/syntax/parser/diagnoses-missing-class-body.java`.
- [ ] Preserve restricted recovered type names in
      `fixtures/java/syntax/parser/recovers-restricted-type-identifiers.java`.
- [ ] Preserve annotated dimension expressions in
      `fixtures/java/syntax/parser/parses-annotated-dim-expression.java`.
- [ ] Preserve module annotations in
      `fixtures/java/syntax/parser/parses-modular-compilation-unit-and-module-directives.java`.
- [ ] Preserve trailing method/annotation-element dimensions and their
      annotations in
      `fixtures/java/syntax/parser/parses-trailing-dims-on-method-and-annotation-element-declarators.java`
      and
      `fixtures/java/syntax/parser/trailing-method-and-annotation-element-dims-have-per-dimension-nodes.java`.

### Kotlin

- [ ] Preserve invalid assignment targets/operators in
      `fixtures/kotlin/syntax/parser/diagnoses-invalid-assignment-targets.kt`.
- [ ] Preserve `?` and stabilize malformed type-argument calls in
      `fixtures/kotlin/syntax/parser/diagnoses-malformed-type-argument-call.kt`.
- [ ] Preserve a dangling Elvis operator in
      `fixtures/kotlin/syntax/parser/recovers-missing-expression-after-elvis.kt`.
- [ ] Preserve string-condition tokens and stabilize output in
      `fixtures/kotlin/syntax/parser/recovers-missing-when-arrow-and-body.kt`.
- [ ] Preserve name-based destructuring defaults/modifiers in
      `fixtures/kotlin/syntax/parser/parses-destructuring-name-based-preview.kt`.

## Kotlin Structural Recovery Debt

### Types And Parameters

- [ ] Format constraints even when `where` is missing:
      `crates/jolt_kotlin_fmt/src/rules/types.rs:55-64`.
- [ ] Format represented bounds when `:` is missing:
      `crates/jolt_kotlin_fmt/src/rules/types.rs:163-185`.
- [ ] Preserve recovered `TypeReference` children when no typed family exists:
      `crates/jolt_kotlin_fmt/src/rules/types.rs:202-210`.
- [ ] Preserve malformed user-type segments, extra dots, annotations, and
      unassigned type arguments:
      `crates/jolt_kotlin_fmt/src/rules/types.rs:225-278`.
- [ ] Do not let a star projection hide a simultaneous represented type:
      `crates/jolt_kotlin_fmt/src/rules/types.rs:383-400`.
- [ ] Do not let the `suspend` nested-function shortcut hide other represented
      function-type pieces: `crates/jolt_kotlin_fmt/src/rules/types.rs:501-517`.
- [ ] Preserve names, colons, and recovered tokens in context-function
      parameters: `crates/jolt_kotlin_fmt/src/rules/types.rs:584-595`.
- [ ] Preserve all represented definitely-non-nullable type children, not only
      the first two: `crates/jolt_kotlin_fmt/src/rules/types.rs:620-643`.
- [ ] Preserve a value-parameter default expression when `=` is missing:
      `crates/jolt_kotlin_fmt/src/rules/variables.rs:86-103`.

### Declarations

- [ ] Preserve recovered enum-entry pieces when its expression is absent:
      `crates/jolt_kotlin_fmt/src/rules/declarations.rs:138-147`.
- [ ] Preserve secondary-constructor delegation when `:` is missing:
      `crates/jolt_kotlin_fmt/src/rules/declarations.rs:308-334`.
- [ ] Replace property-body `unwrap_or_else(nil)` with recovered interleaving:
      `crates/jolt_kotlin_fmt/src/rules/declarations.rs:388-424`.
- [ ] Give property-body gaps before, between, and after backing
      fields/accessors explicit ownership:
      `crates/jolt_kotlin_fmt/src/rules/declarations.rs:461-505`.
- [ ] Preserve accessor expression tails without `=` and simultaneous recovered
      block/expression pieces:
      `crates/jolt_kotlin_fmt/src/rules/declarations.rs:571-618`.
- [ ] Preserve destructuring callable names with a missing close delimiter:
      `crates/jolt_kotlin_fmt/src/rules/declarations.rs:635-644`.
- [ ] Preserve callable receiver/separator pieces when the final name is
      missing: `crates/jolt_kotlin_fmt/src/rules/declarations.rs:669-693`.
- [ ] Preserve type-alias types when `=` is missing:
      `crates/jolt_kotlin_fmt/src/rules/declarations.rs:747-781`.
- [ ] Format context-parameter defaults exposed by syntax accessors:
      `crates/jolt_kotlin_fmt/src/rules/declarations.rs:885-911`.
- [ ] Make primary-constructor structure independent of declaration-name,
      opening-parenthesis, and source-gap success:
      `crates/jolt_kotlin_fmt/src/rules/declarations/type_declarations.rs:23-45,358-433`.
- [ ] Preserve delegation colons and partial specifier pieces:
      `crates/jolt_kotlin_fmt/src/rules/declarations/type_declarations.rs:244-255,324-355`.
- [ ] Prove unclassified class members are genuinely unstructured recovered
      islands or expose their structure through syntax accessors:
      `crates/jolt_kotlin_fmt/src/rules/declarations/member_bodies.rs:269-275`.

### Expressions And Control Flow

- [ ] Preserve labels/type arguments when `this` or `super` is missing:
      `crates/jolt_kotlin_fmt/src/rules/expressions/leaves.rs:41-67`.
- [ ] Preserve lambda parameters/body/close brace when `{` is missing:
      `crates/jolt_kotlin_fmt/src/rules/expressions/lambdas.rs:27-29`.
- [ ] Expose dangling assignment and binary operators without requiring a right
      operand: `crates/jolt_kotlin_syntax/src/nodes/accessors.rs:2385-2426` and
      `crates/jolt_kotlin_fmt/src/rules/expressions/operators.rs:54-69,114-119`.
- [ ] Preserve navigation selectors when the operator is missing:
      `crates/jolt_kotlin_fmt/src/rules/expressions/calls.rs:54-57`.
- [ ] Replace keyword-missing empty returns for `if`, `when`, `try`, `for`,
      `while`, `do`, jump, and throw nodes:
      `crates/jolt_kotlin_fmt/src/rules/expressions/control_flow.rs:26-28,66-68,129-131,192-194,301-303,334-336,395-397,441-443`.
- [ ] Preserve `when` entries without `{` and `do` condition pieces without
      `while`:
      `crates/jolt_kotlin_fmt/src/rules/expressions/control_flow.rs:69-79,337-350`.
- [ ] Preserve lambda-as-branch pieces without `{`:
      `crates/jolt_kotlin_fmt/src/rules/expressions/control_flow.rs:871-873`.
- [ ] Honor collection-literal leading-trivia ownership:
      `crates/jolt_kotlin_fmt/src/rules/expressions/calls.rs:152-170`.

### Containers

- [ ] Add recovered streams for file items and import-list contents:
      `crates/jolt_kotlin_fmt/src/rules/program.rs:31-64,143-150`.
- [ ] Preserve comments owned by EOF in comment-only Kotlin files:
      `crates/jolt_kotlin_fmt/src/rules/program.rs:31-34`.
- [ ] Preserve duplicate represented package headers and import lists instead of
      overwriting option slots:
      `crates/jolt_kotlin_fmt/src/rules/program.rs:139-148`.
- [ ] Expose ordered recovered pieces inside package headers and import
      directives, not only at the enclosing import-list level:
      `crates/jolt_kotlin_fmt/src/rules/program.rs:426-447` and
      `crates/jolt_kotlin_fmt/src/rules/imports.rs:59-78,87-141`.
- [ ] Add recovered streams for `when` bodies and try/catch/finally sequences:
      `crates/jolt_kotlin_fmt/src/rules/expressions/control_flow.rs:81-149`.
- [ ] Add recovered call-suffix and user-type segment streams:
      `crates/jolt_kotlin_fmt/src/rules/expressions/calls.rs:100-124` and
      `crates/jolt_kotlin_fmt/src/rules/types.rs:225-278`.
- [ ] Add recovered qualified-name segments:
      `crates/jolt_kotlin_fmt/src/rules/names.rs:93-169`.
- [ ] Preserve direct type-argument content when the projection-list wrapper is
      absent: `crates/jolt_kotlin_fmt/src/rules/types.rs:312-357`.
- [ ] Make generic recovered-list delimiter skipping identify the actual
      boundary token rather than every token of the same kind:
      `crates/jolt_kotlin_syntax/src/nodes/accessors.rs:1524-1605` and callers
      at
      `:938-949,1302-1308,1366-1372,1815-1826,1902-1913,1963-1969,2037-2048,2847-2853,3319-3333`.
- [ ] Do not stop recovered-list ownership at an orphan early close delimiter:
      `crates/jolt_kotlin_syntax/src/nodes/accessors.rs:530-553,1024-1047,1149-1172,2504-2532,2604-2632`.

## Kotlin Formatter-Owned Syntax Debt

### Partial Replay And Ownership Inference

- [ ] Replace string-template token replay/range matching with ordered syntax
      parts: `crates/jolt_kotlin_fmt/src/rules/expressions/leaves.rs:92-147`.
- [ ] Replace whole-node fallback for identifier-less user types:
      `crates/jolt_kotlin_fmt/src/rules/types.rs:225-232`.
- [ ] Replace whole-node fallback for anonymous functions missing `fun`:
      `crates/jolt_kotlin_fmt/src/rules/expressions/functions.rs:19-21`.
- [ ] Replace whole-node fallbacks for type-binary, unary, and postfix nodes
      with available-piece formatting:
      `crates/jolt_kotlin_fmt/src/rules/expressions/operators.rs:135-143,543-571`.
- [ ] Replace value-argument whole-node fallback with structured prefix plus
      recovered remainder:
      `crates/jolt_kotlin_fmt/src/rules/expressions/calls.rs:727-755`.
- [ ] Move receiver-modifier, declaration-prefix, property-body-order,
      user-type-segment, callable-reference type-argument, and named-argument
      ownership into syntax accessors:
      `crates/jolt_kotlin_fmt/src/rules/declarations.rs:177-191,361-365,461-471,802-828`,
      `crates/jolt_kotlin_fmt/src/rules/types.rs:229-275`,
      `crates/jolt_kotlin_fmt/src/rules/expressions/references.rs:82-105`, and
      `crates/jolt_kotlin_fmt/src/rules/expressions/calls.rs:758-787`.
- [ ] Make `Name` expose malformed additional pieces instead of taking the first
      token: `crates/jolt_kotlin_fmt/src/rules/names.rs:9-23`.
- [ ] Replace expression-order/range role inference for `if`, `for`, and calls:
      `crates/jolt_kotlin_syntax/src/nodes/accessors.rs:2739-2761,2900-2938,3147-3181`.
- [ ] Give trailing-lambda call wrappers explicit member-chain ownership. The
      outer call must own the complete chain, and nested receiver/callee
      expressions must not start independent builders. Then include `CallCallee`
      in member-chain child detection instead of relying on its current
      exclusion:
      `crates/jolt_kotlin_fmt/src/rules/expressions/calls.rs:258-331,485-489`
      and `crates/jolt_kotlin_syntax/src/nodes/accessors.rs:2545-2580`.
- [ ] Represent `fun interface` as one syntax declaration and remove formatter
      pairing of adjacent function/interface declarations:
      `crates/jolt_kotlin_syntax/src/nodes/accessors.rs:438-457`,
      `crates/jolt_kotlin_fmt/src/rules/program.rs:291-336`, and
      `crates/jolt_kotlin_fmt/src/rules/declarations.rs:71-88`.

### Source Gaps And Complexity

- [ ] Replace recovered-gap source slicing with parser trivia ownership:
      `crates/jolt_kotlin_fmt/src/helpers/comments.rs:278-297`.
- [ ] Replace raw blank-line counting in block/program layout:
      `crates/jolt_kotlin_fmt/src/rules/statements/blocks.rs:310-348` and
      `crates/jolt_kotlin_fmt/src/rules/program.rs:412-424`.
- [ ] Remove declaration and constructor source-gap guards in favor of syntax
      ownership:
      `crates/jolt_kotlin_fmt/src/rules/declarations.rs:336-355,388-420,474-504,995-1038,1089-1120`
      and
      `crates/jolt_kotlin_fmt/src/rules/declarations/type_declarations.rs:358-410`.
- [ ] Replace formatter-ignore raw delimiter scanning with represented comment
      ownership as formatter-ignore robustness debt:
      `crates/jolt_kotlin_fmt/src/rules/statements/blocks.rs:247-267`.
- [ ] Make string-template and user-type matching linear:
      `crates/jolt_kotlin_fmt/src/rules/expressions/leaves.rs:97-129` and
      `crates/jolt_kotlin_fmt/src/rules/types.rs:229-275`.
- [ ] Remove property-body sorting by consuming source-ordered syntax entries:
      `crates/jolt_kotlin_fmt/src/rules/declarations.rs:461-471`.
- [ ] Document a finite cost model for import sorting or replace it with a
      compliant bounded strategy:
      `crates/jolt_kotlin_fmt/src/rules/imports.rs:31-47`.

### Synthesis And Panic

- [ ] Prevent malformed import first tokens from being normalized into `import`:
      `crates/jolt_kotlin_syntax/src/nodes/accessors.rs:156-160` and
      `crates/jolt_kotlin_fmt/src/rules/imports.rs:87-105`.
- [ ] Move alias normalization preconditions into the normalization helper:
      `crates/jolt_kotlin_fmt/src/rules/imports.rs:121-140`.
- [ ] Remove production `expect` calls at
      `crates/jolt_kotlin_fmt/src/rules/names.rs:132-137`,
      `crates/jolt_kotlin_fmt/src/rules/statements/blocks.rs:118-123`,
      `crates/jolt_kotlin_fmt/src/rules/expressions/lambdas.rs:61-63,242-247`,
      `crates/jolt_kotlin_fmt/src/rules/expressions/calls.rs:342-347`, and
      `crates/jolt_kotlin_fmt/src/rules/expressions/control_flow.rs:898-906`.

## Java Structural Recovery Debt

### Valid And Recovered Token Loss

- [ ] Preserve trailing unsized annotated array dimensions in array creation:
      `crates/jolt_java_fmt/src/rules/expressions/arrays_objects.rs:79-103` and
      `crates/jolt_java_syntax/src/nodes/accessors.rs:2712-2730`.
- [ ] Make singleton variable, lambda-parameter, switch-label, and enum-constant
      optimizations account for recovered siblings:
      `crates/jolt_java_fmt/src/rules/variables.rs:40-47,81-89,329-339`,
      `crates/jolt_java_fmt/src/rules/expressions/lambdas.rs:78-91`,
      `crates/jolt_java_fmt/src/rules/statements/switches.rs:130-135,183-200`,
      and `crates/jolt_java_fmt/src/rules/declarations/enums.rs:30-34,60-73`.
- [ ] Preserve duplicate represented package/module declarations instead of
      overwriting option slots:
      `crates/jolt_java_fmt/src/rules/program.rs:91-112,161-172`.
- [ ] Preserve partial pattern pieces:
      `crates/jolt_java_fmt/src/rules/patterns.rs:21-28,75-82`.
- [ ] Format unclassified `for` pieces rather than returning `nil`:
      `crates/jolt_java_fmt/src/rules/statements/control_flow.rs:215-227`.
- [ ] Preserve unclassified switch-rule bodies:
      `crates/jolt_java_fmt/src/rules/statements/switches.rs:469-492`.
- [ ] Preserve resource content/trailing-semicolon comments without a resource
      list and catch delimiters without a parameter:
      `crates/jolt_java_fmt/src/rules/statements/try_resources.rs:77-111,290-310`.
- [ ] Add malformed method-reference receiver recovery:
      `crates/jolt_java_fmt/src/rules/expressions/method_references.rs:67-89`.
- [ ] Preserve both leading and trailing EOF comments in comment-only files:
      `crates/jolt_java_fmt/src/rules/program.rs:27-28` and
      `crates/jolt_java_fmt/src/rules/comments.rs:11-19`.

### Recovered Containers And Accessors

- [ ] Add recovered segment streams for names and class types:
      `crates/jolt_java_syntax/src/nodes/accessors.rs:201-227,690-743`.
- [ ] Add recovered entries for array dimensions and modifiers:
      `crates/jolt_java_syntax/src/nodes/accessors.rs:2340-2359,4087-4129`.
- [ ] Preserve direct annotation-interface and annotation-argument content when
      wrapper lists are absent:
      `crates/jolt_java_syntax/src/nodes/accessors.rs:1008-1038,2426-2442`.
- [ ] Expose record-pattern components without requiring source `(`:
      `crates/jolt_java_syntax/src/nodes/accessors.rs:4023-4052`.
- [ ] Expose module directives without requiring `{` and target names without
      requiring `to`/`with`:
      `crates/jolt_java_syntax/src/nodes/accessors.rs:4243-4266,5294-5382`.
- [ ] Preserve orphan/repeated switch colons:
      `crates/jolt_java_syntax/src/nodes/accessors.rs:3723-3749`.
- [ ] Add recovered sequencing between try body, catches, and finally:
      `crates/jolt_java_syntax/src/nodes/accessors.rs:3208-3215,3234-3241`.
- [ ] Establish a general consumed-pieces/recovered-siblings contract instead of
      relying on filtering helpers that silently hide unmatched children:
      `crates/jolt_java_syntax/src/nodes/mod.rs:1144-1225`.

## Java Formatter-Owned Syntax Debt

### Partial Replay

- [ ] Replace whole-node fallbacks for imports, unclassified annotation values,
      component patterns, empty binary expressions, module directives, type
      arguments, expression statements, resources, switch labels, and block
      statements: `crates/jolt_java_fmt/src/rules/imports.rs:99-110`,
      `crates/jolt_java_fmt/src/rules/annotations.rs:64-75`,
      `crates/jolt_java_fmt/src/rules/patterns.rs:65-72`,
      `crates/jolt_java_fmt/src/rules/expressions/operators.rs:96-114`,
      `crates/jolt_java_fmt/src/rules/modules.rs:351-360,417-491`,
      `crates/jolt_java_fmt/src/rules/types.rs:491-503`,
      `crates/jolt_java_fmt/src/rules/statements/simple.rs:36-42`,
      `crates/jolt_java_fmt/src/rules/statements/try_resources.rs:261-276`,
      `crates/jolt_java_fmt/src/rules/statements/switches.rs:447-459`, and
      `crates/jolt_java_fmt/src/rules/statements/blocks.rs:225-236`.
- [ ] Complete the shared `format_token_sequence` audit above for the Java
      primitive at `crates/jolt_java_fmt/src/helpers/comments.rs:354-402`.

### Ownership, Source Gaps, And Complexity

- [ ] Move operator class/precedence/associativity decisions from token text to
      syntax-owned operator metadata:
      `crates/jolt_java_fmt/src/rules/expressions/operators.rs:90-93,179-181,259-260,283-288,434-491`.
- [ ] Move enum separator source-spelling classification into syntax accessors:
      `crates/jolt_java_fmt/src/rules/declarations/enums.rs:231-274`.
- [ ] Remove source-gap layout reconstruction from recovered token formatting:
      `crates/jolt_java_fmt/src/helpers/comments.rs:383-402`.
- [ ] Document finite cost models or replace unbounded sorting for imports,
      module directives, and malformed modifier runs:
      `crates/jolt_java_fmt/src/rules/imports.rs:32-49`,
      `crates/jolt_java_fmt/src/rules/modules.rs:296-305`, and
      `crates/jolt_java_fmt/src/helpers/modifiers.rs:74-105`.
- [ ] Remove quadratic enum lookahead:
      `crates/jolt_java_fmt/src/rules/declarations/enums.rs:103-163`.
- [ ] Make formatter-ignore range/item matching and marker line lookup linear:
      `crates/jolt_fmt_ir/src/formatter_ignore.rs:45-110,172-213,312-325`.
- [ ] Make argument parent-role lookup constant-time or single-pass:
      `crates/jolt_java_syntax/src/nodes/accessors.rs:1986-1989`.

### Synthesis And Panic

- [ ] Stop repairing missing statement, switch, synchronized, try, catch, and
      finally bodies with synthesized `{}`:
      `crates/jolt_java_fmt/src/rules/statements.rs:105`,
      `crates/jolt_java_fmt/src/rules/statements/switches.rs:27-30`,
      `crates/jolt_java_fmt/src/rules/statements/control_flow.rs:607-610`, and
      `crates/jolt_java_fmt/src/rules/statements/try_resources.rs:27-30,58-61,306-309,509-512`.
- [ ] Remove production `expect` calls at
      `crates/jolt_java_fmt/src/rules/modules.rs:318-325` and
      `crates/jolt_java_fmt/src/rules/expressions/member_chains.rs:137-142`.

## Verified Clean Areas

- [x] Raw literal source output remains limited to formatter-ignore ranges.
- [x] Formatter production code does not clone parser-owned source text, token
      buffers, or syntax-node buffers.
- [x] Java enum/list normalization and readability-parenthesis insertion are
      explicitly reason-tagged; malformed missing-body brace repair remains open
      above.
- [x] Kotlin readability parentheses are explicitly reason-tagged.
