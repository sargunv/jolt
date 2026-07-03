# Formatter Performance Investigation

## Summary

Jolt's native formatter is not fundamentally losing because Rust is slow or
because the formatting engine burns more CPU than JVM/Node competitors. The
current realistic benchmark mostly exposes two different problems:

- The native CLI formats files serially.
- The formatter pipeline allocates and clones heavily inside syntax traversal
  and layout construction/rendering.

On the Spring Framework corpus, `jolt fmt` uses roughly one core for about ten
seconds. `google-java-format` reaches similar wall time while burning much more
aggregate CPU, and `dprint` reaches much lower wall time by parallelizing Jolt's
wasm plugin across files.

The first performance win should be CLI-level parallelism. The deeper engine
work should target data shape: fewer red syntax handle clones, fewer parser
token/trivia clones, and a more compact document/layout representation.

## Evidence

Benchmark reports in `tools/bench/reports`:

| Corpus      |                   Tool | Wall time |   User CPU | Notes                                                        |
| ----------- | ---------------------: | --------: | ---------: | ------------------------------------------------------------ |
| realistic   |             `jolt fmt` |  10.319 s |    9.443 s | Native CLI, effectively serial                               |
| realistic   |   `dprint` + Jolt wasm |  847.1 ms | 17065.3 ms | Same core formatter through dprint, parallel                 |
| realistic   |   `google-java-format` |  11.191 s |   64.210 s | Parallel JVM formatter                                       |
| realistic   | `prettier-plugin-java` |  564.0 ms |   720.6 ms | Needs equivalence checks before treating as apples-to-apples |
| adversarial |             `jolt fmt` |   63.9 ms |    57.0 ms | Small corpus, startup/discovery matter more                  |
| adversarial |   `dprint` + Jolt wasm |   30.5 ms |    93.0 ms | Parallelism still helps wall time                            |

Local corpus sizes at the time of investigation:

- `realistic`: 9,136 Java files, 69 MB.
- `adversarial`: 207 Java files, 872 KB.

Additional observations:

- Re-running native `jolt fmt` over an already-formatted Spring tree still took
  about 10.5 seconds, so output writing is not the main cost.
- `dprint output-file-paths` sees all 9,136 realistic Java files when invoked
  from the benchmark working directory.
- `perf stat` on representative inputs showed about 0.9-1.0 CPUs utilized by
  native `jolt fmt`, confirming serial execution.

Single-large-file `perf record` sample highlights:

- `jolt_syntax::red::node::SyntaxNode<L>::children_with_tokens::{closure}`:
  about 25% self in the sample.
- `jolt_java_syntax::parser::source::TokenCursor::token`: about 6%.
- `jolt_fmt_ir::render::FitChecker::fits_stack`: about 6%.
- `jolt_java_syntax::lexer::JavaLexer::next_token`: about 5%.
- Red/green handle drops and `Arc`/`Rc` refcount traffic also showed up.

Callgrind on the same large file showed allocator internals as a visible slice
of total instructions, with `_int_malloc` around 11%. The exact percentage is
less important than the direction: current formatting creates many short-lived
objects.

## Ranked Opportunities

Tags:

- Impact: `H`, `M`, `L`.
- Effort: `S`, `M`, `L`.
- Confidence: `H`, `M`, `L`.

### 1. Parallelize Native CLI File Formatting

Tags: Impact `H`, effort `M`, confidence `H`.

The native CLI formats candidates one at a time in
`crates/jolt_fmt_cli/src/run.rs`. This explains most of the wall-clock gap
against dprint on the realistic corpus.

Likely shape:

- Discover and sort candidates deterministically on the main thread.
- Resolve config before parallel execution, or make config resolution/cache safe
  to share.
- Format files in parallel with a bounded worker pool.
- Preserve deterministic diagnostics and changed-file reporting by collecting
  outcomes and emitting in candidate order.
- Keep `stdin` formatting serial.

Things to watch:

- Do not interleave diagnostics from workers.
- Avoid unbounded parallelism on huge repos.
- Decide whether writes happen in workers or after collecting formatted output.
  Worker writes reduce peak memory; collected writes make ordering simpler.

