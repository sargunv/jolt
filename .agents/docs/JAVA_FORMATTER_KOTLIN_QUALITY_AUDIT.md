# Java Formatter Kotlin Quality Audit

Goal: bring `crates/jolt_java_fmt` up to the same formatter-quality bar used for
the Kotlin cleanup, without adding Java-specific tech debt.

## Project Invariants

- Formatter syntax access uses borrowed parser-owned source, tokens, nodes, and
  trivia. Do not clone token buffers, source text, or syntax nodes to rediscover
  structure.
- Formatter layout is structured. Every representable tree has a representable
  layout; format available children and syntax accessors instead of replaying
  raw source or treating the formatter as a parser.
- A formatter does not validate syntax. Malformed-but-represented trees must
  format consistently without panics, refusals, or dropped represented pieces.
- Formatter rules must not parse by token inspection. Syntax crates own recovery
  accessors and token ownership; formatter crates consume those accessors for
  layout.
- Formatter rules preserve represented source tokens and trivia through the
  standard token-formatting helpers. Token synthesis is limited to documented
  readability or normalization cases that do not discard trivia.
- Raw literal source output is allowed only for formatter-ignore ranges.

## Shared Smell Checklist

- [x] Source-slice audit: remaining source slices are classified as
      formatter-ignore preservation, spacing/trivia inspection, diagnostics, or
      bugs. No source slice emits formatter output outside formatter-ignore.
- [x] Raw fallback audit: no represented Java syntax node is formatted by
      whole-node raw source replay.
- [x] Borrowing audit: recovery and helper paths borrow parser-owned syntax
      nodes/tokens/trivia and do not allocate replacement token or node buffers.
- [x] Clone audit: formatter recovery paths do not clone syntax nodes or token
      buffers to probe structure.
- [x] Token-drop audit: represented source tokens and trivia are routed through
      standard token formatting helpers or documented normalization paths.
- [x] Panic/refusal audit: representable recovered trees format without
      formatter panics or parser-diagnostic refusals.
- [x] Parser-in-formatter audit: formatter code does not infer grammar by
      scanning token streams. Java syntax accessors expose recovery ownership;
      formatter rules consume those accessors for layout.
- [x] Missing-child audit: expression, type, declaration, statement, and
      container formatters tolerate missing recovered pieces while formatting
      available represented children.
- [x] Container coverage audit: lists, blocks, class bodies, enum bodies, switch
      bodies, annotations, try resources, type parameters, arguments,
      parameters, permits/throws/extends/implements clauses, module directives,
      patterns, and imports preserve orphan delimiters and recovered tokens.
- [x] Trivia conservation audit: aggressive crevice-comment fixtures exercise
      uncommon valid trivia positions across Java syntax surfaces.
- [x] Fixture-manifest audit: Java syntax and formatter fixture manifests are
      snapshotted so fixture drift cannot silently shrink coverage.
- [x] Verification: Java formatter/syntax tests, Kotlin formatter/syntax tests,
      CLI tests, fmt checks, `git diff --check`, and `.snap.new` hygiene are
      required before claiming clean.

## Work Completed

- Removed parser-diagnostic formatter refusals from Java formatting so recovered
  represented trees are formatted structurally.
- Replaced recovery-token special cases with syntax-owned recovery accessors and
  structured formatting entry points where the tree shape exists.
- Removed formatter-owned token/node buffers, speculative node clones, and
  panic-based recovered-syntax handling from the Java formatter cleanup.
- Added Java parser progress, fixture-manifest, corpus, and trivia-conservation
  coverage to catch non-progress, fixture drift, token drops, and trivia loss.
- Updated CLI tests to reflect the formatter contract: recovered represented
  Java parses are formatted/checked as changed files rather than rejected by
  parse diagnostics.

## Independent Audit

- Dual subagent audit covered Kotlin and Java against the same smell list:
  borrowing, structured layout, raw fallback, token drop/synthesis, token
  parsing in formatter rules, panics/refusals, source slices, node/token clones,
  and fixture/test coverage.
- Follow-up audit findings were addressed in Java and Kotlin before this
  checklist was marked complete.
