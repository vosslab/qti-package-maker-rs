import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { expect, test } from "@playwright/test";
import type { Page } from "@playwright/test";
import { convert, initialize } from "../dist/src/index.js";

const root = fileURLToPath(new URL("../../../", import.meta.url));
const pythonRoot = fileURLToPath(new URL("../../../../qti-package-maker", import.meta.url));
const encoder = new TextEncoder();
const decoder = new TextDecoder();
const sources = [
  "MC\tEnergy carrier\tATP\tCorrect\tADP\tIncorrect\n",
  "MA\tSelect purines\tadenine\tCorrect\tguanine\tCorrect\tcytosine\tIncorrect\n",
  "MAT\tMatch bases\tadenine\tthymine\tguanine\tcytosine\n",
  "NUM\tConcentration\t2.5\t0.125\n",
  "FIB\tEnergy molecule\tATP\tadenosine triphosphate\n",
  "FIB_PLUS\tDNA has [base] and another [base], plus [pair]\tbase\tadenine\tA\t\tpair\tthymine\tT\n",
  "ORD\tOrder gene expression\ttranscription\tRNA processing\ttranslation\n",
];

function rustHtml(source: string): string {
  const result = convert({
    inputFormat: "bbq_text_upload", outputFormat: "html_selftest",
    input: { kind: "file", name: "selftest.txt", bytes: encoder.encode(source) },
    allowMixed: true, document: { title: "Python migration parity", date: "2026-10-09" }, shuffleSeed: 0,
  });
  assert.equal(result.status, "success", JSON.stringify(result));
  assert.ok(result.artifact && result.artifact.kind === "file");
  return decoder.decode(result.artifact.primary.bytes);
}

function pythonHtml(source: string): { crc: string; html: string } {
  if (!existsSync(`${pythonRoot}/qti_package_maker`)) {
    throw new Error(`Current Python QPM checkout is required for npm run test:python-parity (looked for ${pythonRoot}).`);
  }
  const encoded = Buffer.from(source).toString("base64");
  const result = spawnSync("bash", ["-lc", "source source_me.sh && python3 packages/qti-wasm/tests/selftest_oracle.py \"$1\" \"$2\"", "qti-selftest-oracle", pythonRoot, encoded], {
    cwd: root,
    encoding: "utf8",
    timeout: 60_000,
  });
  assert.ifError(result.error);
  assert.equal(result.status, 0, result.stderr);
  return JSON.parse(result.stdout) as { crc: string; html: string };
}

function crc(html: string): string {
  const match = /id=["']question_html_([^"']+)/.exec(html);
  assert.ok(match?.[1], "self-test must expose a question CRC");
  return match[1];
}

async function freshMount(page: Page, html: string): Promise<string> {
  await page.goto("about:blank");
  await page.setContent(html);
  return crc(html);
}

async function fillCorrectAndGrade(page: Page, id: string, source: string): Promise<string> {
  const box = page.locator(`#question_html_${id}`);
  const kind = source.split("\t", 1)[0];
  if (kind === "MC") await box.getByLabel(/ATP/).check();
  else if (kind === "MA") {
    await box.getByLabel(/adenine/).check();
    await box.getByLabel(/guanine/).check();
  } else if (kind === "FIB") await page.locator(`#fib_input_${id}`).fill("adenosine triphosphate");
  else if (kind === "NUM") await page.locator(`#num_input_${id}`).fill("2.625");
  else if (kind === "FIB_PLUS") {
    const blanks = box.locator(".fib-blank");
    for (let index = 0; index < await blanks.count(); index++) await blanks.nth(index).fill(index < 2 ? "A" : "T");
  } else if (kind === "MAT") {
    await box.locator(".qti-match-choice", { hasText: "thymine" }).click();
    await box.locator(".qti-match-slot").nth(0).click();
    await box.locator(".qti-match-choice", { hasText: "cytosine" }).click();
    await box.locator(".qti-match-slot").nth(1).click();
  } else if (kind === "ORD") {
    const expected = source.trim().split("\t").slice(2);
    for (let index = 0; index < expected.length; index++) {
      const desired = expected[index];
      if (!desired) throw new Error("missing ORDER answer");
      while ((await box.locator(".qti-order-row").allTextContents()).findIndex((text) => text.includes(desired)) > index) {
        await box.locator(".qti-order-row", { hasText: desired }).locator("[data-direction=up]").click();
      }
    }
  } else throw new Error(`unknown self-test kind ${kind}`);
  await box.locator("[data-action=grade], button[onclick^=checkAnswer]").first().click();
  return (await page.locator(`#result_${id}`).textContent()) ?? "";
}

test.beforeAll(async () => {
  await initialize(new Uint8Array(await readFile(new URL("../dist/generated/qti_wasm_bg.wasm", import.meta.url))));
  // Fail before browsers start if the explicitly selected migration oracle is unavailable.
  if (!existsSync(`${pythonRoot}/qti_package_maker`)) throw new Error(`current Python checkout is unavailable: ${pythonRoot}`);
  readFileSync(`${pythonRoot}/qti_package_maker/engines/html_selftest/write_item.py`);
});

test("current Python and delivered Wasm agree on self-test identity and correct-grade feedback", async ({ page }) => {
  for (const source of sources) {
    const python = pythonHtml(source);
    const rust = rustHtml(source);
    expect(crc(rust)).toBe(python.crc);
    const pythonId = await freshMount(page, python.html);
    const pythonFeedback = await fillCorrectAndGrade(page, pythonId, source);
    const expected = source.startsWith("MAT\t") ? "Total Score: 2 out of 2"
      : source.startsWith("ORD\t") ? "Correct positions: 3 of 3" : "CORRECT";
    expect(pythonFeedback).toBe(expected);
    const rustId = await freshMount(page, rust);
    const rustFeedback = await fillCorrectAndGrade(page, rustId, source);
    expect(rustFeedback).toBe(pythonFeedback);
  }
});
