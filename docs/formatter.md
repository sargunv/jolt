# Jolt Formatter Architecture

## Purpose

The first Jolt product should be a formatter engine for Java and Kotlin source code.

The formatter is an adoption wedge for Jolt, but it should not be treated as a throwaway CLI tool. It should be the first durable piece of Jolt's source tooling substrate: a reusable, wasm-compatible formatting engine with a native CLI wrapper and a dprint plugin wrapper.

The formatter should be opinionated, profile-driven, and compatibility-oriented. It should not introduce a new formatting style language, expose arbitrary formatting knobs, or invite users to assemble their own style from dozens of settings.

## Scope

### In scope

- A reusable formatter engine for Java and Kotlin source text.
- A native CLI wrapper for users who do not use dprint.
- A dprint plugin compiled to `wasm32-unknown-unknown`.
- Java formatting profiles compatible with:
  - Google Java Format.
  - Google Java Format AOSP mode.
  - Palantir Java Format.
- Kotlin formatting profiles compatible with:
  - ktfmt.
  - ktfmt's Kotlin language style, if supported separately.
- A native-only oracle test harness that imports upstream formatter fixtures and materializes expected outputs.
- A shared formatter IR and renderer used by both Java and Kotlin printers.
- Formatter-native syntax infrastructure: lexer, parser, lossless CST, trivia, typed syntax wrappers.

### Out of scope for the first product

- Gradle integration.
- Maven integration.
- Project model integration.
- Semantic import cleanup.
- Adding missing imports.
- Removing unused imports.
- Linting.
- Autofix beyond formatting.
- Dependency resolution.
- Build execution.
- IDE/LSP integration.
- Arbitrary user-defined formatting configuration.

## Product Shape

The first product is a formatting engine.

```text
formatter engine
  -> native CLI wrapper
  -> dprint wasm plugin
```

The CLI and dprint plugin should be thin shells over the same core engine. The engine should be pure: given source text, language, and options, it returns formatted source text plus diagnostics.

```text
source text + language + format options
  -> formatted text + diagnostics
```

The engine should not know about filesystems, directory walking, ignore files, terminals, process spawning, Gradle, Maven, or editor state.

## Command and Configuration Surface

One formatting invocation may contain both Java and Kotlin files, so profile options must be language-scoped.

```bash
jolt fmt
jolt fmt --check
jolt fmt --write

jolt fmt --java-profile google
jolt fmt --java-profile aosp
jolt fmt --java-profile palantir

jolt fmt --kotlin-profile ktfmt
jolt fmt --kotlin-profile kotlinlang
```

Tentative defaults:

```text
java-profile   = google
kotlin-profile = ktfmt
```

The dprint plugin should expose equivalent configuration:

```json
{
  "plugins": ["https://example.invalid/jolt_fmt.wasm"],
  "jolt": {
    "javaProfile": "google",
    "kotlinProfile": "ktfmt"
  }
}
```

Profiles configure Jolt's internal formatter behavior. They are not oracle definitions. Oracle suites are test harness concepts that compare one Jolt profile configuration against one upstream formatter's output.

## File Discovery

File discovery belongs to the native CLI, not the formatter engine.

Default behavior:

```text
default include:
  **/*.{java,kt,kts}

default exclude:
  none

always applied:
  .gitignore
  .ignore
```

User-provided includes replace the default include set.

User-provided excludes stack on top of defaults and ignore-file behavior.

In other words:

```text
final candidate files =
  user_includes.unwrap_or(["**/*.{java,kt,kts}"])
  - user_excludes
  - files ignored by .gitignore or .ignore
```

The wasm engine and dprint plugin should not implement recursive file discovery.

## Architecture Overview

The formatter should own its parser and syntax model.

```text
source text
  -> lexer
  -> tokens + trivia
  -> parser
  -> lossless CST
  -> typed syntax API
  -> language-specific printer
  -> common Doc IR
  -> common renderer
  -> formatted text
```

The architecture should be formatter-native from the beginning. It should not be built on Tree-sitter, an AST-only parser, or a parser model that loses whitespace and comments.

