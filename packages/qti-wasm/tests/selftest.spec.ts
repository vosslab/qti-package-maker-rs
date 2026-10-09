import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { expect, test } from "@playwright/test";
import type { Page } from "@playwright/test";
import { convert, initialize } from "../dist/src/index.js";

const root = fileURLToPath(new URL("../../../", import.meta.url));
const encoder = new TextEncoder();
const decoder = new TextDecoder();
const sources = {
  mc: "MC\tEnergy carrier\tATP\tCorrect\tADP\tIncorrect\n",
  ma: "MA\tSelect purines\tadenine\tCorrect\tguanine\tCorrect\tcytosine\tIncorrect\n",
  match: "MAT\tMatch bases\tadenine\tthymine\tguanine\tcytosine\n",
  num: "NUM\tConcentration\t2.5\t0.125\n",
  fib: "FIB\tEnergy molecule\tATP\tadenosine triphosphate\n",
  multi: "FIB_PLUS\tDNA has [base] and another [base], plus [pair]\tbase\tadenine\tA\t\tpair\tthymine\tT\n",
  order: "ORD\tOrder gene expression\ttranscription\tRNA processing\ttranslation\n",
} as const;

type SourceName = keyof typeof sources;
type RenderedHost = { name: "native" | "wasm"; html: Record<SourceName, string> };

let rendered: RenderedHost[];

function wasmHtml(source: string): string {
  const result = convert({
    inputFormat: "bbq_text_upload",
    outputFormat: "html_selftest",
    input: { kind: "file", name: "selftest.txt", bytes: encoder.encode(source) },
    allowMixed: true,
    document: { title: "Self-test parity", date: "2026-10-09" },
    shuffleSeed: 0,
  });
  assert.equal(result.status, "success", JSON.stringify(result));
  assert.ok(result.artifact);
  assert.equal(result.artifact.kind, "file");
  return decoder.decode(result.artifact.primary.bytes);
}

function nativeHtml(sourcesToRender: readonly string[]): string[] {
  const operations = sourcesToRender.map((source) => ({
    operation: "convert",
    request: {
      inputFormat: "bbq_text_upload",
      outputFormat: "html_selftest",
      input: { kind: "file", name: "selftest.txt", bytes: Array.from(encoder.encode(source)) },
      allowMixed: true,
      document: { title: "Self-test parity", date: "2026-10-09" },
      shuffleSeed: 0,
    },
  }));
  const process = spawnSync("cargo", ["run", "--quiet", "--locked", "-p", "qti-wasm", "--example", "parity_oracle"], {
    cwd: root,
    input: operations.map((operation) => JSON.stringify(operation)).join("\n") + "\n",
    encoding: "utf8",
    timeout: 240_000,
    maxBuffer: 32 * 1024 * 1024,
  });
  assert.ifError(process.error);
  assert.equal(process.status, 0, process.stderr);
  return process.stdout.trim().split("\n").map((line) => {
    const result: unknown = JSON.parse(line);
    assert.ok(result && typeof result === "object" && "status" in result);
    const conversion = result as { status: string; artifact?: { kind: string; primary?: { bytes: number[] } } };
    assert.equal(conversion.status, "success");
    assert.equal(conversion.artifact?.kind, "file");
    assert.ok(conversion.artifact?.primary);
    return decoder.decode(new Uint8Array(conversion.artifact.primary.bytes));
  });
}

function crc(html: string): string {
  const found = /id="question_html_([^"]+)"/.exec(html);
  assert.ok(found?.[1], "self-test must expose a question CRC");
  return found[1];
}

async function mount(page: Page, html: string): Promise<string> {
  await page.goto("about:blank");
  await page.setContent(html);
  const id = crc(html);
  await expect(page.locator(`#question_html_${id}`)).toHaveCount(1);
  await expect(page.locator(`#statement_text_${id}`)).toHaveCount(1);
  return id;
}

