# Profile-guided Jolt builds

The PGO build trains a release Jolt CLI on the same adversarial and realistic
Java corpora and realistic Kotlin corpus used by the benchmark harness, then
rebuilds the CLI using the resulting execution profile. The Kotlin corpus is the
pinned MapLibre Compose source import, a representative application and library
workload.

```sh
mise install
mise run build:jolt-pgo
target/pgo/optimized/release/jolt --version
```

All instrumented binaries, copied training inputs, raw profiles, merged profile,
and optimized artifacts stay below the ignored `target/pgo` directory. Each run
removes old profiles and build outputs, so profiles from another revision cannot
silently contaminate the result. The workflow fails when either corpus is
missing or empty, when the matching LLVM profile tool is unavailable, or when
training produces no profile.

PGO is intentionally a separate build rather than the default release workflow:

- It performs two clean release builds and formats both corpora, making it much
  slower than `mise run build:jolt --mode release`.
- Its optimization decisions reflect the checked-in benchmark corpus selection.
  Workloads with substantially different language or file-size distributions may
  benefit less.
- Profiles are compiler-specific. The workflow uses `llvm-profdata` from the
  active Rust toolchain's `llvm-tools-preview` component rather than relying on
  a potentially incompatible system LLVM installation.
- The workflow intentionally replaces inherited `RUSTFLAGS` so both builds use
  the same controlled profile instrumentation flags. Add durable compiler flags
  to the repository build configuration rather than the caller environment.
- Setting `LLVM_PROFDATA` overrides that tool lookup for unusual toolchains; the
  caller is then responsible for selecting a compatible version.

The final binary is `target/pgo/optimized/release/jolt`. Run the command again
after compiler, dependency, source, or representative-workload changes;
generated profiles are not portable release inputs and should not be committed.
