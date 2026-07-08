# Deferred Cleanup Items

Items identified during the syntax/formatter cleanup audit that were
intentionally deferred. Each should be revisited when the surrounding code is
touched next.

## `BlockItem::starts_after_blank_line` accessor (Kotlin)

**Status**: Prototyped and reverted in Phase 6A. The source-gap-based
`block_item_starts_after_blank_line` helper in
`crates/jolt_kotlin_fmt/src/rules/statements/blocks.rs` remains in place.

**Why deferred**: The accessor `BlockItem::starts_after_blank_line() ->
bool`
was added on the Kotlin syntax crate, composing
`first_token().has_leading_blank_line()` (via the shared
`jolt_syntax::trivia_has_blank_line`). This is the correct structured pattern
and matches the Java crate's `BlockItem::starts_after_blank_line` at
`crates/jolt_java_syntax/src/nodes/accessors.rs:1205`.

However, the formatter's `format_trailing_comments` helper (in
`crates/jolt_kotlin_fmt/src/helpers/comments.rs`) emits an implicit
`hard_line()` after any `LineComment` trivia on a token's trailing side. This
means the leading trivia of the _next_ token starts after that implicit
hard_line, so the leading-trivia-only blank-line check undercounts: a blank line
that exists in the _source gap_ between two block items (after a trailing line
comment's newline) is not reflected in the next item's leading trivia when
re-parsed from the formatted output.

Concretely, `DokkaVerifier.kt` has:

```
val pathSeparator = ";" // instead of File.pathSeparator ...
                        <- blank line here in source
val path = listOf(...)
```

The `;` token's trailing trivia carries the `LineComment` and its trailing
newline. The next token's leading trivia then has only one newline (the
formatter's implicit hard_line consumed the other). The source-gap check
correctly counts 2 newlines in the gap; the leading-trivia check counts only 1.

**Resolution path**: Either

1. Teach `format_trailing_comments` to emit an `empty_line()` instead of
   `hard_line()` when the trailing line comment is followed by a blank line in
   source (carrying that signal through the trivia), or
2. Make the structured accessor walk both the previous token's trailing trivia
   AND the current token's leading trivia (similar to the source-gap check, but
   using trivia pieces instead of raw source bytes).

Option 2 is the more localized fix but requires the accessor to know about the
previous token. Option 1 is the principled fix because the blank-line signal
belongs to the gap, not to either token alone. Both require touching
`format_trailing_comments` or adding a `Block::blank_line_before_item(index)`
accessor that walks the surrounding trivia.

**Where to look**:

- `crates/jolt_kotlin_fmt/src/rules/statements/blocks.rs` —
  `block_item_starts_after_blank_line` / `gap_has_blank_line` (the source-gap
  helpers that remain).
- `crates/jolt_kotlin_fmt/src/helpers/comments.rs:71` —
  `format_trailing_comments` (emits implicit `hard_line()` after line comments).
- `crates/jolt_java_syntax/src/nodes/accessors.rs:1205` — the Java
  `BlockItem::starts_after_blank_line` reference implementation.
- `crates/jolt_java_fmt/src/helpers/comments.rs` — Java's
  `format_trailing_comments` for comparison (same implicit hard_line behavior;
  Java's blank-line accessor works there because Java trailing line comments are
  less common in the fixture corpus, but the same bug _could_ surface on Java
  with the right input).

**Invariants at stake** (from AGENTS.md):

- "A formatter must not replay raw source text as a fallback for represented
  syntax" — the source-gap helper violates this.
- "Formatter rules must not parse by inspecting token streams" — the source-gap
  helper is borderline (it counts `\n` bytes, not kinds, but it's still raw
  source inspection for layout decisions on non-ignored blocks).
