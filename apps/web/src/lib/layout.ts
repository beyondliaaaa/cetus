/**
 * Deterministic stock-flow layout for the model canvas.
 *
 * Layout is a pure function of the compiled model metadata: no randomness and
 * no force simulation, so the same model always produces the same picture. The
 * renderer maps these specs onto xyflow nodes/edges; keeping the geometry here
 * makes it testable without a DOM.
 *
 * Columns: external sources → auxiliaries → stocks → external sinks. Flows
 * connect stocks/auxiliaries (or a boundary node when `from`/`to` is null);
 * information links are derived from auxiliary→auxiliary identifier references.
 */

import type { AuxiliaryMeta, FlowMeta, StockMeta } from "@cetus/sim-client";

export interface CanvasTarget {
  kind: "flow" | "auxiliary" | "observable" | "stock";
  name: string;
}

/** The slice of model metadata the canvas needs (ModelMeta is compatible). */
export interface CanvasModel {
  stocks: readonly StockMeta[];
  auxiliaries: readonly AuxiliaryMeta[];
  flows: readonly FlowMeta[];
}

export interface CanvasBounds {
  min: number | null;
  max: number | null;
}

export type CanvasNodeKind = "stock" | "auxiliary" | "boundary";

export interface CanvasNodeSpec {
  id: string;
  kind: CanvasNodeKind;
  name: string;
  unit: string | null;
  initial: number | null;
  bounds: CanvasBounds | null;
  target: CanvasTarget | null;
  position: { x: number; y: number };
}

export interface CanvasEdgeSpec {
  id: string;
  source: string;
  target: string;
  kind: "flow" | "info";
  name: string;
  selection: CanvasTarget | null;
}

export interface CanvasLayout {
  nodes: CanvasNodeSpec[];
  edges: CanvasEdgeSpec[];
}

export const CANVAS_GEOMETRY = {
  sourceBoundaryX: -280,
  auxiliaryX: 0,
  stockX: 280,
  targetBoundaryX: 560,
  rowGap: 144,
} as const;

const SOURCE_BOUNDARY_ID = "boundary:source";
const TARGET_BOUNDARY_ID = "boundary:target";
const IDENTIFIER = /[A-Za-z_][A-Za-z0-9_]*/g;

export function identifiersIn(expression: string): string[] {
  return Array.from(expression.matchAll(IDENTIFIER), (match) => match[0]);
}

export function sameTarget(
  left: CanvasTarget | null,
  right: CanvasTarget | null,
): boolean {
  return left !== null && right !== null && left.kind === right.kind && left.name === right.name;
}

function columnY(index: number, count: number): number {
  return (index - (count - 1) / 2) * CANVAS_GEOMETRY.rowGap;
}

/**
 * Stable topological order of auxiliaries. A ready auxiliary is always emitted
 * in declaration order; cycles fall back to declaration order, so the output is
 * deterministic even for a model the compiler should have rejected.
 */
export function orderAuxiliaries(
  auxiliaries: readonly AuxiliaryMeta[],
): readonly AuxiliaryMeta[] {
  const names = new Set(auxiliaries.map((auxiliary) => auxiliary.name));
  const dependencies = new Map<string, ReadonlySet<string>>();
  for (const auxiliary of auxiliaries) {
    const found = new Set<string>();
    for (const identifier of identifiersIn(auxiliary.eq)) {
      if (identifier !== auxiliary.name && names.has(identifier)) {
        found.add(identifier);
      }
    }
    dependencies.set(auxiliary.name, found);
  }

  const ordered: AuxiliaryMeta[] = [];
  const emitted = new Set<string>();
  const remaining = [...auxiliaries];

  while (remaining.length > 0) {
    const readyIndex = remaining.findIndex((auxiliary) =>
      Array.from(dependencies.get(auxiliary.name) ?? []).every((dependency) =>
        emitted.has(dependency),
      ),
    );
    const index = readyIndex === -1 ? 0 : readyIndex;
    const [next] = remaining.splice(index, 1);
    if (next === undefined) {
      break;
    }
    emitted.add(next.name);
    ordered.push(next);
  }

  return ordered;
}

function average(values: readonly number[]): number {
  if (values.length === 0) {
    return 0;
  }
  return values.reduce((sum, value) => sum + value, 0) / values.length;
}

/**
 * Builds the complete canvas geometry for a model.
 *
 * Node ids are namespaced (`stock:charge`, `aux:irradiance`, `boundary:source`)
 * so a stock and an auxiliary can share a name without colliding.
 */
