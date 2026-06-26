# Milestone 3 Diagnostics

## Direction

Milestone 3 should establish the final shared diagnostics and syntax outcome
policy used by lexer, parser, formatter, CLI, and dprint layers.

The core split is:

```text
Diagnostic = what went wrong
Syntax outcome = what source shape was produced
Formatter policy = what the formatter is willing to write
```

Do not put formatter write-safety policy into the general diagnostic shape.
Fields such as `blocks_formatting` are a smell because they encode one
consumer's decision as if it were a property of the diagnostic itself.

## Reference Shape

This follows the Oxc-style separation, but without coupling Jolt's internal
model to `miette`.

Oxc's parser returns diagnostics plus parse outcome facts. A recovered parse can
still produce a usable tree, while an aborted parse is a different result state.
Jolt should apply that idea to both lexing and parsing.

Biome and Ruff also point in this direction:

- Biome has a shared diagnostics crate with category, severity, location,
  advices, and tags. Formatter or CLI behavior consumes diagnostics; it is not
  stored as a generic diagnostic field.
- Ruff keeps diagnostic reporting separate from fix applicability. Whether a
  transformation is safe to apply is operation policy, not a universal
  diagnostic property.

## Crate Boundary

Add a small shared crate:

```text
crates/jolt_diagnostics
```

It should depend on `jolt_text` and be usable by:

- `jolt_syntax`,
- language syntax crates,
- formatter crates,
- CLI and dprint wrappers.

`jolt_fmt_core` may re-export the public diagnostic types for formatter users,
but lower-level syntax crates should not depend on formatter core.

## Diagnostic Data

The shared diagnostic type should be plain, deterministic data:

```rust
pub struct Diagnostic {
    pub code: &'static str,
    pub severity: Severity,
    pub stage: DiagnosticStage,
    pub message: String,
    pub range: Option<TextRange>,
}

pub trait DiagnosticCode {
    fn as_str(&self) -> &'static str;
}

pub enum Severity {
    InternalError,
    Error,
    Warning,
    Note,
}

pub enum DiagnosticStage {
    Config,
    Lexer,
    Parser,
    Formatter,
}
```

Design notes:

- `code` should be required.
- Diagnostic production should use typed per-domain code enums that implement
  `DiagnosticCode`, not free-form strings at call sites and not one giant
  workspace-wide enum.
- The final shared `Diagnostic` may store the stable rendered code string
  because that is the transport, snapshot, and serialization shape.
- `severity` describes how to understand the diagnostic. `InternalError` is for
  Jolt implementation or invariant failures, similar to an HTTP 500; `Error` is
  for user source/config failures, similar to an HTTP 400.
- `stage` describes where the diagnostic was produced, in runtime order.
- `range` is optional because config, internal, or file-wide diagnostics may not
  have a precise source span.
- Do not store file paths in the engine-level diagnostic. CLI, dprint, LSP, and
  project-level callers can attach file context outside the pure formatter
  engine.
- Do not store line and column. Renderers compute them from `LineIndex`.
- `InternalError` diagnostics should still be user-visible when they block an
  operation. They should be phrased as Jolt bugs, usually with report-this-bug
  advice in the CLI renderer.
- Keep the type easy to snapshot and serialize later.

Language-specific codes are acceptable:

```text
java.lex.invalid_numeric_literal
java.lex.unterminated_string_literal
java.parse.expected_token
java.parse.invalid_statement
format.unimplemented
internal.syntax.invalid_event_stream
internal.format.unhandled_syntax_node
```

Authoring should look like:

```rust
pub enum JavaLexDiagnosticCode {
    InvalidNumericLiteral,
    UnterminatedStringLiteral,
}

impl DiagnosticCode for JavaLexDiagnosticCode {
    fn as_str(&self) -> &'static str {
        match self {
            Self::InvalidNumericLiteral => "java.lex.invalid_numeric_literal",
            Self::UnterminatedStringLiteral => "java.lex.unterminated_string_literal",
        }
    }
}
```

