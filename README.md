# Jolt

Jolt is a fast, opinionated Java formatter. It runs as a native CLI with no JVM,
or as a [dprint](https://dprint.dev) WebAssembly plugin, and has three options:
line width, indent width, and tabs or spaces.

## Install

Download prebuilt binaries from the
[releases page](https://github.com/sargunv/jolt/releases), or install with your
favorite release-downloading tool:

```sh
eget sargunv/jolt
```

```sh
mise use github:sargunv/jolt
```

Or use Jolt through dprint:

```sh
dprint add sargunv/jolt
```

## Usage

```sh
jolt format            # format the Java files under the current directory
jolt format --check    # list the files that need formatting
jolt config init       # write a jolt.toml with the defaults
```

## Docs

- [Documentation site](https://sargunv.github.io/jolt/)
- [Installation guide](https://sargunv.github.io/jolt/guides/installation)
- [Configuration guide](https://sargunv.github.io/jolt/guides/configuration)
- [CLI reference](https://sargunv.github.io/jolt/reference/cli)
- [Java style](https://sargunv.github.io/jolt/internals/java-style)

## Development

See [AGENTS.md](./AGENTS.md) for the project map, commands, and invariants.
