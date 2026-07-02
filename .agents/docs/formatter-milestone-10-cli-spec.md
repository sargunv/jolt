# Formatter Milestone 10 CLI Spec

## Purpose

Milestone 10 turns the Java formatter engine into a usable native command-line
tool.

The CLI should remain a thin wrapper around `jolt_fmt_core`. It owns user
interaction, config loading, file discovery, ignore handling, filesystem I/O,
terminal output, and process exit codes. It must not implement formatting
behavior or call language-specific formatter crates directly.

```text
paths + config + CLI flags
  -> resolved files and effective per-file options
  -> jolt_fmt_core::format_source
  -> write/check/stdout behavior
```

## Scope

Add:

- `jolt fmt`,
- stdin/stdout formatting,
- default write mode for filesystem inputs,
- `--check`,
- format option flags,
- include/exclude selection,
- config loading with Figment,
- `jolt.toml` and `.config/jolt.toml` discovery,
- ignore-aware walking with the `ignore` crate,
- owned CLI tests.

Do not add:

- Kotlin formatting behavior,
- project model integration,
- Gradle or Maven awareness,
- dprint behavior,
- semantic import cleanup,
- arbitrary formatter style options,
- user-global configuration unless a later milestone explicitly scopes it.

## CLI Shape

Primary command:

```bash
jolt fmt [paths...]
jolt fmt --check [paths...]
jolt fmt --line-width 100 [paths...]
jolt fmt --indent-width 4 [paths...]
jolt fmt --tabs [paths...]
jolt fmt --spaces [paths...]
jolt fmt --include 'src/**/*.java' --exclude 'src/generated/**'
jolt fmt --config path/to/jolt.toml
jolt fmt --no-config
jolt fmt -
```

For Milestone 10, default discovery includes only Java:

```text
**/*.java
```

When Kotlin lands, the default include set should become:

```text
**/*.{java,kt,kts}
```

### Positional Paths

If no path is provided, `jolt fmt` formats files under the current working
directory.

If a path is `-`, the CLI reads stdin and writes the formatted result to stdout.
Stdin mode does not write files and does not perform recursive discovery.

If one or more filesystem paths are provided, each path is either:

- a file to format directly, if it is a supported source file,
- or a directory root to walk recursively.

Unsupported file extensions should be ignored during recursive discovery. If a
user passes an unsupported file path explicitly, report a user-facing diagnostic
and fail.

## Mode

Filesystem inputs use write mode by default:

```bash
jolt fmt src
```

This rewrites changed files in place.

Check mode is selected explicitly:

```bash
jolt fmt --check src
```

This formats in memory, reports files that would change, does not write files,
and exits nonzero if any file would change.

`--write` is not required in Milestone 10. If added later, it should be an
explicit spelling of the default filesystem mode and should conflict with
`--check`.

Stdin mode always writes formatted output to stdout:

```bash
jolt fmt -
```

`--check` with stdin should format in memory and exit nonzero if stdin is not
already formatted. It may print no formatted source in check mode.

## Public Options

The CLI exposes the same formatter options as `jolt_fmt_core::FormatOptions`:

```text
--line-width <n>
--indent-width <n>
--tabs
--spaces
```

Use sparse CLI fields internally:

```rust
struct CliFormatOptions {
    line_width: Option<u16>,
    indent_width: Option<u8>,
    tabs: Option<bool>,
}
```

Do not put defaults on these clap fields. The merge layer must know whether the
user actually supplied a value.

`--tabs` sets `tabs = true`. `--spaces` sets `tabs = false`. They conflict with
each other.

Validate option ranges at the CLI/config boundary before calling
`jolt_fmt_core`:

- `line-width` must be greater than zero,
- `indent-width` must be greater than zero,
- upper bounds may be added if renderer behavior needs them.

## Config Files

Milestone 10 uses TOML config loaded through Figment.

The schema is scoped for future Jolt features. Formatter settings live under
`[format]`, not at the top level:

```toml
[format]
line-width = 80
indent-width = 2
tabs = false
include = ["**/*.java"]
exclude = ["generated/**"]

[format.java]
# Reserved for future Java-specific formatting options if they prove necessary.
```

The `[format.java]` table is reserved but should have no Milestone 10 options.
Do not add language-specific options unless a real formatting rule requires one.

Unknown config keys should be errors. Silent typos in formatter config make
rollout harder and can hide user mistakes.

### Config Discovery

Support both project-root and dot-config locations:

```text
jolt.toml
.config/jolt.toml
```

This follows the dot-config convention of storing project-specific tool
configuration under `.config/` while still allowing a root `jolt.toml` as the
manifest-friendly spelling.

For each invocation root, discover base project config by walking upward from
that root until the filesystem root. For each source file under that invocation
root, discover additional nested config files between the invocation root and
the file's parent directory. Merge shallower configs before deeper configs so
nested configs override parent configs.

An invocation root is:

- the current working directory, when no path is provided,
- the directory itself, when a directory path is provided,
- the file's parent directory, when a file path is provided.

For example:

```text
repo/jolt.toml
repo/module-a/.config/jolt.toml
repo/module-a/src/A.java
```

`A.java` receives options from `repo/jolt.toml`, then
`repo/module-a/.config/jolt.toml`.

If both `dir/jolt.toml` and `dir/.config/jolt.toml` exist in the same directory,
merge `dir/jolt.toml` first and `dir/.config/jolt.toml` second.

