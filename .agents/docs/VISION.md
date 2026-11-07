# Jolt Vision

## Now: a Java formatter

Jolt is a fast, opinionated Java formatter, shipped as a static native CLI and
as a dprint WebAssembly plugin built from the same code.

What it promises:

- **Fast enough to run on every save and commit**, even on large codebases. The
  CLI needs no JVM, and every phase is linear or explicitly bounded.
- **Never loses or moves code.** Every token and comment of the input is in the
  output, and formatting the output again changes nothing. Files with syntax
  errors are refused by default rather than guessed at.
- **One style, few options.** Line width, indent width, and tabs or spaces. No
  compatibility modes, no per-construct knobs. The style is written down in
  [Java style](../../docs/internals/java-style.md).
- **Layout only.** The formatter may sort imports and normalize a closed list of
  punctuation (see [the formatter design](../../docs/internals/formatter.md)),
  but it never renames, removes unused imports, or refactors.

How it is built: a lossless syntax tree that borrows the source, comments placed
once before layout, and a Wadler/Prettier-style document printer. The tree and
printer crates know nothing about Java, so other languages can share them.

## Later

These are directions, not commitments. None of them exist yet.

- **More JVM languages.** Kotlin, and perhaps Scala, as formatters on the same
  language-agnostic tree and printer crates, each with its own syntax and rule
  crates and its own written style.
- **Source tools beyond formatting.** A linter, and import cleanup that needs
  more semantic knowledge than sorting does.
- **Project tooling.** Explaining, locking, and eventually building JVM projects
  from a small manifest, alongside Gradle and Maven rather than instead of them
  at first.

Each would keep the formatter's principles: a small product surface, boring
defaults, and machinery that users can see and reason about.
