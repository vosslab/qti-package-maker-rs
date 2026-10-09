import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import { checkPackage, convert, formats, initialize } from "../dist/src/index.js";
import { artifactFiles, bbqBytes, pixelBytes, request } from "./fixtures.mjs";

const started = performance.now();
await initialize(new Uint8Array(await readFile(new URL("../dist/generated/qti_wasm_bg.wasm", import.meta.url))));
console.log(JSON.stringify({ node: process.version, initMs: performance.now() - started }));

test("release ESM uses owned bytes through every writer and all four readers", () => {
  const inventory = formats().formats;
  assert.equal(inventory.filter((format) => format.canWrite).length, 11);
  assert.equal(inventory.filter((format) => format.canRead).length, 4);
  const artifacts = new Map();
  const times = [];
  for (const format of inventory.filter((format) => format.canWrite)) {
    const started = performance.now();
    const result = convert(request(format.name, format.defaultOutputName));
    times.push({ format: format.name, conversionMs: performance.now() - started });
    assert.equal(result.status, "success", JSON.stringify(result));
    assert.equal(result.itemCount, 1);
    const files = artifactFiles(result.artifact);
    assert.ok(files.length > 0, format.name);
    for (const file of files) {
      assert.ok(file.bytes instanceof Uint8Array);
      assert.ok(file.bytes.byteLength > 0);
    }
    artifacts.set(format.name, result.artifact);
    if (format.defaultOutputName.endsWith(".zip")) {
      const checked = checkPackage({ kind: "zip", bytes: files[0].bytes });
      assert.equal(checked.status, "success", JSON.stringify(checked));
      assert.deepEqual(checked.report.errors, [], format.name);
    }
  }
  for (const format of inventory.filter((format) => format.canRead)) {
    const source = artifacts.get(format.name);
    assert.ok(source && source.kind === "file", format.name);
    const result = convert({
      inputFormat: format.name,
      outputFormat: "bbq_text_upload",
      input: { kind: "file", name: source.primary.name, bytes: source.primary.bytes, companions: source.companions },
      outputName: "roundtrip.txt",
      document: { title: "Roundtrip", date: "2026-01-01" },
      shuffleSeed: 0,
      allowMixed: true,
    });
    assert.equal(result.status, "success", JSON.stringify(result));
    assert.equal(result.itemCount, 1, format.name);
  }
  console.log(JSON.stringify({ conversionTimes: times }));
});

test("PLE directory and text2qti companions preserve media bytes across repeated calls", () => {
  const ple = convert(request("ple_native_json", "ple"));
  assert.equal(ple.status, "success", JSON.stringify(ple));
  assert.equal(ple.artifact.kind, "directory");
  const entries = ple.artifact.entries;
  const json = entries.find((file) => file.name.endsWith(".json"));
  const media = entries.find((file) => file.name.startsWith("media/"));
  assert.ok(json && media);
  assert.deepEqual(media.bytes, pixelBytes);
  const source = JSON.parse(new TextDecoder().decode(json.bytes));
  assert.equal(source.response.kind, "singleChoice");
  assert.equal(source.response.correctChoice, "choice-1");
  const saved = new Uint8Array(media.bytes);
  const text = convert(request("text2qti", "questions.txt"));
  assert.equal(text.status, "success", JSON.stringify(text));
  assert.equal(text.artifact.kind, "file");
  assert.equal(text.artifact.companions.length, 1);
  assert.deepEqual(text.artifact.companions[0].bytes, pixelBytes);
  assert.deepEqual(media.bytes, saved);
  assert.deepEqual(bbqBytes, request("text2qti", "questions.txt").input.bytes);
});

test("input and output buffers remain owned after mutation and subsequent conversion", () => {
  const mutable = request("ple_native_json", "ple");
  mutable.input.bytes = new Uint8Array(mutable.input.bytes);
  mutable.input.companions = [{ name: "pixel.png", bytes: new Uint8Array(pixelBytes) }];
  const first = convert(mutable);
  assert.equal(first.status, "success", JSON.stringify(first));
  const media = artifactFiles(first.artifact).find((file) => file.name.startsWith("media/"));
  assert.ok(media);
  const snapshot = new Uint8Array(media.bytes);
  mutable.input.bytes.fill(0);
  mutable.input.companions[0].bytes.fill(0);
  assert.deepEqual(media.bytes, snapshot);
  media.bytes.fill(0);
  const second = convert(request("ple_native_json", "ple"));
  assert.equal(second.status, "success", JSON.stringify(second));
  const freshMedia = artifactFiles(second.artifact).find((file) => file.name.startsWith("media/"));
  assert.ok(freshMedia);
  assert.deepEqual(freshMedia.bytes, pixelBytes);
});

