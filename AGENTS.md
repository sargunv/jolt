# Jolt

Fast, opinionated JVM and Kotlin Multiplatform project tooling.

## Project Map

- `.agents/docs/VISION.md`: product vision.
- `.agents/docs/formatter-plan.md`: formatter architecture notes.
- `crates/`: Rust workspace crates for the formatter engine and wrappers.

## Dev Workflow

- `mise run fix`: run all checks and fixers through hk.
- `mise run test`: run all tests.

Run `mise tasks ls --all` for the full task list.

## Project Invariants

<!-- Add concise invariants to this section when the user asks you to always or never do something. -->

- Do not add tests that only duplicate source definitions, such as pinning enum
  defaults or simple accessors, unless they're grounded in a formal
  specification.
- Tests must fail on missing required fixtures or other test environment
  misconfiguration; do not silently skip them.
- Do not add convenience APIs unless they carry real behavior needed by current
  code.
- Formatter rendering must stay linear or explicitly bounded; do not add
  unbounded layout search, best-fitting, or conditional-group behavior without a
  documented finite cost model and proven Java/Kotlin need.
- Do not add workarounds for unimplemented functionality; instead, just
  implement the functionality itself. Look at how ruff/biome/oxc solve the
  problem (.fixtures/repos/*) and learn from them.
- Proper trivia handling is critical when working on the formatter. Comments are
  likely to exist in between any token; don't lose the trivia by recreating the
  tokens. Don't treat "code with comments" as an exception to layout rules.