Suggested first experiment:

- Add a `rayon`-backed internal path for directory candidates.
- Benchmark realistic/adversarial with `--threads=1`, default threads, and a few
  fixed thread counts.

### 2. Make Benchmark Semantics Stricter

Tags: Impact `H` for measurement quality, effort `S`, confidence `H`.

Before using benchmark numbers as product claims, tighten equivalence checks.
The current benchmark is useful for investigation, but formatter comparisons can
be misleading because tools differ on file matching, stability checks, write
behavior, diagnostics, and parallelism.

Add or verify:

- File count each tool sees before timing.
- Number of files actually changed.
- Exit status and stderr capture.
- Output byte count and maybe checksum per tool directory.
- Cold/warm variants if startup is relevant.
- A native Jolt `--check` benchmark to separate formatting CPU from write cost.
- A native Jolt single-file benchmark to separate engine cost from discovery.

Suggested first experiment:

- Extend `tools/bench/bench.py` to write metadata next to each report: file
  count, total bytes, changed files, output checksum, command exit code, and
  tool version.

### 3. Reduce Red Syntax Traversal Handle Churn

Tags: Impact `H`, effort `M-L`, confidence `H`.

`SyntaxNode::children_with_tokens` in `crates/jolt_syntax/src/red/node.rs`
creates fresh red node/token wrappers while walking children. Perf showed this
as the hottest symbol on a large file. The current representation is ergonomic,
but it pays with repeated `Rc` parent clones, `Arc` green clones, red wrapper
allocation, and drop traffic.

Possible directions:

- Add lighter child cursors/views that borrow the parent and green child rather
  than allocating a new parent-aware red object for every iteration.
- Provide specialized sibling/token accessors that avoid rebuilding all
  preceding wrappers just to find nearby structure.
- Cache red children per node if repeated traversal dominates and memory stays
  acceptable.
- Let formatter rules traverse green children plus offset information for hot
  paths, using typed wrappers only at rule boundaries.

Things to watch:

- Parent-aware APIs are valuable for comments and sibling lookups. Do not erase
  them blindly.
- Borrowed views may make formatter code more lifetime-heavy.
- Caching red children can trade CPU for memory and may interact with wasm.

Suggested first experiment:

- Instrument counts for `children_with_tokens`, `SyntaxNode::new_child`, and
  `SyntaxToken::new` over realistic.
- Prototype a borrowed `children_with_tokens_view` for one hot formatter path
  and compare perf.

### 4. Stop Cloning Parser Tokens and Trivia for Lookahead

Tags: Impact `M-H`, effort `M`, confidence `H`.

`TokenCursor::token` and `logical_token_at` in
`crates/jolt_java_syntax/src/parser/source.rs` return owned `ParserToken`s. That
means ordinary lookahead can clone `Vec<Trivia>`. Perf showed this path clearly.

Possible directions:

- Return borrowed token views for `token`, `range`, and `text`.
- Keep owned token creation for `bump`, `bump_split_gt`, and tree emission.
- Represent virtual split `>` tokens as a small enum/view until they are bumped.
- Store trivia in shared slices or ranges rather than per-token `Vec` clones.

Things to watch:

- Split `>`, `>>`, and `>>>` handling needs careful ownership boundaries.
- Parser checkpoints/forks must stay cheap.
- The tree sink still needs stable token/trivia data when building the green
  tree.

Suggested first experiment:

- Change only non-consuming cursor accessors to borrowed views and measure the
  large-file profile.

### 5. Rework Document IR into an Arena or Command Tape

Tags: Impact `H`, effort `L`, confidence `M-H`.

The current document IR in `crates/jolt_fmt_ir/src/document.rs` is a recursive
owned tree:

- `Concat(Vec<Doc>)`
- `Group(Box<Doc>)`
- `Indent(Box<Doc>)`
- `Align(Box<Doc>)`
- `IfBreak(Box<Doc>, Box<Doc>)`
- `LineSuffix(Box<Doc>)`

