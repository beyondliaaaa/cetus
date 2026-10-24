/**
 * Phone-portrait layout (§12 M2: "实验台的 L0–L1 交互按手机竖屏设计，支持触屏操作").
 *
 * This is a viewport emulation in desktop Chromium. It does not replace the
 * still-open real mid-range Android check (performance, real touch).
 */

import {
  SIR,
  expect,
  golden,
  msg,
  openWorkbench,
  runBar,
  shownHash,
  test,
} from "./support";

for (const width of [360, 390]) {
  test.describe(`${width}px portrait`, () => {
    test.use({ viewport: { width, height: 780 }, isMobile: true, hasTouch: true });

    test("has no horizontal overflow and large enough touch targets", async ({ page }) => {
      await openWorkbench(page);

      const overflow = await page.evaluate(
        () => document.documentElement.scrollWidth - document.documentElement.clientWidth,
      );
      expect(overflow, "page scrolls sideways").toBeLessThanOrEqual(0);

      const bar = runBar(page);
      for (const name of [msg("run.play"), msg("run.step"), msg("run.reset")]) {
        const box = await bar.getByRole("button", { name, exact: true }).boundingBox();
        expect(box, name).not.toBeNull();
        expect(box?.height ?? 0, `${name} height`).toBeGreaterThanOrEqual(44);
        expect(box?.width ?? 0, `${name} width`).toBeGreaterThanOrEqual(44);
      }
    });

    test("runs to the horizon by touch", async ({ page }) => {
      await openWorkbench(page);
      const bar = runBar(page);
      await bar.getByRole("button", { name: msg("run.speedAria", { value: 300 }) }).tap();
      await bar.getByRole("button", { name: msg("run.play"), exact: true }).tap();
      await expect(bar.getByText(msg("run.done"), { exact: true })).toBeVisible({ timeout: 60_000 });
      expect(await shownHash(page)).toBe(golden(SIR));
    });
  });
}
