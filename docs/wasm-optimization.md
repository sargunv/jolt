# Optimized dprint plugin builds

Build the release dprint plugin and run Binaryen's post-link optimizer with:

```sh
mise install
mise run build:dprint-plugin-optimized
```

The optimized artifact is
`target/wasm32-unknown-unknown/release/jolt_fmt_dprint.opt.wasm`. The ordinary
release artifact remains beside it unchanged, so this workflow does not affect
native releases or debug plugin development.

Rust instrumented PGO is not currently practical for this plugin. The Rust
toolchain does not ship `profiler_builtins` for `wasm32-unknown-unknown`, so a
build using `-Cprofile-generate` fails before linking. More fundamentally, the
dprint plugin has no WebAssembly imports: its sandbox has no filesystem call or
host callback through which LLVM's profiling runtime could persist a `.profraw`
file. Dprint does not extract LLVM profile buffers from plugin exports either.
Supporting PGO would therefore require a custom profiler runtime and a change to
the dprint plugin host protocol, not just a release-build flag.

`wasm-opt -O3` is a reproducible, target-appropriate alternative that requires
no runtime profiling channel. The Binaryen version is pinned in `mise.toml`.
Release packaging applies the same pinned optimizer to the published
`plugin.wasm`; the separate task is primarily useful for local validation.
