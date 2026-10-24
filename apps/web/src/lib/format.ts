/**
 * Locale-aware formatting helpers. Pure: no React, no i18next imports, so the
 * node test runner can load them without a DOM.
 */

const DEFAULT_DIGITS = 4;

/** Compact, locale-aware number for readouts and tables. */
export function formatNumber(
  value: number,
  locale: string,
  significantDigits = DEFAULT_DIGITS,
): string {
  if (!Number.isFinite(value)) {
    return "—";
  }
  return new Intl.NumberFormat(locale, {
    maximumSignificantDigits: significantDigits,
  }).format(value);
}

/** Fixed-decimal readout for the instrument clock. */
export function formatFixed(value: number, locale: string, digits = 2): string {
  if (!Number.isFinite(value)) {
    return "—";
  }
  return new Intl.NumberFormat(locale, {
    minimumFractionDigits: digits,
    maximumFractionDigits: digits,
  }).format(value);
}

/** Dot-decimal, grouping-free text suitable for `<input type="number">`. */
export function formatInputNumber(value: number): string {
  if (!Number.isFinite(value)) {
    return "";
  }
  return new Intl.NumberFormat("en-US", {
    maximumSignificantDigits: 10,
    useGrouping: false,
  }).format(value);
}

const TIME_UNIT_KEYS: Record<string, string> = {
  day: "time.unit.day",
  d: "time.unit.day",
  h: "time.unit.h",
  hr: "time.unit.h",
  hour: "time.unit.h",
  s: "time.unit.s",
  sec: "time.unit.s",
  min: "time.unit.min",
  minute: "time.unit.min",
  week: "time.unit.week",
  month: "time.unit.month",
  year: "time.unit.year",
  "1": "time.unit.step",
};

/** Translation key for a model time unit, or `null` for an unknown symbol. */
export function timeUnitKey(unit: string): string | null {
  return TIME_UNIT_KEYS[unit] ?? null;
}