The durable architecture is:

```text
Language-specific:
  - lexer
  - parser
  - syntax kinds
  - typed CST wrappers
  - CST-to-Doc printer
  - profile behavior

Shared:
  - source text utilities
  - text ranges and line index
  - green/red syntax tree infrastructure
  - trivia representation
  - parser diagnostics
  - Doc IR
  - renderer
  - engine API
  - wasm-safe option model
```

## Why Not Tree-sitter

Tree-sitter is useful for editor-oriented parsing and error-tolerant syntax trees, but it is not the right foundation for this formatter.

The formatter needs a lossless source model: tokens, whitespace, comments, byte ranges, newlines, and trivia attachment. That model is central, not incidental.

The most relevant formatter/toolchain references do not use Tree-sitter as their source substrate:

- Ruff owns its parser, trivia utilities, Python formatter, and language-agnostic formatter IR.
- Biome owns parser infrastructure, a lossless CST with trivia, and formatter infrastructure.
- Oxc owns its lexer/parser/AST/trivia/codegen stack.

The lesson is not merely that Tree-sitter is absent. The lesson is that serious formatter infrastructure tends to own the syntax substrate it depends on.

For Jolt, avoiding Tree-sitter means accepting more initial parser work in exchange for:

- wasm-first implementation control,
- formatter-native trivia behavior,
- stable syntax APIs for printers,
- fewer parser-model impedance mismatches,
- a stronger foundation for later source tools.

## Syntax Model

The formatter should use a lossless concrete syntax tree.

A semantic AST is not sufficient for formatting. Formatting needs source-level structure, comments, whitespace, and syntactic edge cases. Semantic meaning may become important for later tools, but pure formatting should remain layout-only.

### Tree model

Use a green/red tree architecture.

Green tree:

- immutable,
- compact,
- parentless,
- stores syntax kind, text length, and children,
- suitable for sharing and future incremental use.

Red tree:

- ergonomic wrapper around green nodes,
- parent-aware,
- computes offsets,
- used by formatter and typed syntax APIs.

The implementation does not need to expose green/red terminology publicly. The important design point is that storage and ergonomic traversal are separate.

### Elements

The syntax tree should represent:

```text
nodes:
  compilation units / files
  package declarations
  imports
  class declarations
  method declarations
  property declarations
  blocks
  expressions
  annotations
  comments where structurally necessary
  error nodes

tokens:
  identifiers
  keywords
  literals
  operators
  braces
  punctuation
  delimiters

trivia:
  whitespace
  newlines
  line comments
  block comments
  Javadoc
  KDoc
  license headers
  dangling comments
```

### Trivia

Trivia should attach to tokens rather than live only in a side table.

A starting model:

```text
leading trivia:
  whitespace/comments before a token that visually belong to that token

trailing trivia:
  whitespace/comments after a token on the same line that visually belong to the previous token

dangling trivia:
  comments inside otherwise-empty or ambiguous syntax positions
```

The model must handle at least:

- file headers before package declarations,
- Javadoc and KDoc before declarations,
- line comments at the ends of statements,
- comments between modifiers and annotations,
- comments inside empty blocks,
- comments around imports,
- disabled-code comments,
- formatter suppression comments, if supported.

## Parser Architecture

Use hand-written parsers.

For both Java and Kotlin:

```text
lexer:
  source text -> tokens + trivia

parser:
  tokens -> syntax events

tree builder:
  syntax events -> lossless green tree

typed syntax layer:
  raw syntax nodes -> ergonomic Java/Kotlin syntax wrappers
```

The parser should use recursive descent for declarations, statements, types, and structural syntax. Expressions can use Pratt parsing or precedence climbing.

The parser should support error recovery. A formatter should be able to report parse errors cleanly and avoid destructive output when source is syntactically invalid.

### Parser event stream

The parser should not allocate final tree nodes directly. It should emit events that a tree builder consumes.

Example shape:

```rust
enum Event {
    StartNode(SyntaxKind),
    Token(SyntaxKind),
    FinishNode,
    Error(ParseError),
}
```

