# Formatter

Jolt formats a lossless concrete syntax tree into a document, then renders that
document directly to a sink. Java and Kotlin share the document machinery, not
their syntax or layout rules.

## Run ownership

The public `jolt_formatter` facade only detects or accepts a language and
dispatches to `jolt_java_fmt` or `jolt_kotlin_fmt`. Each language formatter then
owns:

- parsing source and extracting its typed root;
- the root layout entry point and all syntax-shaped layout rules; and
- stable diagnostics for a missing tree or an invalid formatter document.

After parsing, `jolt_fmt_ir::format_root_to_sink` owns the shared lifetime of
one formatting run:

1. Discover formatter-ignore ranges at the syntax root.
2. Create one root `DocBuilder` and install the immutable ignore plan.
3. Invoke the language's root layout function.
4. Freeze the builder into its document arena.
5. Render the selected layouts to the sink and map completion, early halt, or a
   render error to `FormatSinkResult`.

Parsing and typed recovery never live in the shared root coordinator. Document
construction and rendering never live in the CLI or dprint adapters.

## Syntax and rule boundaries

The syntax crates own tree shape. Their typed accessors expose present, missing,
and malformed fields and list parts, exact token ownership, recovery status,
malformed verbatim cores, and normalization claims. Formatter rules consume
those accessors; they do not parse token streams or reconstruct syntax.

`jolt_java_fmt::rules` and `jolt_kotlin_fmt::rules` mirror their respective
language constructs: programs, declarations, expressions, statements, types, and
language-specific forms. These modules decide grouping, indentation, separators,
comment placement, and other layout policy. Their `helpers` modules own
language-specific token/comment formatting, typed field and list recovery, and
lexical classification. In particular, Kotlin's invisible recovery list parts
remain Kotlin-owned rather than becoming a shared list abstraction.

`jolt_fmt_ir` shares mechanics with identical semantics: document construction,
token/trivia assembly, malformed-fragment assembly, formatter-ignore planning,
source claims, and rendering. There is no shared syntax visitor or generic rule
dispatch layer.

Valid represented syntax is always formatted structurally. Verbatim output is
reserved for a syntax-owned malformed core and for a parser-backed
formatter-ignore range; an unhandled valid node or a formatter failure does not
fall back to source replay.

## Source conservation and normalization

The parser's tree retains the source tokens and trivia. Formatting deliberately
replaces ordinary whitespace with layout, but every represented token and every
source-significant trivia identity must still be accounted for. `DocBuilder`
attaches a source claim when a rule emits a source token, formats conserved
trivia, emits malformed or ignored source, or performs an authorized
normalization.

Normalization authority belongs to the syntax crates. A claim is available only
for a complete, recovery-free syntax owner, and the shared syntax layer defines
the closed operations:

- replace a represented token with a canonical delimiter or separator;
- remove a redundant delimiter, separator, or duplicate import;
- synthesize a canonical brace, parenthesis, comma, or semicolon;
- reorder imports, modifiers, or Java module constructs.

Language formatter rules may apply those claims but cannot manufacture them.
When syntax does not issue a claim, recovery-aware rule paths preserve the
represented source instead of normalizing it.

In builds with debug assertions (including ordinary debug and test builds),
rendering first walks the selected document branches against a root
`SyntaxConservationTracker`. It rejects foreign, missing, or duplicate claims
and checks that recovery-free syntax did not use malformed verbatim output
before any text reaches the real sink. This proof state is compiled out when
debug assertions are disabled, including the release and distribution profiles.

## Recovery and verbatim output

A malformed typed field supplies a `SyntaxVerbatimCore`: the exact syntax-owned
source interval eligible for verbatim output, together with the tokens, source
claim, and neighboring-token facts needed at its boundaries. The Java and Kotlin
recovery helpers resolve their typed field and list enums, format comments
outside that core structurally, and decide language-specific list states. Shared
recovery code only assembles those pieces and resolves the exceptional joins.

A required missing role emits no text after the language helper confirms that
syntax owns an empty verbatim core. A malformed role emits its borrowed core and
claims every contained conserved identity exactly once. These paths format the
represented tree; they do not repair syntax, synthesize missing tokens, or
validate whether the source is acceptable to a compiler.

## Formatter-ignore ranges

