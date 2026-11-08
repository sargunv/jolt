# Jolt

Fast, opinionated Java formatter.

## Project Map

- `crates/`: Rust workspace crates for the formatter engine and wrappers.
  `jolt_cli` (the `jolt` binary) and `jolt_fmt_dprint` (the dprint plugin) use
  only the `jolt_formatter` facade and `jolt_config`, the one definition of the
  user-facing format options.
- `tools/`: corpus import (`import/import.py`), the baseline outputs
  (`baseline.sh`), the profile-guided release build (`pgo.py`), the optimized
  plugin build (`wasm.sh`), and the benchmark (`bench.py`).
- `docs/internals/formatter.md`: the formatter design (crates, contracts,
  comment model, verification). Signatures and per-item contracts live in
  rustdoc, which `mise run check` builds with warnings denied.
- `docs/guides/style.md`: the Java output, rule by rule. A test formats its
  examples (`jolt_java_fmt/tests/style_guide.rs`).

## Useful Commands

- `mise run fix`: run all checks and fixers through hk.
- `mise run test`: run all tests.
- `mise run test --update`: run tests with `INSTA_UPDATE=always`.
- `mise run jolt ...`: run the Jolt CLI from local source.
- `mise run hunt`: sweep the repository fixtures and the imported corpora
  (tools/import/.imports) for formatter bugs. Formats every file at several line
  widths, and with a comment injected at token boundaries in each shape of
  `PROBE_SHAPES` — on the boundary's own line, so it lands in the previous
  token's trailing trivia, and on a line of its own, so it lands in the next
  token's leading trivia — then checks reparse validity, structure, token and
  comment conservation, and idempotence. The fixtures and the gjf, palantir and
  prettier inputs up to 4 KiB get every boundary at every width; larger files
  and Spring are sampled. Every one-token deletion of those small files is
  checked too, which stresses parser recovery and the formatting of recovered
  trees (`jolt format --format-with-errors`). Positions inside a string literal
  are excluded, since text there is content rather than trivia. Runs in release
  mode; too slow in debug for the normal test suite. Reports land in
  `target/hunt/`, capped per failure category. Exits non-zero on any failure
  category not in `KNOWN_OPEN`, which must stay empty, and on a `KNOWN_OPEN`
  entry that stops firing. The `jcheck` example re-runs one file, or one variant
  for a repro, e.g. `.../File.java:ownline12`, `.../File.java:comment3@40` or
  `.../File.java:delete7`.
- `tools/baseline.sh build [revision]` formats the imported corpora with the CLI
  built at a revision (default `origin/main`) into
  `tools/import/.imports/.baseline/`, and `tools/baseline.sh compare <binary>`
  counts the files another build formats differently. Review those differences
  before changing output, and describe them in the PR.
- `mise run jref ...`: compare the formatter with expected outputs (`jref <dir>`
  checks `format(F) == F` for `.java` files and `.snap` `formatted:` sections;
  `jref --input <in> --expected <out>` checks `format(in) == out`).
- `mise run dprint-with-jolt ...`: run the dprint cli with the jolt formatter
  plugin.
- `mise run bench`: time formatting the Spring corpus with hyperfine and print
  the medians; `mise run bench --compare` adds google-java-format and
  prettier-java and writes `docs/bench.json`, which the docs show. Rerun it on a
  quiet machine before committing the numbers.
- `mise run build:jolt --mode release` and
  `mise run build:dprint-plugin --mode release`: the PGO-built CLI in
  `target/release/jolt` and the optimized, smoke-tested plugin in
  `target/plugin.wasm`.
- `mise x aqua:google/google-java-format -- google-java-format ...`: run the
  pinned google-java-format as a useful reference.

Run `mise tasks ls --all` for the full task list.

## Documentation

