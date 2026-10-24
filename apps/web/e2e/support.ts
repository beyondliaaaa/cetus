/**
 * Shared fixtures and helpers for the workbench end-to-end tests.
 *
 * Selectors use roles and accessible names built from the real message
 * catalogue (English), so a copy change updates the tests automatically and a
 * missing accessible name fails them.
 */

import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { expect, test as base, type Locator, type Page } from "@playwright/test";
import { IntlMessageFormat } from "intl-messageformat";
import { en, type MessageKey } from "../src/i18n/messages";

const REPO_ROOT = fileURLToPath(new URL("../../..", import.meta.url));

/** Formats an English UI message exactly as the app does (ICU). */
export function msg(key: MessageKey, values: Record<string, string | number> = {}): string {
  return String(new IntlMessageFormat(en[key], "en").format(values));
}

/** Golden state hash for a sample model (`crates/cetus-testkit/golden/hashes.txt`). */
export function golden(modelPath: string): string {
  const text = readFileSync(`${REPO_ROOT}/crates/cetus-testkit/golden/hashes.txt`, "utf8");
  for (const raw of text.split("\n")) {
    const line = raw.trim();
    if (line === "" || line.startsWith("#")) {
      continue;
    }
    const [key, value] = line.split(/\s+/);
    if (key === modelPath && value !== undefined) {
      return value;
    }
  }
  throw new Error(`no golden hash for ${modelPath}`);
}

export const SIR = "content/modules/bio/epidemic-sir/module.yaml";
export const SOLAR = "content/modules/energy/solar-battery/module.yaml";

/**
 * Every test fails if the page throws, talks to another origin, or gets an
 * HTTP error. This also covers MathLive's fonts and sounds (§10, offline-first)
 * and the wasm/worker chunks.
 */
export const test = base.extend<{ pageHealth: void }>({
  pageHealth: [
    async ({ page, baseURL }, use) => {
      const problems: string[] = [];
      const origin = new URL(baseURL ?? "http://localhost").origin;
      page.on("pageerror", (error) => {
        problems.push(`uncaught error: ${error.message}`);
      });
      page.on("request", (request) => {
        const url = new URL(request.url());
        if ((url.protocol === "http:" || url.protocol === "https:") && url.origin !== origin) {
          problems.push(`external request: ${request.url()}`);
        }
      });
      page.on("response", (response) => {
        if (response.status() >= 400) {
          problems.push(`HTTP ${response.status()}: ${response.url()}`);
        }
      });
      page.on("requestfailed", (request) => {
        const reason = request.failure()?.errorText ?? "";
        // Navigations and reloads abort in-flight requests; that is not a defect.
        if (!reason.includes("ERR_ABORTED") && !request.url().startsWith("blob:")) {
          problems.push(`request failed (${reason}): ${request.url()}`);
        }
      });
      await use();
      expect(problems, problems.join("\n")).toEqual([]);
    },
    { auto: true },
  ],
});

export { expect };

/** The run-controls region (transport, clock, speed, state hash). */
export function runBar(page: Page): Locator {
  return page.getByRole("region", { name: msg("run.section") });
}

/** Opens the workbench and waits until the default model (SIR) is running. */
export async function openWorkbench(page: Page): Promise<void> {
  await page.goto("/");
  await expect(runBar(page).getByRole("button", { name: msg("run.play"), exact: true })).toBeEnabled({
    timeout: 30_000,
  });
}

/** Picks a sample model by its content path. */
export async function selectModel(page: Page, modelPath: string): Promise<void> {
  const picker = page.getByRole("combobox", { name: msg("header.model") });
  // The header renders a placeholder while the catalogue is still booting;
  // wait for the real options before reading their Vite glob keys.
  await expect.poll(() => picker.locator("option").count()).toBeGreaterThan(1);
  // Option values are Vite glob keys (`../../../content/…`); match the suffix.
  const values = await picker.locator("option").evaluateAll((options) =>
    options.map((option) => (option as HTMLOptionElement).value),
  );
  const value = values.find((candidate) => candidate.endsWith(modelPath));
  if (value === undefined) {
    throw new Error(`model ${modelPath} not in picker: ${values.join(", ")}`);
  }
  await picker.selectOption(value);
  await expect(runBar(page).getByRole("button", { name: msg("run.play"), exact: true })).toBeEnabled({
    timeout: 30_000,
  });
}

/** Full state hash currently shown in the run bar (the chip's title). */
export async function shownHash(page: Page): Promise<string> {
  const chip = runBar(page).getByRole("button", { name: msg("hash.copy") });
  const title = await chip.getAttribute("title");
  if (title === null || title.length !== 64) {
    throw new Error(`state hash chip has no full hash: ${String(title)}`);
  }
  return title;
}

/** The clock readout, e.g. `t = 12.40 day / 160 day`. */
export function timeReadout(page: Page): Locator {
  return runBar(page).locator(".time-readout");
}

/** Plays at the fastest speed until the horizon is reached. */
export async function runToHorizon(page: Page): Promise<void> {
  const bar = runBar(page);
  await bar.getByRole("button", { name: msg("run.speedAria", { value: 300 }) }).click();
  await bar.getByRole("button", { name: msg("run.play"), exact: true }).click();
  await expect(bar.getByText(msg("run.done"), { exact: true })).toBeVisible({ timeout: 60_000 });
}

/** Restarts the run (also restores default parameters). */
export async function resetRun(page: Page): Promise<void> {
  const bar = runBar(page);
  await bar.getByRole("button", { name: msg("run.reset"), exact: true }).click();
  await expect(bar.getByText(msg("run.paused"), { exact: true })).toBeVisible();
}