async function wrapGrade(page: Page, id: string): Promise<void> {
  await page.evaluate((crcText) => {
    const name = `checkAnswer_${crcText}`;
    const candidate = Reflect.get(window, name);
    if (typeof candidate !== "function") throw new Error(`missing ${name}`);
    const calls = (Reflect.get(window, "__selftestCalls") as string[] | undefined) ?? [];
    Reflect.set(window, "__selftestCalls", calls);
    Reflect.set(window, name, () => {
      calls.push(crcText);
      candidate();
      const result = document.getElementById(`result_${crcText}`)?.textContent;
      const scoredCorrect = /^(?:Total Score|Correct positions):\s*(\d+)\s+(?:out of|of)\s+(\d+)$/.exec(result ?? "");
      if (result === "CORRECT" || (scoredCorrect !== null && scoredCorrect[1] === scoredCorrect[2])) {
        const completed = (Reflect.get(window, "__completedByCrc") as Set<string> | undefined) ?? new Set<string>();
        completed.add(crcText);
        Reflect.set(window, "__completedByCrc", completed);
      }
    });
  }, id);
}

async function hookCalls(page: Page): Promise<string[]> {
  return page.evaluate(() => (Reflect.get(window, "__selftestCalls") as string[] | undefined) ?? []);
}

async function resultText(page: Page, id: string): Promise<string> {
  return page.locator(`#result_${id}`).textContent().then((value) => value ?? "");
}

async function gradeCorrectViaControls(page: Page, id: string): Promise<string> {
  const box = page.locator(`#question_html_${id}`);
  const kind = await box.getAttribute("data-kind");
  // These are authored fixture answers. Do not read data-correct, answer tokens,
  // or any other renderer-provided answer data: that would make a broken key pass.
  if (kind === "mc") await box.getByLabel(/ATP/).check();
  else if (kind === "ma") {
    await box.getByLabel(/adenine/).check();
    await box.getByLabel(/guanine/).check();
  } else if (kind === "fib") await box.locator(".qti-fib-input").fill("adenosine triphosphate");
  else if (kind === "num") await box.locator(".qti-num-input").fill("2.625");
  else if (kind === "multi-fib") {
    const blanks = box.locator(".fib-blank");
    await blanks.nth(0).fill("A");
    await blanks.nth(1).fill("adenine");
    await blanks.nth(2).fill("T");
  } else if (kind === "match") {
    await box.getByRole("button", { name: /thymine/ }).click();
    await box.getByRole("button", { name: /Assign a choice to prompt 1/ }).click();
    await box.getByRole("button", { name: /cytosine/ }).click();
    await box.getByRole("button", { name: /Assign a choice to prompt 2/ }).click();
  } else if (kind === "order") {
    // Authored order: transcription, RNA processing, translation. The rendered
    // fixture begins reversed, so move transcription to first, then RNA to second.
    const transcription = box.locator(".qti-order-row", { hasText: "transcription" });
    await transcription.locator("[data-direction=up]").click();
    await transcription.locator("[data-direction=up]").click();
    await box.locator(".qti-order-row", { hasText: "RNA processing" }).locator("[data-direction=up]").click();
  } else throw new Error(`unknown self-test kind ${kind}`);
  await box.locator("[data-action=grade]").click();
  return resultText(page, id);
}

async function gradeMeaningfullyWrong(page: Page, id: string): Promise<void> {
  const box = page.locator(`#question_html_${id}`);
  const kind = await box.getAttribute("data-kind");
  if (kind === "mc") await box.getByLabel(/ADP/).check();
  else if (kind === "ma") await box.getByLabel(/adenine/).check();
  else if (kind === "fib") await box.locator(".qti-fib-input").fill("ADP");
  else if (kind === "num") await box.locator(".qti-num-input").fill("3");
  else if (kind === "multi-fib") await box.locator(".fib-blank").first().fill("adenine");
  else if (kind === "match") {
    // Deliberately cross-wire the two known base pairs.
    await box.getByRole("button", { name: /cytosine/ }).click();
    await box.getByRole("button", { name: /Assign a choice to prompt 1/ }).click();
  } else if (kind === "order") {
    // The fixture's initial order is intentionally reversed.
  } else throw new Error(`unknown self-test kind ${kind}`);
  await box.locator("[data-action=grade]").click();
}

async function resetAndExpectCurrentBehavior(page: Page, id: string): Promise<void> {
  const box = page.locator(`#question_html_${id}`);
  const kind = await box.getAttribute("data-kind");
  const reset = box.locator("[data-action=reset]");
  if (kind === "ma") {
    await reset.click();
    await expect(box.locator("input:checked")).toHaveCount(0);
  } else if (kind === "match") {
    await reset.click();
    await expect(box.locator("[role=status]")).toContainText("Matches reset.");
    await expect(box.locator(".qti-match-slot").first()).toHaveText("Drop Your Choice Here");
  } else if (kind === "order") {
    await reset.click();
    await expect(box.locator("[role=status]")).toContainText("Order reset.");
    await expect(box.locator(".qti-order-row").first()).toContainText("translation");
  } else {
    await expect(reset).toHaveCount(0);
  }
}

