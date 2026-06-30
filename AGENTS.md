# Jolt

Fast, opinionated JVM and Kotlin Multiplatform project tooling.

## Project Map

- `.agents/docs/VISION.md`: product vision.
- `.agents/docs/formatter-plan.md`: formatter architecture notes.
- `.oracles/fixtures/`: imported test fixtures for the formatter
- `.oracles/reports/`: formatter per-fixture reports from the latest test run
- `crates/`: Rust workspace crates for the formatter engine and wrappers.

## Dev Workflow

- `mise run fix`: run all checks and fixers through hk.
- `mise run test`: run tests without updating snapshots
  (`INSTA_UPDATE=no cargo test`).
- `mise run test-update`: run tests and update snapshots
  (`INSTA_UPDATE=always cargo test`).

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
- Do not add formatter fallback exits for parser-accepted syntax.
  Parser-accepted syntax must either be invalidated by the parser or receive a
  real formatting rule.
