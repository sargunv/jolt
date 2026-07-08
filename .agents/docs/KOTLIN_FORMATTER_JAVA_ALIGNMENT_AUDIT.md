# Kotlin Formatter Java Architecture Alignment Audit

Goal: align `crates/jolt_kotlin_fmt` with `crates/jolt_java_fmt` architecture
and patterns. Kotlin-specific deviations are allowed only when the syntax model
requires them, and each deviation must be documented at the decision site or in
this audit.

## Accountability Rules

- Start from the Java formatter pattern before changing Kotlin formatter code.
- Prefer porting Java helper architecture over local Kotlin-only heuristics.
- Do not accept formatter refusal, raw replay, token drops, or non-idempotence
  for represented syntax.
- All formatter syntax data is borrowed from parser-owned source, token, trivia,
  and syntax-node buffers. Do not clone buffers or nodes to rediscover
  structure.
- Layout must stay structured. Format every representable tree through its
  represented children and syntax accessors.
- The formatter is not a syntax validator. Format each represented tree as a
  tree: use available structured children, tolerate missing recovered pieces,
  and do not panic for malformed-but-represented syntax.
- Formatter rules must not parse by scanning token streams. Syntax crates own
  recovery accessors and token ownership; formatter crates consume those
  accessors for layout.
- Formatter rules must preserve represented source tokens and trivia through the
  standard token-formatting helpers. Synthesize tokens only for documented
  normalization/readability cases that do not drop trivia.
- Raw literal source is allowed only for formatter-ignore ranges.
- Recovered token-sequence formatting is allowed only for genuinely unstructured
  recovered islands or formatter-ignore ranges. Do not replace partially
  structured nodes with whole-node token replay when their structured children
  can be formatted.
- Keep this document updated as work is completed.
- After local implementation and tests, request an independent subagent audit
  against this document and address findings.

## Remaining Smell Audit Checklist

This checklist is deliberately broader than the previous cursor-only audit. Do
not describe the Kotlin formatter as clean unless every item here is complete or
explicitly moved to a new documented follow-up with a concrete reason.

- [x] Source-slice audit: classify every remaining source slice as
      formatter-ignore raw preservation, trivia/spacing inspection, panic/debug
      text, or a bug. No source slice may emit formatter output except
      formatter-ignore.
- [x] Recovered-token helper audit: recovered islands with no formatter-owned
      structure must route existing syntax tokens through the shared token
      formatter (`format_token` / `format_token_sequence`), not custom token
      text or raw source. Partially structured nodes must format their
      structured children first.
- [x] Expression formatter missing-child audit: every expression formatter that
      returns `nil` for a missing piece must be optional syntax only;
      represented available children/tokens must still be formatted
      structurally.
- [x] Type formatter missing-child audit: every type formatter that returns
      `nil` for a missing piece must be optional syntax only; represented
      available children/tokens must still be formatted structurally.
- [x] Declaration formatter missing-child audit: every declaration/type
      declaration/member-body formatter that returns `nil` for a missing piece
      must be optional syntax only; represented available children/tokens must
      still be formatted structurally.
- [x] Container coverage audit: list/container formatters must not drop orphan
      commas, delimiters, or error children. Known container surfaces include
      value arguments, type arguments/projections, value parameters, type
      parameters, context receivers, delegation specifiers, `when`
      entries/conditions, class members, enum entries, collection literals,
      index expressions, and annotations.
- [x] Token-scan audit: whole-node token scans must be bounded, structural, and
      must not select tokens from nested syntax in place of the current node's
      anchor.
- [x] Recovered-parse tests: add focused tests that format recovered syntax
      through the internal formatter path and assert representative
      tokens/comments survive across expressions, types, declarations, and
      containers.
- [x] Verification: run Kotlin formatter/syntax tests, fmt checks,
      `git diff --check`, and `.snap.new` hygiene after fixes.
- [x] Borrowing audit: no formatter-owned source/token/node buffer clones were
      introduced for recovery formatting.
- [x] No-panic audit: representable recovered trees format without formatter
      panics or syntax validation refusals.
