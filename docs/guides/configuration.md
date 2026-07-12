# Configuration

Jolt's configuration surface is intentionally small. Use `jolt.toml` for the
native CLI and `dprint.jsonc` when running Jolt as a dprint plugin.

## Create a Config

Create `jolt.toml` at the project root:

```sh
jolt config init
```

This writes a root config with a `#:schema` directive for TOML language servers
that support JSON Schema association, such as Taplo and Tombi.

## Discovery

Jolt discovers config by walking from the effective project root down to each
file's directory, layering any config files found at these locations:

- `jolt.toml`
- `.config/jolt.toml`
- `.config/jolt/config.toml`

The effective project root for each file is the nearest ancestor with a VCS
marker (`.git`, `.hg`, `.jj`, `.svn`) or a config file containing `root = true`.
That means a nested `root = true` config stops inheritance from parent
directories.

## Example

```text
#:schema https://github.com/sargunv/jolt/releases/download/<version>/jolt-schema.json
root = true

[format]
line-width = 80
indent-width = 2
use-tabs = false

[files]
include = ["**/*.java", "**/*.kt", "**/*.kts"]
exclude = ["generated/**"]
```

For dprint, put formatter options under the `jolt` key in `dprint.jsonc`. dprint
uses camelCase names such as `lineWidth`, `indentWidth`, and `useTabs`.

## Fields

- `root`: marks this config as the project root when set to `true`.
- `format.line-width`: preferred maximum rendered line width.
- `format.indent-width`: number of spaces per indentation level when using
  spaces.
- `format.use-tabs`: use tabs for indentation when set to `true`.
- `files.include`: source file globs to include.
- `files.exclude`: source file globs to exclude. `.gitignore` and `.ignore`
  files are always respected as well.

## CLI overrides

The `jolt fmt` command can override formatting settings for one run:

```sh
jolt fmt --line-width 100 --indent-width 4 --use-tabs false .
```

Use `--config <path>` to load only an explicit config file. Use `--no-config` to
disable discovered project configs and rely on defaults plus CLI options.

## Inspect Configs

List the config files that apply to the current directory or a path:

```sh
jolt config list [path]
```

Print the effective config for the current directory or a path:

```sh
jolt config resolve [path]
```

`resolve` prints TOML with comments showing which config supplied each value.
When the path is a file or a future `.java`, `.kt`, or `.kts` file, it also
reports whether that file is selected by the resolved include/exclude patterns.

## Schemas

The docs build publishes the raw [Jolt config schema](/schemas/jolt-schema.json)
and [dprint config schema](/schemas/dprint-schema.json).

Print the JSON schema for `jolt.toml` locally:

```sh
jolt config schema
```

Print the dprint plugin schema:

```sh
jolt config schema --dprint
```
