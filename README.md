# Jolt

Jolt is a simple, portable, and extremely fast formatter for Java and Kotlin
source, with a predictable and opinionated style.

Jolt runs as a standalone native CLI or as a [dprint](https://dprint.dev)
WebAssembly plugin. It has a small configuration surface and is designed for
editor, CI, and large-repository formatting.

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
jolt config init
jolt format
jolt format --check
```

## Docs

- [Documentation site](https://sargunv.github.io/jolt/)
- [Installation guide](https://sargunv.github.io/jolt/guides/installation)
- [Configuration guide](https://sargunv.github.io/jolt/guides/configuration)
- [CLI reference](https://sargunv.github.io/jolt/reference/cli)
