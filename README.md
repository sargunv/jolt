# Jolt

Jolt is a simple, portable, and extremely fast formatter for Java source, with a
predictable and opinionated style similar to other modern formatters.

## Performance

Formatting the
[Spring Framework](https://github.com/spring-projects/spring-framework) Java
sources (~9,200 files) on a Ryzen AI Max+ 395. Lower is better.

| Formatter          | Time |
| ------------------ | ---: |
| jolt (native)      | 0.4s |
| jolt (dprint)      | 0.5s |
| google-java-format |  11s |
| prettier-java      |  28s |

See the [benchmark script](./tools/bench/) for how these numbers are produced.

Jolt formats the whole repo in under half a second—about 20× faster than
`google-java-format`.

## Install

Jolt ships prebuilt static binaries for macOS, Linux, and Windows on the
[releases page](https://github.com/sargunv/jolt/releases).

Install automatically with a release-downloading tool:

```sh
eget sargunv/jolt              # https://github.com/zyedidia/eget
mise use github:sargunv/jolt   # https://mise.jdx.dev
```

Or with a shell script:

```sh
# macOS / Linux
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/sargunv/jolt/releases/latest/download/jolt_cli-installer.sh | sh
```

```powershell
# Windows
powershell -ExecutionPolicy Bypass -c "irm https://github.com/sargunv/jolt/releases/latest/download/jolt_cli-installer.ps1 | iex"
```

Or as a [dprint](https://dprint.dev) WASM plugin:

```sh
dprint add sargunv/jolt
```

## Usage

```sh
jolt fmt              # format Java files in the project
jolt fmt --check      # exit non-zero if files aren't formatted (for CI)
```

`jolt fmt` discovers `.java` files under the working directory. Pass `-` as a
path to format from stdin with optional `--stdin-filename` for future language
detection)—handy for editor format-on-save.

Shell completions and a manpage are also available:

```sh
jolt completions zsh    # bash, zsh, fish, ...
jolt manpage | man -l -
```

## Configuration

Jolt is opinionated: the only formatting options are line width, indent width,
and tabs vs. spaces. Configure them in `jolt.toml`:

```toml
root = true

[format]
line-width = 80
indent-width = 2
tabs = false
include = ["**/*.java"]
exclude = ["generated/**"]
```

Jolt discovers config by walking from the project root down to each file's
directory, reading the first of:

- `jolt.toml`
- `.config/jolt.toml`
- `.config/jolt/config.toml`

The project root is the nearest ancestor with a VCS marker (`.git`, `.hg`,
`.jj`, `.svn`) or a config file containing `root = true`.

For the dprint plugin, the same options go under the `jolt` key in
`dprint.jsonc` using dprint-style names: `lineWidth`, `indentWidth`, and
`useTabs`.
