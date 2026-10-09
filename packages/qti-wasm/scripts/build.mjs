import { spawnSync } from "node:child_process";
import { cp, mkdir, rm } from "node:fs/promises";
import { fileURLToPath } from "node:url";

const packageRoot = fileURLToPath(new URL("../", import.meta.url));
const crateRoot = fileURLToPath(new URL("../../../crates/qti-wasm", import.meta.url));
const generatedRoot = fileURLToPath(new URL("../generated", import.meta.url));

function run(command, args) {
  const result = spawnSync(command, args, { cwd: packageRoot, stdio: "inherit" });
  if (result.error) throw result.error;
  if (result.status !== 0) process.exit(result.status ?? 1);
}

run("wasm-pack", ["build", "--target", "web", "--release", "--out-dir", generatedRoot, crateRoot]);
await rm(new URL("../dist/", import.meta.url), { recursive: true, force: true });
run("tsc", ["-p", "tsconfig.build.json"]);
run("tsc", ["-p", "tsconfig.worker.json"]);
await mkdir(new URL("../dist/generated", import.meta.url), { recursive: true });
await cp(new URL("../generated/", import.meta.url), new URL("../dist/generated/", import.meta.url), {
  recursive: true,
  // wasm-pack's generated ignore file belongs to the source checkout, not the npm artifact.
  filter: (source) => !source.endsWith("/.gitignore"),
});
await cp(new URL("../examples/browser/index.html", import.meta.url), new URL("../dist/examples/browser/index.html", import.meta.url));
await mkdir(new URL("../dist/examples/browser/vendor/", import.meta.url), { recursive: true });
await cp(new URL("../node_modules/fflate/esm/browser.js", import.meta.url), new URL("../dist/examples/browser/vendor/fflate.js", import.meta.url));
await cp(new URL("../../../LICENSE.LGPL-3.0", import.meta.url), new URL("../dist/LICENSE.LGPL-3.0", import.meta.url));