- [x] No-parser-in-formatter audit: formatter token iteration is limited to
      syntax-owned recovered islands and comment/trivia preservation helpers,
      not syntax discovery.

Source-slice classification:

- `helpers/formatter_ignore.rs`: the only intentional raw source output path.
- `rules/program.rs`: source slice is used only to classify blank lines between
  already represented top-level item groups.
- `rules/statements/blocks.rs`: source slice is used only by
  `gap_has_blank_line` for block item spacing.
- `rules/declarations.rs`: declaration-tail source slice is used only to test
  whether the tail after a represented header is trivia/empty.
- `helpers/comments.rs`: `literal_text` is used for comment text construction,
  not for source fallback. A temporary source-gap comment scanner was removed;
  recovered comments now follow Java's `comments_from_tokens` /
  `format_removed_comments` pattern over parser-produced trivia.

Progress since this checklist was added:

- Runtime formatter panics/refusals were removed from declaration formatting,
  import formatting, member-chain layout, source item grouping, and recovered
  block/lambda gaps. Test-only `expect` calls remain in unit tests.
- Value argument lists, value parameter lists, and type argument lists now
  preserve orphan/recovered gap tokens as interleaved recovered islands while
  still formatting represented entries structurally.
- No-callee call expressions now format their represented suffix structure
  instead of replaying the whole call token stream.
- Added internal recovered-parse tests for missing lambda/block close braces,
  orphan value argument tokens/comments, orphan value parameter tokens/comments,
  orphan type argument tokens/comments, no-callee call suffix formatting, and
  class-body recovered member tokens/comments.
- Expression statements now have an explicit recovered-token coverage check so
  sibling error nodes under a represented expression statement are not dropped.
- Class body recovered enum separators are preserved as source tokens attached
  to the preceding enum-entry section instead of being dropped or emitted as a
  standalone body member.
- The broad declaration-level
  `expect_formatted(...).unwrap_or_else(format_token_sequence)` fallback was
  removed. Top-level, local, and member declaration entry points now call total
  structural declaration formatters directly.
- Missing-child expression/type cases that previously returned `nil` for
  represented recovered syntax now either format available structured children
  or emit the current recovered island through `format_token_sequence`.
- The expression dispatcher no longer contains formatter-runtime `unreachable!`;
  unexpected represented expression families are formatted as recovered syntax
  tokens.
- Focused recovered tests now cover navigation without a receiver, assignment
  without a left operand, collection/index orphan tokens, type parameter orphan
  tokens, annotation argument orphan tokens, and `when` condition comma
  comments.
- Eighth-audit fixes added after Arendt's not-clean report: declaration
  expression tails now preserve dangling `=` / `by` tail tokens via a tail-only
  recovered island; expression statements interleave recovered sibling tokens
  around the formatted expression instead of replaying the whole statement;
  context parameter clauses and delegation specifier lists now have
  recovered-gap interleaving; `when` without `{`, `do` without `while`, and
  dangling labeled expressions preserve available represented children/tokens.

Seventh independent audit:

- Subagent `Lorentz` audited the current formatter against the clarified
  recovery policy and found remaining blockers. This audit was broad over
  `format_token_sequence` call sites, source slices/raw output, and the main
  Kotlin formatter rules; it did not claim global cleanliness.
- High: broad declaration-level fallback still replayed whole declaration token
  streams when partially structured declaration formatters returned `None`.
- High: control-flow recovery paths still dropped represented children for
  missing `when` close braces, partial `for` headers, missing `else` branches,
  and the lambda-as-branch formatter.
- High: class/member bodies lacked block/lambda-style recovered-gap
  preservation.
- Medium: several containers still need recovered-island coverage, including
  type parameters, annotations, index/collection literals, delegation
  specifiers, context parameter clauses, and `when` condition lists.
- Medium: several missing-child expression/type paths still drop represented
  pieces, including navigation without receiver, assignment without left,
  labeled expressions, and annotated user types without identifiers.

## Current Alignment Gaps

### 1. Binary Expressions

Status: ALIGNED

Java reference:

- `crates/jolt_java_fmt/src/rules/expressions/operators.rs`
- `flatten_binary_expression`
- `collect_binary_chain`
- `binary_chain`
- operator precedence helpers

