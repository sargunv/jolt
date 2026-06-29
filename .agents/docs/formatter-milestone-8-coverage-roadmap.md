# Formatter Milestone 8 Coverage Roadmap

Milestone 8 is not complete until Jolt can format any valid Java source accepted
by `jolt_java_syntax` without returning `java.format.missing_layout_rules`.

The parser/input layer is not expected to be the blocker for Milestone 8. The
remaining work is formatter coverage: the layout builder still contains broad
unsupported-shape guards and explicit `missing_layout` exits for valid Java
syntax. Those exits are useful while developing because they prevent partial
output, but every valid-Java `missing_layout` path must be removed before
Milestone 8 is done.

## Completion Bar

Coverage and oracle compatibility are related but separate:

- Coverage: every valid Java source that parses cleanly formats without
  `java.format.missing_layout_rules`.
- Safety: parse diagnostics still block formatting without output.
- Rendering: render failures are internal bugs, not acceptable blocked output.
- Oracle measurement: the pinned valid google-java-format corpus reports exact
  matches and aggregate diff size as progress metrics, not Milestone 8 exit
  criteria.
- Source inventory: every `missing_layout` call site is either removed, narrowed
  to invalid/unreachable parser-clean structure, or covered by a test proving
  the input is not valid clean Java.

The source inventory criterion is required because the oracle corpus is finite.
Passing the pinned corpus is not the same as being able to format any valid Java
source.

## Early Wrapping Policy And Architecture

Wrapping is not a late oracle-compatibility pass. The current Java formatter can
emit short flat files, but it does not yet use the renderer's width-aware group
machinery in the rules that will unlock declarations, lists, chains,
expressions, and comments. Those rule families should land with wrapping from
the start, because adding them as flat-only formatters would immediately create
valid formatted output that is structurally wrong for long Java sources.

### google-java-format Policy Audit

Primary source revision:
`google-java-format@fb9528917c524c8eb9c8c0d7b4bcd7ce3b6a604b`.

Actionable policy:

- Line width is fixed at 100 columns for the Google profile. The formatter emits
  break opportunities and lets document fitting choose which levels break.
- Style changes the indentation multiplier, not the width. Google uses the
  normal logical indentation unit; AOSP doubles it.
- Wrapping is represented with document breaks, not rule-local
  `if length > width` checks. Breaks are selected as unified, independent, or
  forced groups.
- Method and constructor headers need a specialized helper, not a generic list:
  type parameters, return type, name, parameters, throws clauses, default
  values, and bodies have separate break tags and conditional indentation.
- Variable, field, and parameter declarations need a tagged break between type
  and name. If the type breaks, the name and initializer align through
  conditional indentation; initializers break after `=`.
- Argument and formal-parameter lists open with a break opportunity after `(`,
  then use comma separators with break opportunities. Short item lists may fill
  independently; longer lists break in unified form.
- Type arguments and type parameters need their own helpers because `<...>`
  lists use different opening and separator breaks than call arguments.
- Method chains must be flattened before formatting. Member select, invocation,
  and array access selectors should go through one `dot_chain` helper that
  breaks before dots and handles prefix classification.
- Binary expressions should flatten only across operators of the same
  precedence. Break before the operator for binary operators; break after the
  operator for assignment and compound assignment.
- Annotation layout is declaration-sensitive. Type declarations use vertical
  declaration annotations; fields and locals allow horizontal annotations only
  in narrower cases. Annotation argument lists use the same grouped list
  machinery as calls.
- Comment handling participates in wrapping. Trailing line comments are suffixes
  that affect group fitting; line comments are normalized and wrapped to the
  same 100-column target; Javadocs and block comments need indentation-aware
  preservation.
- Blank lines are explicit layout requests. Top-level sections, blocks,
  statement lists, and class bodies each have their own blank-line policy.

