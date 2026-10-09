import { readFile } from "node:fs/promises";
import { expect, test } from "@playwright/test";
import type { Page } from "@playwright/test";
import { unzipSync } from "fflate";
import { checkPackage, initialize } from "../src/index.js";
import type { ConvertRequest } from "../src/index.js";

const text = "MC\t<p>Identify the image: <img src=\"pixel.png\" alt=\"pixel\" /></p>\talpha\tCorrect\tbeta\tIncorrect\n";
const pixel = Buffer.from("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVQIHWP4z8DwHwAFgAI/ScLbtAAAAABJRU5ErkJggg==", "base64");
const otherPixel = Buffer.from("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAusB9Y9ZlS8AAAAASUVORK5CYII=", "base64");

async function downloadedBytes(page: Page): Promise<{ name: string; bytes: Uint8Array }> {
  const nextDownload = page.waitForEvent("download");
  await page.getByRole("list", { name: "Downloads", exact: true }).getByRole("link").click();
  const download = await nextDownload;
  const path = await download.path();
  if (!path) throw new Error("Download was not saved");
  return { name: download.suggestedFilename(), bytes: new Uint8Array(await readFile(path)) };
}

function archiveEntry(entries: Record<string, Uint8Array>, name: string): Uint8Array {
  const bytes = entries[name];
  expect(bytes, `Missing archive entry: ${name}`).toBeDefined();
  if (!bytes) throw new Error(`Missing archive entry: ${name}`);
  return bytes;
}

test.beforeAll(async () => {
  await initialize(new Uint8Array(await readFile(new URL("../generated/qti_wasm_bg.wasm", import.meta.url))));
});

test("worker converts BBQ media to checked ZIP and complete PLE archive", async ({ page, browser }, testInfo) => {
  const workerEvents: string[] = [];
  page.on("worker", (worker) => workerEvents.push(worker.url()));
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await page.goto("/");
  await expect(page.getByRole("status")).toHaveText("Ready");
  expect(workerEvents).toHaveLength(1);
  await page.getByLabel("Question file", { exact: true }).setInputFiles({ name: "questions.txt", mimeType: "text/plain", buffer: Buffer.from(text) });
  await page.getByLabel("Companion files").setInputFiles({ name: "pixel.png", mimeType: "image/png", buffer: pixel });
  await page.getByLabel("Input format").selectOption("bbq_text_upload");
  await page.getByLabel("Output format").selectOption("blackboard_qti_v2_1");
  await page.getByRole("button", { name: "Convert", exact: true }).click();
  await expect(page.getByRole("status")).toHaveText("Converted 1 item(s)");
  const zipDownload = page.waitForEvent("download");
  await page.getByRole("list", { name: "Downloads" }).getByRole("link").click();
  const downloaded = await zipDownload;
  expect(downloaded.suggestedFilename()).toMatch(/\.zip$/);
  const zipPath = await downloaded.path();
  if (!zipPath) throw new Error("ZIP download was not saved");
  const checked = checkPackage({ kind: "zip", bytes: new Uint8Array(await readFile(zipPath)) });
  expect(checked.status).toBe("success");
  if (checked.status !== "success") throw new Error(checked.error.message);
  expect(checked.report.errors).toEqual([]);

  const zipMs = await page.getByRole("status").getAttribute("data-conversion-ms");
  await page.getByLabel("Output format").selectOption("ple_native_json");
  await page.getByRole("button", { name: "Convert", exact: true }).click();
  await expect(page.getByRole("status")).toHaveText("Converted 1 item(s)");
  await expect(page.getByRole("list", { name: "Downloads", exact: true }).getByRole("link")).toHaveCount(1);
  const ple = await downloadedBytes(page);
  expect(ple.name).toBe("ple.zip");
  const entries = unzipSync(ple.bytes);
  expect(Object.keys(entries).sort()).toEqual(["ple/item_00001.json", "ple/media/pixel.png"]);
  const document: unknown = JSON.parse(new TextDecoder().decode(archiveEntry(entries, "ple/item_00001.json")));
  expect(document).toMatchObject({ prompt: expect.stringContaining('src="media/pixel.png"'), response: { kind: "singleChoice", correctChoice: "choice-1" } });
  expect(archiveEntry(entries, "ple/media/pixel.png")).toEqual(new Uint8Array(pixel));
  await expect(page.getByRole("list", { name: "Archive contents" }).getByRole("listitem")).toHaveText(["ple/item_00001.json", "ple/media/pixel.png"]);
  expect(errors).toEqual([]);
  await testInfo.attach("host-timing", {
    contentType: "application/json",
    body: JSON.stringify({ browser: testInfo.project.name, version: browser.version(), initMs: await page.getByRole("status").getAttribute("data-init-ms"), zipConversionMs: zipMs, pleConversionMs: await page.getByRole("status").getAttribute("data-conversion-ms") }),
  });
});