test.beforeAll(async () => {
  await initialize(new Uint8Array(await readFile(new URL("../dist/generated/qti_wasm_bg.wasm", import.meta.url))));
  const names = Object.keys(sources) as SourceName[];
  const native = nativeHtml(names.map((name) => sources[name]));
  rendered = [
    { name: "native", html: Object.fromEntries(names.map((name, index) => [name, native[index] ?? ""])) as Record<SourceName, string> },
    { name: "wasm", html: Object.fromEntries(names.map((name) => [name, wasmHtml(sources[name])])) as Record<SourceName, string> },
  ];
});

test("native and delivered Wasm self-tests preserve stable question identities", async ({ page }) => {
  const names = Object.keys(sources) as SourceName[];
  const native = names.map((name) => rendered[0]?.html[name] ?? "");
  const wasm = names.map((name) => rendered[1]?.html[name] ?? "");
  for (let index = 0; index < names.length; index++) {
    expect(crc(native[index] ?? "")).toBe(crc(wasm[index] ?? ""));
    expect(native[index]).toContain(`statement_text_${crc(native[index] ?? "")}`);
  }
  const variantA = wasmHtml("NUM\tConcentration\t2.5\t0.125\n");
  const variantB = wasmHtml("NUM\tConcentration\t3.5\t0.125\n");
  expect(crc(variantA)).not.toBe(crc(variantB));
  await mount(page, native[0] ?? "");
});

test("native and delivered Wasm self-tests grade all authored fixtures", async ({ page }) => {
  for (const host of rendered) {
    for (const name of Object.keys(sources) as SourceName[]) {
      const html = host.html[name];
      const id = await mount(page, html);
      const box = page.locator(`#question_html_${id}`);
      await wrapGrade(page, id);
      await expect(box.locator("[data-action=grade]")).toBeEnabled();
      if (name === "num") {
        await box.locator(".qti-num-input").fill("Infinity");
        await box.locator("[data-action=grade]").click();
        await expect(box.locator(`#result_${id}`)).toHaveText("Please enter a valid number.");
      }
      await gradeMeaningfullyWrong(page, id);
      const feedback = await resultText(page, id);
      if (name === "ma") expect(feedback).toBe("Too few answers selected. You got 1 out of 2 correct.");
      else if (name === "num") expect(feedback).toBe("Too high. Try again.");
      else if (name === "multi") expect(feedback).toBe("Correct: 1 of 3");
      else if (name === "match") expect(feedback).toBe("Total Score: 0 out of 2");
      else if (name === "order") expect(feedback).toBe("Correct positions: 1 of 3");
      else expect(feedback).toBe("incorrect");
      expect(await page.evaluate((crcText) => (Reflect.get(window, "__completedByCrc") as Set<string> | undefined)?.has(crcText) ?? false, id)).toBe(false);
      await resetAndExpectCurrentBehavior(page, id);
      await gradeCorrectViaControls(page, id);
      await expect(box.locator(`#result_${id}`)).toHaveText(name === "match" ? "Total Score: 2 out of 2"
        : name === "order" ? "Correct positions: 3 of 3" : "CORRECT");
      await expect(box.locator(`#result_${id}`)).toHaveClass(/qti-feedback-success/);
      await expect(box.locator("[data-action=grade]")).toBeEnabled();
      expect(await hookCalls(page)).toEqual(name === "num" ? [id, id, id] : [id, id]);
      expect(await page.evaluate((crcText) => (Reflect.get(window, "__completedByCrc") as Set<string>).has(crcText), id)).toBe(true);
      await resetAndExpectCurrentBehavior(page, id);
      expect(await page.evaluate((crcText) => (Reflect.get(window, "__completedByCrc") as Set<string>).has(crcText), id)).toBe(true);
    }
  }
});

