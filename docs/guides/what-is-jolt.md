# What is Jolt?

Jolt is a fast, opinionated Java formatter.

The CLI is one static binary with no JVM. The same formatter builds to
WebAssembly as a dprint plugin, which is also what runs the playground on the
home page.

## Approach

A formatter in a pre-commit hook or on editor save runs constantly, so Jolt is
built to stay fast on large codebases: a lossless syntax tree that borrows the
source, and a single-pass
[Wadler/Prettier](https://homepages.inf.ed.ac.uk/wadler/papers/prettier/prettier.pdf)-style
document printer. See [how the formatter works](/internals/formatter) for the
design and [Java style](/internals/java-style) for the output it produces.

Configuration stays small: line width, indent width, and tabs or spaces.

## Performance

Formatting the
[Spring Framework](https://github.com/spring-projects/spring-framework) sources:

<ClientOnly>
  <BenchStrip />
</ClientOnly>

`mise run bench` measures this on your machine.
