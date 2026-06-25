# Jolt

Fast, opinionated JVM and Kotlin Multiplatform project tooling.

## Project Map

- `docs/`: product and architecture notes.
- `crates/`: Rust workspace crates for the formatter engine and wrappers.

## Dev Tool Commands

- `mise run check`: run all checks through hk.
- `mise run fix`: run all fixers through hk.
- `cargo check --workspace`: typecheck the Rust workspace.
- `cargo test --workspace`: run Rust tests.

Run `mise tasks ls --all` for the full task list.

## Project Invariants

<!-- Add concise invariants to this section when the user asks you to always or never do something. -->