This keeps parser control flow separate from syntax tree storage and leaves room for marker-based parsing patterns where the parser starts a node before it knows its final kind.

## Formatter IR

The shared formatter middle should be a Wadler/Prettier/Biome-style document algebra.

Language printers should not render strings directly. They should convert syntax trees into a common Doc IR. The renderer then decides where groups fit, where lines break, and how indentation is applied.

Minimum IR:

```rust
enum Doc {
    Nil,
    Text(String),
    Line,
    SoftLine,
    HardLine,
    Concat(Vec<Doc>),
    Group(Box<Doc>),
    Indent(Box<Doc>),
    IfBreak {
        breaks: Box<Doc>,
        flat: Box<Doc>,
    },
}
```

Likely later additions:

```text
LineSuffix
LineSuffixBoundary
Fill
BestFitting
Labelled groups
Profile-sensitive conditional groups
```

The first implementation should stay small. Add IR features only when real Java/Kotlin formatting cases require them.

## Formatting Profiles

Profiles should be small product-level choices.

They should configure internal whitespace and line-breaking behavior, but should not expose arbitrary style options.

Tentative profile enums:

```rust
pub enum JavaProfile {
    Google,
    Aosp,
    Palantir,
}

pub enum KotlinProfile {
    Ktfmt,
    Kotlinlang,
}
```

A single formatter invocation may use both a Java profile and a Kotlin profile.

```rust
pub struct FormatOptions {
    pub java_profile: JavaProfile,
    pub kotlin_profile: KotlinProfile,
}
```

The formatter should not treat profiles as external executable choices. External executables are only used by the oracle harness.

## Imports Boundary

Import ordering may be formatting.

Import cleanup is not formatting.

`jolt fmt` may:

- sort imports according to the active profile,
- normalize blank lines between import groups according to the active profile.

`jolt fmt` must not:

- remove unused imports,
- add missing imports,
- rename symbols,
- perform semantic refactors,
- expand or collapse wildcard imports unless that behavior is strictly part of the selected profile's formatting behavior and can be reproduced safely without project resolution.

A future `jolt imports` command can perform semantic or project-aware import cleanup.

## Engine API

The formatter core should expose a small, wasm-safe API.

Conceptual shape:

```rust
pub fn format_source(
    source: &str,
    language: Language,
    options: FormatOptions,
) -> FormatResult;

pub enum Language {
    Java,
    Kotlin,
}

pub struct FormatResult {
    pub text: String,
    pub diagnostics: Vec<Diagnostic>,
}
```

The real API may need allocation-aware or FFI-friendly variants for dprint, but the conceptual contract should remain pure.

## Crate Layout

Tentative crate layout:

```text
crates/
  jolt_text/
    SourceText
    TextSize
    TextRange
    LineIndex
    UTF-8 byte/char utilities

  jolt_syntax/
    GreenNode
    GreenToken
    SyntaxNode
    SyntaxToken
    SyntaxElement
    Trivia
    syntax tree traversal
    error nodes

  jolt_java_syntax/
    JavaSyntaxKind
    Java lexer
    Java parser
    Java typed syntax wrappers

  jolt_kotlin_syntax/
    KotlinSyntaxKind
    Kotlin lexer
    Kotlin parser
    Kotlin typed syntax wrappers

  jolt_fmt_ir/
    Doc IR
    groups
    indentation
    line breaking
    renderer

  jolt_java_fmt/
    Java CST -> Doc printer
    Google/AOSP/Palantir profile behavior

  jolt_kotlin_fmt/
    Kotlin CST -> Doc printer
    ktfmt/kotlinlang profile behavior

  jolt_fmt_core/
    public format API
    language dispatch
    option normalization
    diagnostics

  jolt_fmt_cli/
    native CLI wrapper

  jolt_fmt_dprint/
    wasm dprint plugin

  jolt_oracle_tests/
    native-only oracle fixture import and comparison

xtask/
  import-oracles
  update-oracles
```

The exact crate boundaries can change, but the concern boundaries should remain stable.

## Native CLI Wrapper

The CLI owns user-facing command behavior:

