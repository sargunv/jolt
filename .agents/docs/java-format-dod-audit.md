# Jolt Java Formatter Definition Of Done Audit

This report records current evidence for the Java formatter Definition of Done
in [`java-format-implementation-spec.md`](java-format-implementation-spec.md).
It is an audit snapshot, not a replacement for the implementation checklist.

## Evidence Commands

- `cargo fmt --check`
- `cargo test -p jolt_fmt_ir`
- `cargo clippy -p jolt_java_fmt -p jolt_java_syntax --all-targets -- -D warnings`
- `cargo test -p jolt_java_fmt -p jolt_java_syntax`

## Audit Items

### All Style-Guide Rule Fixtures Pass

Status: proven.

Evidence:

- `crates/jolt_java_fmt/tests/style_fixtures.rs` discovers every `*.input.java`
  fixture under `crates/jolt_java_fmt/tests/style`.
- The latest verification passed `style_rule_fixtures_match_expected_output`.
- Current fixture count: 74 style input fixtures.

### Every Style-Guide Rule Has One Or More Focused Tests

Status: proven for the current style-guide documents.

Evidence:

- Focused style fixtures exist across every style-guide domain: comments,
  declarations, expressions, imports, modules, program structure, and
  statements.
- Empty/comment-only file handling and final-newline behavior are pinned by
  `program/empty-file`, `program/comment-only-file`, and
  `program/final-newline`.
- Program section spacing, redundant top-level semicolon removal, and top-level
  blank-line collapse are pinned by `program/package-import-spacing`,
  `program/removes-top-level-semicolons`, and `program/top-level-blank-lines`.
- Package annotations and package-name normalization are pinned by
  `program/package-import-spacing` and `program/qualified-names`.
- Qualified-name dot tightening, block comments around dots, and line-comment
  forced leading-dot layout are pinned by `program/qualified-names`,
  `program/qualified-name-comments`, and `modules/qualified-names`.
- Star-block normalization, unsupported branded ignore comments, formatter
  ignore ranges, text-block preservation, and string/character literal
  preservation are pinned by `comments/star-blocks`,
  `program/unsupported-branded-ignore`, the `formatter-ignore-*` fixtures, and
  `expressions/literals-and-text-blocks`.
- Formatter ignore ranges are covered for block-statement sequences by
  `statements/formatter-ignore-block`, constructor-body sequences by
  `declarations/formatter-ignore-constructor-body`, class-member sequences by
  `declarations/formatter-ignore-members`, interface-member sequences by
  `declarations/formatter-ignore-interface-members`, annotation-interface member
  sequences by `declarations/formatter-ignore-annotation-interface-members`,
  top-level end-of-file sequences by `program/formatter-ignore-top-level`,
  top-level next-item boundaries by
  `program/formatter-ignore-top-level-next-item`, and module directive ranges by
  `modules/formatter-ignore-directives`.
- Moved/normalized construct-leading comments are pinned for top-level type
  declarations by `declarations/type-leading-comments`, parameters/components by
  `declarations/parameter-and-component-comments`, and callable members plus
  constructor invocations by `declarations/member-leading-comments`.
- Canonical declaration modifier sorting, including contextual `sealed` and
  `non-sealed`, plus comments attached to sorted modifier tokens, is pinned by
  `declarations/modifiers-and-annotations`.
- Switch colon groups whose only body is a block are pinned by
  `statements/switch-groups-and-rules`.
- Qualified-name dot comments and line-comment forced leading-dot layout are
  pinned by `program/qualified-name-comments`.
- Module directive ordering, grouping, canonical `requires` modifier ordering,
  target-list breaking, and ordinary comment barriers are pinned by
  `modules/directive-ordering`, `modules/name-list-comments`, and
  `modules/comment-barrier`; case-sensitive module-name comparison is pinned by
  `modules/case-sensitive-sorting`.