export function buildCanvasLayout(model: CanvasModel): CanvasLayout {
  const nodes: CanvasNodeSpec[] = [];
  const edges: CanvasEdgeSpec[] = [];

  const stocks = model.stocks;
  const orderedAuxiliaries = orderAuxiliaries(model.auxiliaries);
  const stockY = new Map<string, number>();

  stocks.forEach((stock, index) => {
    const y = columnY(index, stocks.length);
    stockY.set(stock.name, y);
    nodes.push({
      id: `stock:${stock.name}`,
      kind: "stock",
      name: stock.name,
      unit: stock.unit,
      initial: stock.initial,
      bounds: { min: stock.bounds.min, max: stock.bounds.max },
      target: { kind: "stock", name: stock.name },
      position: { x: CANVAS_GEOMETRY.stockX, y },
    });
  });

  orderedAuxiliaries.forEach((auxiliary, index) => {
    const y = columnY(index, orderedAuxiliaries.length);
    nodes.push({
      id: `aux:${auxiliary.name}`,
      kind: "auxiliary",
      name: auxiliary.name,
      unit: auxiliary.unit,
      initial: null,
      bounds: null,
      target: { kind: "auxiliary", name: auxiliary.name },
      position: { x: CANVAS_GEOMETRY.auxiliaryX, y },
    });
  });

  const endpointIds = new Map<string, string>();
  for (const stock of stocks) {
    endpointIds.set(stock.name, `stock:${stock.name}`);
  }
  for (const auxiliary of orderedAuxiliaries) {
    endpointIds.set(auxiliary.name, `aux:${auxiliary.name}`);
  }

  const sourceOf = (flow: FlowMeta): string =>
    flow.from === null
      ? SOURCE_BOUNDARY_ID
      : (endpointIds.get(flow.from) ?? SOURCE_BOUNDARY_ID);
  const targetOf = (flow: FlowMeta): string =>
    flow.to === null
      ? TARGET_BOUNDARY_ID
      : (endpointIds.get(flow.to) ?? TARGET_BOUNDARY_ID);
  const needsSourceBoundary = model.flows.some(
    (flow) => sourceOf(flow) === SOURCE_BOUNDARY_ID,
  );
  const needsTargetBoundary = model.flows.some(
    (flow) => targetOf(flow) === TARGET_BOUNDARY_ID,
  );

  if (needsSourceBoundary) {
    const targets = model.flows
      .filter((flow) => sourceOf(flow) === SOURCE_BOUNDARY_ID)
      .map((flow) => flow.to)
      .filter((name): name is string => name !== null)
      .map((name) => stockY.get(name))
      .filter((value): value is number => value !== undefined);
    nodes.push({
      id: SOURCE_BOUNDARY_ID,
      kind: "boundary",
      name: "",
      unit: null,
      initial: null,
      bounds: null,
      target: null,
      position: {
        x: CANVAS_GEOMETRY.sourceBoundaryX,
        y: average(targets),
      },
    });
  }

  if (needsTargetBoundary) {
    const sources = model.flows
      .filter((flow) => targetOf(flow) === TARGET_BOUNDARY_ID)
      .map((flow) => flow.from)
      .filter((name): name is string => name !== null)
      .map((name) => stockY.get(name))
      .filter((value): value is number => value !== undefined);
    nodes.push({
      id: TARGET_BOUNDARY_ID,
      kind: "boundary",
      name: "",
      unit: null,
      initial: null,
      bounds: null,
      target: null,
      position: {
        x: CANVAS_GEOMETRY.targetBoundaryX,
        y: average(sources),
      },
    });
  }

  for (const flow of model.flows) {
    edges.push({
      id: `flow:${flow.name}`,
      source: sourceOf(flow),
      target: targetOf(flow),
      kind: "flow",
      name: flow.name,
      selection: { kind: "flow", name: flow.name },
    });
  }

  const auxiliaryNames = new Set(model.auxiliaries.map((auxiliary) => auxiliary.name));
  for (const auxiliary of model.auxiliaries) {
    const seen = new Set<string>();
    for (const identifier of identifiersIn(auxiliary.eq)) {
      if (
        identifier === auxiliary.name ||
        !auxiliaryNames.has(identifier) ||
        seen.has(identifier)
      ) {
        continue;
      }
      seen.add(identifier);
      edges.push({
        id: `info:${identifier}->${auxiliary.name}`,
        source: `aux:${identifier}`,
        target: `aux:${auxiliary.name}`,
        kind: "info",
        name: `${identifier} → ${auxiliary.name}`,
        selection: null,
      });
    }
  }

  return { nodes, edges };
}
