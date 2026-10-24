import { describe, expect, it } from "vitest";
import type { ModelMeta } from "@cetus/sim-client";
import {
  defaultTarget,
  editableTargets,
  effectiveMatch,
  patchFor,
  resolveTarget,
  targetGroups,
  targetKey,
} from "./equation";

const META: ModelMeta = {
  id: "test.model",
  version: "1.0.0",
  title: { en: "Test model" },
  tier: "empirical",
  paradigms: ["sd"],
  views: ["series"],
  contentHash: "abc123",
  time: {
    unit: "h",
    unitFactor: 3600,
    dt: 0.1,
    horizon: 72,
    integrator: "rk4",
    kind: "continuous",
    start: 0,
    startTick: 0,
  },
  parameters: [],
  stocks: [
    {
      name: "charge",
      unit: "kW*h",
      unitFactor: 3_600_000,
      initial: 10,
      siInitial: 36_000_000,
      bounds: { min: 0, max: 20 },
      dim: "energy",
    },
  ],
  auxiliaries: [
    {
      name: "pv_power",
      unit: "W",
      unitFactor: 1,
      eq: "area * irradiance",
      dim: "power",
    },
  ],
  flows: [
    {
      name: "charging",
      from: null,
      to: "charge",
      unit: "W",
      unitFactor: 1,
      eq: "pv_power",
      dim: "power",
    },
  ],
  observables: [
    {
      name: "charge_pct",
      unit: "1",
      unitFactor: 1,
      display: "percent",
      dim: "1",
    },
  ],
  invariants: [],
};

const YAML = `stocks:
  charge:
    unit: "kW*h"
    initial: "0.5 * capacity"
observables:
  charge_pct: { unit: "1", eq: "charge / capacity", display: percent }
`;

describe("equation editor model helpers", () => {
  it("orders editable targets by kind and exposes a default", () => {
    expect(targetGroups(META).map((group) => group.kind)).toEqual([
      "flow",
      "auxiliary",
      "observable",
      "stock",
    ]);
    expect(editableTargets(META).map(targetKey)).toEqual([
      "flow:charging",
      "auxiliary:pv_power",
      "observable:charge_pct",
      "stock:charge",
    ]);
    expect(defaultTarget(META)).toEqual({ kind: "flow", name: "charging" });
  });

  it("reads compiled equations from metadata and observables from YAML", () => {
    expect(resolveTarget(META, YAML, { kind: "flow", name: "charging" })).toEqual({
      unit: "W",
      compiled: "pv_power",
    });
    expect(
      resolveTarget(META, YAML, { kind: "auxiliary", name: "pv_power" }),
    ).toEqual({ unit: "W", compiled: "area * irradiance" });
    expect(
      resolveTarget(META, YAML, { kind: "observable", name: "charge_pct" }),
    ).toEqual({ unit: "1", compiled: "charge / capacity" });
  });

  it("keeps the authored stock initial expression", () => {
    expect(resolveTarget(META, YAML, { kind: "stock", name: "charge" })).toEqual({
      unit: "kW*h",
      compiled: "0.5 * capacity",
    });
  });

  it("falls back to a typed numeric literal when the source has no stock", () => {
    expect(
      resolveTarget(META, "observables:\n  charge_pct: { eq: charge }\n", {
        kind: "stock",
        name: "charge",
      }),
    ).toEqual({ unit: "kW*h", compiled: "10[kW*h]" });
  });

  it("treats a dimensionless stock initial as the declared unit", () => {
    const dimensionless = {
      ok: true as const,
      dim: "1",
      unit: "1",
      type: "scalar" as const,
      matchesExpected: false,
      expectedUnit: "person",
    };
    expect(effectiveMatch(dimensionless, "stock")).toEqual({
      ...dimensionless,
      matchesExpected: true,
    });
    expect(effectiveMatch(dimensionless, "flow")).toEqual(dimensionless);
  });

  it("maps every target kind to the matching worker patch", () => {
    expect(patchFor({ kind: "flow", name: "charging" }, "x")).toEqual({
      op: "set_eq",
      section: "flows",
      name: "charging",
      eq: "x",
    });
    expect(patchFor({ kind: "auxiliary", name: "pv_power" }, "x")).toEqual({
      op: "set_eq",
      section: "auxiliaries",
      name: "pv_power",
      eq: "x",
    });
    expect(patchFor({ kind: "observable", name: "charge_pct" }, "x")).toEqual({
      op: "set_eq",
      section: "observables",
      name: "charge_pct",
      eq: "x",
    });
    expect(patchFor({ kind: "stock", name: "charge" }, "x")).toEqual({
      op: "set_initial",
      name: "charge",
      expr: "x",
    });
  });
});