test("grading feedback and keyboard behavior use the public hook exactly once", async ({ page }) => {
  for (const host of rendered) {
    const mcId = await mount(page, host.html.mc);
    await wrapGrade(page, mcId);
    const mc = page.locator(`#question_html_${mcId}`);
    await mc.locator("[data-action=grade]").click();
    await expect(mc.locator(`#result_${mcId}`)).toHaveText("Please select an answer.");
    await mc.getByLabel(/ATP/).check();
    await mc.locator("[data-action=grade]").click();
    await expect(mc.locator(`#result_${mcId}`)).toHaveText("CORRECT");
    await expect(mc.locator("[data-action=grade]")).toBeEnabled();
    expect(await hookCalls(page)).toEqual([mcId, mcId]);

    const numId = await mount(page, host.html.num);
    await wrapGrade(page, numId);
    const numeric = page.locator(`#question_html_${numId} .qti-num-input`);
    await numeric.press("Enter");
    await expect(page.locator(`#result_${numId}`)).toHaveText("Please enter a value.");
    await numeric.fill("2.625");
    await numeric.press("Enter");
    await expect(page.locator(`#result_${numId}`)).toHaveText("CORRECT");
    await expect(page.locator(`#question_html_${numId} [data-action=grade]`)).toBeEnabled();
    expect(await hookCalls(page)).toEqual([numId, numId]);
  }

  const fibId = await mount(page, wasmHtml(sources.fib));
  await wrapGrade(page, fibId);
  await page.locator(`#question_html_${fibId} .qti-fib-input`).fill("adenosine triphosphate");
  await page.locator(`#question_html_${fibId} .qti-fib-input`).press("Enter");
  expect(await hookCalls(page)).toEqual([]);

  const multiId = await mount(page, wasmHtml(sources.multi));
  await wrapGrade(page, multiId);
  const blanks = page.locator(`#question_html_${multiId} .fib-blank`);
  await expect(blanks).toHaveCount(3);
  await blanks.nth(0).fill("adenine");
  await blanks.nth(0).press("Enter");
  expect(await hookCalls(page)).toEqual([]);
  await page.locator(`#question_html_${multiId} [data-action=grade]`).click();
  await expect(page.locator(`#result_${multiId}`)).toHaveText("Correct: 1 of 3");
  await blanks.nth(1).fill("A");
  await blanks.nth(2).fill("T");
  await page.locator(`#question_html_${multiId} [data-action=grade]`).click();
  await expect(page.locator(`#result_${multiId}`)).toHaveText("CORRECT");
  expect(await hookCalls(page)).toEqual([multiId, multiId]);
});

test("MA, MATCH, and ORDER retain current interactive controls", async ({ page }) => {
  const maId = await mount(page, wasmHtml(sources.ma));
  const ma = page.locator(`#question_html_${maId}`);
  await ma.getByLabel(/adenine/).check();
  await ma.locator("[data-action=grade]").click();
  await expect(ma.locator(`#result_${maId}`)).toHaveText("Too few answers selected. You got 1 out of 2 correct.");
  await ma.locator("[data-action=reset]").click();
  await expect(ma.locator("input:checked")).toHaveCount(0);

  const matchHtml = wasmHtml(sources.match);
  const replayScripts = async (html: string) => page.evaluate((nextHtml) => {
    const parsed = new DOMParser().parseFromString(nextHtml, "text/html");
    for (const control of parsed.querySelectorAll("script")) {
      const script = document.createElement("script");
      script.textContent = control.textContent;
      document.head.append(script);
    }
  }, html);
  const matchId = await mount(page, matchHtml);
  await replayScripts(matchHtml);
  const matchBox = page.locator(`#question_html_${matchId}`);
  const firstSlot = matchBox.locator(".qti-match-slot").first();
  await firstSlot.focus();
  await firstSlot.press("A");
  await expect(firstSlot).not.toHaveText("Drop Your Choice Here");
  const reusableChoice = matchBox.locator(".qti-match-choice").first();
  const secondSlot = matchBox.locator(".qti-match-slot").nth(1);
  await reusableChoice.click();
  await secondSlot.click();
  await reusableChoice.dragTo(firstSlot, { targetPosition: { x: 8, y: 24 } });
  await expect(reusableChoice).toBeEnabled();
  const repeatedValue = await secondSlot.getAttribute("data-value");
  if (!repeatedValue) throw new Error("MATCH assignment did not retain a choice value");
  await expect(firstSlot).toHaveAttribute("data-value", repeatedValue);

  const orderHtml = wasmHtml(sources.order);
  const orderId = await mount(page, orderHtml);
  await replayScripts(orderHtml);
  const orderBox = page.locator(`#question_html_${orderId}`);
  // Repeated initialization must not duplicate the effect of one move command.
  const rows = orderBox.locator(".qti-order-row");
  const answers = rows.locator(".qti-choice-content");
  const before = await answers.allTextContents();
  await rows.last().locator("[data-direction=up]").click();
  await expect(answers).toHaveText([...before.slice(0, -2), ...before.slice(-2).reverse()]);
});

