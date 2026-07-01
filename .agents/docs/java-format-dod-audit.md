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
- Current fixture count: 69 style input fixtures.

### Every Style-Guide Rule Has One Or More Focused Tests

Status: incomplete audit.

Evidence:

- Focused style fixtures exist across comments, declarations, expressions,
  imports, modules, program structure, and statements.
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
- Complex-receiver member chains and blank-line normalization in member chains
  are pinned by `expressions/member-chains`; blank-line normalization in
  argument lists is pinned by `expressions/calls-and-arguments`.
- Empty expression/list array initializers and non-empty initializer list
  formatting are pinned by `expressions/array-access-and-creation`.
- Unsupported branded ignore spellings are pinned as ordinary comments by
  `program/unsupported-branded-ignore`.

Remaining work:

- Cross-check each rule bullet in the style-guide documents against at least one
  fixture case before marking this proven.
- Formatter ignore ranges are covered for block-statement sequences by
  `statements/formatter-ignore-block`, constructor-body sequences by
  `declarations/formatter-ignore-constructor-body`, class-member sequences by
  `declarations/formatter-ignore-members`, interface-member sequences by
  `declarations/formatter-ignore-interface-members`, and annotation-interface
  member sequences by
  `declarations/formatter-ignore-annotation-interface-members`, top-level
  end-of-file sequences by `program/formatter-ignore-top-level`, and top-level
  next-item boundaries by `program/formatter-ignore-top-level-next-item`. Module
  directive ranges are covered by `modules/formatter-ignore-directives`.

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