Relevant google-java-format source identifiers: `Formatter.MAX_LINE_LENGTH`,
`Formatter.format`, `Doc.FillMode`, `Doc.Level.computeBreaks`, `Doc.Break`,
`JavaFormatterOptions.Style`, `JavaInputAstVisitor.visitMethod`,
`JavaInputAstVisitor.visitClassDeclaration`,
`JavaInputAstVisitor.visitRecordDeclaration`,
`JavaInputAstVisitor.typeParametersRest`,
`JavaInputAstVisitor.classDeclarationTypeList`,
`JavaInputAstVisitor.declareOne`, `JavaInputAstVisitor.addArguments`,
`JavaInputAstVisitor.argList`, `JavaInputAstVisitor.hasOnlyShortItems`,
`JavaInputAstVisitor.visitDot`, `JavaInputAstVisitor.visitRegularDot`,
`JavaInputAstVisitor.walkInfix`, `JavaInputAstVisitor.visitBinary`,
`JavaInputAstVisitor.visitAssignment`,
`JavaInputAstVisitor.visitCompoundAssignment`,
`JavaInputAstVisitor.visitAnnotation`, `JavaCommentsHelper.rewrite`,
`BlankLineWanted`, `JavaInputAstVisitor.visitCompilationUnit`,
`JavaInputAstVisitor.visitBlock`, `JavaInputAstVisitor.visitStatements`, and
`JavaInputAstVisitor.addBodyDeclarations`.

### Architecture Audit

Ruff, Oxc, and Prettier agree on the architecture shape Jolt should use:

- Keep `jolt_fmt_ir` language-neutral. The renderer should know about groups,
  lines, fill, best-fitting alternatives, line suffixes, and fitting. It should
  not know Java declaration, argument, chain, or annotation policy.
- Put wrapping policy in Java rule helpers with domain names:
  `declaration_header`, `formal_list`, `argument_list`, `type_argument_list`,
  `dot_chain`, `binary_chain`, `annotation_or_modifier_list`,
  `block_blank_lines`, and `class_body_blank_lines`.
- Build policy-bearing list helpers early. These helpers should own separators,
  open/close behavior, trailing and dangling comments, forced parent expansion,
  and whether a list breaks independently or as one group.
- Use normal groups first. Reserve `best_fitting` or multi-alternative layouts
  for cases that actually need staged expansion, such as method chains or other
  Java-specific special cases.
- Treat comments as formatter-owned source facts. Keep comment attachment and
  accounting in `jolt_java_fmt`, not in CST wrappers or the renderer.
- Rule boundaries should own comment accounting. Leading, trailing, dangling,
  inner, and list-item comments must be consumed deliberately, and supported
  rules should fail tests if a comment is left unformatted.
- Use `line_suffix` and `line_suffix_boundary` for trailing comments so pending
  suffix width affects group fitting.
- Keep generated rule glue out of Milestone 8 until the handwritten Java helper
  surface and wrapper accessors stabilize.
- Add focused narrow-width tests at helper boundaries, plus idempotence checks
  where practical. The oracle scoreboard remains the integration signal, not the
  only wrapping test.

Primary architecture references from the audit:

- Prettier document builders and printer:
  `src/document/builders/{group,if-break,fill,line,line-suffix}.js`,
  `src/document/printer/printer.js`, and `src/main/comments/{attach,print}.js`.
- Ruff formatter core and Python formatter:
  `crates/ruff_formatter/src/{format_element,builders,printer/mod}.rs`,
  `crates/ruff_python_formatter/src/{context,lib,builders}.rs`, and
  `crates/ruff_python_formatter/src/comments/`.
- Oxc formatter core and JavaScript formatter:
  `crates/oxc_formatter_core/src/{format_element,builders,printer/mod}.rs`,
  `crates/oxc_formatter/src/formatter/{context,comments,trivia}.rs`,
  `crates/oxc_formatter/src/print/call_like_expression/arguments.rs`,
  `crates/oxc_formatter/src/utils/member_chain/mod.rs`, and
  `crates/oxc_formatter/src/print/binary_like_expression.rs`.

### Required Early Jolt Work

