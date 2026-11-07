# Configuration

Jolt has three formatting options: line width, indent width, and tabs or spaces.
Set them in `jolt.toml` for the CLI, or in `dprint.json` when running Jolt as a
dprint plugin.

## Create a Config

Create `jolt.toml` in the project root:

```sh
jolt config init
```

It writes every option at its default, with a `#:schema` directive for TOML
language servers that support JSON Schema, such as Taplo and Tombi.

## Example

::: code-group

<!-- dprint-ignore -->
```toml [jolt.toml]
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

:::

## Fields

- `root`: marks this directory as the project root when `true`.
- `format.line-width`: preferred maximum line width. Default `80`.
- `format.indent-width`: spaces per indentation level when not using tabs.
  Default `2`.
- `format.use-tabs`: indent with tabs instead of spaces. Default `false`.
- `files.include`: globs of source files to format, relative to the config's
  directory. Default `["**/*.java"]`. A nearer config's list replaces a farther
  one's.
- `files.exclude`: globs of source files to skip, relative to the config's
  directory. Every config's list applies. `.gitignore` and `.ignore` files are
  always respected too.

The globs select files found in directories given to `jolt format`; files named
on the command line are always formatted.

## Discovery

A config file is any of these, in increasing precedence within one directory:

- `.config/jolt/config.toml`
- `.config/jolt.toml`
- `jolt.toml`

A file's settings come from every config between the project root and the file's
directory, nearer ones taking precedence. The project root is the nearest
ancestor with a VCS marker (`.git`, `.hg`, `.jj`, `.svn`) or a config with
`root = true`, so a nested `root = true` stops inheritance from the directories
above it. Without one, the project root is the directory `jolt` runs in when the
file is under it (so `jolt format .`, `jolt format src/A.java` and
`jolt format --stdin-filename src/A.java -` agree), and otherwise the file's own
directory.

## Command-line Overrides

Options given to `jolt format` override every config:

```sh
jolt format --line-width 100 --indent-width 4 --use-tabs false .
```

`--include` replaces the include globs and `--exclude` adds exclude globs, both
relative to the working directory. `--config <path>` loads only that file, and
`--no-config` loads none.

## Inspect Configs

List the config files that apply to a directory or file, lowest precedence
first:

```sh
jolt config list [path]
```

Print the effective config, and for a file path (existing or not) whether the
include and exclude globs select it:

```sh
jolt config resolve [path]
```

## dprint

Put options under the `jolt` key of `dprint.json`, in camelCase. dprint's own
`lineWidth`, `indentWidth`, and `useTabs` apply when the `jolt` key leaves them
unset.

```json
{ "lineWidth": 100, "jolt": { "indentWidth": 4 } }
```

## Schemas

The docs publish the [Jolt config schema](/schemas/jolt-schema.json) and the
[dprint config schema](/schemas/dprint-schema.json), and each release attaches
both. `jolt config schema` prints the first, and `jolt config schema --dprint`
the second.