Kotlin result:

- Binary-chain flattening follows Java's `flatten_binary_expression` /
  `collect_binary_chain` / `binary_chain` shape.
- Comment-free parenthesized binary expressions are transparent to chain
  collection, matching Java's safety condition that delimiter comments prevent
  flattening.
- Operators are source tokens and go through standard token formatting.
- Kotlin ports Java's readability-parentheses architecture to the analogous
  Kotlin surface: when an infix identifier operator owns a binary operand, the
  formatter inserts explicit `PrecedenceParenthesis` tokens around that operand.
  Kotlin still does not port Java's bitwise/shift-specific predicate because
  Kotlin does not have those binary operator token families.

Required alignment:

- Add Kotlin binary-chain flattening following Java's architecture.
- Preserve Kotlin-specific operators and precedence.
- Keep mixed-precedence parenthesization behavior justified and bounded.
- Use real source operator tokens and standard token formatting.

Acceptance checks:

- Imported ktfmt fixture `MultilineStringFormatter.kt` is idempotent.
- Binary chains with trailing comments on operators format idempotently.
- Existing Kotlin formatter corpus snapshots pass.

### 2. Member Chains

Status: ALIGNED WITH DOCUMENTED KOTLIN-SPECIFIC DEVIATION

Java reference:

- `crates/jolt_java_fmt/src/rules/expressions/member_chains.rs`
- `MemberChainBuilder`
- `member_chain`
- `is_member_chain_child`

Kotlin result:

- The asymmetric `if_break` layout has been removed and the Java `head + rest`
  shape is now used.
- Root leading comments have been moved outside the chain group, matching Java.
- Suffixes that own leading comments or trailing lambdas now force a break
  before the suffix instead of being glued to the root.
- Consecutive navigation suffixes are grouped in a field-run, matching Java's
  `MemberChainBuilder::field_run` pattern for consecutive field accesses.
- Kotlin now has `ExpressionParentRole` accessors for navigation/call/index
  chain ownership and an `is_member_chain_child` guard.
- The guard intentionally covers receiver roles, but not `CallCallee`. Kotlin
  trailing-lambda syntax wraps a call as another call's callee; treating that
  wrapper the same as Java's method-invocation callee suppresses the top-level
  chain builder and collapses multi-call chains.
- Navigation operators now use Java-style "space only if comments exist"
  trailing handling for member-dot comments.

Required alignment:

- Move or reshape Kotlin member-chain logic to match Java's `head + rest`
  architecture.
- Keep the first suffix attached to a simple root in all layouts.
- Add a Kotlin equivalent of `is_member_chain_child` if parent roles are
  available; otherwise document why the CST cannot support it yet and preserve
  equivalent behavior another way.
- Include Kotlin index suffixes as member-chain suffixes, with Java-style stable
  layout.

Acceptance checks:

- `lines[lastStringLineIndex].substringBefore(TQ)` is never split between
  `lines` and `[lastStringLineIndex]`.
- Member chains with calls, indexes, safe calls, and comments are idempotent.

### 3. Comment and Token Ownership

Status: ALIGNED, WITH TOKEN-SCAN DEBT DOCUMENTED UNDER DECLARATIONS

Java reference:

- `crates/jolt_java_fmt/src/helpers/comments.rs`
- `format_token_with_comments`
- delimiter/comment helpers in `helpers/lists.rs`

Kotlin result:

- `TrailingTrivia::RelocatedToEnclosingContext` is restored to Java-like
  behavior: it does not print trailing comments itself.
- The aggressive trivia fixture now passes and covers comments in uncommon token
  crevices across declarations, calls, lists, types, lambdas, templates,
  operators, and control flow.
- Kotlin comment helpers now include Java-style delimiter dangling comments,
  inline leading comments, `BeforeSoftLine`, and `BeforeSpaceIfComments`.
- Whole-declaration token replay was removed for companion objects.
- Remaining token-text paths are source-token formatting helpers or normalized
  import keyword/alias spelling, where source tokens still provide trivia.
