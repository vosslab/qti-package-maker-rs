import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import test from "node:test";
import { unzipSync } from "fflate";
import { checkPackage, convert, formats, initialize } from "../dist/src/index.js";

// A mismatch blocks package acceptance: fix the shared algorithm or transport, then
// rebuild the delivered package and rerun this corpus; do not weaken the comparison.
const root = fileURLToPath(new URL("../../../", import.meta.url));
const encoder = new TextEncoder();
const decoder = new TextDecoder();
const kinds = new Map([
  ["MC", "MC\tWhich molecule carries energy?\tATP\tCorrect\tADP\tIncorrect\n"],
  ["MA", "MA\tSelect purines\tadenine\tCorrect\tguanine\tCorrect\tcytosine\tIncorrect\n"],
  ["MATCH", "MAT\tMatch bases\tadenine\tthymine\tguanine\tcytosine\n"],
  ["NUM", "NUM\tConcentration in mM\t2.5\t0.125\n"],
  ["FIB", "FIB\tEnergy molecule\tATP\tadenosine triphosphate\n"],
  ["MULTIFIB", "FIB_PLUS\tDNA contains [first] and [second]\tfirst\tadenine\tA\t\tsecond\tthymine\tT\n"],
  ["ORDER", "ORD\tOrder gene expression\ttranscription\tRNA processing\ttranslation\n"],
]);
const pixel = new Uint8Array(Buffer.from(
  "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAusB9Y9ZlS8AAAAASUVORK5CYII=",
  "base64",
));

await initialize(new Uint8Array(await readFile(new URL("../dist/generated/qti_wasm_bg.wasm", import.meta.url))));

function request(outputFormat, text, extra = {}) {
  return {
    inputFormat: "bbq_text_upload",
    outputFormat,
    input: { kind: "file", name: "source.txt", bytes: encoder.encode(text) },
    allowMixed: true,
    document: { title: "Genetics \u03b2", date: "2026-10-08" },
    shuffleSeed: 42,
    ...extra,
  };
}

function jsonBytes(_key, value) {
  return value instanceof Uint8Array ? Array.from(value) : value;
}

function native(operations) {
  const process = spawnSync("cargo", ["run", "--quiet", "--locked", "-p", "qti-wasm", "--example", "parity_oracle"], {
    cwd: root,
    input: operations.map((operation) => JSON.stringify(operation, jsonBytes)).join("\n") + "\n",
    encoding: "utf8",
    maxBuffer: 128 * 1024 * 1024,
    timeout: 240_000,
  });
  assert.ifError(process.error);
  assert.equal(process.status, 0, process.stderr);
  const results = process.stdout.trim().split("\n").map((line) => JSON.parse(line));
  assert.equal(results.length, operations.length, "one native response per request");
  return results;
}

function semantic(value) {
  if (Array.isArray(value)) return value.map(semantic);
  if (value instanceof Uint8Array) return Array.from(value);
  if (value === null || typeof value !== "object") return value;
  if ("name" in value && "bytes" in value) {
    const bytes = new Uint8Array(value.bytes);
    if (bytes[0] === 0x50 && bytes[1] === 0x4b && bytes[2] === 3 && bytes[3] === 4) {
      const entries = unzipSync(bytes);
      return { name: value.name, zipEntries: Object.keys(entries).sort().map((name) => ({ name, bytes: Array.from(entries[name]) })) };
    }
  }
  return Object.fromEntries(Object.entries(value).map(([key, entry]) => [key, semantic(entry)]));
}

function wasm(operation) {
  if (operation.operation === "formats") return formats();
  if (operation.operation === "checkPackage") return checkPackage(operation.input);
  return convert(operation.request);
}

function compare(operations, labels) {
  const expected = native(operations);
  return operations.map((operation, index) => {
    const actual = wasm(operation);
    assert.deepEqual(semantic(actual), semantic(expected[index]), labels[index]);
    return actual;
  });
}

function conversion(input) {
  return { operation: "convert", request: input };
}

function artifactFiles(artifact) {
  if (!artifact) return [];
  return artifact.kind === "file" ? [artifact.primary, ...artifact.companions] : artifact.entries;
}

