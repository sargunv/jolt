// Generates the CLI reference page and the config schemas from the CLI.
import { spawnSync } from "node:child_process";
import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const outputs = {
  "docs/reference/cli.md": ["__cli-reference"],
  "docs/public/schemas/jolt-schema.json": ["config", "schema"],
  "docs/public/schemas/dprint-schema.json": ["config", "schema", "--dprint"],
};

for (const [path, args] of Object.entries(outputs)) {
  const result = spawnSync(
    "cargo",
    ["run", "--quiet", "--package", "jolt_cli", "--features", "docs", "--", ...args],
    { cwd: root, encoding: "utf8", stdio: ["ignore", "pipe", "inherit"] },
  );
  if (result.status !== 0) {
    throw new Error(`generating ${path} failed`);
  }
  mkdirSync(dirname(join(root, path)), { recursive: true });
  writeFileSync(join(root, path), result.stdout);
}