- The Kotlin-only import path shortcut through `format_source_token_text` was
  removed; import path tokens now go through `format_token`.

Required alignment:

- Preserve source tokens and trivia through standard token helpers.
- Restrict token text helpers to documented normalization cases.
- Keep raw literal source only for formatter-ignore ranges.

Acceptance checks:

- `cargo test -p jolt_kotlin_fmt --test trivia_conservation -- --nocapture`
  passes.
- Audit `format_token_text`, `format_source_token_text`, `format_source_token`,
  and `FormatterInsertedToken` call sites; document or fix each use.

### 4. List Formatting

Status: ALIGNED FOR CURRENT KOTLIN LIST SURFACES

Java reference:

- `crates/jolt_java_fmt/src/helpers/lists.rs`
- delimiter dangling comments
- separator formatting
- trailing separator policy

Kotlin result:

- Kotlin list helpers now mirror Java's delimiter/comment ownership for
  parenthesized, angle-bracket, and square-bracket comma lists.
- Delimiter dangling comments, close leading comments, inline leading comments,
  separator comments, and `BeforeSoftLine` are centralized in the helper.
- Compact list helper entry points now delegate to the same shared delimited
  list engine; they remain as compatibility names for call sites, not separate
  formatting architecture.
- Kotlin does not port Java's formatter-inserted trailing separator policy
  because Kotlin trailing comma behavior is source-backed in the current
  formatter.

Required alignment:

- Port Java's delimiter/comment list architecture where applicable.
- Keep Kotlin-specific trailing-comma policy explicit.
- Avoid ad hoc per-list comment fixes when a shared helper should own it.

Acceptance checks:

- Annotation, argument, type-argument, square-bracket, parameter, and collection
  lists with comments are idempotent.
- No internal comment placement depends on compact-only list helpers.

### 5. Declaration and Tail Formatting

Status: ALIGNED FOR CURRENT DECLARATION SURFACES

Java reference:

- `crates/jolt_java_fmt/src/rules/declarations/*`
- structured accessors plus helper-owned body/list formatting

Kotlin result:

- Callable names are now represented as `CallableName` syntax nodes instead of
  being rediscovered by scanning all declaration tokens. Function and property
  formatters consume that node directly, including receiver syntax and receiver
  modifiers.
- Callable receiver syntax is now represented structurally: `CallableName` owns
  a `TypeReference` receiver, the final separator dot, and the callable `Name`.
  Function and property formatters route receivers through
  `format_type_reference`, then format the real separator token and name.
- Nullable callable receivers such as `PsiElement?.containsNewline` are now
  parsed as receiver type `PsiElement?`, separator `.`, and callable name
  `containsNewline`. The lexer still recognizes `?.`, but parser-source
  splitting turns it into `Question` plus `Dot`, matching Java's composite-token
  split architecture for `>`/`>>`-style cases.
- Qualified and generic receivers use a bounded final-top-level-separator
  lookahead so `com.example.Scope.render()` formats receiver type
  `com.example.Scope`, while `val x by tasks.registering {}` stops at the soft
  keyword `by` and is not misclassified as a receiver.
- Class, interface, object, companion-object, and secondary-constructor keyword
  anchors now come from typed syntax accessors. The formatter no longer scans
  whole declaration token arrays to find those keywords.
- Soft keyword accessors such as `constructor`, `where`, `by`, `get`, and `set`
  still do direct child-token spelling lookup because the Kotlin parser stores
  many accepted soft keywords as identifier tokens. This is a bounded syntax
  accessor, not a formatter-wide search, and it preserves the real source token
  and trivia.
- Source-gap checks remain for declaration expression tails, property body
  boundaries, and primary constructor tails. These are trivia guards: they prove
  that no represented syntax is being skipped between structured children before
  formatting the structured child. They are not raw replay or fallback.
- The `fun interface` parser artifact is still detected by checking for a
  one-token `FunctionDeclaration` adjacent to an interface declaration. This is
  bounded and not the old callable-name/keyword-anchor scan, but should be
  removed if the parser later represents `fun interface` as one declaration.
- Comments in representable gaps previously caused formatter refusal; known gap
  checks are now trivia-aware.
