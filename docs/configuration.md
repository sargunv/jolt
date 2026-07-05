# Configuration

Jolt's configuration surface is intentionally small. The supported formatting
options cover indent style and file discovery.

## File locations

### CLI

Jolt discovers config by walking from the project root down to each file's
directory, reading the first matching file:

- `jolt.toml`
- `.config/jolt.toml`
- `.config/jolt/config.toml`

The project root is the nearest ancestor with a VCS marker (`.git`, `.hg`,
`.jj`, `.svn`) or a config file containing `root = true`.

The [dprint integration](./integrations) reads configuration from `dprint.jsonc`
rather than a Jolt config file.

## Example

Create `jolt.toml` at the project root:

```toml
root = true

[format]
line-width = 80
indent-width = 2
tabs = false
include = ["**/*.java"]
exclude = ["generated/**"]
```

## Fields

- `root`: marks this config as the project root when set to `true`.
- `format.line-width`: preferred maximum rendered line width.
- `format.indent-width`: number of spaces per indentation level when using
  spaces.
- `format.tabs`: use tabs for indentation when set to `true`.
- `format.include`: source file globs to include.
- `format.exclude`: source file globs to exclude. `.gitignore` and `.ignore`
  files are always respected as well.

For dprint, put formatter options under the `jolt` key in `dprint.jsonc` using
dprint-style camelCase names: `lineWidth`, `indentWidth`, and `useTabs`.

## CLI overrides

The `jolt fmt` command can override formatting settings for one run:

```sh
jolt fmt --line-width 100 --indent-width 4 .
```

Use `--config <path>` to load an explicit config file after discovered project
configs. Use `--no-config` to disable discovered project configs.