Java formatting rules return `Doc`, and helpers frequently compose with
`concat([...])`. This creates many small vectors, boxes, clones, and drops.

A better long-term shape is probably not pure direct streaming to final output.
The renderer still needs delayed layout decisions for groups, flat-vs-break
lines, `if_break`, `indent_if_break`, comments, and line suffixes. Instead, use
a compact intermediate representation:

- An arena of layout nodes, or
- A linear command tape with group/body ranges.

Potential benefits:

- Fewer small heap allocations.
- Better cache locality.
- Less recursive drop work.
- Cheaper fit checks over contiguous command ranges.
- Easier precomputed flat-width or "contains hard break" metadata.

Possible API direction:

```text
FormatRule<N>::fmt(&self, node: &N, formatter: &mut JavaFormatter, out: &mut LayoutBuilder)
```

instead of:

```text
FormatRule<N>::fmt(&self, node: &N, formatter: &mut JavaFormatter) -> Doc
```

Things to watch:

- This is a broad formatter rewrite. It should follow smaller profiling wins,
  not precede CLI parallelism.
- The public `jolt_fmt_ir` API may need migration scaffolding.
- Comments and line suffixes are likely the hardest compatibility test.

Suggested first experiment:

- Build an arena-backed `DocBuilder` that preserves the current renderer
  semantics, then port one isolated rule/helper family to compare allocation
  count and output parity.

### 6. Optimize Fit Checking and Group State

Tags: Impact `M`, effort `M`, confidence `M-H`.

`FitChecker` in `crates/jolt_fmt_ir/src/render.rs` clones renderer state and
walks document structure to decide whether groups fit. The current code uses
`BTreeMap<GroupId, bool>`, stack clones, `Vec<PrintCommand>`, and cloned line
suffix docs.

Possible directions:

- Replace `BTreeMap<GroupId, bool>` with a dense vector if `GroupId` allocation
  is dense and bounded per document.
- Avoid cloning line suffix docs during fit checks.
- Reuse fit stacks rather than allocating fresh vectors.
- Precompute cheap flat widths for command ranges or doc nodes where semantics
  permit.
- Track "definitely breaks" metadata to avoid walking impossible flat groups.

Things to watch:

- Project invariant: no unbounded layout search or conditional-group explosion.
- Fit behavior must stay deterministic and bounded.

Suggested first experiment:

- Add renderer stats for fit-check count, max stack depth, group count, and
  fit-check early exits, then benchmark realistic.

### 7. Reduce Comment and Formatter-Ignore Scans

Tags: Impact `M`, effort `M`, confidence `M`.

The formatter builds comment maps and handles formatter-ignore ranges before or
during formatting. Callgrind attributed about 4% of a large-file run to comment
map construction, and a package sample showed `formatter_ignore_ranges` and
comment normalization.

Possible directions:

- Combine comment collection with syntax/token traversal already needed for
  layout.
- Avoid rebuilding raw source text for subtrees when source ranges suffice.
- Cache comment classification on tokens or trivia.
- Make formatter-ignore range lookup interval-based rather than repeated scans.

Things to watch:

- Comment behavior is correctness-sensitive.
- Some comment formatting requires exact trivia text and original line breaks.

Suggested first experiment:

- Count comment map entries, formatter-ignore ranges, and lookup calls per file.
- Profile a comment-heavy corpus separately from Spring.

### 8. Avoid Unnecessary Full-Tree Text Reconstruction

Tags: Impact `M`, effort `S-M`, confidence `M`.

`jolt_syntax::text::write_node_text` appeared in a package sample. If hot paths
call `node.text()` or compact-text helpers repeatedly, this can reconstruct
subtree text that could be represented by source ranges or token sequences.

Possible directions:

- Prefer original source slices when a node's `TextRange` is available.
- Use compact-name helpers that append into a caller-owned buffer.
- Cache compact names for imports, modules, and qualified names if repeated.

Suggested first experiment:

- Instrument `green_text`/`write_node_text` call counts and total bytes written.

### 9. Tune Lexer Unicode Escape Handling

Tags: Impact `M`, effort `M`, confidence `M`.

