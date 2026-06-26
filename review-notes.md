# Java Parser Review Notes

Review performed after commit
`1734bd0 Tighten Java parser recovery and organization`.

These notes capture the chosen direction for cleaning up the remaining parser
architecture issue: duplicated speculative scanners.

## Cursor Architecture Direction

Grammar decisions still depend on duplicated speculative scanners. Branch
decisions are made by lookahead helpers such as `starts_method_declaration`,
`starts_local_variable_declaration`, `starts_cast_expression`, `starts_pattern`,
and `skip_type_from`. These helpers duplicate parts of the real grammar parser,
especially type parsing.

Keeping this style preserves a simple, fast, predictive recursive-descent
parser. The cost is drift: any new type syntax, annotation placement, pattern
form, or generic-close rule has to be updated in both the scanner and the real
parser. When they disagree, the parser chooses the wrong production before
recovery can be precise.

The target architecture is one token cursor core with two frontends:

```text
immutable lexer tokens
  -> TokenCursor
      owns position, logical token view, checkpoint/rewind, and virtual `>`
      splitting for `>>` / `>>>`
  -> Parser
      consumes TokenCursor, emits syntax events, and records logical CST tokens
  -> JavaLookahead
      forks TokenCursor and runs markerless grammar scans without emitting
      events or CST tokens
```

`TokenCursor` should be the only code that knows how token movement works. It
should expose logical tokens by default:

- `kind()`, `nth_kind(n)`, `text()`, `range()`, and `token()`
- `bump()` returning the logical token it consumed
- `bump_split_gt()` returning a virtual `>` token when a shift token closes type
  arguments
- `checkpoint()`, `rewind(checkpoint)`, and `fork()`

Raw lexer-token indexing should be rare and explicit. Grammar code should use
the logical cursor view so real parsing and speculative lookahead agree on Java
generic-close behavior.

`Parser` should become the event-emitting frontend over `TokenCursor`.
`Parser::bump()` should consume one logical cursor token, emit `Event::Token`,
and push that same logical token into the CST token stream.

`JavaLookahead` should become the markerless grammar-scanning frontend over a
forked `TokenCursor`. It should not emit syntax events, diagnostics, or CST
tokens. Its job is to answer ambiguous grammar questions with the same token
semantics the real parser uses.

## Replacement Plan

1. Audit the current ambiguity snapshot coverage.

   Before changing cursor or lookahead code, inventory the existing parser
   snapshots for the ambiguity families this refactor can affect: generic
   closes, casts, typed lambda parameters, local declarations versus expression
   statements, patterns, array creation, and constructor/member disambiguation.
   If one of those families is not covered by a focused snapshot, add the
   missing test case and generate its snapshot before the refactor. Do not add
   duplicate tests for cases that are already pinned clearly.

2. Introduce `TokenCursor`.

   Move `Parser`'s current token-position state into a dedicated cursor:
   immutable `ParserToken` slice, current position, pending `>` split state, and
   source text access. Preserve the current logical-token behavior for `>>` and
   `>>>` closes.

3. Put parser consumption on top of `TokenCursor`.

   Change `Parser` to own a cursor plus `events` and `tree_tokens`. Implement
   parser `bump`, `bump_split_gt`, `current_kind`, `nth_kind`, `current_text`,
   and error-range helpers by delegating to the cursor. Keep the observable CST
   output unchanged.

4. Audit parser behavior after the cursor extraction.

   Run the focused ambiguity snapshots and the full Java parser tests. This step
   should be behavior-preserving: existing snapshots should not change except
   for an intentional improvement identified before editing.

5. Add checkpoints and forks.

   Add a copyable cursor checkpoint containing position plus pending split
   state. Add `rewind` and `fork` so speculative scans can advance without
   mutating the real parser cursor.

6. Create `JavaLookahead`.

   Add a small wrapper around a forked `TokenCursor` for markerless grammar
   scans. Keep simple token-set predicates simple, but move grammar-shaped scans
   into this type.

7. Port type-shaped scans first.

   Replace `skip_type_from`, `skip_type_base_from`,
   `skip_balanced_type_arguments_from`, and `skip_cast_type_from` with
   `JavaLookahead` scans for annotations, type arguments, base types,
   dimensions, ordinary types, and cast/intersection types.

8. Audit type-scan parity.

   Compare old and new behavior with the focused ambiguity snapshots before
   deleting the old scanners. Pay special attention to source ranges for virtual
   `>` tokens, annotations on qualified type segments, array dimensions, and
   intersection casts.

9. Update ambiguous branch checks.

   Rework `starts_method_declaration`, `starts_annotation_element`,
   `starts_local_variable_declaration`, `starts_typed_lambda_parameter`,
   `starts_cast_expression`, `starts_case_type_pattern`,
   `starts_record_pattern`, and `new_expression_is_array_creation` to use the
   lookahead scans instead of raw index arithmetic.

10. Audit branch-decision parity.

    Run the ambiguity snapshots and parser fixture tests again. Any snapshot
    change should be explained as either an intended correctness fix or a bug in
    the new lookahead path.

11. Delete the old duplicated scanners.

Once the branch checks use `JavaLookahead`, remove the index-based type scanners
from `grammar/util.rs`. Keep direct token predicates only for genuinely shallow
checks such as literal starts, label starts, and keyword adjacency.

12. Final audit.

Confirm no old type-shaped index scanners remain, run the full project checks,
and review the resulting diff for architecture regressions: parser event
emission should live in `Parser`, token movement should live in `TokenCursor`,
and speculative grammar scans should live in `JavaLookahead`.
