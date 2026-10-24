import { describe, expect, it } from "vitest";
import { rangeToSi, toDisplayValue, toSiValue } from "./units";

const KILOWATT_HOUR = 3.6e6;
const PERSON_PER_DAY = 1 / 86_400;
const PERCENT = 0.01;

describe("display-unit conversion", () => {
  it("converts kW*h into SI joules and back", () => {
    expect(toSiValue(20, KILOWATT_HOUR)).toBe(72_000_000);
    expect(toDisplayValue(72_000_000, KILOWATT_HOUR)).toBeCloseTo(20, 12);
  });

  it("converts person/day into SI person/second", () => {
    const si = toSiValue(10, PERSON_PER_DAY);
    expect(si).toBeCloseTo(10 / 86_400, 12);
    expect(toDisplayValue(si, PERSON_PER_DAY)).toBeCloseTo(10, 12);
  });

  it("converts percent into a dimensionless fraction", () => {
    expect(toDisplayValue(0.5, PERCENT)).toBeCloseTo(50, 12);
    expect(toSiValue(50, PERCENT)).toBeCloseTo(0.5, 12);
  });

  it("treats an explicit zero offset as no offset", () => {
    expect(toDisplayValue(7.5, 2.5, 0)).toBe(3);
    expect(toSiValue(3, 2.5, 0)).toBe(7.5);
    expect(toDisplayValue(7.5, 2.5)).toBe(toDisplayValue(7.5, 2.5, 0));
  });

  it("round-trips an affine unit with a non-zero offset", () => {
    const kelvin = 300;
    const celsius = toDisplayValue(kelvin, 1, 273.15);
    expect(celsius).toBeCloseTo(26.85, 12);
    expect(toSiValue(celsius, 1, 273.15)).toBeCloseTo(kelvin, 12);
  });

  it("maps a display range into SI", () => {
    expect(rangeToSi([1, 200], KILOWATT_HOUR)).toEqual([3.6e6, 7.2e8]);
    expect(rangeToSi([0, 100], 1, 273.15)).toEqual([273.15, 373.15]);
  });
});