Documentation describes Jolt as it is, for a reader who does not know its
history. Change it when a code change makes existing text wrong, and fix that
text in place. Notes about what changed ("now", "no longer", "before"), issue
numbers and comparisons with earlier output belong in the PR description, where
the reviewer who knows the old behavior reads them.

- `docs/guides/style.md` states the output for users: rules, then examples. When
  an output change doesn't fit a rule, change the rule in the same PR rather
  than adding a special case to the formatter. Compare with other formatters
  only under "Compared with google-java-format", and check those claims by
  running google-java-format.
- `docs/internals/formatter.md` explains the design to contributors: the crates,
  their contracts, and why each mechanism exists.
- Many readers know Java but not formatter internals, and read English as a
  second language, so say what the formatter does in literal terms.

## Project Invariants

<!-- Add concise invariants to this section when the user asks you to always or never do something. -->

- Do not add tests that only duplicate source definitions, such as pinning enum
  defaults or simple accessors, unless they're grounded in a formal
  specification.
- Tests must fail on missing required fixtures or other test environment
  misconfiguration; do not silently skip them.
- Do not add convenience APIs unless they carry real behavior needed by current
  code.
- Remove legacy code as you go; do not design for backwards compatiblity. This
  is a pre-release project.
- Algorithms must remain linear or explicitly bounded; do not add unbounded
  layout search, best-fitting, or conditional-group behavior without a
  documented finite cost model and proven need.
- Formatter syntax access must use borrowed source, tokens, nodes, and trivia
  from the existing parser buffers. Do not clone source text, token buffers, or
  syntax nodes to rediscover structure or probe recovery cases.
- Formatter layout must be structured. Every representable tree has a
  representable structure; format that tree and its available children instead
  of replaying source, walking token cursors, or inferring syntax rules in the
  formatter.
- A formatter must not panic, refuse, or validate syntax for
  malformed-but-represented trees. Format the represented pieces consistently
  and preserve all existing tokens/trivia.
- Formatter rules emit source tokens only through `Formatter::token`, nodes only
  through `Formatter::node`/`fmt`, and comments only through the chokepoints
  (`node`, `token`, `verbatim`, `removed`, `dangling`, and
  `leading_comments`/`trailing_comments` for the rules that build a node's doc
  themselves: spines, the `else if` chain, N7 parentheses and the declaration
  header). Rules never choose how a comment prints; comment placement is decided
  once, before layout, by `jolt_fmt`'s placement pass, and rules ask it
  (`leading_comments_break` and the other queries) instead of reading trivia.
  Debug builds assert every comment is emitted exactly once.
- Normalizations are the closed list N1–N10 in `docs/internals/formatter.md`,
  applied only to recovery-free nodes.
- Formatter rules must not parse by inspecting token streams. Syntax crates own
  tree shape, recovery accessors, and token ownership; formatter crates consume
  those accessors for layout only.
- A formatter must not synthesize tokens to repair invalid syntax or where
  source tokens (with trivia) are available. Synthesized tokens are allowed in
  very specific cases, like normalizing separators, braces, or parentheses where
  semantics don't change and trivia won't be lost. Formatter rules must preserve
  represented source tokens and their trivia; use the standard token formatting
  helpers unless a documented normalization case requires otherwise.
- A formatter may emit a syntax-owned malformed/bogus subtree verbatim, and may
  emit formatter-ignore ranges verbatim. Valid syntax must always use structured
  formatting: formatter failure, a missing formatter accessor, or an unhandled
  valid node must never select verbatim output. The parser/syntax layer owns
  malformed classification, and verbatim formatting must track every contained
  token and comment so the surrounding structured formatter cannot lose or
  duplicate them.
- Prefer integration tests with `insta` snapshots over inline tests and
  assertions where practical. Inline focused tests should be reserved only for
  important regressions and edge cases that are not possible to test with the
  integration pattern. For example, a single Java fixture corpus is used as
  input to the syntax and formatter crates, and their outputs are snapshotted
  with `insta`.