test("wrapped grading survives same-node replay, concurrent questions, and A-B-A progress", async ({ page }) => {
  const htmlA = wasmHtml("MC\tVariant A\tATP\tCorrect\tADP\tIncorrect\n");
  const htmlB = wasmHtml("MC\tVariant B\tGTP\tCorrect\tGDP\tIncorrect\n");
  const idA = await mount(page, htmlA);
  await wrapGrade(page, idA);

  const replayScripts = async (html: string) => page.evaluate((nextHtml) => {
    const parsed = new DOMParser().parseFromString(nextHtml, "text/html");
    for (const control of parsed.querySelectorAll("script")) {
      const script = document.createElement("script");
      script.textContent = control.textContent;
      document.head.append(script);
    }
  }, html);
  const replaceWith = async (html: string) => page.evaluate((nextHtml) => {
    const parsed = new DOMParser().parseFromString(nextHtml, "text/html");
    const item = parsed.querySelector(".qti-selftest-item");
    if (!item) throw new Error("self-test missing item");
    document.body.replaceChildren(item);
  }, html);

  await page.evaluate((id) => Reflect.set(window, "__wrapperBeforeReplay", Reflect.get(window, `checkAnswer_${id}`)), idA);
  await replayScripts(htmlA);
  expect(await page.evaluate((id) => Reflect.get(window, "__wrapperBeforeReplay") === Reflect.get(window, `checkAnswer_${id}`), idA)).toBe(true);
  await page.locator(`#question_html_${idA}`).getByLabel(/ATP/).check();
  await page.locator(`#question_html_${idA} [data-action=grade]`).click();
  await expect(page.locator(`#result_${idA}`)).toHaveText("CORRECT");
  expect(await hookCalls(page)).toEqual([idA]);

  await replaceWith(htmlB);
  await replayScripts(htmlB);
  const idB = crc(htmlB);
  await wrapGrade(page, idB);
  await expect(page.locator(`#result_${idB}`)).toHaveText("");
  expect(await page.evaluate((id) => (Reflect.get(window, "__completedByCrc") as Set<string>).has(id), idB)).toBe(false);
  await page.locator(`#question_html_${idB}`).getByLabel(/GDP/).check();
  await page.locator(`#question_html_${idB} [data-action=grade]`).click();
  await expect(page.locator(`#result_${idB}`)).toHaveText("incorrect");
  expect(await page.evaluate((id) => (Reflect.get(window, "__completedByCrc") as Set<string>).has(id), idB)).toBe(false);

  await replaceWith(htmlA);
  await replayScripts(htmlA);
  await expect(page.locator(`#result_${idA}`)).toHaveText("");
  expect(await page.evaluate((id) => (Reflect.get(window, "__completedByCrc") as Set<string>).has(id), idA)).toBe(true);
  await page.locator(`#question_html_${idA}`).getByLabel(/ATP/).check();
  await page.locator(`#question_html_${idA} [data-action=grade]`).click();
  expect(await hookCalls(page)).toEqual([idA, idB, idA]);

  // Mount two questions together. Their feedback and completion state must stay
  // local even though the public hook names share the same global namespace.
  await page.goto("about:blank");
  await page.setContent(`${htmlA}${htmlB}`);
  await wrapGrade(page, idA);
  await wrapGrade(page, idB);
  const a = page.locator(`#question_html_${idA}`);
  const b = page.locator(`#question_html_${idB}`);
  await a.getByLabel(/ATP/).check();
  await a.locator("[data-action=grade]").click();
  await expect(a.locator(`#result_${idA}`)).toHaveText("CORRECT");
  await expect(b.locator(`#result_${idB}`)).toHaveText("");
  await b.getByLabel(/GDP/).check();
  await b.locator("[data-action=grade]").click();
  await expect(b.locator(`#result_${idB}`)).toHaveText("incorrect");
  await expect(a.locator(`#result_${idA}`)).toHaveText("CORRECT");
});
