# Installation

This page covers the standalone `jolt` CLI. To use Jolt through
[dprint](https://dprint.dev) instead, see [Integrations](./integrations).

## Releases

CLI binaries are built in CI and published to
[GitHub releases](https://github.com/sargunv/jolt/releases). Each release ships
static binaries for macOS, Linux (musl), and Windows, along with shell
installers and checksum files.

Release artifacts are signed with
[GitHub artifact attestations](https://docs.github.com/en/actions/security-for-github-actions/using-artifact-attestations/using-artifact-attestations-to-establish-provenance-for-builds),
linking each download to the repository's release workflow. To verify a download
with the GitHub CLI:

```sh
gh attestation verify PATH/TO/ARTIFACT -R sargunv/jolt
```

## Install

Pick an install method:

### Shell installer

macOS and Linux:

```sh
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/sargunv/jolt/releases/latest/download/jolt_cli-installer.sh | sh
```

Windows:

```powershell
powershell -ExecutionPolicy Bypass -c "irm https://github.com/sargunv/jolt/releases/latest/download/jolt_cli-installer.ps1 | iex"
```

### eget

[eget](https://github.com/zyedidia/eget) downloads executables from GitHub
releases.

```sh
eget sargunv/jolt
```

### mise

[mise](https://mise.jdx.dev) installs and pins dev tools per project.

```sh
mise use github:sargunv/jolt
```

### Manual download

Download a binary or archive for your platform from
[GitHub releases](https://github.com/sargunv/jolt/releases), verify it if you
want, then put `jolt` on your `PATH`.

## Verify

```sh
jolt --version
```

Format Java files from the project root:

```sh
jolt fmt .
```

Check formatting without writing changes:

```sh
jolt fmt --check .
```