`translate_unicode_escapes` showed up in the single-file profile. Java requires
Unicode escape translation before lexical interpretation, so this path cannot
simply disappear.

Possible directions:

- Fast path ASCII/no-backslash input.
- Avoid allocating translated input when no Unicode escapes are present.
- Store a sparse mapping only when translation changes byte positions.

Things to watch:

- Java Unicode escape semantics are odd and early. This needs strong tests.

Suggested first experiment:

- Count files containing `\u` escapes in realistic/adversarial.
- Benchmark a no-escape fast path.

### 10. Release Profile and Allocator Tuning

Tags: Impact `M`, effort `S-M`, confidence `M`.

Once structural issues are addressed or while experiments are cheap, tune the
build and allocator.

Possible directions:

- `lto = "thin"`.
- `codegen-units = 1` for release artifacts.
- `panic = "abort"` if acceptable for CLI/wasm packaging.
- Try `mimalloc` or `jemalloc` for the native CLI only.

Things to watch:

- Wasm plugin size and portability.
- Build time.
- Allocator gains can mask deeper architecture issues.

Suggested first experiment:

- Benchmark realistic with a release profile matrix:
  - current,
  - thin LTO,
  - thin LTO plus single codegen unit,
  - alternate allocator for native CLI.

### 11. Separate Engine Benchmarks from CLI Benchmarks

Tags: Impact `M`, effort `M`, confidence `H`.

The existing benchmark exercises full CLI behavior. That is useful, but it
combines file walking, config resolution, IO, parsing, formatting, rendering,
and writing.

Add micro/macro layers:

- Engine single-file benchmark: `format_source(source, Java, options)`.
- Engine corpus benchmark: in-process loop over source strings.
- Renderer benchmark: build representative docs and render only.
- Parser benchmark: lex/parse/tree only.
- CLI benchmark: current end-to-end behavior.

Suggested first experiment:

- Add Criterion or a custom `cargo bench` harness for the largest Spring files
  and the adversarial corpus.

### 12. Config and Discovery Caching

Tags: Impact `L-M`, effort `S-M`, confidence `M`.

Discovery/config did not look like the primary cost, especially because
already-formatted Spring still took about the same wall time. Still, once
formatting is parallel, config resolution and candidate metadata may become more
visible.

Possible directions:

- Resolve config once per directory and share immutable `ResolvedConfig`.
- Avoid cloning globsets per candidate.
- Keep candidate metadata compact before parallel dispatch.

Suggested first experiment:

- Time discovery/config separately from formatting in `jolt fmt`.

## Suggested Execution Order

1. Tighten benchmark metadata and add phase timing.
2. Parallelize native CLI formatting.
3. Add engine-level benchmarks and allocation counters.
4. Reduce parser token/trivia cloning in lookahead.
5. Prototype cheaper red syntax child traversal.
6. Instrument renderer fit checks.
7. Prototype arena/tape document IR on a small rule surface.
8. Try release profile and allocator tuning.

This order separates quick wall-clock wins from deeper architecture work. The
layout IR redesign is promising, but it should be driven by measurements from
the smaller changes so it does not become a large speculative rewrite.

## Useful Profiling Commands

Representative commands used during the investigation:

```bash
/usr/bin/time -f 'elapsed=%e user=%U sys=%S maxrss=%M' \
  target/release/jolt fmt --no-config target/bench/realistic/jolt

perf stat -d \
  target/release/jolt fmt --no-config /tmp/jolt-perf-one/SpelCompilationCoverageTests.java

perf record -g --call-graph dwarf -F 999 -o /tmp/jolt-one.perf.data -- \
  target/release/jolt fmt --no-config /tmp/jolt-perf-one/SpelCompilationCoverageTests.java

perf report -i /tmp/jolt-one.perf.data --stdio --no-children --sort symbol,dso

valgrind --tool=callgrind --callgrind-out-file=/tmp/jolt-callgrind.out \
  target/release/jolt fmt --no-config /tmp/jolt-prof/SpelCompilationCoverageTests.java

callgrind_annotate --inclusive=yes --threshold=1 /tmp/jolt-callgrind.out
```
