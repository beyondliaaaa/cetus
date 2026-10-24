import { describe, expect, it } from "vitest";
import { readYamlEquation, readYamlScalar } from "./modelSource";

const SOLAR = `schema: cetus.model/0.1
id: energy.solar-battery
stocks:
  charge:
    unit: "kW*h"
    initial: "0.5 * capacity"
    bounds: { min: 0, max: capacity }
observables:
  charge_pct: { unit: "1", eq: "charge / capacity", display: percent }
  pv_power_w: { unit: W, eq: "pv_power" }
flows:
  charging: { to: charge, unit: W, eq: "pv_power" }
`;

const MULTILINE = `observables:
  charge_pct:
    unit: "1"
    eq: "charge / capacity"
    display: percent
`;

describe("model source reader", () => {
  it("reads an inline observable equation", () => {
    expect(readYamlEquation(SOLAR, "observables", "charge_pct")).toBe(
      "charge / capacity",
    );
    expect(readYamlEquation(SOLAR, "observables", "pv_power_w")).toBe("pv_power");
  });

  it("reads a block-style equation", () => {
    expect(readYamlEquation(MULTILINE, "observables", "charge_pct")).toBe(
      "charge / capacity",
    );
  });

  it("reads a stock initial expression instead of a frozen number", () => {
    expect(readYamlScalar(SOLAR, "stocks", "charge", "initial")).toBe(
      "0.5 * capacity",
    );
  });

  it("returns null when the section or entry is absent", () => {
    expect(readYamlEquation(SOLAR, "auxiliaries", "irradiance")).toBeNull();
    expect(readYamlEquation(SOLAR, "observables", "missing")).toBeNull();
    expect(readYamlEquation(SOLAR, "flows", "discharging")).toBeNull();
    expect(readYamlScalar(SOLAR, "stocks", "missing", "initial")).toBeNull();
  });
});