Config-relative path patterns, such as `include` and `exclude`, are interpreted
relative to the directory that contains the config file. CLI path patterns are
interpreted relative to the current working directory.

`--config <path>` loads exactly that config file after discovered project
configs. It should error if the file does not exist.

`--no-config` disables discovered project configs and conflicts with `--config`.

### Config Merge Order

For each formatted file, resolve the effective config in this order:

```text
built-in defaults
< discovered project configs, shallow to deep
< explicit --config
< CLI flags
```

Built-in defaults are:

```text
line-width   = 80
indent-width = 2
tabs         = false
include      = ["**/*.java"]
exclude      = []
```

Internally, deserialize config into sparse structs with `Option` fields and
merge into a resolved struct only after all sources have been applied. This
keeps precedence explicit and avoids confusing "default came from parser" with
"default came from Jolt".

## File Discovery

Use the `ignore` crate for recursive walking. It already implements the expected
behavior for:

- `.gitignore`,
- `.ignore`,
- git exclude files,
- global gitignore files,
- nested ignore files,
- efficient recursive traversal.

The CLI should not reimplement ignore semantics.

Default source selection:

```text
final candidate files =
  user_includes.unwrap_or(["**/*.java"])
  - user_excludes
  - ignored files
```

`include` replacement and `exclude` stacking apply across config and CLI:

- The nearest provided `include` list replaces the default include set.
- A CLI `--include` list replaces all config include lists.
- Config `exclude` lists stack from parent to child.
- CLI `--exclude` values stack on top of config excludes.

Config discovery and file discovery are related but should remain separate
implementation concerns:

- use `ignore::WalkBuilder` to walk invocation roots and respect ignore files,
- apply Jolt include/exclude patterns as a formatter-specific candidate filter,
- use a cached ancestor-config resolver to compute effective config per file.

Cache config resolution by directory. Most files share parent directories, so a
simple memoized upward search is sufficient for Milestone 10.

## Language Detection

For Milestone 10:

```text
.java -> Language::Java
```

Other extensions are ignored during recursive walking. Explicit unsupported
files are errors.

Do not add parser version flags such as `--source` or `--release`.

## Output and Exit Codes

Exit codes:

```text
0  success, no changes needed in check mode
1  formatting failed, files would change in check mode, invalid input, or I/O error
2  invalid CLI usage, if clap exposes that distinction
```

For write mode:

- write changed files in place,
- leave unchanged files untouched,
- never write output for a file when `jolt_fmt_core` returns no formatted
  source,
- report diagnostics with file paths.

For check mode:

- do not write files,
- print each file that would change,
- return failure if any file would change,
- also return failure for formatting diagnostics that block output.

For stdin/stdout:

- read all stdin as UTF-8 text,
- infer Java unless a later CLI option introduces explicit language selection,
- write formatted output to stdout in normal mode,
- write diagnostics to stderr.

## Diagnostics

CLI diagnostics should include enough location context for users:

```text
path/to/File.java: formatter diagnostic message
path/to/jolt.toml: invalid config key `format.line-wdith`
```

When `jolt_fmt_core` provides ranges, the CLI may add line/column reporting
using `LineIndex`. Pretty terminal rendering can be deferred; stable, clear text
is enough for Milestone 10.

Config errors should preserve Figment provenance where possible.

## Tests

Add owned tests in `jolt_fmt_cli`.

Cover:

- `jolt fmt -` formats stdin to stdout,
- write mode rewrites changed Java files,
- write mode leaves unchanged Java files unchanged,
- `--check` returns success when files are already formatted,
- `--check` returns failure when files would change,
- parse errors do not write files,
- `--line-width`, `--indent-width`, `--tabs`, and `--spaces` reach
  `jolt_fmt_core`,
- config options are applied,
- CLI options override config options,
- nested configs override parent configs,
- `jolt.toml` and `.config/jolt.toml` are both discovered,
- `--config` errors on missing files,
- `--no-config` ignores discovered configs,
- `include` replacement behavior,
- `exclude` stacking behavior,
- `.gitignore` and `.ignore` are respected,
- explicit unsupported files fail,
- recursive unsupported files are ignored.

Use temporary directories and small Java samples. Do not depend on user-global
machine config. Do not silently skip tests when fixtures or temp setup are
missing.

## Implementation Notes

Recommended dependencies for `jolt_fmt_cli`:

- `clap` with derive support,
- `figment` with TOML support,
- `serde`,
- `ignore`,
- `camino` if path handling benefits from UTF-8 paths,
- `assert_cmd` and `assert_fs` or `tempfile` for CLI tests.

Keep the implementation split into small CLI-owned modules:

```text
args.rs       clap structs and command parsing
config.rs     Figment providers, discovery, sparse config structs
discover.rs   ignore walking and include/exclude handling
run.rs        command execution and mode behavior
```

The crate may keep a tiny `main.rs` that delegates to a testable `run` function.

## Completion Criteria

Milestone 10 is complete when:

- `jolt fmt` can format Java files through `jolt_fmt_core`,
- config and CLI precedence are covered by tests,
- ignore-aware recursive discovery is covered by tests,
- check/write/stdin behavior is covered by tests,
- the CLI no longer calls `jolt_java_fmt` directly,
- `mise run test` passes.