This prevents typos where diagnostics are produced while avoiding a central enum
that every language and tool crate has to edit.

## Syntax Outcomes

Lexer and parser results should expose outcome facts separately from
diagnostics:

```rust
pub enum SyntaxOutcome {
    Clean,
    Recovered,
    Aborted,
}
```

Meaning:

- `Clean`: lexing/parsing completed without diagnostics that affected syntax
  validity.
- `Recovered`: lexing/parsing emitted diagnostics but still produced a complete
  lossless syntax tree. For example, an unknown token or parser error node may
  preserve source and allow later tools to inspect nearby structure.
- `Aborted`: lexing/parsing could not produce a trustworthy complete syntax
  tree.

For the current green tree parser, `Aborted` may be rare because structural
event failures currently panic or return construction errors. Milestone 3 should
make this state explicit before formatter policy depends on it.

Lexer diagnostics participate in this outcome. If lexing emits an unknown token
and parsing still constructs a full tree, the syntax result is `Recovered`. If
lexing cannot produce enough token structure for parsing to proceed, the syntax
result is `Aborted`.

## Java Diagnostics

Java lexer and parser code should produce the shared `Diagnostic` shape
directly. The type-safe authoring surface is the per-domain diagnostic code
enum, not a separate Java-only diagnostic kind hierarchy.

Final state:

- Java lexer diagnostics are `Diagnostic` values with `DiagnosticStage::Lexer`
  and Java lexer code enums.
- Java parser diagnostics are `Diagnostic` values with `DiagnosticStage::Parser`
  and Java parser code enums.
- Parser recovery state is parser state, not a second diagnostic model.
- Public Java parse results expose shared diagnostics and syntax outcome facts.

Lexer example:

```rust
impl JavaLexDiagnosticCode {
    pub const fn message(self) -> &'static str;
}
```

Parser diagnostics should move away from arbitrary message-only diagnostics
toward stable Java parser code enums. The message can remain human-readable and
specific, but the emitted code should be stable enough for snapshots, CLI
output, and future machine consumers.

## Formatter Policy

Formatter policy consumes syntax outcome:

```rust
match syntax.outcome {
    SyntaxOutcome::Clean => format,
    SyntaxOutcome::Recovered => block_by_default,
    SyntaxOutcome::Aborted => block,
}
```

The first policy should be conservative: recovered or aborted syntax produces
diagnostics and no formatted output by default.

Later, Jolt may add an explicit option similar to Biome's `formatWithErrors` to
format through recovered syntax. That should be a formatter option, not a
diagnostic field.

`FormatResult` should make blocked output structural:

```rust
pub enum FormatStatus {
    Formatted,
    Unchanged,
    Blocked,
}

pub struct FormatResult {
    pub formatted_source: Option<String>,
    pub diagnostics: Vec<Diagnostic>,
    pub status: FormatStatus,
}
```

Returning `None` for formatted source when blocked is clearer than returning the
original source plus an error diagnostic. Callers already have the original
source if they need to preserve it.

## Miette

Do not couple the internal diagnostic model to `miette`.

`miette` remains a good candidate for native CLI rendering because it has rich
terminal reports, labels, source snippets, help text, and screen-reader-aware
output. Use it as an adapter at the CLI boundary if it earns its dependency
cost.

The wasm formatter engine, syntax crates, and shared diagnostic model should
remain plain data and independent from terminal rendering.

## Tests

Milestone 3 tests should live with the crate that owns the behavior.

`jolt_diagnostics` should stay language-agnostic. Its tests should be minimal
and cover only behavior the shared crate owns, such as typed code enums feeding
the stored stable code string. Do not add tests that merely duplicate enum
definitions, constructors, or field accessors.

`jolt_java_syntax` should port the existing Java lexer/parser diagnostic
production tests to the shared `Diagnostic` shape. Parser diagnostic cases
should naturally be covered by `JavaParse` debug snapshots, since parse
snapshots include diagnostics and syntax outcome. Lexer-only diagnostic tests
can remain focused lexer tests unless the lexer result gains its own useful
snapshot serializer.
