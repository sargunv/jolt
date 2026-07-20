# What is Jolt?

Jolt is a high-performance Java and Kotlin source formatter.

The CLI is a static binary with no runtime dependency. The same engine also
builds to WebAssembly for dprint, editors, and other WASM hosts. You can run it
in CI, from a plugin, or on the command line without installing a JVM to drive
the formatter.

## Approach

Jolt is aimed at large Java and Kotlin codebases and editor integrations. A slow
formatter in pre-commit or editor save hooks adds noticeable latency on every
run. Jolt is built so that latency stays miniscule even on large codebases.

The layout model is meant to be uniform: the same wrapping and indentation rules
apply to method calls, annotation arguments, arrays, enum constants, records,
switch rules, and the rest, without special exceptions for any particular
symbols.

Like other modern anti-bikeshedding formatters, configuration stays small. You
set line width, indent width, and tabs vs. spaces.

## Performance

On the [Spring Framework](https://github.com/spring-projects/spring-framework)
Java sources (~9,200 files):

<ClientOnly>
  <BenchStrip />
</ClientOnly>

Jolt’s formatter builds on the
[Wadler/Prettier](https://homepages.inf.ed.ac.uk/wadler/papers/prettier/prettier.pdf)
document-IR tradition; see [how the formatter works](/internals/formatter) for
details and full references.
