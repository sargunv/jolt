# `jolt fmt` Performance Pipeline Report

## Purpose

This report maps the data flow of `jolt fmt` and identifies the changes most
likely to improve CPU time and memory use before publishing a binary.

The priority order below is based on what enables the next layer of performance
work. It avoids optimizing for extreme project sizes. Candidate lists are small
enough for Jolt's expected use cases, and Rayon already bounds active formatting
work.

## Current Pipeline

Today the filesystem path flow is:

```text
CLI args
  -> collect all candidate files and resolved config
  -> format candidates in parallel with Rayon
       -> read the whole file into a String
       -> call jolt_fmt_core::format_source_to_sink
            -> parse Java source
            -> build Java formatter Doc IR
       -> render Doc IR into a host-provided sink
       -> check mode: compare rendered chunks and halt after mismatch
       -> write mode: buffer rendered output while comparing to source
       -> commit the buffered output if changed
  -> emit collected stdout and stderr output on the main thread
```

The stdin path is single-file and single-threaded:

```text
stdin
  -> read all input into a String
  -> format
  -> buffered sink commits complete output to stdout
```

## Current Memory Shape

For filesystem formatting, the command currently materializes these major data
sets:

- all candidate files before formatting starts,
- one full input `String` per active worker,
- Java Unicode-translated input characters,
- a full lexer token vector,
- parser event and parser-token vectors,
- a green/red syntax tree,
- copied token and trivia text inside green tokens,
- a comment map for the compilation unit,
- a full formatter `Doc` tree,
- a full rendered output buffer for normal write/stdout/dprint paths,
- all per-file output records before terminal emission.

Some of these are short-lived, but at peak each worker can hold several
whole-file representations at once.

## Current Parallelism And Synchronization

Formatting is parallel across files. Each worker formats one file independently.
The number of actively formatting files is already bounded by Rayon.

Discovery and config resolution happen before the parallel formatting phase.
Each `CandidateFile` already owns its resolved config by the time Rayon sees it.
That means candidate discovery materializes the candidate list, but this is not
currently a priority problem.

File writes happen inside formatter workers. Terminal output does not: workers
return `FileFormatResult` values, Rayon collects them, and the main thread emits
stdout and stderr afterward. This preserves deterministic output order, but it
requires holding those per-file results until all workers finish.

## Target Pipeline

The target architecture keeps Rayon as the outer parallelism model and focuses
on reducing per-file work and allocations:

```text
candidate Vec
  -> Rayon formats files in parallel
       -> read or map source
       -> parse with fewer duplicate buffers
       -> build syntax without unnecessary text copies
       -> build layout representation
       -> validate layout
       -> render into a sink
            check mode: compare against source and cancel after mismatch
            write mode: buffer output while comparing against source, then
              write once if changed
            dprint mode: buffer output bytes and return them to dprint
       -> return a small per-file result
  -> main thread emits terminal output
```

The important shift is inside each worker: make output behavior host-specific
through sinks, avoid full formatted output in check mode, and remove duplicate
parser/syntax representations where they do not buy useful behavior.

## Design Principle: Stream Where It Pays

The strongest streaming opportunity is check-mode rendered output. The formatter
should not need to allocate a full formatted `String` just to answer "changed?".

Normal write mode is more subtle. A fully streaming file write is fast, but it
can partially rewrite the destination if layout/rendering fails or panics after
the first emitted chunk. A temp-file streaming write avoids that, but benchmark
data showed that per-file temp creation, replacement, metadata handling, and
especially fsync-style durability costs can dominate large changed-file
workloads.

Java formatting needs enough structural context to parse a compilation unit and
make layout decisions. Trying to make every intra-file phase byte-streaming is
possible, but it requires changing core syntax and layout representations.

A better foundation is:

- stream renderer output to sinks,
- cancel check-mode rendering after the first mismatch,
- buffer normal write/stdout/dprint output in a sink and commit only after a
  complete render,
- write changed files with one final `fs::write`,
- reduce duplicate parser and syntax storage,
- keep full-file candidate discovery unless profiling proves it matters.

This means "sink-based" is the formatter boundary, but not every sink should be
a direct I/O stream. Some sinks are deliberately completion-gated buffers.

## Work Order

### 1. Introduce Render Sinks

The renderer should not require building a final `String`.

Add a sink-oriented rendering API. Keep owned-string rendering private to the
renderer as the implementation of `render`, not as a parallel formatter API:

```text
render_to(doc, options, sink) -> RenderOutcome { stats, halted }
render(doc, options) -> Rendered { text, stats }
```

Useful host sinks:

- `CompareSink` for CLI `--check`,
- buffered compare-and-commit sink for CLI write mode,
- buffered stdout sink for CLI stdin formatting,
- buffered byte sink for dprint,
- possibly `CountingSink` or `NullSink` for benchmarks and diagnostics.

This change should come early because many later optimizations depend on it.
Without render sinks, the pipeline always has to allocate the full formatted
output even when it only needs to know whether the file changed.

