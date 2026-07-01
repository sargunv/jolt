# Jolt Java Formatting Style

This is Jolt's Java formatting policy. It is a formatter contract, not an
oracle-compatibility target. Prettier-Java, Ruff, and other formatters are
useful references where noted, but the rules here describe Jolt's own style.

## Documents

- [Program, comments, modules, names, lexical structure](java-format-style-program-comments.md)
- [Declarations, modifiers, parameters, types](java-format-style-declarations.md)
- [Statements, blocks, switch, try/resources](java-format-style-statements.md)
- [Expressions, calls, chains, operators, arrays](java-format-style-expressions.md)

## Formatter Contract

- Formatting is deterministic and idempotent.
- Parser-accepted Java must receive a real formatting rule unless the parser
  rejects it before formatting.
- Rendering must remain bounded and linear. Do not add unbounded layout search
  or Prettier-style arbitrary best-fitting behavior.
- The formatter owns style. Fixtures from google-java-format, Palantir,
  Prettier-Java, and other projects are coverage inputs and comparison
  references, not pass/fail output truth.
- Java and Kotlin may share low-level document/list helpers, but each language
  owns its high-level syntax policies.

## Indentation

- Default line width is 80.
- Jolt has one indent policy. There is no separate continuation indent.
- Broken continuation lines use the same indent unit as blocks and lists.
- Prefer structural wrapping over alignment columns.

## Program Shape

- Files end with a final newline.
- Package declarations, import groups, and top-level declarations are separated
  by one blank line where present.
- Redundant top-level semicolons are removed.
- User blank lines generally normalize, but output preserves one intentional
  blank line in bodies so users can separate dense code.

## Imports

- Sorting existing imports is part of formatting.
- Formatting must not add or remove imports.
- Import groups are ordered normal imports first, then static imports.
- Imports sort by a deterministic, locale-free, case-sensitive comparator over
  the import path text or path segments.
- Comments between imports are sorting barriers for v1. Sort only uninterrupted
  import runs; do not move comments with imports yet.

## Modules

- Module directives are sorted by directive kind first, then by the same
  comparator used for imports.
- Directive kind order is `requires`, `exports`, `opens`, `uses`, `provides`.
- Different directive kinds are separated by one blank line.
- Comments between module directives are sorting barriers.
- `requires` modifiers use canonical order: `static` before `transitive`.
- Broken module `to` and `with` target lists use ordinary one-indent
  continuation. Keep `to` or `with` with the subject when possible, then break
  targets one per continuation line as needed.

## Comments And Ignoring

- Comments use leading, trailing, and dangling vocabulary, but attachment is
  computed from Jolt CST roles and source spans rather than porting Prettier's
  global attachment heuristics.
- Constructs that move code own their comment placement.
- Star-block comments are structurally normalized.
- Semantic Javadoc parsing, tag reflow, and embedded-language formatting are
  deferred.
- Formatter ignore comments use generic or IDE-compatible spelling, such as
  `@formatter:off` and `@formatter:on`.
- Do not support branded compatibility spellings such as `prettier-ignore`.
- Do not introduce a Jolt-branded ignore spelling unless a future need emerges.
- `@formatter:off` / `@formatter:on` ranges preserve their interior source
  slices verbatim while still requiring the surrounding file to parse.

## Names And Lexical Values

- Formatter accessors expose qualified names as semantic segment lists, with
  segment-level annotations and comments where needed.
- Qualified-name dots are normalized tightly.
- Block comments around qualified-name dots attach to adjacent segments without
  forcing multiline.
- Line comments inside qualified names force a leading-dot continuation layout.
- Embedded-language content in text blocks and Javadocs is preserved for v1.
- Text-block internal indentation and content are preserved exactly for v1.
- Java template expressions are out of initial formatter scope unless the
  language reintroduces a stable form.
- Numeric literal style may be normalized broadly when semantics are unchanged:
  prefix/suffix casing, hex digit casing, separator grouping, and leading-zero
  decimal forms.
- Do not rewrite string or character escape content as formatter behavior.

## Declarations

- Modifier keywords are reordered canonically.
- Declaration annotations and type-use annotations are distinct accessor roles.
- Declaration annotations print one per line.
- Type-use annotations remain inline with the annotated type.
- Body declarations get automatic blank-line padding between member categories,
  capped at one blank line.
- Empty statements in type bodies are removed while preserving comments.
- Multiline enum constant lists get a trailing comma when the enum has no body
  declarations. Enums with body declarations use the required semicolon before
  declarations instead.

## Braces And Blocks

- Empty blocks expand with the closing brace on its own line.
- Comments-only blocks are expanded because comments are block contents.
- Unbraced statement bodies are normalized to braced bodies.
- Empty statement loop/control bodies normalize to braced empty blocks.
- Standalone empty statements inside blocks are removed unless comments need to
  be preserved.
- When a type or executable declaration header breaks, put the opening brace on
  its own line, unless implementation proves this materially complicates the
  layout engine.

## Parameters And Lists

- Broken parameter and record-component lists put the closing delimiter on its
  own line, Ruff-style, instead of tying `)` to the last item.
- Record components otherwise use ordinary parameter-list layout.
- Call arguments use a simple break-all policy for v1: when an argument list
  breaks, each argument gets its own line.
- Do not add first/last-argument expansion unless a later Jolt policy explicitly
  needs it.
- Blank lines inside argument lists and member chains are normalized away.
  Intentional blank-line preservation is for bodies, not expression internals.

## Statements

- Labeled statements put the label on its own line and do not add an extra
  indentation scope for the labeled body.
- Long multi-value `case` labels keep the first value with `case`; subsequent
  values continue as an indented comma list.
- A single block body in a colon switch case stays on the same line as the case
  label.
- Switch guards use normal expression formatting and preserve user-authored
  parentheses.
- Broken `for` headers split first into init, condition, and update segments;
  each segment then uses normal declaration/expression/list formatting only when
  that segment itself needs to break.
- `return`, `throw`, and `yield` arguments use normal expression formatting.
- Optional trailing semicolons in try-with-resources resource lists are removed.

## Expressions

- Preserve user-authored expression parentheses and required syntactic grouping
  from the parsed tree.
- Do not add readability parentheses in v1; that belongs to a later, explicitly
  scoped policy.
- Binary operators wrap at the start of continuation lines, with continuation
  operator lines aligned to the first operand rather than extra-indented.
- Multi-catch alternatives use the same leading-operator list shape as binary
  expressions.
- Broken ternaries use the same flat expression-continuation shape as binary
  operators: `?` and `:` start continuation lines aligned to the condition.
- Optional parentheses are omitted for a lone untyped lambda parameter. This
  does not change the policy of preserving user-authored expression parentheses.
- Broken member chains keep a simple receiver plus first selector together when
  that head fits. If the receiver is complex or multiline, the first selector
  moves to the continuation line. This follows Ruff's general shape.
- Wildcards and `_` are exposed as distinct context-specific syntax constructs
  in formatter accessors, such as wildcard type arguments, unnamed variables,
  unnamed lambda parameters, unnamed exception parameters, and unnamed patterns.
  Leaf printers may still render them as simple tokens.
