# Jolt Java Formatter Definition Of Done Audit

This report records current evidence for the Java formatter Definition of Done
in [`java-format-implementation-spec.md`](java-format-implementation-spec.md).
It is an audit snapshot, not a replacement for the implementation checklist.

## Evidence Commands

- `cargo fmt --check`
- `cargo clippy -p jolt_java_fmt -p jolt_java_syntax --all-targets -- -D warnings`
- `cargo test -p jolt_java_fmt -p jolt_java_syntax`

## Audit Items

### All Style-Guide Rule Fixtures Pass

Status: proven.

Evidence:

- `crates/jolt_java_fmt/tests/style_fixtures.rs` discovers every `*.input.java`
  fixture under `crates/jolt_java_fmt/tests/style`.
- The latest verification passed `style_rule_fixtures_match_expected_output`.
- Current fixture count: 52 style input fixtures.

### Every Style-Guide Rule Has One Or More Focused Tests

Status: incomplete audit.

Evidence:

- Focused style fixtures exist across comments, declarations, expressions,
  imports, modules, program structure, and statements.

Remaining work:

- Cross-check each rule bullet in the style-guide documents against at least one
  fixture case before marking this proven.
- Formatter ignore ranges remain an uncovered style-guide rule.

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

### Formatting Choices Are Traceable To The Style Guide Or Spec

Status: incomplete audit.

Evidence:

- The implementation checklist records broad rule coverage and no permanent
  intentional deviations.

Remaining work:

- Review each formatter helper/rule against the style-guide documents and this
  spec, then record any permanent intentional deviations.
