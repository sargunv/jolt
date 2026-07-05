# What is Jolt?

Jolt is a high performance Java source formatter.

The CLI is a static binary with no runtime dependency. The same engine also
builds to WebAssembly for dprint, editors, and other WASM hosts. You can run it
in CI, from a plugin, or on the command line without installing a JVM to drive
the formatter.

## Approach

Jolt is aimed at large Java codebases and editor integrations. A slow formatter
in pre-commit or editor save hooks adds noticeable latency on every run. Jolt is
built so that latency stays miniscule even on large codebases.

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
  <SpringBenchmarkChart />
</ClientOnly>

Measured on a Ryzen AI Max+ 395. Lower is better.

In native mode, Jolt formats the full corpus in under half a second—about 20×
faster than `google-java-format` in the same run.

## Acknowledgements

Jolt’s formatter is built around a document IR in the
[Wadler/Prettier](https://homepages.inf.ed.ac.uk/wadler/papers/prettier/prettier.pdf)
tradition, with practical influence from
[Biome](https://biomejs.dev/formatter/),
[Oxfmt](https://oxc.rs/docs/guide/usage/formatter.html), and
[Ruff](https://docs.astral.sh/ruff/formatter/). Its renderer uses local flat-fit
probing for grouped documents, with an explicit boundedness guard inspired by
the
[Oppen](http://i.stanford.edu/pub/cstr/reports/cs/tr/79/770/CS-TR-79-770.pdf)
lineage and later work on linear, bounded pretty-printing by
[Swierstra and Chitil](https://kar.kent.ac.uk/24041/1/LinearOlaf.pdf).
