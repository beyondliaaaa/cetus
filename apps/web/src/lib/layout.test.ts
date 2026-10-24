import { describe, expect, it } from "vitest";
import type { AuxiliaryMeta, FlowMeta, StockMeta } from "@cetus/sim-client";
import {
  buildCanvasLayout,
  CANVAS_GEOMETRY,
  orderAuxiliaries,
  type CanvasModel,
} from "./layout";

function stock(name: string, initial = 0): StockMeta {
  return {
    name,
    unit: "person",
    unitFactor: 1,
    initial,
    siInitial: initial,
    bounds: { min: 0, max: null },
    dim: "person",
  };
}

function auxiliary(name: string, eq: string, unit = "1"): AuxiliaryMeta {
  return { name, unit, unitFactor: 1, eq, dim: "1" };
}

function flow(
  name: string,
  from: string | null,
  to: string | null,
  eq = "0",
): FlowMeta {
  return {
    name,
    from,
    to,
    unit: "person/day",
    unitFactor: 1,
    eq,
    dim: "person/day",
  };
}

const SIR: CanvasModel = {
  stocks: [stock("susceptible", 990), stock("infectious", 10), stock("recovered", 0)],
  auxiliaries: [],
  flows: [
    flow("infection", "susceptible", "infectious"),
    flow("recovery", "infectious", "recovered"),
  ],
};

const SOLAR: CanvasModel = {
  stocks: [stock("charge", 10)],
  auxiliaries: [
    auxiliary("irradiance", "peak_irradiance * sin(2*pi*time/day_length)", "W/m^2"),
    auxiliary("pv_power", "efficiency * area * irradiance", "W"),
  ],
  flows: [
    flow("charging", null, "charge"),
    flow("discharging", "charge", null),
  ],
};

describe("canvas layout", () => {
  it("places stocks and auxiliaries in deterministic, centred columns", () => {
    const layout = buildCanvasLayout(SIR);
    const stocks = layout.nodes.filter((node) => node.kind === "stock");

    expect(stocks.map((node) => node.position.x)).toEqual([
      CANVAS_GEOMETRY.stockX,
      CANVAS_GEOMETRY.stockX,
      CANVAS_GEOMETRY.stockX,
    ]);
    expect(stocks.map((node) => node.position.y)).toEqual([
      -CANVAS_GEOMETRY.rowGap,
      0,
      CANVAS_GEOMETRY.rowGap,
    ]);
  });

  it("keeps declaration order for independent auxiliaries", () => {
    const auxiliaries = [auxiliary("alpha", "1"), auxiliary("beta", "2")];
    expect(orderAuxiliaries(auxiliaries).map((item) => item.name)).toEqual([
      "alpha",
      "beta",
    ]);
  });

  it("orders auxiliaries topologically by identifier references", () => {
    const auxiliaries = [
      auxiliary("pv_power", "efficiency * irradiance"),
      auxiliary("irradiance", "peak_irradiance * sin(time)"),
    ];
    const model: CanvasModel = { stocks: [], auxiliaries, flows: [] };
    const layout = buildCanvasLayout(model);
    const nodes = layout.nodes.filter((node) => node.kind === "auxiliary");

    expect(nodes.map((node) => node.name)).toEqual(["irradiance", "pv_power"]);
    expect(nodes[0]?.position.y).toBeLessThan(nodes[1]?.position.y ?? 0);
  });

  it("derives auxiliary information links without self edges", () => {
    const layout = buildCanvasLayout(SOLAR);
    const info = layout.edges.filter((edge) => edge.kind === "info");

    expect(info).toHaveLength(1);
    expect(info[0]).toMatchObject({
      id: "info:irradiance->pv_power",
      source: "aux:irradiance",
      target: "aux:pv_power",
      selection: null,
    });
  });

  it("creates boundary nodes for flows with a missing end", () => {
    const layout = buildCanvasLayout(SOLAR);
    const boundaries = layout.nodes.filter((node) => node.kind === "boundary");

    expect(boundaries.map((node) => node.id)).toEqual([
      "boundary:source",
      "boundary:target",
    ]);
    expect(boundaries.map((node) => node.position.x)).toEqual([
      CANVAS_GEOMETRY.sourceBoundaryX,
      CANVAS_GEOMETRY.targetBoundaryX,
    ]);
    expect(boundaries.map((node) => node.position.y)).toEqual([0, 0]);

    const charging = layout.edges.find((edge) => edge.id === "flow:charging");
    const discharging = layout.edges.find((edge) => edge.id === "flow:discharging");
    expect(charging).toMatchObject({
      source: "boundary:source",
      target: "stock:charge",
      kind: "flow",
      name: "charging",
    });
    expect(discharging).toMatchObject({
      source: "stock:charge",
      target: "boundary:target",
      kind: "flow",
      name: "discharging",
    });
  });

  it("marks flows as editable and keeps stocks/auxiliaries selectable", () => {
    const layout = buildCanvasLayout(SOLAR);
    const charge = layout.nodes.find((node) => node.id === "stock:charge");
    const irradiance = layout.nodes.find((node) => node.id === "aux:irradiance");
    const charging = layout.edges.find((edge) => edge.id === "flow:charging");

    expect(charge?.target).toEqual({ kind: "stock", name: "charge" });
    expect(charge?.initial).toBe(10);
    expect(irradiance?.target).toEqual({ kind: "auxiliary", name: "irradiance" });
    expect(charging?.selection).toEqual({ kind: "flow", name: "charging" });
    expect(layout.nodes.some((node) => node.id.startsWith("parameter:"))).toBe(false);
  });

  it("is deterministic across repeated builds", () => {
    expect(buildCanvasLayout(SOLAR)).toEqual(buildCanvasLayout(SOLAR));
    expect(buildCanvasLayout(SIR)).toEqual(buildCanvasLayout(SIR));
  });
});