Before expanding method declarations, argument lists, chained selectors, binary
expressions, or annotation layouts, add the Java wrapping substrate:

1. Make `JavaFormatContext` carry the state needed by wrapping helpers: profile,
   source text, comment cursor, group ids, break markers, and any current
   container/list context proven by call sites.
2. Add policy-bearing helpers for declaration headers, parenthesized lists,
   comma lists, type argument/type parameter lists, method chains, binary
   chains, annotations/modifiers, braced blocks, and blank-line decisions.
3. Map google-java-format's unified/independent/forced break choices onto the
   existing IR primitives. Add an IR primitive only if a concrete Java helper
   cannot express the required policy.
4. Add narrow-width tests for each helper as it lands: parameters, arguments,
   throws clauses, type arguments, binary chains, method chains, trailing
   comments, dangling comments, and blank-line-sensitive class bodies.
5. Keep flat short-case tests beside the forced-wrapping tests so helpers prove
   both fit modes.

## Source-Level Audit

Effort ratings below estimate the Jolt work to remove the current coverage
blocker once the wrapping substrate exists: S is local rule work, M is one
helper family, L is multiple helper families, and XL is cross-cutting
infrastructure. Confidence is confidence in the estimate, not oracle exact-match
parity.

Local source citation shorthand used below:

- GJF visitor:
  `.oracles/repos/google__google-java-format/core/src/main/java/com/google/googlejavaformat/java/JavaInputAstVisitor.java`
- GJF comments:
  `.oracles/repos/google__google-java-format/core/src/main/java/com/google/googlejavaformat/java/JavaCommentsHelper.java`
- GJF formatter:
  `.oracles/repos/google__google-java-format/core/src/main/java/com/google/googlejavaformat/java/Formatter.java`
- PJF visitor:
  `.oracles/repos/palantir__palantir-java-format/palantir-java-format/src/main/java/com/palantir/javaformat/java/JavaInputAstVisitor.java`
- PJF Java 14 visitor:
  `.oracles/repos/palantir__palantir-java-format/palantir-java-format/src/main/java/com/palantir/javaformat/java/java14/Java14InputAstVisitor.java`
- PJF comments:
  `.oracles/repos/palantir__palantir-java-format/palantir-java-format/src/main/java/com/palantir/javaformat/java/JavaCommentsHelper.java`

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

Upstream evidence and estimates:

- Module declarations: GJF formats annotations, `open module`, directive blocks,
  blank lines between directive kinds, and `exports`/`opens`/`provides`/
  `requires`/`uses`; PJF Java 14 delegates modules from `handleModule` into the
  same directive visitors. Effort L, confidence Medium. Citations: GJF
  visitor:2828, :2867, :2897, :2903, :2909, :2915, :2935; PJF Java 14
  visitor:70.
- Package annotations: GJF and PJF print each package annotation on its own line
  before `package`. Effort S, confidence High. Citations: GJF visitor:389,
  :1818; PJF visitor:347, :1653.
- Compact compilation-unit field declarations: GJF detects implicit classes and
  formats compilation-unit members through `addBodyDeclarations` without braces.
  Effort M, confidence Medium. Citation: GJF visitor:454, :471.
- Compact compilation-unit empty declarations: GJF handles extra semicolons with
  `dropEmptyDeclarations` at compilation-unit and member boundaries; PJF has the
  same cleanup in ordinary compilation units/imports/members. Effort S,
  confidence High. Citations: GJF visitor:397, :440, :471; PJF visitor:355,
  :395, :1138.

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

Upstream evidence and estimates:

- Nested class declarations: GJF and PJF do not special-case nesting; class-like
  members flow through `addBodyDeclarations` and re-enter the same declaration
  visitors. Effort M, confidence High. Citations: GJF visitor:454, :475, :858,
  :961, :2181, :3882; PJF visitor:406, :424, :817, :1989; PJF Java 14
  visitor:153, :174.