Early-halt rendering must be explicit in the renderer return value. A halted
render is useful for check mode, but it is not a complete render and downstream
callers must not treat it as a successful full output.

### 2. Add Source Comparison As A Sink

For `--check`, the formatter only needs to know whether rendered output equals
the original source.

A compare sink can receive rendered chunks and compare them against the original
source bytes. Once it sees a mismatch, it can mark the file as changed.

The end state is cancellable check-mode rendering. After parser and formatter
diagnostics have been produced, check mode only needs a yes/no answer for
whether the rendered bytes match the source. It should be able to stop rendering
after the first output mismatch unless a specific diagnostic requirement says
otherwise.

### 3. Add Buffered Commit Sinks

Write mode should render to a buffered sink that compares rendered output with
the source while accumulating the final output. At the end:

- if unchanged, do not touch the file,
- if changed, write the buffered output with one `fs::write`,
- if parsing, layout, rendering, or diagnostics block formatting, leave the
  original file untouched.

This keeps the successful changed-file path close to the original fast
implementation, restores unchanged-file behavior, and avoids exposing users to
partially written files on ordinary formatter/render errors.

This does not provide crash-safe atomic replacement. That is an explicit
tradeoff for the normal formatter path. If a future `--safe-write` or release
hardening mode needs atomic replacement, it should be added as a separate host
policy with fresh benchmark data.

Research notes from the walked-back atomic-write experiment:

- `atomic-write-file` gives a streaming file-like writer and safe replacement,
  but its commit path performs durability work that caused very large system
  time regressions when thousands of files changed.
- `tempfile::NamedTempFile::persist` avoids the fsync behavior and can
  atomically replace an existing target, but the temp-file lifecycle and
  metadata handling still added measurable overhead in changed-heavy corpora.
- In-place direct streaming was fast, but it made render/layout failures after
  the first output chunk destructive. Buffered commit recovered the fast path
  while keeping ordinary formatter failures non-destructive.

Metadata policy for the current normal write path is simple: changed files are
written in place with `fs::write`, so existing Unix mode bits and inode identity
are preserved. Modification time changes only when contents change.

### 4. Reduce Parser Token Duplication

After check mode no longer has to materialize rendered output, turn to parser
memory.

Current parse flow includes:

```text
source String
  -> Vec<InputChar>
  -> Vec<Token>
  -> Arc<[ParserToken]> in TokenCursor
  -> tree_tokens Vec<ParserToken>
  -> green tree
```

The first parser goal should be removing duplicate token storage, not full
streaming parsing.

Possible direction:

- parse from a token source with bounded lookahead,
- store only the logical tree tokens needed by green-tree construction,
- avoid cloning trivia vectors during normal consumption,
- keep special handling for split `>` tokens explicit and cheap.

This reduces memory while preserving the existing event-to-green architecture.

### 5. Make Unicode Escape Translation Lazy

The lexer currently translates Java Unicode escapes for the whole source before
tokenization.

Java's Unicode escape rules are global enough that this logic must stay careful,
but it does not necessarily require a full `Vec<InputChar>` for the entire file.

A lazy translated-character cursor could:

- scan source bytes/chars on demand,
- maintain Unicode escape eligibility state,
- expose checkpoints for lexer rewind,
- preserve original source ranges,
- report malformed escape diagnostics.

This is a significant lexer rewrite, but it removes one whole-file allocation
from every Java parse.

### 6. Revisit Green Token Text Ownership

The syntax tree currently stores token and trivia text as owned `Arc<str>`. That
is simple and robust, but it copies text that already exists in the source.

A lower-memory syntax representation would store:

- token kind,
- source range,
- leading/trailing trivia ranges and kinds,
- shared reference to source text.

This is a major representation change. It affects syntax APIs, token text
access, comments, formatter helpers, and any future incremental parsing plans.

It is also one of the biggest possible per-file memory wins because comments and
trivia can be large.

### 7. Rework Comment Extraction

The formatter builds a whole-file `CommentMap` before formatting.

That may remain acceptable, but after syntax storage is improved, comment
handling should be revisited. Possible improvements:

- store comment references instead of owned `String` text,
- avoid repeated token collection for node helper methods,
- make comment lookup lazy where rules only need local comments,
- keep a full map only if profiling shows lookup cost is better than repeated
  traversal.

This should come after syntax ownership changes, because comment representation
depends on token/trivia representation.

### 8. Evaluate Doc IR Compaction Before Doc Streaming

Full streaming Doc construction is the hardest layout change.

The renderer's group decisions depend on fit checks over future document
structure. That means the renderer needs some inspectable representation of
upcoming layout, at least within groups. A naive streaming builder would either
lose formatting quality or reintroduce buffering in a less explicit form.

Better options to evaluate:

- compact `Doc` allocation with arenas,
- avoid cloning `Doc` in line suffix and fit paths,
- make common small docs cheaper,
- render from borrowed/arena-backed doc nodes,
- stream only top-level independent regions when layout boundaries are clear.

Only pursue true streaming Doc construction if profiling shows `Doc` memory or
construction time remains a dominant cost after render sinks and parser storage
work.

