# Formatter benchmarks

Run the formatter benchmark:

```sh
mise run benchmark
```

The command builds the benchmark drivers, measures the imported Spring Framework
Java and MapLibre Compose Kotlin corpora, and overwrites
`tools/bench/reports/machines/<id>.json`. The ID combines a platform prefix with
a hash of the OS family, architecture, processor, logical CPU count, and
installed memory. The documentation's Spring benchmark chart imports the
designated reference machine report directly. Review the report diff with the
code; committing it is acceptance. Git retains earlier measurements, so the
harness does not implement a separate record or acceptance workflow.

Each report contains parse-only, format-only over an already-parsed tree, and
end-to-end measurements. It records raw timing samples and dispersion,
allocation counts and bytes, peak RSS, syntax-tree bytes per token and node,
parse, format, and end-to-end nanoseconds per token, formatter document nodes
per token, source identity, toolchain, corpus digests, and the commands used.
The same run measures fresh-corpus whole-CLI performance for native Jolt and the
optimized dprint plugin on both languages. Pass `--comparison-tools` to also
measure google-java-format and prettier-java on Java:

```sh
mise run benchmark --comparison-tools
```

Corpus copying happens outside the timed region. Whole-CLI results use one
warmup and five recorded samples and include tool versions and modified-file
counts.

Timing and allocation samples run in separate release binaries so allocation
accounting cannot affect timing. Peak RSS uses a dedicated one-shot subprocess
wrapped by the platform `/usr/bin/time`. Syntax-tree and document-arena metrics
use benchmark-only accessors and add no fields or counters to production data.

The harness digest covers the Mise contract, corpus definitions, Python
orchestration, and Rust driver. Measurement fails if the source, harness, or
corpora change during a run. Reports from different machines are not directly
comparable.