- file discovery,
- `.gitignore` and `.ignore` handling,
- include/exclude options,
- check mode,
- write mode,
- stdin/stdout,
- terminal diagnostics,
- optional diff output,
- parallel formatting,
- config file loading, if added.

The CLI should call the same formatter engine used by the dprint plugin.

## dprint Plugin

The dprint plugin should compile to `wasm32-unknown-unknown`.

The plugin owns only dprint integration:

- file extension registration,
- dprint config parsing,
- mapping dprint config to Jolt format options,
- calling the core formatter engine.

The plugin should not contain separate formatting behavior.

The dprint plugin is the reason wasm compatibility must be a hard CI target from the beginning.

## Oracle Test Harness

The oracle test harness is native-only. It can spawn JVM tools, clone upstream repositories, and perform filesystem-heavy fixture import work.

The engine and dprint plugin must not depend on oracle machinery.

### Oracle suites

Initial oracle suites:

```text
google-java-format:
  upstream executable: google-java-format
  upstream fixtures: google-java-format fixtures
  Jolt config: java-profile = google

google-java-format-aosp:
  upstream executable: google-java-format --aosp
  upstream fixtures: google-java-format fixtures
  Jolt config: java-profile = aosp

palantir-java-format:
  upstream executable: palantir-java-format
  upstream fixtures: palantir-java-format fixtures
  Jolt config: java-profile = palantir

ktfmt:
  upstream executable: ktfmt
  upstream fixtures: ktfmt fixtures
  Jolt config: kotlin-profile = ktfmt
```

### Fixture import

Oracle output should be materialized during an explicit import/update step.

```text
xtask import-oracles
  -> checkout pinned upstream formatter repos
  -> collect fixture inputs
  -> run upstream formatter once
  -> write input and expected output into Jolt's oracle fixture directory
  -> record metadata with upstream repo, commit, command, and profile
```

Ordinary test runs should not spawn upstream formatters.

```text
cargo test -p jolt_oracle_tests
  -> read materialized input and expected output
  -> run Jolt formatter in-process
  -> compare output
```

No hash cache is necessary for the initial design. Oracle import is a deliberate update operation, and normal tests are pure and fast.

### Owned tests

Jolt should not invent broad formatter fixtures for upstream-compatible profiles.

Owned tests should focus on Jolt-owned behavior:

- CLI check/write/stdin/stdout behavior,
- include/exclude/ignore behavior,
- dprint plugin loading and config mapping,
- engine API behavior,
- wasm build viability,
- invalid syntax diagnostics,
- narrow regression cases not covered by upstream fixtures.

If Jolt invents its own formatting profile later, that profile should get its own fixture suite.

## Bootstrap Plan

### Milestone 1: shared foundation

Build:

- `jolt_text`,
- `jolt_syntax`,
- `jolt_fmt_ir`,
- a minimal renderer.

Prove:

- crates compile natively,
- wasm-compatible crates compile to `wasm32-unknown-unknown`,
- the renderer can print simple grouped/indented documents.

### Milestone 2: Java syntax skeleton

Build:

- Java lexer,
- Java token and trivia model,
- Java parser skeleton,
- Java lossless CST output.

Support enough Java to parse:

- package declarations,
- imports,
- classes,
- fields,
- methods,
- blocks,
- basic statements,
- basic expressions,
- comments.

### Milestone 3: Java formatter skeleton

Build:

- Java CST-to-Doc printer,
- Google profile option path,
- `format_source` entry point.

Prove:

- a tiny Java file can travel through the full pipeline:

```text
source -> lexer -> parser -> CST -> printer -> Doc -> renderer -> formatted text
```

### Milestone 4: CLI wrapper

Build:

- `jolt fmt`,
- stdin/stdout formatting,
- `--check`,
- `--write`,
- `--java-profile`,
- `--kotlin-profile`,
- default file discovery,
- `.gitignore` and `.ignore` support.

### Milestone 5: dprint proof

Build:

- wasm dprint plugin,
- config mapping,
- Java extension registration,
- one end-to-end Java formatting case through dprint.

