import { mkdirSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const referenceDir = join(root, "docs/reference");
const manDir = join(root, "docs/public/man/man1");
const schemaDir = join(root, "docs/public/schemas");

rmSync(referenceDir, { recursive: true, force: true });
rmSync(manDir, { recursive: true, force: true });
mkdirSync(referenceDir, { recursive: true });
mkdirSync(manDir, { recursive: true });
mkdirSync(schemaDir, { recursive: true });

writeText(
  join(referenceDir, "cli.md"),
  run("cargo", [
    "run",
    "--quiet",
    "--package",
    "jolt_cli",
    "--features",
    "docs-generation",
    "--",
    "__docs",
    "cli-reference",
  ]),
);

run("cargo", [
  "run",
  "--quiet",
  "--package",
  "jolt_cli",
  "--features",
  "docs-generation",
  "--",
  "__docs",
  "manpages",
  manDir,
]);

const joltSchemaPath = join(schemaDir, "jolt-schema.json");
const dprintSchemaPath = join(schemaDir, "dprint-schema.json");

writeText(
  joltSchemaPath,
  run("cargo", ["run", "--quiet", "--package", "jolt_cli", "--", "config", "schema"]),
);
writeText(
  dprintSchemaPath,
  run("cargo", [
    "run",
    "--quiet",
    "--package",
    "jolt_cli",
    "--",
    "config",
    "schema",
    "--dprint",
  ]),
);

function writeText(path, text) {
  writeFileSync(path, text.endsWith("\n") ? text : `${text}\n`);
}

function run(command, args) {
  const result = spawnSync(command, args, {
    cwd: root,
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  });

  if (result.status !== 0) {
    process.stderr.write(result.stdout);
    process.stderr.write(result.stderr);
    throw new Error(`${command} ${args.join(" ")} failed`);
  }

  if (result.stderr) {
    process.stderr.write(result.stderr);
  }

  return result.stdout;
}