- Enum declarations: GJF and PJF separate enum constants from ordinary members,
  format constant arguments/class bodies, preserve blank lines between
  constants, handle optional semicolons, then reuse class-body formatting for
  members. Effort L, confidence High. Citations: GJF visitor:839, :858, :927,
  :951; PJF visitor:799, :817, :894, :911.
- Interface declarations and nested interface declarations: GJF and PJF use the
  class-declaration path with an `interface` keyword and `extends` supertype
  list, including when nested. Effort M, confidence High. Citations: GJF
  visitor:454, :2181, :2207, :2216, :3882; PJF visitor:406, :1989, :2017.
- Annotation interface declarations: GJF and PJF format `@interface` in a
  dedicated declaration visitor and then reuse class-body formatting for
  annotation elements and members. Effort M, confidence High. Citations: GJF
  visitor:475, :491, :1501, :1633; PJF visitor:424, :441.
- Record declarations and nested record declarations: GJF formats records with
  type parameters, record components via formal-parameter helpers, implements
  clauses, generated-member filtering, and class-body reuse; PJF Java 14 follows
  the same record-specific path. Effort L, confidence High. Citations: GJF
  visitor:454, :961, :975, :984, :1007, :1011, :3882; PJF Java 14 visitor:153,
  :174, :189, :199, :222.
- Nested enum declarations: GJF and PJF re-enter enum formatting from class-body
  member formatting, so nested enums share constant/member handling with
  top-level enums. Effort M, confidence High. Citations: GJF visitor:454, :858,
  :951, :3882; PJF visitor:406, :817, :911.
- Class type parameters: GJF and PJF print type parameters with
  `typeParametersRest`, then format bounds in `visitTypeParameter` with
  indentation driven by following header clauses. Effort M, confidence High.
  Citations: GJF visitor:2191, :2197, :2222; PJF visitor:2007, :2031.

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

Upstream evidence and estimates:

- Declaration annotations: GJF splits modifier tokens and annotation AST nodes
  into declaration modifiers versus type annotations, prints declaration
  annotations vertically or horizontally by context, then returns type
  annotations to the owning type rule; PJF uses the same split with
  list-returning op helpers. Effort M, confidence High. Citations: GJF
  visitor:2291, :2407, :2431, :2453, :2585; PJF visitor:2097, :2219.
- Contextual class modifiers: GJF scans modifier tokens from the input stream
  instead of relying only on AST modifier enums, so contextual tokens such as
  `sealed`/`non-sealed` can stay in source order before header clauses like
  `permits`; PJF has the same token-scanning modifier infrastructure. Effort M,
  confidence Medium. Citations: GJF visitor:2181, :2210, :2585; PJF
  visitor:1989, :2019, :2228.

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

Upstream evidence and estimates:

- Type shapes: GJF and PJF use structured visitors for primitive/void,
  parameterized, annotated, array, wildcard, type-parameter, union/intersection,
  and dimension formatting rather than token flattening. Effort L, confidence
  High. Citations: GJF visitor:629, :1474, :1836, :1927, :2222, :2251, :2272,
  :3767; PJF visitor:582, :1359, :1670, :1751, :2031, :2060, :2078.

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

Upstream evidence and estimates:

- Method declaration shapes: GJF uses one specialized `visitMethod` path for
  declaration annotations, type parameters, return type, name, receiver/formal
  parameters, trailing dimensions, throws, annotation defaults, semicolon
  bodies, and block bodies, with `BreakTag`s tying type/name/parameter
  indentation together; PJF keeps the same shape but uses Palantir
  `BreakBehaviours` for wrapping. Effort L, confidence High. Citations: GJF
  visitor:1501, :1618, :1629; PJF visitor:1386.
- Field declaration shapes: GJF and PJF format fields through
  `visitVariable`/`visitVariables`, then `declareOne` or `declareMany`, carrying
  modifier/annotation policy, initializer wrapping, array dimensions, and
  semicolon handling. Effort M, confidence High. Citations: GJF visitor:1048,
  :1057, :3635, :3832; PJF visitor:959, :965, :1932.