### Out Of Scope Unless Profiling Says Otherwise

Do not prioritize these before the first binary release:

- replacing Rayon with a custom worker pipeline,
- bounded discovery-to-format queues,
- parallel filesystem traversal,
- thread-safe config resolution for parallel walking,
- ordered-vs-unordered terminal output architecture.

Rayon already bounds active formatting work, and candidate lists are acceptable
for the expected project sizes. These changes can come later if real benchmark
data shows discovery, config resolution, or output buffering matters.

## Check Mode

The best check-mode pipeline is:

```text
source
  -> parse
  -> build layout
  -> validate layout
  -> render to CompareSink
  -> stop at first output mismatch
  -> report changed/unchanged
```

The intended end state is cancellation after the first output mismatch. Check
mode does not need a diff, so once parsing, layout construction, and layout
validation have succeeded, a byte mismatch is enough information to report that
the file is not formatted.

Check mode should never allocate the full formatted output unless a debug or
test path asks for it.

## Write Mode

The best write-mode pipeline is:

```text
source
  -> parse
  -> build layout
  -> render to BufferedCompareFileSink
       buffers formatted bytes
       compares against source as bytes are emitted
  -> if unchanged: do not touch the destination
  -> if changed: write the buffered output once
```

This still materializes the full output, but only inside the host sink. The
formatter API remains sink-based, check mode avoids the allocation, and normal
write mode commits only after complete successful rendering.

## Stdin Mode

Stdin cannot avoid reading all input if the parser requires a full source
string. However, it can still benefit from render sinks:

- check mode can compare rendered output to the input buffer,
- normal mode can render into the same completion-gated buffer shape as file
  writes, then commit to stdout after success.

Stdout uses buffering for the same reason file writes do: avoid emitting partial
formatter output if rendering fails after the first chunk. This also gives
stdout and file mode one consistent completion contract.

## Suggested Milestones

### Milestone 1: Pipeline Interfaces

- Introduce render sinks.
- Replace string-first formatting with sink-first rendering.
- Represent early halt explicitly in the renderer outcome.

### Milestone 2: Host Sink Output Paths

- Implement compare sink for check mode.
- Implement buffered compare-and-commit sinks for write, stdout, and dprint host
  paths.
- Remove parallel public formatter APIs that return owned formatted strings.
- Keep owned-string rendering only inside tests or host-local adapters that
  actually need a `String`.

Milestone 2 no longer includes atomic replacement semantics for the normal
formatter path. That experiment exposed too much per-file overhead for the
current benchmarks.

### Milestone 3: Parser Memory Reduction

- Remove duplicate token buffers.
- Parse from a bounded-lookahead source where practical.
- Keep the event-to-green architecture initially.

### Milestone 4: Lexer And Syntax Storage

- Make Unicode escape translation lazy.
- Move green token/trivia storage toward source ranges.
- Update comment APIs to avoid owned strings where possible.

### Milestone 5: Layout Representation Tuning

- Profile `Doc` allocation and fit-check costs.
- Compact or arena-allocate Doc nodes if needed.
- Consider limited streaming only at proven layout boundaries.

## Settled Non-Goals

Terminal output ordering is not a requirement, but output architecture is not a
priority performance target unless profiling shows the collected per-file
results are costly after sinked output paths land. Check mode should be
cancellable once it has enough information to report changed/unchanged.
Explicit-file language fallback is a CLI policy question outside this
performance workstream.

Atomic replacement is not part of the normal write-mode performance pipeline. It
may be revisited as an opt-in safety policy if product requirements demand
crash-safe replacement and benchmarks justify the overhead.

## Input Text Ownership

The formatter should stream output, not input.

The Java parser and formatter need stable source text:

- the parser does lookahead and rewind,
- tokens and trivia carry source ranges,
- comments need to expose original text,
- diagnostics need byte ranges and line/column lookup,
- formatting traverses the syntax tree after parsing.

A raw input stream would have to be buffered internally before those operations
could work. That would make the design more complicated without removing the
need to hold source text.

The practical target is simple:

- CLI filesystem input may read or memory-map a file, then expose valid UTF-8
  source text to the formatter.
- Stdin and tests can use owned strings.
- dprint already gives the plugin bytes; the plugin should validate UTF-8 and
  pass borrowed text into the formatter.
- If green tokens move to source ranges instead of owned `Arc<str>` text, the
  syntax tree needs a way to keep the backing source text alive. That can be a
  concrete owned/shared text type, not a general streaming abstraction.

Memory mapping remains an optional CLI input strategy. It should not drive the
core parser API unless benchmarks show normal reads are a real bottleneck.

## Bottom Line

The strongest architecture is not "stream every byte immediately." It is
Rayon-backed file parallelism with carefully chosen intra-file ownership.

The first foundational move is to make formatting output sink-based. That
unlocks check-mode comparison, host-specific buffered commit behavior, lower
memory use in check mode, and cleaner write behavior. After that, the deeper
work is reducing duplicate parser storage, making Unicode translation lazy, and
eventually storing syntax text by source range instead of copying it into green
tokens.
