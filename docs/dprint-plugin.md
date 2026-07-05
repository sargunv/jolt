# dprint Plugin

Jolt ships as a dprint Wasm plugin for projects that already use
[dprint](https://dprint.dev).

## Install

Add the plugin to a dprint project:

```sh
dprint add sargunv/jolt
```

## Format

Run dprint normally:

```sh
dprint fmt
```

## Configure

The dprint plugin uses the same formatting options as the CLI under the `jolt`
key in `dprint.jsonc`. See [Configuration](./configuration) for what each option
does.

```jsonc
{
  "plugins": ["https://plugins.dprint.dev/sargunv/jolt/latest.wasm"],
  "jolt": { "lineWidth": 80, "indentWidth": 2, "tabs": false }
}
```