- Constructor declaration shape: GJF and PJF use `visitMethod` for constructors,
  detecting `<init>` and compact record constructors, omitting return type,
  supporting receiver/formals/throws/dims, and choosing semicolon versus body
  output. Effort L, confidence High. Citations: GJF visitor:1501, :1597, :1602,
  :1618, :1656; PJF visitor:1386.

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

Upstream evidence and estimates:

- Block statement shapes: GJF formats blocks with `visitBlock`, explicit
  blank-line policy, `visitStatements`, local-variable-fragment grouping, and
  `visitStatement` for braced versus unbraced statement bodies; PJF mirrors this
  and adds an `inlineFirst` variant for some call sites. Effort M, confidence
  High. Citations: GJF visitor:2320, :2363, :2383; PJF visitor:2117, :2180.
- Local variable declaration shape: GJF and PJF group adjacent javac variable
  fragments in statement lists and format them through the same declaration
  helpers used for fields, including annotations, dims, initializers, and
  semicolons. Effort M, confidence High. Citations: GJF visitor:2383, :2395,
  :1048, :3832; PJF visitor:2180, :2197, :959.

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

Upstream evidence and estimates:

- Method invocation shapes: GJF formats invocations as flattened dot chains;
  `visitMethodInvocation` delegates to `visitDot`, which classifies prefixes and
  emits type arguments, arguments, and parens via dedicated helpers. PJF uses
  the same chain model with Palantir breakability controls. Effort L, confidence
  High. Citations: GJF visitor:1688, :3019, :3294, :3379, :3406; PJF
  visitor:1547, :2686, :3040, :3134, :3160.
- Method invocation receivers: GJF flattens member select, invocation, and array
  access receiver chains, including primary-expression prefixes, type-name
  prefixes, `this`/`super`, and stream prefixes; PJF keeps the same model with
  additional chain-fitting knobs. Effort L, confidence High. Citations: GJF
  visitor:3019, :3073, :3124, :3144, :3223; PJF visitor:2686, :2823, :2839,
  :2932.
- Multiline literals: GJF preserves literal source text and has text-block
  handling for indentation/deindentation before emitting the token; PJF has the
  same general literal-token path. Effort M, confidence Medium. Citations: GJF
  visitor:1782; PJF visitor:1638.
- Lambda expressions: GJF formats lambda parameters through variable declaration
  helpers, emits `->`, and chooses block versus expression-body indentation
  separately; PJF adds custom break behavior for lambda arguments/body. Effort
  M, confidence High. Citations: GJF visitor:1354, :1368, :1387; PJF
  visitor:1221, :1228.
- Method reference expressions: GJF emits the qualifier, a break before `::`,
  optional type arguments, then either the member name or `new`; PJF applies
  Palantir inline-suffix breakability to the same structure. Effort S,
  confidence High. Citations: GJF visitor:1025; PJF visitor:922.
- Array creation expressions: GJF formats `new` array base types, dimensions
  with annotations, and optional initializers; initializer lists support empty,
  tabular, filled, unified, forced-trailing-comma, and annotation-array cases.
  PJF mirrors this with Palantir breakability settings. Effort M, confidence
  High. Citations: GJF visitor:504, :522, :535, :590, :596; PJF visitor:454,
  :485.
- Object creation expressions: GJF handles optional enclosing expressions,
  constructor type arguments, anonymous-class modifiers, constructor arguments,
  and anonymous class bodies through shared class-body formatting; PJF uses the
  same structure with Palantir breakability for `new` expressions. Effort M,
  confidence High. Citations: GJF visitor:725, :735, :743, :746; PJF
  visitor:687, :705, :710, :713.
- Conditional expressions: GJF and PJF emit condition, `?`, true expression,
  `:`, and false expression in one indented group with break opportunities
  around the operators. Effort S, confidence High. Citations: GJF visitor:753;
  PJF visitor:719.
