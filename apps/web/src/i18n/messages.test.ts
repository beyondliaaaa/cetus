import { IntlMessageFormat } from "intl-messageformat";
import { describe, expect, it } from "vitest";
import { LATEX_ERROR_CODES } from "../lib/latex";
import { en, zhCN } from "./messages";

const catalogs = { "zh-CN": zhCN, en } as const;

// LaTeX hints only ever receive `detail` (MathFormulaInput).
function formatHint(locale: string, message: string): string {
  return String(new IntlMessageFormat(message, locale).format({ detail: "DETAIL" }));
}

describe("message catalogue", () => {
  it("has the same keys in every locale", () => {
    expect(Object.keys(en).sort()).toEqual(Object.keys(zhCN).sort());
  });

  for (const [locale, catalog] of Object.entries(catalogs)) {
    it(`${locale}: every message is valid ICU`, () => {
      // The constructor parses the message and throws on syntax errors, such
      // as an unquoted literal brace (`\mathrm{...}` instead of `\mathrm'{...}'`).
      for (const [key, message] of Object.entries(catalog)) {
        expect(() => new IntlMessageFormat(message, locale), key).not.toThrow();
      }
    });

    it(`${locale}: every LaTeX error code has a message`, () => {
      for (const code of LATEX_ERROR_CODES) {
        expect(catalog, code).toHaveProperty([`editor.latex.${code}`]);
      }
    });

    it(`${locale}: LaTeX hints show their braces literally`, () => {
      // ICU treats `{…}` as a placeholder, so literal braces must be quoted
      // ('{…}'). `\log_{10}` unquoted would even parse — as argument "10".
      const subscript = formatHint(locale, catalog["editor.latex.subscript"]);
      expect(subscript).toContain("\\mathrm{...}");
      expect(subscript).toContain("\\mathrm{pv\\_power}");
      expect(formatHint(locale, catalog["editor.latex.text"])).toContain("\\text{...}");
      expect(formatHint(locale, catalog["editor.latex.log-base"])).toContain("\\log_{10}");
      const unsupported = formatHint(locale, catalog["editor.latex.log-base-unsupported"]);
      expect(unsupported).toContain("DETAIL");
      expect(unsupported).toContain("\\log_{10}");
    });
  }
});
