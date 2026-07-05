# Configuration

Jolt's configuration surface is intentionally small. Formatting options and file
discovery options are separate.

## File locations

### CLI

Jolt discovers config by walking from the effective project root down to each
file's directory, layering any config files found at these locations:

- `jolt.toml`
- `.config/jolt.toml`
- `.config/jolt/config.toml`

The effective project root for each file is the nearest ancestor with a VCS
marker (`.git`, `.hg`, `.jj`, `.svn`) or a config file containing `root = true`.
That means a nested `root = true` config stops inheritance from parent
directories.

The [dprint integration](./integrations) reads configuration from `dprint.jsonc`
rather than a Jolt config file.

## Example

Create `jolt.toml` at the project root:

```sh
jolt init
```

This writes a `#:schema` directive for TOML language servers that support JSON
Schema association, such as Taplo and Tombi.

The generated file is equivalent to:

```text
#:schema https://github.com/sargunv/jolt/releases/download/<version>/jolt-schema.json
root = true

[format]
line-width = 80
indent-width = 2
use-tabs = false

[files]
include = ["**/*.java"]
exclude = ["generated/**"]
```

## Fields

- `root`: marks this config as the project root when set to `true`.
- `format.line-width`: preferred maximum rendered line width.
- `format.indent-width`: number of spaces per indentation level when using
  spaces.
- `format.use-tabs`: use tabs for indentation when set to `true`.
- `files.include`: source file globs to include.
- `files.exclude`: source file globs to exclude. `.gitignore` and `.ignore`
  files are always respected as well.

For dprint, put formatter options under the `jolt` key in `dprint.jsonc` using
dprint-style camelCase names: `lineWidth`, `indentWidth`, and `useTabs`.

## CLI overrides

The `jolt fmt` command can override formatting settings for one run:

```sh
jolt fmt --line-width 100 --indent-width 4 --use-tabs false .
```

Use `--config <path>` to load only an explicit config file. Use `--no-config` to
disable discovered project configs and rely on defaults plus CLI options.

## Schemas

Print the JSON schema for `jolt.toml`:

```sh
jolt config schema
```

Print the dprint plugin schema:

```sh
jolt config schema --dprint
```
