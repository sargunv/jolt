# What is Jolt?

Jolt is a high performance Java source formatter.

The CLI is a static binary with no runtime dependency. The same engine also
builds to WebAssembly for dprint, editors, and other WASM hosts. You can run it
in CI, from a plugin, or on the command line without installing a JVM to drive
the formatter.

## Approach

Jolt is aimed at large Java codebases. Format-on-save always runs, but a slow
formatter adds noticeable latency after every save. Jolt is built so that
latency stays small even on big trees.

The layout model is meant to be uniform: the same wrapping and indentation rules
apply to method calls, annotation arguments, arrays, enum constants, records,
switch rules, and the rest.

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
