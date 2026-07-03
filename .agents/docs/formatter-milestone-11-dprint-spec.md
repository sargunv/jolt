# Formatter Milestone 11: dprint Wrapper Spec

## Purpose

Milestone 11 adds dprint integration after the Java formatter and native CLI
have proven that `jolt_fmt_core::format_source` is the stable public engine
boundary.

This is a wrapper milestone, not a formatter milestone. The dprint plugin should
translate dprint's plugin protocol into Jolt's existing pure engine API, then
get out of the way.

## Research Summary

dprint Wasm plugins are schema-versioned `.wasm` modules. Rust plugins normally
use `dprint-core`'s `wasm` feature, implement `SyncPluginHandler`, and let
`generate_plugin_code!` export the protocol functions. The handler resolves a
plugin-specific configuration object from a `ConfigKeyMap` plus dprint's
`GlobalConfiguration`, returns `FileMatchingInfo`, and formats each request by
returning changed bytes, no-change, or an error. For Jolt, the resolved
plugin-specific configuration should be `jolt_fmt_core::FormatOptions`.

dprint owns repository-level behavior around plugin URLs, plugin order,
top-level includes/excludes, associations, overrides, nested config discovery,
incremental formatting, and `.gitignore` handling. The Jolt plugin should not
duplicate any of that. It should only declare the file extensions it supports,
resolve Jolt's small option set, choose the Jolt language for the requested
file, and call `jolt_fmt_core`.

## Public dprint Configuration

The initial dprint configuration key is `jolt`.

```json
{
  "lineWidth": 80,
  "indentWidth": 2,
  "useTabs": false,
  "jolt": { "lineWidth": 80, "indentWidth": 2, "useTabs": false },
  "plugins": ["./target/wasm32-unknown-unknown/release/jolt_fmt_dprint.wasm"]
}
```

Supported plugin keys:

- `lineWidth`,
- `indentWidth`,
- `useTabs`.

Resolution order:

```text
Jolt defaults
  <- dprint global lineWidth / indentWidth / useTabs
  <- jolt.lineWidth / jolt.indentWidth / jolt.useTabs
  <- dprint per-file override config, as applied by dprint-core
```

Resolved defaults remain Jolt's defaults:

```text
lineWidth   = 80
indentWidth = 2
useTabs     = false
```

The resolver should produce `jolt_fmt_core::FormatOptions` directly. Do not add
a parallel dprint-owned `Configuration` or `DprintFormatOptions` struct when the
existing core type already represents the full option set.

Any dprint JSON schema for the `jolt` configuration should be generated from the
same `FormatOptions` definition and field metadata. Do not maintain a
handwritten schema or a schema-only mirror type. If schema generation needs
additional derives or field metadata, add them to `FormatOptions` behind the
same narrow feature used by the dprint wrapper.

The plugin should report dprint configuration diagnostics for:

- unknown `jolt` properties,
- `lineWidth = 0`,
- `indentWidth = 0`,
- values outside `jolt_fmt_core::FormatOptions`' representable range.

The plugin should not add compatibility modes, language-specific style presets,
include/exclude keys, suppression-comment settings, import options, or Jolt CLI
config loading.

## Plugin Metadata and File Matching

`jolt_fmt_dprint` should expose a Wasm plugin using dprint's current Rust helper
APIs:

- `dprint-core` with the `wasm` feature for the Wasm target,
- `serde` and `serde_json` only if required by the dprint-core plugin API,
- a `SyncPluginHandler<jolt_fmt_core::FormatOptions>` implementation,
- `generate_plugin_code!`.

If dprint-core requires the resolved handler config to implement serde traits,
add narrowly scoped serde support to `FormatOptions` in `jolt_fmt_core`, ideally
behind an optional feature used by `jolt_fmt_dprint`. Do not introduce a wrapper
configuration struct just to satisfy serialization bounds.

Initial plugin metadata:

```text
name       = Cargo package name
version    = Cargo package version
configKey  = "jolt"
helpUrl    = repository or formatter docs URL
schemaUrl  = empty until Jolt publishes a schema
updateUrl  = none until Jolt publishes plugin update metadata
```

When `schemaUrl` becomes non-empty, it should point to the generated schema for
`FormatOptions`, not a separately authored dprint schema.

Initial file matching:

```text
fileExtensions = ["java"]
fileNames      = []
```

Do not register `.kt` or `.kts` until the Kotlin formatter is implemented.
dprint associations may still route an arbitrary file to the plugin; milestone
11 should treat unsupported extensions as a formatter error with no output
rather than guessing a language.

