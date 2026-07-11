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

PGO is intentionally separate from ordinary local Cargo release builds, while
the production packaging workflow uses it by default:

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

## Native release artifacts

`mise run release:build` packages PGO builds for every release target whose
architecture matches its cargo-dist runner, while
`mise run build:jolt --mode release` remains the faster, ordinary release build
for local development. The release task selects the configured native target for
the current host (the static musl target on Linux). cargo-dist invokes
`dist/jolt/build.py` once per release target. Training always runs as a host
executable, then the merged profile is applied while compiling the requested
`CARGO_DIST_TARGET`. Each same-architecture result is executed for version and
stdin-formatting smoke tests before packaging.

The ARM64 Windows artifact is currently built by cargo-dist on an x64 Windows
runner. Indexed profiles are not reused across architectures, so that artifact
uses the ordinary LTO release profile until a native ARM64 Windows runner is
available.

cargo-dist 0.32 does not support overriding a Cargo package's build command, so
the shipped CLI is represented by a small generic package in
`dist/jolt/dist.toml`. Its version must match `crates/jolt_cli/Cargo.toml`; the
`release:cut` task updates both. Generic builds also cannot use cargo-dist's
`cargo-auditable` integration, so release binaries currently omit that metadata.
The regular Cargo package remains the source of the binary and all development
builds.