- A `::class` annotation argument exposed a concrete token-scan bug: type
  declaration keyword lookup matched `class` inside an annotation argument. That
  class of bug is fixed by structural keyword accessors instead of whole-node
  token searches.
- Enum entry commas were dropped because the comma belonged to the enclosing
  class-member declaration. `ClassMemberDeclarationEntry` now exposes the comma
  and class-body formatting uses it.
- `for` statements without block bodies now format their structured body
  expression instead of dropping it.
- Anonymous `object : Type {}` inside statement contexts is now parsed as an
  `ObjectExpression`; `companion object` remains a declaration.
- Declaration expression tails now preserve trailing line comments on `=` tokens
  before formatting the expression.
- Companion objects now format through the structured object-declaration path
  instead of replaying every source token.
- Top-level and class-member declaration formatter refusal has been removed:
  declaration entry points format available represented structure directly
  without a whole-declaration token replay fallback.
- Missing required declaration subparts are treated as recovered syntax:
  available siblings still format structurally, and genuinely unstructured
  recovered islands use shared token formatting.

Required alignment:

- Prefer structured syntax accessors over token scans.
- Treat comments as trivia, not uncovered syntax.
- Eliminate formatter refusal for represented syntax.
- Route declaration receiver types through type formatting rather than token
  replay.

Acceptance checks:

- No formatter path drops declaration tokens such as parameter defaults,
  constructor tails, `when` subject bindings, or type/call arguments.
- Gap checks are trivia-aware and do not reject comments.
- No declaration formatter path performs whole-declaration token scans for
  callable names or declaration keywords.
- Extension function/property receiver fixtures include nullable, qualified, and
  generic receivers, and formatter snapshots are idempotent.

### 6. Formatter Coverage and Fallbacks

Status: ALIGNED FOR RAW SOURCE POLICY; COVERAGE REMAINS TEST-DRIVEN

Java reference:

- Java formatter formats represented syntax or has explicit removed-comment /
  formatter-ignore handling.

Kotlin result:

- Raw literal source is only used by formatter-ignore ranges.
- Uncovered non-trivia block/lambda/class-body tokens are interleaved as
  recovered islands through shared token formatting. Tokenless source gaps are
  not emitted as raw output.
- Formatter refusal at represented declaration entry points and known required
  declaration subparts has been removed rather than converted into panics.
- New syntax accessors have still been added incrementally rather than from a
  complete architecture-first coverage pass; the imported corpora and aggressive
  trivia conservation test are the current guardrails.

Required alignment:

- Represent every syntax tree branch the parser represents.
- Use formatter-ignore as the only raw literal source path.
- Add integration fixtures before accepting new formatter branches.

Acceptance checks:

- Kotlin syntax and formatter corpora pass.
- Imported fixture formatter test either passes or has a documented, scoped
  Java-deviation issue remaining in this file.

## Subagent Audit Findings

Status: COMPLETED; findings addressed or documented.

The first independent audit confirmed these gaps, which have since been fixed or
documented as Kotlin-specific deviations:

- List helper architecture was ported for current Kotlin list surfaces.
- Aggressive trivia conservation now passes.
- Member-chain parent-role detection was added, with Kotlin's trailing-lambda
  `CallCallee` wrapper documented as a deviation.
- Parenthesized binary chain flattening was ported; Java's bitwise/shift
  readability-parentheses rule is documented as non-applicable to Kotlin
  operator tokens.
- Silent declaration refusal at represented declaration entry points was
  removed.
- String-template `${`, named/spread argument trivia, and uncovered block source
  now have owners or explicit failure behavior.
- Whole-declaration token replay was removed for companion objects; remaining
  token-text paths are documented above.

Second independent audit:

- Subagent `Russell` found no raw source fallback outside formatter-ignore /
  comment-token preservation paths, confirmed targeted checks were green, and
  reported no `.snap.new` files.
- High finding: declaration formatting still had refusal-to-empty-output paths.
  Addressed by making declaration entry points total and structural. Remaining
  `nil` sites in declaration code are optional absences such as no constructor
  delegation colon, no type annotation colon, no delegation list, empty
  delegation list, or no class body.
