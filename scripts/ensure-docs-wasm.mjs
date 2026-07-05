import { access, copyFile, mkdir } from "node:fs/promises";
import { createWriteStream } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { pipeline } from "node:stream/promises";

const root = join(fileURLToPath(import.meta.url), "..", "..");
const dest = join(root, "docs/public/jolt-plugin.wasm");

const localCandidates = [
  join(root, "plugin.wasm"),
  join(
    root,
    "target/wasm32-unknown-unknown/release/jolt_fmt_dprint.wasm",
  ),
  join(root, "target/wasm32-unknown-unknown/debug/jolt_fmt_dprint.wasm"),
];

async function exists(path) {
  try {
    await access(path);
    return true;
  } catch {
    return false;
  }
}

async function copyLocal(source) {
  await mkdir(dirname(dest), { recursive: true });
  await copyFile(source, dest);
  console.log(`Copied ${source} -> ${dest}`);
}

async function downloadFromRelease() {
  const releaseResponse = await fetch(
    "https://api.github.com/repos/sargunv/jolt/releases/latest",
    { headers: { "User-Agent": "jolt-docs" } },
  );
  if (!releaseResponse.ok) {
    throw new Error(`GitHub API returned ${releaseResponse.status}`);
  }

  const release = await releaseResponse.json();
  const asset = release.assets?.find((entry) => entry.name === "plugin.wasm");
  if (!asset) {
    throw new Error("plugin.wasm not found in latest GitHub release");
  }

  const wasmResponse = await fetch(asset.browser_download_url, {
    headers: { "User-Agent": "jolt-docs" },
  });
  if (!wasmResponse.ok || !wasmResponse.body) {
    throw new Error(`Failed to download plugin.wasm (${wasmResponse.status})`);
  }

  await mkdir(dirname(dest), { recursive: true });
  await pipeline(wasmResponse.body, createWriteStream(dest));
  console.log(`Downloaded ${asset.browser_download_url} -> ${dest}`);
}

for (const candidate of localCandidates) {
  if (await exists(candidate)) {
    await copyLocal(candidate);
    process.exit(0);
  }
}

if (await exists(dest)) {
  console.log(`Using existing ${dest}`);
  process.exit(0);
}

await downloadFromRelease();
