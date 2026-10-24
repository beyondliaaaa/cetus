/**
 * M2 exit criteria in a real browser (§12 M2):
 *   SIR and solar–battery run and can be edited in the page; editing errors
 *   are readable; long runs stay responsive (throughput: scripts/bench-wasm.cjs).
 *
 * The strongest check here is determinism through the UI: playing a model to
 * its horizon in the browser must show exactly the golden state hash that
 * native and wasm agree on in CI.
 */

import { readFile } from "node:fs/promises";
import {
  SIR,
  SOLAR,
  expect,
  golden,
  msg,
  openWorkbench,
  resetRun,
  runBar,
  runToHorizon,
  selectModel,
  shownHash,
  test,
  timeReadout,
} from "./support";

test.describe("SIR (default model)", () => {
  test.beforeEach(async ({ page }) => {
    await openWorkbench(page);
  });

  test("plays to the horizon and shows the golden state hash", async ({ page }) => {
    await runToHorizon(page);
    expect(await shownHash(page)).toBe(golden(SIR));
    // Play is disabled once the horizon is reached.
    await expect(runBar(page).getByRole("button", { name: msg("run.play"), exact: true })).toBeDisabled();
  });

  test("steps, pauses and resets", async ({ page }) => {
    const bar = runBar(page);
    const start = await timeReadout(page).textContent();

    await bar.getByRole("button", { name: msg("run.step"), exact: true }).click();
    await expect(timeReadout(page)).not.toHaveText(start ?? "");

    await bar.getByRole("button", { name: msg("run.speedAria", { value: 1 }) }).click();
    await bar.getByRole("button", { name: msg("run.play"), exact: true }).click();
    await expect(bar.getByText(msg("run.live"), { exact: true })).toBeVisible();
    await bar.getByRole("button", { name: msg("run.pause"), exact: true }).click();
    await expect(bar.getByText(msg("run.paused"), { exact: true })).toBeVisible();

    await resetRun(page);
    await expect(timeReadout(page)).toHaveText(start ?? "");
  });

  test("a parameter change moves the result; reset restores the golden run", async ({ page }) => {
    const recovery = page.getByRole("spinbutton", { name: /^Value of recovery in / });
    await recovery.fill("0.3");
    await recovery.press("Enter");
    await runToHorizon(page);
    const changed = await shownHash(page);
    expect(changed).not.toBe(golden(SIR));

    // Changing it back at t = 0 must land on exactly the default run.
    await resetRun(page);
    await recovery.fill("0.3");
    await recovery.press("Enter");
    await page.getByRole("button", { name: msg("param.resetAria", { name: "recovery" }) }).click();
    await expect(recovery).toHaveValue("0.1");
    await runToHorizon(page);
    expect(await shownHash(page)).toBe(golden(SIR));
  });

  test("snapshots restore from memory and from a downloaded file", async ({ page }, testInfo) => {
    const bar = runBar(page);
    const step = bar.getByRole("button", { name: msg("run.step"), exact: true });
    for (let i = 0; i < 5; i += 1) {
      await step.click();
    }
    // The worker answers in order, so once the save reports tick 5 the run bar
    // already shows tick 5 too; only then read it.
    await page.getByRole("button", { name: msg("snapshot.saveMemory") }).click();
    await expect(page.getByText(/^Snapshot saved at tick 5,/)).toBeVisible();
    const savedTime = await timeReadout(page).textContent();
    const savedHash = await shownHash(page);

    const download = page.waitForEvent("download");
    await page.getByRole("button", { name: msg("snapshot.download") }).click();
    const file = testInfo.outputPath("run.cetussnap");
    await (await download).saveAs(file);
    expect((await readFile(file)).length).toBeGreaterThan(8);

    for (let i = 0; i < 5; i += 1) {
      await step.click();
    }
    await expect(timeReadout(page)).not.toHaveText(savedTime ?? "");

    await page.getByRole("button", { name: msg("snapshot.restoreMemory") }).click();
    await expect(page.getByText(msg("snapshot.restored"))).toBeVisible();
    await expect(timeReadout(page)).toHaveText(savedTime ?? "");
    await expect.poll(() => shownHash(page)).toBe(savedHash);

    // A fresh page has no memory snapshot; the file alone must restore the run.
    await page.reload();
    await openWorkbench(page);
    await page.getByLabel(msg("snapshot.fileAria")).setInputFiles(file);
    await expect(page.getByText(msg("snapshot.restoredFile"))).toBeVisible();
    await expect(timeReadout(page)).toHaveText(savedTime ?? "");
    await expect.poll(() => shownHash(page)).toBe(savedHash);
  });

  test("the formula input converts LaTeX and rejects a bare \\log", async ({ page }) => {
    await page.getByRole("button", { name: msg("editor.mathShow") }).click();
    const field = page.locator("math-field");
    await expect(field).toBeVisible({ timeout: 30_000 });

    // Drive the field programmatically: MathLive's own keyboard handling
    // (command mode, inline shortcuts) is its concern, not ours. What matters
    // here is the LaTeX → Cetus conversion and the UI around it.
    const setLatex = async (latex: string): Promise<void> => {
      await field.evaluate((element, value) => {
        (element as HTMLElement & { value: string }).value = value;
        element.dispatchEvent(new Event("input", { bubbles: true }));
      }, latex);
    };

    await setLatex("2x");
    await expect(page.locator(".equation-editor__preview code")).toHaveText("2*x");
    await expect(page.getByRole("button", { name: msg("editor.useFormula") })).toBeEnabled();

    await setLatex("\\log x");
    await expect(page.getByRole("alert").filter({ hasText: "\\lg" })).toBeVisible();
    await expect(page.getByRole("button", { name: msg("editor.useFormula") })).toBeDisabled();

    await setLatex("\\lg x");
    await expect(page.locator(".equation-editor__preview code")).toHaveText("log10(x)");
  });
});