test("native and delivered Wasm agree on every writer and all seven question kinds", () => {
  const [inventory] = compare([{ operation: "formats" }], ["shared format inventory"]);
  const writers = inventory.formats.filter((format) => format.canWrite);
  assert.equal(writers.length, 11);
  assert.equal(inventory.formats.filter((format) => format.canRead).length, 4);
  const operations = [];
  const labels = [];
  for (const writer of writers) {
    for (const [kind, line] of kinds) {
      operations.push(conversion(request(writer.name, line)));
      labels.push(`${writer.name}: ${kind}`);
    }
    const supported = [...kinds].filter(([kind]) => writer.supportedKinds.includes(kind));
    operations.push(conversion(request(writer.name, supported.map(([, line]) => line).join(""))));
    labels.push(`${writer.name}: mixed supported kinds`);
    operations.push(conversion(request(writer.name, "")));
    labels.push(`${writer.name}: empty baseline for unsupported kinds`);
  }
  const results = compare(operations, labels);
  let index = 0;
  for (const writer of writers) {
    const empty = results[index + kinds.size + 1];
    for (const [kind] of kinds) {
      const result = results[index++];
      if (writer.supportedKinds.includes(kind)) {
        assert.equal(result.status, "success", `${writer.name}: ${kind}`);
        assert.equal(result.itemCount, 1, `${writer.name}: ${kind}`);
        assert.ok(artifactFiles(result.artifact).some((file) => file.bytes.length > 0), `${writer.name}: supported ${kind}`);
      } else if (result.status === "error") {
        assert.equal(result.error.category, "unsupportedItemKind", `${writer.name}: rejected ${kind}`);
      } else {
        assert.equal(result.itemCount, 1, `${writer.name}: skipped ${kind}`);
        assert.deepEqual(semantic(result.artifact), semantic(empty.artifact), `${writer.name}: unsupported ${kind} emits no question`);
      }
    }
    assert.equal(results[index++].status, "success", `${writer.name}: mixed`);
    index++;
  }
});

test("all four readers and package integrity consume identical writer-produced inputs", () => {
  const readers = formats().formats.filter((format) => format.canRead);
  const sourceRequests = readers.map((reader) => conversion(request(reader.name,
    [...kinds].filter(([kind]) => reader.supportedKinds.includes(kind)).map(([, line]) => line).join(""))));
  const sources = native(sourceRequests);
  const operations = [];
  const labels = [];
  for (let index = 0; index < readers.length; index++) {
    const reader = readers[index];
    const source = sources[index];
    assert.equal(source.status, "success", reader.name);
    const artifact = source.artifact;
    assert.equal(artifact.kind, "file", reader.name);
    const input = {
      kind: "file", name: artifact.primary.name, bytes: new Uint8Array(artifact.primary.bytes),
      companions: artifact.companions.map((file) => ({ name: file.name, bytes: new Uint8Array(file.bytes) })),
    };
    operations.push(conversion(request("bbq_text_upload", "", { inputFormat: reader.name, input, outputName: "roundtrip.txt" })));
    labels.push(`${reader.name}: reader from identical native writer bytes`);
    if (reader.name === "blackboard_export_zip") {
      // ZIP directory records are containers, not logical file entries.
      const entries = Object.entries(unzipSync(input.bytes)).filter(([name]) => !name.endsWith("/"))
        .map(([name, bytes]) => ({ name, bytes }));
      operations.push(conversion(request("bbq_text_upload", "", { inputFormat: reader.name, input: { kind: "entries", name: "pool.zip", entries } })));
      labels.push("Blackboard extracted entries reader");
      operations.push({ operation: "checkPackage", input: { kind: "zip", bytes: input.bytes } });
      labels.push("Blackboard ZIP integrity");
      operations.push({ operation: "checkPackage", input: { kind: "entries", entries } });
      labels.push("Blackboard extracted integrity");
    }
  }
  const results = compare(operations, labels);
  for (let index = 0; index < operations.length; index++) {
    assert.equal(results[index].status, "success", labels[index]);
    if (operations[index].operation === "convert") assert.ok(results[index].itemCount > 0, labels[index]);
    else assert.deepEqual(results[index].report.errors, [], labels[index]);
  }
});

