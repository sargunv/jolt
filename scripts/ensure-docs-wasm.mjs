// Copies the dprint plugin that `mise run build:dprint-plugin --mode <mode>`
// built to where the playground imports it.
import { copyFileSync, existsSync, mkdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const dest = join(root, "docs/.vitepress/theme/assets/jolt-plugin.wasm");
const sources = {
  debug: "target/wasm32-unknown-unknown/debug/jolt_fmt_dprint.wasm",
  release: "target/plugin.wasm",
};

const mode = process.argv[2];
const source = sources[mode];
if (!source) {
  throw new Error("Usage: node scripts/ensure-docs-wasm.mjs <debug|release>");
}
if (!existsSync(join(root, source))) {
  throw new Error(
    `Missing ${source}. Run \`mise run build:dprint-plugin --mode ${mode}\` first.`,
  );
}
mkdirSync(dirname(dest), { recursive: true });
copyFileSync(join(root, source), dest);
console.log(`Copied ${source} -> ${dest}`);