- Medium finding: list formatting was a partial Java port because compact
  helpers remained. Addressed by routing compact helper entry points through the
  shared delimited-list engine.
- Medium finding: member chains omitted `CallCallee` and lacked field-run
  grouping. Field-run grouping was added. `CallCallee` remains intentionally
  excluded because Kotlin trailing-lambda calls use a call-as-callee wrapper
  that Java does not have; including it collapses top-level chain formatting.
- Medium finding: bespoke token/comment formatting paths. Import path token text
  was removed. Qualified-name formatting remains because it mirrors Java's own
  name formatter architecture.
- Low finding: binary expressions lacked Java's readability-parentheses layer.
  Addressed for Kotlin's analogous readability-sensitive surface by adding
  synthesized `PrecedenceParenthesis` tokens around binary operands of infix
  identifier operators.
- Low finding: tests accept `KotlinFormatSinkResult::Halted`. Documented as
  inherited from Java's fixture tests rather than changed in this alignment
  pass.

Third independent audit:

- Subagent `Mill` found no blocking issues after the declaration cleanup.
- Confirmed that callable names are represented as `CallableName` syntax nodes
  and that function/property formatters consume structured accessors rather than
  scanning whole declarations for names.
- Confirmed that declaration keyword anchors now come from syntax accessors.
- Historical finding now fixed: callable receiver formatting used a bounded
  source-token slice. Callable receivers are now structural `TypeReference`
  children and format through the type formatter.
- Non-blocking note: `fun interface` still uses a scoped one-token
  `FunctionDeclaration` artifact check. This is bounded and not the old
  formatter-wide scan, but is documented above.
- Recommended making `parser_progress` and no-`.snap.new` checks explicit in
  verification.

Fourth independent audit:

- Subagent `Beauvoir` confirmed the callable receiver gap is fixed:
  `CallableName` owns a structured `TypeReference` receiver, nullable receivers
  use parser-source split `Question` plus `Dot`, and formatter receivers route
  through `format_type_reference`.
- Medium finding: lambda bodies called the uncovered-source coverage check but
  discarded the result. Addressed by routing uncovered lambda body gaps through
  the same recovered-token preservation policy as block bodies: if non-trivia
  source has syntax tokens, those tokens are emitted through `format_token`.
  Tokenless source gaps are not emitted as raw formatter output.
- Low finding: value arguments without expressions replayed all argument tokens.
  Addressed by making this an explicitly recovered-syntax path that emits the
  existing argument tokens through `format_token`, preserving comments without
  pretending the missing expression is structurally represented.
- Low finding: this document overstated fallback/refusal policy while those two
  paths remained. Addressed with the fixes above.

Fifth independent audit:

- Subagent `Mencius` found that the new recovered-token emitters were correct
  where reached, but several missing-child branches could return before those
  emitters ran.
- High finding: lambda expressions with a missing close brace formatted only the
  opening brace. Addressed by using the lambda text end as the body recovery
  boundary when `}` is absent, so represented body items and uncovered body
  tokens still format.
- High finding: block contents recovery required a close brace, so missing `}`
  could drop block items. Addressed by using the block text end as the recovery
  boundary when `}` is absent.
- High finding: recovered block items could be considered covered by range but
  format to `nil` when their structured child was absent. Addressed by emitting
  the statement/local-declaration/expression-statement token stream through
  `format_token` in those recovered-child paths.
- Medium finding: call expressions with no recognized callee returned `nil`
  before formatting argument lists or trailing lambdas. Addressed by emitting
  the whole recovered call token stream through `format_token`.
- Medium finding: value argument lists only preserved recovered tokens inside
  `ValueArgument` children. Addressed by checking that structural entries cover
  the list contents; orphan non-trivia list tokens now force recovered token
  formatting for the whole list.
- Added focused internal recovered-parse tests for missing lambda close braces,
  missing block close braces, and orphan argument-list tokens/comments.

Sixth independent audit:

- Subagent `Dalton` confirmed the five behavioral recovery findings were fixed
  and found no raw/literal fallback in the reviewed paths.