`@formatter:off` and `@formatter:on` are handled as a root-scoped plan, not by
recursive rule-local searches. When the source contains a possible marker, the
shared planner walks root tokens and their comments in source order, records
complete first-off-wins pairs, derives their source claims, and computes their
lexical boundaries. The resulting ordered plan is immutable for the run.

Only source-ordered list rules that can own an ignored interval query the plan.
They pass their syntax-owned container and direct item ranges, receive runs to
insert and items to skip, and retain ownership of separators and surrounding
layout. This is used at file, import, member, block, and the corresponding Java
declaration/module/statement boundaries; formatter-ignore is not an ambient
per-node switch.

An emitted run claims its parser-backed range, removes the first non-empty
line's indent prefix from lines that share it, normalizes source line endings,
and lets its enclosing list place it at the current indentation. Whether the
`on` marker is part of the run depends on which list item owns that boundary.
Marker comments handled structurally are consumed as control trivia rather than
printed twice.

Plan discovery is linear in source bytes, root tokens, and comments. A list
query uses binary searches into the ordered ranges and one source-order splice
walk; its comparisons are bounded by
`O(items * log(ranges + 1) + items + runs)`. Nested rules do not rescan the
source.

## Lexical joins

Ordinary structured rules place their own spaces and line breaks. Extra lexical
safety is needed only where source-backed exceptional fragments meet structured
output: malformed verbatim cores and formatter-ignore runs.

The shared IR identifies at most the adjacent atom pair on each side and asks
the language's `LexicalSafety` implementation for `None`, `Space`, or
`HardLine`. Java and Kotlin own token classification and their finite
punctuation/keyword join tables. The shared code calls that policy at most once
per present boundary pair; it does not inspect source gaps or retokenize
fragment text. A trailing line comment bypasses the policy and forces a hard
line before following syntax.

Exceptional boundary metadata stays outside ordinary document nodes, so normal
structured layout does not pay for it.

## Document topology

Language rules manipulate opaque, copyable `Doc` handles; apart from the nil
sentinel, they construct documents through a `DocBuilder`. The builder exposes
text, source-backed fragments, concatenation/join, groups, indentation, line
variants, and `if_break`. `DocNode`, the arena, source-claim representation, and
renderer traversal are crate-private to `jolt_fmt_ir`.

One formatting run has one append-only arena. A `Doc` is an index into that
arena; recursive topology is stored as handles and compact child ranges rather
than boxed trait objects or a public recursive tree. Small concatenations stay
inline, larger dynamic lists use reusable builder scratch, and the root builder
reserves arena capacity from a fixed linear estimate for larger inputs.

The relevant operators are:

| Operator                  | Meaning                                                   |
| ------------------------- | --------------------------------------------------------- |
| `text`, `space`, `nil`    | Literal output, one pending space, or no output.          |
| `literal_text`            | Pre-measured text that may span source lines.             |
| `concat`, `join`          | Ordered composition.                                      |
| `group`, `force_group`    | Fit-or-break layout, or an always-broken group.           |
| `indent`                  | One additional indentation level while rendering a child. |
| `line`, `soft_line`       | A space/empty string when flat and a newline when broken. |
| `hard_line`, `empty_line` | One or two unconditional newlines.                        |
| `if_break`                | Select a branch from the current group's actual mode.     |

`if_break` outside a group is a document error, not an implicit layout choice.

## Rendering and bounded fits

The renderer is iterative: explicit command stacks walk the arena and stream
chunks to a `RenderSink`. A sink may halt early without building the whole
output string. Pending spaces and indentation are resolved at write time, and
text widths are precomputed on document text nodes.

At a non-forced group, a flat-fit probe measures the group's flat branch plus
the already-pending render stack against the configured line width. Within the
group's measured flat branch, a hard line or multiline literal forces a break;
in the pending stack, a line boundary ends the current-line probe successfully.
Excess width forces a break. Fit probing uses reused scratch stacks and has a
fixed budget of 4,096 semantic commands per group; exhausting the budget
conservatively chooses break mode. There is no unbounded best-fit search,
alternative scoring, or recursive renderer walk.

## Further reading

- [Wadler, "A prettier printer"](https://homepages.inf.ed.ac.uk/wadler/papers/prettier/prettier.pdf)
- [Prettier, "Technical details"](https://prettier.io/docs/technical-details)
- [Biome, "Architecture"](https://biomejs.dev/internals/architecture/)
