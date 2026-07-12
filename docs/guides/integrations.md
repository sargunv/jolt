# Integrations

Jolt can run as a native CLI or as a dprint Wasm plugin. Pick the integration
mode that fits the place you want formatting to happen.

For direct command-line usage, see [Installation](./installation). This page
covers integration into other tooling like dprint, editors, and pre-commit
hooks.

## dprint

You can use dprint's plugin and editor ecosystem to host Jolt.

Add Jolt to a dprint project:

```sh
dprint add sargunv/jolt
```

Run dprint normally:

```sh
dprint fmt
dprint check
```

The Jolt options go under the `jolt` key using dprint-style names. See
[Configuration](./configuration) for what each option does.

## Editors

Jolt does not ship native editor plugins yet. For editor formatting today,
configure Jolt through dprint, then use dprint's editor support.

### VS Code

Install the dprint extension and use settings like:

```json
{
  "editor.formatOnSave": true,
  "editor.defaultFormatter": "dprint.dprint",
  "json.schemaDownload.trustedDomains": ["https://dprint.dev"]
}
```

### Zed

Zed supports external formatters per language. Configure jolt as the formatter
for Java and Kotlin in `.zed/settings.json` or your user `settings.json`:

```json
{
  "languages": {
    "Java": {
      "formatter": {
        "external": {
          "command": "jolt",
          "arguments": ["fmt", "-", "--stdin-filename", "{buffer_path}"]
        }
      },
      "format_on_save": "on"
    },
    "Kotlin": {
      "formatter": {
        "external": {
          "command": "jolt",
          "arguments": ["fmt", "-", "--stdin-filename", "{buffer_path}"]
        }
      },
      "format_on_save": "on"
    }
  }
}
```

Or configure both languages with dprint:

```json
{
  "languages": {
    "Java": {
      "formatter": {
        "external": {
          "command": "dprint",
          "arguments": ["fmt", "--stdin", "{buffer_path}"]
        }
      },
      "format_on_save": "on"
    },
    "Kotlin": {
      "formatter": {
        "external": {
          "command": "dprint",
          "arguments": ["fmt", "--stdin", "{buffer_path}"]
        }
      },
      "format_on_save": "on"
    }
  }
}
```

### IntelliJ IDEA

Install the
[dprint JetBrains plugin](https://plugins.jetbrains.com/plugin/18192-dprint),
then:

1. Open `Settings` or `Preferences` > `Tools` > `dprint`.
2. Enable dprint.
3. Enable `Run dprint on save` for format-on-save.
4. Enable `Default formatter override` to route IntelliJ's reformat action
   through dprint when the file is supported.

## Pre-commit Hooks

### hk

[hk](https://github.com/jdx/hk) can define separate check and fix commands.

```text
amends "package://github.com/jdx/hk/releases/download/v1.48.0/hk@1.48.0#/Config.pkl"

local lintSteps = new Mapping<String, Step> {
  ["jolt"] {
    glob = List("**/*.java", "**/*.kt", "**/*.kts", "jolt.toml")
    check = "jolt fmt --check ."
    fix = "jolt fmt ."
  }
}

hooks {
  ["pre-commit"] {
    fix = true
    stash = "git"
    steps = lintSteps
  }
  ["check"] {
    steps = lintSteps
  }
}
```

Install the hook once:

```sh
hk install
```

Then use hk directly when you want the same behavior without committing:

```sh
hk fix
hk check
```

### pre-commit

With the [pre-commit](https://pre-commit.com) framework, define a local hook in
`.pre-commit-config.yaml`.

Native CLI:

```yaml
repos:
  - repo: local
    hooks:
      - id: jolt
        name: jolt
        entry: jolt fmt
        language: unsupported
        files: \.(java|kt|kts)$
```

Install the hook once:

```sh
pre-commit install
```

Run it manually across the repository:

```sh
pre-commit run --all-files
```