- Medium finding: lambda and block recovered-gap emission rescanned the entire
  token stream for every gap, making malformed bodies `items * tokens`.
  Addressed by collecting each body token stream once and advancing a token
  cursor through gaps, preserving the same `format_token` behavior with linear
  recovery cost.
- Subagent `Euclid` performed a final read-only audit after the cursor change
  and found no blocking issues. The token cursors are monotonic, trivia-only
  gaps do not create repeated rescans, and recovered output still routes through
  `format_token_sequence`.

Eighth independent audit:

- Subagent `Arendt` audited the completed checklist and correctly rejected a
  clean result.
- Blocker: declaration expression tails could still drop dangling represented
  `=` / `by` tokens. Addressed by formatting assign-only tails structurally and
  adding `format_recovered_declaration_tail`, scoped to the declaration tail
  range rather than the whole declaration.
- Blocker: expression statements with structured expressions plus recovered
  sibling tokens replayed the whole statement. Addressed by interleaving
  recovered statement gaps around the structurally formatted expression.
- Blocker: navigation without receiver replayed the whole node. Addressed by
  formatting available navigation operator and selector structure directly.
- Blocker: several containers lacked recovered-gap interleaving. Addressed for
  type parameter lists, annotation argument lists, square-bracket collection and
  index lists, context parameter clauses, delegation specifier lists, and `when`
  condition fallback formatting.
- Blocker: missing-child control-flow branches dropped represented children.
  Addressed for `when` without an open brace, `do` without `while`, dangling
  labeled expressions, and empty recovered `when` conditions.
- Blocker: audit documentation overstated recovered-test coverage. Addressed by
  adding internal formatter tests in `rules/program.rs` for the recovered parser
  paths; these are intentionally unit-level because the public formatter
  currently blocks diagnostic parses.
- Follow-up blocker: assignment expressions without a left operand replayed the
  whole assignment node. Addressed by formatting the available operator and
  right-hand expression structurally, using token-sequence recovery only when
  the assignment node has no operator or right expression to format.
- Final re-audit: Arendt rechecked the assignment fix plus a quick broad
  clean-gate scan for raw source output, formatter panics/refusals, broad token
  fallbacks, recovered container paths, and `nil` hotspots. Result: CLEAN.

Current verification:

- `cargo fmt --check -p jolt_kotlin_fmt -p jolt_kotlin_syntax` passes.
- `cargo test -p jolt_kotlin_fmt --test corpus -- --nocapture` passes.
- `cargo test -p jolt_kotlin_fmt --test trivia_conservation -- --nocapture`
  passes.
- `cargo test -p jolt_kotlin_fmt` passes.
- `cargo test -p jolt_kotlin_syntax --test corpus -- --nocapture` passes.
- `cargo test -p jolt_kotlin_syntax --test imported_fixtures -- --nocapture`
  passes.
- `cargo test -p jolt_kotlin_syntax --test parser_progress -- --nocapture`
  passes.
- No `.snap.new` files are present.

## Final Checklist

- [x] Binary expression architecture matches Java pattern where applicable,
      including readability parentheses for Kotlin infix binary operands.
- [x] Member-chain architecture matches Java pattern with documented Kotlin
      trailing-lambda callee deviation.
- [x] Comment/token ownership audit complete for known formatter paths.
- [x] List helper architecture aligned.
- [x] Declaration formatter token/gap audit complete for the known Java
      alignment gap: callable names and declaration keywords are structural;
      remaining source-gap checks are bounded trivia guards.
- [x] Callable receiver syntax is owned by structured type formatting.
- [x] Raw fallback/refusal policy enforced for known formatter paths.
- [x] Focused Kotlin trivia conservation test passes.
- [x] Imported Kotlin formatter corpus passes.
- [x] Imported Kotlin syntax corpus passes.
- [x] Kotlin formatter corpus snapshots pass without hiding dropped trivia.
- [x] Full `cargo test -p jolt_kotlin_fmt` passes.
- [x] Final independent subagent audit completed after the recovered-syntax and
      declaration cleanup.
- [x] Second subagent findings addressed or documented.