test("selftest progress identities distinguish variants and survive document and media changes", () => {
  const stem = '<img src="diagram.png" /> Calculate the concentration';
  const bank = `NUM\t${stem}\t2.5\t0.125\nNUM\t${stem}\t5\t0.125\n`;
  const input = (image) => ({
    kind: "file", name: "variants.txt", bytes: encoder.encode(bank),
    companions: [{ name: "diagram.png", bytes: image }],
  });
  const operations = [
    conversion(request("html_selftest", "", { input: input(pixel), shuffleSeed: 0 })),
    conversion(request("html_selftest", "", { input: input(pixel), shuffleSeed: 1 })),
    conversion(request("html_selftest", "", {
      input: input(new Uint8Array([...pixel, 1])), shuffleSeed: 2,
      document: { title: "Another document", date: "2026-10-09" },
    })),
  ];
  const results = compare(operations, ["variant A", "variant B", "variant A with new presentation"]);
  const html = results.map((result) => {
    assert.equal(result.status, "success");
    return decoder.decode(result.artifact.primary.bytes);
  });
  const crcs = html.map((page) => {
    const match = page.match(/id="question_html_([0-9a-f]{4}_[0-9a-f]{4})"/);
    assert.ok(match, "a complete question CRC is exposed to the host");
    return match[1];
  });
  assert.equal(crcs[0].split("_")[0], crcs[1].split("_")[0], "these variants share their stem");
  assert.notEqual(crcs[0], crcs[1], "completion keys distinguish the answer variants");
  assert.equal(crcs[0], crcs[2], "selection, metadata, and media bytes do not replace source identity");
  assert.notEqual(html[0], html[2], "the changed presentation was actually exported");
});