test.describe("solar panel and battery", () => {
  test.beforeEach(async ({ page }) => {
    await openWorkbench(page);
    await selectModel(page, SOLAR);
  });

  test("plays to the horizon and shows the golden state hash", async ({ page }) => {
    await runToHorizon(page);
    expect(await shownHash(page)).toBe(golden(SOLAR));
  });

  test("editing explains a unit error, then applies a valid edit", async ({ page }) => {
    await page.getByRole("combobox", { name: msg("editor.target") }).selectOption({ label: "pv_power" });
    const equation = page.getByLabel(msg("editor.expression"));
    await expect(equation).toHaveValue("efficiency * area * irradiance");
    const apply = page.getByRole("button", { name: msg("editor.apply") });

    // Wrong dimension: an area is not a power.
    await equation.fill("area");
    await expect(page.getByText(/but the target unit is/)).toBeVisible();
    await expect(apply).toBeDisabled();

    // Unparseable: readable error, nothing applied.
    await equation.fill("efficiency * (");
    await expect(page.locator(".equation-editor__error[role=alert]")).toBeVisible();
    await expect(apply).toBeDisabled();

    // Valid: applies, recompiles and resets the run.
    await equation.fill("0.5 * efficiency * area * irradiance");
    await expect(page.getByText(msg("editor.matches"))).toBeVisible();
    await apply.click();
    await expect(page.getByText(msg("editor.applied"))).toBeVisible();
    await expect(runBar(page).getByText(msg("run.paused"), { exact: true })).toBeVisible();

    // The edited model runs, and differs from the original.
    await runToHorizon(page);
    expect(await shownHash(page)).not.toBe(golden(SOLAR));
  });

  test("a stock's authored initial value is kept, not frozen to a number", async ({ page }) => {
    await page.getByRole("combobox", { name: msg("editor.target") }).selectOption({ label: "charge" });
    await expect(page.getByLabel(msg("editor.initial"))).toHaveValue("0.5 * capacity");
  });
});
