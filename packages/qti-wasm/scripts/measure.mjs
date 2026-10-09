import { readFile } from "node:fs/promises";
import { gzipSync } from "node:zlib";

const files = ["generated/qti_wasm_bg.wasm", "generated/qti_wasm.js", "src/index.js"];
const sizes = [];
for (const name of files) {
  const bytes = await readFile(new URL(`../dist/${name}`, import.meta.url));
  sizes.push({ name, rawBytes: bytes.byteLength, gzipBytes: gzipSync(bytes).byteLength });
}
console.log(JSON.stringify({ node: process.version, platform: process.platform, arch: process.arch, sizes }, null, 2));