- Class literal expressions: GJF and PJF handle `Type.class` through the
  member-select/dot pipeline, so coverage depends on both type formatting and
  receiver-chain formatting. Effort M, confidence Medium. Citations: GJF
  visitor:1775, :3019; PJF visitor:1631, :2686.
- Array access expressions: GJF and PJF treat array access as a selector in the
  dot pipeline, using array-base/index extraction so chained calls, fields, and
  indexes share one receiver path. Effort M, confidence High. Citations: GJF
  visitor:497, :3019, :3028, :3066; PJF visitor:447, :2686.

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

Upstream evidence and estimates:

- Multiline block comments: GJF routes every comment through
  `JavaCommentsHelper.rewrite`; block/Javadoc comments either use
  `JavadocFormatter`, Javadoc-shaped indentation, or preserved relative
  indentation. PJF follows the same preserve-or-indent policy without newer
  markdown-Javadoc handling. Effort M, confidence High. Citations: GJF
  comments:41, :64, :71, :149; PJF comments:45, :60, :69, :159.
- Non-own-line leading comments: GJF's separate token stream assigns non-tokens
  before/after tokens; rules use `sync` and token emission to place skipped
  trivia, while `tokenBreakTrailingComment` gives selected tokens special
  trailing block/Javadoc handling. PJF follows the same token/comment model.
  Effort L, confidence Medium. Citations: GJF formatter:37; GJF visitor:4077,
  :4095; PJF comments:45.
- Unhandled comment or ignored trivia: GJF lexes input separately from javac,
  assigns non-tokens to adjacent tokens/EOF, attaches comments while building
  docs, and rewrites them with `JavaCommentsHelper`; Jolt needs comparable
  formatter-owned trivia accounting rather than rule-local skips. Effort XL,
  confidence Medium. Citations: GJF formatter:37, :102; GJF comments:41.

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
After each step, run the full oracle harness and use the generated blocker
reports to choose the next highest-impact coverage gap.

1. Add the Java wrapping substrate before expanding list-bearing syntax:
   - `JavaFormatContext` state for groups, markers, comments, and profile,
   - policy-bearing helpers for declarations, lists, chains, binary expressions,
     annotations, blocks, and blank lines,
   - narrow-width tests that force multiline output,
   - trailing-comment tests proving `line_suffix` affects fitting.
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
9. Expressions: selector chains, object/array creation, class literals, casts,
   conditionals, lambdas, method references, patterns and switch expressions,
   and multiline literals.

10. Milestone 8 coverage closeout:
    - drive `missing-rule blocked` and `other blocked` to zero,
    - keep exact-match percentage and aggregate diff size reporting so later
      compatibility milestones can drive policy diffs down,
    - do not require 100% google-java-format exact matches for Milestone 8,
    - keep import sorting, modifier ordering, suppression comments, and range
      formatting explicitly scoped unless they are required to format all valid
      Java source without blocked output.

## Tracking Checklist

Milestone 8 remains open until all of these are true:

- [ ] `missing-rule blocked: 0` for the pinned valid google-java-format corpus.
- [ ] `other blocked: 0` for the pinned valid google-java-format corpus.
- [ ] `formatted` equals the number of valid corpus files.
- [ ] exact-match percentage and aggregate diff size are still reported for the
      pinned valid google-java-format corpus, but are not Milestone 8 gates.
- [ ] every valid-Java `missing_layout` call site in `jolt_java_fmt` has been
      removed or proven unreachable for clean parser output.
- [ ] broad shape guards no longer block valid Java grammar families.
- [ ] Java wrapping helpers exist for declarations, lists, chains, binary
      expressions, comments, and blank-line-sensitive blocks before those
      grammar families report formatted output.
- [ ] narrow-width tests force multiline behavior for every wrapping helper that
      can affect oracle output.
- [ ] parser diagnostics still block formatting without output.
- [ ] formatter output remains comment/trivia-accounted.
- [ ] targeted tests cover each Java grammar family, not only oracle fixtures.
- [ ] `mise run test` passes.
