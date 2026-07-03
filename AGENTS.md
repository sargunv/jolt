# Jolt

Fast, opinionated JVM and Kotlin Multiplatform project tooling.

## Project Map

- `.agents/docs/VISION.md`: product vision.
- `.agents/docs/formatter-plan.md`: formatter architecture notes.
- `crates/`: Rust workspace crates for the formatter engine and wrappers.

## Useful Commands

- `mise run fix`: run all checks and fixers through hk.
- `mise run test`: run all tests.
- `mise run test --update`: run tests with `INSTA_UPDATE=always`.
- `mise run jolt ...`: run the Jolt CLI from local source.
- `mise run dprint-with-jolt ...`: run the dprint cli with the jolt formatter
  plugin.
- `mise x google-java-format -- google-java-format ...`: run the
  google-java-format formatter as a useful reference.
- `mise x oxfmt -- oxfmt ...`: run the oxfmt formatter as a useful reference.

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
- Algorithms must remain linear or explicitly bounded; do not add unbounded
  layout search, best-fitting, or conditional-group behavior without a
  documented finite cost model and proven need.