test("editable relative names keep colliding basenames associated with their bytes", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByRole("status")).toHaveText("Ready");
  const nested = 'MC\t<p><img src="first/pixel.png" alt="first" /><img src="second/pixel.png" alt="second" /></p>\tA\tCorrect\tB\tIncorrect\n';
  await page.getByLabel("Question file", { exact: true }).setInputFiles({ name: "nested.txt", mimeType: "text/plain", buffer: Buffer.from(nested) });
  await page.getByLabel("Companion files").setInputFiles([
    { name: "pixel.png", mimeType: "image/png", buffer: pixel },
    { name: "pixel.png", mimeType: "image/png", buffer: otherPixel },
  ]);
  const first = page.getByRole("textbox", { name: "Relative name for companion 1 (pixel.png)", exact: true });
  const second = page.getByRole("textbox", { name: "Relative name for companion 2 (pixel.png)", exact: true });
  await expect(first).toHaveValue("pixel.png");
  await expect(second).toHaveValue("pixel.png");
  await first.fill("first/pixel.png");
  await second.fill("second/pixel.png");
  for (const format of ["ple_native_json", "text2qti"]) {
    await page.getByLabel("Output format").selectOption(format);
    await page.getByRole("button", { name: "Convert", exact: true }).click();
    await expect(page.getByRole("status")).toHaveText("Converted 1 item(s)");
    const download = await downloadedBytes(page);
    const entries = unzipSync(download.bytes);
    const primaryName = format === "ple_native_json" ? "ple/item_00001.json" : "text2qti-package.txt";
    const parent = format === "ple_native_json" ? "ple/" : "";
    const primary = new TextDecoder().decode(archiveEntry(entries, primaryName));
    const content: string = format === "ple_native_json" ? (JSON.parse(primary) as { prompt: string }).prompt : primary;
    const references = format === "ple_native_json"
      ? Array.from(content.matchAll(/src="([^"]+)" alt="(?:first|second)"/g), (match) => match[1])
      : Array.from(content.matchAll(/!\[(?:first|second)\]\(([^ \n]+?\.png)\)/g), (match) => match[1]);
    expect(references).toHaveLength(2);
    const [firstName, secondName] = references;
    if (!firstName || !secondName) throw new Error("Generated document omitted images");
    expect(firstName).not.toBe(secondName);
    expect(Object.keys(entries).sort()).toEqual([primaryName, `${parent}${firstName}`, `${parent}${secondName}`].sort());
    expect(archiveEntry(entries, `${parent}${firstName}`)).toEqual(new Uint8Array(pixel));
    expect(archiveEntry(entries, `${parent}${secondName}`)).toEqual(new Uint8Array(otherPixel));
    await expect(page.locator("img")).toHaveCount(0);
  }
});