## Formatting Behavior

For each dprint format request:

1. Decode `request.file_bytes` as UTF-8. Invalid UTF-8 is a formatter error.
2. Determine `Language` from `request.file_path`.
3. Use `request.config` as the resolved `FormatOptions`.
4. Call `jolt_fmt_core::format_source(source, language, &options)`.
5. Return no change when the core status is `Unchanged`.
6. Return changed bytes when the core status is `Formatted`.
7. Return an error when the core status is `Blocked`.

Blocked formatting should preserve Jolt's diagnostic facts. The dprint-facing
error text should include stable diagnostic code, severity, stage, message, and
line/column when a range exists. The plugin should not invent a separate
diagnostic model.

The plugin should not:

- read or write files,
- walk directories,
- inspect `.gitignore`, `.ignore`, or `jolt.toml`,
- spawn the native `jolt` CLI,
- call other dprint plugins through `format_with_host`,
- implement formatter rules outside `jolt_fmt_core`,
- maintain its own formatter option type.

## Build and Packaging

The crate already exists as `jolt_fmt_dprint`. Milestone 11 should turn it from
a placeholder into a real plugin while preserving native testability.

Expected crate shape:

```text
jolt_fmt_dprint/
  src/lib.rs
  src/configuration.rs  # dprint ConfigKeyMap -> FormatOptions resolver
  src/handler.rs
  tests/
    configuration.rs
    dprint_smoke.rs
```

Use target/feature gates so native tests can exercise configuration resolution
without requiring a Wasm runtime:

```text
native build:
  compiles configuration and test helpers

wasm32-unknown-unknown build with the wasm feature:
  compiles the exported dprint plugin
```

Update the local check path so `mise run fix` and `mise run test` keep the Wasm
target honest. A plain target check is not enough if the exported plugin code is
behind a `wasm` feature:

```bash
cargo check --target wasm32-unknown-unknown --package jolt_fmt_dprint
```

The check must exercise the same feature set used for the release `.wasm`.

Release packaging, plugin hosting, checksums, `dprint config update` metadata,
and npm distribution are outside milestone 11. The milestone only needs a local
`.wasm` that dprint can load from a file path.

## Tests

Add owned tests in `jolt_fmt_dprint`; do not hide dprint behavior inside the CLI
test crate.

Configuration tests:

- plugin config uses Jolt defaults when neither global nor plugin values exist,
- dprint global config maps to `FormatOptions`,
- `jolt` plugin config overrides global config,
- `lineWidth`, `indentWidth`, and `useTabs` use camelCase dprint names,
- the resolved plugin config type is `jolt_fmt_core::FormatOptions`,
- unknown properties produce dprint configuration diagnostics,
- zero and out-of-range numeric values produce configuration diagnostics.

Handler tests:

- `.java` requests call the Java formatter and return changed bytes,
- already-formatted Java returns no change,
- parse errors return a dprint error and no formatted bytes,
- unsupported associated paths return an error and no formatted bytes,
- invalid UTF-8 returns an error and no formatted bytes.

dprint smoke tests:

- build the local Wasm plugin,
- create a temporary `dprint.json` that references the local `.wasm`,
- run `dprint fmt` on an unformatted Java file and assert it changes,
- run `dprint check` on an unformatted Java file and assert it fails,
- verify `lineWidth`, `indentWidth`, and `useTabs` influence output through
  dprint configuration.

The smoke tests should use tiny owned Java fixtures plus at least one committed
fixture-corpus input. They must fail when required fixture files, the dprint
binary, the Wasm target, or the built plugin are missing.

## Acceptance Criteria

Milestone 11 is complete when:

- `jolt_fmt_dprint` exports a dprint Wasm plugin that loads in dprint,
- the plugin registers Java files and formats them through `jolt_fmt_core`,
- dprint config maps exactly to the shared `FormatOptions`,
- parse and formatter failures produce no output and visible diagnostics,
- local checks compile the real Wasm plugin code path,
- dprint smoke tests prove local file-path plugin loading and config mapping,
- no dprint-specific formatting behavior exists outside the wrapper.

## References

- dprint Wasm plugin development:
  https://github.com/dprint/dprint/blob/main/docs/wasm-plugin-development.md
- dprint configuration reference: https://dprint.dev/config/
- dprint-core Rust plugin APIs:
  https://docs.rs/dprint-core/latest/dprint_core/plugins/
- dprint JSON plugin Rust wrapper example:
  https://github.com/dprint/dprint-plugin-json