- Import ordering, normal/static grouping, module-import handling, star segment
  ordering, case-sensitive comparison, and comment barriers are pinned by
  `imports/normal-and-static-groups`, `imports/star-import-sorting`,
  `imports/case-sensitive-sorting`, and `imports/comment-barrier`.
- Declaration headers, clause ordering, broken-header brace placement,
  parameters/components, receiver parameters, varargs annotations, `throws`,
  constructors, compact record constructors, enum constants, annotation
  interface members/defaults, type parameters/arguments, wildcards, annotated
  dimensions, and empty type-body statement removal are pinned by the
  declaration fixtures under `declarations/`.
- Body blank-line preservation/capping, same-kind method separation, empty
  blocks, comments-only blocks, local type declarations, unbraced-body
  normalization, empty statement removal, labels, loops, `for` headers, switch
  labels/rules/guards, jump/assert statements, try/catch/finally chains, and
  try-with-resources resource lists are pinned by the statement fixtures under
  `statements/`.
- Complex-receiver member chains and blank-line normalization in member chains
  are pinned by `expressions/member-chains`; blank-line normalization in
  argument lists is pinned by `expressions/calls-and-arguments`.
- Empty expression/list array initializers and non-empty initializer list
  formatting are pinned by `expressions/array-access-and-creation`.
- Parentheses preservation, binary/operator chains, ternaries, assignments,
  calls/arguments, member chains, lambdas, method references, casts,
  `instanceof`, patterns, object creation, anonymous class bodies, constructor
  type arguments, array access/creation, and class literals are pinned by the
  expression fixtures under `expressions/`.
- Java template expressions remain outside the initial formatter scope by style
  policy; parser-accepted syntax must still be formatted by a real rule, and the
  imported-corpus harness rejects formatter diagnostics for accepted inputs.

### Formatting Expected Fixtures Is Idempotent

Status: proven.

Evidence:

- `style_rule_fixtures_match_expected_output` formats every expected fixture and
  asserts the result is unchanged.

### Imported Java Fixture Inputs Format Without Formatter Panics

Status: proven.

Evidence:

- `imported_fixture_inputs_format_idempotently_and_parse` formats the pinned
  imported fixture corpora.
- The test rejects formatter-stage diagnostics for every imported input.

### Formatted Imported Fixtures Parse

Status: proven.

Evidence:

- `imported_fixture_inputs_format_idempotently_and_parse` reparses formatted
  output with `parse_compilation_unit` and asserts a clean parse with syntax.

### Repeated Formatting Is Deterministic

Status: proven.

Evidence:

- `style_rule_fixtures_match_expected_output` repeats formatting of each style
  input and compares the same output.
- `imported_fixture_inputs_format_idempotently_and_parse` repeats formatting of
  each imported input and compares the same output.

### No Parser-Accepted Syntax Reaches An Unimplemented Formatter Fallback

Status: proven for the public formatter path.

Evidence:

- `format_source` returns before layout when parsing is not clean.
- `declaration_recovery_nodes_do_not_reach_layout` covers remaining declaration
  recovery shapes with missing required names.
- Raw token-sequence declaration branches remain only for recovery trees that
  the public formatter does not lay out.

### Shared Renderer Fit Probes Are Bounded

Status: proven for the current document algebra.

Evidence:

- `fit_checks_use_nested_current_group_state` covers group fit probes pushing
  the measured group as the current flat group.
- `deeply_nested_fitting_groups_render_without_exploration` covers deep nested
  group fitting without nested group exploration.
- `cargo test -p jolt_fmt_ir` covers the shared renderer algebra.

### Formatting Choices Are Traceable To The Style Guide Or Spec

Status: incomplete audit.

Evidence:

- The implementation checklist records broad rule coverage and no permanent
  intentional deviations.

Remaining work:

- Review each formatter helper/rule against the style-guide documents and this
  spec, then record any permanent intentional deviations.
