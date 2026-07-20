import { access, copyFile, mkdir, rm } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(fileURLToPath(import.meta.url), "..", "..");
const dest = join(root, "docs/.vitepress/theme/assets/jolt-plugin.wasm");
const oldDest = join(root, "docs/public/jolt-plugin.wasm");

const localCandidates = [
  join(root, "target/wasm32-unknown-unknown/release/jolt_fmt_dprint.wasm"),
  join(root, "target/wasm32-unknown-unknown/debug/jolt_fmt_dprint.wasm"),
  join(root, "plugin.wasm"),
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

await rm(oldDest, { force: true });

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

throw new Error(
  "No local Jolt WASM artifact found. Run `mise run build:dprint-plugin` before building the docs.",
);