test("worker file artifact downloads companions relative to a nested primary parent", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByRole("status")).toHaveText("Ready");
  const worker = page.workers()[0];
  if (!worker) throw new Error("Example worker did not start");
  // Exercise the same worker request handler with the public API's nested output name.
  await worker.evaluate(({ text, pixel }) => {
    const request: ConvertRequest = {
      inputFormat: "bbq_text_upload", outputFormat: "text2qti",
      input: { kind: "file", name: "questions.txt", bytes: new TextEncoder().encode(text), companions: [{ name: "pixel.png", bytes: new Uint8Array(pixel) }] },
      outputName: "course/practice/questions.txt", allowMixed: true,
      document: { title: "Nested output", date: "2026-01-01" }, shuffleSeed: 0,
    };
    self.dispatchEvent(new MessageEvent("message", { data: request }));
  }, { text, pixel: Array.from(pixel) });
  await expect(page.getByRole("status")).toHaveText("Converted 1 item(s)");
  const download = await downloadedBytes(page);
  expect(download.name).toBe("questions.txt.zip");
  const entries = unzipSync(download.bytes);
  expect(Object.keys(entries).sort()).toEqual(["course/practice/media/pixel.png", "course/practice/questions.txt"]);
  expect(new TextDecoder().decode(archiveEntry(entries, "course/practice/questions.txt"))).toContain("![pixel](media/pixel.png)");
  expect(archiveEntry(entries, "course/practice/media/pixel.png")).toEqual(new Uint8Array(pixel));
});

test("Rust validates edited companion names and permits identical duplicate bytes", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByRole("status")).toHaveText("Ready");
  await page.getByLabel("Question file", { exact: true }).setInputFiles({ name: "questions.txt", mimeType: "text/plain", buffer: Buffer.from(text) });
  await page.getByLabel("Companion files").setInputFiles([
    { name: "pixel.png", mimeType: "image/png", buffer: pixel },
    { name: "pixel.png", mimeType: "image/png", buffer: pixel },
  ]);
  await page.getByRole("button", { name: "Convert", exact: true }).click();
  await expect(page.getByRole("status")).toHaveText("Converted 1 item(s)");
  await page.getByRole("textbox", { name: "Relative name for companion 1 (pixel.png)", exact: true }).fill("../pixel.png");
  await page.getByRole("button", { name: "Convert", exact: true }).click();
  await expect(page.getByRole("status")).toHaveText("Conversion failed");
  await expect(page.getByLabel("Diagnostics")).toContainText("media: invalid asset name '../pixel.png'");
  await expect(page.getByRole("list", { name: "Downloads", exact: true }).getByRole("link")).toHaveCount(0);
});

test("authored markup stays downloadable and malformed input stays text", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByRole("status")).toHaveText("Ready");
  const unsafe = "MC\tChoose A. <img src=\"https://example.invalid/x.png\" onerror=\"document.body.dataset.executed='yes'\" />\tA\tCorrect\tB\tIncorrect\n";
  await page.getByLabel("Question file", { exact: true }).setInputFiles({ name: "markup.txt", mimeType: "text/plain", buffer: Buffer.from(unsafe) });
  await page.getByLabel("Output format").selectOption("html_selftest");
  await page.getByRole("button", { name: "Convert", exact: true }).click();
  await expect(page.getByRole("status")).toHaveText("Converted 1 item(s)");
  expect(await page.locator("body").getAttribute("data-executed")).toBeNull();
  await expect(page.locator("img")).toHaveCount(0);
  const html = await downloadedBytes(page);
  expect(html.name).toMatch(/\.html$/);
  expect(new TextDecoder().decode(html.bytes)).toContain("document.body.dataset.executed");
  await page.getByLabel("Question file", { exact: true }).setInputFiles({ name: "bad.txt", mimeType: "text/plain", buffer: Buffer.from([255]) });
  await page.getByRole("button", { name: "Convert", exact: true }).click();
  await expect(page.getByRole("status")).toHaveText("Conversion failed");
  await expect(page.getByLabel("Diagnostics")).not.toHaveText("");
  await expect(page.getByRole("list", { name: "Downloads" }).getByRole("link")).toHaveCount(0);
  expect(await page.locator("body").getAttribute("data-executed")).toBeNull();
});