Prove:

```bash
cargo build --target wasm32-unknown-unknown -p jolt_fmt_dprint
```

### Milestone 6: oracle importer

Build:

- upstream fixture import for Google Java Format,
- expected output materialization,
- metadata recording,
- in-process comparison against Jolt output.

Then drive formatter development from oracle failures.

### Milestone 7: Java profile expansion

Add:

- AOSP profile behavior,
- Palantir profile behavior,
- Palantir oracle import,
- broader Java grammar coverage.

### Milestone 8: Kotlin syntax and formatter

Build:

- Kotlin lexer,
- Kotlin parser,
- Kotlin lossless CST,
- Kotlin formatter printer,
- ktfmt profile,
- ktfmt oracle import.

Kotlin should follow the same architecture as Java, but it should not be forced into a shared grammar model. The CST is language-specific. The Doc IR and renderer are shared.

## Compatibility Goals

The formatter should be judged by oracle compatibility, not subjective style quality.

Early Java target:

```text
java-profile = google
  -> high compatibility with materialized google-java-format fixtures
  -> idempotent on all passing cases
  -> no parse failures on valid fixture inputs
```

Eventually:

```text
java-profile = google
  -> compatible with Google Java Format fixture output

java-profile = aosp
  -> compatible with Google Java Format AOSP fixture output

java-profile = palantir
  -> compatible with Palantir Java Format fixture output

kotlin-profile = ktfmt
  -> compatible with ktfmt fixture output
```

Compatibility should be reported as a measurable percentage during development.

## Failure Behavior

The formatter should avoid destructive output.

If parsing fails severely, the formatter should return diagnostics and avoid rewriting the file unless a safe partial-formatting strategy is explicitly designed.

The first implementation can choose a conservative rule:

```text
If parse errors exist, do not write formatted output by default.
```

Later, Jolt can distinguish recoverable parse errors from fatal formatter errors.

## Design Principles

### Own the substrate

Formatting depends on syntax shape, token ranges, and trivia. Jolt should own those layers.

### Keep the core pure

The core formatter should be deterministic, wasm-compatible, and filesystem-free.

### Reuse the middle

Java and Kotlin need separate syntax frontends and printers, but they should share the Doc IR and renderer.

### Let oracles guide compatibility

For upstream-compatible profiles, imported upstream fixtures and materialized upstream outputs are the source of truth.

### Avoid style configuration sprawl

Expose profiles, not knobs.

### Keep formatting layout-only

Formatting should not perform semantic source actions.

## References

Architecture and formatter references:

- Philip Wadler, "A prettier printer": https://homepages.inf.ed.ac.uk/wadler/papers/prettier/prettier.pdf
- Biome architecture: https://biomejs.dev/internals/architecture/
- Biome formatter crate: https://docs.rs/biome_formatter
- Ruff formatter documentation: https://docs.astral.sh/ruff/formatter/
- Ruff contributing architecture notes: https://docs.astral.sh/ruff/contributing/
- Oxc parser documentation: https://oxc.rs/docs/contribute/parser.html
- Oxc architecture: https://github.com/oxc-project/oxc/blob/main/ARCHITECTURE.md
- dprint Wasm plugin development: https://github.com/dprint/dprint/blob/main/docs/wasm-plugin-development.md

Oracle references:

- Google Java Format: https://github.com/google/google-java-format
- Palantir Java Format: https://github.com/palantir/palantir-java-format
- ktfmt: https://github.com/facebook/ktfmt

## Open Questions

- Should the Java parser initially support all modern Java syntax before broad formatting work, or should grammar coverage expand only as oracle failures require it?
- Should formatter suppression comments be supported in the first version?
- Should invalid syntax always block writes, or should recoverable parse errors allow partial formatting later?
- Should `kotlinlang` be a distinct Kotlin profile in the first public version, or should the first Kotlin target be only ktfmt compatibility?
- How much of the green/red syntax infrastructure should be generic before Java exists, versus extracted after the Java path proves the shape?
- Should oracle fixtures be committed to the repo, stored as a generated artifact, or both?