test("malformed JavaScript DTO shapes become diagnostics and leave later calls usable", () => {
  const valid = request("ple_native_json", "ple");
  for (const [label, malformed] of Object.entries({
    nullRequest: null,
    unknownTopLevel: { ...valid, unexpected: true },
    nativeRenderOption: { ...valid, htmlToImage: true },
    unknownDocument: { ...valid, document: { ...valid.document, unexpected: true } },
    unknownInput: { ...valid, input: { ...valid.input, unexpected: true } },
    unknownCompanion: { ...valid, input: { ...valid.input, companions: [{ name: "pixel.png", bytes: pixelBytes, unexpected: true }] } },
    unknownEntriesInput: { ...valid, input: { kind: "entries", name: "bank", entries: [], unexpected: true } },
    unknownEntry: { ...valid, input: { kind: "entries", name: "bank", entries: [{ name: "item.txt", bytes: bbqBytes, unexpected: true }] } },
    negativeLimit: { ...valid, limit: -1 },
    fractionalSeed: { ...valid, shuffleSeed: 0.5 },
    ordinaryBytes: { ...valid, input: { kind: "file", name: "x", bytes: [1, 2] } },
    ordinaryCompanionBytes: { ...valid, input: { ...valid.input, companions: [{ name: "pixel.png", bytes: [1] }] } },
  })) {
    const result = convert(malformed);
    assert.equal(result.status, "error", label);
    assert.equal(typeof result.error.category, "string");
  }
  for (const [label, malformed] of Object.entries({
    nullPackage: null,
    ordinaryZipBytes: { kind: "zip", bytes: [1, 2] },
    ordinaryEntryBytes: { kind: "entries", entries: [{ name: "x", bytes: [1] }] },
    unknownZipField: { kind: "zip", bytes: new Uint8Array([1]), unexpected: true },
    unknownEntriesField: { kind: "entries", entries: [], unexpected: true },
    unknownPackageEntryField: { kind: "entries", entries: [{ name: "x", bytes: new Uint8Array([1]), unexpected: true }] },
  })) {
    const checked = checkPackage(malformed);
    assert.equal(checked.status, "error", label);
  }
  assert.equal(convert(valid).status, "success");
});

test("malformed inputs return discriminated diagnostics without throwing", () => {
  const unknown = convert(request("unknown-format", "out.txt"));
  assert.equal(unknown.status, "error");
  assert.equal(typeof unknown.error.category, "string");
  const malformed = convert({ ...request("bbq_text_upload", "out.txt"), input: { kind: "file", name: "bad.txt", bytes: new Uint8Array([255]) } });
  assert.equal(malformed.status, "error");
  const invalidName = convert({ ...request("text2qti", "out.txt"), input: { kind: "file", name: "input.txt", bytes: bbqBytes, companions: [{ name: "../pixel.png", bytes: pixelBytes }] } });
  assert.equal(invalidName.status, "error");
  const brokenZip = convert({ ...request("blackboard_qti_v2_1", "out.zip"), inputFormat: "blackboard_export_zip", input: { kind: "file", name: "broken.zip", bytes: new Uint8Array([1]) } });
  assert.equal(brokenZip.status, "error");
  assert.equal(brokenZip.error.format, "blackboard_export_zip");
  assert.equal(brokenZip.error.logicalName, "broken.zip");
  assert.equal(brokenZip.error.source, "broken.zip");
  const archive = checkPackage({ kind: "zip", bytes: new Uint8Array([0, 1, 2]) });
  assert.equal(archive.status, "success");
  assert.ok(archive.report.errors.some((finding) => finding.code === "package-input"));
  const entries = checkPackage({ kind: "entries", entries: [{ name: "../escape.txt", bytes: new Uint8Array([1]) }] });
  assert.equal(entries.status, "error");
});