test("parity preserves media, grading, ordered diagnostics, limits, and state across repeated calls", () => {
  const operations = [];
  const labels = [];
  function add(label, input) {
    operations.push(conversion(input));
    labels.push(label);
  }
  const mc = kinds.get("MC");
  add("Unicode", request("bbq_text_upload", "MC\t\u03b2-oxidation \ud83e\uddec caf\u00e9\t\u0394G\tCorrect\t\u03b1\tIncorrect\n"));
  add("empty source", request("bbq_text_upload", ""));
  add("malformed UTF-8", request("bbq_text_upload", "", { input: { kind: "file", name: "bad.txt", bytes: new Uint8Array([255]) } }));
  add("ordered read and media warnings", request("bbq_text_upload", `\nINVALID\tbad\nMC\t<img src="https://example.test/one.png" /><img src="missing.png" /> Which?\tATP\tCorrect\tADP\tIncorrect\n`));
  add("missing packaged media", request("html_selftest", `MC\t<img src="missing.png" /> Which?\tATP\tCorrect\tADP\tIncorrect\n`));
  const otherPixel = new Uint8Array([...pixel, 1]);
  const mediaInput = {
    kind: "file", name: "media.txt",
    bytes: encoder.encode(`MC\t<img src="first/pixel.png" /><img src="second/pixel.png" /> Which?\tATP\tCorrect\tADP\tIncorrect\n`),
    companions: [{ name: "first/pixel.png", bytes: pixel }, { name: "second/pixel.png", bytes: otherPixel }],
  };
  for (const format of ["html_selftest", "text2qti", "blackboard_export_zip", "canvas_qti_v1_2", "blackboard_qti_v2_1", "ple_native_json"]) {
    add(`${format}: colliding media basenames`, request(format, "", { input: mediaInput }));
  }
  for (const name of ["../escape.txt", "/absolute.txt", "nested/../../escape.txt"]) {
    add(`unsafe input ${name}`, request("bbq_text_upload", "", { input: { kind: "file", name, bytes: encoder.encode(mc) } }));
    add(`unsafe companion ${name}`, request("html_selftest", mc, { input: { kind: "file", name: "safe.txt", bytes: encoder.encode(mc), companions: [{ name, bytes: pixel }] } }));
  }
  add("duplicate companion", request("html_selftest", mc, { input: { kind: "file", name: "safe.txt", bytes: encoder.encode(mc), companions: [{ name: "pixel.png", bytes: pixel }, { name: "pixel.png", bytes: pixel }] } }));
  add("conflicting companion", request("html_selftest", mc, { input: { kind: "file", name: "safe.txt", bytes: encoder.encode(mc), companions: [{ name: "pixel.png", bytes: pixel }, { name: "pixel.png", bytes: otherPixel }] } }));
  add("unknown output format", request("not-a-format", mc));
  add("unsupported reader direction", request("bbq_text_upload", mc, { inputFormat: "exam_yaml" }));
  add("malformed Blackboard archive", request("bbq_text_upload", "", { inputFormat: "blackboard_export_zip", input: { kind: "file", name: "broken.zip", bytes: new Uint8Array([0, 1, 2]) } }));
  add("graded answer resembles HTML", request("ple_native_json", "FIB\tLiteral markup answer\t<b>ATP</b>\n"));
  add("limit keeps first input item", request("bbq_text_upload", [...kinds.values()].join(""), { limit: 1, outputName: "limited.txt" }));
  add("zero limit", request("bbq_text_upload", mc, { limit: 0 }));
  add("mixed kinds disabled", request("bbq_text_upload", [...kinds.values()].join(""), { allowMixed: false }));
  add("representative 1 MiB source", request("bbq_text_upload", Array.from({ length: 512 }, (_, index) => `MC\tQuestion ${index} ${"x".repeat(2048)}\tATP\tCorrect\tADP\tIncorrect\n`).join("")));
  add("success after errors", request("bbq_text_upload", mc));
  add("repeat success", request("bbq_text_upload", mc));
  operations.push({ operation: "checkPackage", input: { kind: "zip", bytes: new Uint8Array([0, 1, 2]) } });
  labels.push("malformed ZIP integrity");
  operations.push({ operation: "checkPackage", input: { kind: "entries", entries: [{ name: "../escape.txt", bytes: new Uint8Array([1]) }] } });
  labels.push("unsafe integrity entry");
  operations.push({ operation: "checkPackage", input: { kind: "entries", entries: [{ name: "one.txt", bytes: pixel }, { name: "one.txt", bytes: pixel }] } });
  labels.push("duplicate package entry");
  const results = compare(operations, labels);
  const resultFor = (label) => results[labels.indexOf(label)];
  const unicode = resultFor("Unicode");
  assert.equal(unicode.itemCount, 0, "native CRC contract reports non-ASCII authored text");
  assert.ok(unicode.warnings[0].message.includes("\u03b2-oxidation \ud83e\uddec caf\u00e9"));
  assert.equal(resultFor("empty source").itemCount, 0);
  assert.equal(resultFor("malformed UTF-8").status, "error");
  const warnings = resultFor("ordered read and media warnings").warnings;
  assert.deepEqual(warnings.map((warning) => warning.stage), ["read", "read", "write", "write"]);
  assert.equal(resultFor("missing packaged media").status, "error");
  for (const label of labels.filter((label) => label.includes("colliding media basenames"))) {
    const result = resultFor(label);
    assert.equal(result.status, "success", label);
    const files = artifactFiles(result.artifact);
    if (label.startsWith("html_selftest:")) {
      const html = decoder.decode(files[0].bytes);
      assert.ok(html.includes(Buffer.from(pixel).toString("base64")), label);
      assert.ok(html.includes(Buffer.from(otherPixel).toString("base64")), label);
    } else {
      const payloads = files.flatMap((file) => file.bytes[0] === 0x50 && file.bytes[1] === 0x4b
        ? Object.values(unzipSync(file.bytes)) : [file.bytes]);
      assert.ok(payloads.some((bytes) => Buffer.from(bytes).equals(Buffer.from(pixel))), `${label}: first payload`);
      assert.ok(payloads.some((bytes) => Buffer.from(bytes).equals(Buffer.from(otherPixel))), `${label}: second payload`);
    }
  }
  for (const label of labels.filter((label) => label.startsWith("unsafe input"))) {
    assert.equal(resultFor(label).error.category, "invalidName", label);
  }
  for (const label of labels.filter((label) => label.startsWith("unsafe companion"))) {
    assert.equal(resultFor(label).error.category, "media", label);
  }
  assert.equal(resultFor("duplicate companion").status, "success");
  assert.equal(resultFor("conflicting companion").error.category, "media");
  assert.equal(resultFor("duplicate package entry").error.category, "duplicateEntry");
  const grading = resultFor("graded answer resembles HTML");
  assert.equal(grading.status, "success");
  const jsonFile = artifactFiles(grading.artifact).find((file) => file.name.endsWith(".json"));
  assert.deepEqual(JSON.parse(decoder.decode(jsonFile.bytes)).response.answers, ["<b>ATP</b>"],
    "literal graded text remains in the accepted answers");
  assert.equal(resultFor("limit keeps first input item").itemCount, 1);
  assert.equal(resultFor("limit keeps first input item").artifact.primary.name, "limited.txt");
  assert.equal(resultFor("zero limit").itemCount, 0);
  assert.equal(resultFor("mixed kinds disabled").itemCount, 1);
  assert.equal(resultFor("representative 1 MiB source").itemCount, 512);
  assert.deepEqual(resultFor("success after errors"), resultFor("repeat success"));
});
